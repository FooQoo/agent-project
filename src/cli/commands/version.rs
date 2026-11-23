//! `version ls|add|release`: the project's versions and its release version.

use serde_json::json;

use crate::cli::args::{VersionAddArgs, VersionArgs, VersionCommand, VersionReleaseArgs};
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
        VersionCommand::Release(args) => release(ctx, args),
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

fn release(ctx: &mut Context, args: &VersionReleaseArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    if args.version.is_none() && !args.clear {
        let versions = ctx.files.project_versions(&project)?;
        let text = match &versions.release {
            Some(release) => format!("Release version: {release}\n"),
            None => "No release version.\n".to_string(),
        };
        return Ok(outcome(false, text, &versions));
    }
    let result =
        ctx.files
            .set_release_version(&project, args.version.as_deref(), ctx.clock.now())?;
    let text = match (&result.value.release, result.changed) {
        (Some(release), true) => format!("Release version is now {release}.\n"),
        (Some(release), false) => format!("Release version is already {release}.\n"),
        (None, true) => "Cleared the release version.\n".to_string(),
        (None, false) => "There is no release version.\n".to_string(),
    };
    Ok(outcome(result.changed, text, &result.value))
}

fn listing(versions: &ProjectVersions) -> String {
    if versions.versions.is_empty() {
        return "No versions.\n".to_string();
    }
    let mut out = String::new();
    for version in &versions.versions {
        let mark = if versions.release.as_deref() == Some(version) {
            " (release)"
        } else {
            ""
        };
        out.push_str(&format!("{version}{mark}\n"));
    }
    out
}

/// `versions` lists every version in the order they were added; `releaseVersion` is `null`
/// when none is set.
fn outcome(changed: bool, human: String, versions: &ProjectVersions) -> Outcome {
    Outcome {
        changed,
        human,
        json: json!({
            "versions": versions.versions,
            "releaseVersion": versions.release,
        }),
    }
}
