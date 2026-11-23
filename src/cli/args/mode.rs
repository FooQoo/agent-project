//! Arguments of `agpj mode` and `agpj decision`.

use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Debug)]
pub struct ModeArgs {
    /// The mode to switch to; omit it to show the current mode
    #[arg(value_enum)]
    pub mode: Option<ModeArg>,
    /// With "schedule": when the daily sleep window starts (HH:MM, local time of this machine)
    #[arg(long, value_name = "HH:MM")]
    pub start: Option<String>,
    /// With "schedule": when the daily sleep window ends (HH:MM); it may be earlier than the start
    #[arg(long, value_name = "HH:MM")]
    pub end: Option<String>,
    /// With "schedule": turn the daily window off (the times are kept)
    #[arg(long)]
    pub off: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeArg {
    /// The default: listen with "wait" and handle requests and answers
    Sparring,
    /// Do not accept questions to the user; subagents decide
    Sleep,
    /// Forget the manual mode, so the daily window decides
    Auto,
    /// Set the daily sleep window: --start HH:MM --end HH:MM, or --off
    Schedule,
}

#[derive(Args, Debug)]
pub struct DecisionArgs {
    #[command(subcommand)]
    pub command: DecisionCommand,
}

#[derive(Subcommand, Debug)]
pub enum DecisionCommand {
    /// Record a decision that a subagent made in place of the user, in the task history
    Add(DecisionAddArgs),
}

#[derive(Args, Debug)]
pub struct DecisionAddArgs {
    pub id: i64,
    /// The question that would have been asked
    #[arg(long, value_name = "TEXT")]
    pub question: String,
    /// The choice that was taken
    #[arg(long, value_name = "TEXT")]
    pub choice: String,
    /// Why this choice was taken
    #[arg(long, value_name = "TEXT")]
    pub reason: String,
    /// Who decided, as subagent:NAME
    #[arg(long, value_name = "subagent:NAME")]
    pub decided_by: String,
    /// What the decision rests on: a code path, a document, or a past question such as Q-123.
    /// Repeat it for each ground
    #[arg(long, value_name = "TEXT", required = true)]
    pub basis: Vec<String>,
}
