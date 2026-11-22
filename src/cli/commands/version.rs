//! `version ls|add|current`: the project's versions and its current version.

use serde_json::json;

use crate::cli::args::{VersionAddArgs, VersionArgs, VersionCommand, VersionCurrentArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::file_store::ProjectVersions;

pub fn run(ctx: &mut Context, args: &VersionArgs) -> Result<Outcome, AgpjError> {
    match &args.command {
        VersionCommand::Ls => {
            let versions = ctx.files.project_versions(ctx.project_id()?)?;
            Ok(outcome(false, listing(&versions), &versions))
        }
        VersionCommand::Add(args) => add(ctx, args),
        VersionCommand::Current(args) => current(ctx, args),
    }
}

fn add(ctx: &mut Context, args: &VersionAddArgs) -> Result<Outcome, AgpjError> {
    let result = ctx
        .files
        .add_version(ctx.project_id()?, &args.version, ctx.clock.now())?;
    let text = if result.changed {
        format!("Added version {}.\n", args.version)
    } else {
        format!("Version {} is already listed.\n", args.version)
    };
    Ok(outcome(result.changed, text, &result.value))
}

fn current(ctx: &mut Context, args: &VersionCurrentArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    if args.version.is_none() && !args.clear {
        let versions = ctx.files.project_versions(&project)?;
        let text = match &versions.current {
            Some(current) => format!("Current version: {current}\n"),
            None => "No current version.\n".to_string(),
        };
        return Ok(outcome(false, text, &versions));
    }
    let result =
        ctx.files
            .set_current_version(&project, args.version.as_deref(), ctx.clock.now())?;
    let text = match (&result.value.current, result.changed) {
        (Some(current), true) => format!("Current version is now {current}.\n"),
        (Some(current), false) => format!("Current version is already {current}.\n"),
        (None, true) => "Cleared the current version.\n".to_string(),
        (None, false) => "There is no current version.\n".to_string(),
    };
    Ok(outcome(result.changed, text, &result.value))
}

fn listing(versions: &ProjectVersions) -> String {
    if versions.versions.is_empty() {
        return "No versions.\n".to_string();
    }
    let mut out = String::new();
    for version in &versions.versions {
        let mark = if versions.current.as_deref() == Some(version) {
            " (current)"
        } else {
            ""
        };
        out.push_str(&format!("{version}{mark}\n"));
    }
    out
}

/// `versions` lists every version in the order they were added; `currentVersion` is `null`
/// when none is set.
fn outcome(changed: bool, human: String, versions: &ProjectVersions) -> Outcome {
    Outcome {
        changed,
        human,
        json: json!({
            "versions": versions.versions,
            "currentVersion": versions.current,
        }),
    }
}
