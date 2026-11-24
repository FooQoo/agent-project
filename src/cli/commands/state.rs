//! `state`: a task's run state, idle or running. Questions go through `qa`.

use serde_json::json;

use crate::cli::args::{StateArgs, read_reason};
use crate::cli::{Context, Outcome};
use crate::domain::{ChangeContext, validate_work_reason};
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
    // The claimed actor decides nothing. Once the user has registered an agent, a run state is
    // set through the session's connection, and `--actor human` is a person's word at a
    // terminal: a person acts in the web UI. A project with no agents keeps the plain rules.
    let result = if ctx.files.has_agents(project_id)? {
        if actor == crate::domain::Actor::Human {
            return Err(AgpjError::HumanOnly {
                action: "Changing a run state from the command line as a person".into(),
            });
        }
        let secret = ctx
            .files
            .shared_dir()
            .parent()
            .and_then(|data_dir| crate::agent::connection_store::load_secret(data_dir, project_id))
            .ok_or(AgpjError::NotConnected)?;
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
