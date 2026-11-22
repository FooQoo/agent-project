mod messages;

use super::*;

pub(super) fn all_errors() -> Vec<AgpjError> {
    vec![
        AgpjError::ProjectNotFound {
            project: "Alpha".into(),
        },
        AgpjError::ProjectNameExists {
            name: "Alpha".into(),
        },
        AgpjError::TaskNotFound { id: 7 },
        AgpjError::InvalidArgument {
            message: "unexpected argument '--x'".into(),
            command: Some("add".into()),
        },
        AgpjError::InvalidValue {
            field: "title".into(),
            reason: "r".into(),
        },
        AgpjError::InvalidStateForColumn {
            id: 11,
            state: RunState::Running,
            column: Column::Todo,
        },
        AgpjError::InvalidStateForAssignee,
        AgpjError::InvalidStateChange {
            from: RunState::Ran,
            to: RunState::Idle,
        },
        AgpjError::NoCurrentVersion,
        AgpjError::TaskIsDraft { id: 5 },
        AgpjError::TaskInBacklog { id: 5 },
        AgpjError::EpicIsDraft { id: "E".into() },
        AgpjError::SendWithOpenQuestions {
            ids: vec!["Q-1".into()],
        },
        AgpjError::HumanOnly {
            action: "start".into(),
        },
        AgpjError::TaskIsDone { id: 5 },
        AgpjError::AcceptanceUnmet {
            id: 5,
            unmet: vec!["AC-2".into()],
        },
        AgpjError::TaskNotDraft { id: 5 },
        AgpjError::InvalidStateForDraft {
            id: 6,
            state: RunState::Running,
        },
        AgpjError::QuestionsUnanswered { id: 5 },
        AgpjError::QuestionAnswered {
            ids: vec!["Q-1".into()],
        },
        AgpjError::QuestionWithdrawn {
            ids: vec!["Q-2".into()],
        },
        AgpjError::StartBlockedByQuestions {
            ids: vec!["Q-2".into()],
        },
        AgpjError::MessageNotFound { id: "Q-3".into() },
        AgpjError::MessageClosed { id: "Q-3".into() },
        AgpjError::MessageTypeMismatch {
            id: "Q-3".into(),
            expected: "request".into(),
        },
        AgpjError::RequestAccepted { id: "Q-4".into() },
        AgpjError::AnswerAccepted { id: "Q-4".into() },
        AgpjError::QuestionOpen { id: "Q-4".into() },
        AgpjError::RequestWithdrawn { id: "Q-4".into() },
        AgpjError::AiOnly {
            action: "withdrawing a question".into(),
        },
        AgpjError::TaskStatusChanged {
            id: 5,
            current: Column::Done,
        },
        AgpjError::PlannedVersionChanged {
            id: 5,
            current: Some("v1".into()),
        },
        AgpjError::ConfirmationRequired {
            operation: "deleting a task".into(),
        },
        AgpjError::FileError {
            action: FileAction::Read,
            file: "a.json".into(),
            reason: "No such file or directory (os error 2)".into(),
        },
        AgpjError::FileInvalid {
            file: "projects/p/tasks/1/task.json".into(),
            field: "title".into(),
            reason: "Titles must be 1 to 100 characters on one line.".into(),
        },
        AgpjError::FileUnreadable {
            file: "projects/p/project.json".into(),
            reason: "Permission denied (os error 13)".into(),
        },
        AgpjError::FormatUnsupported {
            file: "workspace.json".into(),
            version: "2".into(),
            expected: 1,
        },
        AgpjError::ChoiceUnknown {
            question_id: "Q-1".into(),
            choice_id: "b".into(),
        },
        AgpjError::DatabaseBusy,
        AgpjError::DatabaseError {
            reason: "boom".into(),
        },
        AgpjError::AlreadyServing {
            port: 7420,
            auto: false,
        },
        AgpjError::PortInUse {
            port: 7420,
            holder: PortHolder::Unknown,
        },
        AgpjError::LocalNetworkDenied,
        AgpjError::BoardUnavailable {
            reason: "no running server".into(),
        },
        AgpjError::CursorInvalid {
            reason: "wrong project".into(),
        },
        AgpjError::WaitTimeout,
        AgpjError::DocsPathInvalid {
            reason: "Paths are relative and use /.".into(),
        },
        AgpjError::DocsNotFound {
            path: "a/b.md".into(),
        },
        AgpjError::DocsNotMarkdown {
            path: "a.txt".into(),
        },
        AgpjError::DocsTooLarge {
            path: "a.md".into(),
            limit: 2_097_152,
        },
        AgpjError::DocsNotUtf8 {
            path: "a.md".into(),
        },
        AgpjError::DocsAssetType {
            path: "a.svg".into(),
        },
        AgpjError::DocsNotImplemented,
        AgpjError::DocsOpenFailed {
            reason: "xdg-open is not installed".into(),
        },
        AgpjError::ReleaseDocRefused {
            file: "docs/release/v1.8.0.md".into(),
            reason: "The file changed while it was generated.".into(),
        },
        AgpjError::RemoteUnavailable {
            reason: "No server is running.".into(),
        },
        AgpjError::InvalidDomain,
        AgpjError::KeychainUnavailable {
            reason: "The keychain is not available.".into(),
        },
        AgpjError::BrowserOpenFailed {
            url: "http://127.0.0.1:7420/?project=0123456789abcdef0123456789abcdef".into(),
            reason: "Cannot start xdg-open: No such file or directory (os error 2).".into(),
        },
        AgpjError::AgentCliMissing {
            tool: "claude".into(),
        },
        AgpjError::AgentCliLoggedOut {
            tool: "codex".into(),
        },
        AgpjError::SessionLaunchRefused {
            reason: "A board-launched claude-code session already exists.".into(),
        },
        AgpjError::SessionLaunchFailed {
            reason: "claude could not be started.".into(),
        },
        AgpjError::ProjectHasNoFolder {
            project: "demo".into(),
        },
    ]
}

#[test]
fn codes_and_exit_codes_follow_the_design_table() {
    let got: Vec<(&str, i32)> = all_errors()
        .iter()
        .map(|e| (e.code(), e.exit_code()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("project_not_found", 4),
            ("project_name_exists", 2),
            ("task_not_found", 3),
            ("invalid_argument", 2),
            ("invalid_value", 2),
            ("invalid_state_for_column", 1),
            ("invalid_state_for_assignee", 2),
            ("invalid_state_change", 2),
            ("no_current_version", 1),
            ("task_is_draft", 1),
            ("task_in_backlog", 1),
            ("epic_is_draft", 1),
            ("send_with_open_questions", 1),
            ("human_only", 1),
            ("task_is_done", 1),
            ("acceptance_unmet", 2),
            ("task_not_draft", 1),
            ("invalid_state_for_draft", 1),
            ("questions_unanswered", 1),
            ("question_answered", 1),
            ("question_withdrawn", 1),
            ("start_blocked_by_questions", 1),
            ("message_not_found", 3),
            ("message_closed", 1),
            ("message_type_mismatch", 2),
            ("request_accepted", 1),
            ("answer_accepted", 1),
            ("question_open", 1),
            ("request_withdrawn", 1),
            ("ai_only", 1),
            ("task_status_changed", 1),
            ("planned_version_changed", 1),
            ("confirmation_required", 1),
            ("file_error", 1),
            ("file_invalid", 1),
            ("file_unreadable", 1),
            ("format_unsupported", 1),
            ("choice_unknown", 2),
            ("database_busy", 1),
            ("database_error", 1),
            ("already_serving", 1),
            ("port_in_use", 1),
            ("local_network_denied", 1),
            ("board_unavailable", 1),
            ("cursor_invalid", 1),
            ("wait_timeout", 1),
            ("docs_path_invalid", 1),
            ("docs_not_found", 1),
            ("docs_not_markdown", 1),
            ("docs_too_large", 1),
            ("docs_not_utf8", 1),
            ("docs_asset_type", 1),
            ("docs_not_implemented", 1),
            ("docs_open_failed", 1),
            ("release_doc_refused", 1),
            ("remote_unavailable", 1),
            ("invalid_domain", 2),
            ("keychain_unavailable", 1),
            ("browser_open_failed", 1),
            ("agent_cli_missing", 1),
            ("agent_cli_logged_out", 1),
            ("session_launch_refused", 1),
            ("session_launch_failed", 1),
            ("project_has_no_folder", 1),
        ]
    );
}

#[test]
fn invalid_argument_without_command_points_to_top_level_help() {
    let e = AgpjError::InvalidArgument {
        message: "bad".into(),
        command: None,
    };
    assert_eq!(e.reason(), "Run \"agboard --help\" for usage.");
}

#[test]
fn write_file_error_says_cannot_write() {
    let e = AgpjError::FileError {
        action: FileAction::Write,
        file: "out.json".into(),
        reason: "denied".into(),
    };
    assert_eq!(e.message(), "cannot write out.json.");
}

#[test]
fn display_is_the_message() {
    assert_eq!(AgpjError::DatabaseBusy.to_string(), "the board is busy.");
}
