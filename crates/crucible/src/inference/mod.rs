//! Inference engine for schema detection and analysis.

mod fusion;
mod semantic;
mod statistical;

pub use fusion::{FusedInference, FusionConfig, InferenceFusion};
pub use semantic::{SemanticAnalysis, SemanticAnalyzer};
pub use statistical::{StatisticalAnalysis, StatisticalAnalyzer};
