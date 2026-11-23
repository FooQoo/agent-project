//! `decision add`: record a decision a subagent made in the user's place.

use serde_json::json;

use crate::cli::args::{DecisionAddArgs, DecisionArgs, DecisionCommand};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::file_store::model::Decision;

pub fn run(ctx: &mut Context, args: &DecisionArgs) -> Result<Outcome, AgpjError> {
    let DecisionCommand::Add(add) = &args.command;
    let decision = decision_of(add)?;
    let task = ctx
        .files
        .add_decision(ctx.project_id()?, add.id, &decision, ctx.clock.now())?;
    Ok(Outcome {
        changed: true,
        human: format!("Recorded the decision on #{}.\n", add.id),
        json: json!({"task": task}),
    })
}

fn invalid(message: &str) -> AgpjError {
    AgpjError::InvalidArgument {
        message: message.to_string(),
        command: Some("decision add".to_string()),
    }
}

fn decision_of(add: &DecisionAddArgs) -> Result<Decision, AgpjError> {
    let decided_by = add.decided_by.trim();
    match decided_by.strip_prefix("subagent:") {
        Some(name) if !name.trim().is_empty() => {}
        _ => return Err(invalid("--decided-by must look like subagent:NAME.")),
    }
    let fields = [
        ("--question", add.question.as_str()),
        ("--choice", add.choice.as_str()),
        ("--reason", add.reason.as_str()),
    ];
    for (name, value) in fields {
        if value.trim().is_empty() {
            return Err(invalid(&format!("{name} cannot be empty.")));
        }
    }
    let basis: Vec<String> = add
        .basis
        .iter()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect();
    if basis.is_empty() {
        return Err(invalid(
            "--basis needs at least one ground: code, a document, or a past question.",
        ));
    }
    Ok(Decision {
        question: add.question.clone(),
        choice: add.choice.clone(),
        reason: add.reason.clone(),
        decided_by: decided_by.to_string(),
        basis,
    })
}
