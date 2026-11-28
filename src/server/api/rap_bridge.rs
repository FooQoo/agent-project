use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use super::parse_task_id;
use super::rap::{now_secs, rap_error};
use crate::domain::ProjectId;
use crate::file_store::FileStore;
use crate::rap::scheduler::Provider;
use crate::rap::{NewWork, WorkId, WorkKind};
use crate::server::AppState;
use crate::server::resident::resolve_project;
use crate::server::response::agpj_error_response;
use crate::server::sessions::Tool;
use crate::server::store::{parse_project_id, with_files};

/// Resolves the RAP provider from the saved auto-start tool; `None` when none is saved.
pub(super) fn preferred_provider(state: &AppState) -> Option<Provider> {
    state.resident.autostart().get().map(|a| match a.tool {
        Tool::ClaudeCode => Provider::ClaudeCode,
        _ => Provider::Codex,
    })
}

/// Bridges an existing board task into `rap.db`, authorizes direct execution, and
/// queues it for immediate single-item dispatch even when `auto_run` is OFF.
pub(super) async fn run_board_task(
    State(state): State<Arc<AppState>>,
    AxumPath((raw_project, raw_id)): AxumPath<(String, String)>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agpj_error_response(&error),
    };
    let task_id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let shared = state.shared.clone();
    let pid_for_read = project_id.clone();
    let task = match with_files(&state, move |store| store.task(&pid_for_read, task_id)).await {
        Ok(task) => task,
        Err(error) => return agpj_error_response(&error),
    };
    let (_, workspace) = match resolve_project(&FileStore::open(&shared), &project_id) {
        Ok(pair) => pair,
        Err(error) => return agpj_error_response(&error),
    };
    let Some(provider) = preferred_provider(&state) else {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "No agent is set up to run tasks; choose one in the resident session settings."
            })),
        )
            .into_response();
    };
    if let Some(reason) = manual_run_blocker(&state, &workspace, provider, now_secs()) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": reason })),
        )
            .into_response();
    }
    match enqueue_board_task(
        &state,
        &project_id,
        &workspace,
        &task.title,
        &task.description,
        task.position,
        provider,
        true,
    ) {
        Ok(work_id) => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "workId": work_id.0, "queued": true })),
        )
            .into_response(),
        Err(error) => rap_error(error),
    }
}

fn manual_run_blocker(
    state: &AppState,
    workspace: &Path,
    provider: Provider,
    now: i64,
) -> Option<String> {
    let in_flight = state.rap.scheduler.in_flight().ok()?;
    let settings = state.rap.scheduler.settings().ok()?;
    if in_flight.len() >= settings.max_sessions.get() as usize {
        return Some("Another RAP session is already running.".to_owned());
    }
    let canonical = std::fs::canonicalize(workspace).ok()?;
    if in_flight.iter().any(|f| f.claim.workspace == canonical) {
        return Some("This workspace is already in use by another RAP work item.".to_owned());
    }
    // A paused or retry-waiting attempt holds no session slot but keeps its workspace
    // reservation, and the scheduler silently skips every other work item on that workspace.
    let reserved_by: Option<i64> = state
        .rap
        .store
        .read(|conn| {
            use rusqlite::OptionalExtension;
            Ok(conn
                .query_row(
                    "SELECT work_id FROM scheduler_workspaces WHERE path = ?1",
                    [canonical.to_string_lossy().as_ref()],
                    |row| row.get(0),
                )
                .optional()?)
        })
        .ok()
        .flatten();
    if let Some(work_id) = reserved_by {
        return Some(format!(
            "This workspace is reserved by paused RAP work item {work_id}; finish or cancel it first."
        ));
    }
    let provider_str = match provider {
        Provider::Codex => "codex",
        Provider::ClaudeCode => "claude_code",
    };
    let retry_at: Option<i64> = state
        .rap
        .store
        .read(|conn| {
            use rusqlite::OptionalExtension;
            Ok(conn
                .query_row(
                    "SELECT retry_at FROM scheduler_provider_wait WHERE provider = ?1",
                    [provider_str],
                    |row| row.get(0),
                )
                .optional()?)
        })
        .ok()
        .flatten();
    if let Some(until) = retry_at.filter(|until| *until > now) {
        return Some(format!(
            "Provider {provider_str} is rate-limited until unix {until}."
        ));
    }
    None
}

/// Registers a board task in `rap.db` as an authorized direct-execution work item.
#[allow(clippy::too_many_arguments)]
pub(super) fn enqueue_board_task(
    state: &AppState,
    project_id: &ProjectId,
    workspace: &Path,
    title: &str,
    description: &str,
    rank: i64,
    provider: Provider,
    manual_run: bool,
) -> Result<WorkId, crate::rap::RapError> {
    let item = state.rap.store.create_work(NewWork {
        project_id: project_id.as_str(),
        workspace,
        kind: WorkKind::Task,
        parent_epic_id: None,
        title,
        rank,
    })?;
    state
        .rap
        .scheduler
        .configure_work(item.id, provider, false)?;
    let brief = format!("# {title}\n\n{description}");
    state
        .rap
        .store
        .authorize_direct_execution(item.id, brief.as_bytes(), "human", now_secs())?;
    if manual_run {
        state.rap.enqueue_manual_run(item.id);
    }
    state
        .rap
        .notify("direct_authorized", Some(item.id), Some(item.version));
    Ok(item.id)
}
