//! `state`: a task's run state, idle or running. Questions go through `qa`.

use serde_json::json;

use crate::cli::args::{StateArgs, read_reason};
use crate::cli::{Context, Outcome};
use crate::domain::{Actor, ChangeContext, validate_work_reason};
use crate::error::AgpjError;

pub fn run(ctx: &mut Context, args: &StateArgs) -> Result<Outcome, AgpjError> {
    let state = args.state.run_state();
    let actor = args.actor.actor();
    let reason = read_reason(args.reason.as_deref(), args.reason_file.as_deref())?
        .filter(|raw| !raw.trim().is_empty())
        .map(|raw| {
            validate_work_reason(&raw)?;
            Ok::<_, AgpjError>(raw)
        })
        .transpose()?
        .unwrap_or_default();
    let project_id = ctx.project_id()?;
    // A connected session binds the run to its connection, even when there is only one agent.
    // The unconnected path checks the project's compatibility rules under the save lock.
    let secret = ctx
        .files
        .shared_dir()
        .parent()
        .and_then(|data_dir| crate::agent::connection_store::load_secret(data_dir, project_id));
    let result = if let Some(secret) = secret {
        if actor == Actor::Human {
            return Err(AgpjError::HumanOnly {
                action: "Changing a run state from the command line as a person".into(),
            });
        }
        ctx.files.set_agent_status_connected(
            project_id,
            args.id,
            state,
            ChangeContext { actor, reason },
            &secret,
            ctx.clock.now(),
        )?
    } else {
        ctx.files.set_agent_status_unconnected(
            project_id,
            args.id,
            state,
            ChangeContext { actor, reason },
            ctx.clock.now(),
        )?
    };
    let human = if result.changed {
        format!("#{} agent status is now {}.\n", args.id, state.as_str())
    } else {
        format!("#{} agent status is already {}.\n", args.id, state.as_str())
    };
    Ok(Outcome {
        changed: result.changed,
        human,
        json: json!({"task": result.value}),
    })
}
