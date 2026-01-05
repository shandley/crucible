//! Application state for the web server.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crucible::{CurationLayer, LlmProvider};

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
    /// Optional LLM provider for interactive explanations.
    /// If None, LLM features (Ask, explanations) are disabled.
    pub llm_provider: Option<Arc<dyn LlmProvider>>,
    /// Name of the configured LLM provider (for display).
    pub llm_provider_name: Option<String>,
}

impl AppState {
    /// Create new application state.
    pub fn new(curation: CurationLayer, curation_path: PathBuf, data_path: PathBuf) -> Self {
        Self {
            curation: Arc::new(RwLock::new(curation)),
            curation_path,
            data_path,
            auto_save: true,
            llm_provider: None,
            llm_provider_name: None,
        }
    }

    /// Create new application state with an LLM provider.
    pub fn with_llm(
        curation: CurationLayer,
        curation_path: PathBuf,
        data_path: PathBuf,
        provider: Arc<dyn LlmProvider>,
    ) -> Self {
        let name = provider.name().to_string();
        Self {
            curation: Arc::new(RwLock::new(curation)),
            curation_path,
            data_path,
            auto_save: true,
            llm_provider: Some(provider),
            llm_provider_name: Some(name),
        }
    }

    /// Check if LLM features are available.
    pub fn has_llm(&self) -> bool {
        self.llm_provider.is_some()
    }

    /// Save the curation layer to disk.
    pub async fn save(&self) -> Result<(), crucible::CrucibleError> {
        let curation = self.curation.read().await;
        curation.save(&self.curation_path)
    }
}
