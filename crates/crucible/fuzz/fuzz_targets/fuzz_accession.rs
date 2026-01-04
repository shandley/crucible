//! Fuzz target for AccessionValidator.
//!
//! This fuzzer tests that the accession validator:
//! 1. Never panics on any input
//! 2. Correctly handles malformed accession formats
//! 3. Doesn't have overlapping patterns that cause issues

#![no_main]

use libfuzzer_sys::fuzz_target;
use crucible::bio::AccessionValidator;

fuzz_target!(|data: &[u8]| {
    let validator = AccessionValidator::new();

    // Try to interpret as UTF-8 string
    if let Ok(input) = std::str::from_utf8(data) {
        // Validate the input - should never panic
        let result = validator.validate(input);

        // Access all fields to ensure no lazy evaluation panics
        let _ = result.is_valid;
        let _ = result.accession_type;
        let _ = result.normalized;
        let _ = result.error;
        let _ = result.archive;

        // Test URL generation
        let _ = validator.get_url(input);
    }

    // Test with lossy UTF-8 conversion
    let lossy = String::from_utf8_lossy(data);
    let _ = validator.validate(&lossy);
});
