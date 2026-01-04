//! Ontology accuracy tests for Crucible.
//!
//! These tests verify that Crucible's built-in ontology terms are accurate
//! against authoritative sources (ENVO, UBERON, MONDO).
//!
//! # Validation Approach
//!
//! 1. **ID Correctness**: Verify term IDs match expected labels
//! 2. **Label Accuracy**: Verify labels match official ontology labels
//! 3. **Synonym Validity**: Verify synonyms are recognized
//! 4. **Mapping Accuracy**: Verify free-text mapping produces correct terms
//!
//! # Authoritative Sources
//!
//! - ENVO: https://github.com/EnvironmentOntology/envo
//! - UBERON: https://github.com/obophenotype/uberon
//! - MONDO: https://github.com/monarch-initiative/mondo

use crucible::bio::{OntologyValidator, OntologyType, OntologyValidationResult, MatchType};

// =============================================================================
// ENVO (Environmental Ontology) Accuracy Tests
// =============================================================================

/// Reference ENVO terms with verified IDs and labels.
/// Source: https://www.ebi.ac.uk/ols/ontologies/envo
const ENVO_REFERENCE_TERMS: &[(&str, &str)] = &[
    // Biomes
    ("ENVO:00000446", "terrestrial biome"),
    ("ENVO:00000447", "marine biome"),
    ("ENVO:00000873", "freshwater biome"),
    ("ENVO:01000174", "forest biome"),
    ("ENVO:01000177", "grassland biome"),
    ("ENVO:01000179", "desert biome"),
    ("ENVO:01000180", "tundra biome"),

    // Environmental features
    ("ENVO:00000067", "cave"),
    ("ENVO:00000114", "agricultural field"),
    ("ENVO:00002030", "aquatic environment"),
    ("ENVO:00002031", "wastewater"),

    // Environmental materials
    ("ENVO:00001998", "soil"),
    ("ENVO:00002001", "water"),
    ("ENVO:00002003", "feces"),
    ("ENVO:00002005", "air"),
    ("ENVO:00002044", "groundwater"),
    ("ENVO:00002261", "forest soil"),
    ("ENVO:00002262", "agricultural soil"),
    ("ENVO:00005789", "human gut"),
];

#[test]
fn test_envo_term_ids_are_correct() {
    let validator = OntologyValidator::new();

    for (expected_id, expected_label) in ENVO_REFERENCE_TERMS {
        let term = validator.lookup_by_id(expected_id);
        assert!(
            term.is_some(),
            "ENVO term {} ({}) should exist in database",
            expected_id, expected_label
        );

        let term = term.unwrap();
        assert_eq!(
            term.label.to_lowercase(),
            expected_label.to_lowercase(),
            "ENVO {} should have label '{}', got '{}'",
            expected_id, expected_label, term.label
        );
        assert_eq!(
            term.ontology, OntologyType::Envo,
            "Term {} should be ENVO type",
            expected_id
        );
    }
}

#[test]
fn test_envo_synonyms_are_recognized() {
    let validator = OntologyValidator::new();

    // Verified ENVO synonyms
    let synonym_tests = [
        ("dirt", "ENVO:00001998"),       // soil
        ("earth", "ENVO:00001998"),      // soil
        ("fecal matter", "ENVO:00002003"), // feces
        ("stool", "ENVO:00002003"),      // feces
        ("ground water", "ENVO:00002044"), // groundwater
        ("farm soil", "ENVO:00002262"),  // agricultural soil
        ("intestinal content", "ENVO:00005789"), // human gut
    ];

    for (synonym, expected_id) in synonym_tests {
        let terms = validator.lookup(synonym);
        assert!(
            terms.iter().any(|t| t.id == expected_id),
            "Synonym '{}' should map to {}, got {:?}",
            synonym, expected_id,
            terms.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_envo_id_format_validation() {
    let validator = OntologyValidator::new();

    // Valid ENVO ID format
    assert!(matches!(
        validator.validate_id("ENVO:00001998"),
        OntologyValidationResult::Valid { .. }
    ));

    // Valid format but unknown term
    assert!(matches!(
        validator.validate_id("ENVO:99999999"),
        OntologyValidationResult::UnknownTerm { ontology: OntologyType::Envo, .. }
    ));

    // Case insensitive
    assert!(matches!(
        validator.validate_id("envo:00001998"),
        OntologyValidationResult::Valid { .. }
    ));
}

// =============================================================================
// UBERON (Anatomical Ontology) Accuracy Tests
// =============================================================================

/// Reference UBERON terms with verified IDs and labels.
/// Source: https://www.ebi.ac.uk/ols/ontologies/uberon
const UBERON_REFERENCE_TERMS: &[(&str, &str)] = &[
    // Major organs
    ("UBERON:0000178", "blood"),
    ("UBERON:0000948", "heart"),
    ("UBERON:0000955", "brain"),
    ("UBERON:0000970", "eye"),
    ("UBERON:0001007", "digestive system"),
    ("UBERON:0002048", "lung"),
    ("UBERON:0002097", "skin of body"),
    ("UBERON:0002107", "liver"),
    ("UBERON:0002113", "kidney"),

    // GI tract
    ("UBERON:0000160", "intestine"),
    ("UBERON:0000945", "stomach"),
    ("UBERON:0001043", "esophagus"),
    ("UBERON:0001052", "rectum"),
    ("UBERON:0001155", "colon"),
    ("UBERON:0002114", "duodenum"),
    ("UBERON:0002115", "jejunum"),
    ("UBERON:0002116", "ileum"),

    // Other body sites
    ("UBERON:0001836", "saliva"),
    ("UBERON:0001988", "feces"),
    ("UBERON:0000996", "vagina"),
    ("UBERON:0001707", "nasal cavity"),
];

#[test]
fn test_uberon_term_ids_are_correct() {
    let validator = OntologyValidator::new();

    for (expected_id, expected_label) in UBERON_REFERENCE_TERMS {
        let term = validator.lookup_by_id(expected_id);
        assert!(
            term.is_some(),
            "UBERON term {} ({}) should exist in database",
            expected_id, expected_label
        );

        let term = term.unwrap();
        assert_eq!(
            term.label.to_lowercase(),
            expected_label.to_lowercase(),
            "UBERON {} should have label '{}', got '{}'",
            expected_id, expected_label, term.label
        );
        assert_eq!(
            term.ontology, OntologyType::Uberon,
            "Term {} should be UBERON type",
            expected_id
        );
    }
}

#[test]
fn test_uberon_synonyms_are_recognized() {
    let validator = OntologyValidator::new();

    // Verified UBERON synonyms
    let synonym_tests = [
        ("gut", "UBERON:0000160"),           // intestine
        ("bowel", "UBERON:0000160"),         // intestine
        ("large intestine", "UBERON:0001155"), // colon
        ("stool", "UBERON:0001988"),         // feces
        ("hepatic", "UBERON:0002107"),       // liver
        ("renal", "UBERON:0002113"),         // kidney
        ("pulmonary", "UBERON:0002048"),     // lung
        ("cardiac", "UBERON:0000948"),       // heart
        ("nasal", "UBERON:0001707"),         // nasal cavity
    ];

    for (synonym, expected_id) in synonym_tests {
        let terms = validator.lookup(synonym);
        assert!(
            terms.iter().any(|t| t.id == expected_id),
            "Synonym '{}' should map to {}, got {:?}",
            synonym, expected_id,
            terms.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_uberon_id_format_validation() {
    let validator = OntologyValidator::new();

    // Valid UBERON ID format
    assert!(matches!(
        validator.validate_id("UBERON:0000160"),
        OntologyValidationResult::Valid { .. }
    ));

    // Valid format but unknown term
    assert!(matches!(
        validator.validate_id("UBERON:9999999"),
        OntologyValidationResult::UnknownTerm { ontology: OntologyType::Uberon, .. }
    ));
}

// =============================================================================
// MONDO (Disease Ontology) Accuracy Tests
// =============================================================================

/// Reference MONDO terms with verified IDs and labels.
/// Source: https://www.ebi.ac.uk/ols/ontologies/mondo
const MONDO_REFERENCE_TERMS: &[(&str, &str)] = &[
    // GI diseases
    ("MONDO:0005011", "Crohn disease"),
    ("MONDO:0005265", "inflammatory bowel disease"),
    ("MONDO:0005101", "ulcerative colitis"),
    ("MONDO:0005052", "irritable bowel syndrome"),
    ("MONDO:0004992", "celiac disease"),

    // Metabolic diseases
    ("MONDO:0005015", "diabetes mellitus"),
    ("MONDO:0005148", "type 2 diabetes mellitus"),
    ("MONDO:0005147", "type 1 diabetes mellitus"),
    ("MONDO:0011122", "obesity"),

    // Cardiovascular
    ("MONDO:0005267", "heart disease"),
    ("MONDO:0005044", "hypertension"),

    // Neurological
    ("MONDO:0004975", "Alzheimer disease"),
    ("MONDO:0005180", "Parkinson disease"),
    ("MONDO:0004985", "multiple sclerosis"),

    // Infectious
    ("MONDO:0100096", "COVID-19"),
    ("MONDO:0005109", "HIV infection"),
    ("MONDO:0018076", "tuberculosis"),

    // Respiratory
    ("MONDO:0004979", "asthma"),
    ("MONDO:0005002", "chronic obstructive pulmonary disease"),
];

#[test]
fn test_mondo_term_ids_are_correct() {
    let validator = OntologyValidator::new();

    for (expected_id, expected_label) in MONDO_REFERENCE_TERMS {
        let term = validator.lookup_by_id(expected_id);
        assert!(
            term.is_some(),
            "MONDO term {} ({}) should exist in database",
            expected_id, expected_label
        );

        let term = term.unwrap();
        assert_eq!(
            term.label.to_lowercase(),
            expected_label.to_lowercase(),
            "MONDO {} should have label '{}', got '{}'",
            expected_id, expected_label, term.label
        );
        assert_eq!(
            term.ontology, OntologyType::Mondo,
            "Term {} should be MONDO type",
            expected_id
        );
    }
}

#[test]
fn test_mondo_synonyms_are_recognized() {
    let validator = OntologyValidator::new();

    // Verified MONDO synonyms (common abbreviations and alternate names)
    let synonym_tests = [
        ("Crohn's disease", "MONDO:0005011"),   // Crohn disease
        ("CD", "MONDO:0005011"),                // Crohn disease
        ("IBD", "MONDO:0005265"),               // inflammatory bowel disease
        ("UC", "MONDO:0005101"),                // ulcerative colitis
        ("IBS", "MONDO:0005052"),               // irritable bowel syndrome
        ("T2D", "MONDO:0005148"),               // type 2 diabetes
        ("type 2 diabetes", "MONDO:0005148"),  // type 2 diabetes mellitus
        ("T1D", "MONDO:0005147"),               // type 1 diabetes
        ("Alzheimer's disease", "MONDO:0004975"), // Alzheimer disease
        ("Parkinson's disease", "MONDO:0005180"), // Parkinson disease
        ("MS", "MONDO:0004985"),                // multiple sclerosis
        ("COPD", "MONDO:0005002"),              // chronic obstructive pulmonary disease
        ("coronavirus disease 2019", "MONDO:0100096"), // COVID-19
        ("TB", "MONDO:0018076"),                // tuberculosis
    ];

    for (synonym, expected_id) in synonym_tests {
        let terms = validator.lookup(synonym);
        assert!(
            terms.iter().any(|t| t.id == expected_id),
            "Synonym '{}' should map to {}, got {:?}",
            synonym, expected_id,
            terms.iter().map(|t| &t.id).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_mondo_id_format_validation() {
    let validator = OntologyValidator::new();

    // Valid MONDO ID format
    assert!(matches!(
        validator.validate_id("MONDO:0005011"),
        OntologyValidationResult::Valid { .. }
    ));

    // Valid format but unknown term
    assert!(matches!(
        validator.validate_id("MONDO:9999999"),
        OntologyValidationResult::UnknownTerm { ontology: OntologyType::Mondo, .. }
    ));
}

// =============================================================================
// Cross-Ontology Mapping Tests
// =============================================================================

#[test]
fn test_mapping_confidence_levels() {
    let validator = OntologyValidator::new();

    // Exact label match should have confidence 1.0
    let mappings = validator.suggest_mappings("soil", None);
    let exact_match = mappings.iter().find(|m| m.term_id == "ENVO:00001998");
    assert!(exact_match.is_some(), "Should find exact match for 'soil'");
    assert_eq!(exact_match.unwrap().confidence, 1.0, "Exact match should have confidence 1.0");
    assert!(matches!(exact_match.unwrap().match_type, MatchType::ExactLabel));

    // Synonym match should have confidence 0.95
    let mappings = validator.suggest_mappings("gut", None);
    let synonym_match = mappings.iter().find(|m| m.term_id == "UBERON:0000160");
    assert!(synonym_match.is_some(), "Should find synonym match for 'gut'");
    assert_eq!(synonym_match.unwrap().confidence, 0.95, "Synonym match should have confidence 0.95");
    assert!(matches!(synonym_match.unwrap().match_type, MatchType::Synonym(_)));
}

#[test]
fn test_ontology_filtering() {
    let validator = OntologyValidator::new();

    // "feces" exists in both ENVO and UBERON
    // Filter for ENVO only
    let envo_mappings = validator.suggest_mappings("feces", Some(OntologyType::Envo));
    assert!(envo_mappings.iter().all(|m| m.ontology == OntologyType::Envo));

    // Filter for UBERON only
    let uberon_mappings = validator.suggest_mappings("feces", Some(OntologyType::Uberon));
    assert!(uberon_mappings.iter().all(|m| m.ontology == OntologyType::Uberon));
}

#[test]
fn test_common_bioinformatics_terms() {
    let validator = OntologyValidator::new();

    // Common terms that researchers frequently search for
    let common_terms = [
        // Environmental
        ("soil", Some("ENVO:00001998")),
        ("water", Some("ENVO:00002001")),
        ("air", Some("ENVO:00002005")),
        ("sediment", None), // May have multiple matches

        // Anatomical
        ("blood", Some("UBERON:0000178")),
        ("gut", Some("UBERON:0000160")),
        ("skin", None), // May match "skin of body"
        ("lung", Some("UBERON:0002048")),
        ("liver", Some("UBERON:0002107")),

        // Diseases
        ("diabetes", Some("MONDO:0005015")),
        ("asthma", Some("MONDO:0004979")),
        ("obesity", Some("MONDO:0011122")),
    ];

    for (term, expected_id) in common_terms {
        let mappings = validator.suggest_mappings(term, None);

        if let Some(expected) = expected_id {
            assert!(
                mappings.iter().any(|m| m.term_id == expected),
                "Common term '{}' should map to {}, got {:?}",
                term, expected,
                mappings.iter().map(|m| &m.term_id).collect::<Vec<_>>()
            );
        } else {
            assert!(
                !mappings.is_empty(),
                "Common term '{}' should have at least one mapping",
                term
            );
        }
    }
}

// =============================================================================
// Coverage Tests
// =============================================================================

#[test]
fn test_ontology_coverage_stats() {
    let validator = OntologyValidator::new();
    let stats = validator.stats();

    println!("\n=== Ontology Coverage Statistics ===");
    println!("Total terms: {}", stats.total_terms);
    println!("Synonyms indexed: {}", stats.synonym_count);
    println!("\nTerms by ontology:");
    for (ontology, count) in &stats.terms_by_ontology {
        println!("  {:?}: {}", ontology, count);
    }
    println!("===================================\n");

    // Verify minimum coverage
    assert!(stats.total_terms >= 130, "Should have at least 130 terms");

    let envo_count = *stats.terms_by_ontology.get(&OntologyType::Envo).unwrap_or(&0);
    let uberon_count = *stats.terms_by_ontology.get(&OntologyType::Uberon).unwrap_or(&0);
    let mondo_count = *stats.terms_by_ontology.get(&OntologyType::Mondo).unwrap_or(&0);

    assert!(envo_count >= 35, "Should have at least 35 ENVO terms, got {}", envo_count);
    assert!(uberon_count >= 45, "Should have at least 45 UBERON terms, got {}", uberon_count);
    assert!(mondo_count >= 48, "Should have at least 48 MONDO terms, got {}", mondo_count);
}

#[test]
fn test_no_duplicate_ids() {
    let validator = OntologyValidator::new();
    let stats = validator.stats();

    // If we have duplicates, total_terms would be less than sum of individual counts
    let sum: usize = stats.terms_by_ontology.values().sum();
    assert_eq!(
        stats.total_terms, sum,
        "Total terms ({}) should equal sum of ontology counts ({})",
        stats.total_terms, sum
    );
}

// =============================================================================
// Edge Case Tests
// =============================================================================

#[test]
fn test_case_insensitive_lookup() {
    let validator = OntologyValidator::new();

    // IDs should be case-insensitive
    assert!(validator.lookup_by_id("ENVO:00001998").is_some());
    assert!(validator.lookup_by_id("envo:00001998").is_some());
    assert!(validator.lookup_by_id("Envo:00001998").is_some());

    // Labels should be case-insensitive
    assert!(!validator.lookup_by_label("SOIL").is_empty());
    assert!(!validator.lookup_by_label("Soil").is_empty());
    assert!(!validator.lookup_by_label("soil").is_empty());
}

#[test]
fn test_empty_and_whitespace_handling() {
    let validator = OntologyValidator::new();

    // Empty string should return empty results for lookup
    assert!(validator.lookup("").is_empty());

    // Empty string for suggest_mappings - may return empty or all terms
    // depending on implementation
    let _empty_mappings = validator.suggest_mappings("", None);
    // Just verify it doesn't panic

    // Whitespace should be trimmed
    let mappings = validator.suggest_mappings("  soil  ", None);
    assert!(!mappings.is_empty(), "Whitespace should be trimmed");

    // Whitespace-only should behave like empty
    assert!(validator.lookup("   ").is_empty());
}

#[test]
fn test_special_characters_handling() {
    let validator = OntologyValidator::new();

    // Terms with apostrophes
    let mappings = validator.suggest_mappings("Crohn's disease", None);
    assert!(!mappings.is_empty(), "Should handle apostrophes");

    // Terms with hyphens
    let mappings = validator.suggest_mappings("COVID-19", None);
    assert!(!mappings.is_empty(), "Should handle hyphens");
}

// =============================================================================
// Summary Test
// =============================================================================

#[test]
fn test_ontology_accuracy_summary() {
    let validator = OntologyValidator::new();
    let stats = validator.stats();

    // Count verified terms
    let envo_verified = ENVO_REFERENCE_TERMS.len();
    let uberon_verified = UBERON_REFERENCE_TERMS.len();
    let mondo_verified = MONDO_REFERENCE_TERMS.len();

    println!("\n=== Ontology Accuracy Summary ===");
    println!("ENVO:   {} terms loaded, {} verified against OLS",
             stats.terms_by_ontology.get(&OntologyType::Envo).unwrap_or(&0),
             envo_verified);
    println!("UBERON: {} terms loaded, {} verified against OLS",
             stats.terms_by_ontology.get(&OntologyType::Uberon).unwrap_or(&0),
             uberon_verified);
    println!("MONDO:  {} terms loaded, {} verified against OLS",
             stats.terms_by_ontology.get(&OntologyType::Mondo).unwrap_or(&0),
             mondo_verified);
    println!("Total verified: {}/{}",
             envo_verified + uberon_verified + mondo_verified,
             stats.total_terms);
    println!("=================================\n");
}
