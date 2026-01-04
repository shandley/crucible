//! Fuzz target for date detection and type inference.
//!
//! This fuzzer tests that the type inference engine:
//! 1. Never panics on any input values
//! 2. Correctly handles malformed dates
//! 3. Regex-based date detection doesn't crash on pathological input

#![no_main]

use libfuzzer_sys::fuzz_target;
use crucible::{Crucible, MockProvider};
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Only process reasonable-sized inputs
    if data.len() > 10_000 {
        return;
    }

    // Try to interpret as UTF-8 string
    if let Ok(content) = std::str::from_utf8(data) {
        // Create a minimal TSV with the fuzzed content as values
        // Header + single data row
        let tsv_content = format!("col1\tcol2\tcol3\n{}\t{}\t{}\n", content, content, content);

        // Write to temp file for analysis
        if let Ok(mut temp_file) = tempfile::NamedTempFile::with_suffix(".tsv") {
            if temp_file.write_all(tsv_content.as_bytes()).is_ok() {
                let path = temp_file.path();

                // Try full analysis - this exercises type inference
                let crucible = Crucible::new().with_llm(MockProvider::new());
                let _ = crucible.analyze(path);
            }
        }
    }
});
