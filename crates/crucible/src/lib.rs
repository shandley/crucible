//! Crucible: LLM-native data curation tool for tabular datasets.
//!
//! Crucible takes an intent-driven approach to data validation where an AI system
//! infers validation rules from data context rather than requiring manual rule definition.
//!
//! # Core Principles
//!
//! - **Intent-driven**: Infer what the data *should* look like from context
//! - **Non-destructive**: Original data is never modified
//! - **Full provenance**: Every observation and decision is tracked
//!
//! # Example
//!
//! ```no_run
//! use crucible::Crucible;
//!
//! let crucible = Crucible::new();
//! let result = crucible.analyze("metadata.tsv").unwrap();
//!
//! println!("Columns: {}", result.schema.columns.len());
//! println!("Observations: {}", result.observations.len());
//! ```

pub mod error;
pub mod input;
pub mod inference;
pub mod schema;
pub mod validation;

mod crucible;

pub use crate::crucible::{AnalysisResult, Crucible, CrucibleConfig};
pub use error::{CrucibleError, Result};
pub use input::{DataTable, SourceMetadata};
pub use schema::{ColumnSchema, ColumnType, Constraint, SemanticRole, TableSchema};
pub use validation::{Observation, ObservationType, Severity};
