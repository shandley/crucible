# Golden File Test Data

This directory contains reference datasets for deterministic testing of Crucible's analysis pipeline. Each subdirectory represents a specific test scenario with:

- `input.tsv` - The input data file
- `expected_observations.json` - Expected observations (issues detected)
- `expected_schema.json` - Expected inferred schema
- `manifest.json` - Test metadata and configuration

## Test Cases

| Test Case | Purpose | Key Features Tested |
|-----------|---------|---------------------|
| `case_consistency/` | Detect case variants in categorical columns | Case normalization, grouping detection |
| `date_formats/` | Mixed date format detection | Date parsing, format standardization |
| `taxonomy_validation/` | NCBI taxonomy checking | Abbreviation expansion, typo detection |
| `accession_formats/` | Database accession validation | BioSample, SRA, GenBank patterns |
| `outlier_detection/` | Statistical outlier flagging | IQR method, impossible values |
| `mixs_compliance/` | MIxS standard compliance | Mandatory fields, environmental packages |

## Adding New Test Cases

1. Create a new subdirectory with a descriptive name
2. Add `input.tsv` with test data
3. Run `cargo run -- analyze input.tsv --json > expected_observations.json`
4. Add `manifest.json` with test metadata
5. Document the expected behavior

## Updating Golden Files

When intentional changes are made to Crucible's analysis:

```bash
# Regenerate all golden files
cargo test --features update-golden

# Or manually for specific test
cargo run -- analyze test_data/golden/<case>/input.tsv --json > expected.json
```
