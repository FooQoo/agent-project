//! Cases of the state rules over answers, requests, conditions, epics, running tasks, and drafts.

use chrono::{Duration, Utc};

use super::cases::{Case, case};
use super::test_support::{Scenario, progress_with};
use crate::domain::{Column, RunState};
use crate::progress::{EpicToClose, Progress, ProgressAnswer, ProgressRequest, ProgressTask};

/// The table's only task, for a case to change.
fn task(p: &mut Progress) -> &mut ProgressTask {
    p.epics
        .iter_mut()
        .flat_map(|e| e.tasks.iter_mut())
        .chain(p.unassigned.iter_mut())
        .next()
        .unwrap()
}

fn with(change: impl FnOnce(&mut Progress)) -> Scenario {
    let mut p = progress_with(false, &[]);
    change(&mut p);
    Scenario::state(p)
}

fn doing(p: &mut Progress, running: bool) {
    let t = task(p);
    t.task_status = Column::Doing;
    t.agent_status = if running {
        RunState::Running
    } else {
        RunState::Idle
    };
}

fn running_since(minutes: i64) -> Scenario {
    with(|p| {
        doing(p, true);
        task(p).last_activity_at = Some(Utc::now() - Duration::minutes(minutes));
    })
}

/// A table whose last lane move was `minutes` ago, with the table's only task in Doing.
fn doing_after(minutes: i64, running: bool, human: bool) -> Scenario {
    with(|p| {
        doing(p, running);
        if human {
            task(p).assignee = crate::domain::Assignee::Human;
        }
        p.last_lane_move_at = Some(Utc::now() - Duration::minutes(minutes));
    })
}

/// A table whose last lane move was an hour ago, Doing is all running, and the only task is in
/// Todo with the given owner.
fn todo_only(human: bool) -> Scenario {
    with(|p| {
        let t = task(p);
        t.task_status = Column::Todo;
        if human {
            t.assignee = crate::domain::Assignee::Human;
        }
        p.last_lane_move_at = Some(Utc::now() - Duration::minutes(60));
    })
}

fn final_with(done: usize, has_final: bool) -> Scenario {
    with(|p| {
        doing(p, true);
        let t = task(p);
        t.has_final_result = has_final;
        t.acceptance.done = done;
        t.acceptance.total = 2;
    })
}

fn epic(path: &str) -> EpicToClose {
    EpicToClose {
        id: "E1".into(),
        name: "E".into(),
        document_path: path.into(),
    }
}

pub fn all() -> Vec<Case> {
    let at = Utc::now();
    vec![
        case(
            "answer",
            "an answer not taken in",
            true,
            with(|p| {
                p.answers.push(ProgressAnswer {
                    id: "Q-1".into(),
                    title: "t".into(),
                    tasks: vec![],
                    choice_id: None,
                    choice_label: None,
                    text: None,
                    answered_at: at,
                    image_paths: Vec::new(),
                })
            }),
        ),
        case("answer", "no answer", false, with(|_| {})),
        case(
            "request",
            "an open request",
            true,
            with(|p| {
                p.requests.push(ProgressRequest {
                    id: "Q-2".into(),
                    created_at: at,
                    title: "t".into(),
                })
            }),
        ),
        case("request", "no request", false, with(|_| {})),
        case(
            "conditions",
            "a final result and conditions left",
            true,
            final_with(1, true),
        ),
        case(
            "conditions",
            "all conditions done",
            false,
            final_with(2, true),
        ),
        case(
            "conditions",
            "no final result yet",
            false,
            final_with(0, false),
        ),
        case(
            "epic-close",
            "all tasks done with an unchecked condition",
            true,
            with(|p| p.epics_to_close.push(epic("/tmp/EPIC.md"))),
        ),
        case("epic-close", "no epic to close", false, with(|_| {})),
        case(
            "stale-running",
            "running for an hour with no history",
            true,
            running_since(60),
        ),
        case(
            "stale-running",
            "running with a recent entry",
            false,
            running_since(5),
        ),
        case(
            "stale-running",
            "an idle task",
            false,
            with(|p| doing(p, false)),
        ),
        case(
            "lane-review",
            "an idle Doing card and no move for an hour",
            true,
            doing_after(60, false, false),
        ),
        case(
            "lane-review",
            "a human card in Doing and no move for an hour",
            true,
            doing_after(60, true, true),
        ),
        case(
            "lane-review",
            "a move within 30 minutes",
            false,
            doing_after(5, false, false),
        ),
        case(
            "lane-review",
            "every Doing card is running",
            false,
            doing_after(60, true, false),
        ),
        case(
            "lane-review",
            "an AI card in Todo, nothing parked in Doing, and no move for an hour",
            true,
            todo_only(false),
        ),
        case(
            "lane-review",
            "only a human card in Todo and no move for an hour",
            false,
            todo_only(true),
        ),
        case(
            "lane-review",
            "an AI card in Todo and a move within 30 minutes",
            false,
            with(|p| p.last_lane_move_at = Some(Utc::now() - Duration::minutes(5))),
        ),
        case(
            "lane-review",
            "only a Backlog card",
            false,
            with(|p| {
                task(p).task_status = Column::Backlog;
                p.last_lane_move_at = Some(Utc::now() - Duration::minutes(60));
            }),
        ),
        case(
            "lane-review",
            "no lane move recorded",
            false,
            with(|p| {
                doing(p, false);
                p.last_lane_move_at = None;
            }),
        ),
        case(
            "next-task",
            "a next task and nothing running",
            true,
            with(|p| {
                let id = task(p).id;
                p.next_tasks.push(crate::progress::NextTask {
                    id,
                    title: "T".into(),
                    epic_id: None,
                    task_path: "p".into(),
                    why: crate::progress::NextReason::NextInEpic,
                    agent_status: crate::domain::RunState::Idle,
                });
            }),
        ),
        case("next-task", "no next task", false, with(|_| {})),
        case(
            "next-task",
            "a task is running",
            false,
            with(|p| {
                doing(p, true);
                let id = task(p).id;
                p.next_tasks.push(crate::progress::NextTask {
                    id,
                    title: "T".into(),
                    epic_id: None,
                    task_path: "p".into(),
                    why: crate::progress::NextReason::Resume,
                    agent_status: crate::domain::RunState::Paused,
                });
            }),
        ),
    ]
}

#[test]
fn the_stale_running_reminder_says_to_continue_and_never_to_pause() {
    let scenario = running_since(60);
    let findings = super::state_rules::stale_running(&scenario.facts());
    assert_eq!(findings.len(), 1);
    let text = format!("{:?}", findings[0]);
    assert!(text.contains("continue the work"), "{text}");
    assert!(text.contains("history add"), "{text}");
    assert!(text.contains("<why it cannot resume>"), "{text}");
    assert!(!text.contains("paused --actor ai"), "{text}");
}

#[test]
fn the_lane_review_reminder_lists_the_cards_and_the_choices() {
    let scenario = doing_after(60, false, false);
    let id = scenario.progress.as_ref().unwrap().unassigned[0].id;
    let findings = super::state_rules::lane_review(&scenario.facts());
    assert_eq!(findings.len(), 1);
    let text = &findings[0].text;
    assert!(text.contains(&format!("#{id}")), "{text}");
    assert!(text.contains("Resume"), "{text}");
    assert!(text.contains("message ask"), "{text}");
    assert!(text.contains("Todo"), "{text}");
    assert!(text.contains("confirm"), "{text}");
}

#[test]
fn the_lane_review_reminder_covers_todo_with_its_own_guidance() {
    let scenario = todo_only(false);
    let id = scenario.progress.as_ref().unwrap().unassigned[0].id;
    let findings = super::state_rules::lane_review(&scenario.facts());
    assert_eq!(findings.len(), 1);
    let text = &findings[0].text;
    assert!(
        text.contains(&format!("Todo holds AI cards: #{id}")),
        "{text}"
    );
    assert!(text.contains("history add"), "{text}");
    assert!(!text.contains("Doing holds"), "{text}");
}
