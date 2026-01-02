//! Validation engine for detecting data quality issues.

mod observation;
mod validators;

pub use observation::{Evidence, Observation, ObservationType, Severity};
pub use validators::{
    CompletenessValidator, ConsistencyValidator, MissingPatternValidator, RangeValidator,
    SetValidator, TypeValidator, UniquenessValidator, ValidationEngine, Validator,
};
