//! Task shapes: the in-memory aggregate, the `task.json` metadata and the
//! view that the readers return.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Choice, HistoryRecord, RequestAnswer};
use crate::domain::{Assignee, Attachment, AttachmentInput, Column, Draft, RunState, TaskResult};
use crate::file_store::frontmatter::{AcceptanceItem, ResultPlan};
use crate::file_store::onboarding_model::OnboardingSample;
use crate::file_store::validate::EpicId;

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
            history,
        }
    }
}

/// The epic a task belongs to, resolved from the epic's own file. Renaming or
/// recoloring the epic changes this; the task file keeps only the ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEpic {
    pub id: EpicId,
    pub name: String,
    pub color: String,
}

/// A task as callers receive it: `task.json` plus `TASK.md`, with the epic resolved from its
/// file and the run state derived from the questions it refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub id: i64,
    pub title: String,
    pub description: String,
    /// TASK.md without its frontmatter: the text after the closing line, or the whole file when there is no closing line.
    pub body: String,
    pub acceptance: Vec<AcceptanceItem>,
    pub acceptance_error: bool,
    /// The planned results from the `resultPlans` frontmatter (empty when none or unreadable).
    pub result_plans: Vec<ResultPlan>,
    pub result_plans_error: bool,
    pub assignee: Assignee,
    pub epic_id: Option<String>,
    pub epic: Option<TaskEpic>,
    pub task_status: Column,
    pub position: i64,
    /// The stored run state: idle, running, or paused (running and paused only for an AI task in
    /// Doing). Waiting for an answer is the separate `awaiting_answer` flag.
    pub agent_status: RunState,
    pub draft: Option<Draft>,
    /// The question IDs in the `questions` frontmatter.
    pub question_ids: Vec<String>,
    /// The `questions` frontmatter cannot be read; the task then counts as waiting.
    pub questions_error: bool,
    /// The referenced questions that are still open, lowest number first.
    pub open_question_ids: Vec<String>,
    /// The task refers to an open question (or its `questions` cannot be read), whatever its
    /// assignee, lane, or run state; never true for an archived task.
    pub awaiting_answer: bool,
    /// The request, question and choices of the first open question while the task waits.
    pub pending_request_id: Option<String>,
    pub pending_reason: Option<String>,
    pub pending_choices: Vec<Choice>,
    #[serde(with = "crate::clock::serde_opt_utc")]
    pub archived_at: Option<DateTime<Utc>>,
    pub attachments: Vec<Attachment>,
    /// The results delivered to the requester, oldest first (`[]` when none).
    pub results: Vec<TaskResult>,
    #[serde(with = "crate::clock::serde_utc")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub updated_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub agent_status_changed_at: DateTime<Utc>,
    #[serde(with = "crate::clock::serde_utc")]
    pub task_status_changed_at: DateTime<Utc>,
    /// The guide's sample identity; absent for every ordinary card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onboarding_sample: Option<OnboardingSample>,
    /// The `plannedVersion` of TASK.md, shown as written; absent when the task has no plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_version: Option<String>,
    pub history: Vec<HistoryRecord>,
}
