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

/// Resolves the preferred RAP provider from the saved auto-start tool (defaults to Codex).
pub(super) fn preferred_provider(state: &AppState) -> Provider {
    match state.resident.autostart().get().map(|a| a.tool) {
        Some(Tool::ClaudeCode) => Provider::ClaudeCode,
        _ => Provider::Codex,
    }
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
    let provider = preferred_provider(&state);
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
