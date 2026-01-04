//! Fuzz target for the data parser.
//!
//! This fuzzer tests that the CSV/TSV parser:
//! 1. Never panics on malformed input
//! 2. Handles all delimiter combinations
//! 3. Doesn't allocate unbounded memory

#![no_main]

use libfuzzer_sys::fuzz_target;
use crucible::Parser;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    // Only process reasonable-sized inputs to avoid OOM
    if data.len() > 100_000 {
        return;
    }

    // Write to temp file for parsing
    if let Ok(mut temp_file) = tempfile::NamedTempFile::new() {
        if temp_file.write_all(data).is_ok() {
            let path = temp_file.path();

            // Try parsing with auto-detection
            let parser = Parser::new();
            let _ = parser.parse_file(path);
        }
    }
});
