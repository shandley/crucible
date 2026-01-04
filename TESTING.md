# Crucible Testing Documentation

## Overview

Crucible is a scientific data curation tool designed for bioinformatics metadata validation. As a tool intended for research applications, we maintain rigorous testing standards to ensure **reproducibility**, **accuracy**, and **scientific validity**.

This document describes our testing methodology, validation approaches, and quality assurance practices.

---

## Testing Philosophy

### Core Principles

1. **Determinism**: Given identical input, Crucible must produce identical output
2. **Accuracy**: Validations must correctly identify issues according to established standards
3. **Coverage**: All features must have corresponding tests
4. **Regression Prevention**: Bug fixes must include tests to prevent recurrence
5. **Scientific Validity**: Results must be validated against authoritative sources

---

## Test Categories

### 1. Unit Tests (104 tests)

Unit tests verify individual components in isolation.

```
crates/crucible/src/
├── analysis/       # Data analysis logic
├── bio/           # Bioinformatics validators (47 tests)
│   ├── accession  # Database accession validation
│   ├── biosample  # NCBI BioSample pre-validation
│   ├── mixs       # MIxS standard compliance
│   ├── ontology   # ENVO, UBERON, MONDO mapping
│   └── taxonomy   # NCBI Taxonomy validation
├── schema/        # Type and role inference
└── validation/    # Core validators
```

**Run unit tests:**
```bash
cargo test -p crucible
```

### 2. Integration Tests (32 tests)

Integration tests verify end-to-end analysis workflows.

```
crates/crucible/tests/
├── integration_test.rs  # Full analysis pipelines
├── curation_test.rs     # Curation layer operations
└── golden_tests.rs      # Reference output verification
```

**Key scenarios tested:**
- CSV/TSV parsing and format detection
- Type inference (integer, float, string, boolean, date)
- Semantic role inference (identifier, grouping, covariate)
- Missing value detection (NA, null, empty, "missing")
- Outlier detection (IQR method, impossible values)
- Inconsistency detection (case variants, typos, semantic equivalents)
- LLM integration (mock provider for deterministic testing)

**Run integration tests:**
```bash
cargo test -p crucible --test integration_test
```

### 3. Golden File Tests (7 tests)

Golden file tests ensure deterministic output by comparing analysis results against reference files.

```
test_data/golden/
├── case_consistency/     # Case variant detection
├── date_formats/         # Date format normalization
├── taxonomy_validation/  # NCBI taxonomy checking
├── accession_formats/    # Database accession validation
├── outlier_detection/    # Statistical outlier flagging
└── mixs_compliance/      # MIxS standard compliance
```

Each test case contains:
- `input.tsv` - Input data with known issues
- `expected.json` - Expected curation layer output
- `manifest.json` - Test metadata and assertions

**Run golden tests:**
```bash
cargo test -p crucible --test golden_tests
```

### 4. Regression Tests

Every bug fix includes a test to prevent recurrence:

| Issue | Test | Description |
|-------|------|-------------|
| PDB vs Gene ID | `regression_pdb_vs_gene_id` | Pure numeric gene IDs should not match PDB pattern |
| Short SRA | `regression_short_sra_accessions` | SRR123 should be flagged as invalid (too short) |

### 5. Property-Based Tests (32 tests)

Property-based testing uses [proptest](https://github.com/proptest-rs/proptest) to generate random inputs and verify that validators maintain their invariants under all conditions. This is particularly important for scientific software where edge cases can lead to incorrect results.

```
crates/crucible/tests/property_tests.rs
├── taxonomy_tests       # 8 tests for taxonomy validation
├── accession_tests      # 9 tests for accession validation
├── ontology_tests       # 6 tests for ontology validation
├── date_tests           # 3 tests for date parsing
├── cross_validator_tests# 2 tests for validator independence
└── regression_property_tests # 4 tests for known edge cases
```

**Core properties verified:**

| Property | Description |
|----------|-------------|
| **No panics** | Validators never crash on any UTF-8 input |
| **Determinism** | Same input always produces identical output |
| **Pattern exclusivity** | Accession patterns don't overlap |
| **Known valid inputs** | Standard organisms/accessions always validate |
| **Edge case handling** | Empty strings, special characters handled gracefully |

**Example property test:**
```rust
proptest! {
    /// Taxonomy validator never panics on any UTF-8 input.
    #[test]
    fn never_panics_on_random_utf8(input in random_bytes()) {
        let validator = TaxonomyValidator::new();
        let _ = validator.validate(&input);
    }

    /// Known valid taxa always return Valid result.
    #[test]
    fn known_valid_taxa_are_valid(
        taxon in prop_oneof![
            Just("Escherichia coli"),
            Just("Homo sapiens"),
            Just("Mus musculus"),
        ]
    ) {
        let validator = TaxonomyValidator::new();
        let result = validator.validate(taxon);
        prop_assert!(matches!(result, TaxonomyValidationResult::Valid { .. }));
    }
}
```

**Run property tests:**
```bash
# Standard run (256 cases per test)
cargo test -p crucible --test property_tests

# Thorough run (10,000 cases per test)
PROPTEST_CASES=10000 cargo test -p crucible --test property_tests
```

**Regression properties:**
- Gene IDs (pure numbers) never match PDB pattern
- Short SRA accessions (< 6 digits) never match SraRun type
- Taxonomy abbreviations with various punctuation don't panic

### 6. Real-World Validation Tests (14 tests)

Real-world validation tests verify Crucible against datasets that mirror actual NCBI/EBI submission problems. These tests ensure Crucible detects issues that commonly cause submission rejections.

```
test_data/real_world/
├── README.md                       # Documentation of expected issues
├── ncbi_microbiome_submission.tsv  # 16S/metagenome study (10 samples)
├── ncbi_environmental_submission.tsv # Environmental samples (10 samples)
└── ncbi_human_clinical.tsv         # Human clinical study (10 samples)
```

**Test datasets and documented issues:**

| Dataset | Samples | Issues Tested |
|---------|---------|---------------|
| Microbiome | 10 | Abbreviated organisms, date formats, case inconsistencies, null values, duplicates |
| Environmental | 10 | Coordinate formats, missing values, date inconsistencies |
| Clinical | 10 | Invalid accessions, outliers (impossible ages), organism variations |

**Key validation tests:**
```
test_microbiome_detects_organism_inconsistencies
test_microbiome_detects_date_format_issues
test_microbiome_detects_case_inconsistencies
test_environmental_detects_coordinate_issues
test_environmental_detects_data_quality_issues
test_clinical_detects_invalid_accessions
test_clinical_detects_outliers
test_taxonomy_validator_on_real_names
test_accession_validator_on_real_accessions
test_biosample_validator_on_microbiome_data
```

**Run real-world validation tests:**
```bash
cargo test -p crucible --test real_world_validation_test

# View detection summary
cargo test -p crucible --test real_world_validation_test test_real_world_detection_summary -- --nocapture
```

**Validation sources:**
- [NCBI BioSample Validation Errors](https://www.ncbi.nlm.nih.gov/biosample/docs/submission/validation-errors/)
- [SRA Submission Guidelines](https://www.ncbi.nlm.nih.gov/sra/docs/submit/)

### 7. Ontology Accuracy Tests (18 tests)

Ontology accuracy tests verify that Crucible's built-in ontology terms are correct against authoritative sources. These tests ensure scientific validity by cross-referencing term IDs, labels, and synonyms with official ontology databases.

```
crates/crucible/tests/ontology_accuracy_test.rs
├── envo_tests          # 5 tests for ENVO terms
├── uberon_tests        # 5 tests for UBERON terms
├── mondo_tests         # 5 tests for MONDO terms
└── cross_ontology_tests # 3 tests for cross-cutting concerns
```

**Reference term verification:**

| Ontology | Reference Terms | Coverage |
|----------|-----------------|----------|
| ENVO | 20 terms | Environmental biomes, soils, water bodies |
| UBERON | 23 terms | Anatomical structures, tissues, organs |
| MONDO | 22 terms | Diseases, disorders, conditions |

**Example reference terms verified:**
```rust
// ENVO terms verified against official ontology
("ENVO:00000446", "terrestrial biome"),
("ENVO:00001998", "soil"),
("ENVO:00000015", "ocean"),

// UBERON anatomical terms
("UBERON:0000178", "blood"),
("UBERON:0000160", "intestine"),
("UBERON:0002107", "liver"),

// MONDO disease terms
("MONDO:0005011", "Crohn disease"),
("MONDO:0005148", "type 2 diabetes mellitus"),
("MONDO:0005015", "diabetes mellitus"),
```

**Tests performed:**
```
test_envo_reference_terms_are_accurate
test_envo_term_count_minimum
test_envo_synonyms_resolve_correctly
test_uberon_reference_terms_are_accurate
test_uberon_term_count_minimum
test_uberon_anatomical_hierarchy
test_mondo_reference_terms_are_accurate
test_mondo_term_count_minimum
test_mondo_disease_synonyms
test_no_duplicate_ontology_ids
test_all_ids_have_valid_format
test_suggest_mappings_returns_high_confidence
```

**Run ontology accuracy tests:**
```bash
cargo test -p crucible --test ontology_accuracy_test
```

**Validation sources:**
- [ENVO](https://github.com/EnvironmentOntology/envo) - Environmental Ontology
- [UBERON](https://github.com/obophenotype/uberon) - Anatomy Ontology
- [MONDO](https://github.com/monarch-initiative/mondo) - Disease Ontology

### 8. Fuzz Testing (5 targets)

Fuzz testing uses [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) with libFuzzer to find crashes and panics in validators and parsers. This is critical for scientific software that must handle arbitrary user input gracefully.

```
crates/crucible/fuzz/
├── Cargo.toml
├── README.md
└── fuzz_targets/
    ├── fuzz_taxonomy.rs    # Taxonomy validator fuzzing
    ├── fuzz_accession.rs   # Accession validator fuzzing
    ├── fuzz_ontology.rs    # Ontology mapper fuzzing
    ├── fuzz_parser.rs      # CSV/TSV parser fuzzing
    └── fuzz_date.rs        # Date detection/type inference fuzzing
```

**Fuzz targets and coverage:**

| Target | Component | Tests |
|--------|-----------|-------|
| `fuzz_taxonomy` | TaxonomyValidator | UTF-8 handling, abbreviation expansion |
| `fuzz_accession` | AccessionValidator | Pattern matching, URL generation |
| `fuzz_ontology` | OntologyValidator | ID lookup, suggestion algorithms |
| `fuzz_parser` | Parser | Delimiter detection, malformed files |
| `fuzz_date` | Type inference | Date regex, analysis pipeline |

**Prerequisites:**
```bash
# Requires nightly Rust via rustup
rustup install nightly
cargo install cargo-fuzz
```

**Run fuzz tests:**
```bash
cd crates/crucible

# List available targets
cargo +nightly fuzz list

# Run a specific target (press Ctrl+C to stop)
cargo +nightly fuzz run fuzz_taxonomy

# Run with time limit (60 seconds)
cargo +nightly fuzz run fuzz_taxonomy -- -max_total_time=60
```

**Properties verified:**
- Validators never panic on any UTF-8 input
- Validators never panic on lossy UTF-8 conversion
- Parser handles malformed files gracefully
- No unbounded memory allocation on large inputs

---

## Bioinformatics Validation

### MIxS Standard Compliance

Crucible validates metadata against the [Minimum Information about any (x) Sequence](https://github.com/GenomicsStandardsConsortium/mixs) (MIxS) standard.

**Tested components:**
- Core mandatory fields (8 fields)
- Environmental packages (15 packages)
- Field format validation (dates, coordinates, ENVO terms)

**Test coverage:**
```
test bio::mixs::tests::test_field_matching
test bio::mixs::tests::test_find_field_with_alias
test bio::mixs::tests::test_package_from_string
test bio::mixs::tests::test_schema_core_fields
test bio::mixs::tests::test_schema_package_fields
```

### NCBI Taxonomy Validation

Validates organism names against NCBI Taxonomy database.

**Tested scenarios:**
| Input | Issue | Expected Output |
|-------|-------|-----------------|
| `E. coli` | Abbreviation | `Escherichia coli` (taxid:562) |
| `Bacteroides fragalis` | Typo | `Bacteroides fragilis` (taxid:817) |
| `Homo Sapiens` | Case error | `Homo sapiens` (taxid:9606) |
| `human` | Common name | `Homo sapiens` (taxid:9606) |

**Test coverage:**
```
test bio::taxonomy::tests::test_validate_abbreviation
test bio::taxonomy::tests::test_expand_abbreviation
test bio::taxonomy::tests::test_validate_valid
test bio::taxonomy::tests::test_validate_case_error
test bio::taxonomy::tests::test_lookup_organism
test bio::taxonomy::tests::test_levenshtein
```

### Ontology Term Mapping

Maps free-text terms to standard biological ontologies.

**Supported ontologies:**
| Ontology | Terms | Use Case |
|----------|-------|----------|
| ENVO | ~40 | Environmental terms |
| UBERON | ~50 | Anatomical terms |
| MONDO | ~55 | Disease terms |

**Test coverage:**
```
test bio::ontology::tests::test_lookup_by_id
test bio::ontology::tests::test_lookup_by_label
test bio::ontology::tests::test_lookup_by_synonym
test bio::ontology::tests::test_suggest_mappings
test bio::ontology::tests::test_validate_id
test bio::ontology::tests::test_filter_by_ontology
```

### Database Accession Validation

Validates accession formats for major biological databases.

**Supported formats:**
| Database | Format | Example |
|----------|--------|---------|
| BioSample | SAMN*/SAME*/SAMD* | SAMN12345678 |
| SRA Run | SRR*/ERR*/DRR* | SRR1234567 |
| BioProject | PRJNA*/PRJEB*/PRJDB* | PRJNA123456 |
| GenBank | Various | NM_001234567 |
| RefSeq | NM_*/NC_*/XM_* | NC_000001.11 |
| UniProt | P12345/A0A0A0ABC1 | P53_HUMAN |
| PDB | 4-char | 6LU7 |

**Test coverage:**
```
test bio::accession::tests::test_biosample_validation
test bio::accession::tests::test_sra_run_validation
test bio::accession::tests::test_genbank_validation
test bio::accession::tests::test_refseq_validation
test bio::accession::tests::test_uniprot_validation
test bio::accession::tests::test_pdb_validation
test bio::accession::tests::test_column_detection
test bio::accession::tests::test_url_generation
```

### NCBI BioSample Pre-validation

Catches common NCBI submission errors before submission.

**Validated rules:**
- Sample uniqueness (attributes must differ)
- Date formats (ISO 8601 required)
- Coordinate formats (decimal degrees)
- Null value usage (NA vs NCBI-accepted values)
- Organism/package compatibility

**Test coverage:**
```
test bio::biosample::tests::test_sample_uniqueness
test bio::biosample::tests::test_date_format_validation
test bio::biosample::tests::test_coordinate_validation
test bio::biosample::tests::test_null_value_validation
test bio::biosample::tests::test_organism_validation
test bio::biosample::tests::test_readiness_score
```

---

## Determinism Verification

Crucible must produce consistent results across runs. We verify:

1. **Schema stability** - Column types and roles remain constant
2. **Observation consistency** - Same issues detected in same order
3. **Suggestion reproducibility** - Same fixes suggested

**Determinism tests:**
```
test test_analysis_structure_is_deterministic
test test_observation_content_is_stable
```

---

## Running the Full Test Suite

```bash
# Run all tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test module
cargo test -p crucible bio::

# Run golden tests only
cargo test -p crucible --test golden_tests

# Run property tests only
cargo test -p crucible --test property_tests

# Run property tests with more cases (thorough)
PROPTEST_CASES=10000 cargo test -p crucible --test property_tests

# Run real-world validation tests
cargo test -p crucible --test real_world_validation_test

# Run ontology accuracy tests
cargo test -p crucible --test ontology_accuracy_test

# Run fuzz tests (requires nightly)
cd crates/crucible && cargo +nightly fuzz run fuzz_taxonomy -- -max_total_time=60

# Check test coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

---

## Test Data

### Reference Datasets

| Dataset | Rows | Purpose |
|---------|------|---------|
| `ibd_cohort_metadata.tsv` | 8 | IBD study with case/date issues |
| `microbiome_study_metadata.tsv` | 20 | Microbiome study with outliers |
| `sample_accessions.tsv` | 4 | Accession validation testing |

### Golden Test Cases

| Test Case | Issues | Observations | Suggestions |
|-----------|--------|--------------|-------------|
| case_consistency | Case variants | 9 | 9 |
| date_formats | Mixed dates | 3 | 3 |
| outlier_detection | Impossible values | 9 | 9 |
| taxonomy_validation | Abbreviations, typos | 1+ | 1+ |
| accession_formats | Invalid accessions | 3 | 1 |
| mixs_compliance | Missing fields | 20 | 3 |

---

## Adding New Tests

### Adding a Golden Test

1. Create test directory:
```bash
mkdir test_data/golden/my_test_case
```

2. Add input file with known issues:
```bash
cat > test_data/golden/my_test_case/input.tsv << 'EOF'
sample_id	value
S001	correct
S002	WRONG_CASE
EOF
```

3. Generate expected output:
```bash
cargo run -- analyze test_data/golden/my_test_case/input.tsv
mv test_data/golden/my_test_case/input.curation.json \
   test_data/golden/my_test_case/expected.json
```

4. Add manifest:
```json
{
  "name": "my_test_case",
  "description": "Tests XYZ detection",
  "expected_observations": {
    "min_count": 1,
    "types": ["Inconsistency"],
    "columns_with_issues": ["value"]
  },
  "expected_suggestions": {
    "min_count": 1,
    "actions": ["Standardize"]
  }
}
```

5. Add test macro to `golden_tests.rs`:
```rust
golden_test!(test_golden_my_test_case, "my_test_case");
```

### Adding a Regression Test

```rust
/// Issue #123: Description of the bug
#[test]
fn regression_issue_123_description() {
    // Minimal reproduction case
    let content = "sample_id\tvalue\nS001\tbad_value\n";
    // ... test that the bug is fixed
}
```

---

## Continuous Integration

Tests run automatically on:
- Every push to `main`
- Every pull request
- Nightly builds (full test suite + coverage)

### CI Configuration

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo test --all-features
```

---

## Quality Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Unit tests | 104 | 150+ |
| Integration tests | 32 | 50+ |
| Golden tests | 7 | 15+ |
| Property tests | 32 | 50+ |
| Real-world validation | 14 | 25+ |
| Ontology accuracy | 18 | 30+ |
| Fuzz targets | 5 | 10+ |
| Doc tests | 9 | 20+ |
| **Total tests** | **246** | **400+** |
| Code coverage | ~75% | 85%+ |
| Bio module coverage | ~90% | 95%+ |

---

## Validation Against Standards

### NCBI Requirements

Crucible's validators are designed to catch errors documented in:
- [NCBI BioSample Validation Errors](https://www.ncbi.nlm.nih.gov/biosample/docs/submission/validation-errors/)
- [SRA Submission Guidelines](https://www.ncbi.nlm.nih.gov/sra/docs/submit/)

### MIxS Standard

Validated against:
- [GSC MIxS Repository](https://github.com/GenomicsStandardsConsortium/mixs)
- [MIxS 6.0 Specification](https://genomicsstandardsconsortium.github.io/mixs/)

### Ontology Sources

Terms validated against:
- [ENVO](https://github.com/EnvironmentOntology/envo)
- [UBERON](https://github.com/obophenotype/uberon)
- [MONDO](https://github.com/monarch-initiative/mondo)

---

## Reporting Issues

If you find a validation issue:

1. Create minimal reproduction case
2. Document expected vs actual behavior
3. Reference relevant standard if applicable
4. Submit issue with `[validation]` tag

---

## Version History

| Version | Tests | Coverage | Notes |
|---------|-------|----------|-------|
| 0.1.0 | 143 | ~75% | Initial release with bio module |
| 0.1.1 | 214 | ~78% | Added property-based tests, golden tests |
| 0.1.2 | 228 | ~80% | Added real-world validation tests |
| 0.1.3 | 246 | ~82% | Added ontology accuracy tests, fixed duplicate MONDO IDs |
| 0.1.4 | 246 | ~82% | Added fuzz testing infrastructure (5 targets) |

---

*Last updated: 2026-01-04*
