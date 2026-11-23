//! State rules about answers, requests, conditions, epics, running tasks, and drafts. Each
//! reads only the progress table and the clock.

use chrono::Duration;

use super::{Facts, Finding};
use crate::domain::{Assignee, Column, RunState};
use crate::progress::{Progress, ProgressTask};

/// A running task with no history for this long may have stopped.
pub use crate::progress::STALE_RUNNING_MINUTES;

fn tasks(progress: &Progress) -> impl Iterator<Item = &ProgressTask> {
    progress
        .epics
        .iter()
        .flat_map(|e| e.tasks.iter())
        .chain(progress.unassigned.iter())
}

fn ai_tasks(progress: &Progress) -> impl Iterator<Item = &ProgressTask> {
    tasks(progress).filter(|t| t.assignee == Assignee::Ai)
}

/// An answer no agent has taken in.
pub fn answer(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    progress
        .answers
        .iter()
        .map(|a| {
            Finding::new(format!(
                "Answer {id} (\"{title}\") is not taken in yet: read it with `agpj message show {id}`, act on it, then run `agpj message accept {id}`.",
                id = a.id,
                title = a.title
            ))
            .message(a.id.clone())
        })
        .collect()
}

/// A request that is open.
pub fn request(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    progress
        .requests
        .iter()
        .map(|r| {
            Finding::new(format!(
                "Request {id} (\"{title}\") is open: read it with `agpj message show {id}`, accept it with `agpj message accept {id}`, and plan it.",
                id = r.id,
                title = r.title
            ))
            .message(r.id.clone())
        })
        .collect()
}

/// A task in Doing has a final result but not every condition is done.
pub fn conditions(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    ai_tasks(progress)
        .filter(|t| {
            t.draft.is_none()
                && t.task_status == Column::Doing
                && t.has_final_result
                && !t.acceptance_error
                && t.acceptance.done < t.acceptance.total
        })
        .map(|t| {
            Finding::new(format!(
                "Task #{id} has a final result but {done} of {total} acceptance conditions are done: record the verification and set the conditions done in its TASK.md.",
                id = t.id,
                done = t.acceptance.done,
                total = t.acceptance.total
            ))
            .task(t.id)
        })
        .collect()
}

/// An epic has every task Done and an unchecked reference condition.
pub fn epic_close(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    progress
        .epics_to_close
        .iter()
        .map(|e| {
            Finding::new(format!(
                "Epic {id} has every task Done but unchecked reference conditions in its EPIC.md ({path}): verify them and record the outcome.",
                id = e.id,
                path = e.document_path.display()
            ))
            .epic(e.id.clone())
        })
        .collect()
}

/// An AI task has been running with no history for a long time.
pub fn stale_running(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    let limit = Duration::minutes(STALE_RUNNING_MINUTES);
    ai_tasks(progress)
        .filter(|t| t.draft.is_none() && t.agent_status == RunState::Running)
        .filter_map(|t| {
            let idle = facts.now - t.last_activity_at?;
            (idle > limit).then(|| {
                Finding::new(format!(
                    "Task #{id} has been running for {minutes} minutes with no history entry: continue the work on it (check its PR and CI state and carry on; if you are blocked on a person, ask with `agpj message ask`, which pauses the task). If you truly cannot resume it, record why with `agpj history add {id} --reason \"<why it cannot resume>\"`.",
                    id = t.id,
                    minutes = idle.num_minutes()
                ))
                .task(t.id)
            })
        })
        .collect()
}

/// There is a next task and no AI task is running.
pub fn next_task(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    let running = ai_tasks(progress).any(|t| t.agent_status == RunState::Running);
    // A stalled task is surfaced by the stale-running reminder.
    let next = progress
        .next_tasks
        .iter()
        .find(|t| t.why != crate::progress::NextReason::Stalled);
    let Some(next) = next.filter(|_| !running) else {
        return Vec::new();
    };
    vec![
        Finding::new(format!(
            "No AI task is running and task #{id} is next: pick it up (read its epic and `TASK.md`, then `mv <id> doing --actor ai` and `state <id> running --actor ai`; a plan in the Backlog is never next).",
            id = next.id
        ))
        .task(next.id),
    ]
}
