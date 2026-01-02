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
//! - **LLM-enhanced**: Optional LLM integration for semantic insights and suggestions
//!
//! # Basic Example
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
//!
//! # With LLM Enhancement
//!
//! ```no_run
//! use crucible::{AnthropicProvider, ContextHints, Crucible};
//!
//! let crucible = Crucible::new()
//!     .with_llm(AnthropicProvider::from_env().unwrap())
//!     .with_context(ContextHints::new()
//!         .with_domain("biomedical")
//!         .with_study_name("IBD Cohort Study"));
//!
//! let result = crucible.analyze("metadata.tsv").unwrap();
//!
//! // Schema columns now have LLM-generated insights
//! for col in &result.schema.columns {
//!     if let Some(insight) = &col.llm_insight {
//!         println!("{}: {}", col.name, insight);
//!     }
//! }
//!
//! // Suggestions for fixing data quality issues
//! for suggestion in &result.suggestions {
//!     println!("{:?}: {}", suggestion.action, suggestion.rationale);
//! }
//! ```

pub mod error;
pub mod inference;
pub mod input;
pub mod llm;
pub mod schema;
pub mod suggestion;
pub mod validation;

mod crucible;

pub use crate::crucible::{AnalysisResult, Crucible, CrucibleConfig};
pub use error::{CrucibleError, Result};
pub use input::{ContextHints, DataTable, SourceMetadata};
pub use llm::{AnthropicProvider, LlmConfig, LlmProvider, MockProvider, SchemaEnhancement};
pub use schema::{ColumnSchema, ColumnType, Constraint, SemanticRole, TableSchema};
pub use suggestion::{Suggestion, SuggestionAction};
pub use validation::{Observation, ObservationType, Severity};
