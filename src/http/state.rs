//! Shared state for HTTP handlers.

use parking_lot::RwLock;
use std::sync::Arc;

use crate::app::App;

/// Shared application state for HTTP handlers.
#[derive(Clone)]
pub struct AppState {
    pub app: Arc<RwLock<App>>,
}

impl AppState {
    pub fn new(app: App) -> Self {
        Self {
            app: Arc::new(RwLock::new(app)),
        }
    }
}
