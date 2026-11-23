//! The epic and task entries of the progress table.

use std::path::PathBuf;

use serde::Serialize;

use crate::domain::{Assignee, Column, RunState};

use super::Count;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEpic {
    pub id: String,
    pub name: String,
    pub document_path: PathBuf,
    /// The `waves` list of EPIC.md cannot be read; the tasks keep ID order.
    pub waves_error: bool,
    pub tasks: Vec<ProgressTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressTask {
    pub id: i64,
    pub title: String,
    pub assignee: Assignee,
    pub task_status: Column,
    pub agent_status: RunState,
    pub draft: Option<crate::domain::Draft>,
    /// The version the task is planned for; left out when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_version: Option<String>,
    pub task_path: PathBuf,
    pub acceptance: Count,
    pub acceptance_error: bool,
    /// The waiting question, shortened. Only on a task waiting for a decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// The time of the latest history entry, or of the last run-state change when there is none.
    /// Only on a running AI task, for the stale-running reminder.
    #[serde(
        with = "crate::clock::serde_opt_utc",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
    /// A final result is recorded. Only on an AI task in Doing; left out when false.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub has_final_result: bool,
    /// The result plans not yet delivered, in plan order. Only on an AI task in Todo or Doing;
    /// left out when none is undelivered (or the task has no plans).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub result_plans: Vec<ProgressResultPlan>,
    /// The time of the latest history entry. Not printed: the `lane-review` rule reads it.
    #[serde(skip)]
    pub last_history_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// An undelivered result plan as the progress table shows it: what to deliver, and when.
/// `content` and `condition` are cut to a short length; `show` has the full text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressResultPlan {
    pub id: String,
    pub content: String,
    pub condition: String,
    #[serde(rename = "final")]
    pub is_final: bool,
}

/// An epic whose tasks are all Done while its `EPIC.md` still has an unchecked reference
/// condition (`- [ ]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicToClose {
    pub id: String,
    pub name: String,
    pub document_path: PathBuf,
}

/// Extra sections of the table. The default adds nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Include {
    /// Add the tasks waiting for a decision, with their questions.
    pub awaiting_answer: bool,
    /// Add this many of the most recently completed tasks of epics that still
    /// have Todo or Doing work; 0 adds none.
    pub done: usize,
}
