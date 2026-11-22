//! `release doc`: generate the release document of a version.

use serde_json::json;

use crate::cli::args::{ReleaseArgs, ReleaseCommand, ReleaseDocArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;

pub fn run(ctx: &mut Context, args: &ReleaseArgs) -> Result<Outcome, AgpjError> {
    match &args.command {
        ReleaseCommand::Doc(args) => doc(ctx, args),
    }
}

fn doc(ctx: &mut Context, args: &ReleaseDocArgs) -> Result<Outcome, AgpjError> {
    let doc =
        ctx.files
            .generate_release_doc(ctx.project_id()?, &args.choice()?, ctx.clock.now())?;
    let verb = if doc.created {
        "Created"
    } else if doc.changed {
        "Updated"
    } else {
        "Unchanged:"
    };
    let human = format!(
        "{verb} {} for {}: {} planned, {} verified, {} done without acceptance conditions, {} remaining, {} moved to another version, {} with a publication record. Generating publishes nothing.\n",
        doc.path,
        doc.version,
        doc.planned,
        doc.verified,
        doc.done_unchecked,
        doc.remaining,
        doc.moved,
        doc.published
    );
    Ok(Outcome {
        changed: doc.changed,
        human,
        json: json!({
            "version": doc.version,
            "path": doc.path,
            "created": doc.created,
            "changed": doc.changed,
            "planned": doc.planned,
            "verified": doc.verified,
            "doneWithoutAcceptance": doc.done_unchecked,
            "remaining": doc.remaining,
            "moved": doc.moved,
            "published": doc.published,
        }),
    })
}
