use super::{AgpjError, FileAction};

impl AgpjError {
    /// One-line summary of what went wrong.
    pub fn message(&self) -> String {
        match self {
            AgpjError::ProjectNotFound { .. } => "project not found.".to_string(),
            AgpjError::FolderBoundElsewhere { .. } => {
                "this folder already belongs to another project.".to_string()
            }
            AgpjError::FolderUnbound { .. } => "this folder is not bound to a project.".to_string(),
            AgpjError::BoundProjectMissing { .. } => {
                "the project bound to this folder no longer exists.".to_string()
            }
            AgpjError::ProjectNameExists { name } => {
                format!("project \"{name}\" already exists.")
            }
            AgpjError::TaskNotFound { id } => format!("task #{id} not found."),
            AgpjError::ResultPlanNotFound { id, plan } => {
                format!("task #{id} has no result plan {plan}.")
            }
            AgpjError::ResultPlanDelivered { id, plan, version } => {
                format!(
                    "result plan {plan} of task #{id} is already delivered by result v{version}."
                )
            }
            AgpjError::TaskArchived { id } => format!("task #{id} is archived."),
            AgpjError::TaskNotArchived { id } => format!("task #{id} is not archived."),
            AgpjError::NoReleaseVersion => "the project has no release version.".into(),
            AgpjError::ArchiveTargetsChanged => "archive targets changed.".into(),
            AgpjError::TaskDocumentChanged { .. } => "task document changed after review.".into(),
            AgpjError::AttachmentNotFound { id } => format!("attachment {id} not found."),
            AgpjError::AttachmentLimitExceeded => "attachment limits were exceeded.".into(),
            AgpjError::EpicIdTaken { id } => format!("epic {id} already exists."),
            AgpjError::EpicNotFound { id } => format!("epic {id} not found."),
            AgpjError::EpicInUse { id } => format!("epic {id} is used by tasks."),
            AgpjError::InvalidArgument { message, .. } => message.clone(),
            AgpjError::InvalidValue { field, .. } => format!("{field} is invalid."),
            AgpjError::InvalidStateForColumn { id, state, .. } => {
                format!("cannot set #{id} to {}.", state.as_str())
            }
            AgpjError::InvalidStateForAssignee => "state is invalid for this assignee.".into(),
            AgpjError::InvalidStateChange { from, to } => {
                format!(
                    "cannot change the state from {} to {}.",
                    from.as_str(),
                    to.as_str()
                )
            }
            AgpjError::BrainstormNotActive => {
                "the project is not in brainstorm mode, so brainstorm questions are not accepted."
                    .into()
            }
            AgpjError::TaskIsDraft { id } => {
                format!("task #{id} is a draft and read-only for agents.")
            }
            AgpjError::TaskInBacklog { id } => {
                format!("task #{id} is in the Backlog and has not been sent to Todo.")
            }
            AgpjError::EpicIsDraft { id } => {
                format!("epic {id} is a draft and read-only for the CLI.")
            }
            AgpjError::SendWithOpenQuestions { ids } => {
                format!("the plan has open questions: {}.", ids.join(", "))
            }
            AgpjError::NotConnected => "This session is not connected to an agent.".to_string(),
            AgpjError::ConnectionReplaced => {
                "This session's connection was replaced or revoked.".to_string()
            }
            AgpjError::NotAssignee { id } => {
                format!("Task #{id} is not assigned to this session's agent.")
            }
            AgpjError::AgentBusy { agent } => {
                format!("Agent {agent} already has an active connection.")
            }
            AgpjError::AgentInUse { agent } => {
                format!("Agent {agent} still has a Doing task or an active connection.")
            }
            AgpjError::AgentNotFound { agent } => format!("There is no agent {agent}."),
            AgpjError::AgentNameTaken { name } => {
                format!("Another agent is already named {name}.")
            }
            AgpjError::ConnectCodeInvalid => {
                "The connection code is unknown, already used, or expired.".to_string()
            }
            AgpjError::HumanOnly { action } => format!("{action} is for a person only."),
            AgpjError::AiOnly { action } => format!("{action} is for an agent only."),
            AgpjError::TaskIsDone { id } => {
                format!("task #{id} is done and read-only for agents.")
            }
            AgpjError::AcceptanceUnmet { id, unmet } => format!(
                "task #{id} has unmet acceptance conditions: {}.",
                unmet.join(", ")
            ),
            AgpjError::TaskNotDraft { id } => format!("task #{id} is not a draft."),
            AgpjError::InvalidStateForDraft { id, state } => {
                format!("cannot make #{id} a draft while it is {}.", state.as_str())
            }
            AgpjError::QuestionsUnanswered { id } => {
                format!("task #{id} waits for answers to its questions.")
            }
            AgpjError::QuestionAnswered { ids } => {
                format!("{} already answered.", ids.join(", "))
            }
            AgpjError::QuestionWithdrawn { ids } => {
                format!("{} withdrawn.", ids.join(", "))
            }
            AgpjError::StartBlockedByQuestions { ids } => {
                format!("cannot send while {} has no answer.", ids.join(", "))
            }
            AgpjError::MessageNotFound { id } => format!("message {id} not found."),
            AgpjError::MessageClosed { id } => {
                format!("message {id} already has its reply.")
            }
            AgpjError::MessageTypeMismatch { id, expected } => {
                format!("{id} is not a {expected}.")
            }
            AgpjError::RequestAccepted { id } => format!("request {id} was already accepted."),
            AgpjError::AnswerAccepted { id } => {
                format!("the answer of {id} was already accepted.")
            }
            AgpjError::QuestionOpen { id } => format!("question {id} has no answer yet."),
            AgpjError::RequestWithdrawn { id } => format!("request {id} was withdrawn."),
            AgpjError::TaskStatusChanged { id, current } => {
                format!("task #{id} is now {}.", current.as_str())
            }
            AgpjError::PlannedVersionChanged { id, .. } => {
                format!("the planned version of task #{id} changed.")
            }
            AgpjError::ConfirmationRequired { operation } => {
                format!("{operation} needs confirmation from a human.")
            }
            AgpjError::FileError { action, file, .. } => match action {
                FileAction::Read => format!("cannot read {file}."),
                FileAction::Write => format!("cannot write {file}."),
            },
            AgpjError::FileInvalid { file, .. } => format!("{file} is not valid."),
            AgpjError::FileUnreadable { file, .. } => format!("cannot read {file}."),
            AgpjError::FormatUnsupported { file, .. } => {
                format!("{file} has an unsupported format version.")
            }
            AgpjError::ChoiceUnknown { .. } => "the choice is not part of this question.".into(),
            AgpjError::DatabaseBusy => "the board is busy.".to_string(),
            AgpjError::DatabaseError { .. } => "cannot open the board.".to_string(),
            AgpjError::AlreadyServing { .. } => "the board is already being served.".to_string(),
            AgpjError::PortInUse { port, .. } => format!("port {port} is already in use."),
            AgpjError::LocalNetworkDenied => {
                "local network connections are not allowed.".to_string()
            }
            AgpjError::BoardUnavailable { .. } => "the board is unavailable.".to_string(),
            AgpjError::CursorInvalid { .. } => "the cursor is invalid.".to_string(),
            AgpjError::WaitTimeout => "no board change before the timeout.".to_string(),
            AgpjError::DocsPathInvalid { .. } => "the document path is invalid.".to_string(),
            AgpjError::DocsNotFound { path } => format!("document {path} not found."),
            AgpjError::DocsNotMarkdown { path } => format!("{path} is not a Markdown file."),
            AgpjError::DocsTooLarge { path, .. } => format!("{path} is too large."),
            AgpjError::DocsNotUtf8 { path } => format!("{path} is not UTF-8 text."),
            AgpjError::DocsAssetType { path } => format!("{path} is not a supported image."),
            AgpjError::DocsNotImplemented => {
                "this document operation is not available yet.".to_string()
            }
            AgpjError::DocsOpenFailed { .. } => "cannot open the Docs folder.".to_string(),
            AgpjError::ReleaseDocRefused { file, .. } => format!("did not write {file}."),
            AgpjError::RemoteUnavailable { .. } => {
                "the ngrok connection is not available.".to_string()
            }
            AgpjError::InvalidDomain => "the ngrok domain is not valid.".to_string(),
            AgpjError::KeychainUnavailable { .. } => {
                "secure storage on this computer is not available.".to_string()
            }
            AgpjError::BrowserOpenFailed { .. } => "cannot open the browser.".to_string(),
            AgpjError::AgentCliMissing { tool } => {
                format!("the {tool} command is not installed.")
            }
            AgpjError::AgentCliLoggedOut { tool } => format!("{tool} is not logged in."),
            AgpjError::SessionLaunchRefused { .. } => "the session was not launched.".to_string(),
            AgpjError::SessionLaunchFailed { .. } => {
                "the session could not be started.".to_string()
            }
            AgpjError::ProjectHasNoFolder { .. } => "the project has no bound folder.".to_string(),
        }
    }
}
