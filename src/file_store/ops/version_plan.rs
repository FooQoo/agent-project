//! Changing planned versions from the board: one task checked against what the page saw, and
//! one epic's checked tasks, which can be previewed without writing.

use chrono::{DateTime, Utc};

use super::version::resolve;
use super::{EpicVersionSet, VersionChoice};
use super::{lock_project, save_task};
use crate::domain::Column;
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::model::TaskView;
use crate::file_store::reader;
use crate::file_store::validate::EpicId;

use super::epic::read_epic_file;

/// Which tasks of the checked ones an epic-wide change touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpicVersionScope {
    /// Only tasks that have no planned version.
    UnplannedOnly,
    /// Every task, replacing the version it has.
    All,
}

/// Why a task was left alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Archived,
    /// A shipped version is not rewritten, whoever asks.
    Done,
    /// The scope is `UnplannedOnly` and the task already has a version.
    Planned,
}

impl SkipReason {
    pub fn as_str(self) -> &'static str {
        match self {
            SkipReason::Archived => "archived",
            SkipReason::Done => "done",
            SkipReason::Planned => "planned",
        }
    }
}

/// The outcome of a person's epic-wide change, or of its preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicVersionPlan {
    pub result: EpicVersionSet,
    /// The reason for each skipped task, in the order of `result.skipped`.
    pub skipped_reasons: Vec<SkipReason>,
}

impl FileStore {
    /// Set or clear one task's planned version as a person. With `expected` (`Some(None)` for
    /// "no version", `Some(Some(v))` for a version), a task whose planned version is no longer
    /// that is `planned_version_changed` and nothing is written. A draft, Done, or archived task
    /// can be changed by a person.
    pub fn set_task_planned_version_checked(
        &self,
        project: &crate::domain::ProjectId,
        task_id: i64,
        choice: &VersionChoice,
        expected: Option<Option<&str>>,
        now: DateTime<Utc>,
    ) -> Result<crate::domain::Changed<TaskView>, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        let target = resolve(shared, project, choice)?;
        let mut task = reader::read_task_strict(shared, project, task_id)?;
        if let Some(expected) = expected
            && task.planned_version.as_deref() != expected
        {
            return Err(AgpjError::PlannedVersionChanged {
                id: task_id,
                current: task.planned_version.clone(),
            });
        }
        let changed = self.apply_version(project, &mut task, target, now)?;
        if changed {
            save_task(shared, &project_dir, &task)?;
        }
        Ok(crate::domain::Changed {
            value: reader::read_task_locked(shared, project, task_id)?,
            changed,
        })
    }

    /// Set or clear the planned version of the tasks of an epic that a person left checked.
    /// Done and archived tasks are always skipped, and with `UnplannedOnly` so are tasks that
    /// already have a version. A task ID that is not a task of the epic is `invalid_value`.
    /// With `dry_run` nothing is written and `changed` lists the tasks that would change.
    /// Each task is saved on its own, as in `set_epic_planned_version`.
    #[allow(clippy::too_many_arguments)]
    pub fn plan_epic_version(
        &self,
        project: &crate::domain::ProjectId,
        epic: &EpicId,
        choice: &VersionChoice,
        scope: EpicVersionScope,
        task_ids: &[i64],
        dry_run: bool,
        now: DateTime<Utc>,
    ) -> Result<EpicVersionPlan, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        read_epic_file(shared, project, epic)?;
        let target = resolve(shared, project, choice)?;
        let mut outcome = EpicVersionPlan {
            result: EpicVersionSet {
                changed: Vec::new(),
                unchanged: Vec::new(),
                skipped: Vec::new(),
            },
            skipped_reasons: Vec::new(),
        };
        let mut wanted: Vec<i64> = task_ids.to_vec();
        wanted.sort_unstable();
        wanted.dedup();
        // Read everything first so an unknown ID writes nothing.
        let mut tasks = Vec::new();
        for task_id in wanted {
            let task =
                reader::read_task_strict(shared, project, task_id).map_err(
                    |error| match error {
                        AgpjError::TaskNotFound { .. } => invalid_task(task_id),
                        other => other,
                    },
                )?;
            if task.epic_id.as_deref() != Some(epic.as_str()) {
                return Err(invalid_task(task_id));
            }
            tasks.push(task);
        }
        for mut task in tasks {
            let task_id = task.id;
            let skip = if task.archived_at.is_some() {
                Some(SkipReason::Archived)
            } else if task.task_status == Column::Done {
                Some(SkipReason::Done)
            } else if scope == EpicVersionScope::UnplannedOnly && task.planned_version.is_some() {
                Some(SkipReason::Planned)
            } else {
                None
            };
            if let Some(reason) = skip {
                outcome.result.skipped.push(task_id);
                outcome.skipped_reasons.push(reason);
                continue;
            }
            if task.planned_version == target {
                outcome.result.unchanged.push(task_id);
            } else {
                if !dry_run {
                    self.apply_version(project, &mut task, target.clone(), now)?;
                    save_task(shared, &project_dir, &task)?;
                }
                outcome.result.changed.push(task_id);
            }
        }
        Ok(outcome)
    }
}

fn invalid_task(task_id: i64) -> AgpjError {
    AgpjError::InvalidValue {
        field: "taskIds".into(),
        reason: format!("Task #{task_id} is not a task of this epic."),
    }
}
