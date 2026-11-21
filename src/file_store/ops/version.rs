//! Versions: the project's list of versions and its current one, and the version a task is
//! planned for. Every write holds the project's write lock and saves atomically.

use chrono::{DateTime, Utc};

use super::epic::read_epic_file;
use super::version_names::{add_listed, resolve};
use super::{
    ensure_ai_task_not_done, lock_project, reason_or_default, save_project, save_task, stamp,
};
use crate::domain::{Actor, Changed, Draft, ProjectId, validate_version_name};
use crate::error::AgboardError;
use crate::file_store::FileStore;
use crate::file_store::model::{HistoryRecord, PlannedVersionChange, TaskFile, TaskView};
use crate::file_store::reader;
use crate::file_store::validate::EpicId;

/// The versions of a project and the current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectVersions {
    pub versions: Vec<String>,
    pub current: Option<String>,
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

/// Which tasks of an epic a bulk change reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpicVersionScope {
    /// Every task the actor may change.
    All,
    /// Only the tasks with no planned version; the others are skipped.
    UnplannedOnly,
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
    /// The project's versions and its current version.
    pub fn project_versions(&self, project: &ProjectId) -> Result<ProjectVersions, AgboardError> {
        let file = reader::read_project_file(self.shared_dir(), project)?;
        Ok(ProjectVersions {
            versions: file.versions,
            current: file.current_version,
        })
    }

    /// List a version in the project. Listing a version that is already listed changes nothing.
    pub fn add_version(
        &self,
        project: &ProjectId,
        name: &str,
    ) -> Result<Changed<ProjectVersions>, AgboardError> {
        let name = validate_version_name(name)?;
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut file = reader::read_project_file(shared, project)?;
        let changed = add_listed(&mut file.versions, &name)?;
        if changed {
            save_project(shared, &project_dir, &file)?;
        }
        Ok(Changed {
            value: ProjectVersions {
                versions: file.versions,
                current: file.current_version,
            },
            changed,
        })
    }

    /// Set (`Some`) or clear (`None`) the project's current version. A version that is not yet
    /// listed is listed first. Tasks keep the version they were given.
    pub fn set_current_version(
        &self,
        project: &ProjectId,
        name: Option<&str>,
    ) -> Result<Changed<ProjectVersions>, AgboardError> {
        let name = name.map(validate_version_name).transpose()?;
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let mut file = reader::read_project_file(shared, project)?;
        let mut changed = false;
        if let Some(name) = &name {
            changed |= add_listed(&mut file.versions, name)?;
        }
        if file.current_version != name {
            file.current_version = name;
            changed = true;
        }
        if changed {
            save_project(shared, &project_dir, &file)?;
        }
        Ok(Changed {
            value: ProjectVersions {
                versions: file.versions,
                current: file.current_version,
            },
            changed,
        })
    }

    /// Check that `choice` can be stored now, without saving anything: a named version must be
    /// listed, and `Current` needs a current version.
    pub fn check_version_choice(
        &self,
        project: &ProjectId,
        choice: &VersionChoice,
    ) -> Result<(), AgboardError> {
        resolve(self.shared_dir(), project, choice).map(|_| ())
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

    /// Put `target` on the task and record the change. Returns whether anything changed.
    fn apply_version(
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
        task.planned_version_changes.push(PlannedVersionChange {
            at,
            from,
            to: target.clone(),
        });
        task.planned_version = target;
        task.updated_at = at;
        Ok(true)
    }
}
