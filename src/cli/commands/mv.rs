use crate::cli::args::MvArgs;
use crate::cli::{Context, Outcome};
use crate::domain::Actor;
use crate::{domain::ChangeContext, error::AgboardError};
use serde_json::json;

/// Only a person can confirm a warning; `--force` from an agent is refused.
pub(super) fn ack_of(actor: Actor, force: bool) -> Result<bool, AgboardError> {
    if force && actor == Actor::Ai {
        return Err(AgboardError::HumanOnly {
            action: "confirming a send with open questions (--force)".into(),
        });
    }
    Ok(force)
}

pub fn run(ctx: &mut Context, args: &MvArgs) -> Result<Outcome, AgboardError> {
    let reason = args.change.resolve_reason()?;
    let actor = args.change.actor.actor();
    let ack = ack_of(actor, args.force)?;
    let change = ChangeContext { actor, reason };
    let project_id = ctx.project_id()?;
    let to = args.column.column();
    let changed = if ack {
        ctx.files
            .move_work_task_confirmed(project_id, args.id, to, change, ctx.clock.now())?
    } else {
        ctx.files
            .move_work_task(project_id, args.id, to, change, ctx.clock.now())?
    };
    let human = if changed.changed {
        format!("Moved #{} to {}.\n", args.id, to.as_str())
    } else {
        format!("#{} is already in {}.\n", args.id, to.as_str())
    };
    Ok(Outcome {
        changed: changed.changed,
        human,
        json: json!({"task": changed.value}),
    })
}
