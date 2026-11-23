//! One task of the progress table.

use super::{ProgressResultPlan, ProgressTask, count, is_waiting, text};
use crate::domain::{Assignee, Column, ResultKind, RunState};
use crate::file_store::layout;
use crate::file_store::model::TaskView;

pub(super) fn task_of(task: &TaskView, project_dir: &std::path::Path) -> ProgressTask {
    ProgressTask {
        id: task.id,
        title: task.title.clone(),
        assignee: task.assignee,
        task_status: task.task_status,
        agent_status: task.agent_status,
        draft: task.draft,
        planned_version: task.planned_version.clone(),
        task_path: layout::task_md(&layout::task_dir(project_dir, task.id)),
        acceptance: count::count(&task.acceptance),
        acceptance_error: task.acceptance_error,
        last_activity_at: (task.assignee == Assignee::Ai && task.agent_status == RunState::Running)
            .then(|| {
                task.history
                    .iter()
                    .map(|entry| entry.at)
                    .max()
                    .map_or(task.agent_status_changed_at, |at| {
                        at.max(task.agent_status_changed_at)
                    })
            }),
        result_plans: undelivered_plans(task),
        last_history_at: task.history.iter().map(|entry| entry.at).max(),
        has_final_result: task.assignee == Assignee::Ai
            && task.task_status == Column::Doing
            && task.results.iter().any(|r| r.kind == ResultKind::Final),
        question: if is_waiting(task) {
            task.pending_reason.as_deref().map(text::short_question)
        } else {
            None
        },
    }
}

/// The plans of an AI task in Todo or Doing that have no delivered result yet. A task whose
/// plans cannot be read has none to show.
fn undelivered_plans(task: &TaskView) -> Vec<ProgressResultPlan> {
    if task.assignee != Assignee::Ai
        || !matches!(task.task_status, Column::Todo | Column::Doing)
        || task.result_plans_error
    {
        return Vec::new();
    }
    task.result_plans
        .iter()
        .filter(|view| view.delivered_result_version.is_none())
        .map(|view| ProgressResultPlan {
            id: view.plan.id.clone(),
            content: text::short_plan(&view.plan.content),
            condition: text::short_plan(&view.plan.condition),
            is_final: view.plan.is_final,
        })
        .collect()
}
