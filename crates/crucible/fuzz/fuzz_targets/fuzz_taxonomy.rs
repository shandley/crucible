//! Fuzz target for TaxonomyValidator.
//!
//! This fuzzer tests that the taxonomy validator:
//! 1. Never panics on any UTF-8 input
//! 2. Never panics on arbitrary byte sequences
//! 3. Handles all edge cases gracefully

#![no_main]

use libfuzzer_sys::fuzz_target;
use crucible::bio::TaxonomyValidator;

fuzz_target!(|data: &[u8]| {
    let validator = TaxonomyValidator::new();

    // Try to interpret as UTF-8 string
    if let Ok(input) = std::str::from_utf8(data) {
        // Validate the input - should never panic
        let _ = validator.validate(input);

        // Also test abbreviation expansion
        let _ = validator.expand_abbreviation(input);
    }

    // Even with invalid UTF-8, creating from lossy should work
    let lossy = String::from_utf8_lossy(data);
    let _ = validator.validate(&lossy);
});
