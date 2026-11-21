//! Planning tasks: one task's planned version, and the checked tasks of an epic. Only a person
//! uses these routes, so the actor is always the person.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::{Deserialize, Deserializer};

use super::epics::parse_epic_id;
use super::parse_task_id;
use crate::error::AgboardError;
use crate::file_store::{EpicVersionScope, VersionChoice};
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agboard_error_response, json_response};
use crate::server::store::{parse_project_id, refresh_index, with_request_files};

/// Which version to plan: a named one, the project's current one, or none.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum Choice {
    Named,
    Current,
    Unset,
}

fn choice_of(choice: Choice, version: Option<String>) -> Result<VersionChoice, AgboardError> {
    match (choice, version) {
        (Choice::Named, Some(name)) => Ok(VersionChoice::Named(name)),
        (Choice::Current, None) => Ok(VersionChoice::Current),
        (Choice::Unset, None) => Ok(VersionChoice::Unset),
        (Choice::Named, None) => Err(AgboardError::InvalidValue {
            field: "version".into(),
            reason: "A named choice needs a version.".into(),
        }),
        (_, Some(_)) => Err(AgboardError::InvalidValue {
            field: "version".into(),
            reason: "Only a named choice takes a version.".into(),
        }),
    }
}

/// Tells a missing field from `null`: `Some(None)` is `null`.
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<String>>, D::Error> {
    Option::<String>::deserialize(d).map(Some)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TaskVersionInput {
    choice: Choice,
    #[serde(default)]
    version: Option<String>,
    /// The planned version the page showed (`null` for none). A task that has another one now
    /// is refused with `planned_version_changed`. Absent: the choice is saved over the latest.
    #[serde(default, deserialize_with = "present")]
    expected_planned_version: Option<Option<String>>,
}

/// `PUT .../tasks/{id}/planned-version`: set or clear one task's planned version.
pub(super) async fn set_task_version(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_id)): Path<(String, String)>,
    StrictJson(input): StrictJson<TaskVersionInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let id = match parse_task_id(&raw_id) {
        Ok(id) => id,
        Err(response) => return *response,
    };
    let choice = match choice_of(input.choice, input.version) {
        Ok(choice) => choice,
        Err(error) => return agboard_error_response(&error),
    };
    let expected = input.expected_planned_version;
    let result = with_request_files(&state, &headers, move |store| {
        store.set_task_planned_version_checked(
            &project_id,
            id,
            &choice,
            expected.as_ref().map(|value| value.as_deref()),
            chrono::Utc::now(),
        )
    })
    .await;
    match result {
        Ok(result) => {
            if result.changed {
                refresh_index(&state);
            }
            json_response(
                StatusCode::OK,
                &serde_json::json!({"task": result.value, "changed": result.changed}),
            )
        }
        // The page names the version the task has now, so it can show it without reading again.
        Err(error @ AgboardError::PlannedVersionChanged { .. }) => {
            let current = match &error {
                AgboardError::PlannedVersionChanged { current, .. } => current.clone(),
                _ => None,
            };
            json_response(
                StatusCode::CONFLICT,
                &serde_json::json!({
                    "error": {
                        "code": error.code(),
                        "message": error.message(),
                        "reason": error.reason(),
                    },
                    "currentPlannedVersion": current,
                }),
            )
        }
        Err(error) => agboard_error_response(&error),
    }
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum Scope {
    UnplannedOnly,
    All,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct EpicVersionInput {
    choice: Choice,
    #[serde(default)]
    version: Option<String>,
    scope: Scope,
    /// The tasks the person left checked.
    task_ids: Vec<i64>,
    /// Only report what would change.
    #[serde(default)]
    dry_run: bool,
}

/// `POST .../epics/{epicId}/planned-version`: set or clear the planned version of the checked
/// tasks of an epic. Done and archived tasks are always skipped.
pub(super) async fn set_epic_version(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path((raw_project, raw_epic)): Path<(String, String)>,
    StrictJson(input): StrictJson<EpicVersionInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let epic_id = match parse_epic_id(&raw_epic) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let choice = match choice_of(input.choice, input.version) {
        Ok(choice) => choice,
        Err(error) => return agboard_error_response(&error),
    };
    let scope = match input.scope {
        Scope::UnplannedOnly => EpicVersionScope::UnplannedOnly,
        Scope::All => EpicVersionScope::All,
    };
    let dry_run = input.dry_run;
    let task_ids = input.task_ids;
    let result = with_request_files(&state, &headers, move |store| {
        store.plan_epic_version(
            &project_id,
            &epic_id,
            &choice,
            scope,
            &task_ids,
            dry_run,
            chrono::Utc::now(),
        )
    })
    .await;
    match result {
        Ok(plan) => {
            if !dry_run && !plan.result.changed.is_empty() {
                refresh_index(&state);
            }
            let reasons: serde_json::Map<String, serde_json::Value> = plan
                .result
                .skipped
                .iter()
                .zip(&plan.skipped_reasons)
                .map(|(id, reason)| (id.to_string(), reason.as_str().into()))
                .collect();
            json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "changed": plan.result.changed,
                    "unchanged": plan.result.unchanged,
                    "skipped": plan.result.skipped,
                    "skippedReasons": reasons,
                    "dryRun": dry_run,
                }),
            )
        }
        Err(error) => agboard_error_response(&error),
    }
}
