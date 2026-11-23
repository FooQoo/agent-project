//! Editing a task: the text of a draft, the draft flag, and the task's epic.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::{Deserialize, Deserializer};

use super::epics::{whole_or_parts, whole_text};
use super::{changed_task_response, parse_task_id};
use crate::domain::{Actor, Draft};
use crate::error::AgpjError;
use crate::file_store::frontmatter::{self, AcceptanceItem, ResultPlan};
use crate::file_store::validate::{self, EpicId};
use crate::file_store::{DraftSave, TaskTextParts};
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agpj_error_response, error_response};
use crate::server::store::{parse_project_id, with_request_files};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EditTaskInput {
    title: Option<String>,
    description: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    acceptance: Option<Vec<AcceptanceItem>>,
    /// The planned results; an `id` left out is assigned by the server.
    #[serde(default)]
    result_plans: Option<Vec<ResultPlan>>,
    #[serde(default)]
    finalize: bool,
    /// `"human"` keeps the draft after the save.
    #[serde(default, deserialize_with = "super::draft_wire::named_draft")]
    draft: Option<Option<crate::domain::Draft>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DraftInput {
    /// `"human"` makes the task a person's draft, `null` finalizes it.
    #[serde(default, deserialize_with = "super::draft_wire::present_draft")]
    draft: Option<Option<crate::domain::Draft>>,
    /// Leaving the draft discards its changes instead of confirming them.
    #[serde(default)]
    discard: bool,
}

/// The whole text for a new task: the given text, or one written from the body, the conditions,
/// and the plans over the template. The store writes its template for an empty result. Plans
/// are parts, so they cannot come with a whole text.
pub(super) fn create_description(
    given: Option<String>,
    body: Option<String>,
    acceptance: Option<Vec<AcceptanceItem>>,
    result_plans: Option<Vec<ResultPlan>>,
) -> Result<String, AgpjError> {
    if result_plans.is_some() && given.is_some() {
        return Err(whole_or_parts());
    }
    let text = whole_text(given, body, acceptance, validate::task_template)?.unwrap_or_default();
    let Some(plans) = result_plans else {
        return Ok(text);
    };
    let base = if text.trim().is_empty() {
        validate::task_template()
    } else {
        text
    };
    frontmatter::compose_result_plans(&base, &plans).map_err(|defect| AgpjError::InvalidValue {
        field: defect.field,
        reason: defect.reason,
    })
}

/// Save the title and description of a draft, and confirm it with `finalize`.
pub(super) async fn edit_task(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<EditTaskInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if input.title.is_none()
        && input.description.is_none()
        && input.body.is_none()
        && input.acceptance.is_none()
        && input.result_plans.is_none()
        && !input.finalize
        && input.draft.is_none()
    {
        return agpj_error_response(&AgpjError::InvalidArgument {
            message: "Nothing to change.".to_string(),
            command: None,
        });
    }
    let finalize = DraftSave {
        target: finalize_target(input.finalize, input.draft),
    };
    let result = with_request_files(&state, &headers, move |store| {
        if input.body.is_some() || input.acceptance.is_some() || input.result_plans.is_some() {
            if input.description.is_some() {
                return Err(whole_or_parts());
            }
            store.edit_draft_task_parts_to(
                &project_id,
                id,
                input.title.as_deref(),
                TaskTextParts {
                    body: input.body.as_deref(),
                    acceptance: input.acceptance.as_deref(),
                    result_plans: input.result_plans.as_deref(),
                },
                finalize,
                crate::clock::now(),
            )
        } else {
            store.edit_draft_task_to(
                &project_id,
                id,
                input.title.as_deref(),
                input.description.as_deref(),
                finalize,
                crate::clock::now(),
            )
        }
    })
    .await;
    changed_task_response(&state, result)
}

/// Turn the draft flag of a task on or off.
pub(super) async fn set_draft(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<DraftInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let Some(draft) = input.draft else {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_value",
            "request body is invalid.",
            "draft is required (\"human\" or null).",
        );
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.set_draft(&project_id, id, draft, input.discard, crate::clock::now())
    })
    .await;
    changed_task_response(&state, result)
}

/// `epicId` of `PUT .../epic`: an EPIC-ID, or `null` to take the task out of
/// its epic. A missing field is `None` and is refused.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SetEpicInput {
    #[serde(default, deserialize_with = "present_or_null")]
    epic_id: Option<Option<String>>,
}

pub(super) fn present_or_null<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Set or clear the epic of a task. A person may do this on any task, including
/// an archived one or a draft.
pub(super) async fn set_task_epic(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<SetEpicInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let epic = match input.epic_id {
        None => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_value",
                "request body is invalid.",
                "epicId is required (an EPIC-ID, or null).",
            );
        }
        Some(None) => None,
        Some(Some(raw)) => match raw.parse::<EpicId>() {
            Ok(id) => Some(id),
            Err(error) => return agpj_error_response(&error),
        },
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.set_task_epic(
            &project_id,
            id,
            epic.as_ref(),
            Actor::Human,
            crate::clock::now(),
        )
    })
    .await;
    changed_task_response(&state, result)
}

/// What a saved edit does to the draft: nothing, or the draft it ends as.
pub(super) fn finalize_target(
    finalize: bool,
    draft: Option<Option<Draft>>,
) -> Option<Option<Draft>> {
    match (finalize, draft) {
        (_, Some(owner)) => Some(owner),
        (true, None) => Some(None),
        (false, None) => None,
    }
}
