//! Project snapshots: the shape, the read of one project and of all of them.

use super::{
    epics::read_epics,
    project::{list_projects, meta_of, project_dir_checked, read_project_file},
    task::{attach_epic, read_task_lenient, task_ids},
};
use std::path::Path;

use serde::Serialize;

use crate::domain::{ChangeCursor, ProjectCatalog, ProjectId, ProjectMeta};
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::index;
use crate::file_store::layout;
use crate::file_store::lock::WriteLock;
use crate::file_store::model::{EpicView, Problem, TaskView};
use crate::file_store::qa::{self, QaIndex, QuestionView, RequestView};
use crate::file_store::transaction;

/// One project as the snapshot returns it: the project, its content version and
/// the cursor for that version, its tasks with their full history, and every
/// defect that was tolerated while reading it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub project: ProjectMeta,
    pub next_task_id: i64,
    pub version: String,
    pub cursor: ChangeCursor,
    pub epics: Vec<EpicView>,
    pub tasks: Vec<TaskView>,
    pub problems: Vec<Problem>,
    /// The project's versions (the files of `docs/release/`) and its release version, which is none
    /// while its file is missing. The board reads them; agents read them with
    /// the version commands, so the snapshot JSON leaves them out.
    #[serde(skip)]
    pub versions: Vec<String>,
    #[serde(skip)]
    pub release_version: Option<String>,
    /// Every question of the project with its state, in ID order. Agents read them with
    /// `message ls` and `message show`, so the snapshot JSON leaves them out.
    #[serde(skip)]
    pub questions: Vec<QuestionView>,
    /// Every request of the project with its state, in ID order.
    #[serde(skip)]
    pub requests: Vec<RequestView>,
    #[serde(skip)]
    pub agents: Vec<crate::file_store::ops::AgentRecord>,
}

/// Orders archived tasks for the archive list: newest `archivedAt` first, a task
/// without an archive time after all the others, and the higher id first when
/// the times are equal (or both missing), so the order never depends on read order.
pub fn sort_archived_newest_first(tasks: &mut [&TaskView]) {
    tasks.sort_by(|a, b| {
        b.archived_at
            .is_some()
            .cmp(&a.archived_at.is_some())
            .then_with(|| b.archived_at.cmp(&a.archived_at))
            .then_with(|| b.id.cmp(&a.id))
    });
}

impl ProjectSnapshot {
    /// The same snapshot without the archived tasks (the board and the CLI snapshot).
    pub fn without_archived(mut self) -> ProjectSnapshot {
        self.tasks.retain(|task| task.archived_at.is_none());
        self
    }

    /// The same snapshot with only the archived tasks (the archive list).
    pub fn only_archived(mut self) -> ProjectSnapshot {
        self.tasks.retain(|task| task.archived_at.is_some());
        self
    }
}

/// A snapshot with the time it was fetched, as the CLI, the hook and wait print
/// it. The fetch time is added here, never stored.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchedSnapshot<'a> {
    pub fetched_at: String,
    #[serde(flatten)]
    pub snapshot: &'a ProjectSnapshot,
}

/// The snapshot of one project: its project file (strict), and every task
/// with the defects that were tolerated.
pub fn read_snapshot(shared: &Path, project_id: &ProjectId) -> Result<ProjectSnapshot, AgpjError> {
    let project_dir = project_dir_checked(shared, project_id)?;
    let _lock = WriteLock::acquire(&layout::write_lock(&project_dir))?;
    transaction::recover_locked(&project_dir)?;
    crate::file_store::ops::migrate_legacy_locked(shared, project_id, &project_dir)?;
    read_snapshot_locked(shared, project_id)
}

/// Read a project snapshot while its project lock is already held.
pub(crate) fn read_snapshot_locked(
    shared: &Path,
    project_id: &ProjectId,
) -> Result<ProjectSnapshot, AgpjError> {
    let project_dir = project_dir_checked(shared, project_id)?;
    // The version is taken before the files are read: a save in between makes
    // the next reload see a different version and read again.
    let version = index::project_version(&project_dir);
    let cursor = ChangeCursor::new(project_id.clone(), &version)?;
    let file = read_project_file(shared, project_id)?;
    let next_task_id = file.next_task_id;
    let versions = crate::file_store::releases::listed_versions(&project_dir);
    let release_version =
        crate::file_store::releases::effective_release(&versions, file.release_version.as_deref());
    let project = meta_of(file);
    // Read the registry once for the snapshot, then resolve legacy AI cards in memory. A
    // missing registry intentionally leaves legacy boards untouched.
    let agents = crate::file_store::ops::registered_agents(shared, &project_dir);
    let legacy_default = agents
        .iter()
        .find(|agent| agent.is_default && agent.archived_at.is_none())
        .map(|agent| &agent.id);
    let mut tasks = Vec::new();
    let (epics, mut problems) = read_epics(shared, &project_dir);
    for task_id in task_ids(&project_dir)? {
        match read_task_lenient(shared, project_id, task_id) {
            Ok(load) => {
                let mut view = load.view;
                if view.agent_id.is_none() && view.assignee == crate::domain::Assignee::Ai {
                    view.agent_id = legacy_default.cloned();
                }
                tasks.push(view);
                problems.extend(load.problems);
            }
            Err(found) => problems.extend(found),
        }
    }
    let qa_index = QaIndex::load(shared, &project_dir)?;
    for task in &mut tasks {
        attach_epic(task, &epics);
        qa::apply(task, &qa_index);
    }
    problems.extend(qa_index.problems.iter().cloned());
    problems.extend(qa::missing_references(&tasks, &qa_index, |id| {
        layout::shared_label(
            shared,
            &layout::task_md(&layout::task_dir(&project_dir, id)),
        )
    }));
    let questions = qa::question_views(&qa_index, &tasks);
    let requests = qa::request_views(&qa_index);
    Ok(ProjectSnapshot {
        project,
        next_task_id,
        version,
        cursor,
        epics,
        tasks,
        problems,
        versions,
        release_version,
        questions,
        requests,
        agents,
    })
}

/// Every project's unarchived snapshot, with the catalog version of the same read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllSnapshots {
    pub catalog_version: String,
    pub projects: Vec<ProjectSnapshot>,
}

impl FileStore {
    /// The project list with the catalog version, computed from the files read now.
    pub fn catalog(&self) -> Result<ProjectCatalog, AgpjError> {
        let all = self.all_snapshots()?;
        Ok(ProjectCatalog {
            projects: all.projects.iter().map(|p| p.project.clone()).collect(),
            catalog_version: all.catalog_version,
        })
    }

    /// The project with this name. A name that matches no project is
    /// `project_not_found`.
    pub fn project_by_name(&self, name: &str) -> Result<ProjectMeta, AgpjError> {
        list_projects(self.shared_dir())?
            .into_iter()
            .find(|project| project.name == name)
            .ok_or_else(|| AgpjError::ProjectNotFound {
                project: name.to_string(),
            })
    }

    /// One project's unarchived tasks, with its version, its cursor, and the
    /// defects of its files.
    pub fn board(&self, id: &ProjectId) -> Result<ProjectSnapshot, AgpjError> {
        Ok(read_snapshot(self.shared_dir(), id)?.without_archived())
    }

    /// One project's archived tasks, read the same way as [`FileStore::board`].
    pub fn archive_snapshot(&self, id: &ProjectId) -> Result<ProjectSnapshot, AgpjError> {
        Ok(read_snapshot(self.shared_dir(), id)?.only_archived())
    }

    /// Every project's unarchived snapshot in name order, with the catalog
    /// version. A project whose `project.json` is defective fails the whole read.
    pub fn all_snapshots(&self) -> Result<AllSnapshots, AgpjError> {
        let mut projects = Vec::new();
        for meta in list_projects(self.shared_dir())? {
            projects.push(self.board(&meta.id)?);
        }
        let mut by_id: Vec<(&ProjectId, &str)> = projects
            .iter()
            .map(|p| (&p.project.id, p.version.as_str()))
            .collect();
        by_id.sort_by(|a, b| a.0.cmp(b.0));
        let catalog_version = index::catalog_version_of(by_id);
        Ok(AllSnapshots {
            catalog_version,
            projects,
        })
    }
}
