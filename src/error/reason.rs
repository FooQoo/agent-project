use super::{AgpjError, PortHolder};

impl AgpjError {
    /// What to do about it, or why it happened.
    pub fn reason(&self) -> String {
        match self {
            AgpjError::ProjectNotFound { project } => format!(
                "No project matches \"{project}\". Run \"agpj project ls\" to see the projects."
            ),
            AgpjError::ProjectNameExists { .. } => {
                "Project names must be unique. Choose another name.".to_string()
            }
            AgpjError::FolderBoundElsewhere { .. } => {
                "A folder belongs to one project. Choose another folder.".to_string()
            }
            AgpjError::FolderUnbound { .. } => {
                "Create a project here with \"agpj project create NAME\", or bind this folder with \"agpj project bind NAME\".".to_string()
            }
            AgpjError::BoundProjectMissing { .. } => {
                "Bind this folder to a project with \"agpj project bind NAME\".".to_string()
            }
            AgpjError::TaskNotFound { id } => {
                format!("No task has ID {id}. Run \"agpj ls\" to see the tasks.")
            }
            AgpjError::TaskArchived { .. } => "Restore the task before editing it.".into(),
            AgpjError::TaskNotArchived { .. } => "Only an archived task can be deleted here.".into(),
            AgpjError::NoCurrentVersion => {
                "Set one with \"version current <VERSION>\", or name a version.".into()
            }
            AgpjError::ArchiveTargetsChanged => "Read the latest Done tasks and confirm the archive again.".into(),
            AgpjError::AttachmentNotFound { .. } => "The attachment does not belong to this task.".into(),
            AgpjError::AttachmentLimitExceeded => "A task may contain at most 20 images and 50 MiB in total.".into(),
            AgpjError::EpicIdTaken { .. } => "Choose another EPIC-ID.".into(),
            AgpjError::EpicNotFound { .. } => "No epic has this EPIC-ID in the project.".into(),
            AgpjError::EpicInUse { .. } => "Move or clear the tasks that belong to this epic first.".into(),
            AgpjError::InvalidArgument { command, .. } => match command {
                Some(command) => format!("Run \"agpj {command} --help\" for usage."),
                None => "Run \"agpj --help\" for usage.".to_string(),
            },
            AgpjError::InvalidValue { reason, .. } => reason.clone(),
            AgpjError::InvalidStateForColumn { id, column, .. } => format!(
                "Only tasks in Doing can be running or paused. #{id} is in {}.",
                column.label()
            ),
            AgpjError::InvalidStateForAssignee => {
                "Only AI-owned tasks can be running, paused, or have run.".into()
            }
            AgpjError::InvalidStateChange { .. } => {
                "In Doing a task goes from idle to running, from running to paused, and from paused to running. Move the card to change the other states.".into()
            }
            AgpjError::TaskIsDraft { .. } => {
                "Wait for the user to finalize it, then try again.".into()
            }
            AgpjError::TaskInBacklog { .. } => {
                "Send it to Todo first (agpj mv <id> todo --actor human), then try again.".into()
            }
            AgpjError::EpicIsDraft { .. } => {
                "Wait for the user to finalize it, then try again.".into()
            }
            AgpjError::SendWithOpenQuestions { .. } => {
                "Check the questions, then send again with --force (the API takes ack: true)."
                    .into()
            }
            AgpjError::HumanOnly { .. } => {
                "Ask the user to do this in the web UI or run it with --actor human.".into()
            }
            AgpjError::AiOnly { .. } => {
                "An agent does this with --actor ai; a person has no way to do it.".into()
            }
            AgpjError::TaskIsDone { .. } => {
                "A person must move the task out of Done before you can change it.".into()
            }
            AgpjError::AcceptanceUnmet { .. } => {
                "Mark each condition done once it is verified, or ask a person to move the task.".into()
            }
            AgpjError::TaskNotDraft { id } => {
                format!("Only a draft can be saved or finalized this way. #{id} is not a draft.")
            }
            AgpjError::InvalidStateForDraft { id, .. } => format!(
                "Only tasks whose agent status is idle can become drafts. #{id} is not idle."
            ),
            AgpjError::QuestionsUnanswered { .. } => {
                "Wait for the answers with \"agpj message wait\", or see the questions with \"agpj message ls --task ID\".".into()
            }
            AgpjError::QuestionAnswered { .. } => {
                "The first recorded answer is kept. Compare it with the answer you have, and ask again if they differ.".into()
            }
            AgpjError::QuestionWithdrawn { .. } => {
                "A withdrawn question takes no answer. Ask a new question if one is still needed.".into()
            }
            AgpjError::StartBlockedByQuestions { .. } => {
                "Answer the questions (the board, or \"agpj message answer\"), then send again.".into()
            }
            AgpjError::MessageNotFound { .. } => {
                "Run \"agpj message ls --status all\" to see the messages of the project.".into()
            }
            AgpjError::MessageClosed { .. } => {
                "The message already has its reply; there is nothing to withdraw.".into()
            }
            AgpjError::MessageTypeMismatch { expected, .. } => {
                format!("Name a {expected} for this command.")
            }
            AgpjError::RequestAccepted { .. } => {
                "Another agent accepted this request first. Leave it to that agent.".into()
            }
            AgpjError::AnswerAccepted { .. } => {
                "An agent took this answer in before. Read it with \"agpj message show ID\".".into()
            }
            AgpjError::QuestionOpen { .. } => {
                "Only an answered question can be accepted. Wait for the answer with \"agpj message wait\"."
                    .into()
            }
            AgpjError::RequestWithdrawn { .. } => {
                "The person withdrew this request. Nothing is written to a withdrawn request.".into()
            }
            AgpjError::TaskStatusChanged { .. } => {
                "Another change moved the task first. Check its current status and move it again."
                    .into()
            }
            AgpjError::PlannedVersionChanged { .. } => {
                "Another change set the planned version first. Read the task again and choose the version again."
                    .into()
            }
            AgpjError::ConfirmationRequired { .. } => {
                "Run this command yourself in a terminal. Agents cannot delete tasks or projects, or replace a board."
                    .to_string()
            }
            AgpjError::FileError { reason, .. } => reason.clone(),
            AgpjError::FileInvalid { field, reason, .. } => {
                if field.is_empty() {
                    reason.clone()
                } else {
                    format!("{field}: {reason}")
                }
            }
            AgpjError::FileUnreadable { reason, .. } => reason.clone(),
            AgpjError::FormatUnsupported {
                version, expected, ..
            } => format!(
                "Only formatVersion {expected} is supported, found {version}. The file is not converted."
            ),
            AgpjError::ChoiceUnknown { .. } => {
                "Choose one of the choices of the question.".into()
            }
            AgpjError::DatabaseBusy => "Another agpj command is writing. Try again.".to_string(),
            AgpjError::DatabaseError { reason } => reason.clone(),
            AgpjError::AlreadyServing { port, auto: false } => {
                format!("Open http://127.0.0.1:{port}/ or stop the other \"agpj serve\".")
            }
            AgpjError::AlreadyServing { port, auto: true } => format!(
                "A server started automatically is running at http://127.0.0.1:{port}/. Run \"agpj serve -d\" to keep it running, or stop it with \"agpj stop\"."
            ),
            AgpjError::PortInUse { port, holder } => port_in_use_reason(*port, *holder),
            AgpjError::LocalNetworkDenied => LOCAL_NETWORK_DENIED.to_string(),
            AgpjError::BoardUnavailable { reason } => reason.clone(),
            AgpjError::CursorInvalid { reason } => reason.clone(),
            AgpjError::WaitTimeout => {
                "Nothing changed in the project. This is not an answer; read the snapshot before acting."
                    .to_string()
            }
            AgpjError::DocsPathInvalid { reason } => reason.clone(),
            AgpjError::DocsNotFound { .. } => {
                "No such file in the project's Docs. Choose a file from the tree.".to_string()
            }
            AgpjError::DocsNotMarkdown { .. } => {
                "Only files ending in .md or .mdx can be opened.".to_string()
            }
            AgpjError::DocsTooLarge { limit, .. } => {
                format!("Files larger than {limit} bytes cannot be opened here.")
            }
            AgpjError::DocsNotUtf8 { .. } => {
                "Only UTF-8 text can be opened here. The file is not changed.".to_string()
            }
            AgpjError::DocsAssetType { .. } => {
                "Only PNG, JPEG, WebP, GIF, AVIF and SVG images are served.".to_string()
            }
            AgpjError::DocsNotImplemented => {
                "The Docs API is still being built. Try again in a later version.".to_string()
            }
            AgpjError::DocsOpenFailed { reason } => reason.clone(),
            AgpjError::ReleaseDocRefused { reason, .. } => reason.clone(),
            AgpjError::RemoteUnavailable { reason } => reason.clone(),
            AgpjError::InvalidDomain => "Give a host name only, such as example.ngrok-free.dev, without https://, a path, or a port.".to_string(),
            AgpjError::KeychainUnavailable { reason } => reason.clone(),
            AgpjError::BrowserOpenFailed { url, reason } => {
                format!("{reason} Open {url} in your browser yourself.")
            }
            AgpjError::AgentCliMissing { tool } => format!(
                "Install {tool} and make it available on the PATH of the server, then try again."
            ),
            AgpjError::AgentCliLoggedOut { tool } => {
                format!("Log in to {tool} in a terminal, then try again.")
            }
            AgpjError::SessionLaunchRefused { reason } => reason.clone(),
            AgpjError::SessionLaunchFailed { reason } => reason.clone(),
            AgpjError::ProjectHasNoFolder { project } => format!(
                "Bind a folder to project {project} with \"agpj project bind\" in that folder."
            ),
        }
    }
}

/// Why `agpj` cannot listen on or reach 127.0.0.1, and the fix for Codex.
const LOCAL_NETWORK_DENIED: &str = "This environment does not allow local network connections, so the Agent Board server cannot be started or reached. In Codex, run \"agpj setup\" to allow agpj to run outside the sandbox.";

fn port_in_use_reason(port: u16, holder: PortHolder) -> String {
    match holder {
        PortHolder::AgentBoard { pid } => format!(
            "Port {port} is used by another Agent Board server (pid {pid}) with a different data folder. Choose another port with --port."
        ),
        PortHolder::OtherProgram => {
            format!("Port {port} is used by another program. Choose another port with --port.")
        }
        PortHolder::Unknown => "Choose another port with --port.".to_string(),
    }
}
