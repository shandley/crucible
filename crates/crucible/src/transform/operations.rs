//! Transformation operations that can be applied to data.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A transformation operation to apply to data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformOperation {
    /// Replace values in a column based on a mapping.
    Standardize {
        column: String,
        mapping: HashMap<String, String>,
    },

    /// Add a flag column for specific rows.
    Flag {
        source_column: String,
        flag_column: String,
        rows: Vec<usize>,
        flag_value: String,
    },

    /// Convert values to null.
    ConvertNa {
        column: String,
        values: Vec<String>,
    },

    /// Coerce values to a specific type (non-convertible become NA).
    Coerce {
        column: String,
        target_type: String,
        rows: Vec<usize>,
    },

    /// No operation - just a marker that the suggestion was acknowledged.
    NoOp {
        reason: String,
    },
}

impl TransformOperation {
    /// Get a human-readable description of the operation.
    pub fn description(&self) -> String {
        match self {
            TransformOperation::Standardize { column, mapping } => {
                let examples: Vec<String> = mapping
                    .iter()
                    .take(3)
                    .map(|(from, to)| format!("'{}' → '{}'", from, to))
                    .collect();
                format!("Standardize '{}': {}", column, examples.join(", "))
            }
            TransformOperation::Flag {
                source_column,
                flag_column,
                rows,
                ..
            } => {
                format!(
                    "Flag {} rows in '{}' → '{}'",
                    rows.len(),
                    source_column,
                    flag_column
                )
            }
            TransformOperation::ConvertNa { column, values } => {
                format!("Convert {:?} to NA in '{}'", values, column)
            }
            TransformOperation::Coerce {
                column,
                target_type,
                rows,
            } => {
                format!(
                    "Coerce {} values in '{}' to {}",
                    rows.len(),
                    column,
                    target_type
                )
            }
            TransformOperation::NoOp { reason } => {
                format!("No action: {}", reason)
            }
        }
    }
}

/// Result of applying transformations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    /// Number of operations applied.
    pub operations_applied: usize,

    /// Number of rows modified.
    pub rows_modified: usize,

    /// Number of columns added.
    pub columns_added: usize,

    /// Detailed changes for each operation.
    pub changes: Vec<TransformChange>,
}

/// A single change made during transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformChange {
    /// Description of the change.
    pub description: String,

    /// Column affected.
    pub column: String,

    /// Number of values changed.
    pub values_changed: usize,

    /// Per-row audit information.
    pub row_audits: Vec<RowAudit>,
}

/// Audit information for a single row change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowAudit {
    /// Row index (0-based).
    pub row: usize,

    /// Column that was changed.
    pub column: String,

    /// Original value before transformation.
    pub original_value: String,

    /// New value after transformation.
    pub new_value: String,

    /// Type of transformation applied.
    pub transform_type: String,

    /// Reason for the change.
    pub reason: String,
}

impl TransformResult {
    /// Create an empty result.
    pub fn new() -> Self {
        Self {
            operations_applied: 0,
            rows_modified: 0,
            columns_added: 0,
            changes: Vec::new(),
        }
    }

    /// Add a change to the result.
    pub fn add_change(&mut self, change: TransformChange) {
        self.operations_applied += 1;
        self.rows_modified += change.values_changed;
        self.changes.push(change);
    }
}

impl Default for TransformResult {
    fn default() -> Self {
        Self::new()
    }
}
