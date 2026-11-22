//! The JSON error body and the responses every router shares.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::error::AgpjError;

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
    reason: String,
}

pub(crate) fn error_response(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    reason: impl Into<String>,
) -> Response {
    let body = ErrorBody {
        error: ErrorDetail {
            code: code.to_string(),
            message: message.into(),
            reason: reason.into(),
        },
    };
    json_response(status, &body)
}

pub(crate) fn agboard_error_response(error: &AgpjError) -> Response {
    let status = match error {
        AgpjError::TaskNotFound { .. }
        | AgpjError::ProjectNotFound { .. }
        | AgpjError::AttachmentNotFound { .. }
        | AgpjError::EpicNotFound { .. }
        | AgpjError::MessageNotFound { .. } => StatusCode::NOT_FOUND,
        AgpjError::HumanOnly { .. } | AgpjError::AiOnly { .. } => StatusCode::FORBIDDEN,
        AgpjError::QuestionsUnanswered { .. }
        | AgpjError::QuestionAnswered { .. }
        | AgpjError::QuestionWithdrawn { .. }
        | AgpjError::StartBlockedByQuestions { .. }
        | AgpjError::MessageClosed { .. }
        | AgpjError::RequestAccepted { .. }
        | AgpjError::AnswerAccepted { .. }
        | AgpjError::QuestionOpen { .. }
        | AgpjError::RequestWithdrawn { .. }
        | AgpjError::TaskStatusChanged { .. }
        | AgpjError::PlannedVersionChanged { .. }
        | AgpjError::TaskIsDraft { .. }
        | AgpjError::TaskInBacklog { .. }
        | AgpjError::EpicIsDraft { .. }
        | AgpjError::SendWithOpenQuestions { .. }
        | AgpjError::TaskIsDone { .. }
        | AgpjError::TaskNotDraft { .. }
        | AgpjError::InvalidStateForDraft { .. }
        | AgpjError::ProjectNameExists { .. }
        | AgpjError::EpicIdTaken { .. }
        | AgpjError::EpicInUse { .. }
        | AgpjError::AgentCliMissing { .. }
        | AgpjError::AgentCliLoggedOut { .. }
        | AgpjError::SessionLaunchRefused { .. }
        | AgpjError::ProjectHasNoFolder { .. } => StatusCode::CONFLICT,
        AgpjError::TaskArchived { .. }
        | AgpjError::TaskNotArchived { .. }
        | AgpjError::ArchiveTargetsChanged => StatusCode::CONFLICT,
        AgpjError::InvalidArgument { .. }
        | AgpjError::InvalidValue { .. }
        | AgpjError::NoCurrentVersion
        | AgpjError::FileInvalid { .. }
        | AgpjError::InvalidStateForColumn { .. }
        | AgpjError::InvalidStateForAssignee
        | AgpjError::InvalidStateChange { .. }
        | AgpjError::ChoiceUnknown { .. }
        | AgpjError::MessageTypeMismatch { .. }
        | AgpjError::AttachmentLimitExceeded => StatusCode::UNPROCESSABLE_ENTITY,
        AgpjError::DatabaseBusy => StatusCode::SERVICE_UNAVAILABLE,
        AgpjError::DocsPathInvalid { .. } => StatusCode::BAD_REQUEST,
        AgpjError::DocsNotFound { .. } => StatusCode::NOT_FOUND,
        AgpjError::ReleaseDocRefused { .. } => StatusCode::CONFLICT,
        AgpjError::DocsNotMarkdown { .. } | AgpjError::DocsAssetType { .. } => {
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        }
        AgpjError::DocsTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
        AgpjError::DocsNotUtf8 { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        AgpjError::DocsNotImplemented => StatusCode::NOT_IMPLEMENTED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    error_response(status, error.code(), error.message(), error.reason())
}

/// A `sections` query that is not `document`, `history` or `document,history`.
pub(crate) fn invalid_sections() -> Response {
    error_response(
        StatusCode::BAD_REQUEST,
        super::contracts::INVALID_SECTIONS,
        "sections is invalid.",
        "sections must be document, history or document,history.",
    )
}

pub(crate) fn forbidden(reason: &str) -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        "forbidden",
        "request is not allowed.",
        reason,
    )
}

pub(crate) fn json_response<T: Serialize>(status: StatusCode, body: &T) -> Response {
    match serde_json::to_vec(body) {
        Ok(bytes) => (
            status,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

/// The page holds a token from an earlier server (the server was restarted). Its writes are refused with a
/// code of their own, so the page can ask the person to reload instead of retrying.
pub(crate) fn page_out_of_date() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        "page_out_of_date",
        "The server was restarted, so this page is out of date.",
        "Reload the page to load the current token. Changes made on this page are not saved.",
    )
}

/// A stored file's bytes with the headers that stop a browser from sniffing them.
pub(crate) fn file_response(content_type: &str, bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CACHE_CONTROL, "no-store".to_string()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
        ],
        bytes,
    )
        .into_response()
}
