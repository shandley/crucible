//! Bioinformatics-specific validators and data structures.
//!
//! This module provides specialized validation for biological and biomedical
//! metadata, including:
//!
//! - MIxS (Minimum Information about any (x) Sequence) compliance checking
//! - NCBI Taxonomy validation
//! - Ontology term mapping (ENVO, UBERON, MONDO)
//! - BioSample submission pre-validation
//! - Database accession validation (BioSample, SRA, BioProject, GenBank, RefSeq)
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

mod accession;
mod biosample;
mod mixs;
mod ontology;
mod taxonomy;
mod validators;

pub use accession::{
    AccessionStats, AccessionType, AccessionValidationResult, AccessionValidator,
};
pub use biosample::{
    BioSampleValidator, IssueCategory, NcbiReadiness, ReadinessIssue, ReadinessStats,
};
pub use mixs::{
    MixsField, MixsFieldRequirement, MixsPackage, MixsSchema, MIXS_CORE_FIELDS,
};
pub use ontology::{
    MatchType, OntologyMapping, OntologyStats, OntologyTerm, OntologyType, OntologyValidationResult,
    OntologyValidator,
};
pub use taxonomy::{TaxonomyEntry, TaxonomyStats, TaxonomyValidationResult, TaxonomyValidator};
pub use validators::{BioValidator, MixsComplianceValidator};
