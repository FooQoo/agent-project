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
    // A board-launched session keys its secret by `AGPJ_RESIDENT_ID`, which wins over the
    // Claude session ID, so release the secrets held under either key.
    let mut keys: Vec<String> = Vec::new();
    keys.extend(session_id.map(super::session_identity::claude_key));
    if std::env::var_os("AGPJ_RESIDENT_ID").is_some() {
        let key = super::session_identity::SessionIdentity::from_env().key;
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    let mut ended: Vec<crate::domain::ProjectId> = Vec::new();
    if let Some(meta) = meta {
        // Try every stored secret: one can belong to a replaced connection and be refused while
        // another key holds the live one.
        let mut secrets: Vec<String> = keys
            .iter()
            .filter_map(|key| super::connection_store::load_secret_for_key(data_dir, &meta.id, key))
            .collect();
        if secrets.is_empty() {
            secrets.extend(super::connection_store::load_secret(data_dir, &meta.id));
        }
        for secret in secrets {
            let _ = store.end_connection_session(&meta.id, &secret, now);
        }
        ended.push(meta.id);
    }
    // The folder may be unbound, or bound to another project than the one the session
    // connected to with `--project`: release every project that holds this session's secret.
    for key in &keys {
        for (project, secret) in super::connection_store::stored_secrets_for_key(data_dir, key) {
            if !ended.contains(&project) {
                let _ = store.end_connection_session(&project, &secret, now);
                ended.push(project);
            }
        }
    }
}
