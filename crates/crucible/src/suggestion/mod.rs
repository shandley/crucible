//! Suggestions for data quality improvements.
//!
//! This module defines types for proposed fixes to data quality issues,
//! including standardization, type coercion, and value conversion.

mod suggestion;

pub use suggestion::{
    ConvertNaParams, FlagParams, StandardizeParams, Suggestion, SuggestionAction,
};
