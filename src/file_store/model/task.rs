//! Task shapes: the in-memory aggregate, the `task.json` metadata and the
//! view that the readers return.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{HistoryRecord, RequestAnswer};
use crate::domain::{Assignee, Attachment, AttachmentInput, Column, Draft, RunState, TaskResult};
use crate::file_store::onboarding_model::OnboardingSample;

/// In-memory task aggregate. Metadata is in `task.json`; text and history are in TASK.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskFile {
    pub format_version: u32,
    pub id: i64,
    pub title: String,
    pub assignee: Assignee,
    pub epic_id: Option<String>,
    pub task_status: Column,
    pub agent_status: RunState,
    pub position: i64,
    pub draft: Option<Draft>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub archived_at: Option<DateTime<Utc>>,
    /// Answers to decision requests of earlier versions. Kept as a record; never added to.
    pub request_answers: Vec<RequestAnswer>,
    #[serde(with = "crate::clock::serde_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub updated_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub agent_status_changed_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub task_status_changed_at: DateTime<Utc>,
    pub attachments: Vec<Attachment>,
    /// Results delivered to the requester, oldest first.
    #[serde(default)]
    pub results: Vec<TaskResult>,
    /// Legacy request metadata retained when reading existing task files.
    /// New simple requests do not write or use this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simple_request: Option<SimpleRequestMetadata>,
    /// The guide's sample identity; absent for every ordinary card.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onboarding_sample: Option<OnboardingSample>,
    /// The version whose release notes this task writes; set only on the card that creating the
    /// version issued, so creating it again finds the card and issues no second one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes_version: Option<String>,
    /// When the release-work card first reached Done. Only that first completion can move the
    /// release version; a later Done never does.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::clock::serde_opt_utc"
    )]
    pub release_completed_at: Option<DateTime<Utc>>,
    /// The version the task is planned for; `None` when it has no plan. It lives in the
    /// `plannedVersion` key of TASK.md, not in `task.json`.
    #[serde(default)]
    pub planned_version: Option<String>,
    /// The agent (`A-<n>`) that owns the task; `None` for an unassigned card and for every card
    /// of a person.
    pub agent_id: Option<String>,
    /// Counts up from 1 at every claim, assignment, and handover.
    pub assignment_generation: u32,
    /// The connection that set the task running; `None` while it is not running.
    pub run_connection_id: Option<String>,
    pub history: Vec<HistoryRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SimpleRequestMetadata {
    pub request_id: String,
    pub payload_hash: String,
}

/// A submitted simple request for atomic FileStore creation.
#[derive(Debug, Clone)]
pub struct SimpleRequestInput {
    pub title: String,
    pub body: String,
    pub epic_id: Option<String>,
    pub images: Vec<AttachmentInput>,
}

/// `task.json`: operational metadata. History is stored only in TASK.md.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskMetadata {
    pub format_version: u32,
    pub id: i64,
    pub title: String,
    pub assignee: Assignee,
    pub epic_id: Option<String>,
    pub task_status: Column,
    pub agent_status: RunState,
    pub position: i64,
    /// Files of earlier versions hold `isDraft`, read as `draft`.
    #[serde(
        default,
        alias = "isDraft",
        deserialize_with = "Draft::deserialize_stored"
    )]
    pub draft: Option<Draft>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub archived_at: Option<DateTime<Utc>>,
    /// The decision request of earlier versions. Read and ignored; never written again.
    #[serde(default, skip_serializing)]
    pub pending_request_id: Option<serde_json::Value>,
    /// The question of that request. Read and ignored; never written again.
    #[serde(default, skip_serializing)]
    pub pending_request: Option<serde_json::Value>,
    #[serde(default)]
    pub request_answers: Vec<RequestAnswer>,
    #[serde(with = "crate::clock::serde_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub updated_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub agent_status_changed_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub task_status_changed_at: DateTime<Utc>,
    pub attachments: Vec<Attachment>,
    /// Absent from files written before results existed, and while there are none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<TaskResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simple_request: Option<SimpleRequestMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onboarding_sample: Option<OnboardingSample>,
    /// See `TaskFile::release_notes_version`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes_version: Option<String>,
    /// See `TaskFile::release_completed_at`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "crate::clock::serde_opt_utc"
    )]
    pub release_completed_at: Option<DateTime<Utc>>,
    /// The planned version that an earlier build kept here. Read and ignored; never written
    /// again: it is the `plannedVersion` key of TASK.md.
    #[serde(default, skip_serializing)]
    pub planned_version: Option<serde_json::Value>,
    /// The changes of that field. Read and ignored; never written again.
    #[serde(default, skip_serializing)]
    pub planned_version_changes: Option<serde_json::Value>,
    /// See `TaskFile::agent_id`. Always written, `null` when there is none.
    pub agent_id: Option<String>,
    /// See `TaskFile::assignment_generation`.
    pub assignment_generation: u32,
    /// See `TaskFile::run_connection_id`.
    pub run_connection_id: Option<String>,
}

impl From<&TaskFile> for TaskMetadata {
    fn from(task: &TaskFile) -> Self {
        Self {
            format_version: crate::file_store::layout::entity_format_version(
                task.format_version,
                task.task_status == Column::Backlog,
            ),
            id: task.id,
            title: task.title.clone(),
            assignee: task.assignee,
            epic_id: task.epic_id.clone(),
            task_status: task.task_status,
            agent_status: task.agent_status,
            position: task.position,
            draft: task.draft,
            archived_at: task.archived_at,
            pending_request_id: None,
            pending_request: None,
            request_answers: task.request_answers.clone(),
            created_at: task.created_at,
            updated_at: task.updated_at,
            agent_status_changed_at: task.agent_status_changed_at,
            task_status_changed_at: task.task_status_changed_at,
            attachments: task.attachments.clone(),
            results: task.results.clone(),
            simple_request: task.simple_request.clone(),
            onboarding_sample: task.onboarding_sample.clone(),
            release_notes_version: task.release_notes_version.clone(),
            release_completed_at: task.release_completed_at,
            planned_version: None,
            planned_version_changes: None,
            agent_id: task.agent_id.clone(),
            assignment_generation: task.assignment_generation,
            run_connection_id: task.run_connection_id.clone(),
        }
    }
}

impl TaskMetadata {
    /// Join persisted metadata with validated TASK.md events for operations.
    pub fn with_history(self, history: Vec<HistoryRecord>) -> TaskFile {
        TaskFile {
            format_version: self.format_version,
            id: self.id,
            title: self.title,
            assignee: self.assignee,
            epic_id: self.epic_id,
            task_status: self.task_status,
            agent_status: self.agent_status,
            position: self.position,
            draft: self.draft,
            archived_at: self.archived_at,
            request_answers: self.request_answers,
            created_at: self.created_at,
            updated_at: self.updated_at,
            agent_status_changed_at: self.agent_status_changed_at,
            task_status_changed_at: self.task_status_changed_at,
            attachments: self.attachments,
            results: self.results,
            simple_request: self.simple_request,
            onboarding_sample: self.onboarding_sample,
            release_notes_version: self.release_notes_version,
            release_completed_at: self.release_completed_at,
            planned_version: None,
            agent_id: self.agent_id,
            assignment_generation: self.assignment_generation,
            run_connection_id: self.run_connection_id,
            history,
        }
    }
}
