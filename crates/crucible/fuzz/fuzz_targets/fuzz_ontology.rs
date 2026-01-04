//! Fuzz target for OntologyValidator.
//!
//! This fuzzer tests that the ontology validator:
//! 1. Never panics on any input
//! 2. Handles malformed ontology IDs gracefully
//! 3. Suggestion algorithm doesn't crash on edge cases

#![no_main]

use libfuzzer_sys::fuzz_target;
use crucible::bio::{OntologyValidator, OntologyType};

fuzz_target!(|data: &[u8]| {
    let validator = OntologyValidator::new();

    // Try to interpret as UTF-8 string
    if let Ok(input) = std::str::from_utf8(data) {
        // Test lookup by ID
        let _ = validator.lookup_by_id(input);

        // Test lookup by label
        let _ = validator.lookup_by_label(input);

        // Test ID validation
        let _ = validator.validate_id(input);

        // Test suggestion generation with no filter
        let _ = validator.suggest_mappings(input, None);

        // Test with specific ontology types
        let _ = validator.suggest_mappings(input, Some(OntologyType::Envo));
        let _ = validator.suggest_mappings(input, Some(OntologyType::Uberon));
        let _ = validator.suggest_mappings(input, Some(OntologyType::Mondo));
    }

    // Test with lossy UTF-8 conversion
    let lossy = String::from_utf8_lossy(data);
    let _ = validator.suggest_mappings(&lossy, None);
});
