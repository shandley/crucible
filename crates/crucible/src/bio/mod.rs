//! Bioinformatics-specific validators and data structures.
//!
//! This module provides specialized validation for biological and biomedical
//! metadata, including:
//!
//! - MIxS (Minimum Information about any (x) Sequence) compliance checking
//! - NCBI Taxonomy validation
//! - Ontology term mapping (ENVO, UBERON, MONDO)
//! - BioSample submission pre-validation
//!
//! # Example
//!
//! ```no_run
//! use crucible::bio::{MixsValidator, MixsPackage};
//!
//! let validator = MixsValidator::new()
//!     .with_package(MixsPackage::HumanGut);
//!
//! // Check for missing mandatory fields
//! let observations = validator.validate(&data, &schema);
//! ```

mod mixs;
mod taxonomy;
mod validators;

pub use mixs::{
    MixsField, MixsFieldRequirement, MixsPackage, MixsSchema, MIXS_CORE_FIELDS,
};
pub use taxonomy::{TaxonomyEntry, TaxonomyValidator};
pub use validators::{BioValidator, MixsComplianceValidator};
