//! Context hints for LLM-enhanced analysis.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// User-provided and file-derived context hints for LLM enhancement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextHints {
    /// Name of the study or project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub study_name: Option<String>,

    /// Domain of the data (e.g., "biomedical", "financial", "survey").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Expected number of samples/rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_sample_count: Option<usize>,

    /// Name of the identifier column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier_column: Option<String>,

    /// Expected columns and their descriptions.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub column_hints: HashMap<String, String>,

    /// Custom key-value hints.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, String>,

    /// Related files in the same directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_files: Vec<String>,

    /// Source of the data (e.g., "extracted from RISK_CCFA.rds").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
}

impl ContextHints {
    /// Create empty context hints.
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
    pub fn with_expected_samples(mut self, count: usize) -> Self {
        self.expected_sample_count = Some(count);
        self
    }

    /// Set the identifier column.
    pub fn with_identifier_column(mut self, column: impl Into<String>) -> Self {
        self.identifier_column = Some(column.into());
        self
    }

    /// Add a column hint.
    pub fn with_column_hint(
        mut self,
        column: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.column_hints.insert(column.into(), description.into());
        self
    }

    /// Add a custom hint.
    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Check if any hints are provided.
    pub fn is_empty(&self) -> bool {
        self.study_name.is_none()
            && self.domain.is_none()
            && self.expected_sample_count.is_none()
            && self.identifier_column.is_none()
            && self.column_hints.is_empty()
            && self.custom.is_empty()
            && self.related_files.is_empty()
            && self.data_source.is_none()
    }

    /// Format hints as a string for LLM prompts.
    pub fn to_prompt_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref name) = self.study_name {
            parts.push(format!("Study: {}", name));
        }
        if let Some(ref domain) = self.domain {
            parts.push(format!("Domain: {}", domain));
        }
        if let Some(count) = self.expected_sample_count {
            parts.push(format!("Expected samples: {}", count));
        }
        if let Some(ref col) = self.identifier_column {
            parts.push(format!("Identifier column: {}", col));
        }
        if let Some(ref source) = self.data_source {
            parts.push(format!("Data source: {}", source));
        }

        for (key, value) in &self.custom {
            parts.push(format!("{}: {}", key, value));
        }

        if parts.is_empty() {
            "No additional context provided.".to_string()
        } else {
            parts.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder() {
        let ctx = ContextHints::new()
            .with_study_name("RISK Pediatric IBD")
            .with_domain("biomedical")
            .with_expected_samples(1400)
            .with_identifier_column("sample_id");

        assert_eq!(ctx.study_name, Some("RISK Pediatric IBD".to_string()));
        assert_eq!(ctx.domain, Some("biomedical".to_string()));
        assert!(!ctx.is_empty());
    }

    #[test]
    fn test_prompt_string() {
        let ctx = ContextHints::new()
            .with_study_name("Test Study")
            .with_domain("test");

        let prompt = ctx.to_prompt_string();
        assert!(prompt.contains("Study: Test Study"));
        assert!(prompt.contains("Domain: test"));
    }
}
