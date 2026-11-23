//! Arguments of `agpj version` and the version options of the other commands.

use clap::{Args, Subcommand};

use crate::domain::{VersionFilter, validate_version_name};
use crate::error::AgpjError;

#[derive(Args, Debug)]
pub struct VersionArgs {
    #[command(subcommand)]
    pub command: VersionCommand,
}

#[derive(Subcommand, Debug)]
pub enum VersionCommand {
    /// List the project's versions and its release version
    #[command(
        after_help = "Examples:\n  agpj --project demo version ls\n  agpj --project demo version ls --json"
    )]
    Ls,
    /// List a version in the project without making it the release version
    #[command(after_help = "Examples:\n  agpj --project demo version add v1.9.0")]
    Add(VersionAddArgs),
    /// Show the project's release version, set it, or clear it
    #[command(
        after_help = "A version that is not listed yet is listed when it is set. Tasks keep the version they were given: changing the release version moves none of them.\n\nExamples:\n  agpj --project demo version release\n  agpj --project demo version release v1.8.0\n  agpj --project demo version release --clear"
    )]
    Release(VersionReleaseArgs),
}

#[derive(Args, Debug)]
pub struct VersionAddArgs {
    /// The version name (1 to 64 characters, no whitespace)
    pub version: String,
}

#[derive(Args, Debug)]
pub struct VersionReleaseArgs {
    /// The version to make the release version; omit it to show the release version
    #[arg(value_name = "VERSION", conflicts_with = "clear")]
    pub version: Option<String>,
    /// Clear the release version
    #[arg(long)]
    pub clear: bool,
}

/// The version a task is planned for, as `add` and `edit` take it.
#[derive(Args, Debug, Clone, Default)]
#[group(id = "plan_target", multiple = false)]
pub struct PlanArgs {
    /// Plan the task for this version; it must be listed in the project (version add)
    #[arg(long, value_name = "VERSION")]
    pub planned_version: Option<String>,
    /// Plan the task for the project's release version, stored as its name now
    #[arg(long)]
    pub release_version: bool,
    /// Remove the task's planned version
    #[arg(long)]
    pub unplanned: bool,
}

impl PlanArgs {
    /// The choice asked for, or `None` when no option was given.
    pub fn choice(&self) -> Result<Option<crate::file_store::VersionChoice>, AgpjError> {
        use crate::file_store::VersionChoice;
        if let Some(name) = &self.planned_version {
            return Ok(Some(VersionChoice::Named(validate_version_name(name)?)));
        }
        Ok(if self.release_version {
            Some(VersionChoice::Release)
        } else if self.unplanned {
            Some(VersionChoice::Unset)
        } else {
            None
        })
    }
}

/// Narrow `ls` and `progress` to the tasks of one version. The options exclude each other.
#[derive(Args, Debug, Clone, Default)]
#[group(id = "version_filter", multiple = false)]
pub struct VersionFilterArgs {
    /// Only tasks planned for this version
    #[arg(long = "version", value_name = "VERSION")]
    pub version: Option<String>,
    /// Only tasks planned for the project's release version
    #[arg(long)]
    pub release_version: bool,
    /// Only tasks with no planned version
    #[arg(long)]
    pub unplanned: bool,
}

impl VersionFilterArgs {
    /// The filter asked for. `release` reads the project's release version when `--release-version`
    /// was given; a project with none is `no_release_version`.
    pub fn resolve(
        &self,
        release: impl FnOnce() -> Result<Option<String>, AgpjError>,
    ) -> Result<Option<VersionFilter>, AgpjError> {
        if let Some(name) = &self.version {
            return Ok(Some(VersionFilter::Planned(validate_version_name(name)?)));
        }
        if self.release_version {
            return release()?
                .map(|name| Some(VersionFilter::Planned(name)))
                .ok_or(AgpjError::NoReleaseVersion);
        }
        Ok(self.unplanned.then_some(VersionFilter::Unplanned))
    }

    /// Whether any option was given.
    pub fn is_set(&self) -> bool {
        self.version.is_some() || self.release_version || self.unplanned
    }
}
