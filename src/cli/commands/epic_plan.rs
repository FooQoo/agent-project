//! `epic plan`: plan the cards of an epic for a version.

use serde_json::json;

use crate::cli::args::EpicPlanArgs;
use crate::cli::{Context, Outcome};
use crate::domain::Actor;
use crate::error::AgpjError;
use crate::file_store::EpicVersionScope;
use crate::file_store::validate::EpicId;

/// Plan every card of an epic for one version. The command always acts as an agent: it skips
/// archived cards, cards a person is drafting, and Done cards, which shipped.
pub fn run(ctx: &mut Context, args: &EpicPlanArgs) -> Result<Outcome, AgpjError> {
    let Some(choice) = args.plan.choice()? else {
        return Err(AgpjError::InvalidArgument {
            message: "epic plan needs one of --planned-version, --current-version, or --unplanned."
                .to_string(),
            command: Some("epic".to_string()),
        });
    };
    let epic_id = EpicId::normalize(&args.id)?;
    let scope = if args.only_unplanned {
        EpicVersionScope::UnplannedOnly
    } else {
        EpicVersionScope::All
    };
    let result = ctx.files.set_epic_planned_version_in_scope(
        ctx.project_id()?,
        &epic_id,
        &choice,
        scope,
        Actor::Ai,
        ctx.clock.now(),
    )?;
    let ids = |ids: &[i64]| {
        if ids.is_empty() {
            String::new()
        } else {
            let list: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
            format!(" ({})", list.join(", "))
        }
    };
    let human = format!(
        "Epic {epic_id}: {} changed{}, {} unchanged{}, {} skipped{}.\n",
        result.changed.len(),
        ids(&result.changed),
        result.unchanged.len(),
        ids(&result.unchanged),
        result.skipped.len(),
        ids(&result.skipped),
    );
    Ok(Outcome {
        changed: !result.changed.is_empty(),
        human,
        json: json!({
            "epic": epic_id.as_str(),
            "changed": result.changed,
            "unchanged": result.unchanged,
            "skipped": result.skipped,
        }),
    })
}
