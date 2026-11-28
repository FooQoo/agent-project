use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::{Notify, broadcast};

use super::scheduler::SchedulerStore;
use super::{RapError, RapStore, WorkId};

#[derive(Debug, Clone, Serialize)]
pub struct RapEvent {
    pub kind: &'static str,
    pub work_id: Option<WorkId>,
    pub version: Option<i64>,
}

/// Shared runtime surface for CLI and HTTP handlers: commits to SQLite first,
/// then broadcasts a post-commit event to SSE subscribers and wakes the RAP worker.
pub struct RapRuntime {
    pub store: RapStore,
    pub scheduler: SchedulerStore,
    events: broadcast::Sender<RapEvent>,
    wake: Arc<Notify>,
    manual_runs: Mutex<VecDeque<WorkId>>,
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
            wake: Arc::new(Notify::new()),
            manual_runs: Mutex::new(VecDeque::new()),
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
        self.wake.notify_one();
    }

    pub fn enqueue_manual_run(&self, work_id: WorkId) {
        self.manual_runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push_back(work_id);
        self.wake.notify_one();
    }

    pub fn take_manual_run(&self) -> Option<WorkId> {
        self.manual_runs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pop_front()
    }

    pub async fn woken(&self) {
        self.wake.notified().await;
    }
}
