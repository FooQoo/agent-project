//! Cases of the `lane-review` rule: no lane move for a while, and cards in Doing or Todo to review.

use chrono::{Duration, Utc};

use super::cases::{Case, case};
use super::cases_state::{doing, task, with};
use super::test_support::Scenario;
use crate::domain::Column;

/// A table whose last lane move was `minutes` ago, with the table's only task in Doing.
fn doing_after(minutes: i64, running: bool, human: bool) -> Scenario {
    with(|p| {
        doing(p, running);
        if human {
            task(p).assignee = crate::domain::Assignee::Human;
        }
        p.last_lane_move_at = Some(Utc::now() - Duration::minutes(minutes));
        task(p).last_history_at = None;
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
        task(p).last_history_at = None;
    })
}

/// `doing_after(60, false, false)` where the only task's latest history entry was `minutes_ago`.
fn reviewed(minutes_ago: i64) -> Scenario {
    let mut s = doing_after(60, false, false);
    let p = s.progress.as_mut().unwrap();
    task(p).last_history_at = Some(Utc::now() - Duration::minutes(minutes_ago));
    s
}

pub fn all() -> Vec<Case> {
    vec![
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
            "the only candidate has history after the last move",
            false,
            reviewed(10),
        ),
        case(
            "lane-review",
            "the only candidate has history before the last move",
            true,
            reviewed(90),
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
    ]
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

#[test]
fn a_reviewed_card_is_left_out_of_the_list_and_the_others_stay() {
    let mut scenario = todo_only(false);
    let p = scenario.progress.as_mut().unwrap();
    let mut other = p.unassigned[0].clone();
    other.id += 7;
    other.last_history_at = None;
    p.unassigned[0].last_history_at = Some(Utc::now() - Duration::minutes(10));
    let (kept, left) = (other.id, p.unassigned[0].id);
    p.unassigned.push(other);
    let findings = super::state_rules::lane_review(&scenario.facts());
    assert_eq!(findings.len(), 1);
    let text = &findings[0].text;
    assert!(text.contains(&format!("#{kept}")), "{text}");
    assert!(!text.contains(&format!("#{left}")), "{text}");
    assert!(text.contains("until the next lane move"), "{text}");
}

#[test]
fn a_later_lane_move_makes_a_reviewed_card_eligible_again_after_30_minutes() {
    let mut scenario = reviewed(10);
    // A lane move 40 minutes ago is later than the history entry 90 minutes ago, not 10.
    let p = scenario.progress.as_mut().unwrap();
    task(p).last_history_at = Some(Utc::now() - Duration::minutes(50));
    p.last_lane_move_at = Some(Utc::now() - Duration::minutes(40));
    assert_eq!(super::state_rules::lane_review(&scenario.facts()).len(), 1);
    // Within 30 minutes of that move it stays quiet.
    scenario.progress.as_mut().unwrap().last_lane_move_at = Some(Utc::now() - Duration::minutes(5));
    assert!(super::state_rules::lane_review(&scenario.facts()).is_empty());
}
