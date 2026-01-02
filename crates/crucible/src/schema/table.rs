//! Table-level schema definition.

use serde::{Deserialize, Serialize};

use super::column::ColumnSchema;

/// A constraint that spans multiple columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RowConstraint {
    /// One or more columns form a unique identifier.
    UniqueIdentifier {
        columns: Vec<String>,
        confidence: f64,
    },
    /// A combination of columns must be unique.
    UniqueComposite {
        columns: Vec<String>,
        confidence: f64,
    },
}

/// A rule relating multiple columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossColumnRule {
    /// Type of relationship.
    #[serde(rename = "type")]
    pub rule_type: String,
    /// Human-readable description.
    pub description: String,
    /// Condition expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Expected outcome.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expectation: Option<String>,
    /// Confidence in this rule.
    pub confidence: f64,
}

/// Schema for an entire table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    /// Schemas for each column.
    pub columns: Vec<ColumnSchema>,
    /// Row-level constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub row_constraints: Vec<RowConstraint>,
    /// Cross-column rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_column_rules: Vec<CrossColumnRule>,
}

impl TableSchema {
    /// Create a new empty table schema.
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            row_constraints: Vec::new(),
            cross_column_rules: Vec::new(),
        }
    }

    /// Create a table schema with the given columns.
    pub fn with_columns(columns: Vec<ColumnSchema>) -> Self {
        Self {
            columns,
            row_constraints: Vec::new(),
            cross_column_rules: Vec::new(),
        }
    }

    /// Get a column by name.
    pub fn get_column(&self, name: &str) -> Option<&ColumnSchema> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get a column by position.
    pub fn get_column_by_position(&self, position: usize) -> Option<&ColumnSchema> {
        self.columns.iter().find(|c| c.position == position)
    }

    /// Get all column names.
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get the number of columns.
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// Find columns with a specific semantic role.
    pub fn columns_with_role(
        &self,
        role: super::types::SemanticRole,
    ) -> impl Iterator<Item = &ColumnSchema> {
        self.columns.iter().filter(move |c| c.semantic_role == role)
    }

    /// Find the likely identifier column(s).
    pub fn identifier_columns(&self) -> impl Iterator<Item = &ColumnSchema> {
        self.columns.iter().filter(|c| c.is_likely_identifier())
    }
}

impl Default for TableSchema {
    fn default() -> Self {
        Self::new()
    }
}
