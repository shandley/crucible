//! Curation layer for data quality management.
//!
//! The curation layer is a JSON document that captures all inferences,
//! observations, suggestions, and decisions for a dataset. It sits alongside
//! the original data without modifying it.
//!
//! # Overview
//!
//! ```text
//! data/
//! ├── metadata.tsv                    # Original data (never modified)
//! └── metadata.curation.json          # Curation layer
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use crucible::{Crucible, MockProvider};
//! use crucible::curation::{CurationLayer, CurationContext};
//!
//! // Analyze data and create curation layer
//! let crucible = Crucible::new().with_llm(MockProvider::new());
//! let result = crucible.analyze("metadata.tsv").unwrap();
//!
//! let mut curation = CurationLayer::from_analysis(
//!     result,
//!     CurationContext::new().with_domain("biomedical")
//! );
//!
//! // Review and decide on suggestions
//! curation.accept("sug_001").unwrap();
//! curation.reject("sug_002", "Not applicable").unwrap();
//!
//! // Persist
//! curation.save("metadata.curation.json").unwrap();
//!
//! // Later, load and continue
//! let curation = CurationLayer::load("metadata.curation.json").unwrap();
//! println!("Pending: {}", curation.pending_suggestions().len());
//! ```

mod context;
mod decision;
mod layer;
mod persistence;

pub use context::{CurationContext, FileContext, InferenceConfig, UserHints};
pub use decision::{Decision, DecisionStatus};
pub use layer::{CurationLayer, CurationSummary, SuggestionCounts, CRUCIBLE_VERSION};
pub use persistence::{crucible_curation_path, curation_path};
