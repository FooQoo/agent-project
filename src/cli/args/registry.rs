//! Arguments of `connect` and `agent`.

use clap::{Args, Subcommand};

/// `connect` joins this session to an agent with a code the user got from the board.
#[derive(Args, Debug)]
#[command(
    after_help = "The user registers agents and shows a one-time connection code (valid for ten minutes) in the board's settings. This command exchanges the code for this session's connection and keeps its secret in a file only the user can read.\nA second session cannot take an agent over: the user must choose Switch in the board and give you the new code.\n\nExamples:\n  agpj --project demo connect K7M2-QX9P"
)]
pub struct ConnectArgs {
    /// The connection code shown by the board
    #[arg(value_name = "CODE")]
    pub code: String,
}

/// `agent` reads the registered agents.
#[derive(Args, Debug)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// List the project's agents and who is connected to each
    Ls,
}
