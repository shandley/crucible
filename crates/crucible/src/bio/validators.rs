//! Bioinformatics-specific validators.
//!
//! This module provides validators for biological metadata including
//! MIxS compliance checking, taxonomy validation, and ontology term mapping.

use crate::bio::mixs::{MixsPackage, MixsSchema};
use crate::bio::ontology::{OntologyType, OntologyValidator};
use crate::bio::taxonomy::{TaxonomyValidationResult, TaxonomyValidator};
use crate::input::DataTable;
use crate::schema::TableSchema;
use crate::validation::{Evidence, Observation, ObservationType, Severity};

use serde_json::json;
use std::collections::HashSet;

/// Trait for bioinformatics validators.
pub trait BioValidator: Send + Sync {
    /// Validate the data and return observations.
    fn validate(&self, data: &DataTable, schema: &TableSchema) -> Vec<Observation>;

    /// Get the validator name.
    fn name(&self) -> &'static str;
}

/// Validates metadata against MIxS (Minimum Information about any (x) Sequence) standards.
pub struct MixsComplianceValidator {
    /// The MIxS schema.
    schema: MixsSchema,
    /// The environmental package to validate against (if specified).
    package: Option<MixsPackage>,
    /// Taxonomy validator for organism fields.
    taxonomy_validator: TaxonomyValidator,
    /// Ontology validator for environmental and anatomical terms.
    ontology_validator: OntologyValidator,
}

impl MixsComplianceValidator {
    /// Create a new MIxS compliance validator.
    pub fn new() -> Self {
        Self {
            schema: MixsSchema::new(),
            package: None,
            taxonomy_validator: TaxonomyValidator::new(),
            ontology_validator: OntologyValidator::new(),
        }
    }

    /// Set the MIxS package to validate against.
    pub fn with_package(mut self, package: MixsPackage) -> Self {
        self.package = Some(package);
        self
    }

    /// Try to detect the appropriate MIxS package from the data.
    pub fn detect_package(&self, data: &DataTable, schema: &TableSchema) -> Option<MixsPackage> {
        // Look for clues in column names and values
        let column_names: HashSet<String> = schema
            .columns
            .iter()
            .map(|c| c.name.to_lowercase())
            .collect();

        // Check for human-associated indicators
        if column_names.contains("host_subject_id")
            || column_names.contains("subject_id")
            || column_names.contains("patient_id")
        {
            // Try to detect specific body site
            if let Some(body_site_col) = schema.columns.iter().find(|c| {
                let name = c.name.to_lowercase();
                name.contains("body_site") || name.contains("tissue") || name.contains("sample_site")
            }) {
                let col_idx = schema
                    .columns
                    .iter()
                    .position(|c| c.name == body_site_col.name);
                if let Some(idx) = col_idx {
                    // Sample some values
                    for row in data.rows.iter().take(10) {
                        if let Some(value) = row.get(idx) {
                            let value_lower = value.to_lowercase();
                            if value_lower.contains("gut")
                                || value_lower.contains("stool")
                                || value_lower.contains("fecal")
                                || value_lower.contains("intestin")
                            {
                                return Some(MixsPackage::HumanGut);
                            }
                            if value_lower.contains("skin") || value_lower.contains("dermal") {
                                return Some(MixsPackage::HumanSkin);
                            }
                            if value_lower.contains("oral")
                                || value_lower.contains("saliva")
                                || value_lower.contains("mouth")
                            {
                                return Some(MixsPackage::HumanOral);
                            }
                        }
                    }
                }
            }
            return Some(MixsPackage::HumanAssociated);
        }

        // Check for environmental indicators
        if column_names.contains("depth") && column_names.contains("salinity") {
            return Some(MixsPackage::Water);
        }
        if column_names.contains("soil_type") || column_names.contains("ph") {
            return Some(MixsPackage::Soil);
        }

        None
    }

    /// Get MIxS compliance score (0.0 to 1.0).
    pub fn compliance_score(&self, data: &DataTable, schema: &TableSchema) -> f64 {
        let package = self.package.or_else(|| self.detect_package(data, schema));
        let package = match package {
            Some(p) => p,
            None => return 0.0, // Can't calculate without knowing the package
        };

        let mandatory_fields = self.schema.mandatory_fields_for_package(package);
        if mandatory_fields.is_empty() {
            return 1.0;
        }

        let mut found = 0;
        for field in &mandatory_fields {
            // Check if column exists (including aliases)
            let exists = schema.columns.iter().any(|c| field.matches_column(&c.name));
            if exists {
                found += 1;
            }
        }

        found as f64 / mandatory_fields.len() as f64
    }

    /// Validate lat_lon format.
    fn validate_lat_lon(&self, value: &str) -> bool {
        // Accept various formats:
        // "38.98 -77.11" (decimal degrees)
        // "38.98N 77.11W"
        // "missing", "not collected", "not applicable"

        let value_lower = value.to_lowercase().trim().to_string();

        // Accept placeholder values
        if value_lower == "missing"
            || value_lower == "not collected"
            || value_lower == "not applicable"
            || value_lower == "na"
        {
            return true;
        }

        // Try to parse as decimal degrees
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() == 2 {
            // Try to parse both parts as numbers (with optional N/S/E/W suffix)
            let parse_coord = |s: &str| -> Option<f64> {
                let s = s
                    .trim_end_matches(['N', 'S', 'E', 'W', 'n', 's', 'e', 'w'])
                    .trim();
                s.parse::<f64>().ok()
            };

            if let (Some(lat), Some(lon)) = (parse_coord(parts[0]), parse_coord(parts[1])) {
                // Basic range check
                return (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon);
            }
        }

        false
    }

    /// Validate date format.
    fn validate_date_format(&self, value: &str) -> bool {
        let value_lower = value.to_lowercase().trim().to_string();

        // Accept placeholder values
        if value_lower == "missing"
            || value_lower == "not collected"
            || value_lower == "not applicable"
            || value_lower == "na"
        {
            return true;
        }

        // Check for ISO 8601 formats
        // YYYY-MM-DD
        if value.len() == 10 && value.chars().nth(4) == Some('-') && value.chars().nth(7) == Some('-') {
            return true;
        }
        // YYYY-MM
        if value.len() == 7 && value.chars().nth(4) == Some('-') {
            return true;
        }
        // YYYY
        if value.len() == 4 && value.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }

        false
    }
}

impl Default for MixsComplianceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl BioValidator for MixsComplianceValidator {
    fn validate(&self, data: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        // Determine package to validate against
        let package = self.package.or_else(|| self.detect_package(data, schema));
        let package = match package {
            Some(p) => p,
            None => {
                // Can't determine package - add informational observation
                observations.push(
                    Observation::new(
                        ObservationType::ConstraintViolation,
                        Severity::Info,
                        "_metadata",
                        "Could not determine MIxS environmental package. Specify --mixs-package for full validation.",
                    )
                    .with_confidence(0.5)
                    .with_detector("MixsComplianceValidator"),
                );
                return observations;
            }
        };

        // Get fields for this package
        let mandatory_fields = self.schema.mandatory_fields_for_package(package);

        // Check for missing mandatory fields
        for field in &mandatory_fields {
            let column_exists = schema.columns.iter().any(|c| field.matches_column(&c.name));

            if !column_exists {
                observations.push(
                    Observation::new(
                        ObservationType::Completeness,
                        Severity::Error,
                        &field.name,
                        format!(
                            "Missing mandatory MIxS field '{}' ({}) for {} package",
                            field.name,
                            field.label,
                            package.name()
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_expected(json!({
                                "field": field.name,
                                "requirement": "mandatory",
                                "package": package.name(),
                                "description": field.description,
                                "format": field.format,
                                "example": field.example,
                            })),
                    )
                    .with_confidence(0.95)
                    .with_detector("MixsComplianceValidator"),
                );
            }
        }

        // Validate format of specific fields
        for (col_idx, col) in schema.columns.iter().enumerate() {
            if let Some(field) = self.schema.find_field(&col.name, Some(package)) {
                // Validate lat_lon format
                if field.name == "lat_lon" {
                    let mut invalid_rows = Vec::new();
                    for (row_idx, row) in data.rows.iter().enumerate() {
                        if let Some(value) = row.get(col_idx) {
                            if !value.is_empty() && !self.validate_lat_lon(value) {
                                invalid_rows.push(row_idx);
                            }
                        }
                    }
                    if !invalid_rows.is_empty() {
                        let sample_value = data.rows.get(invalid_rows[0])
                            .and_then(|r| r.get(col_idx))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        observations.push(
                            Observation::new(
                                ObservationType::PatternViolation,
                                Severity::Warning,
                                &col.name,
                                format!(
                                    "Invalid lat_lon format in {} rows. Expected 'DD.DDDD DD.DDDD' format.",
                                    invalid_rows.len()
                                ),
                            )
                            .with_evidence(
                                Evidence::new()
                                    .with_value(sample_value)
                                    .with_occurrences(invalid_rows.len())
                                    .with_sample_rows(invalid_rows.into_iter().take(5).collect())
                                    .with_expected("38.98 -77.11 or 38.98N 77.11W"),
                            )
                            .with_confidence(0.9)
                            .with_detector("MixsComplianceValidator"),
                        );
                    }
                }

                // Validate collection_date format
                if field.name == "collection_date" {
                    let mut invalid_rows = Vec::new();
                    for (row_idx, row) in data.rows.iter().enumerate() {
                        if let Some(value) = row.get(col_idx) {
                            if !value.is_empty() && !self.validate_date_format(value) {
                                invalid_rows.push(row_idx);
                            }
                        }
                    }
                    if !invalid_rows.is_empty() {
                        let sample_value = data.rows.get(invalid_rows[0])
                            .and_then(|r| r.get(col_idx))
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        observations.push(
                            Observation::new(
                                ObservationType::PatternViolation,
                                Severity::Warning,
                                &col.name,
                                format!(
                                    "Non-ISO date format in {} rows. MIxS requires ISO 8601 dates.",
                                    invalid_rows.len()
                                ),
                            )
                            .with_evidence(
                                Evidence::new()
                                    .with_value(sample_value)
                                    .with_occurrences(invalid_rows.len())
                                    .with_sample_rows(invalid_rows.into_iter().take(5).collect())
                                    .with_expected("YYYY-MM-DD, YYYY-MM, or YYYY"),
                            )
                            .with_confidence(0.9)
                            .with_detector("MixsComplianceValidator"),
                        );
                    }
                }
            }

            // Check for organism/taxonomy columns
            let col_lower = col.name.to_lowercase();
            if is_taxonomy_column(&col_lower) {
                // Validate taxonomy values
                let mut abbreviations = Vec::new();
                let mut case_errors = Vec::new();
                let mut typos = Vec::new();
                let mut unknown = Vec::new();

                for (row_idx, row) in data.rows.iter().enumerate() {
                    if let Some(value) = row.get(col_idx) {
                        if value.is_empty() {
                            continue;
                        }

                        match self.taxonomy_validator.validate(value) {
                            TaxonomyValidationResult::Valid { .. } => {}
                            TaxonomyValidationResult::Abbreviation { input, expanded, taxid } => {
                                abbreviations.push((row_idx, input, expanded, taxid));
                            }
                            TaxonomyValidationResult::CaseError { input, correct, taxid } => {
                                case_errors.push((row_idx, input, correct, taxid));
                            }
                            TaxonomyValidationResult::PossibleTypo { input, suggestion, taxid, .. } => {
                                typos.push((row_idx, input, suggestion, taxid));
                            }
                            TaxonomyValidationResult::Unknown { input } => {
                                unknown.push((row_idx, input));
                            }
                            TaxonomyValidationResult::Invalid { .. } => {}
                        }
                    }
                }

                // Report abbreviations
                if !abbreviations.is_empty() {
                    let sample = &abbreviations[0];
                    observations.push(
                        Observation::new(
                            ObservationType::Inconsistency,
                            Severity::Warning,
                            &col.name,
                            format!(
                                "Abbreviated taxonomy names found ({} occurrences). NCBI prefers full scientific names.",
                                abbreviations.len()
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_value(json!({
                                    "example": sample.1,
                                    "suggested": sample.2,
                                    "taxid": sample.3,
                                }))
                                .with_occurrences(abbreviations.len())
                                .with_sample_rows(abbreviations.iter().take(5).map(|a| a.0).collect()),
                        )
                        .with_confidence(0.95)
                        .with_detector("TaxonomyValidator"),
                    );
                }

                // Report case errors
                if !case_errors.is_empty() {
                    let sample = &case_errors[0];
                    observations.push(
                        Observation::new(
                            ObservationType::Inconsistency,
                            Severity::Warning,
                            &col.name,
                            format!(
                                "Incorrect capitalization in taxonomy names ({} occurrences).",
                                case_errors.len()
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_value(json!({
                                    "example": sample.1,
                                    "correct": sample.2,
                                    "taxid": sample.3,
                                }))
                                .with_occurrences(case_errors.len())
                                .with_sample_rows(case_errors.iter().take(5).map(|a| a.0).collect()),
                        )
                        .with_confidence(0.9)
                        .with_detector("TaxonomyValidator"),
                    );
                }

                // Report possible typos
                if !typos.is_empty() {
                    let sample = &typos[0];
                    observations.push(
                        Observation::new(
                            ObservationType::Inconsistency,
                            Severity::Warning,
                            &col.name,
                            format!(
                                "Possible typos in taxonomy names ({} occurrences).",
                                typos.len()
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_value(json!({
                                    "example": sample.1,
                                    "suggestion": sample.2,
                                    "taxid": sample.3,
                                }))
                                .with_occurrences(typos.len())
                                .with_sample_rows(typos.iter().take(5).map(|a| a.0).collect()),
                        )
                        .with_confidence(0.7)
                        .with_detector("TaxonomyValidator"),
                    );
                }

                // Report unknown taxa (info level - might just not be in our limited database)
                if !unknown.is_empty() && unknown.len() <= 5 {
                    observations.push(
                        Observation::new(
                            ObservationType::ConstraintViolation,
                            Severity::Info,
                            &col.name,
                            format!(
                                "Unrecognized taxonomy names ({} values). Verify against NCBI Taxonomy.",
                                unknown.len()
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_value(json!(unknown.iter().map(|u| &u.1).collect::<Vec<_>>()))
                                .with_occurrences(unknown.len())
                                .with_sample_rows(unknown.iter().take(5).map(|a| a.0).collect()),
                        )
                        .with_confidence(0.5)
                        .with_detector("TaxonomyValidator"),
                    );
                }
            }

            // Check for ontology columns (ENVO, UBERON, MONDO)
            if let Some(ontology_type) = classify_ontology_column(&col.name) {
                let target_ontology = match ontology_type {
                    OntologyColumnType::Envo => OntologyType::Envo,
                    OntologyColumnType::Uberon => OntologyType::Uberon,
                    OntologyColumnType::Mondo => OntologyType::Mondo,
                };

                let ontology_name = match ontology_type {
                    OntologyColumnType::Envo => "ENVO (Environmental Ontology)",
                    OntologyColumnType::Uberon => "UBERON (Anatomy Ontology)",
                    OntologyColumnType::Mondo => "MONDO (Disease Ontology)",
                };

                let mut unmapped_terms: Vec<(usize, String, Vec<String>)> = Vec::new();
                let mut invalid_ids: Vec<(usize, String)> = Vec::new();
                let mut _valid_terms = 0; // Prefixed with _ to suppress warning - could be used for stats

                for (row_idx, row) in data.rows.iter().enumerate() {
                    if let Some(value) = row.get(col_idx) {
                        if value.is_empty()
                            || value.to_lowercase() == "missing"
                            || value.to_lowercase() == "not applicable"
                            || value.to_lowercase() == "na"
                        {
                            continue;
                        }

                        // Check if value looks like an ontology ID (e.g., ENVO:00000446)
                        if value.contains(':') && value.chars().any(|c| c.is_ascii_digit()) {
                            // Validate the ontology ID
                            match self.ontology_validator.validate_id(value) {
                                crate::bio::ontology::OntologyValidationResult::Valid { .. } => {
                                    _valid_terms += 1;
                                }
                                crate::bio::ontology::OntologyValidationResult::UnknownTerm { .. }
                                | crate::bio::ontology::OntologyValidationResult::InvalidFormat { .. } => {
                                    invalid_ids.push((row_idx, value.clone()));
                                }
                            }
                        } else {
                            // Free-text term - suggest ontology mappings
                            let mappings =
                                self.ontology_validator.suggest_mappings(value, Some(target_ontology));
                            if mappings.is_empty() {
                                unmapped_terms.push((row_idx, value.clone(), vec![]));
                            } else {
                                let suggestions: Vec<String> = mappings
                                    .iter()
                                    .map(|m| format!("{} ({})", m.term_id, m.term_label))
                                    .collect();
                                unmapped_terms.push((row_idx, value.clone(), suggestions));
                            }
                        }
                    }
                }

                // Report unmapped free-text terms with suggestions
                if !unmapped_terms.is_empty() {
                    let terms_with_suggestions: Vec<_> =
                        unmapped_terms.iter().filter(|t| !t.2.is_empty()).collect();
                    let terms_without_suggestions: Vec<_> =
                        unmapped_terms.iter().filter(|t| t.2.is_empty()).collect();

                    if !terms_with_suggestions.is_empty() {
                        let sample = &terms_with_suggestions[0];
                        observations.push(
                            Observation::new(
                                ObservationType::Inconsistency,
                                Severity::Warning,
                                &col.name,
                                format!(
                                    "Free-text values could be mapped to {}. Consider using ontology IDs for FAIR compliance.",
                                    ontology_name
                                ),
                            )
                            .with_evidence(
                                Evidence::new()
                                    .with_value(json!({
                                        "example_term": sample.1,
                                        "suggested_mappings": sample.2,
                                    }))
                                    .with_occurrences(terms_with_suggestions.len())
                                    .with_sample_rows(
                                        terms_with_suggestions.iter().take(5).map(|t| t.0).collect(),
                                    ),
                            )
                            .with_confidence(0.8)
                            .with_detector("OntologyValidator"),
                        );
                    }

                    if !terms_without_suggestions.is_empty() {
                        observations.push(
                            Observation::new(
                                ObservationType::ConstraintViolation,
                                Severity::Info,
                                &col.name,
                                format!(
                                    "Unrecognized terms for {} ({} values). Consider verifying against the ontology.",
                                    ontology_name,
                                    terms_without_suggestions.len()
                                ),
                            )
                            .with_evidence(
                                Evidence::new()
                                    .with_value(json!(terms_without_suggestions
                                        .iter()
                                        .take(5)
                                        .map(|t| &t.1)
                                        .collect::<Vec<_>>()))
                                    .with_occurrences(terms_without_suggestions.len()),
                            )
                            .with_confidence(0.5)
                            .with_detector("OntologyValidator"),
                        );
                    }
                }

                // Report invalid ontology IDs
                if !invalid_ids.is_empty() {
                    let sample = &invalid_ids[0];
                    observations.push(
                        Observation::new(
                            ObservationType::PatternViolation,
                            Severity::Error,
                            &col.name,
                            format!(
                                "Invalid ontology IDs found ({} occurrences). IDs should match {} terms.",
                                invalid_ids.len(),
                                ontology_name
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_value(json!({
                                    "example_id": sample.1,
                                }))
                                .with_occurrences(invalid_ids.len())
                                .with_sample_rows(invalid_ids.iter().take(5).map(|t| t.0).collect()),
                        )
                        .with_confidence(0.9)
                        .with_detector("OntologyValidator"),
                    );
                }
            }
        }

        observations
    }

    fn name(&self) -> &'static str {
        "MixsComplianceValidator"
    }
}

/// Ontology column classification for validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OntologyColumnType {
    /// ENVO environmental terms (env_broad_scale, env_local_scale, env_medium)
    Envo,
    /// UBERON anatomical terms (body_site, tissue)
    Uberon,
    /// MONDO disease terms (disease, host_disease)
    Mondo,
}

/// Classify a column as an ontology column if applicable.
fn classify_ontology_column(col_name: &str) -> Option<OntologyColumnType> {
    let name = col_name.to_lowercase();

    // ENVO columns (environmental terms)
    let envo_columns = [
        "env_broad_scale",
        "env_local_scale",
        "env_medium",
        "broad_scale_environmental_context",
        "local_environmental_context",
        "environmental_medium",
        "biome",
        "environment",
        "env_biome",
        "env_feature",
        "env_material",
        "habitat",
    ];
    if envo_columns.contains(&name.as_str()) || name.contains("env_") && name.contains("scale") {
        return Some(OntologyColumnType::Envo);
    }

    // UBERON columns (anatomical terms)
    let uberon_columns = [
        "body_site",
        "tissue",
        "tissue_type",
        "body_part",
        "sample_site",
        "anatomical_site",
        "organ",
        "body_habitat",
        "isolation_source",
    ];
    if uberon_columns.contains(&name.as_str())
        || name.contains("body_site")
        || name.contains("tissue")
        || name.contains("anatomical")
    {
        return Some(OntologyColumnType::Uberon);
    }

    // MONDO columns (disease terms)
    let mondo_columns = [
        "disease",
        "diagnosis",
        "condition",
        "host_disease",
        "disease_status",
        "health_status",
        "clinical_diagnosis",
        "primary_diagnosis",
        "disease_name",
    ];
    if mondo_columns.contains(&name.as_str())
        || name.contains("disease")
        || name.contains("diagnosis")
    {
        return Some(OntologyColumnType::Mondo);
    }

    None
}

/// Check if a column name indicates taxonomy content.
///
/// This function recognizes common patterns for taxonomy/organism columns
/// in microbiome and genomics metadata.
fn is_taxonomy_column(col_name: &str) -> bool {
    let name = col_name.to_lowercase();

    // Exact matches
    let exact_matches = [
        "organism",
        "host",
        "species",
        "taxon",
        "taxonomy",
        "taxa",
        "genus",
        "phylum",
        "class",
        "order",
        "family",
        "kingdom",
        "domain",
        "scientific_name",
        "scientificname",
        "organism_name",
        "organismname",
        "host_organism",
        "host_species",
        "host_taxid",
        "tax_id",
        "taxid",
        "ncbi_taxid",
        "ncbi_taxonomy_id",
    ];

    if exact_matches.contains(&name.as_str()) {
        return true;
    }

    // Partial matches (contains)
    let partial_matches = ["organism", "species", "taxon"];
    for pattern in partial_matches {
        if name.contains(pattern) {
            return true;
        }
    }

    // Suffix matches (ends with _species, _organism, etc.)
    let suffix_matches = ["_species", "_organism", "_taxon", "_taxonomy", "_host"];
    for suffix in suffix_matches {
        if name.ends_with(suffix) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ColumnSchema;

    fn create_test_data() -> (DataTable, TableSchema) {
        let data = DataTable {
            headers: vec![
                "sample_id".to_string(),
                "organism".to_string(),
                "collection_date".to_string(),
                "lat_lon".to_string(),
            ],
            rows: vec![
                vec![
                    "S001".to_string(),
                    "E. coli".to_string(),
                    "2024-01-15".to_string(),
                    "38.98 -77.11".to_string(),
                ],
                vec![
                    "S002".to_string(),
                    "Bacteroides fragalis".to_string(), // typo
                    "Jan 15, 2024".to_string(),         // wrong format
                    "invalid".to_string(),              // invalid lat_lon
                ],
            ],
            delimiter: b'\t',
        };

        let mut schema = TableSchema::new();
        schema.columns = vec![
            ColumnSchema::new("sample_id", 0),
            ColumnSchema::new("organism", 1),
            ColumnSchema::new("collection_date", 2),
            ColumnSchema::new("lat_lon", 3),
        ];

        (data, schema)
    }

    #[test]
    fn test_mixs_validator_detects_missing_fields() {
        let (data, schema) = create_test_data();
        let validator = MixsComplianceValidator::new().with_package(MixsPackage::HumanGut);

        let observations = validator.validate(&data, &schema);

        // Should detect missing mandatory fields
        let missing_field_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == ObservationType::Completeness)
            .collect();

        assert!(!missing_field_obs.is_empty());

        // Should mention env_broad_scale or other missing fields
        let has_env_field = missing_field_obs
            .iter()
            .any(|o| o.description.contains("env_broad_scale") || o.description.contains("MIxS"));
        assert!(has_env_field);
    }

    #[test]
    fn test_mixs_validator_detects_taxonomy_issues() {
        let (data, schema) = create_test_data();
        let validator = MixsComplianceValidator::new().with_package(MixsPackage::HumanGut);

        let observations = validator.validate(&data, &schema);

        // Should detect E. coli abbreviation
        let taxonomy_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.column == "organism")
            .collect();

        assert!(!taxonomy_obs.is_empty());
    }

    #[test]
    fn test_lat_lon_validation() {
        let validator = MixsComplianceValidator::new();

        assert!(validator.validate_lat_lon("38.98 -77.11"));
        assert!(validator.validate_lat_lon("38.98N 77.11W"));
        assert!(validator.validate_lat_lon("missing"));
        assert!(validator.validate_lat_lon("not collected"));
        assert!(!validator.validate_lat_lon("invalid"));
        assert!(!validator.validate_lat_lon("abc xyz"));
    }

    #[test]
    fn test_date_validation() {
        let validator = MixsComplianceValidator::new();

        assert!(validator.validate_date_format("2024-01-15"));
        assert!(validator.validate_date_format("2024-01"));
        assert!(validator.validate_date_format("2024"));
        assert!(validator.validate_date_format("missing"));
        assert!(!validator.validate_date_format("Jan 15, 2024"));
        assert!(!validator.validate_date_format("15/01/2024"));
    }

    #[test]
    fn test_compliance_score() {
        let (data, schema) = create_test_data();
        let validator = MixsComplianceValidator::new().with_package(MixsPackage::HumanGut);

        let score = validator.compliance_score(&data, &schema);

        // Should have some compliance (has collection_date, lat_lon) but not full
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    #[test]
    fn test_is_taxonomy_column() {
        // Exact matches
        assert!(is_taxonomy_column("organism"));
        assert!(is_taxonomy_column("Organism"));
        assert!(is_taxonomy_column("host"));
        assert!(is_taxonomy_column("species"));
        assert!(is_taxonomy_column("taxid"));
        assert!(is_taxonomy_column("scientific_name"));

        // Partial matches
        assert!(is_taxonomy_column("sample_organism"));
        assert!(is_taxonomy_column("host_species"));
        assert!(is_taxonomy_column("taxon_id"));

        // Should NOT match
        assert!(!is_taxonomy_column("sample_id"));
        assert!(!is_taxonomy_column("diagnosis"));
        assert!(!is_taxonomy_column("treatment"));
        assert!(!is_taxonomy_column("age"));
    }

    #[test]
    fn test_classify_ontology_column() {
        // ENVO columns
        assert_eq!(
            classify_ontology_column("env_broad_scale"),
            Some(OntologyColumnType::Envo)
        );
        assert_eq!(
            classify_ontology_column("env_local_scale"),
            Some(OntologyColumnType::Envo)
        );
        assert_eq!(
            classify_ontology_column("env_medium"),
            Some(OntologyColumnType::Envo)
        );
        assert_eq!(
            classify_ontology_column("biome"),
            Some(OntologyColumnType::Envo)
        );
        assert_eq!(
            classify_ontology_column("habitat"),
            Some(OntologyColumnType::Envo)
        );

        // UBERON columns
        assert_eq!(
            classify_ontology_column("body_site"),
            Some(OntologyColumnType::Uberon)
        );
        assert_eq!(
            classify_ontology_column("tissue"),
            Some(OntologyColumnType::Uberon)
        );
        assert_eq!(
            classify_ontology_column("tissue_type"),
            Some(OntologyColumnType::Uberon)
        );
        assert_eq!(
            classify_ontology_column("isolation_source"),
            Some(OntologyColumnType::Uberon)
        );

        // MONDO columns
        assert_eq!(
            classify_ontology_column("disease"),
            Some(OntologyColumnType::Mondo)
        );
        assert_eq!(
            classify_ontology_column("diagnosis"),
            Some(OntologyColumnType::Mondo)
        );
        assert_eq!(
            classify_ontology_column("host_disease"),
            Some(OntologyColumnType::Mondo)
        );
        assert_eq!(
            classify_ontology_column("clinical_diagnosis"),
            Some(OntologyColumnType::Mondo)
        );

        // Should NOT match any ontology
        assert_eq!(classify_ontology_column("sample_id"), None);
        assert_eq!(classify_ontology_column("age"), None);
        assert_eq!(classify_ontology_column("organism"), None);
    }

    #[test]
    fn test_mixs_validator_detects_ontology_columns() {
        let data = DataTable {
            headers: vec![
                "sample_id".to_string(),
                "diagnosis".to_string(),
                "body_site".to_string(),
                "env_broad_scale".to_string(),
            ],
            rows: vec![
                vec![
                    "S001".to_string(),
                    "Crohn's disease".to_string(),
                    "gut".to_string(),
                    "human-associated habitat".to_string(),
                ],
                vec![
                    "S002".to_string(),
                    "ulcerative colitis".to_string(),
                    "intestine".to_string(),
                    "ENVO:00002030".to_string(), // valid ENVO ID
                ],
            ],
            delimiter: b'\t',
        };

        let mut schema = TableSchema::new();
        schema.columns = vec![
            ColumnSchema::new("sample_id", 0),
            ColumnSchema::new("diagnosis", 1),
            ColumnSchema::new("body_site", 2),
            ColumnSchema::new("env_broad_scale", 3),
        ];

        let validator = MixsComplianceValidator::new().with_package(MixsPackage::HumanGut);
        let observations = validator.validate(&data, &schema);

        // Should have ontology-related observations
        let ontology_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.detector == "OntologyValidator")
            .collect();

        // Should detect disease terms for mapping
        let disease_obs = ontology_obs
            .iter()
            .any(|o| o.column == "diagnosis");
        assert!(disease_obs, "Should detect diagnosis column for ontology mapping");

        // Should detect body_site terms for mapping
        let body_site_obs = ontology_obs
            .iter()
            .any(|o| o.column == "body_site");
        assert!(body_site_obs, "Should detect body_site column for ontology mapping");
    }
}
