use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::middleware;

use super::AppState;
use super::guards::no_referrer;

const MAX_MULTIPART_BODY: usize = 52 * 1024 * 1024;

pub(super) trait Finish {
    fn finish(self, state: Arc<AppState>) -> Router;
}

impl Finish for Router<Arc<AppState>> {
    fn finish(self, state: Arc<AppState>) -> Router {
        self.layer(middleware::from_fn_with_state(
            state.activity.clone(),
            super::super::idle::track,
        ))
        .layer(DefaultBodyLimit::max(MAX_MULTIPART_BODY))
        .layer(middleware::from_fn(no_referrer))
        .with_state(state)
    }
}
