//! The task as callers receive it: the view the readers return, with its epic and planned
//! results resolved.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::{Choice, HistoryRecord};
use crate::domain::{Assignee, Attachment, Column, Draft, RunState, TaskResult};
use crate::file_store::frontmatter::{AcceptanceItem, ResultPlan};
use crate::file_store::onboarding_model::OnboardingSample;
use crate::file_store::validate::EpicId;

/// The epic a task belongs to, resolved from the epic's own file. Renaming or
/// recoloring the epic changes this; the task file keeps only the ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEpic {
    pub id: EpicId,
    pub name: String,
    pub color: String,
}

/// A planned result as `show` and the API return it: the plan as written in `TASK.md`, and the
/// version of the first saved result that carries its ID (`null` while it is undelivered). The
/// delivered state is derived from the results; nothing is stored in the plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultPlanView {
    #[serde(flatten)]
    pub plan: ResultPlan,
    pub delivered_result_version: Option<u32>,
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
    pub result_plans: Vec<ResultPlanView>,
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
    /// The agent (`A-<n>`) that owns the task; `None` when nobody does.
    pub agent_id: Option<String>,
    /// Counts up at every claim and assignment.
    pub assignment_generation: u32,
    /// The connection that set the task running; `None` while it is not running.
    pub run_connection_id: Option<String>,
    pub external_edit: bool,
    pub history: Vec<HistoryRecord>,
}
