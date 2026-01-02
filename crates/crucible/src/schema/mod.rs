//! Schema types for representing inferred table structure.

mod column;
mod table;
mod types;

pub use column::{ColumnSchema, ColumnStatistics, NumericStatistics, StringStatistics};
pub use table::{CrossColumnRule, RowConstraint, TableSchema};
pub use types::{ColumnType, Constraint, SemanticRole, SemanticType};
