use super::Command;

impl Command {
    /// The subcommand's name as typed on the command line.
    pub fn name(&self) -> &'static str {
        match self {
            Command::Project(_) => "project",
            Command::Init(_) => "init",
            Command::Add(_) => "add",
            Command::Edit(_) => "edit",
            Command::Acceptance(_) => "acceptance",
            Command::Mv(_) => "mv",
            Command::State(_) => "state",
            Command::Connect(_) => "connect",
            Command::Disconnect => "disconnect",
            Command::Agent(_) => "agent",
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
