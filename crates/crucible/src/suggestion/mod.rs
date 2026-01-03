//! Suggestions for data quality improvements.
//!
//! This module defines types for proposed fixes to data quality issues,
//! including standardization, type coercion, and value conversion.
//!
//! The [`SuggestionEngine`] generates rule-based suggestions from observations
//! without requiring an LLM, while LLM providers can generate enhanced
//! suggestions with more context-aware rationale.

mod generator;
mod suggestion;

pub use generator::SuggestionEngine;
pub use suggestion::{
    ConvertNaParams, FlagParams, StandardizeParams, Suggestion, SuggestionAction,
};
