use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use super::rap::{now_secs, rap_error};
use crate::rap::scheduler::SchedulerSettings;
use crate::rap::{Evidence, QuestionId, RapResult, ResultId, RevisionId, WorkId};
use crate::server::AppState;

#[derive(Serialize)]
struct WorkResultsResponse {
    results: Vec<RapResult>,
    evidence: Vec<Evidence>,
}

#[derive(Deserialize)]
pub(super) struct ApprovePlanBody {
    revision_id: RevisionId,
    authorized_by: String,
}

#[derive(Deserialize)]
pub(super) struct AuthorizeDirectBody {
    brief: String,
    authorized_by: String,
}

#[derive(Deserialize)]
pub(super) struct ChangeRequestBody {
    body: String,
}

#[derive(Deserialize)]
pub(super) struct AnswerQuestionBody {
    answer: String,
    answered_by: String,
}

pub(super) async fn approve_plan(
    State(state): State<Arc<AppState>>,
    Path(work_id): Path<i64>,
    Json(body): Json<ApprovePlanBody>,
) -> Response {
    let work = WorkId(work_id);
    match state.rap.store.approve_plan_revision(
        work,
        body.revision_id,
        &body.authorized_by,
        now_secs(),
    ) {
        Ok(contract) => {
            let version = state.rap.store.work(work).ok().map(|w| w.version);
            state.rap.notify("plan_approved", Some(work), version);
            Json(contract).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn authorize_direct(
    State(state): State<Arc<AppState>>,
    Path(work_id): Path<i64>,
    Json(body): Json<AuthorizeDirectBody>,
) -> Response {
    let work = WorkId(work_id);
    match state.rap.store.authorize_direct_execution(
        work,
        body.brief.as_bytes(),
        &body.authorized_by,
        now_secs(),
    ) {
        Ok(contract) => {
            let version = state.rap.store.work(work).ok().map(|w| w.version);
            state.rap.notify("direct_authorized", Some(work), version);
            Json(contract).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn create_change_request(
    State(state): State<Arc<AppState>>,
    Path(work_id): Path<i64>,
    Json(body): Json<ChangeRequestBody>,
) -> Response {
    let work = WorkId(work_id);
    match state
        .rap
        .store
        .receive_change_request(work, &body.body, now_secs())
    {
        Ok(req) => {
            let version = state.rap.store.work(work).ok().map(|w| w.version);
            state
                .rap
                .notify("change_request_received", Some(work), version);
            (StatusCode::CREATED, Json(req)).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn list_results(
    State(state): State<Arc<AppState>>,
    Path(work_id): Path<i64>,
) -> Response {
    let work = WorkId(work_id);
    let results = match state.rap.store.list_results(work) {
        Ok(v) => v,
        Err(e) => return rap_error(e),
    };
    let evidence = match state.rap.store.list_evidence(work) {
        Ok(v) => v,
        Err(e) => return rap_error(e),
    };
    Json(WorkResultsResponse { results, evidence }).into_response()
}

pub(super) async fn answer_question(
    State(state): State<Arc<AppState>>,
    Path(question_id): Path<i64>,
    Json(body): Json<AnswerQuestionBody>,
) -> Response {
    let qid = QuestionId(question_id);
    let now = now_secs();
    let answered = match state
        .rap
        .store
        .answer_question(qid, &body.answer, &body.answered_by, now)
    {
        Ok(q) => q,
        Err(e) => return rap_error(e),
    };
    match state
        .rap
        .store
        .deliver_answer(qid, answered.session_id, now)
    {
        Ok(outcome) => {
            state
                .rap
                .notify("question_answered", Some(answered.work_id), None);
            Json(serde_json::json!({ "question": answered, "delivery": outcome })).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn revise_result(
    State(state): State<Arc<AppState>>,
    Path(result_id): Path<i64>,
    Json(body): Json<ChangeRequestBody>,
) -> Response {
    match state
        .rap
        .store
        .request_revision_from_result(ResultId(result_id), &body.body, now_secs())
    {
        Ok(req) => {
            state
                .rap
                .notify("change_request_received", Some(req.work_id), None);
            (StatusCode::CREATED, Json(req)).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn enter_sleep(State(state): State<Arc<AppState>>) -> Response {
    match state.rap.store.enter_sleep(now_secs()) {
        Ok(()) => {
            state.rap.notify("sleep_entered", None, None);
            Json(serde_json::json!({ "sleeping": true })).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn wake_sleep(State(state): State<Arc<AppState>>) -> Response {
    match state.rap.store.wake_from_sleep(now_secs()) {
        Ok(report) => {
            state.rap.notify("sleep_woken", None, None);
            Json(report).into_response()
        }
        Err(e) => rap_error(e),
    }
}

pub(super) async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SchedulerSettings>,
) -> Response {
    match state.rap.scheduler.set_settings(body) {
        Ok(()) => {
            state.rap.notify("settings_updated", None, None);
            Json(body).into_response()
        }
        Err(e) => rap_error(e),
    }
}
