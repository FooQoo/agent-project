//! State rules about answers, requests, conditions, epics, running tasks, and drafts. Each
//! reads only the progress table and the clock.

use chrono::Duration;

use super::{Facts, Finding, Lang};
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
            Finding::new(match facts.lang {
                Lang::En => format!(
                    "Answer {id} (\"{title}\") is not taken in yet: read it with `agpj message show {id}`, act on it, then run `agpj message accept {id}`.",
                    id = a.id,
                    title = a.title
                ),
                Lang::Ja => format!(
                    "回答 {id}(「{title}」)をまだ取り込んでいません。`agpj message show {id}` で読み、内容に沿って対応してから、`agpj message accept {id}` を実行してください。",
                    id = a.id,
                    title = a.title
                ),
            })
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
            Finding::new(match facts.lang {
                Lang::En => format!(
                    "Request {id} (\"{title}\") is open: read it with `agpj message show {id}`, accept it with `agpj message accept {id}`, and plan it.",
                    id = r.id,
                    title = r.title
                ),
                Lang::Ja => format!(
                    "依頼 {id}(「{title}」)が未対応です。`agpj message show {id}` で読み、`agpj message accept {id}` で受け付けて、計画を立ててください。",
                    id = r.id,
                    title = r.title
                ),
            })
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
            Finding::new(match facts.lang {
                Lang::En => format!(
                    "Task #{id} has a final result but {done} of {total} acceptance conditions are done: record the verification and set the conditions done in its TASK.md.",
                    id = t.id,
                    done = t.acceptance.done,
                    total = t.acceptance.total
                ),
                Lang::Ja => format!(
                    "タスク #{id} は最終成果がありますが、受け入れ条件は {total} 件中 {done} 件しか完了していません。検証内容を記録し、TASK.md の条件を完了にしてください。",
                    id = t.id,
                    done = t.acceptance.done,
                    total = t.acceptance.total
                ),
            })
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
            Finding::new(match facts.lang {
                Lang::En => format!(
                    "Epic {id} has every task Done but unchecked reference conditions in its EPIC.md ({path}): verify them and record the outcome.",
                    id = e.id,
                    path = e.document_path.display()
                ),
                Lang::Ja => format!(
                    "エピック {id} はすべてのタスクが Done ですが、EPIC.md({path})に未チェックの参照条件が残っています。確認して、結果を記録してください。",
                    id = e.id,
                    path = e.document_path.display()
                ),
            })
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
                Finding::new(match facts.lang {
                    Lang::En => format!(
                        "Task #{id} has been running for {minutes} minutes with no history entry: if you are actually working on it, continue (check its PR and CI state, and record progress with `agpj history add {id} --reason \"<progress>\"`; if you are blocked on a person, ask with `agpj message ask`, which pauses the task). If you are not working on it, pause it with `agpj state {id} paused --actor ai --reason \"<why you stopped>\"`.",
                        id = t.id,
                        minutes = idle.num_minutes()
                    ),
                    Lang::Ja => format!(
                        "タスク #{id} は {minutes} 分間、履歴の記録がないまま実行中です。実際に作業しているなら続けてください(PR と CI の状態を確かめ、`agpj history add {id} --reason \"<進捗>\"` で進捗を記録します。人の判断待ちなら、`agpj message ask` で質問すると、タスクは一時停止になります)。作業していないなら、`agpj state {id} paused --actor ai --reason \"<止めた理由>\"` で一時停止にしてください。",
                        id = t.id,
                        minutes = idle.num_minutes()
                    ),
                })
                .task(t.id)
            })
        })
        .collect()
}

/// No card has moved between lanes for a while, and Doing holds a card nothing is working on or
/// Todo holds an AI card. Backlog never counts. A card with a history entry later than the last
/// lane move counts as reviewed and is left out until the next lane move.
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
            .filter(|t| {
                t.draft.is_none() && t.last_history_at.is_none_or(|at| at <= last_move) && keep(t)
            })
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
    let ja = facts.lang == Lang::Ja;
    let mut text = if ja {
        format!(
            "{LANE_REVIEW_MINUTES} 分間、どのカードもレーン間を移動していません。ボードを見直してください。"
        )
    } else {
        format!(
            "No card has moved between lanes for {LANE_REVIEW_MINUTES} minutes. Review the board."
        )
    };
    if !parked.is_empty() {
        text.push_str(&if ja {
            format!(
                " Doing に、誰も作業していないカードがあります: {parked}。続けられるものは再開してください。順番待ちのカード(たとえばリリース待ち)は、`agpj message ask` で Todo に戻してよいか本人に確認してください(エージェントは Doing から Todo へ移せません)。完了条件を満たしているカードは、完了を確認してください。"
            )
        } else {
            format!(
                " Doing holds cards nothing is working on: {parked}. Resume the ones that can continue; for a card that is waiting its turn (for example, for a release), ask the person with `agpj message ask` whether to send it back to Todo (an agent cannot move Doing to Todo); for a card whose completion conditions are met, confirm its completion."
            )
        });
    }
    if !queued.is_empty() {
        text.push_str(&if ja {
            format!(
                " Todo に AI のカードがあります: {queued}。準備ができているものは、作業ループとエピックの順序に従って着手してください。順番待ちのカード(たとえばリリース待ち)は Todo に残し、理由がまだ記録されていなければ `agpj history add` で記録してください。"
            )
        } else {
            format!(
                " Todo holds AI cards: {queued}. Start the ones that are ready, following the work loop and the epic order; a card that is waiting its turn (for example, for a release) stays in Todo, with the reason recorded by `agpj history add` if it is not recorded yet."
            )
        });
    }
    text.push_str(facts.lang.pick(
        " Recording the decision with `agpj history add` keeps a card out of this reminder until the next lane move.",
        " 判断を `agpj history add` で記録すると、次にレーン間の移動があるまで、そのカードはこの通知に出なくなります。",
    ));
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
        Finding::new(match facts.lang {
            Lang::En => format!(
                "No AI task is running and task #{id} is next: pick it up (read its epic and `TASK.md`, then `mv <id> doing --actor ai` and `state <id> running --actor ai`; a plan in the Backlog is never next).",
                id = next.id
            ),
            Lang::Ja => format!(
                "実行中の AI タスクがなく、次はタスク #{id} です。着手してください(エピックと `TASK.md` を読み、`mv <id> doing --actor ai` と `state <id> running --actor ai` を実行します。Backlog の計画が次になることはありません)。",
                id = next.id
            ),
        })
        .task(next.id),
    ]
}
