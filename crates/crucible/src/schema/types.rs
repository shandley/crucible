//! Core type definitions for schema representation.

use serde::{Deserialize, Serialize};

/// Inferred data type for a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    /// Whole numbers (no decimal point).
    Integer,
    /// Floating-point numbers.
    Float,
    /// Text/string values.
    String,
    /// Boolean values (true/false).
    Boolean,
    /// Date and/or time values.
    DateTime,
    /// Date only (no time component).
    Date,
    /// Time only (no date component).
    Time,
    /// Unable to determine type.
    Unknown,
}

impl ColumnType {
    /// Returns true if this type is numeric.
    pub fn is_numeric(&self) -> bool {
        matches!(self, ColumnType::Integer | ColumnType::Float)
    }

    /// Returns true if this type is temporal.
    pub fn is_temporal(&self) -> bool {
        matches!(
            self,
            ColumnType::DateTime | ColumnType::Date | ColumnType::Time
        )
    }
}

impl Default for ColumnType {
    fn default() -> Self {
        ColumnType::Unknown
    }
}

/// Semantic role of a column in the dataset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticRole {
    /// Unique identifier for rows (e.g., sample_id, patient_id).
    Identifier,
    /// Grouping/categorical variable (e.g., diagnosis, treatment).
    Grouping,
    /// Continuous covariate (e.g., age, weight).
    Covariate,
    /// Outcome/response variable.
    Outcome,
    /// Metadata about the data (e.g., date_collected).
    Metadata,
    /// Unable to determine role.
    Unknown,
}

impl Default for SemanticRole {
    fn default() -> Self {
        SemanticRole::Unknown
    }
}

/// Semantic type providing more specific categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticType {
    /// Unique identifier.
    Identifier,
    /// Categorical variable with discrete values.
    Categorical,
    /// Continuous numeric variable.
    Continuous,
    /// Ordinal variable (ordered categories).
    Ordinal,
    /// Binary variable (two values).
    Binary,
    /// Count data (non-negative integers).
    Count,
    /// Proportion/percentage (0-1 or 0-100).
    Proportion,
    /// Free text.
    FreeText,
    /// Unable to determine.
    Unknown,
}

impl Default for SemanticType {
    fn default() -> Self {
        SemanticType::Unknown
    }
}

/// A constraint on column values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// Values must match a regex pattern.
    Pattern {
        value: String,
        confidence: f64,
    },
    /// Values must be in a specific set.
    SetMembership {
        values: Vec<String>,
        confidence: f64,
    },
    /// Numeric values must be in a range.
    Range {
        min: Option<f64>,
        max: Option<f64>,
        confidence: f64,
    },
    /// String length constraint.
    Length {
        min: Option<usize>,
        max: Option<usize>,
        confidence: f64,
    },
    /// Values must be unique.
    Unique {
        confidence: f64,
    },
    /// Values must not be null.
    NotNull {
        confidence: f64,
    },
}

impl Constraint {
    /// Get the confidence level for this constraint.
    pub fn confidence(&self) -> f64 {
        match self {
            Constraint::Pattern { confidence, .. } => *confidence,
            Constraint::SetMembership { confidence, .. } => *confidence,
            Constraint::Range { confidence, .. } => *confidence,
            Constraint::Length { confidence, .. } => *confidence,
            Constraint::Unique { confidence } => *confidence,
            Constraint::NotNull { confidence } => *confidence,
        }
    }
}
