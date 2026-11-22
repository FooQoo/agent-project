//! The running pieces of one connection: the gate, the backend, the opener and
//! the tunnel. Closing a run stops all of them.

use std::any::Any;

use tokio::task::AbortHandle;

use super::router::{Gate, close_gate};

pub(super) struct Run {
    /// The provider this run was started with; it stays when the saved setting changes.
    pub(super) provider: super::contracts::Provider,
    pub(super) gate: Gate,
    pub(super) backend: AbortHandle,
    pub(super) opener: AbortHandle,
    pub(super) tunnel: Option<Box<dyn Any + Send>>,
}

impl Run {
    pub(super) fn close(self) {
        // First, so nothing more is admitted and open streams end.
        close_gate(&self.gate);
        self.opener.abort();
        self.backend.abort();
        drop(self.tunnel);
    }
}
