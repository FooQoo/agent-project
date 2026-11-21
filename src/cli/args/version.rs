//! Arguments of `agboard version` and the version options of the other commands.

use clap::{Args, Subcommand};

use crate::domain::{VersionFilter, validate_version_name};
use crate::error::AgboardError;

#[derive(Args, Debug)]
pub struct VersionArgs {
    #[command(subcommand)]
    pub command: VersionCommand,
}

#[derive(Subcommand, Debug)]
pub enum VersionCommand {
    /// List the project's versions and its current one
    #[command(
        after_help = "Examples:\n  agboard --project demo version ls\n  agboard --project demo version ls --json"
    )]
    Ls,
    /// List a version in the project without making it current
    #[command(after_help = "Examples:\n  agboard --project demo version add v1.9.0")]
    Add(VersionAddArgs),
    /// Show the project's current version, set it, or clear it
    #[command(
        after_help = "A version that is not listed yet is listed when it is set. Tasks keep the version they were given: changing the current version moves none of them.\n\nExamples:\n  agboard --project demo version current\n  agboard --project demo version current v1.8.0\n  agboard --project demo version current --clear"
    )]
    Current(VersionCurrentArgs),
}

#[derive(Args, Debug)]
pub struct VersionAddArgs {
    /// The version name (1 to 64 characters, no whitespace)
    pub version: String,
}

#[derive(Args, Debug)]
pub struct VersionCurrentArgs {
    /// The version to make current; omit it to show the current version
    #[arg(value_name = "VERSION", conflicts_with = "clear")]
    pub version: Option<String>,
    /// Clear the current version
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
    /// Plan the task for the project's current version, stored as its name now
    #[arg(long)]
    pub current_version: bool,
    /// Remove the task's planned version
    #[arg(long)]
    pub unplanned: bool,
}

impl PlanArgs {
    /// The choice asked for, or `None` when no option was given.
    pub fn choice(&self) -> Result<Option<crate::file_store::VersionChoice>, AgboardError> {
        use crate::file_store::VersionChoice;
        if let Some(name) = &self.planned_version {
            return Ok(Some(VersionChoice::Named(validate_version_name(name)?)));
        }
        Ok(if self.current_version {
            Some(VersionChoice::Current)
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
    /// Only tasks planned for the project's current version
    #[arg(long)]
    pub current_version: bool,
    /// Only tasks with no planned version
    #[arg(long)]
    pub unplanned: bool,
}

impl VersionFilterArgs {
    /// The filter asked for. `current` reads the project's current version when `--current-version`
    /// was given; a project with none is `no_current_version`.
    pub fn resolve(
        &self,
        current: impl FnOnce() -> Result<Option<String>, AgboardError>,
    ) -> Result<Option<VersionFilter>, AgboardError> {
        if let Some(name) = &self.version {
            return Ok(Some(VersionFilter::Planned(validate_version_name(name)?)));
        }
        if self.current_version {
            return current()?
                .map(|name| Some(VersionFilter::Planned(name)))
                .ok_or(AgboardError::NoCurrentVersion);
        }
        Ok(self.unplanned.then_some(VersionFilter::Unplanned))
    }

    /// Whether any option was given.
    pub fn is_set(&self) -> bool {
        self.version.is_some() || self.current_version || self.unplanned
    }
}
