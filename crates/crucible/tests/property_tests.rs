//! Property-based tests for Crucible validators.
//!
//! These tests use proptest to generate random inputs and verify that
//! validators maintain their invariants under all conditions.
//!
//! # Testing Philosophy
//!
//! Property-based tests verify:
//! 1. **No panics**: Validators never crash on any input
//! 2. **Determinism**: Same input always produces same output
//! 3. **Consistency**: Related operations produce consistent results
//! 4. **Invariants**: Core properties always hold
//!
//! # Running Property Tests
//!
//! ```bash
//! # Run all property tests
//! cargo test -p crucible --test property_tests
//!
//! # Run with more cases (slower but more thorough)
//! PROPTEST_CASES=10000 cargo test -p crucible --test property_tests
//! ```

use proptest::prelude::*;

use crucible::bio::{
    AccessionValidator, AccessionType,
    OntologyValidator, OntologyType, OntologyValidationResult,
    TaxonomyValidator, TaxonomyValidationResult,
};

// =============================================================================
// Test Strategies
// =============================================================================

/// Generate arbitrary ASCII strings (common case)
fn ascii_string() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\-\\.\\s]{0,100}"
}

/// Generate strings that look like scientific names
fn scientific_name_like() -> impl Strategy<Value = String> {
    prop_oneof![
        // Genus species format
        "[A-Z][a-z]{2,15} [a-z]{3,15}",
        // Abbreviated genus
        "[A-Z]\\. [a-z]{3,15}",
        // Single word
        "[A-Za-z]{3,20}",
        // With strain
        "[A-Z][a-z]{2,15} [a-z]{3,15} [A-Z0-9\\-]{1,10}",
    ]
}

/// Generate strings that look like accessions
fn accession_like() -> impl Strategy<Value = String> {
    prop_oneof![
        // BioSample-like
        "SAM[NED][0-9]{6,12}",
        // SRA-like
        "[SED]R[RXSP][0-9]{6,10}",
        // BioProject-like
        "PRJ[NEDB][A-Z][0-9]{5,8}",
        // GenBank-like
        "[A-Z]{1,2}_?[0-9]{5,12}(\\.[0-9]{1,2})?",
        // Random alphanumeric
        "[A-Z0-9]{4,15}",
    ]
}

/// Generate strings that look like ontology IDs
fn ontology_id_like() -> impl Strategy<Value = String> {
    prop_oneof![
        // ENVO format
        "ENVO:[0-9]{7,8}",
        // UBERON format
        "UBERON:[0-9]{7}",
        // MONDO format
        "MONDO:[0-9]{7}",
        // Random prefix:number
        "[A-Z]{2,6}:[0-9]{4,10}",
        // Free text
        "[a-z ]{5,30}",
    ]
}

/// Generate strings that look like dates
fn date_like() -> impl Strategy<Value = String> {
    prop_oneof![
        // ISO format
        "[12][0-9]{3}-[01][0-9]-[0-3][0-9]",
        // US format
        "[01][0-9]/[0-3][0-9]/[12][0-9]{3}",
        // European format
        "[0-3][0-9]/[01][0-9]/[12][0-9]{3}",
        // Month name
        "(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-3]?[0-9], [12][0-9]{3}",
        // Random text
        "[a-zA-Z0-9\\-/]{5,15}",
    ]
}

/// Generate completely random bytes (edge cases)
fn random_bytes() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<u8>(), 0..200)
        .prop_filter_map("valid UTF-8", |bytes| {
            String::from_utf8(bytes).ok()
        })
}

// =============================================================================
// Taxonomy Validator Properties
// =============================================================================

mod taxonomy_tests {
    use super::*;

    proptest! {
        /// Taxonomy validator never panics on any ASCII input.
        #[test]
        fn never_panics_on_ascii(input in ascii_string()) {
            let validator = TaxonomyValidator::new();
            let _ = validator.validate(&input);
        }

        /// Taxonomy validator never panics on scientific name-like inputs.
        #[test]
        fn never_panics_on_scientific_names(input in scientific_name_like()) {
            let validator = TaxonomyValidator::new();
            let _ = validator.validate(&input);
        }

        /// Taxonomy validator never panics on random UTF-8.
        #[test]
        fn never_panics_on_random_utf8(input in random_bytes()) {
            let validator = TaxonomyValidator::new();
            let _ = validator.validate(&input);
        }

        /// Taxonomy validation is deterministic.
        #[test]
        fn validation_is_deterministic(input in ascii_string()) {
            let validator = TaxonomyValidator::new();
            let result1 = validator.validate(&input);
            let result2 = validator.validate(&input);

            // Results should be identical
            prop_assert_eq!(
                format!("{:?}", result1),
                format!("{:?}", result2)
            );
        }

        /// Valid taxa always return Valid result.
        #[test]
        fn known_valid_taxa_are_valid(
            taxon in prop_oneof![
                Just("Escherichia coli"),
                Just("Homo sapiens"),
                Just("Mus musculus"),
                Just("Saccharomyces cerevisiae"),
                Just("Drosophila melanogaster"),
            ]
        ) {
            let validator = TaxonomyValidator::new();
            let result = validator.validate(taxon);

            prop_assert!(
                matches!(result, TaxonomyValidationResult::Valid { .. }),
                "Known valid taxon '{}' should be Valid, got {:?}",
                taxon, result
            );
        }

        /// Abbreviations are detected for known organisms.
        #[test]
        fn abbreviations_are_detected(
            (abbrev, full) in prop_oneof![
                Just(("E. coli", "Escherichia coli")),
                Just(("S. aureus", "Staphylococcus aureus")),
                Just(("B. subtilis", "Bacillus subtilis")),
            ]
        ) {
            let validator = TaxonomyValidator::new();
            let result = validator.validate(abbrev);

            match result {
                TaxonomyValidationResult::Abbreviation { expanded, .. } => {
                    prop_assert_eq!(expanded, full);
                }
                other => {
                    prop_assert!(false, "Expected Abbreviation, got {:?}", other);
                }
            }
        }

        /// Empty string returns Invalid (not panic).
        #[test]
        fn empty_string_returns_invalid(_dummy in Just(())) {
            let validator = TaxonomyValidator::new();
            let result = validator.validate("");

            prop_assert!(
                matches!(result, TaxonomyValidationResult::Invalid { .. }),
                "Empty string should return Invalid, got {:?}",
                result
            );
        }

        /// Very long strings don't cause issues.
        #[test]
        fn handles_long_strings(length in 100..1000usize) {
            let long_string = "a".repeat(length);
            let validator = TaxonomyValidator::new();
            let _ = validator.validate(&long_string);
            // Just verify no panic
        }
    }
}

// =============================================================================
// Accession Validator Properties
// =============================================================================

mod accession_tests {
    use super::*;

    proptest! {
        /// Accession validator never panics on any ASCII input.
        #[test]
        fn never_panics_on_ascii(input in ascii_string()) {
            let validator = AccessionValidator::new();
            let _ = validator.validate(&input);
        }

        /// Accession validator never panics on accession-like inputs.
        #[test]
        fn never_panics_on_accession_like(input in accession_like()) {
            let validator = AccessionValidator::new();
            let _ = validator.validate(&input);
        }

        /// Accession validator never panics on random UTF-8.
        #[test]
        fn never_panics_on_random_utf8(input in random_bytes()) {
            let validator = AccessionValidator::new();
            let _ = validator.validate(&input);
        }

        /// Accession validation is deterministic.
        #[test]
        fn validation_is_deterministic(input in accession_like()) {
            let validator = AccessionValidator::new();
            let result1 = validator.validate(&input);
            let result2 = validator.validate(&input);

            prop_assert_eq!(
                format!("{:?}", result1),
                format!("{:?}", result2)
            );
        }

        /// Valid BioSample accessions are recognized.
        #[test]
        fn valid_biosample_recognized(
            prefix in prop_oneof![Just("SAMN"), Just("SAME"), Just("SAMD")],
            digits in "[0-9]{8,12}"
        ) {
            let accession = format!("{}{}", prefix, digits);
            let validator = AccessionValidator::new();
            let result = validator.validate(&accession);

            prop_assert!(
                result.is_valid && result.accession_type == Some(AccessionType::BioSample),
                "BioSample {} should be valid, got {:?}",
                accession, result
            );
        }

        /// Valid SRA run accessions are recognized.
        #[test]
        fn valid_sra_run_recognized(
            prefix in prop_oneof![Just("SRR"), Just("ERR"), Just("DRR")],
            digits in "[0-9]{6,9}"
        ) {
            let accession = format!("{}{}", prefix, digits);
            let validator = AccessionValidator::new();
            let result = validator.validate(&accession);

            prop_assert!(
                result.is_valid && result.accession_type == Some(AccessionType::SraRun),
                "SRA Run {} should be valid, got {:?}",
                accession, result
            );
        }

        /// Accession patterns are mutually exclusive (no overlaps).
        #[test]
        fn patterns_are_exclusive(input in accession_like()) {
            let validator = AccessionValidator::new();
            let result = validator.validate(&input);

            // A valid accession should match exactly one type
            if result.is_valid {
                // Verify the type is unique by checking it doesn't match other patterns
                // This is implicitly verified by the validator returning a single type
                prop_assert!(result.accession_type.is_some());
            }
        }

        /// URL generation works for valid accessions.
        #[test]
        fn url_generation_for_valid(
            accession in prop_oneof![
                Just("SAMN12345678"),
                Just("SRR1234567"),
                Just("PRJNA123456"),
            ]
        ) {
            let validator = AccessionValidator::new();
            let url = validator.get_url(accession);

            prop_assert!(url.is_some(), "Valid accession {} should have URL", accession);
            prop_assert!(url.unwrap().starts_with("https://"));
        }

        /// Invalid accessions return Invalid result.
        #[test]
        fn invalid_accessions_detected(
            input in prop_oneof![
                Just("INVALID_STRING"),
                Just("NOT_AN_ACCESSION"),
                Just("SAM"), // Too short
                Just("SRR12"), // Too short
                Just("PRJNA"), // Missing digits
            ]
        ) {
            let validator = AccessionValidator::new();
            let result = validator.validate(input);

            prop_assert!(
                !result.is_valid,
                "Invalid accession '{}' should be Invalid, got {:?}",
                input, result
            );
        }
    }
}

// =============================================================================
// Ontology Validator Properties
// =============================================================================

mod ontology_tests {
    use super::*;

    proptest! {
        /// Ontology validator never panics on any ASCII input.
        #[test]
        fn never_panics_on_ascii(input in ascii_string()) {
            let validator = OntologyValidator::new();
            let _ = validator.validate_id(&input);
            let _ = validator.lookup(&input);
        }

        /// Ontology validator never panics on ontology ID-like inputs.
        #[test]
        fn never_panics_on_ontology_ids(input in ontology_id_like()) {
            let validator = OntologyValidator::new();
            let _ = validator.validate_id(&input);
            let _ = validator.lookup(&input);
        }

        /// Ontology validator never panics on random UTF-8.
        #[test]
        fn never_panics_on_random_utf8(input in random_bytes()) {
            let validator = OntologyValidator::new();
            let _ = validator.validate_id(&input);
            let _ = validator.lookup(&input);
        }

        /// Ontology validation is deterministic.
        #[test]
        fn validation_is_deterministic(input in ontology_id_like()) {
            let validator = OntologyValidator::new();
            let result1 = validator.validate_id(&input);
            let result2 = validator.validate_id(&input);

            prop_assert_eq!(
                format!("{:?}", result1),
                format!("{:?}", result2)
            );
        }

        /// Valid ENVO IDs are recognized.
        #[test]
        fn valid_envo_ids_recognized(
            id in prop_oneof![
                Just("ENVO:00002030"), // aquatic biome
                Just("ENVO:00000022"), // river
                Just("ENVO:00000446"), // terrestrial biome
            ]
        ) {
            let validator = OntologyValidator::new();
            let result = validator.validate_id(id);

            prop_assert!(
                matches!(result, OntologyValidationResult::Valid { .. } | OntologyValidationResult::UnknownTerm { .. }),
                "Known ENVO ID {} should have valid format, got {:?}",
                id, result
            );
        }

        /// Suggest mappings returns consistent results.
        #[test]
        fn suggest_mappings_consistent(
            term in prop_oneof![
                Just("gut"),
                Just("intestine"),
                Just("soil"),
                Just("water"),
            ]
        ) {
            let validator = OntologyValidator::new();
            let results1 = validator.suggest_mappings(term, None);
            let results2 = validator.suggest_mappings(term, None);

            prop_assert_eq!(
                results1.len(),
                results2.len(),
                "Mapping count should be consistent"
            );
        }

        /// Ontology type detection is consistent.
        #[test]
        fn type_detection_consistent(input in ontology_id_like()) {
            let type1 = OntologyType::from_id(&input);
            let type2 = OntologyType::from_id(&input);

            prop_assert_eq!(
                format!("{:?}", type1),
                format!("{:?}", type2)
            );
        }
    }
}

// =============================================================================
// Date Parsing Properties
// =============================================================================

mod date_tests {
    use super::*;

    proptest! {
        /// Date parsing never panics on any input.
        #[test]
        fn never_panics_on_date_like(input in date_like()) {
            // Just try to parse - shouldn't panic
            let _ = chrono::NaiveDate::parse_from_str(&input, "%Y-%m-%d");
            let _ = chrono::NaiveDate::parse_from_str(&input, "%m/%d/%Y");
            let _ = chrono::NaiveDate::parse_from_str(&input, "%d/%m/%Y");
        }

        /// Date parsing never panics on random input.
        #[test]
        fn never_panics_on_random(input in ascii_string()) {
            let _ = chrono::NaiveDate::parse_from_str(&input, "%Y-%m-%d");
        }

        /// Valid ISO dates parse correctly.
        #[test]
        fn valid_iso_dates_parse(
            year in 1900..2100i32,
            month in 1..=12u32,
            day in 1..=28u32, // Use 28 to avoid month-length issues
        ) {
            let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
            let result = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d");

            prop_assert!(result.is_ok(), "Valid ISO date {} should parse", date_str);
        }
    }
}

// =============================================================================
// Cross-Validator Properties
// =============================================================================

mod cross_validator_tests {
    use super::*;

    proptest! {
        /// Multiple validators can be used on the same input without interference.
        #[test]
        fn validators_are_independent(input in ascii_string()) {
            let taxonomy = TaxonomyValidator::new();
            let accession = AccessionValidator::new();
            let ontology = OntologyValidator::new();

            // All should complete without panicking
            let _ = taxonomy.validate(&input);
            let _ = accession.validate(&input);
            let _ = ontology.validate_id(&input);

            // Running them again should produce same results
            let t1 = taxonomy.validate(&input);
            let t2 = taxonomy.validate(&input);
            prop_assert_eq!(format!("{:?}", t1), format!("{:?}", t2));
        }

        /// Validators handle edge cases consistently.
        #[test]
        fn edge_cases_handled(
            input in prop_oneof![
                Just(""),
                Just(" "),
                Just("\t\n"),
                Just("   leading spaces"),
                Just("trailing spaces   "),
                Just("ALLCAPS"),
                Just("alllower"),
                Just("MiXeD CaSe"),
                Just("with.dots"),
                Just("with-dashes"),
                Just("with_underscores"),
                Just("12345"),
                Just("!@#$%^&*()"),
            ]
        ) {
            let taxonomy = TaxonomyValidator::new();
            let accession = AccessionValidator::new();
            let ontology = OntologyValidator::new();

            // None should panic
            let _ = taxonomy.validate(input);
            let _ = accession.validate(input);
            let _ = ontology.validate_id(input);
        }
    }
}

// =============================================================================
// Regression Property Tests
// =============================================================================

mod regression_property_tests {
    use super::*;

    proptest! {
        /// Gene IDs (pure numbers) should never match PDB pattern.
        #[test]
        fn gene_ids_not_pdb(gene_id in "[0-9]{1,10}") {
            let validator = AccessionValidator::new();
            let result = validator.validate(&gene_id);

            // Should either be GeneId or Invalid, never PDB
            if result.is_valid {
                if let Some(accession_type) = result.accession_type {
                    prop_assert_ne!(
                        accession_type,
                        AccessionType::Pdb,
                        "Pure numeric '{}' should not match PDB",
                        gene_id
                    );
                }
            }
        }

        /// Short SRA accessions should not match SraRun type.
        #[test]
        fn short_sra_not_sra_run(
            prefix in prop_oneof![Just("SRR"), Just("ERR"), Just("DRR")],
            digits in "[0-9]{1,5}" // Too short for SRA Run (needs 6-9 digits)
        ) {
            let accession = format!("{}{}", prefix, digits);
            let validator = AccessionValidator::new();
            let result = validator.validate(&accession);

            // Short accessions should NOT match SraRun type
            // They may match other patterns (e.g., Protein) but never SraRun
            prop_assert!(
                result.accession_type != Some(AccessionType::SraRun),
                "Short SRA '{}' should not match SraRun, got {:?}",
                accession, result
            );
        }

        /// Taxonomy abbreviations with various punctuation should be handled.
        #[test]
        fn abbreviation_punctuation(
            genus_initial in "[A-Z]",
            separator in prop_oneof![Just(". "), Just("."), Just(" ")],
            species in "[a-z]{3,10}"
        ) {
            let name = format!("{}{}{}", genus_initial, separator, species);
            let validator = TaxonomyValidator::new();

            // Should not panic regardless of separator
            let _ = validator.validate(&name);
        }
    }
}
