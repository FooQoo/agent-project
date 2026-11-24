//! The registry. Add a rule by adding an entry to [`RULES`] and a function it
//! points to, then its cases in `cases.rs` and a row in the rule list of
//! `docs/dev/concepts/agents.md` (and its Japanese copy).

use super::commands::{contains_sequence, runs_agpj, tokens};
use super::question_rules;
use super::record_rules;
use super::state_rules;
use super::{Area, Call, Facts, Finding, Kind, Lang, Rule, Source, When};

/// Every rule, in the order their reminders are listed.
pub const RULES: &[Rule] = &[
    Rule {
        id: "question",
        area: Area::Question,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Check,
        check: question_rules::question,
    },
    Rule {
        id: "question-images",
        area: Area::Question,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Check,
        check: question_rules::question_images,
    },
    Rule {
        id: "question-in-chat",
        area: Area::Question,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Definite,
        check: question_rules::question_in_chat,
    },
    Rule {
        id: "question-wait",
        area: Area::Question,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Definite,
        check: question_rules::question_wait,
    },
    Rule {
        id: "final-result",
        area: Area::Record,
        source: Source::State,
        when: When::Both,
        kind: Kind::Definite,
        check: final_result,
    },
    Rule {
        id: "result-pr",
        area: Area::Record,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Definite,
        check: result_pr,
    },
    Rule {
        id: "result-artifact",
        area: Area::Record,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Check,
        check: result_artifact,
    },
    Rule {
        id: "history",
        area: Area::Record,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Check,
        check: record_rules::history,
    },
    Rule {
        id: "conditions-after-merge",
        area: Area::Acceptance,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Check,
        check: record_rules::conditions_after_merge,
    },
    Rule {
        id: "start-task",
        area: Area::RunState,
        source: Source::Turn,
        when: When::TurnEnd,
        kind: Kind::Definite,
        check: record_rules::start_task,
    },
    entry(
        "answer",
        Area::Question,
        Kind::Definite,
        state_rules::answer,
    ),
    entry(
        "request",
        Area::Question,
        Kind::Definite,
        state_rules::request,
    ),
    entry(
        "conditions",
        Area::Acceptance,
        Kind::Check,
        state_rules::conditions,
    ),
    entry(
        "epic-close",
        Area::Epic,
        Kind::Definite,
        state_rules::epic_close,
    ),
    entry(
        "stale-running",
        Area::RunState,
        Kind::Check,
        state_rules::stale_running,
    ),
    entry(
        "lane-review",
        Area::RunState,
        Kind::Check,
        state_rules::lane_review,
    ),
    entry(
        "next-task",
        Area::RunState,
        Kind::Check,
        state_rules::next_task,
    ),
];

/// A `State` rule shown in the progress table and the Stop checklist.
const fn entry(
    id: &'static str,
    area: Area,
    kind: Kind,
    check: fn(&Facts) -> Vec<Finding>,
) -> Rule {
    Rule {
        id,
        area,
        source: Source::State,
        when: When::Both,
        kind,
        check,
    }
}

/// An AI task has all its conditions met and no final result.
fn final_result(facts: &Facts) -> Vec<Finding> {
    let Some(progress) = facts.progress else {
        return Vec::new();
    };
    progress
        .final_result_missing
        .iter()
        .map(|task| {
            Finding::new(match facts.lang {
                Lang::En => format!(
                    "Task #{id} has all conditions met but no final result: record `agpj result add {id} --final ...`.",
                    id = task.id
                ),
                Lang::Ja => format!(
                    "タスク #{id} は受け入れ条件がすべて満たされていますが、最終成果が記録されていません。`agpj result add {id} --final ...` で記録してください。",
                    id = task.id
                ),
            })
            .task(task.id)
        })
        .collect()
}

/// The turn opened a pull request and recorded no result.
fn result_pr(facts: &Facts) -> Vec<Finding> {
    if !result_expected(facts) || !facts.calls().iter().any(is_pr_create) {
        return Vec::new();
    }
    vec![Finding::new(facts.lang.pick(
        "You opened a pull request in this turn but recorded no result.",
        "このターンでプルリクエストを作成しましたが、成果が記録されていません。",
    ))]
}

/// The turn pushed or produced an image (and opened no pull request) and
/// recorded no result.
fn result_artifact(facts: &Facts) -> Vec<Finding> {
    let calls = facts.calls();
    if !result_expected(facts)
        || calls.iter().any(is_pr_create)
        || !calls.iter().any(produced_viewable)
    {
        return Vec::new();
    }
    vec![Finding::new(facts.lang.pick(
        "You pushed or produced an image. If it is something the requester can look at, record it with `agpj result add`.",
        "プッシュまたは画像の生成をしました。依頼者が見られるものなら、`agpj result add` で記録してください。",
    ))]
}

/// A result is due: the turn recorded none while an AI task is running.
fn result_expected(facts: &Facts) -> bool {
    !facts
        .calls()
        .iter()
        .any(|c| runs_agpj(&c.command, &[&["result", "add"]]))
        && has_running_ai_task(facts)
}

pub(super) fn is_pr_create(call: &Call) -> bool {
    contains_sequence(&tokens(&call.command), &["gh", "pr", "create"])
}

pub(super) fn has_running_ai_task(facts: &Facts) -> bool {
    use crate::domain::{Assignee, Column, RunState};
    let Some(progress) = facts.progress else {
        return false;
    };
    progress
        .epics
        .iter()
        .flat_map(|e| e.tasks.iter())
        .chain(progress.unassigned.iter())
        .any(|t| {
            t.assignee == Assignee::Ai
                && t.draft.is_none()
                && t.task_status == Column::Doing
                && t.agent_status == RunState::Running
        })
}

const IMAGE_EXTENSIONS: [&str; 5] = [".png", ".jpg", ".jpeg", ".gif", ".webp"];

fn is_image(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    IMAGE_EXTENSIONS.iter().any(|e| lower.ends_with(e))
}

/// A call that opened a pull request, pushed, or wrote an image.
fn produced_viewable(call: &Call) -> bool {
    let tokens = tokens(&call.command);
    contains_sequence(&tokens, &["gh", "pr", "create"])
        || contains_sequence(&tokens, &["git", "push"])
        || produced_image(call)
}

/// A call that wrote an image or took a screenshot.
pub(super) fn produced_image(call: &Call) -> bool {
    if is_image(&call.path) || call.name.ends_with("take_screenshot") {
        return true;
    }
    let tokens = tokens(&call.command);
    // A command that writes an image: it names an image file and a way to
    // produce one. Reading or listing one does not count.
    let lower = call.command.to_ascii_lowercase();
    tokens.iter().any(|t| is_image(t))
        && !runs_agpj(&call.command, &[&["result", "add"]])
        && [
            "screenshot",
            "screencapture",
            "capture",
            "--output",
            "--out",
            "--filename",
            "--path",
            " -o ",
            ">",
            "save",
            "convert",
            "magick",
            "ffmpeg",
        ]
        .iter()
        .any(|k| lower.contains(k))
}
