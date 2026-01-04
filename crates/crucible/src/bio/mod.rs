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
//! ```ignore
//! use crucible::bio::{BioValidator, MixsComplianceValidator, MixsPackage};
//!
//! let validator = MixsComplianceValidator::new()
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
pub use taxonomy::{TaxonomyEntry, TaxonomyStats, TaxonomyValidator};
pub use validators::{BioValidator, MixsComplianceValidator};
