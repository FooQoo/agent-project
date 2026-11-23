//! Putting the running tasks of quiet connections back to waiting.

use std::sync::Arc;

use super::AppState;

/// Every minute, put the running tasks whose connection went quiet back to waiting: a session
/// that was killed or crashed sends no end notice (see `STALE_RUN_SECONDS`).
pub(super) fn release_stale_runs(state: &Arc<AppState>) {
    let shared = state.shared.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            let shared = shared.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let store = crate::file_store::FileStore::open(&shared);
                let now = crate::clock::Clock::now(&crate::clock::SystemClock);
                // Read the folder names, not the catalog: one defective project.json must not
                // stop the cleanup of the others.
                let names = std::fs::read_dir(crate::file_store::layout::projects_dir(&shared))
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|entry| entry.file_name().into_string().ok());
                for name in names {
                    let Ok(id) = name.parse::<crate::domain::ProjectId>() else {
                        continue;
                    };
                    if let Err(error) = store.release_stale_runs(&id, now) {
                        eprintln!("Stale runs of project {name} were not released: {error}");
                    }
                }
            })
            .await;
        }
    });
}
