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

use crate::mode::{self, ModeKind, Schedule};
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
    json_response(StatusCode::OK, &current_json(&state, &id))
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
    let data_dir = mode::data_dir_of(&state.shared);
    let saved = match parse_change(&body) {
        Some(Change::Mode(kind)) => mode::set(&data_dir, &id, kind, chrono::Utc::now()).map(drop),
        Some(Change::Auto) => mode::clear_manual(&data_dir, &id),
        Some(Change::Schedule(schedule)) => mode::set_schedule(&data_dir, &id, schedule),
        None => {
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_mode",
                "mode is invalid.",
                "The body must be {\"mode\": \"sparring\" | \"sleep\" | \"auto\"} or {\"schedule\": {\"enabled\": true, \"start\": \"23:00\", \"end\": \"08:00\"}}.",
            );
        }
    };
    match saved {
        Ok(()) => json_response(StatusCode::OK, &current_json(&state, &id)),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "mode_unavailable",
            "mode could not be saved.",
            "The modes file could not be written. The saved mode is unchanged.",
        ),
    }
}

enum Change {
    Mode(ModeKind),
    Auto,
    Schedule(Schedule),
}

/// Exactly one field: `mode` (`sparring`, `sleep`, or `auto` to follow the schedule again), or a
/// `schedule` of two different `HH:MM` times.
fn parse_change(body: &serde_json::Value) -> Option<Change> {
    let fields = body.as_object().filter(|fields| fields.len() == 1)?;
    if let Some(text) = fields.get("mode") {
        return match text.as_str()? {
            "auto" => Some(Change::Auto),
            other => ModeKind::parse(other).map(Change::Mode),
        };
    }
    Schedule::from_json(fields.get("schedule")?).map(Change::Schedule)
}

fn current_json(state: &AppState, id: &str) -> serde_json::Value {
    mode::state(&mode::data_dir_of(&state.shared), id).api_json(chrono::Local::now())
}

/// The route of the project's mode, served with the other saved settings.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new().route(
        "/api/projects/{project_id}/mode",
        get(get_mode).put(put_mode),
    )
}
