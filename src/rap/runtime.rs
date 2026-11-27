use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::broadcast;

use super::scheduler::SchedulerStore;
use super::{RapError, RapStore, WorkId};

#[derive(Debug, Clone, Serialize)]
pub struct RapEvent {
    pub kind: &'static str,
    pub work_id: Option<WorkId>,
    pub version: Option<i64>,
}

/// Shared runtime surface for CLI and HTTP handlers: commits to SQLite first,
/// then broadcasts a post-commit event to SSE subscribers.
pub struct RapRuntime {
    pub store: RapStore,
    pub scheduler: SchedulerStore,
    events: broadcast::Sender<RapEvent>,
}

impl RapRuntime {
    pub fn open(database: &Path) -> Result<Arc<Self>, RapError> {
        let store = RapStore::open(database)?;
        let scheduler = SchedulerStore::new(store.clone());
        let (events, _) = broadcast::channel(128);
        Ok(Arc::new(Self {
            store,
            scheduler,
            events,
        }))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RapEvent> {
        self.events.subscribe()
    }

    pub fn notify(&self, kind: &'static str, work_id: Option<WorkId>, version: Option<i64>) {
        let _ = self.events.send(RapEvent {
            kind,
            work_id,
            version,
        });
    }
}
