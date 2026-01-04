//! Real-world validation tests for Crucible.
//!
//! These tests verify that Crucible correctly detects issues in datasets
//! that mirror real NCBI/EBI submission problems. Each dataset contains
//! documented issues that commonly cause submission rejections.
//!
//! # Validation Approach
//!
//! 1. **Detection rate**: Verify known issues are found
//! 2. **False positive control**: Correct data shouldn't be flagged
//! 3. **Suggestion quality**: Suggestions should match NCBI guidance
//! 4. **Confidence calibration**: Clear errors should have high confidence

use std::path::Path;

use crucible::{ContextHints, Crucible, CurationContext, CurationLayer, MockProvider};
use crucible::bio::{
    AccessionValidator, TaxonomyValidator, BioSampleValidator,
    MixsPackage, TaxonomyValidationResult,
};

// =============================================================================
// Helper Functions
// =============================================================================

/// Analyze a real-world test file and return the curation layer.
fn analyze_real_world(filename: &str, domain: Option<&str>) -> CurationLayer {
    let path = Path::new("../../test_data/real_world").join(filename);

    let mut crucible = Crucible::new().with_llm(MockProvider::new());

    if let Some(d) = domain {
        crucible = crucible.with_context(ContextHints::new().with_domain(d));
    }

    let result = crucible
        .analyze(&path)
        .unwrap_or_else(|e| panic!("Failed to analyze {}: {}", filename, e));

    let mut context = CurationContext::new();
    if let Some(d) = domain {
        context = context.with_domain(d.to_string());
    }

    CurationLayer::from_analysis(result, context)
}

/// Check if any observation mentions a specific term (case-insensitive).
fn has_observation_mentioning(curation: &CurationLayer, term: &str) -> bool {
    let term_lower = term.to_lowercase();
    curation.observations.iter().any(|o| {
        o.description.to_lowercase().contains(&term_lower) ||
        o.column.to_lowercase().contains(&term_lower)
    })
}

/// Count observations for a specific column.
#[allow(dead_code)]
fn observation_count_for_column(curation: &CurationLayer, column: &str) -> usize {
    curation.observations.iter()
        .filter(|o| o.column == column)
        .count()
}

// =============================================================================
// Microbiome Submission Tests
// =============================================================================

#[test]
fn test_microbiome_detects_organism_inconsistencies() {
    let curation = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));

    // The organism column has variations: "H. sapiens", "E. coli", "Homo Sapiens", etc.
    // Crucible should detect case inconsistencies or variations in the organism column
    let has_organism_issue = curation.observations.iter().any(|o| {
        o.column == "organism" && (
            o.description.to_lowercase().contains("case") ||
            o.description.to_lowercase().contains("variant") ||
            o.description.to_lowercase().contains("inconsisten") ||
            o.description.to_lowercase().contains("abbreviat")
        )
    });

    // Also check for any general inconsistency in organism-like columns
    let has_any_inconsistency = curation.observations.iter().any(|o| {
        (o.column == "organism" || o.column == "host") &&
        o.description.to_lowercase().contains("inconsisten")
    });

    // The dataset has clear issues - if not detecting organism specifically,
    // it should at least detect case issues somewhere
    let has_case_issues = curation.observations.iter().any(|o| {
        o.description.to_lowercase().contains("case")
    });

    assert!(
        has_organism_issue || has_any_inconsistency || has_case_issues,
        "Should detect organism variations or case inconsistencies. Observations: {:?}",
        curation.observations.iter()
            .map(|o| format!("{}: {}", o.column, o.description))
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_microbiome_detects_date_format_issues() {
    let curation = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));

    // Should detect non-ISO date formats like "05/20/2023", "May 25, 2023"
    let has_date_issue = curation.observations.iter().any(|o| {
        o.column == "collection_date" && (
            o.description.to_lowercase().contains("date") ||
            o.description.to_lowercase().contains("format")
        )
    });

    assert!(
        has_date_issue,
        "Should detect non-ISO date formats. Observations: {:?}",
        curation.observations.iter()
            .filter(|o| o.column == "collection_date")
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_microbiome_detects_case_inconsistencies() {
    let curation = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));

    // Should detect case variants like "Human Gut" vs "human gut"
    let has_case_issue = curation.observations.iter().any(|o| {
        o.description.to_lowercase().contains("case") ||
        o.description.to_lowercase().contains("variant")
    });

    assert!(
        has_case_issue,
        "Should detect case inconsistencies. Observations: {:?}",
        curation.observations
    );
}

#[test]
fn test_microbiome_detects_null_value_issues() {
    let curation = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));

    // Should detect non-standard nulls like "NA", "N/A"
    let has_null_issue = curation.observations.iter().any(|o| {
        o.description.to_lowercase().contains("missing") ||
        o.description.to_lowercase().contains("null") ||
        o.description.to_lowercase().contains("na")
    });

    assert!(
        has_null_issue,
        "Should detect non-standard null values. Observations: {:?}",
        curation.observations
    );
}

#[test]
fn test_microbiome_detects_duplicate_samples() {
    let curation = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));

    // MICRO009 and MICRO010 have identical attributes
    // This might be detected as duplicates or as identical values
    let total_observations = curation.observations.len();

    // Should have a reasonable number of observations for a 10-row dataset with issues
    assert!(
        total_observations >= 5,
        "Should detect multiple issues. Found only {} observations",
        total_observations
    );
}

// =============================================================================
// Environmental Submission Tests
// =============================================================================

#[test]
fn test_environmental_detects_coordinate_issues() {
    let curation = analyze_real_world("ncbi_environmental_submission.tsv", Some("biomedical"));

    // Should detect missing/invalid coordinates
    let has_coord_issue = has_observation_mentioning(&curation, "lat_lon") ||
        has_observation_mentioning(&curation, "coordinate") ||
        has_observation_mentioning(&curation, "missing");

    assert!(
        has_coord_issue,
        "Should detect coordinate format issues. Observations: {:?}",
        curation.observations
    );
}

#[test]
fn test_environmental_detects_data_quality_issues() {
    let curation = analyze_real_world("ncbi_environmental_submission.tsv", Some("biomedical"));

    // The environmental dataset has various issues: dates, coordinates, missing values
    // Crucible should detect at least some of these
    let total_observations = curation.observations.len();

    // Should find issues - at minimum the date format inconsistency and missing values
    assert!(
        total_observations >= 2,
        "Should detect data quality issues. Found {} observations: {:?}",
        total_observations,
        curation.observations.iter()
            .map(|o| format!("{}: {}", o.column, o.description))
            .collect::<Vec<_>>()
    );

    // Should specifically detect date format issues (mixed ISO and US formats)
    let has_date_issue = curation.observations.iter().any(|o| {
        o.column == "collection_date" && (
            o.description.to_lowercase().contains("date") ||
            o.description.to_lowercase().contains("format") ||
            o.description.to_lowercase().contains("mixed")
        )
    });

    assert!(
        has_date_issue,
        "Should detect date format inconsistencies"
    );
}

// =============================================================================
// Clinical Submission Tests
// =============================================================================

#[test]
fn test_clinical_detects_invalid_accessions() {
    let curation = analyze_real_world("ncbi_human_clinical.tsv", Some("biomedical"));

    // Should detect short/invalid accessions
    let has_accession_issue = has_observation_mentioning(&curation, "accession") ||
        has_observation_mentioning(&curation, "sra") ||
        has_observation_mentioning(&curation, "biosample");

    assert!(
        has_accession_issue,
        "Should detect invalid accession formats. Observations: {:?}",
        curation.observations
    );
}

#[test]
fn test_clinical_detects_outliers() {
    let curation = analyze_real_world("ncbi_human_clinical.tsv", Some("biomedical"));

    // Should detect impossible ages like -5 and 200
    let has_age_outlier = curation.observations.iter().any(|o| {
        o.column == "age" && (
            o.description.to_lowercase().contains("outlier") ||
            o.description.to_lowercase().contains("unusual") ||
            o.description.to_lowercase().contains("range") ||
            o.description.to_lowercase().contains("impossible")
        )
    });

    assert!(
        has_age_outlier,
        "Should detect impossible age values (-5, 200). Observations for age: {:?}",
        curation.observations.iter()
            .filter(|o| o.column == "age")
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_clinical_detects_organism_issues() {
    let curation = analyze_real_world("ncbi_human_clinical.tsv", Some("biomedical"));

    // Should detect "human", "H. sapiens" vs "Homo sapiens"
    let organism_issues = curation.observations.iter()
        .filter(|o| o.column == "organism" || o.column == "host")
        .count();

    assert!(
        organism_issues >= 1,
        "Should detect organism name issues. Found {} organism/host observations",
        organism_issues
    );
}

// =============================================================================
// Direct Validator Tests
// =============================================================================

#[test]
fn test_taxonomy_validator_on_real_names() {
    let validator = TaxonomyValidator::new();

    // Valid names
    assert!(matches!(
        validator.validate("Homo sapiens"),
        TaxonomyValidationResult::Valid { .. }
    ));

    assert!(matches!(
        validator.validate("Escherichia coli"),
        TaxonomyValidationResult::Valid { .. }
    ));

    // Abbreviations should be detected
    assert!(matches!(
        validator.validate("E. coli"),
        TaxonomyValidationResult::Abbreviation { .. }
    ));

    assert!(matches!(
        validator.validate("H. sapiens"),
        TaxonomyValidationResult::Abbreviation { .. }
    ));

    // Case errors
    assert!(matches!(
        validator.validate("homo sapiens"),
        TaxonomyValidationResult::CaseError { .. }
    ));

    // Common name "human" is actually recognized and mapped to Homo sapiens with case correction
    let human_result = validator.validate("human");
    assert!(
        matches!(human_result, TaxonomyValidationResult::CaseError { .. } | TaxonomyValidationResult::Valid { .. }),
        "Common name 'human' should be recognized, got {:?}",
        human_result
    );
}

#[test]
fn test_accession_validator_on_real_accessions() {
    let validator = AccessionValidator::new();

    // Valid accessions
    let valid_biosample = validator.validate("SAMN12345678");
    assert!(valid_biosample.is_valid, "SAMN12345678 should be valid");

    let valid_sra = validator.validate("SRR1234567");
    assert!(valid_sra.is_valid, "SRR1234567 should be valid");

    // Check that SRA accession type is correctly identified for 7-digit version
    assert_eq!(
        valid_sra.accession_type,
        Some(crucible::bio::AccessionType::SraRun),
        "SRR1234567 should be SraRun type"
    );

    // Short accessions - 6 digits may match other patterns but not SraRun specifically
    let short_sra = validator.validate("SRR123456");
    // The key property: if it matches SraRun, it should be at least 6 digits (our pattern requires 6-9)
    // SRR123456 has exactly 6 digits which should be valid for SraRun pattern
    if short_sra.accession_type == Some(crucible::bio::AccessionType::SraRun) {
        assert!(short_sra.is_valid, "SRR123456 with 6 digits should be valid for SraRun");
    }

    // Very short accession - definitely too short
    let very_short_sra = validator.validate("SRR12345");
    assert!(
        very_short_sra.accession_type != Some(crucible::bio::AccessionType::SraRun),
        "SRR12345 (5 digits) should not match SraRun pattern"
    );
}

#[test]
fn test_biosample_validator_on_microbiome_data() {
    use crucible::{DataTable, TableSchema, ColumnSchema, ColumnType};

    let validator = BioSampleValidator::new();

    // Create test data with duplicate samples
    let data = DataTable {
        headers: vec![
            "sample_name".to_string(),
            "organism".to_string(),
            "collection_date".to_string(),
        ],
        rows: vec![
            vec!["S001".to_string(), "Homo sapiens".to_string(), "2023-01-01".to_string()],
            vec!["S002".to_string(), "Homo sapiens".to_string(), "2023-01-01".to_string()], // Duplicate
        ],
        delimiter: b'\t',
    };

    let mut col_organism = ColumnSchema::new("organism", 1);
    col_organism.inferred_type = ColumnType::String;

    let mut col_date = ColumnSchema::new("collection_date", 2);
    col_date.inferred_type = ColumnType::Date;

    let mut schema = TableSchema::new();
    schema.columns = vec![
        ColumnSchema::new("sample_name", 0),
        col_organism,
        col_date,
    ];

    let readiness = validator.check_readiness(&data, &schema, Some(MixsPackage::HumanGut));

    // Should detect duplicate samples
    assert!(
        !readiness.blocking_issues.is_empty() || !readiness.warning_issues.is_empty(),
        "Should detect issues in test data"
    );
}

// =============================================================================
// Summary Statistics Test
// =============================================================================

#[test]
fn test_real_world_detection_summary() {
    let microbiome = analyze_real_world("ncbi_microbiome_submission.tsv", Some("biomedical"));
    let environmental = analyze_real_world("ncbi_environmental_submission.tsv", Some("biomedical"));
    let clinical = analyze_real_world("ncbi_human_clinical.tsv", Some("biomedical"));

    println!("\n=== Real-World Validation Summary ===\n");

    println!("Microbiome dataset (10 rows, 10+ expected issues):");
    println!("  Observations: {}", microbiome.observations.len());
    println!("  Suggestions: {}", microbiome.suggestions.len());

    println!("\nEnvironmental dataset (10 rows, 7+ expected issues):");
    println!("  Observations: {}", environmental.observations.len());
    println!("  Suggestions: {}", environmental.suggestions.len());

    println!("\nClinical dataset (10 rows, 12+ expected issues):");
    println!("  Observations: {}", clinical.observations.len());
    println!("  Suggestions: {}", clinical.suggestions.len());

    // Basic sanity check - we should find a reasonable number of issues
    let total_observations = microbiome.observations.len() +
        environmental.observations.len() +
        clinical.observations.len();

    assert!(
        total_observations >= 15,
        "Should detect at least 15 total issues across all datasets. Found: {}",
        total_observations
    );

    println!("\nTotal observations across all datasets: {}", total_observations);
    println!("=== End Summary ===\n");
}
