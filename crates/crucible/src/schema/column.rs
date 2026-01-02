//! Column schema definition and statistics.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::types::{ColumnType, Constraint, SemanticRole, SemanticType};

/// Statistics computed for a column.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnStatistics {
    /// Total number of values (including nulls).
    pub count: usize,
    /// Number of null/missing values.
    pub null_count: usize,
    /// Number of unique non-null values.
    pub unique_count: usize,
    /// Sample of values for display.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sample_values: Vec<String>,
    /// Value frequency counts (for categorical).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_counts: Option<IndexMap<String, usize>>,
    /// Numeric statistics (for numeric columns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric: Option<NumericStatistics>,
    /// String statistics (for string columns).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string: Option<StringStatistics>,
}

/// Statistics for numeric columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericStatistics {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub median: f64,
    /// First quartile (25th percentile).
    pub q1: f64,
    /// Third quartile (75th percentile).
    pub q3: f64,
}

impl NumericStatistics {
    /// Calculate the interquartile range.
    pub fn iqr(&self) -> f64 {
        self.q3 - self.q1
    }

    /// Check if a value is an outlier using the IQR method.
    pub fn is_outlier_iqr(&self, value: f64, multiplier: f64) -> bool {
        let iqr = self.iqr();
        let lower = self.q1 - multiplier * iqr;
        let upper = self.q3 + multiplier * iqr;
        value < lower || value > upper
    }

    /// Calculate the z-score for a value.
    pub fn z_score(&self, value: f64) -> f64 {
        if self.std == 0.0 {
            0.0
        } else {
            (value - self.mean) / self.std
        }
    }
}

/// Statistics for string columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringStatistics {
    pub min_length: usize,
    pub max_length: usize,
    pub avg_length: f64,
}

/// Schema for a single column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    /// Column name.
    pub name: String,
    /// Zero-based position in the table.
    pub position: usize,
    /// Inferred data type.
    pub inferred_type: ColumnType,
    /// Semantic type classification.
    #[serde(default)]
    pub semantic_type: SemanticType,
    /// Semantic role in the dataset.
    #[serde(default)]
    pub semantic_role: SemanticRole,
    /// Whether null values are present.
    pub nullable: bool,
    /// Whether all non-null values are unique.
    pub unique: bool,
    /// Expected values for categorical columns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_values: Option<Vec<String>>,
    /// Expected range for numeric columns.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_range: Option<(f64, f64)>,
    /// Inferred constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<Constraint>,
    /// Computed statistics.
    pub statistics: ColumnStatistics,
    /// Confidence in the inferred schema (0.0-1.0).
    pub confidence: f64,
    /// Sources that contributed to the inference.
    #[serde(default)]
    pub inference_sources: Vec<String>,
    /// LLM-generated insight (when available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_insight: Option<String>,
}

impl ColumnSchema {
    /// Create a new column schema with basic information.
    pub fn new(name: impl Into<String>, position: usize) -> Self {
        Self {
            name: name.into(),
            position,
            inferred_type: ColumnType::Unknown,
            semantic_type: SemanticType::Unknown,
            semantic_role: SemanticRole::Unknown,
            nullable: false,
            unique: false,
            expected_values: None,
            expected_range: None,
            constraints: Vec::new(),
            statistics: ColumnStatistics::default(),
            confidence: 0.0,
            inference_sources: Vec::new(),
            llm_insight: None,
        }
    }

    /// Check if this column appears to be an identifier.
    pub fn is_likely_identifier(&self) -> bool {
        self.unique
            && !self.nullable
            && matches!(self.semantic_role, SemanticRole::Identifier)
    }

    /// Get the null percentage.
    pub fn null_percentage(&self) -> f64 {
        if self.statistics.count == 0 {
            0.0
        } else {
            (self.statistics.null_count as f64 / self.statistics.count as f64) * 100.0
        }
    }
}
