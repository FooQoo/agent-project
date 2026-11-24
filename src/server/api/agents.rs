//! Agent routes: the registry and the assignment of a card to an agent. Only a person uses them
//! (the write guard has already checked the page token), and the connection code is shown on the
//! local entrance only, so a phone cannot hand out a way into a session on the PC.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{get, patch, post, put};
use serde::Deserialize;
use serde_json::json;

use super::parse_task_id;
use crate::clock::{Clock, SystemClock};
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agpj_error_response, json_response};
use crate::server::store::{parse_project_id, refresh_index, with_request_files};

/// The agent routes every entrance serves (the connection code is a local route).
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/projects/{project_id}/agents",
            get(list_agents).post(register_agent),
        )
        .route(
            "/api/projects/{project_id}/agents/default",
            post(prepare_default_agent),
        )
        .route(
            "/api/projects/{project_id}/agents/{agent_id}",
            patch(update_agent).delete(archive_agent),
        )
        .route(
            "/api/projects/{project_id}/tasks/{id}/agent",
            put(assign_task_agent),
        )
}

macro_rules! project_of {
    ($raw:expr) => {
        match parse_project_id(&$raw) {
            Ok(id) => id,
            Err(error) => return agpj_error_response(&error),
        }
    };
}

/// `GET .../agents`
pub(super) async fn list_agents(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
) -> Response {
    let project = project_of!(raw_project);
    match with_request_files(&state, &headers, move |files| files.list_agents(&project)).await {
        Ok(agents) => json_response(StatusCode::OK, &json!({"agents": agents})),
        Err(error) => agpj_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RegisterAgentInput {
    name: String,
    #[serde(default)]
    role: String,
}

/// `POST .../agents`: register an agent.
pub(super) async fn register_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
    StrictJson(input): StrictJson<RegisterAgentInput>,
) -> Response {
    let project = project_of!(raw_project);
    let saved = with_request_files(&state, &headers, move |files| {
        files.register_agent(&project, &input.name, &input.role, SystemClock.now())
    })
    .await;
    match saved {
        Ok(agent) => {
            refresh_index(&state);
            json_response(StatusCode::CREATED, &json!({"agent": agent}))
        }
        Err(error) => agpj_error_response(&error),
    }
}

/// `POST .../agents/default`: prepare the default agent for an existing board.
pub(super) async fn prepare_default_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
) -> Response {
    let project = project_of!(raw_project);
    let saved = with_request_files(&state, &headers, move |files| {
        files.prepare_default_agent(&project, SystemClock.now())
    })
    .await;
    match saved {
        Ok(agent_id) => {
            refresh_index(&state);
            json_response(StatusCode::OK, &json!({"agentId": agent_id}))
        }
        Err(error) => agpj_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UpdateAgentInput {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

/// `PATCH .../agents/{agent_id}`: rename an agent or change its role.
pub(super) async fn update_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, agent_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<UpdateAgentInput>,
) -> Response {
    let project = project_of!(raw_project);
    let saved = with_request_files(&state, &headers, move |files| {
        files.update_agent(
            &project,
            &agent_id,
            input.name.as_deref(),
            input.role.as_deref(),
        )
    })
    .await;
    match saved {
        Ok(agent) => {
            refresh_index(&state);
            json_response(StatusCode::OK, &json!({"agent": agent}))
        }
        Err(error) => agpj_error_response(&error),
    }
}

/// `DELETE .../agents/{agent_id}`: delete (archive) an agent that has no Doing card and no
/// active connection.
pub(super) async fn archive_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, agent_id)): Path<(String, String)>,
) -> Response {
    let project = project_of!(raw_project);
    let saved = with_request_files(&state, &headers, move |files| {
        files.archive_agent(&project, &agent_id, SystemClock.now())
    })
    .await;
    match saved {
        Ok(agent) => {
            refresh_index(&state);
            json_response(StatusCode::OK, &json!({"agent": agent}))
        }
        Err(error) => agpj_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ConnectCodeInput {
    /// Replace the agent's active connection when the code is exchanged.
    #[serde(default)]
    switch: bool,
}

/// `POST .../agents/{agent_id}/connect-code` (local entrance only): a one-time code that a
/// session exchanges with `agpj connect`.
pub(super) async fn issue_connect_code(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, agent_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<ConnectCodeInput>,
) -> Response {
    let project = project_of!(raw_project);
    let issued = with_request_files(&state, &headers, move |files| {
        files.issue_connect_code(&project, &agent_id, input.switch, SystemClock.now())
    })
    .await;
    match issued {
        Ok(code) => json_response(StatusCode::OK, &json!({"connectCode": code})),
        Err(error) => agpj_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AssignInput {
    /// The agent to give the task to; `null` takes the agent away (not for a Doing task).
    agent_id: Option<String>,
}

/// `PUT .../tasks/{id}/agent`: give a task to an agent. The generation goes up and a running
/// task goes back to waiting.
pub(super) async fn assign_task_agent(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<AssignInput>,
) -> Response {
    let project = project_of!(raw_project);
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let saved = with_request_files(&state, &headers, move |files| {
        files.assign_task_agent(&project, id, input.agent_id.as_deref(), SystemClock.now())
    })
    .await;
    match saved {
        Ok(task) => {
            refresh_index(&state);
            json_response(StatusCode::OK, &json!({"task": task}))
        }
        Err(error) => agpj_error_response(&error),
    }
}
