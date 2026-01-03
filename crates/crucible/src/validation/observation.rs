//! Observation types for data quality issues.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Type of observation/issue detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationType {
    /// NA-like values not properly encoded (e.g., "missing", "N/A").
    MissingPattern,
    /// Format/case variations of the same concept.
    Inconsistency,
    /// Values outside expected range.
    Outlier,
    /// Duplicate rows or identifiers.
    Duplicate,
    /// Values don't match inferred type.
    TypeMismatch,
    /// Violates an inferred constraint.
    ConstraintViolation,
    /// High rate of missing values.
    Completeness,
    /// Unexpected number of unique values.
    Cardinality,
    /// Cross-column rule violation.
    CrossColumn,
    /// Value doesn't match expected pattern (email, URL, identifier format).
    PatternViolation,
    /// Logical inconsistency between related columns.
    CrossColumnInconsistency,
}

impl ObservationType {
    /// Get a human-readable label for the observation type.
    pub fn label(&self) -> &'static str {
        match self {
            ObservationType::MissingPattern => "Missing Pattern",
            ObservationType::Inconsistency => "Inconsistency",
            ObservationType::Outlier => "Outlier",
            ObservationType::Duplicate => "Duplicate",
            ObservationType::TypeMismatch => "Type Mismatch",
            ObservationType::ConstraintViolation => "Constraint Violation",
            ObservationType::Completeness => "Completeness Issue",
            ObservationType::Cardinality => "Cardinality Issue",
            ObservationType::CrossColumn => "Cross-Column Issue",
            ObservationType::PatternViolation => "Pattern Violation",
            ObservationType::CrossColumnInconsistency => "Cross-Column Inconsistency",
        }
    }
}

/// Severity level of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Informational only, may not require action.
    Info,
    /// Potential issue that should be reviewed.
    Warning,
    /// Definite issue that should be addressed.
    Error,
}

impl Severity {
    /// Get a human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "Info",
            Severity::Warning => "Warning",
            Severity::Error => "Error",
        }
    }
}

/// Evidence supporting an observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// The problematic value(s).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// Pattern detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Number of occurrences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrences: Option<usize>,
    /// Percentage of affected rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
    /// Sample row indices.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sample_rows: Vec<usize>,
    /// Expected value or range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Value>,
    /// Actual value counts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_counts: Option<Value>,
    /// Z-score for outliers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub z_score: Option<f64>,
}

impl Evidence {
    /// Create empty evidence.
    pub fn new() -> Self {
        Self {
            value: None,
            pattern: None,
            occurrences: None,
            percentage: None,
            sample_rows: Vec::new(),
            expected: None,
            value_counts: None,
            z_score: None,
        }
    }

    /// Set the value.
    pub fn with_value(mut self, value: impl Into<Value>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Set the pattern.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Set occurrences.
    pub fn with_occurrences(mut self, count: usize) -> Self {
        self.occurrences = Some(count);
        self
    }

    /// Set percentage.
    pub fn with_percentage(mut self, pct: f64) -> Self {
        self.percentage = Some(pct);
        self
    }

    /// Set sample rows.
    pub fn with_sample_rows(mut self, rows: Vec<usize>) -> Self {
        self.sample_rows = rows;
        self
    }

    /// Set expected value.
    pub fn with_expected(mut self, expected: impl Into<Value>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    /// Set z-score.
    pub fn with_z_score(mut self, z: f64) -> Self {
        self.z_score = Some(z);
        self
    }

    /// Set value counts.
    pub fn with_value_counts(mut self, counts: Option<Value>) -> Self {
        self.value_counts = counts;
        self
    }
}

impl Default for Evidence {
    fn default() -> Self {
        Self::new()
    }
}

/// An observation about data quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Unique identifier for this observation.
    pub id: String,
    /// Type of issue.
    #[serde(rename = "type")]
    pub observation_type: ObservationType,
    /// Severity level.
    pub severity: Severity,
    /// Affected column name.
    pub column: String,
    /// Human-readable description.
    pub description: String,
    /// Supporting evidence.
    pub evidence: Evidence,
    /// Confidence in this observation (0.0-1.0).
    pub confidence: f64,
    /// When detected.
    pub detected_at: DateTime<Utc>,
    /// What detected this issue.
    pub detector: String,
    /// LLM-generated explanation (when available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_explanation: Option<String>,
}

impl Observation {
    /// Create a new observation.
    pub fn new(
        observation_type: ObservationType,
        severity: Severity,
        column: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: generate_observation_id(),
            observation_type,
            severity,
            column: column.into(),
            description: description.into(),
            evidence: Evidence::new(),
            confidence: 0.0,
            detected_at: Utc::now(),
            detector: String::new(),
            llm_explanation: None,
        }
    }

    /// Set the evidence.
    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence = evidence;
        self
    }

    /// Set the confidence.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set the detector name.
    pub fn with_detector(mut self, detector: impl Into<String>) -> Self {
        self.detector = detector.into();
        self
    }
}

/// Generate a unique observation ID.
fn generate_observation_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    format!("obs_{:03}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_observation() {
        let obs = Observation::new(
            ObservationType::MissingPattern,
            Severity::Warning,
            "status",
            "String 'missing' appears to represent NA values",
        )
        .with_confidence(0.92)
        .with_detector("statistical_analyzer");

        assert!(obs.id.starts_with("obs_"));
        assert_eq!(obs.severity, Severity::Warning);
        assert_eq!(obs.column, "status");
    }

    #[test]
    fn test_evidence_builder() {
        let evidence = Evidence::new()
            .with_pattern("missing")
            .with_occurrences(193)
            .with_percentage(14.2)
            .with_sample_rows(vec![5, 12, 23]);

        assert_eq!(evidence.pattern, Some("missing".to_string()));
        assert_eq!(evidence.occurrences, Some(193));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }
}
