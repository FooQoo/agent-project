//! State rules about answers, requests, conditions, epics, running tasks, and drafts. Each
//! reads only the progress table and the clock.

use chrono::Duration;

use super::{Facts, Finding};
use crate::domain::{Assignee, Column, RunState};
use crate::progress::{Progress, ProgressTask};

/// A running task with no history for this long may have stopped.
pub use crate::progress::STALE_RUNNING_MINUTES;

/// No task has changed lane for this long: the Doing lane is due for a review.
pub const LANE_REVIEW_MINUTES: i64 = 30;

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

/// No card has moved between lanes for a while, and Doing holds a card nothing is working on or
/// Todo holds an AI card. Backlog never counts.
pub fn lane_review(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    let Some(last_move) = progress.last_lane_move_at else {
        return Vec::new();
    };
    if facts.now - last_move <= Duration::minutes(LANE_REVIEW_MINUTES) {
        return Vec::new();
    }
    let ids = |keep: &dyn Fn(&ProgressTask) -> bool| -> String {
        let mut ids: Vec<i64> = tasks(progress)
            .filter(|t| t.draft.is_none() && keep(t))
            .map(|t| t.id)
            .collect();
        ids.sort_unstable();
        ids.iter()
            .map(|id| format!("#{id}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let parked = ids(&|t| {
        t.task_status == Column::Doing
            && (t.assignee == Assignee::Human || t.agent_status != RunState::Running)
    });
    let queued = ids(&|t| t.task_status == Column::Todo && t.assignee == Assignee::Ai);
    if parked.is_empty() && queued.is_empty() {
        return Vec::new();
    }
    let mut text = format!(
        "No card has moved between lanes for {LANE_REVIEW_MINUTES} minutes. Review the board."
    );
    if !parked.is_empty() {
        text.push_str(&format!(
            " Doing holds cards nothing is working on: {parked}. Resume the ones that can continue; for a card that is waiting its turn (for example, for a release), ask the person with `agpj message ask` whether to send it back to Todo (an agent cannot move Doing to Todo); for a card whose completion conditions are met, confirm its completion."
        ));
    }
    if !queued.is_empty() {
        text.push_str(&format!(
            " Todo holds AI cards: {queued}. Start the ones that are ready, following the work loop and the epic order; a card that is waiting its turn (for example, for a release) stays in Todo, with the reason recorded by `agpj history add` if it is not recorded yet."
        ));
    }
    vec![Finding::new(text)]
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
