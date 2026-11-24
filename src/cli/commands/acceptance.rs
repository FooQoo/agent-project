use crate::cli::args::{AcceptanceArgs, AcceptanceCommand};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use serde_json::json;

pub fn run(ctx: &mut Context, args: &AcceptanceArgs) -> Result<Outcome, AgpjError> {
    let AcceptanceCommand::Check(args) = &args.command;
    let result = ctx.files.check_acceptance_as_agent(
        ctx.project_id()?,
        args.id,
        &args.condition,
        ctx.clock.now(),
    )?;
    Ok(Outcome {
        changed: result.changed,
        human: format!("Checked {} on #{}.\n", args.condition, args.id),
        json: json!({"task": result.value, "changed": result.changed}),
    })
}
