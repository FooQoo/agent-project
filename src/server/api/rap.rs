use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use super::rap_actions::{
    answer_question, approve_plan, authorize_direct, create_change_request, enter_sleep,
    list_results, revise_result, update_settings, wake_sleep,
};
use crate::rap::scheduler::SchedulerSettings;
use crate::rap::{
    ChangeRequest, CompletionCheck, NewWork, RapError, RapResult, WorkId, WorkItem, WorkKind,
};
use crate::server::AppState;

pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/rap/events", get(rap_events))
        .route("/api/rap/settings", put(update_settings))
        .route("/api/rap/projects/{project_id}/summary", get(rap_summary))
        .route("/api/rap/projects/{project_id}/work", post(create_work))
        .route("/api/rap/work/{work_id}/approve-plan", post(approve_plan))
        .route(
            "/api/rap/work/{work_id}/authorize-direct",
            post(authorize_direct),
        )
        .route(
            "/api/rap/work/{work_id}/change-requests",
            post(create_change_request),
        )
        .route("/api/rap/work/{work_id}/results", get(list_results))
        .route(
            "/api/rap/questions/{question_id}/answer",
            post(answer_question),
        )
        .route("/api/rap/results/{result_id}/revise", post(revise_result))
        .route("/api/rap/sleep/enter", post(enter_sleep))
        .route("/api/rap/sleep/wake", post(wake_sleep))
}

#[derive(Serialize)]
struct RapSummary {
    sleeping: bool,
    settings: SchedulerSettings,
    items: Vec<RapSummaryItem>,
}

#[derive(Serialize)]
struct RapSummaryItem {
    work: WorkItem,
    waiting_reason: Option<String>,
    retry_at: Option<i64>,
    pending_change_requests: Vec<ChangeRequest>,
    results: Vec<RapResult>,
    completion: CompletionCheck,
}

#[derive(Deserialize)]
struct CreateWorkBody {
    workspace: PathBuf,
    kind: WorkKind,
    parent_epic_id: Option<WorkId>,
    title: String,
    rank: Option<i64>,
}

pub(super) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

pub(super) fn rap_error(error: RapError) -> Response {
    let status = match error {
        RapError::NotFound(_) => StatusCode::NOT_FOUND,
        RapError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        RapError::Conflict(_) | RapError::CompletionBlocked(_) | RapError::DocumentIntegrity(_) => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(serde_json::json!({ "error": error.to_string() })),
    )
        .into_response()
}

async fn rap_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.rap.subscribe()).filter_map(|item| match item {
        Ok(event) => Some(Ok(Event::default()
            .event(event.kind)
            .data(serde_json::to_string(&event).unwrap_or_default()))),
        Err(_) => Some(Ok(Event::default().event("resync").data("{}"))),
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn rap_summary(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
) -> Response {
    let sleeping = match state.rap.store.is_sleeping() {
        Ok(v) => v,
        Err(e) => return rap_error(e),
    };
    let settings = match state.rap.scheduler.settings() {
        Ok(v) => v,
        Err(e) => return rap_error(e),
    };
    let works = match state.rap.store.project_work(&project_id) {
        Ok(v) => v,
        Err(e) => return rap_error(e),
    };
    let mut items = Vec::with_capacity(works.len());
    for work in works {
        let wait = match state.rap.scheduler.waiting_reason(work.id) {
            Ok(v) => v,
            Err(e) => return rap_error(e),
        };
        let (waiting_reason, retry_at) = match wait {
            Some((r, t)) => (Some(r), t),
            None => (None, None),
        };
        let pending_change_requests = match state.rap.store.pending_change_requests(work.id) {
            Ok(v) => v,
            Err(e) => return rap_error(e),
        };
        let results = match state.rap.store.list_results(work.id) {
            Ok(v) => v,
            Err(e) => return rap_error(e),
        };
        let completion = match state.rap.store.evaluate_completion(work.id) {
            Ok(v) => v,
            Err(e) => return rap_error(e),
        };
        items.push(RapSummaryItem {
            work,
            waiting_reason,
            retry_at,
            pending_change_requests,
            results,
            completion,
        });
    }
    Json(RapSummary {
        sleeping,
        settings,
        items,
    })
    .into_response()
}

async fn create_work(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(body): Json<CreateWorkBody>,
) -> Response {
    match state.rap.store.create_work(NewWork {
        project_id: &project_id,
        workspace: &body.workspace,
        kind: body.kind,
        parent_epic_id: body.parent_epic_id,
        title: &body.title,
        rank: body.rank.unwrap_or(0),
    }) {
        Ok(item) => {
            state
                .rap
                .notify("work_created", Some(item.id), Some(item.version));
            (StatusCode::CREATED, Json(item)).into_response()
        }
        Err(e) => rap_error(e),
    }
}
