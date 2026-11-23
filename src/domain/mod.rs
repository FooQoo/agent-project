//! Pure domain types and rules. No I/O.

mod column;
mod limits;
pub mod markdown_blocks;
mod plain_title;
mod project;
mod task;
mod version;
mod work;

pub use column::{Column, RunState};
pub use limits::{
    MAX_ANSWER_TEXT_CHARS, MAX_ATTACHMENT_BYTES, MAX_CHOICE_ID_CHARS, MAX_CHOICE_LABEL_CHARS,
    MAX_CHOICES, MAX_QUESTION_CHARS, MAX_TASK_ATTACHMENT_BYTES, MAX_TASK_ATTACHMENTS,
    MAX_TITLE_CHARS, MIN_CHOICES, normalize_and_validate_description, validate_attachment,
    validate_board_name, validate_project_name, validate_reason, validate_title,
};
pub use plain_title::{first_line_plain, strip_inline_markdown};
pub use project::{ChangeCursor, ProjectCatalog, ProjectId, ProjectMeta};
pub use task::BoardMeta;
pub use version::{
    MAX_VERSION_CHARS, MAX_VERSIONS, VersionFilter, compare_versions, validate_version_name,
};
pub use work::{
    Actor, Assignee, Attachment, AttachmentData, AttachmentInput, BulkArchiveResult, ChangeContext,
    Changed, Draft, HistoryEvent, MAX_RESULT_TITLE_CHARS, NewTask, ResultKind, Task, TaskResult,
    ValidatedAttachment, ValidatedBoard, derive_result_title,
};

const WORK_REASON_MAX: usize = 10_000;

/// Normalize line endings, trim the outer boundary, and validate a work-history reason.
pub fn validate_work_reason(value: &str) -> Result<String, crate::error::AgpjError> {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim();
    let count = value.chars().count();
    if trimmed.is_empty() || count > WORK_REASON_MAX {
        return Err(crate::error::AgpjError::InvalidValue {
            field: "reason".into(),
            reason: "Reasons must be 1 to 10000 characters.".into(),
        });
    }
    Ok(trimmed.to_owned())
}

/// Which run states a task may take in a column. Only an AI task in Doing can be running or
/// paused; every other task is idle.
pub fn check_work_state(
    assignee: Assignee,
    column: Column,
    state: RunState,
) -> Result<(), crate::error::AgpjError> {
    let allowed = state == RunState::Idle || (assignee == Assignee::Ai && column == Column::Doing);
    if allowed {
        Ok(())
    } else {
        Err(crate::error::AgpjError::InvalidStateForAssignee)
    }
}

/// The only changes of the run state an agent or the web API may ask for, all in Doing:
/// idle to running, running to paused, and paused to running. Anything else is refused, so a
/// task that has run never goes back to idle. Asking for the state a task is in is no change.
pub fn check_state_change(from: RunState, to: RunState) -> Result<(), crate::error::AgpjError> {
    if matches!(
        (from, to),
        (RunState::Idle, RunState::Running)
            | (RunState::Running, RunState::Paused)
            | (RunState::Paused, RunState::Running)
    ) {
        Ok(())
    } else {
        Err(crate::error::AgpjError::InvalidStateChange { from, to })
    }
}
