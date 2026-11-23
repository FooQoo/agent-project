//! Recording a publication of a task in a version, or the delivery of a result plan, from the
//! board: the JSON route and the multipart route that also takes images.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Multipart, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use serde::Deserialize;

use super::multipart::{MultipartAttachmentMeta, parse_multipart};
use super::parse_task_id;
use crate::domain::{AttachmentInput, ResultKind};
use crate::file_store::ResultInput;
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agpj_error_response, error_response, json_response};
use crate::server::store::{parse_project_id, refresh_index, with_request_files};

/// The routes that read and record a task's results.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/{project_id}/tasks/{id}/results",
            get(super::list_task_results).post(record_publication),
        )
        .route(
            "/api/projects/{project_id}/tasks/{id}/results/multipart",
            post(record_publication_multipart),
        )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PublicationInput {
    /// A version name: the body records a publication (`link` required). Without it the body
    /// delivers a result, optionally against a result plan.
    published: Option<String>,
    link: Option<String>,
    /// Required, except that a delivery against `plan` defaults to the plan's content.
    #[serde(default)]
    summary: String,
    title: Option<String>,
    key: Option<String>,
    /// A result plan ID (`RP-<n>`) of the task this result delivers.
    plan: Option<String>,
    /// Marks the result final (implied by a final plan).
    #[serde(default, rename = "final")]
    is_final: bool,
}

/// `POST .../tasks/{id}/results`: record a publication or a delivery of a plan and answer
/// `{task, result, created}`; `created` is false when the same record already existed.
pub(super) async fn record_publication(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<PublicationInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let result = add_result(&state, &headers, project_id, id, input, Vec::new()).await;
    result_response(&state, result)
}

/// The metadata part of `POST .../tasks/{id}/results/multipart`: the fields of the JSON body, and
/// one entry in `attachments` for each image part, in order.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublicationMeta {
    published: Option<String>,
    link: Option<String>,
    #[serde(default)]
    summary: String,
    title: Option<String>,
    key: Option<String>,
    plan: Option<String>,
    #[serde(default, rename = "final")]
    is_final: bool,
    #[serde(default)]
    attachments: Vec<MultipartAttachmentMeta>,
}

impl super::multipart::ImageList for PublicationMeta {
    fn images(&self) -> &[MultipartAttachmentMeta] {
        &self.attachments
    }
}

/// `POST .../tasks/{id}/results/multipart`: the same delivery as the JSON route, with images.
pub(super) async fn record_publication_multipart(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    multipart: Result<Multipart, axum::extract::multipart::MultipartRejection>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let Ok(multipart) = multipart else {
        return super::multipart::multipart_too_large();
    };
    let payload = match parse_multipart::<PublicationMeta>(multipart).await {
        Ok(value) => value,
        Err(response) => return response,
    };
    let meta = payload.meta;
    if meta.published.is_some() && !payload.additions.is_empty() {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_value",
            "request body is invalid.",
            "A publication takes no images.",
        );
    }
    let result = add_result(
        &state,
        &headers,
        project_id,
        id,
        PublicationInput {
            published: meta.published,
            link: meta.link,
            summary: meta.summary,
            title: meta.title,
            key: meta.key,
            plan: meta.plan,
            is_final: meta.is_final,
        },
        payload.additions,
    )
    .await;
    result_response(&state, result)
}

async fn add_result(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    project_id: crate::domain::ProjectId,
    id: i64,
    input: PublicationInput,
    images: Vec<AttachmentInput>,
) -> Result<crate::file_store::RecordedResult, crate::error::AgpjError> {
    with_request_files(state, headers, move |store| {
        store.add_result(
            &project_id,
            id,
            ResultInput {
                kind: if input.is_final {
                    ResultKind::Final
                } else {
                    ResultKind::Progress
                },
                title: input.title,
                summary: input.summary,
                link: input.link,
                images,
                key: input.key,
                published: input.published,
                plan: input.plan,
            },
            crate::clock::now(),
        )
    })
    .await
}

fn result_response(
    state: &Arc<AppState>,
    result: Result<crate::file_store::RecordedResult, crate::error::AgpjError>,
) -> Response {
    match result {
        Ok(recorded) => {
            if recorded.created {
                refresh_index(state);
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
        Err(error) => agpj_error_response(&error),
    }
}
