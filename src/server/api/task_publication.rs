//! Recording that a task was published in a version, from the board or remote access.
//! It is the one result a person can add through the API, and the one a Done task still takes.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use super::parse_task_id;
use crate::domain::ResultKind;
use crate::file_store::ResultInput;
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agboard_error_response, json_response};
use crate::server::store::{parse_project_id, refresh_index, with_request_files};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PublicationInput {
    published: String,
    link: String,
    summary: String,
    title: Option<String>,
    key: Option<String>,
}

/// `POST .../tasks/{id}/results`: record a publication result and answer
/// `{task, result, created}`; `created` is false when the same record already existed.
pub(super) async fn record_publication(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<PublicationInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.add_result(
            &project_id,
            id,
            ResultInput {
                kind: ResultKind::Progress,
                title: input.title,
                summary: input.summary,
                link: Some(input.link),
                images: Vec::new(),
                key: input.key,
                published: Some(input.published),
            },
            chrono::Utc::now(),
        )
    })
    .await;
    match result {
        Ok(recorded) => {
            if recorded.created {
                refresh_index(&state);
            }
            let status = if recorded.created {
                StatusCode::CREATED
            } else {
                StatusCode::OK
            };
            json_response(
                status,
                &serde_json::json!({
                    "task": recorded.task,
                    "result": recorded.result,
                    "created": recorded.created,
                }),
            )
        }
        Err(error) => agboard_error_response(&error),
    }
}
