//! NCBI Taxonomy validation.
//!
//! This module provides validation of taxonomic names against the NCBI Taxonomy
//! database, including detection of common abbreviations, typos, and suggestions
//! for standardization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A taxonomy entry from NCBI Taxonomy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyEntry {
    /// NCBI Taxonomy ID.
    pub taxid: u32,
    /// Scientific name.
    pub scientific_name: String,
    /// Taxonomic rank (e.g., species, genus, family).
    pub rank: String,
    /// Common names (if any).
    pub common_names: Vec<String>,
}

/// Taxonomy validator for checking organism names.
#[derive(Debug, Clone)]
pub struct TaxonomyValidator {
    /// Common abbreviations and their expansions.
    abbreviations: HashMap<String, String>,
    /// Common organism names (hardcoded for now, will load from NCBI later).
    known_organisms: HashMap<String, TaxonomyEntry>,
}

impl TaxonomyValidator {
    /// Create a new taxonomy validator with common organisms.
    pub fn new() -> Self {
        let mut validator = Self {
            abbreviations: HashMap::new(),
            known_organisms: HashMap::new(),
        };
        validator.load_common_abbreviations();
        validator.load_common_organisms();
        validator
    }

    /// Load common taxonomic abbreviations.
    fn load_common_abbreviations(&mut self) {
        let abbrevs = [
            ("E. coli", "Escherichia coli"),
            ("S. aureus", "Staphylococcus aureus"),
            ("B. subtilis", "Bacillus subtilis"),
            ("P. aeruginosa", "Pseudomonas aeruginosa"),
            ("S. cerevisiae", "Saccharomyces cerevisiae"),
            ("C. elegans", "Caenorhabditis elegans"),
            ("D. melanogaster", "Drosophila melanogaster"),
            ("M. musculus", "Mus musculus"),
            ("H. sapiens", "Homo sapiens"),
            ("A. thaliana", "Arabidopsis thaliana"),
            ("B. fragilis", "Bacteroides fragilis"),
            ("C. difficile", "Clostridioides difficile"),
            ("F. prausnitzii", "Faecalibacterium prausnitzii"),
            ("L. reuteri", "Limosilactobacillus reuteri"),
            ("B. longum", "Bifidobacterium longum"),
        ];

        for (abbrev, full) in abbrevs {
            self.abbreviations
                .insert(abbrev.to_lowercase(), full.to_string());
            // Also add without period
            let no_period = abbrev.replace('.', "");
            self.abbreviations
                .insert(no_period.to_lowercase(), full.to_string());
        }
    }

    /// Load common organisms for quick validation.
    fn load_common_organisms(&mut self) {
        let organisms = [
            (562, "Escherichia coli", "species", vec!["E. coli"]),
            (1280, "Staphylococcus aureus", "species", vec!["S. aureus", "staph"]),
            (1423, "Bacillus subtilis", "species", vec!["B. subtilis"]),
            (287, "Pseudomonas aeruginosa", "species", vec!["P. aeruginosa"]),
            (4932, "Saccharomyces cerevisiae", "species", vec!["yeast", "baker's yeast"]),
            (6239, "Caenorhabditis elegans", "species", vec!["C. elegans", "roundworm"]),
            (7227, "Drosophila melanogaster", "species", vec!["fruit fly"]),
            (10090, "Mus musculus", "species", vec!["mouse", "house mouse"]),
            (9606, "Homo sapiens", "species", vec!["human"]),
            (3702, "Arabidopsis thaliana", "species", vec!["thale cress"]),
            (817, "Bacteroides fragilis", "species", vec!["B. fragilis"]),
            (1496, "Clostridioides difficile", "species", vec!["C. difficile", "C. diff"]),
            (853, "Faecalibacterium prausnitzii", "species", vec!["F. prausnitzii"]),
            (1598, "Lactobacillus reuteri", "species", vec!["L. reuteri"]),
            (216816, "Bifidobacterium longum", "species", vec!["B. longum"]),
            // Common metagenome terms
            (408170, "human gut metagenome", "no rank", vec!["gut metagenome"]),
            (412755, "marine metagenome", "no rank", vec![]),
            (410658, "soil metagenome", "no rank", vec![]),
            (433733, "human skin metagenome", "no rank", vec!["skin metagenome"]),
            (447426, "human oral metagenome", "no rank", vec!["oral metagenome"]),
        ];

        for (taxid, name, rank, common) in organisms {
            let entry = TaxonomyEntry {
                taxid,
                scientific_name: name.to_string(),
                rank: rank.to_string(),
                common_names: common.into_iter().map(String::from).collect(),
            };
            // Index by lowercase scientific name
            self.known_organisms
                .insert(name.to_lowercase(), entry.clone());
            // Also index by common names
            for common_name in &entry.common_names {
                self.known_organisms
                    .insert(common_name.to_lowercase(), entry.clone());
            }
        }
    }

    /// Check if a value is a known abbreviation.
    pub fn expand_abbreviation(&self, value: &str) -> Option<&str> {
        self.abbreviations.get(&value.to_lowercase()).map(|s| s.as_str())
    }

    /// Look up an organism by name.
    pub fn lookup(&self, name: &str) -> Option<&TaxonomyEntry> {
        let name_lower = name.to_lowercase().trim().to_string();

        // Direct lookup
        if let Some(entry) = self.known_organisms.get(&name_lower) {
            return Some(entry);
        }

        // Try expanding abbreviation
        if let Some(expanded) = self.expand_abbreviation(&name_lower) {
            return self.known_organisms.get(&expanded.to_lowercase());
        }

        None
    }

    /// Validate an organism name and return suggestions.
    pub fn validate(&self, name: &str) -> TaxonomyValidationResult {
        let name_trimmed = name.trim();

        // Empty or placeholder values
        if name_trimmed.is_empty()
            || name_trimmed.to_lowercase() == "na"
            || name_trimmed.to_lowercase() == "unknown"
        {
            return TaxonomyValidationResult::Invalid {
                reason: "Empty or placeholder taxonomy value".to_string(),
            };
        }

        // Check if it's a known abbreviation
        if let Some(expanded) = self.expand_abbreviation(name_trimmed) {
            if let Some(entry) = self.known_organisms.get(&expanded.to_lowercase()) {
                return TaxonomyValidationResult::Abbreviation {
                    input: name_trimmed.to_string(),
                    expanded: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                };
            }
        }

        // Direct lookup
        if let Some(entry) = self.lookup(name_trimmed) {
            // Check if case is correct
            if entry.scientific_name != name_trimmed {
                return TaxonomyValidationResult::CaseError {
                    input: name_trimmed.to_string(),
                    correct: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                };
            }
            return TaxonomyValidationResult::Valid {
                scientific_name: entry.scientific_name.clone(),
                taxid: entry.taxid,
            };
        }

        // Try fuzzy matching (simple Levenshtein for now)
        if let Some((entry, distance)) = self.fuzzy_match(name_trimmed) {
            if distance <= 2 {
                return TaxonomyValidationResult::PossibleTypo {
                    input: name_trimmed.to_string(),
                    suggestion: entry.scientific_name.clone(),
                    taxid: entry.taxid,
                    distance,
                };
            }
        }

        // Unknown taxonomy
        TaxonomyValidationResult::Unknown {
            input: name_trimmed.to_string(),
        }
    }

    /// Simple fuzzy matching using Levenshtein distance.
    fn fuzzy_match(&self, name: &str) -> Option<(&TaxonomyEntry, usize)> {
        let name_lower = name.to_lowercase();
        let mut best_match: Option<(&TaxonomyEntry, usize)> = None;

        for entry in self.known_organisms.values() {
            let distance = levenshtein(&name_lower, &entry.scientific_name.to_lowercase());
            if distance <= 3 {
                match &best_match {
                    None => best_match = Some((entry, distance)),
                    Some((_, best_dist)) if distance < *best_dist => {
                        best_match = Some((entry, distance));
                    }
                    _ => {}
                }
            }
        }

        best_match
    }
}

impl Default for TaxonomyValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of taxonomy validation.
#[derive(Debug, Clone)]
pub enum TaxonomyValidationResult {
    /// The name is valid and matches NCBI Taxonomy.
    Valid { scientific_name: String, taxid: u32 },
    /// The name is an abbreviation that should be expanded.
    Abbreviation {
        input: String,
        expanded: String,
        taxid: u32,
    },
    /// The name has incorrect capitalization.
    CaseError {
        input: String,
        correct: String,
        taxid: u32,
    },
    /// The name appears to be a typo.
    PossibleTypo {
        input: String,
        suggestion: String,
        taxid: u32,
        distance: usize,
    },
    /// The name is not recognized in our database.
    Unknown { input: String },
    /// The value is invalid (empty, placeholder, etc.).
    Invalid { reason: String },
}

/// Simple Levenshtein distance implementation.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_abbreviation() {
        let validator = TaxonomyValidator::new();
        assert_eq!(
            validator.expand_abbreviation("E. coli"),
            Some("Escherichia coli")
        );
        assert_eq!(
            validator.expand_abbreviation("e coli"),
            Some("Escherichia coli")
        );
    }

    #[test]
    fn test_lookup_organism() {
        let validator = TaxonomyValidator::new();

        let result = validator.lookup("Escherichia coli");
        assert!(result.is_some());
        assert_eq!(result.unwrap().taxid, 562);

        let result = validator.lookup("human");
        assert!(result.is_some());
        assert_eq!(result.unwrap().taxid, 9606);
    }

    #[test]
    fn test_validate_valid() {
        let validator = TaxonomyValidator::new();

        match validator.validate("Escherichia coli") {
            TaxonomyValidationResult::Valid { taxid, .. } => assert_eq!(taxid, 562),
            other => panic!("Expected Valid, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_abbreviation() {
        let validator = TaxonomyValidator::new();

        match validator.validate("E. coli") {
            TaxonomyValidationResult::Abbreviation { taxid, expanded, .. } => {
                assert_eq!(taxid, 562);
                assert_eq!(expanded, "Escherichia coli");
            }
            other => panic!("Expected Abbreviation, got {:?}", other),
        }
    }

    #[test]
    fn test_validate_case_error() {
        let validator = TaxonomyValidator::new();

        match validator.validate("escherichia coli") {
            TaxonomyValidationResult::CaseError { correct, .. } => {
                assert_eq!(correct, "Escherichia coli");
            }
            other => panic!("Expected CaseError, got {:?}", other),
        }
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("hello", "hello"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
    }
}
