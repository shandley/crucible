# Crucible Fuzz Testing

This directory contains fuzz targets for testing Crucible's robustness against malformed inputs.

## Prerequisites

Fuzzing requires **nightly Rust** via rustup:

```bash
# Install rustup (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install nightly toolchain
rustup install nightly

# Install cargo-fuzz
cargo install cargo-fuzz
```

## Fuzz Targets

| Target | Description | Tests |
|--------|-------------|-------|
| `fuzz_taxonomy` | Taxonomy validator | UTF-8 handling, abbreviation expansion |
| `fuzz_accession` | Accession validator | Format patterns, URL generation |
| `fuzz_ontology` | Ontology mapper | ID lookup, suggestion generation |
| `fuzz_parser` | CSV/TSV parser | Delimiter detection, malformed files |
| `fuzz_date` | Type inference | Date detection, regex patterns |

## Running Fuzz Tests

```bash
# Change to the crucible crate directory
cd crates/crucible

# List available targets
cargo +nightly fuzz list

# Run a specific target (runs indefinitely until stopped)
cargo +nightly fuzz run fuzz_taxonomy

# Run with a time limit (e.g., 60 seconds)
cargo +nightly fuzz run fuzz_taxonomy -- -max_total_time=60

# Run all targets for 30 seconds each
for target in fuzz_taxonomy fuzz_accession fuzz_ontology fuzz_parser fuzz_date; do
    echo "Fuzzing $target..."
    cargo +nightly fuzz run $target -- -max_total_time=30
done
```

## Corpus Management

Fuzz corpora are stored in `fuzz/corpus/<target_name>/`. These contain inputs that triggered new code paths.

```bash
# Minimize corpus (remove redundant inputs)
cargo +nightly fuzz cmin fuzz_taxonomy

# View corpus statistics
ls -la fuzz/corpus/fuzz_taxonomy/ | wc -l
```

## Crash Investigation

When a crash is found:

1. The crashing input is saved to `fuzz/artifacts/<target_name>/`
2. Reproduce the crash:
   ```bash
   cargo +nightly fuzz run fuzz_taxonomy fuzz/artifacts/fuzz_taxonomy/crash-<hash>
   ```
3. Create a regression test from the input
4. Fix the bug and verify the fix

## CI Integration

For CI, run fuzz tests with a short timeout:

```bash
cargo +nightly fuzz run fuzz_taxonomy -- -max_total_time=60 -jobs=4
```

## Coverage

Generate coverage reports for fuzz testing:

```bash
cargo +nightly fuzz coverage fuzz_taxonomy
```
