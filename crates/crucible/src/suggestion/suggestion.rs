//! Suggestion types for proposed data fixes.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type of action to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionAction {
    /// Normalize format, case, or encoding.
    Standardize,
    /// Convert string values to proper NA/null.
    ConvertNa,
    /// Type conversion (e.g., string to number).
    Coerce,
    /// Standardize date formats to ISO (YYYY-MM-DD).
    ConvertDate,
    /// Add a flag column for human review.
    Flag,
    /// Remove row or column.
    Remove,
    /// Combine duplicate entries.
    Merge,
    /// Rename a column.
    Rename,
    /// Split compound values into multiple columns.
    Split,
    /// Create a computed/derived column.
    Derive,
}

impl SuggestionAction {
    /// Get a human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            SuggestionAction::Standardize => "Standardize",
            SuggestionAction::ConvertNa => "Convert to NA",
            SuggestionAction::Coerce => "Type Coercion",
            SuggestionAction::ConvertDate => "Standardize Dates",
            SuggestionAction::Flag => "Flag for Review",
            SuggestionAction::Remove => "Remove",
            SuggestionAction::Merge => "Merge Duplicates",
            SuggestionAction::Rename => "Rename",
            SuggestionAction::Split => "Split Values",
            SuggestionAction::Derive => "Derive Column",
        }
    }
}

/// A proposed fix for an observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Unique identifier for this suggestion.
    pub id: String,

    /// ID of the observation this suggestion addresses.
    pub observation_id: String,

    /// Type of action to perform.
    pub action: SuggestionAction,

    /// Priority (1 = highest, larger = lower priority).
    pub priority: u8,

    /// Action-specific parameters.
    pub parameters: Value,

    /// Human-readable rationale for the suggestion.
    pub rationale: String,

    /// Number of rows affected by this suggestion.
    pub affected_rows: usize,

    /// Confidence in this suggestion (0.0-1.0).
    pub confidence: f64,

    /// Whether this action can be reversed.
    pub reversible: bool,

    /// When this suggestion was generated.
    pub suggested_at: DateTime<Utc>,

    /// What generated this suggestion.
    pub suggester: String,
}

impl Suggestion {
    /// Create a new suggestion.
    pub fn new(
        observation_id: impl Into<String>,
        action: SuggestionAction,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            id: generate_suggestion_id(),
            observation_id: observation_id.into(),
            action,
            priority: 5, // Default middle priority
            parameters: Value::Null,
            rationale: rationale.into(),
            affected_rows: 0,
            confidence: 0.0,
            reversible: true,
            suggested_at: Utc::now(),
            suggester: String::new(),
        }
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the parameters.
    pub fn with_parameters(mut self, params: Value) -> Self {
        self.parameters = params;
        self
    }

    /// Set affected rows count.
    pub fn with_affected_rows(mut self, count: usize) -> Self {
        self.affected_rows = count;
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set whether reversible.
    pub fn with_reversible(mut self, reversible: bool) -> Self {
        self.reversible = reversible;
        self
    }

    /// Set the suggester name.
    pub fn with_suggester(mut self, suggester: impl Into<String>) -> Self {
        self.suggester = suggester.into();
        self
    }
}

/// Generate a unique suggestion ID.
fn generate_suggestion_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("sug_{:03}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Parameters for standardization suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardizeParams {
    /// Column to standardize.
    pub column: String,
    /// Mapping from current values to standardized values.
    pub mapping: std::collections::HashMap<String, String>,
}

/// Parameters for NA conversion suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertNaParams {
    /// Column to convert.
    pub column: String,
    /// Values to convert to NA.
    pub from_values: Vec<String>,
}

/// Parameters for flag suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagParams {
    /// Column with the issue.
    pub column: String,
    /// Row indices to flag.
    pub rows: Vec<usize>,
    /// Name for the flag column.
    pub flag_column: String,
    /// Value to put in the flag column.
    pub flag_value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_suggestion() {
        let sug = Suggestion::new("obs_001", SuggestionAction::Standardize, "Normalize case")
            .with_priority(1)
            .with_confidence(0.9)
            .with_affected_rows(100);

        assert!(sug.id.starts_with("sug_"));
        assert_eq!(sug.observation_id, "obs_001");
        assert_eq!(sug.action, SuggestionAction::Standardize);
        assert_eq!(sug.priority, 1);
        assert_eq!(sug.confidence, 0.9);
        assert_eq!(sug.affected_rows, 100);
    }

    #[test]
    fn test_action_labels() {
        assert_eq!(SuggestionAction::Standardize.label(), "Standardize");
        assert_eq!(SuggestionAction::ConvertNa.label(), "Convert to NA");
        assert_eq!(SuggestionAction::Flag.label(), "Flag for Review");
    }
}
