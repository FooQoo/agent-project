//! The Claude Code `SessionEnd` hook: the session's running tasks go back to waiting.

use std::path::Path;

use super::hook::hook_folder;
use crate::clock::{Clock, SystemClock};
use crate::domain::validate_project_name;
use crate::file_store::FileStore;

/// The session ended: its running tasks go back to waiting. Best effort and silent; a session
/// that was never connected has nothing to release.
pub(super) fn run(
    shared: &Path,
    project: Option<&str>,
    cwd: Option<&serde_json::Value>,
    session_id: Option<&str>,
) {
    let store = FileStore::open(shared);
    let meta = if let Some(name) = project {
        let Ok(name) = validate_project_name(name) else {
            return;
        };
        store.project_by_name(&name).ok()
    } else {
        hook_folder(cwd)
            .ok()
            .and_then(|cwd| store.resolve_binding(&cwd).ok().flatten())
            .map(|r| r.project)
    };
    let Some(data_dir) = shared.parent() else {
        return;
    };
    let now = SystemClock.now();
    let key = session_id.map(super::session_identity::claude_key);
    let mut ended: Vec<crate::domain::ProjectId> = Vec::new();
    if let Some(meta) = meta {
        let secret = key
            .as_deref()
            .and_then(|key| super::connection_store::load_secret_for_key(data_dir, &meta.id, key))
            .or_else(|| super::connection_store::load_secret(data_dir, &meta.id));
        if let Some(secret) = secret {
            let _ = store.end_connection_session(&meta.id, &secret, now);
        }
        ended.push(meta.id);
    }
    // The folder may be unbound, or bound to another project than the one the session
    // connected to with `--project`: release every project that holds this session's secret.
    if let Some(key) = key.as_deref() {
        for (project, secret) in super::connection_store::stored_secrets_for_key(data_dir, key) {
            if !ended.contains(&project) {
                let _ = store.end_connection_session(&project, &secret, now);
            }
        }
    }
}
