//! Application state for the web server.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crucible::CurationLayer;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// The curation layer being reviewed.
    pub curation: Arc<RwLock<CurationLayer>>,
    /// Path to the curation file.
    pub curation_path: PathBuf,
    /// Path to the original data file.
    pub data_path: PathBuf,
    /// Whether to auto-save on changes.
    pub auto_save: bool,
}

impl AppState {
    /// Create new application state.
    pub fn new(curation: CurationLayer, curation_path: PathBuf, data_path: PathBuf) -> Self {
        Self {
            curation: Arc::new(RwLock::new(curation)),
            curation_path,
            data_path,
            auto_save: true,
        }
    }

    /// Save the curation layer to disk.
    pub async fn save(&self) -> Result<(), crucible::CrucibleError> {
        let curation = self.curation.read().await;
        curation.save(&self.curation_path)
    }
}
