//! Creating a task.

use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::domain::{Column, NewTask, ProjectId, RunState, validate_title};
use crate::error::AgpjError;
use crate::file_store::FileStore;
use crate::file_store::frontmatter;
use crate::file_store::history::{ParseMode, compose_task, parse_task};
use crate::file_store::history_language::HistoryLanguage;
use crate::file_store::io;
use crate::file_store::layout;
use crate::file_store::model::{TaskFile, TaskMetadata, TaskView};
use crate::file_store::reader;
use crate::file_store::validate;

use super::super::{
    HistoryKind, Transition, lane_end, lock_project, save_project, stamp, transition_event,
};
use super::{normalize_description, view};

/// Write a new task folder: `task.json` and `TASK.md` go into a temporary
/// folder that is renamed into place. A failure removes the temporary folder.
fn write_new_task(
    shared: &Path,
    project_dir: &Path,
    task: &TaskFile,
    body: &str,
) -> Result<(), AgpjError> {
    let tasks = layout::tasks_dir(project_dir);
    let target = layout::task_dir(project_dir, task.id);
    let label = layout::shared_label(shared, &target);
    if target.exists() {
        return Err(io::write_error(&label, "the task folder already exists"));
    }
    let temp = tasks.join(format!(
        ".new-{}",
        io::random_hex(16).map_err(|e| io::write_error(&label, e))?
    ));
    fs::create_dir(&temp).map_err(|e| io::write_error(&label, e))?;
    let result = (|| -> Result<(), AgpjError> {
        let parsed = parse_task(body, ParseMode::NewInput);
        if let Some(defect) = parsed.history_defects.first() {
            return Err(AgpjError::FileInvalid {
                file: format!("{label}/TASK.md"),
                field: defect.field.clone(),
                reason: defect.reason.clone(),
            });
        }
        let body = compose_task(body, &parsed.body, None, &task.history).map_err(|defect| {
            AgpjError::FileInvalid {
                file: format!("{label}/TASK.md"),
                field: defect.field,
                reason: defect.reason,
            }
        })?;
        // Result plans without an ID get one before the file is written.
        let body = frontmatter::normalize_result_plans(&body).map_err(|defect| {
            AgpjError::FileInvalid {
                file: format!("{label}/TASK.md"),
                field: defect.field,
                reason: defect.reason,
            }
        })?;
        // The plan lives in the TASK.md frontmatter, so a task created with one writes the key.
        let body =
            match task.planned_version.as_deref() {
                Some(version) => frontmatter::compose_planned_version(&body, Some(version))
                    .map_err(|defect| AgpjError::FileInvalid {
                        file: format!("{label}/TASK.md"),
                        field: defect.field,
                        reason: defect.reason,
                    })?,
                None => body,
            };
        let mut metadata = TaskMetadata::from(task);
        metadata.content_digest = Some(super::super::save::content_digest(body.as_bytes()));
        io::write_json(
            &layout::task_json(&temp),
            &metadata,
            &format!("{label}/task.json"),
        )?;
        io::write_atomic(
            &layout::task_md(&temp),
            body.as_bytes(),
            &format!("{label}/TASK.md"),
        )?;
        fs::rename(&temp, &target).map_err(|e| io::write_error(&label, e))?;
        io::sync_dir(&tasks).map_err(|e| io::write_error(&label, e))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temp);
    }
    result
}

impl FileStore {
    /// Create a task at the end of its lane: Todo, or the Backlog for a task an
    /// agent creates (which must say how far the work is handed over). The number is reserved in
    /// `project.json` before the task folder is written, so a failed write
    /// leaves a gap in the numbers and never reuses one. A body that is not
    /// given (empty or blank) gets the TASK.md template.
    pub fn create_task(
        &self,
        project: &ProjectId,
        new: NewTask,
        now: DateTime<Utc>,
    ) -> Result<TaskView, AgpjError> {
        self.create_task_in(project, new, Column::Todo, now)
    }

    /// Create a task at the end of the Backlog: a plan that must not be started
    /// until a person sends it to Todo.
    pub fn create_task_in_backlog(
        &self,
        project: &ProjectId,
        new: NewTask,
        now: DateTime<Utc>,
    ) -> Result<TaskView, AgpjError> {
        self.create_task_in(project, new, Column::Backlog, now)
    }

    fn create_task_in(
        &self,
        project: &ProjectId,
        new: NewTask,
        lane: Column,
        now: DateTime<Utc>,
    ) -> Result<TaskView, AgpjError> {
        let shared = self.shared_dir();
        let (project_dir, _lock) = lock_project(shared, project)?;
        self.create_task_locked(&project_dir, project, new, lane, None, now)
    }

    /// Create a task while the caller holds the project's write lock. `release_notes_version`
    /// marks the card that creating a version issues; that card is also planned for that version.
    pub(in crate::file_store) fn create_task_locked(
        &self,
        project_dir: &Path,
        project: &ProjectId,
        new: NewTask,
        lane: Column,
        release_notes_version: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<TaskView, AgpjError> {
        let title = validate_title(&new.title)?;
        let description = normalize_description(&new.description)?;
        let shared = self.shared_dir();
        let mut file = reader::read_project_file(shared, project)?;
        let id = file.next_task_id;
        let body = if description.trim().is_empty() {
            validate::task_template()
        } else {
            description
        };
        let label =
            layout::shared_label(shared, &layout::task_md(&layout::task_dir(project_dir, id)));
        validate::ensure_task_body(&body, &label)?;
        let position = lane_end(shared, project, lane)?;
        let at = stamp(now);
        let language = self
            .history_language
            .unwrap_or_else(|| HistoryLanguage::infer(&body, &title));
        let task = TaskFile {
            format_version: layout::entity_format_version(
                layout::FORMAT_VERSION,
                lane == Column::Backlog,
            ),
            id,
            title,
            assignee: new.assignee,
            epic_id: None,
            task_status: lane,
            agent_status: RunState::Idle,
            position,
            draft: new.draft,
            archived_at: None,
            request_answers: Vec::new(),
            created_at: at,
            updated_at: at,
            agent_status_changed_at: at,
            task_status_changed_at: at,
            attachments: Vec::new(),
            results: Vec::new(),
            simple_request: None,
            onboarding_sample: None,
            planned_version: release_notes_version.clone(),
            release_notes_version,
            agent_id: if new.assignee == crate::domain::Assignee::Ai {
                Some(super::super::assignment::default_agent_id_locked(
                    shared,
                    project_dir,
                    language,
                    now,
                )?)
            } else {
                None
            },
            assignment_generation: 1,
            run_connection_id: None,
            content_digest: None,
            external_edit: false,
            release_completed_at: None,
            history: vec![transition_event(Transition {
                language,
                kind: HistoryKind::Created,
                reason: None,
                from_task: None,
                to_task: lane,
                from_agent: None,
                to_agent: RunState::Idle,
                at,
            })?],
        };
        file.next_task_id = id
            .checked_add(1)
            .ok_or_else(|| io::write_error("project.json", "The task number overflowed."))?;
        save_project(shared, project_dir, &file)?;
        write_new_task(shared, project_dir, &task, &body)?;
        view(shared, project, id)
    }
}
