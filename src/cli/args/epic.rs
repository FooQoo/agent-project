//! Arguments of `agpj epic`.

use std::io::Read;
use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::enums::ActorArg;
use super::task::strip_final_newline;
use crate::error::{AgpjError, FileAction};

#[derive(Args, Debug)]
#[command(
    after_help = "Examples:\n  agpj --project demo epic add file-share --name \"File share\" --actor human\n  agpj --project demo epic add file-share --name \"File share\" --color '#7C3AED' --actor human\n  agpj --project demo epic ls --json\n  agpj --project demo epic show FILE-SHARE\n  agpj --project demo epic edit FILE-SHARE --document-file epic.md\n  agpj --project demo epic rm FILE-SHARE"
)]
pub struct EpicArgs {
    #[command(subcommand)]
    pub command: EpicCommand,
}

#[derive(Subcommand, Debug)]
pub enum EpicCommand {
    /// Create an epic with an empty document (EPIC-ID, name, and an optional color)
    Add(EpicAddArgs),
    /// List the epics in EPIC-ID order
    Ls,
    /// Show an epic, its color, and its full document
    Show(EpicIdArgs),
    /// Change an epic's name, color, document, or task order; a draft epic is refused with epic_is_draft
    Edit(EpicEditArgs),
    /// Delete an epic (humans only; asks for confirmation)
    Rm(EpicIdArgs),
    /// Send the Backlog cards of an epic to Todo
    Send(EpicSendArgs),
    /// Plan the cards of an epic for a version, the current version, or none
    Plan(EpicPlanArgs),
}

#[derive(Args, Debug)]
#[command(
    after_help = "Sends the Backlog cards of the epic to Todo in one save: all of them, or only those named by --tasks. Only a person sends (an agent is refused with human_only). If any card cannot be sent, none is: a card a person is editing (task_is_draft), or an open question (send_with_open_questions until the person confirms with --force). Cards that are not in the Backlog stay as they are.\n\nExamples:\n  agpj --project demo epic send FILE-SHARE --actor human\n  agpj --project demo epic send FILE-SHARE --actor human --tasks 12,13 --force"
)]
pub struct EpicSendArgs {
    /// The epic's EPIC-ID
    pub id: String,
    /// Who is making this change
    #[arg(long, value_enum, required = true)]
    pub actor: ActorArg,
    /// Only these cards, as task IDs joined by commas (for example 12,13); all Backlog cards when omitted
    #[arg(long, value_name = "IDS")]
    pub tasks: Option<String>,
    /// Send even though a question about a card is open
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
#[command(
    after_help = "Sets the planned version of the epic's cards, each saved on its own: a failure part way leaves the earlier cards changed, and running the command again finishes the rest. Archived cards, Done cards, and cards a person is drafting are skipped, because they shipped or are not the agent's to change. With --only-unplanned, a card that already has a planned version is skipped as well. The result counts the changed, unchanged (already had the version), and skipped cards, and names them.\n\nExamples:\n  agpj --project demo epic plan FILE-SHARE --planned-version v1.9.0\n  agpj --project demo epic plan FILE-SHARE --current-version --only-unplanned\n  agpj --project demo epic plan FILE-SHARE --unplanned"
)]
pub struct EpicPlanArgs {
    /// The epic's EPIC-ID
    pub id: String,
    #[command(flatten)]
    pub plan: super::version::PlanArgs,
    /// Leave the cards that already have a planned version alone
    #[arg(long, conflicts_with = "unplanned")]
    pub only_unplanned: bool,
}

#[derive(Args, Debug)]
pub struct EpicIdArgs {
    /// The epic's EPIC-ID (letters, digits, and hyphens; lowercase is folded)
    pub id: String,
}

#[derive(Args, Debug)]
pub struct EpicAddArgs {
    /// The epic's EPIC-ID (letters, digits, and hyphens)
    pub id: String,
    /// Formal name of the epic (1 to 100 characters, one line)
    #[arg(long, value_name = "NAME")]
    pub name: String,
    /// Color as #RRGGBB; a random color that no epic uses when omitted
    #[arg(long, value_name = "#RRGGBB")]
    pub color: Option<String>,
    /// Who is making this change
    #[arg(long, value_enum, required = true)]
    pub actor: ActorArg,
}

#[derive(Args, Debug)]
pub struct EpicEditArgs {
    /// The epic's EPIC-ID
    pub id: String,
    /// New formal name
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// New color as #RRGGBB
    #[arg(long, value_name = "#RRGGBB")]
    pub color: Option<String>,
    /// Task order as waves, one value per wave: a task ID or task IDs joined by commas (for example --waves 24 25,26); a wave starts when the waves before it are done; an empty value clears it
    #[arg(long, value_name = "WAVE", num_args(1..))]
    pub waves: Option<Vec<String>>,
    #[command(flatten)]
    pub document: DocumentArgs,
}

/// The two ways to give an epic document: the text itself, or a file. `-` reads standard input.
#[derive(Args, Debug, Clone, Default)]
#[group(id = "document_source", multiple = false)]
pub struct DocumentArgs {
    /// Whole EPIC.md text in Markdown, frontmatter included, up to 100,000 characters; use "-" to read it from standard input
    #[arg(long, value_name = "TEXT")]
    pub document: Option<String>,
    /// Read the whole EPIC.md text, frontmatter included (up to 100,000 characters), from a file
    #[arg(long, value_name = "PATH")]
    pub document_file: Option<PathBuf>,
}

impl DocumentArgs {
    /// The document the user asked for, or `None` when neither option was given.
    pub fn resolve(&self, stdin: &mut dyn Read) -> Result<Option<String>, AgpjError> {
        if let Some(text) = &self.document
            && text == "-"
        {
            let mut buffer = String::new();
            stdin
                .read_to_string(&mut buffer)
                .map_err(|e| AgpjError::FileError {
                    action: FileAction::Read,
                    file: "standard input".to_string(),
                    reason: e.to_string(),
                })?;
            return Ok(Some(strip_final_newline(buffer)));
        }
        if let Some(text) = &self.document {
            return Ok(Some(text.clone()));
        }
        if let Some(path) = &self.document_file {
            let text = std::fs::read_to_string(path).map_err(|e| AgpjError::FileError {
                action: FileAction::Read,
                file: path.display().to_string(),
                reason: e.to_string(),
            })?;
            return Ok(Some(strip_final_newline(text)));
        }
        Ok(None)
    }
}
