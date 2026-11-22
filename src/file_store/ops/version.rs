//! Versions: the project's list of versions (the files of `docs/release/`) and its current one, and the version a task is
//! planned for. Every write holds the project's write lock and saves atomically.

use chrono::{DateTime, Utc};

use super::epic::read_epic_file;
pub(super) use super::version_names::resolve;
use super::version_plan::EpicVersionScope;
use super::{
    ensure_ai_task_not_done, lock_project, reason_or_default, save_project, save_task, stamp,
};
use crate::domain::{Actor, Changed, Draft, ProjectId, validate_version_name};
use crate::error::AgboardError;
use crate::file_store::FileStore;
use crate::file_store::model::{HistoryRecord, ProjectFile, TaskFile, TaskView};
use crate::file_store::reader;
use crate::file_store::releases::{create_release_file, effective_current, listed_versions};
use crate::file_store::validate::EpicId;

/// The versions of a project and the current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectVersions {
    pub versions: Vec<String>,
    pub current: Option<String>,
}

/// The listed versions and the current one of a project, read now.
fn versions_of(
    shared: &std::path::Path,
    project: &ProjectId,
    file: &ProjectFile,
) -> ProjectVersions {
    let versions = listed_versions(&crate::file_store::layout::project_dir(shared, project));
    let current = effective_current(&versions, file.current_version.as_deref());
    ProjectVersions { versions, current }
}

/// Which version a task is planned for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionChoice {
    /// Remove the plan.
    Unset,
    /// The project's current version, resolved to its name when the task is saved.
    Current,
    /// A version the project lists.
    Named(String),
}

/// The outcome of setting one version on every task of an epic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicVersionSet {
    /// The tasks whose planned version changed.
    pub changed: Vec<i64>,
    /// The tasks that already had the version.
    pub unchanged: Vec<i64>,
    /// The tasks left alone: archived, or not changeable by the actor.
    pub skipped: Vec<i64>,
}

impl FileStore {
    /// The project's versions (the files in `docs/release/`) and its current version. A current
    /// version whose file is missing is reported as none.
    pub fn project_versions(&self, project: &ProjectId) -> Result<ProjectVersions, AgboardError> {
        let file = reader::read_project_file(self.shared_dir(), project)?;
        Ok(versions_of(self.shared_dir(), project, &file))
    }

    /// List a version in the project by creating `docs/release/<name>.md` from the release-note
    /// template, and make sure one Backlog card exists for writing its release notes. A version
    /// that is already listed keeps its file untouched; the card is still issued when none exists,
    /// so a retry after a failed first call finishes the work.
    pub fn add_version(
        &self,
        project: &ProjectId,
        name: &str,
        now: DateTime<Utc>,
    ) -> Result<Changed<ProjectVersions>, AgboardError> {
        let name = validate_version_name(name)?;
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let file = reader::read_project_file(shared, project)?;
        let mut changed = create_release_file(shared, &project_dir, &name)?;
        changed |= self.ensure_release_notes_task(&project_dir, project, &name, now)?;
        Ok(Changed {
            value: versions_of(shared, project, &file),
            changed,
        })
    }

    /// Set (`Some`) or clear (`None`) the project's current version. A version that is not yet
    /// listed is listed first, by creating its file from the template and issuing its
    /// release-notes card. Tasks keep the version they were given.
    pub fn set_current_version(
        &self,
        project: &ProjectId,
        name: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<Changed<ProjectVersions>, AgboardError> {
        let name = name.map(validate_version_name).transpose()?;
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut file = reader::read_project_file(shared, project)?;
        let mut changed = false;
        if let Some(name) = &name
            && create_release_file(shared, &project_dir, name)?
        {
            self.ensure_release_notes_task(&project_dir, project, name, now)?;
            // Creating the card advanced the task counter in project.json: read the file
            // again so saving the current version does not write the old counter back.
            file = reader::read_project_file(shared, project)?;
            changed = true;
        }
        if file.current_version != name {
            file.current_version = name;
            save_project(shared, &project_dir, &file)?;
            changed = true;
        }
        Ok(Changed {
            value: versions_of(shared, project, &file),
            changed,
        })
    }

    /// Turn `choice` into a concrete one without saving anything: `Current` becomes the current
    /// version's name, and a named version must be listed. A listed version is never removed, so
    /// storing the result later cannot fail for the version itself.
    pub fn resolve_version_choice(
        &self,
        project: &ProjectId,
        choice: &VersionChoice,
    ) -> Result<VersionChoice, AgboardError> {
        Ok(match resolve(self.shared_dir(), project, choice)? {
            Some(name) => VersionChoice::Named(name),
            None => VersionChoice::Unset,
        })
    }

    /// Set or clear the version a task is planned for. `Current` stores the project's current
    /// version as it is now. Only a real change records a history entry and moves `updatedAt`;
    /// the same value again changes nothing. An AI cannot change an archived, draft, or Done
    /// task; a person can change a draft or Done task but not an archived one.
    pub fn set_task_planned_version(
        &self,
        project: &ProjectId,
        task_id: i64,
        choice: &VersionChoice,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<Changed<TaskView>, AgboardError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let target = resolve(shared, project, choice)?;
        let mut task = reader::read_task_strict(shared, project, task_id)?;
        if task.archived_at.is_some() {
            return Err(AgboardError::TaskArchived { id: task_id });
        }
        if actor == Actor::Ai {
            if task.draft == Some(Draft::Human) {
                return Err(AgboardError::TaskIsDraft { id: task_id });
            }
            ensure_ai_task_not_done(&task, actor)?;
        }
        let changed = self.apply_version(project, &mut task, target, now)?;
        if changed {
            save_task(shared, &project_dir, &task)?;
        }
        Ok(Changed {
            value: reader::read_task_locked(shared, project, task_id)?,
            changed,
        })
    }

    /// Set or clear the planned version of every unarchived task of an epic, each saved on its
    /// own, so a failure part way leaves the earlier tasks changed and a retry finishes the rest.
    /// Tasks that already have the version are not touched. The tasks keep their other fields.
    pub fn set_epic_planned_version(
        &self,
        project: &ProjectId,
        epic: &EpicId,
        choice: &VersionChoice,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<EpicVersionSet, AgboardError> {
        self.set_epic_planned_version_in_scope(
            project,
            epic,
            choice,
            EpicVersionScope::All,
            actor,
            now,
        )
    }

    /// [`Self::set_epic_planned_version`] limited to `scope`. With `UnplannedOnly`, a task that
    /// already has a planned version is skipped, whatever version it has.
    pub fn set_epic_planned_version_in_scope(
        &self,
        project: &ProjectId,
        epic: &EpicId,
        choice: &VersionChoice,
        scope: EpicVersionScope,
        actor: Actor,
        now: DateTime<Utc>,
    ) -> Result<EpicVersionSet, AgboardError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        read_epic_file(shared, project, epic)?;
        let target = resolve(shared, project, choice)?;
        let mut result = EpicVersionSet {
            changed: Vec::new(),
            unchanged: Vec::new(),
            skipped: Vec::new(),
        };
        for task_id in reader::task_ids(&project_dir)? {
            let mut task = reader::read_task_strict(shared, project, task_id)?;
            if task.epic_id.as_deref() != Some(epic.as_str()) {
                continue;
            }
            let blocked = task.archived_at.is_some()
                || (scope == EpicVersionScope::UnplannedOnly && task.planned_version.is_some())
                || (actor == Actor::Ai
                    && (task.draft == Some(Draft::Human)
                        || ensure_ai_task_not_done(&task, actor).is_err()));
            if blocked {
                result.skipped.push(task_id);
                continue;
            }
            if self.apply_version(project, &mut task, target.clone(), now)? {
                save_task(shared, &project_dir, &task)?;
                result.changed.push(task_id);
            } else {
                result.unchanged.push(task_id);
            }
        }
        Ok(result)
    }

    /// A description replacement that names a different `plannedVersion` is an explicit edit of
    /// the plan: keep its value and record the change like any other. A replacement that omits
    /// the key, or holds an unreadable one, leaves the plan alone.
    pub(super) fn record_replacement_plan(
        &self,
        project: &ProjectId,
        task: &mut TaskFile,
        markdown: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<(), AgboardError> {
        if let Some(Ok(Some(version))) =
            markdown.map(crate::file_store::frontmatter::planned_version_of)
        {
            self.apply_version(project, task, Some(version), now)?;
        }
        Ok(())
    }

    /// Put `target` on the task and record the change. Returns whether anything changed.
    pub(super) fn apply_version(
        &self,
        project: &ProjectId,
        task: &mut TaskFile,
        target: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<bool, AgboardError> {
        if task.planned_version == target {
            return Ok(false);
        }
        let at = stamp(now);
        let language = self.history_language(project, task)?;
        let from = task.planned_version.take();
        let english = format!(
            "Changed planned version from {} to {}.",
            from.as_deref().unwrap_or("none"),
            target.as_deref().unwrap_or("none")
        );
        let japanese = format!(
            "リリース予定の版を{}から{}に変更しました。",
            from.as_deref().unwrap_or("なし"),
            target.as_deref().unwrap_or("なし")
        );
        let reason = reason_or_default(None, language.text(&english, &japanese).to_owned())?;
        task.history.push(HistoryRecord { at, reason });
        task.planned_version = target;
        task.updated_at = at;
        Ok(true)
    }
}
