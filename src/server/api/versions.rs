//! The project's versions, its current version, and the version tasks are planned for. Only a
//! person changes them here, so the actor is always the person.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use serde::Deserialize;

use crate::error::AgpjError;
use crate::file_store::{ProjectVersions, VersionChoice};
use crate::server::AppState;
use crate::server::request::StrictJson;
use crate::server::response::{agboard_error_response, json_response};
use crate::server::store::{indexed_view, parse_project_id, refresh_index, with_request_files};

fn versions_json(value: &ProjectVersions, changed: Option<bool>) -> serde_json::Value {
    let mut json = serde_json::json!({
        "versions": value.versions,
        "currentVersion": value.current,
    });
    if let Some(changed) = changed {
        json["changed"] = changed.into();
    }
    json
}

/// `GET .../versions`: the versions and the current one, from the index.
pub(super) async fn get_versions(
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    match indexed_view(&state, &project_id) {
        Ok(view) => json_response(
            StatusCode::OK,
            &versions_json(
                &ProjectVersions {
                    versions: view.versions.clone(),
                    current: view.current_version.clone(),
                },
                None,
            ),
        ),
        Err(error) => agboard_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AddVersionInput {
    name: String,
}

/// `POST .../versions`: list a version. Listing one that is listed changes nothing.
pub(super) async fn add_version(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
    StrictJson(input): StrictJson<AddVersionInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.add_version(&project_id, &input.name, chrono::Utc::now())
    })
    .await;
    versions_answer(&state, result)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CurrentVersionInput {
    /// The version to make current, or `null` to clear it.
    version: Option<String>,
}

/// `PUT .../versions/current`: set or clear the current version. A version that is not listed
/// is listed first.
pub(super) async fn set_current_version(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
    StrictJson(input): StrictJson<CurrentVersionInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.set_current_version(&project_id, input.version.as_deref(), chrono::Utc::now())
    })
    .await;
    versions_answer(&state, result)
}

fn versions_answer(
    state: &AppState,
    result: Result<crate::domain::Changed<ProjectVersions>, AgpjError>,
) -> Response {
    match result {
        Ok(result) => {
            if result.changed {
                refresh_index(state);
            }
            json_response(
                StatusCode::OK,
                &versions_json(&result.value, Some(result.changed)),
            )
        }
        Err(error) => agboard_error_response(&error),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseDocInput {
    /// The version to generate for, or `null` for the project's current version.
    version: Option<String>,
}

/// Generate the document. The answer names the version and the path in the Docs, and says
/// whether the file was created or changed. The Docs viewer opens the path as any Docs file.
pub(super) async fn generate_release_doc(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(raw_project): Path<String>,
    StrictJson(input): StrictJson<ReleaseDocInput>,
) -> Response {
    let project_id = match parse_project_id(&raw_project) {
        Ok(id) => id,
        Err(error) => return agboard_error_response(&error),
    };
    let choice = match input.version {
        Some(name) => VersionChoice::Named(name),
        None => VersionChoice::Current,
    };
    let result = with_request_files(&state, &headers, move |store| {
        store.generate_release_doc(&project_id, &choice, chrono::Utc::now())
    })
    .await;
    match result {
        Ok(doc) => {
            if doc.changed {
                refresh_index(&state);
            }
            json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "version": doc.version,
                    "path": doc.path,
                    "created": doc.created,
                    "changed": doc.changed,
                    "planned": doc.planned,
                    "verified": doc.verified,
                    "doneWithoutAcceptance": doc.done_unchecked,
                    "remaining": doc.remaining,
                    "moved": doc.moved,
                    "published": doc.published,
                }),
            )
        }
        Err(error) => agboard_error_response(&error),
    }
}
