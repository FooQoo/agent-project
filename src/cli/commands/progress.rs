//! `agpj progress`: the tasks an agent can pick up, by epic in wave
//! order. The table is built by `crate::progress`, the same code the hook uses.

use std::fmt::Write as _;

use serde_json::json;

use crate::cli::args::ProgressCommandArgs;
use crate::cli::{Context, Outcome};
use crate::domain::{Assignee, Column, RunState};
use crate::error::AgpjError;
use crate::progress::{self, Include, Progress, ProgressTask};

pub fn run(ctx: &Context, args: &ProgressCommandArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?;
    let filter = args
        .version
        .resolve(|| Ok(ctx.files.project_versions(project)?.current))?;
    let snapshot = ctx.files.board(project)?;
    let mut table = progress::build_filtered(
        &snapshot,
        ctx.files.shared_dir(),
        ctx.clock.now(),
        Include {
            awaiting_answer: args.progress.include_awaiting_answer,
            done: args.progress.include_done,
        },
        filter.as_ref(),
    );
    table.locale = crate::server::preferences::saved_locale(ctx.files.shared_dir());
    Ok(Outcome {
        changed: false,
        human: render(&table),
        json: json!(table),
    })
}

/// The table for a person: one block per epic, then the unassigned tasks.
pub fn render(table: &Progress) -> String {
    let mut out = String::new();
    if !table.next_tasks.is_empty() {
        out.push_str("Next tasks\n");
        for task in &table.next_tasks {
            let why = match task.why {
                progress::NextReason::Stalled => "stalled",
                progress::NextReason::Resume => "resume",
                progress::NextReason::NextInEpic => "next_in_epic",
            };
            let _ = writeln!(out, "  #{} {} [{why}]", task.id, task.title);
        }
        out.push('\n');
    }
    if !table.final_result_missing.is_empty() {
        out.push_str("Final result missing (record with result add --final)\n");
        for task in &table.final_result_missing {
            let _ = writeln!(out, "  #{} {}", task.id, task.title);
        }
        out.push('\n');
    }
    for epic in &table.epics {
        let _ = writeln!(out, "{} {}", epic.id, epic.name);
        for task in &epic.tasks {
            write_task(&mut out, task);
        }
        out.push('\n');
    }
    if !table.unassigned.is_empty() {
        out.push_str("Unassigned\n");
        for task in &table.unassigned {
            write_task(&mut out, task);
        }
    }
    if let Some(pending) = &table.awaiting_answer {
        out.push_str("Waiting for a decision\n");
        for task in pending {
            write_task(&mut out, task);
        }
        out.push('\n');
    }
    if let Some(done) = &table.done {
        out.push_str("Recently done\n");
        for task in done {
            write_task(&mut out, task);
        }
        out.push('\n');
    }
    if out.is_empty() {
        out.push_str("No tasks to pick up.\n");
    }
    out
}

fn write_task(out: &mut String, task: &ProgressTask) {
    let assignee = match task.assignee {
        Assignee::Ai => "ai",
        Assignee::Human => "human",
    };
    let column = match task.task_status {
        Column::Backlog => "backlog",
        Column::Todo => "todo",
        Column::Doing => "doing",
        Column::Done => "done",
    };
    let state = match task.agent_status {
        RunState::Idle => "idle",
        RunState::Running => "running",
        RunState::Paused => "paused",
    };
    let draft = match task.draft {
        None => "",
        Some(crate::domain::Draft::Human) => ", draft",
    };
    let plan = task
        .planned_version
        .as_ref()
        .map_or(String::new(), |version| format!(", plan {version}"));
    let _ = writeln!(
        out,
        "  #{} {} [{assignee}, {column}, {state}{draft}{plan}] acceptance {}/{}",
        task.id, task.title, task.acceptance.done, task.acceptance.total
    );
    if let Some(question) = &task.question {
        let _ = writeln!(out, "    question: {question}");
    }
}
