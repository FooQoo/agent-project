//! `GET` and `PUT /api/projects/{id}/mode`: the interaction mode of a project (sparring or
//! sleep), saved in the app's internal area by `crate::mode`. The write guard in `api` checks
//! each `PUT` first, and the ngrok entrance serves the same pair behind its session and CSRF
//! token. The board's other streams learn of a change from the `mode` event of `/api/events`.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;

use crate::mode::{self, ModeKind};
use crate::server::AppState;
use crate::server::request::ApiJson;
use crate::server::response::{agpj_error_response, error_response, json_response};
use crate::server::store::parse_project_id;

fn known_project(state: &AppState, project: &str) -> Result<String, Box<Response>> {
    let id = parse_project_id(project).map_err(|error| Box::new(agpj_error_response(&error)))?;
    let id = id.to_string();
    if state.shared.join("projects").join(&id).is_dir() {
        Ok(id)
    } else {
        Err(Box::new(error_response(
            StatusCode::NOT_FOUND,
            "project_not_found",
            "project not found.",
            "No project has this ID.",
        )))
    }
}

async fn get_mode(State(state): State<Arc<AppState>>, Path(project): Path<String>) -> Response {
    let id = match known_project(&state, &project) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let saved = mode::load(&mode::data_dir_of(&state.shared), &id);
    json_response(StatusCode::OK, &saved.api_json())
}

async fn put_mode(
    State(state): State<Arc<AppState>>,
    Path(project): Path<String>,
    ApiJson(body): ApiJson<serde_json::Value>,
) -> Response {
    let id = match known_project(&state, &project) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let kind = body
        .as_object()
        .filter(|fields| fields.len() == 1)
        .and_then(|fields| fields.get("mode"))
        .and_then(serde_json::Value::as_str)
        .and_then(ModeKind::parse);
    let Some(kind) = kind else {
        return error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_mode",
            "mode is invalid.",
            "The body must be {\"mode\": \"sparring\"} or {\"mode\": \"sleep\"}.",
        );
    };
    match mode::set(
        &mode::data_dir_of(&state.shared),
        &id,
        kind,
        chrono::Utc::now(),
    ) {
        Ok(saved) => json_response(StatusCode::OK, &saved.api_json()),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mode_unavailable",
            "mode could not be saved.",
            "The modes file could not be written. The saved mode is unchanged.",
        ),
    }
}

/// The route of the project's mode, served with the other saved settings.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/{project_id}/mode",
        get(get_mode).put(put_mode),
    )
}
