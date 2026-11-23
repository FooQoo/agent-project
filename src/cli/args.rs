//! Command line definition (clap derive) and argument post-validation.

mod agent;
mod attachment;
mod dash_hint;
mod enums;
mod epic;
mod help;
mod history;
mod message;
mod mode;
mod project;
mod release;
mod result;
mod task;
mod validate;
mod version;

pub use self::agent::*;
pub use self::attachment::*;
pub use self::enums::*;
pub use self::epic::*;
pub use self::history::*;
pub use self::message::*;
pub use self::mode::*;
pub use self::project::*;
pub use self::release::*;
pub use self::result::*;
pub use self::task::*;
pub use self::validate::*;
pub use self::version::*;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

use self::help::TOP_HELP;
use crate::error::AgpjError;

#[derive(Parser, Debug)]
#[command(
    name = "agpj",
    version,
    disable_help_subcommand = true,
    override_help = TOP_HELP
)]
pub struct Cli {
    /// The project to work on. Progress and hooks resolve the nearest folder
    /// binding when omitted; most other commands require this value.
    #[arg(long, global = true, value_name = "NAME")]
    project: Option<String>,
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create, list, rename, or delete projects
    Project(ProjectArgs),
    /// Create the project's board, or empty an existing one
    Init(InitArgs),
    /// Add a task to Todo
    Add(AddArgs),
    /// Change a task's title or description
    Edit(EditArgs),
    /// Move a task to todo, doing, or done (a person can also take an unstarted Todo task back to backlog)
    Mv(MvArgs),
    /// Report a task's run state: idle or running
    State(StateArgs),
    /// Append a work note to a task history
    History(HistoryArgs),
    /// Record a decision that a subagent made in the user's place
    Decision(DecisionArgs),
    /// Show or switch the mode: sparring (default), sleep, or brainstorm
    Mode(ModeArgs),
    /// Ask the user questions, wait for the answers, and handle requests from the user
    Message(MessageArgs),
    /// Delete a task (humans only; asks for confirmation)
    Rm(RmArgs),
    /// List tasks
    Ls(LsArgs),
    /// Show a task and its history
    Show(ShowArgs),
    /// Add, list, or read image attachments
    Attachment(AttachmentArgs),
    /// Record the results delivered to the requester, or list them
    Result(ResultArgs),
    /// Create, list, show, edit, or delete epics
    Epic(EpicArgs),
    /// Fetch the complete board and history without modifying it
    Snapshot(SnapshotArgs),
    /// Add the current progress table to an agent hook
    Hook(HookArgs),
    /// Print the instructions of the agent-project Skill
    Skills(SkillsArgs),
    /// Wait for a board change or the next task
    Wait(WaitArgs),
    /// Show the board in your browser
    Serve(ServeArgs),
    /// Open the project's board in your browser, starting the server when needed
    Open(OpenArgs),
    /// Stop the running server (the one started with serve, or serve -d)
    #[command(after_help = "Examples:\n  agpj stop\n  agpj stop --json")]
    Stop(StopArgs),
    /// Create the data folder and install the Agent Project plugin for Codex and Claude Code
    Setup(SetupArgs),
    /// Update the installed Agent Project plugin to this version
    Update(UpdateArgs),
    /// Show the project's shared folder and where its files are
    Workspace(WorkspaceArgs),
    /// Show the tasks an agent can pick up, by epic in wave order
    #[command(
        after_help = "A version option only narrows what is shown: which tasks are ready is decided on the whole board first, so hiding a task never makes another one ready. Requests, answers, and epics to close are not narrowed.\n\nExamples:\n  agpj progress\n  agpj progress --json\n  agpj progress --include-awaiting-answer --include-done 5\n  agpj progress --release-version\n  agpj progress --version v1.8.0 --json\n  agpj progress --unplanned\n  agpj --project \"My project\" progress --json"
    )]
    Progress(ProgressCommandArgs),
    /// List the project's versions, add one, or show, set, or clear the release version
    Version(VersionArgs),
    /// Generate the release document of a version in docs/release/
    Release(ReleaseArgs),
}

impl Command {
    /// The subcommand's name as typed on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Project(_) => "project",
            Command::Init(_) => "init",
            Command::Add(_) => "add",
            Command::Edit(_) => "edit",
            Command::Mv(_) => "mv",
            Command::State(_) => "state",
            Command::History(_) => "history",
            Command::Decision(_) => "decision",
            Command::Mode(_) => "mode",
            Command::Message(_) => "message",
            Command::Rm(_) => "rm",
            Command::Ls(_) => "ls",
            Command::Show(_) => "show",
            Command::Attachment(_) => "attachment",
            Command::Result(_) => "result",
            Command::Epic(_) => "epic",
            Command::Snapshot(_) => "snapshot",
            Command::Hook(_) => "hook",
            Command::Skills(_) => "skills",
            Command::Wait(_) => "wait",
            Command::Serve(_) => "serve",
            Command::Open(_) => "open",
            Command::Stop(_) => "stop",
            Command::Setup(_) => "setup",
            Command::Update(_) => "update",
            Command::Workspace(_) => "workspace",
            Command::Progress(_) => "progress",
            Command::Version(_) => "version",
            Command::Release(_) => "release",
        }
    }
}

pub const SUBCOMMAND_NAMES: [&str; 27] = [
    "project",
    "init",
    "add",
    "edit",
    "mv",
    "state",
    "history",
    "message",
    "rm",
    "ls",
    "show",
    "attachment",
    "result",
    "epic",
    "snapshot",
    "serve",
    "open",
    "stop",
    "hook",
    "wait",
    "workspace",
    "progress",
    "version",
    "release",
    "setup",
    "update",
    "skills",
];

/// What parsing the command line produced.
#[derive(Debug)]
pub enum Parsed {
    /// A command to run.
    Run(Cli),
    /// `--help` or `--version`: print this text to standard output, exit 0.
    Print(String),
}

/// Parse `args` (the first item is the program name).
pub fn parse(args: &[OsString]) -> Result<Parsed, AgpjError> {
    match Cli::try_parse_from(args) {
        // The parser lets a later `--project` after the subcommand replace an
        // earlier one before it; a command must never guess which one was meant.
        Ok(_) if project_occurrences(args) > 1 => Err(AgpjError::InvalidArgument {
            message: "--project can be given only once.".to_string(),
            command: subcommand_in(args),
        }),
        Ok(cli) => Ok(Parsed::Run(cli)),
        Err(error) => {
            use clap::error::ErrorKind;
            match error.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                    Ok(Parsed::Print(error.to_string()))
                }
                kind => Err(AgpjError::InvalidArgument {
                    message: match kind {
                        // The parser would print the whole help text here.
                        ErrorKind::MissingSubcommand
                        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                            "a command is required.".to_string()
                        }
                        _ => {
                            let mut message = parser_message(&error.to_string());
                            if let Some(hint) = dash_hint::dash_value_hint(&error, args) {
                                message.push(' ');
                                message.push_str(&hint);
                            }
                            message
                        }
                    },
                    command: subcommand_in(args),
                }),
            }
        }
    }
}

/// How many times `--project` appears as an option (not as the value of
/// another option). Everything after `--` is a value.
fn project_occurrences(args: &[OsString]) -> usize {
    let mut count = 0;
    let mut skip_next = false;
    for arg in args.iter().skip(1) {
        let Some(arg) = arg.to_str() else { continue };
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg {
            "--" => break,
            "--project" => {
                count += 1;
                skip_next = true; // its value
            }
            // These take a value that may itself start with a dash.
            "--desc" | "--reason" => skip_next = true,
            _ if arg.starts_with("--project=") => count += 1,
            _ => {}
        }
    }
    count
}

/// The parser's one-line message: the first line without its `error: ` prefix,
/// plus the list of possible values when the parser gives one.
fn parser_message(rendered: &str) -> String {
    let mut lines = rendered.lines();
    let first = lines.next().unwrap_or_default();
    let mut message = first.strip_prefix("error: ").unwrap_or(first).to_string();
    if let Some(values) = lines
        .map(str::trim)
        .find(|line| line.starts_with("[possible values:"))
    {
        message.push(' ');
        message.push_str(values);
    }
    message
}

/// The subcommand named on the command line, if any: the first argument that
/// is not an option.
fn subcommand_in(args: &[OsString]) -> Option<String> {
    args.iter()
        .skip(1)
        .filter_map(|arg| arg.to_str())
        .find(|arg| !arg.starts_with('-'))
        .filter(|arg| SUBCOMMAND_NAMES.contains(arg))
        .map(str::to_string)
}
