//! `nextTasks`: the ready AI tasks to pick up next.

use std::path::PathBuf;

use serde::Serialize;

use chrono::{DateTime, Duration, Utc};

use crate::domain::{Assignee, Column, RunState};
use crate::file_store::layout;
use crate::file_store::model::TaskView;
use crate::file_store::reader::ProjectSnapshot;

/// A running task with no history for this long may have stopped; `nextTasks` lists it first.
pub const STALE_RUNNING_MINUTES: i64 = 30;

/// How many tasks `nextTasks` holds at most.
pub const MAX_NEXT_TASKS: usize = 3;

/// Why a task is in `nextTasks`. The values are stable; agents may rely on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NextReason {
    /// A running task in Doing with no history entry for `STALE_RUNNING_MINUTES`: continue it.
    /// Listed before everything else.
    Stalled,
    /// In Doing, idle or paused, and not waiting for an answer: work to resume.
    Resume,
    /// In Todo with an Idle run state: the next ready task in its epic's order.
    NextInEpic,
}

/// A task recommended as the next work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextTask {
    pub id: i64,
    pub title: String,
    /// `null` when the task has no epic.
    pub epic_id: Option<String>,
    pub task_path: PathBuf,
    pub why: NextReason,
    /// The task's run state.
    pub agent_status: RunState,
}

/// A Todo task of an epic that orders its tasks in waves is held back while a task
/// of an earlier wave is not Done. Archived tasks are ignored, tasks that no wave
/// lists are never held back, and an unreadable order gates nothing.
pub(crate) fn gated_by_waves(task: &TaskView, snapshot: &ProjectSnapshot) -> bool {
    let Some(epic) = task
        .epic_id
        .as_deref()
        .and_then(|id| snapshot.epics.iter().find(|epic| epic.id.as_str() == id))
    else {
        return false;
    };
    let Some(position) = epic.waves.iter().position(|wave| wave.contains(&task.id)) else {
        return false;
    };
    let earlier: Vec<i64> = epic.waves[..position].iter().flatten().copied().collect();
    snapshot.tasks.iter().any(|other| {
        earlier.contains(&other.id)
            && other.archived_at.is_none()
            && other.task_status != Column::Done
    })
}

/// Ready AI tasks, resume work before Todo work, each in table order. A Todo task
/// waits for the earlier waves of its epic; other dependencies between tasks are
/// prose in EPIC.md and are not evaluated here.
pub(super) fn next_tasks(
    listed: &[&TaskView],
    snapshot: &ProjectSnapshot,
    project_dir: &std::path::Path,
    now: DateTime<Utc>,
    keep: &dyn Fn(&TaskView) -> bool,
) -> Vec<NextTask> {
    let stalled: Vec<(&TaskView, NextReason)> = listed
        .iter()
        .filter(|task| {
            task.assignee == Assignee::Ai
                && task.task_status == Column::Doing
                && task.agent_status == RunState::Running
                && task.draft.is_none()
                && task.archived_at.is_none()
        })
        .filter(|task| keep(task))
        .filter(|task| {
            let at = super::task_entry::task_of(task, project_dir).last_activity_at;
            at.is_some_and(|at| now - at > Duration::minutes(STALE_RUNNING_MINUTES))
        })
        .map(|task| (*task, NextReason::Stalled))
        .collect();
    let ready = |task: &&&TaskView| {
        task.assignee == Assignee::Ai
            && task.agent_status.can_start()
            && task.draft.is_none()
            && task.archived_at.is_none()
    };
    let pick = |column: Column, why: NextReason| {
        listed
            .iter()
            .filter(ready)
            .filter(move |task| task.task_status == column)
            .filter(move |task| column != Column::Todo || !gated_by_waves(task, snapshot))
            .filter(|task| keep(task))
            .map(move |task| (*task, why))
            .collect::<Vec<_>>()
    };
    stalled
        .into_iter()
        .chain(pick(Column::Doing, NextReason::Resume))
        .chain(pick(Column::Todo, NextReason::NextInEpic))
        .take(MAX_NEXT_TASKS)
        .map(|(task, why)| NextTask {
            id: task.id,
            title: task.title.clone(),
            epic_id: task.epic_id.clone(),
            task_path: layout::task_md(&layout::task_dir(project_dir, task.id)),
            why,
            agent_status: task.agent_status,
        })
        .collect()
}
