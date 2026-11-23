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
        let Ok(cwd) = hook_folder(cwd) else {
            return;
        };
        store
            .resolve_binding(&cwd)
            .ok()
            .flatten()
            .map(|r| r.project)
    };
    let (Some(meta), Some(data_dir)) = (meta, shared.parent()) else {
        return;
    };
    let secret = session_id
        .map(super::session_identity::claude_key)
        .and_then(|key| super::connection_store::load_secret_for_key(data_dir, &meta.id, &key))
        .or_else(|| super::connection_store::load_secret(data_dir, &meta.id));
    if let Some(secret) = secret {
        let _ = store.end_connection_session(&meta.id, &secret, SystemClock.now());
    }
}
