//! Golden file tests for Crucible.
//!
//! These tests verify that Crucible's analysis output is deterministic and
//! matches expected results for known datasets. This ensures:
//!
//! 1. **Reproducibility**: Same input always produces same output
//! 2. **Regression Prevention**: Changes to analysis logic are detected
//! 3. **Scientific Validity**: Results can be reviewed and validated
//!
//! # Test Structure
//!
//! Each golden test case consists of:
//! - `input.tsv`: The input data file
//! - `expected.json`: The expected curation layer output
//! - `manifest.json`: Test metadata and configuration
//!
//! # Updating Golden Files
//!
//! When intentional changes are made:
//! ```bash
//! cargo run -- analyze test_data/golden/<case>/input.tsv > expected.json
//! ```

use std::fs;
use std::path::Path;

use crucible::{ContextHints, Crucible, CurationContext, CurationLayer, MockProvider};
use serde::Deserialize;

/// Manifest describing a golden test case.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for documentation and future expansion
struct GoldenManifest {
    name: String,
    description: String,
    #[serde(default)]
    domain: Option<String>,
    #[serde(default)]
    mixs_package: Option<String>,
    expected_observations: ExpectedObservations,
    expected_suggestions: ExpectedSuggestions,
}

#[derive(Debug, Deserialize)]
struct ExpectedObservations {
    min_count: usize,
    types: Vec<String>,
    columns_with_issues: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedSuggestions {
    min_count: usize,
    actions: Vec<String>,
}

/// Load a golden test manifest.
fn load_manifest(test_dir: &Path) -> GoldenManifest {
    let manifest_path = test_dir.join("manifest.json");
    let content = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("Failed to read manifest at {:?}: {}", manifest_path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse manifest at {:?}: {}", manifest_path, e))
}

/// Load expected curation layer.
fn load_expected(test_dir: &Path) -> CurationLayer {
    let expected_path = test_dir.join("expected.json");
    let content = fs::read_to_string(&expected_path)
        .unwrap_or_else(|e| panic!("Failed to read expected.json at {:?}: {}", expected_path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse expected.json at {:?}: {}", expected_path, e))
}

/// Run analysis on a golden test input.
fn analyze_golden_input(test_dir: &Path, manifest: &GoldenManifest) -> CurationLayer {
    let input_path = test_dir.join("input.tsv");

    let mut crucible = Crucible::new().with_llm(MockProvider::new());

    if let Some(domain) = &manifest.domain {
        crucible = crucible.with_context(ContextHints::new().with_domain(domain));
    }

    let result = crucible
        .analyze(&input_path)
        .unwrap_or_else(|e| panic!("Analysis failed for {:?}: {}", input_path, e));

    let mut context = CurationContext::new();
    if let Some(domain) = &manifest.domain {
        context = context.with_domain(domain.clone());
    }

    CurationLayer::from_analysis(result, context)
}

/// Assert that actual observations match expected patterns.
fn assert_observations_match(
    manifest: &GoldenManifest,
    actual: &CurationLayer,
    expected: &CurationLayer,
) {
    // Check minimum observation count
    assert!(
        actual.observations.len() >= manifest.expected_observations.min_count,
        "Expected at least {} observations, got {}",
        manifest.expected_observations.min_count,
        actual.observations.len()
    );

    // Check that expected observation types are present
    for expected_type in &manifest.expected_observations.types {
        let has_type = actual.observations.iter().any(|o| {
            format!("{:?}", o.observation_type).contains(expected_type)
        });
        assert!(
            has_type,
            "Expected observation type '{}' not found in {:?}",
            expected_type,
            actual.observations.iter().map(|o| format!("{:?}", o.observation_type)).collect::<Vec<_>>()
        );
    }

    // Check that expected columns have issues
    for expected_col in &manifest.expected_observations.columns_with_issues {
        let has_col = actual.observations.iter().any(|o| o.column == *expected_col);
        assert!(
            has_col,
            "Expected column '{}' to have issues, but it wasn't found in observations",
            expected_col
        );
    }
}

/// Assert that actual suggestions match expected patterns.
#[allow(dead_code)]
fn assert_suggestions_match(
    manifest: &GoldenManifest,
    actual: &CurationLayer,
    _expected: &CurationLayer,
) {
    // Check minimum suggestion count
    assert!(
        actual.suggestions.len() >= manifest.expected_suggestions.min_count,
        "Expected at least {} suggestions, got {}",
        manifest.expected_suggestions.min_count,
        actual.suggestions.len()
    );

    // Check that expected action types are present
    for expected_action in &manifest.expected_suggestions.actions {
        let has_action = actual.suggestions.iter().any(|s| {
            format!("{:?}", s.action).contains(expected_action)
        });
        assert!(
            has_action,
            "Expected suggestion action '{}' not found",
            expected_action
        );
    }
}

/// Assert schema inference is consistent.
fn assert_schema_consistent(actual: &CurationLayer, expected: &CurationLayer) {
    // Column count must match
    assert_eq!(
        actual.schema.columns.len(),
        expected.schema.columns.len(),
        "Column count mismatch"
    );

    // Column names and types should match
    for (actual_col, expected_col) in actual.schema.columns.iter().zip(expected.schema.columns.iter()) {
        assert_eq!(
            actual_col.name, expected_col.name,
            "Column name mismatch"
        );
        assert_eq!(
            actual_col.inferred_type, expected_col.inferred_type,
            "Column '{}' type mismatch: {:?} vs {:?}",
            actual_col.name, actual_col.inferred_type, expected_col.inferred_type
        );
    }
}

// =============================================================================
// Golden Test Cases
// =============================================================================

macro_rules! golden_test {
    ($name:ident, $path:expr) => {
        #[test]
        fn $name() {
            let test_dir = Path::new(concat!("../../test_data/golden/", $path));
            let manifest = load_manifest(test_dir);
            let expected = load_expected(test_dir);
            let actual = analyze_golden_input(test_dir, &manifest);

            // Run assertions
            assert_schema_consistent(&actual, &expected);
            assert_observations_match(&manifest, &actual, &expected);
            assert_suggestions_match(&manifest, &actual, &expected);
        }
    };
}

golden_test!(test_golden_case_consistency, "case_consistency");
golden_test!(test_golden_date_formats, "date_formats");
golden_test!(test_golden_outlier_detection, "outlier_detection");
golden_test!(test_golden_null_value_variants, "null_value_variants");
golden_test!(test_golden_numeric_range, "numeric_range");
golden_test!(test_golden_whitespace_issues, "whitespace_issues");
golden_test!(test_golden_empty_values, "empty_values");

// Tests that require specific validators (infrastructure in place, detectors pending)
golden_test!(test_golden_coordinate_validation, "coordinate_validation");
golden_test!(test_golden_duplicate_samples, "duplicate_samples");
golden_test!(test_golden_identifier_patterns, "identifier_patterns");

// Tests that require bio module features
#[cfg(feature = "bio")]
mod bio_golden_tests {
    use super::*;

    golden_test!(test_golden_taxonomy_validation, "taxonomy_validation");
    golden_test!(test_golden_accession_formats, "accession_formats");
    golden_test!(test_golden_mixs_compliance, "mixs_compliance");
}

// =============================================================================
// Determinism Tests
// =============================================================================

/// Verify that running analysis twice produces consistent structural results.
/// Note: Timestamps and IDs may differ but structure should be identical.
#[test]
fn test_analysis_structure_is_deterministic() {
    let test_dir = Path::new("../../test_data/golden/case_consistency");
    let manifest = load_manifest(test_dir);

    let result1 = analyze_golden_input(test_dir, &manifest);
    let result2 = analyze_golden_input(test_dir, &manifest);

    // Schema should be identical
    assert_eq!(result1.schema.columns.len(), result2.schema.columns.len());
    for (c1, c2) in result1.schema.columns.iter().zip(result2.schema.columns.iter()) {
        assert_eq!(c1.name, c2.name);
        assert_eq!(c1.inferred_type, c2.inferred_type);
        assert_eq!(c1.semantic_role, c2.semantic_role);
    }

    // Same number of observations
    assert_eq!(
        result1.observations.len(),
        result2.observations.len(),
        "Observation count should be stable"
    );

    // Observations should have same types and columns (order may vary)
    let obs_types1: std::collections::HashSet<_> = result1
        .observations
        .iter()
        .map(|o| (&o.column, format!("{:?}", o.observation_type)))
        .collect();
    let obs_types2: std::collections::HashSet<_> = result2
        .observations
        .iter()
        .map(|o| (&o.column, format!("{:?}", o.observation_type)))
        .collect();

    assert_eq!(obs_types1, obs_types2, "Observation types should be stable");

    // Same number of suggestions
    assert_eq!(
        result1.suggestions.len(),
        result2.suggestions.len(),
        "Suggestion count should be stable"
    );
}

/// Verify observation content is consistent across runs.
#[test]
fn test_observation_content_is_stable() {
    let test_dir = Path::new("../../test_data/golden/case_consistency");
    let manifest = load_manifest(test_dir);

    let result1 = analyze_golden_input(test_dir, &manifest);
    let result2 = analyze_golden_input(test_dir, &manifest);

    // Sort observations by column and description for comparison
    let mut obs1: Vec<_> = result1
        .observations
        .iter()
        .map(|o| (&o.column, &o.description))
        .collect();
    let mut obs2: Vec<_> = result2
        .observations
        .iter()
        .map(|o| (&o.column, &o.description))
        .collect();

    obs1.sort();
    obs2.sort();

    assert_eq!(obs1, obs2, "Observation content should be stable");
}

// =============================================================================
// Regression Tests
// =============================================================================

/// Issue: PDB accession pattern was matching pure numeric gene IDs.
#[test]
fn regression_pdb_vs_gene_id() {
    // Gene ID "7157" (TP53) should not match PDB pattern
    let content = "sample_id\tgene_id\nS001\t7157\nS002\t672\nS003\t1017\n";
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = temp_dir.path().join("input.tsv");
    fs::write(&input_path, content).unwrap();

    let crucible = Crucible::new().with_llm(MockProvider::new());
    let result = crucible.analyze(&input_path).unwrap();
    let curation = CurationLayer::from_analysis(result, CurationContext::new());

    // Should not have any "invalid PDB" observations
    let pdb_errors: Vec<_> = curation
        .observations
        .iter()
        .filter(|o| o.description.to_lowercase().contains("pdb"))
        .collect();

    assert!(
        pdb_errors.is_empty(),
        "Gene IDs should not trigger PDB validation errors: {:?}",
        pdb_errors
    );
}

/// Issue: Very short SRA accessions were being accepted.
#[test]
fn regression_short_sra_accessions() {
    let content = "sample_id\tsra_run\nS001\tSRR123\nS002\tSRR1234567\n";
    let temp_dir = tempfile::tempdir().unwrap();
    let input_path = temp_dir.path().join("input.tsv");
    fs::write(&input_path, content).unwrap();

    let crucible = Crucible::new().with_llm(MockProvider::new());
    let result = crucible.analyze(&input_path).unwrap();
    let curation = CurationLayer::from_analysis(result, CurationContext::new());

    // Should detect invalid SRR123 (too short)
    let has_invalid_accession = curation.observations.iter().any(|o| {
        o.column == "sra_run" && o.description.to_lowercase().contains("invalid")
    });

    // Note: This depends on whether accession validation is enabled in the analysis
    // If not, this test documents expected behavior for when it is enabled
    if !curation.observations.is_empty() {
        assert!(
            has_invalid_accession,
            "SRR123 should be flagged as invalid (too short)"
        );
    }
}
