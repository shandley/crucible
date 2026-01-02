//! Curation context for domain-aware analysis.
//!
//! This module provides an enhanced context structure that matches the
//! curation layer spec, with nested hint, file context, and inference config sections.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::input::ContextHints;

/// Context hints provided by the user.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserHints {
    /// Name of the study or project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study_name: Option<String>,

    /// Domain of the data (e.g., "biomedical", "financial").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Expected number of samples/rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_sample_count: Option<usize>,

    /// Column that serves as the primary identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier_column: Option<String>,

    /// Hints for specific columns.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub column_hints: HashMap<String, String>,

    /// Custom key-value hints.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,
}

impl UserHints {
    /// Create empty user hints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the study name.
    pub fn with_study_name(mut self, name: impl Into<String>) -> Self {
        self.study_name = Some(name.into());
        self
    }

    /// Set the domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set expected sample count.
    pub fn with_expected_sample_count(mut self, count: usize) -> Self {
        self.expected_sample_count = Some(count);
        self
    }

    /// Set identifier column.
    pub fn with_identifier_column(mut self, column: impl Into<String>) -> Self {
        self.identifier_column = Some(column.into());
        self
    }
}

/// File-derived context information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileContext {
    /// Directory containing the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,

    /// Related files in the same directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_files: Vec<String>,

    /// Source of extraction (e.g., original R object name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_source: Option<String>,
}

impl FileContext {
    /// Create empty file context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the directory.
    pub fn with_directory(mut self, dir: impl Into<String>) -> Self {
        self.directory = Some(dir.into());
        self
    }

    /// Add a related file.
    pub fn with_related_file(mut self, file: impl Into<String>) -> Self {
        self.related_files.push(file.into());
        self
    }

    /// Set related files.
    pub fn with_related_files(mut self, files: Vec<String>) -> Self {
        self.related_files = files;
        self
    }

    /// Set extraction source.
    pub fn with_extraction_source(mut self, source: impl Into<String>) -> Self {
        self.extraction_source = Some(source.into());
        self
    }
}

/// Configuration for inference behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Minimum confidence threshold for inferences.
    pub confidence_threshold: f64,

    /// Whether LLM enhancement is enabled.
    pub llm_enabled: bool,

    /// LLM model to use (if enabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.8,
            llm_enabled: false,
            llm_model: None,
        }
    }
}

impl InferenceConfig {
    /// Create default inference config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable LLM with a specific model.
    pub fn with_llm(mut self, model: impl Into<String>) -> Self {
        self.llm_enabled = true;
        self.llm_model = Some(model.into());
        self
    }

    /// Set confidence threshold.
    pub fn with_confidence_threshold(mut self, threshold: f64) -> Self {
        self.confidence_threshold = threshold;
        self
    }
}

/// Complete curation context matching the spec structure.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurationContext {
    /// User-provided hints.
    #[serde(default)]
    pub hints: UserHints,

    /// File-derived context.
    #[serde(default)]
    pub file_context: FileContext,

    /// Inference configuration.
    #[serde(default)]
    pub inference_config: InferenceConfig,
}

impl CurationContext {
    /// Create empty curation context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create from the simpler ContextHints structure.
    pub fn from_hints(hints: &ContextHints) -> Self {
        Self {
            hints: UserHints {
                study_name: hints.study_name.clone(),
                domain: hints.domain.clone(),
                expected_sample_count: hints.expected_sample_count,
                identifier_column: hints.identifier_column.clone(),
                column_hints: hints.column_hints.clone(),
                custom: hints.custom.clone(),
            },
            file_context: FileContext {
                directory: None,
                related_files: hints.related_files.clone(),
                extraction_source: hints.data_source.clone(),
            },
            inference_config: InferenceConfig::default(),
        }
    }

    /// Set user hints.
    pub fn with_hints(mut self, hints: UserHints) -> Self {
        self.hints = hints;
        self
    }

    /// Set file context.
    pub fn with_file_context(mut self, ctx: FileContext) -> Self {
        self.file_context = ctx;
        self
    }

    /// Set inference config.
    pub fn with_inference_config(mut self, config: InferenceConfig) -> Self {
        self.inference_config = config;
        self
    }

    // Convenience methods that delegate to hints

    /// Set the domain.
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.hints.domain = Some(domain.into());
        self
    }

    /// Set the study name.
    pub fn with_study_name(mut self, name: impl Into<String>) -> Self {
        self.hints.study_name = Some(name.into());
        self
    }

    /// Set the identifier column.
    pub fn with_identifier_column(mut self, column: impl Into<String>) -> Self {
        self.hints.identifier_column = Some(column.into());
        self
    }

    /// Get the domain if set.
    pub fn domain(&self) -> Option<&str> {
        self.hints.domain.as_deref()
    }

    /// Get the study name if set.
    pub fn study_name(&self) -> Option<&str> {
        self.hints.study_name.as_deref()
    }

    /// Convert to the simpler ContextHints for LLM prompts.
    pub fn to_context_hints(&self) -> ContextHints {
        ContextHints {
            study_name: self.hints.study_name.clone(),
            domain: self.hints.domain.clone(),
            expected_sample_count: self.hints.expected_sample_count,
            identifier_column: self.hints.identifier_column.clone(),
            column_hints: self.hints.column_hints.clone(),
            custom: self.hints.custom.clone(),
            related_files: self.file_context.related_files.clone(),
            data_source: self.file_context.extraction_source.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_context() {
        let ctx = CurationContext::new()
            .with_domain("biomedical")
            .with_study_name("IBD Cohort");

        assert_eq!(ctx.domain(), Some("biomedical"));
        assert_eq!(ctx.study_name(), Some("IBD Cohort"));
    }

    #[test]
    fn test_from_context_hints() {
        let hints = ContextHints::new()
            .with_domain("clinical")
            .with_study_name("Phase 2 Trial")
            .with_identifier_column("patient_id");

        let ctx = CurationContext::from_hints(&hints);

        assert_eq!(ctx.hints.domain, Some("clinical".to_string()));
        assert_eq!(ctx.hints.study_name, Some("Phase 2 Trial".to_string()));
        assert_eq!(ctx.hints.identifier_column, Some("patient_id".to_string()));
    }

    #[test]
    fn test_to_context_hints() {
        let ctx = CurationContext::new()
            .with_domain("financial")
            .with_study_name("Q4 Analysis");

        let hints = ctx.to_context_hints();

        assert_eq!(hints.domain, Some("financial".to_string()));
        assert_eq!(hints.study_name, Some("Q4 Analysis".to_string()));
    }

    #[test]
    fn test_inference_config() {
        let config = InferenceConfig::new()
            .with_llm("claude-3-opus")
            .with_confidence_threshold(0.9);

        assert!(config.llm_enabled);
        assert_eq!(config.llm_model, Some("claude-3-opus".to_string()));
        assert_eq!(config.confidence_threshold, 0.9);
    }

    #[test]
    fn test_serialization() {
        let ctx = CurationContext::new()
            .with_domain("biomedical")
            .with_inference_config(InferenceConfig::new().with_llm("claude-3-opus"));

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: CurationContext = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.domain(), Some("biomedical"));
        assert!(parsed.inference_config.llm_enabled);
    }
}
