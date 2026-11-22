//! Arguments of `agpj release`.

use clap::{Args, Subcommand};

use crate::domain::validate_version_name;
use crate::error::AgpjError;
use crate::file_store::VersionChoice;

#[derive(Args, Debug)]
pub struct ReleaseArgs {
    #[command(subcommand)]
    pub command: ReleaseCommand,
}

#[derive(Subcommand, Debug)]
pub enum ReleaseCommand {
    /// Generate the release document of a version in docs/release/
    #[command(
        after_help = "Writes docs/release/<version>.md in the project's Docs: what was planned for the version, what is verified, what remains, and what moved to another version. Only the block between the agpj:generated markers is replaced; text outside it is kept. Generating again changes nothing unless the board changed. It publishes nothing outside the shared folder.\n\nExamples:\n  agpj --project demo release doc --current-version\n  agpj --project demo release doc --version v1.8.0\n  agpj --project demo release doc --version v1.8.0 --json"
    )]
    Doc(ReleaseDocArgs),
}

#[derive(Args, Debug, Clone)]
#[group(id = "doc_target", required = true, multiple = false)]
pub struct ReleaseDocArgs {
    /// Generate the document of this version; a version that is not listed yet is listed first
    #[arg(long = "version", value_name = "VERSION")]
    pub version: Option<String>,
    /// Generate the document of the project's current version
    #[arg(long)]
    pub current_version: bool,
}

impl ReleaseDocArgs {
    pub fn choice(&self) -> Result<VersionChoice, AgpjError> {
        match &self.version {
            Some(name) => Ok(VersionChoice::Named(validate_version_name(name)?)),
            None => Ok(VersionChoice::Current),
        }
    }
}
