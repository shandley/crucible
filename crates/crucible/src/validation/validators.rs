//! Validators for checking data against inferred schema.

use indexmap::IndexMap;
use serde_json::json;

use crate::input::DataTable;
use crate::schema::{ColumnSchema, ColumnType, Constraint, SemanticRole, TableSchema};

use super::observation::{Evidence, Observation, ObservationType, Severity};

/// Trait for validators.
pub trait Validator {
    /// Run validation and return observations.
    fn validate(
        &self,
        table: &DataTable,
        schema: &TableSchema,
    ) -> Vec<Observation>;
}

/// Validates that values match their inferred type.
pub struct TypeValidator;

impl Validator for TypeValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            let mismatches = self.find_type_mismatches(table, col_schema);
            if !mismatches.is_empty() {
                let count = mismatches.len();
                let pct = (count as f64 / table.row_count() as f64) * 100.0;

                let obs = Observation::new(
                    ObservationType::TypeMismatch,
                    if pct > 10.0 { Severity::Error } else { Severity::Warning },
                    &col_schema.name,
                    format!(
                        "{} values ({:.1}%) don't match expected type {:?}",
                        count, pct, col_schema.inferred_type
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(mismatches.into_iter().take(5).collect())
                        .with_expected(json!(format!("{:?}", col_schema.inferred_type))),
                )
                .with_confidence(0.9)
                .with_detector("type_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl TypeValidator {
    fn find_type_mismatches(&self, table: &DataTable, col_schema: &ColumnSchema) -> Vec<usize> {
        let mut mismatches = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            let matches = match col_schema.inferred_type {
                ColumnType::Integer => value.trim().parse::<i64>().is_ok(),
                ColumnType::Float => value.trim().parse::<f64>().is_ok(),
                ColumnType::Boolean => matches!(
                    value.trim().to_lowercase().as_str(),
                    "true" | "false" | "yes" | "no" | "t" | "f" | "y" | "n" | "1" | "0"
                ),
                ColumnType::String => true, // Strings always match
                _ => true,
            };

            if !matches {
                mismatches.push(row_idx);
            }
        }

        mismatches
    }
}

/// Validates that numeric values are within expected range.
pub struct RangeValidator;

impl Validator for RangeValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            if !col_schema.inferred_type.is_numeric() {
                continue;
            }

            // Find range constraint
            let range_constraint = col_schema.constraints.iter().find_map(|c| {
                if let Constraint::Range { min, max, .. } = c {
                    Some((*min, *max))
                } else {
                    None
                }
            });

            if let Some((min, max)) = range_constraint {
                let out_of_range = self.find_out_of_range(table, col_schema, min, max);
                if !out_of_range.is_empty() {
                    let count = out_of_range.len();
                    let pct = (count as f64 / table.row_count() as f64) * 100.0;

                    let obs = Observation::new(
                        ObservationType::Outlier,
                        Severity::Info,
                        &col_schema.name,
                        format!(
                            "{} values outside expected range [{}, {}]",
                            count,
                            min.map(|v| v.to_string()).unwrap_or("∞".to_string()),
                            max.map(|v| v.to_string()).unwrap_or("∞".to_string())
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(count)
                            .with_percentage(pct)
                            .with_sample_rows(out_of_range.into_iter().take(5).collect())
                            .with_expected(json!({
                                "min": min,
                                "max": max
                            })),
                    )
                    .with_confidence(0.85)
                    .with_detector("range_validator");

                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl RangeValidator {
    fn find_out_of_range(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
        min: Option<f64>,
        max: Option<f64>,
    ) -> Vec<usize> {
        let mut out_of_range = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            if let Ok(num) = value.trim().parse::<f64>() {
                let below_min = min.map(|m| num < m).unwrap_or(false);
                let above_max = max.map(|m| num > m).unwrap_or(false);
                if below_min || above_max {
                    out_of_range.push(row_idx);
                }
            }
        }

        out_of_range
    }
}

/// Validates that categorical values are in the expected set.
pub struct SetValidator;

impl Validator for SetValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Find set membership constraint
            let expected_values = col_schema.constraints.iter().find_map(|c| {
                if let Constraint::SetMembership { values, .. } = c {
                    Some(values.clone())
                } else {
                    None
                }
            });

            if let Some(expected) = expected_values {
                let unexpected = self.find_unexpected_values(table, col_schema, &expected);
                if !unexpected.is_empty() {
                    let unique_unexpected: Vec<_> = unexpected.iter().map(|(_, v)| v.clone()).collect();
                    let count: usize = unexpected.len();
                    let pct = (count as f64 / table.row_count() as f64) * 100.0;

                    let obs = Observation::new(
                        ObservationType::ConstraintViolation,
                        Severity::Warning,
                        &col_schema.name,
                        format!(
                            "{} values not in expected set: {:?}",
                            count,
                            unique_unexpected.iter().take(3).collect::<Vec<_>>()
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(count)
                            .with_percentage(pct)
                            .with_sample_rows(unexpected.iter().take(5).map(|(r, _)| *r).collect())
                            .with_expected(json!(expected)),
                    )
                    .with_confidence(0.85)
                    .with_detector("set_validator");

                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl SetValidator {
    fn find_unexpected_values(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
        expected: &[String],
    ) -> Vec<(usize, String)> {
        let mut unexpected = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            let trimmed = value.trim();
            if !expected.iter().any(|e| e == trimmed) {
                unexpected.push((row_idx, trimmed.to_string()));
            }
        }

        unexpected
    }
}

/// Validates uniqueness constraints.
pub struct UniquenessValidator;

impl Validator for UniquenessValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Check if column should be unique
            let should_be_unique = col_schema
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Unique { .. }));

            if should_be_unique {
                let duplicates = self.find_duplicates(table, col_schema);
                if !duplicates.is_empty() {
                    let count: usize = duplicates.values().map(|v| v.len() - 1).sum();
                    let pct = (count as f64 / table.row_count() as f64) * 100.0;

                    let obs = Observation::new(
                        ObservationType::Duplicate,
                        Severity::Error,
                        &col_schema.name,
                        format!(
                            "{} duplicate values found in column that should be unique",
                            count
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(count)
                            .with_percentage(pct)
                            .with_value_counts(Some(json!(
                                duplicates
                                    .iter()
                                    .take(5)
                                    .map(|(k, v)| (k.clone(), v.len()))
                                    .collect::<IndexMap<_, _>>()
                            ))),
                    )
                    .with_confidence(0.95)
                    .with_detector("uniqueness_validator");

                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl UniquenessValidator {
    fn find_duplicates(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, Vec<usize>> {
        let mut value_rows: IndexMap<String, Vec<usize>> = IndexMap::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            value_rows
                .entry(value.to_string())
                .or_default()
                .push(row_idx);
        }

        // Keep only duplicates
        value_rows.retain(|_, rows| rows.len() > 1);
        value_rows
    }
}

/// Validates completeness (missing value patterns).
pub struct CompletenessValidator {
    /// Threshold for warning about missing values (percentage).
    warning_threshold: f64,
    /// Threshold for error about missing values (percentage).
    error_threshold: f64,
}

impl Default for CompletenessValidator {
    fn default() -> Self {
        Self {
            warning_threshold: 5.0,
            error_threshold: 20.0,
        }
    }
}

impl Validator for CompletenessValidator {
    fn validate(&self, _table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            let null_pct = col_schema.null_percentage();

            // Check for high missing rate
            if null_pct >= self.warning_threshold {
                let severity = if null_pct >= self.error_threshold {
                    Severity::Error
                } else {
                    Severity::Warning
                };

                let obs = Observation::new(
                    ObservationType::Completeness,
                    severity,
                    &col_schema.name,
                    format!("{:.1}% of values are missing", null_pct),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(col_schema.statistics.null_count)
                        .with_percentage(null_pct),
                )
                .with_confidence(0.95)
                .with_detector("completeness_validator");

                observations.push(obs);
            }

            // Check for non-standard NA patterns
            // This is done during inference, but we can report it here
        }

        observations
    }
}

/// Validates for inconsistencies (case variations, format variations).
pub struct ConsistencyValidator;

impl Validator for ConsistencyValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Check for case inconsistencies in categorical columns
            if let Some(ref expected) = col_schema.expected_values {
                let case_issues = self.find_case_inconsistencies(table, col_schema, expected);
                if !case_issues.is_empty() {
                    let count = case_issues.len();
                    let pct = (count as f64 / table.row_count() as f64) * 100.0;

                    let obs = Observation::new(
                        ObservationType::Inconsistency,
                        Severity::Warning,
                        &col_schema.name,
                        format!(
                            "Case inconsistencies detected: {:?}",
                            case_issues.keys().take(3).collect::<Vec<_>>()
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(count)
                            .with_percentage(pct)
                            .with_value_counts(Some(json!(case_issues))),
                    )
                    .with_confidence(0.88)
                    .with_detector("consistency_validator");

                    observations.push(obs);
                }
            }

            // Check for boolean inconsistencies
            if col_schema.inferred_type == ColumnType::Boolean {
                let bool_variants = self.find_boolean_variants(table, col_schema);
                if bool_variants.len() > 2 {
                    let obs = Observation::new(
                        ObservationType::Inconsistency,
                        Severity::Warning,
                        &col_schema.name,
                        format!(
                            "Mixed boolean representations: {:?}",
                            bool_variants.keys().collect::<Vec<_>>()
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_value_counts(Some(json!(bool_variants))),
                    )
                    .with_confidence(0.92)
                    .with_detector("consistency_validator");

                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl ConsistencyValidator {
    fn find_case_inconsistencies(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
        expected: &[String],
    ) -> IndexMap<String, usize> {
        let mut variants: IndexMap<String, usize> = IndexMap::new();
        let expected_lower: Vec<String> = expected.iter().map(|s| s.to_lowercase()).collect();

        for value in table.column_values(col_schema.position) {
            if DataTable::is_null_value(value) {
                continue;
            }

            let trimmed = value.trim();
            let lower = trimmed.to_lowercase();

            // Check if it matches an expected value case-insensitively but not exactly
            if expected_lower.contains(&lower) && !expected.contains(&trimmed.to_string()) {
                *variants.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }

        variants
    }

    fn find_boolean_variants(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, usize> {
        let mut variants: IndexMap<String, usize> = IndexMap::new();

        for value in table.column_values(col_schema.position) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                *variants.entry(trimmed.to_string()).or_insert(0) += 1;
            }
        }

        variants
    }
}

/// Validates for non-standard missing value patterns.
pub struct MissingPatternValidator {
    /// Common patterns that represent missing values.
    patterns: Vec<&'static str>,
}

impl Default for MissingPatternValidator {
    fn default() -> Self {
        Self {
            patterns: vec![
                "missing", "unknown", "not available", "not recorded",
                "n.a.", "n.a", "na.", "#n/a", "#null", "undefined",
                "-999", "-9999", "999", "9999",
            ],
        }
    }
}

impl Validator for MissingPatternValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            let patterns_found = self.find_missing_patterns(table, col_schema);

            for (pattern, count) in patterns_found {
                let pct = (count as f64 / table.row_count() as f64) * 100.0;

                let obs = Observation::new(
                    ObservationType::MissingPattern,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "String '{}' appears to represent NA values ({} occurrences)",
                        pattern, count
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_pattern(&pattern)
                        .with_occurrences(count)
                        .with_percentage(pct),
                )
                .with_confidence(0.88)
                .with_detector("missing_pattern_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl MissingPatternValidator {
    fn find_missing_patterns(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Vec<(String, usize)> {
        let mut pattern_counts: IndexMap<String, usize> = IndexMap::new();

        for value in table.column_values(col_schema.position) {
            let lower = value.trim().to_lowercase();
            for &pattern in &self.patterns {
                if lower == pattern {
                    *pattern_counts.entry(pattern.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Only report patterns that appear multiple times
        pattern_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .collect()
    }
}

/// Validates identifier columns for duplicates.
/// This catches cases where sample_id, patient_id etc. have duplicate values.
pub struct IdentifierDuplicateValidator;

impl Validator for IdentifierDuplicateValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        use crate::schema::SemanticRole;
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Check identifier columns that aren't already marked as unique
            // (UniquenessValidator handles columns with Unique constraint)
            if col_schema.semantic_role == SemanticRole::Identifier && !col_schema.unique {
                let duplicates = self.find_duplicates(table, col_schema);
                if !duplicates.is_empty() {
                    let dup_count: usize = duplicates.values().map(|v| v.len() - 1).sum();
                    let pct = (dup_count as f64 / table.row_count() as f64) * 100.0;

                    // Get sample duplicate values for the message
                    let sample_dups: Vec<_> = duplicates.keys().take(3).cloned().collect();

                    let obs = Observation::new(
                        ObservationType::Duplicate,
                        Severity::Error,
                        &col_schema.name,
                        format!(
                            "Identifier column has {} duplicate values: {:?}",
                            dup_count, sample_dups
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(dup_count)
                            .with_percentage(pct)
                            .with_value_counts(Some(json!(
                                duplicates
                                    .iter()
                                    .take(5)
                                    .map(|(k, v)| (k.clone(), v.len()))
                                    .collect::<IndexMap<_, _>>()
                            ))),
                    )
                    .with_confidence(0.95)
                    .with_detector("identifier_duplicate_validator");

                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl IdentifierDuplicateValidator {
    fn find_duplicates(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, Vec<usize>> {
        let mut value_rows: IndexMap<String, Vec<usize>> = IndexMap::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            value_rows
                .entry(value.trim().to_string())
                .or_default()
                .push(row_idx);
        }

        // Keep only duplicates
        value_rows.retain(|_, rows| rows.len() > 1);
        value_rows
    }
}

/// Validates for statistical outliers using IQR method and domain knowledge.
pub struct StatisticalOutlierValidator {
    /// IQR multiplier for outlier detection (typically 1.5 for mild, 3.0 for extreme).
    iqr_multiplier: f64,
}

impl Default for StatisticalOutlierValidator {
    fn default() -> Self {
        Self {
            iqr_multiplier: 1.5,
        }
    }
}

impl Validator for StatisticalOutlierValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            if !col_schema.inferred_type.is_numeric() {
                continue;
            }

            // Check for statistical outliers using IQR
            if let Some(ref numeric_stats) = col_schema.statistics.numeric {
                let outliers = self.find_iqr_outliers(table, col_schema, numeric_stats);
                if !outliers.is_empty() {
                    let count = outliers.len();
                    let pct = (count as f64 / table.row_count() as f64) * 100.0;

                    // Get actual outlier values for display
                    let outlier_values: Vec<f64> = outliers
                        .iter()
                        .take(5)
                        .filter_map(|(_, v)| v.parse::<f64>().ok())
                        .collect();

                    let obs = Observation::new(
                        ObservationType::Outlier,
                        if count > 1 { Severity::Warning } else { Severity::Info },
                        &col_schema.name,
                        format!(
                            "{} statistical outlier(s) detected (IQR method): {:?}",
                            count, outlier_values
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(count)
                            .with_percentage(pct)
                            .with_sample_rows(outliers.iter().map(|(r, _)| *r).take(5).collect())
                            .with_expected(json!({
                                "q1": numeric_stats.q1,
                                "q3": numeric_stats.q3,
                                "iqr": numeric_stats.iqr(),
                                "lower_bound": numeric_stats.q1 - self.iqr_multiplier * numeric_stats.iqr(),
                                "upper_bound": numeric_stats.q3 + self.iqr_multiplier * numeric_stats.iqr()
                            })),
                    )
                    .with_confidence(0.85)
                    .with_detector("statistical_outlier_validator");

                    observations.push(obs);
                }
            }

            // Check for domain-specific invalid values (negative ages, weights, counts)
            let negative_issues = self.find_invalid_negative_values(table, col_schema);
            if !negative_issues.is_empty() {
                let count = negative_issues.len();
                let pct = (count as f64 / table.row_count() as f64) * 100.0;

                let obs = Observation::new(
                    ObservationType::Outlier,
                    Severity::Error,
                    &col_schema.name,
                    format!(
                        "{} impossible negative value(s) in column '{}' (should be non-negative)",
                        count, col_schema.name
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(negative_issues.iter().map(|(r, _)| *r).take(5).collect())
                        .with_expected(json!({"min": 0})),
                )
                .with_confidence(0.95)
                .with_detector("statistical_outlier_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl StatisticalOutlierValidator {
    fn find_iqr_outliers(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
        stats: &crate::schema::NumericStatistics,
    ) -> Vec<(usize, String)> {
        let mut outliers = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            if let Ok(num) = value.trim().parse::<f64>() {
                if stats.is_outlier_iqr(num, self.iqr_multiplier) {
                    outliers.push((row_idx, value.to_string()));
                }
            }
        }

        outliers
    }

    fn find_invalid_negative_values(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Vec<(usize, f64)> {
        // Column names that should never be negative
        let non_negative_patterns = [
            "age", "weight", "height", "bmi", "count", "reads", "read_count",
            "concentration", "conc", "length", "size", "distance", "duration",
            "price", "cost", "amount", "quantity", "score", "rating",
        ];

        let col_lower = col_schema.name.to_lowercase();
        let should_be_non_negative = non_negative_patterns
            .iter()
            .any(|p| col_lower.contains(p));

        if !should_be_non_negative {
            return Vec::new();
        }

        let mut negatives = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            if let Ok(num) = value.trim().parse::<f64>() {
                if num < 0.0 {
                    negatives.push((row_idx, num));
                }
            }
        }

        negatives
    }
}

/// Validates for case variant inconsistencies.
/// Detects when the same value appears in different cases (e.g., "CD" and "cd").
pub struct CaseVariantValidator;

impl Validator for CaseVariantValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Only check string/categorical columns
            if col_schema.inferred_type != ColumnType::String {
                continue;
            }

            let case_groups = self.find_case_variant_groups(table, col_schema);

            // Filter to groups that have multiple case variants
            let problematic_groups: Vec<_> = case_groups
                .into_iter()
                .filter(|(_, variants)| variants.len() > 1)
                .collect();

            if !problematic_groups.is_empty() {
                let total_affected: usize = problematic_groups
                    .iter()
                    .flat_map(|(_, variants)| variants.values())
                    .sum();
                let pct = (total_affected as f64 / table.row_count() as f64) * 100.0;

                // Create a description of the case variants
                let variant_examples: Vec<String> = problematic_groups
                    .iter()
                    .take(3)
                    .map(|(_, variants)| {
                        let vars: Vec<_> = variants.keys().take(3).cloned().collect();
                        format!("{:?}", vars)
                    })
                    .collect();

                let obs = Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "Case variants detected for {} value(s): {}",
                        problematic_groups.len(),
                        variant_examples.join(", ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total_affected)
                        .with_percentage(pct)
                        .with_value_counts(Some(json!(
                            problematic_groups
                                .iter()
                                .take(5)
                                .map(|(canonical, variants)| {
                                    (canonical.clone(), variants.clone())
                                })
                                .collect::<IndexMap<_, _>>()
                        ))),
                )
                .with_confidence(0.90)
                .with_detector("case_variant_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl CaseVariantValidator {
    /// Groups values by their lowercase form to find case variants.
    fn find_case_variant_groups(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, IndexMap<String, usize>> {
        // Map from lowercase -> (original_case -> count)
        let mut groups: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();

        for value in table.column_values(col_schema.position) {
            if DataTable::is_null_value(value) {
                continue;
            }

            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }

            let lowercase = trimmed.to_lowercase();

            groups
                .entry(lowercase)
                .or_default()
                .entry(trimmed.to_string())
                .and_modify(|c| *c += 1)
                .or_insert(1);
        }

        groups
    }
}

/// Validates for potential typos using edit distance.
/// Detects when values are very similar to other values (e.g., "stoool" vs "stool").
pub struct TypoValidator {
    /// Maximum edit distance to consider as a typo (default: 2).
    max_distance: usize,
    /// Minimum string length to check (very short strings have too many false positives).
    min_length: usize,
}

impl Default for TypoValidator {
    fn default() -> Self {
        Self {
            max_distance: 2,
            min_length: 4,
        }
    }
}

impl Validator for TypoValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Only check string columns with expected values (categorical)
            if col_schema.inferred_type != ColumnType::String {
                continue;
            }

            // Skip identifier columns - they're meant to be unique, not typos of each other
            if col_schema.semantic_role == SemanticRole::Identifier {
                continue;
            }

            let potential_typos = self.find_potential_typos(table, col_schema);

            if !potential_typos.is_empty() {
                let count: usize = potential_typos.values().map(|(_, c)| c).sum();
                let pct = (count as f64 / table.row_count() as f64) * 100.0;

                // Format typo suggestions
                let typo_examples: Vec<String> = potential_typos
                    .iter()
                    .take(3)
                    .map(|(typo, (suggestion, _))| format!("'{}' → '{}'", typo, suggestion))
                    .collect();

                let obs = Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "{} potential typo(s) detected: {}",
                        potential_typos.len(),
                        typo_examples.join(", ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_value_counts(Some(json!(
                            potential_typos
                                .iter()
                                .map(|(typo, (suggestion, count))| {
                                    (typo.clone(), json!({"suggestion": suggestion, "count": count}))
                                })
                                .collect::<IndexMap<_, _>>()
                        ))),
                )
                .with_confidence(0.75)
                .with_detector("typo_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl TypoValidator {
    /// Find values that appear to be typos of more common values.
    fn find_potential_typos(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, (String, usize)> {
        // Count all values
        let mut value_counts: IndexMap<String, usize> = IndexMap::new();
        for value in table.column_values(col_schema.position) {
            if DataTable::is_null_value(value) {
                continue;
            }
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                *value_counts.entry(trimmed).or_insert(0) += 1;
            }
        }

        // Find rare values that are close to common values
        let mut potential_typos: IndexMap<String, (String, usize)> = IndexMap::new();

        // Sort by count descending to identify "canonical" values
        let mut sorted_values: Vec<_> = value_counts.iter().collect();
        sorted_values.sort_by(|a, b| b.1.cmp(a.1));

        // Common values are those appearing more than once
        let common_values: Vec<&String> = sorted_values
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(val, _)| *val)
            .collect();

        // Check rare values (count == 1) against common values
        for (rare_value, count) in &value_counts {
            if *count > 1 || rare_value.len() < self.min_length {
                continue;
            }

            for common_value in &common_values {
                if common_value.len() < self.min_length {
                    continue;
                }

                let distance = levenshtein_distance(rare_value, common_value);

                // Only flag if distance is small relative to string length
                // and the strings are reasonably similar
                if distance > 0
                    && distance <= self.max_distance
                    && distance < rare_value.len() / 2
                {
                    potential_typos.insert(
                        rare_value.clone(),
                        ((*common_value).clone(), *count),
                    );
                    break; // Only suggest one correction per typo
                }
            }
        }

        potential_typos
    }
}

/// Calculate Levenshtein (edit) distance between two strings.
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    // Early exits
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Create distance matrix
    let mut matrix: Vec<Vec<usize>> = vec![vec![0; len2 + 1]; len1 + 1];

    // Initialize first column
    for i in 0..=len1 {
        matrix[i][0] = i;
    }

    // Initialize first row
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    // Fill in the rest of the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = (matrix[i - 1][j] + 1) // deletion
                .min(matrix[i][j - 1] + 1) // insertion
                .min(matrix[i - 1][j - 1] + cost); // substitution
        }
    }

    matrix[len1][len2]
}

/// Validates for semantic equivalents (synonyms).
/// Detects when values represent the same concept with different names.
pub struct SemanticEquivalenceValidator {
    /// Known synonym groups for biomedical terms.
    synonym_groups: Vec<Vec<&'static str>>,
}

impl Default for SemanticEquivalenceValidator {
    fn default() -> Self {
        Self {
            synonym_groups: vec![
                // Disease names
                vec!["CD", "Crohn's", "Crohns", "Crohn's Disease", "Crohn Disease"],
                vec!["UC", "Ulcerative Colitis", "ulcerative colitis"],
                vec!["IBD", "Inflammatory Bowel Disease"],
                vec!["T2D", "Type 2 Diabetes", "Type II Diabetes", "Diabetes Mellitus Type 2"],
                vec!["T1D", "Type 1 Diabetes", "Type I Diabetes", "Diabetes Mellitus Type 1"],
                vec!["HTN", "Hypertension", "High Blood Pressure"],
                vec!["MI", "Myocardial Infarction", "Heart Attack"],
                vec!["CVD", "Cardiovascular Disease"],
                vec!["COPD", "Chronic Obstructive Pulmonary Disease"],
                vec!["CKD", "Chronic Kidney Disease"],
                vec!["HF", "Heart Failure", "CHF", "Congestive Heart Failure"],

                // Sample types
                vec!["stool", "feces", "fecal", "faeces", "faecal"],
                vec!["gut", "intestine", "intestinal", "GI", "gastrointestinal"],
                vec!["blood", "serum", "plasma"],
                vec!["urine", "urinary"],
                vec!["saliva", "oral", "buccal"],
                vec!["biopsy", "tissue"],

                // Sex/Gender
                vec!["M", "Male", "male", "man", "Man"],
                vec!["F", "Female", "female", "woman", "Woman"],

                // Boolean-like
                vec!["yes", "Yes", "YES", "Y", "y", "true", "True", "TRUE", "1"],
                vec!["no", "No", "NO", "N", "n", "false", "False", "FALSE", "0"],

                // Treatment status
                vec!["control", "Control", "healthy", "Healthy", "normal", "Normal"],
                vec!["treated", "treatment", "Treatment", "drug", "Drug"],
                vec!["placebo", "Placebo", "vehicle", "Vehicle"],

                // Response status
                vec!["responder", "Responder", "response", "Response", "R"],
                vec!["non-responder", "Non-responder", "nonresponder", "Nonresponder", "NR", "no response"],
                vec!["partial", "Partial", "partial response", "PR"],

                // Severity
                vec!["mild", "Mild", "low", "Low"],
                vec!["moderate", "Moderate", "medium", "Medium"],
                vec!["severe", "Severe", "high", "High"],

                // Activity status
                vec!["active", "Active", "flare", "Flare"],
                vec!["inactive", "Inactive", "remission", "Remission", "quiescent"],

                // Smoking status
                vec!["never", "Never", "non-smoker", "Non-smoker", "nonsmoker"],
                vec!["former", "Former", "ex-smoker", "Ex-smoker", "past"],
                vec!["current", "Current", "smoker", "Smoker", "active smoker"],

                // Treatment types
                vec!["chemotherapy", "Chemotherapy", "chemo", "Chemo"],
                vec!["radiation", "Radiation", "radiotherapy", "Radiotherapy", "RT"],
                vec!["surgery", "Surgery", "surgical", "Surgical"],
                vec!["immunotherapy", "Immunotherapy", "immuno"],
            ],
        }
    }
}

impl Validator for SemanticEquivalenceValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Only check string columns
            if col_schema.inferred_type != ColumnType::String {
                continue;
            }

            let equivalent_groups = self.find_semantic_equivalents(table, col_schema);

            if !equivalent_groups.is_empty() {
                let total_affected: usize = equivalent_groups
                    .iter()
                    .flat_map(|(_, variants)| variants.values())
                    .sum();
                let pct = (total_affected as f64 / table.row_count() as f64) * 100.0;

                // Format examples
                let examples: Vec<String> = equivalent_groups
                    .iter()
                    .take(3)
                    .map(|(canonical, variants)| {
                        let vars: Vec<_> = variants.keys().take(3).cloned().collect();
                        format!("{:?} (same as '{}')", vars, canonical)
                    })
                    .collect();

                let obs = Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "{} semantic equivalent group(s) detected: {}",
                        equivalent_groups.len(),
                        examples.join("; ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total_affected)
                        .with_percentage(pct)
                        .with_value_counts(Some(json!(
                            equivalent_groups
                                .iter()
                                .take(5)
                                .map(|(canonical, variants)| {
                                    (canonical.clone(), variants.clone())
                                })
                                .collect::<IndexMap<_, _>>()
                        ))),
                )
                .with_confidence(0.85)
                .with_detector("semantic_equivalence_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

impl SemanticEquivalenceValidator {
    /// Find values that are semantic equivalents (synonyms).
    fn find_semantic_equivalents(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> IndexMap<String, IndexMap<String, usize>> {
        // Collect all unique values with counts
        let mut value_counts: IndexMap<String, usize> = IndexMap::new();
        for value in table.column_values(col_schema.position) {
            if DataTable::is_null_value(value) {
                continue;
            }
            let trimmed = value.trim().to_string();
            if !trimmed.is_empty() {
                *value_counts.entry(trimmed).or_insert(0) += 1;
            }
        }

        // Map values to their canonical form
        let mut canonical_map: IndexMap<String, String> = IndexMap::new();
        for value in value_counts.keys() {
            for group in &self.synonym_groups {
                // Case-insensitive matching
                let value_lower = value.to_lowercase();
                for &synonym in group {
                    if value_lower == synonym.to_lowercase() {
                        // Use the first item in the group as canonical
                        canonical_map.insert(value.clone(), group[0].to_string());
                        break;
                    }
                }
                if canonical_map.contains_key(value) {
                    break;
                }
            }
        }

        // Group values by canonical form
        let mut groups: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
        for (value, count) in &value_counts {
            if let Some(canonical) = canonical_map.get(value) {
                groups
                    .entry(canonical.clone())
                    .or_default()
                    .insert(value.clone(), *count);
            }
        }

        // Only keep groups with multiple variants
        groups.retain(|_, variants| variants.len() > 1);

        groups
    }
}

/// Validates date columns for format inconsistencies.
///
/// Detects when dates in the same column use different formats
/// (e.g., "2024-01-15" vs "01/15/2024" vs "Jan 15 2024").
pub struct DateFormatValidator;

/// Known date format patterns with their regex and description.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
enum DateFormat {
    /// ISO format: 2024-01-15
    Iso,
    /// US format with slashes: 01/15/2024
    UsSlash,
    /// US format with dashes: 01-15-2024
    UsDash,
    /// European format with slashes: 15/01/2024
    EuSlash,
    /// European format with dashes: 15-01-2024
    EuDash,
    /// Month name format: Jan 15 2024 or January 15, 2024
    MonthName,
    /// Year/month/day with slashes: 2024/01/15
    YearSlash,
    /// Unknown format
    Unknown,
}

impl DateFormat {
    fn description(&self) -> &'static str {
        match self {
            DateFormat::Iso => "ISO (YYYY-MM-DD)",
            DateFormat::UsSlash => "US (MM/DD/YYYY)",
            DateFormat::UsDash => "US (MM-DD-YYYY)",
            DateFormat::EuSlash => "EU (DD/MM/YYYY)",
            DateFormat::EuDash => "EU (DD-MM-YYYY)",
            DateFormat::MonthName => "Month name (Mon DD YYYY)",
            DateFormat::YearSlash => "Year first (YYYY/MM/DD)",
            DateFormat::Unknown => "Unknown",
        }
    }
}

impl DateFormatValidator {
    /// Detect the date format of a string value.
    fn detect_format(value: &str) -> Option<DateFormat> {
        let trimmed = value.trim();

        // ISO format: 2024-01-15
        if Self::matches_iso(trimmed) {
            return Some(DateFormat::Iso);
        }

        // Year/month/day with slashes: 2024/01/15
        if Self::matches_year_slash(trimmed) {
            return Some(DateFormat::YearSlash);
        }

        // US format with slashes: 01/15/2024 or 1/15/2024
        if Self::matches_us_slash(trimmed) {
            return Some(DateFormat::UsSlash);
        }

        // US format with dashes: 01-15-2024
        if Self::matches_us_dash(trimmed) {
            return Some(DateFormat::UsDash);
        }

        // Month name format: Jan 15 2024, January 15, 2024, etc.
        if Self::matches_month_name(trimmed) {
            return Some(DateFormat::MonthName);
        }

        None
    }

    fn matches_iso(s: &str) -> bool {
        // YYYY-MM-DD
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        parts[0].len() == 4
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].len() <= 2
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() <= 2
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    fn matches_year_slash(s: &str) -> bool {
        // YYYY/MM/DD
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return false;
        }
        parts[0].len() == 4
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].len() <= 2
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() <= 2
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    fn matches_us_slash(s: &str) -> bool {
        // MM/DD/YYYY
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return false;
        }
        parts[0].len() <= 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].len() <= 2
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 4
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    fn matches_us_dash(s: &str) -> bool {
        // MM-DD-YYYY (non-ISO with year at end)
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return false;
        }
        parts[0].len() <= 2
            && parts[0].chars().all(|c| c.is_ascii_digit())
            && parts[1].len() <= 2
            && parts[1].chars().all(|c| c.is_ascii_digit())
            && parts[2].len() == 4
            && parts[2].chars().all(|c| c.is_ascii_digit())
    }

    fn matches_month_name(s: &str) -> bool {
        let months = [
            "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
            "january", "february", "march", "april", "june", "july", "august", "september",
            "october", "november", "december",
        ];

        let lower = s.to_lowercase();
        // Remove commas and extra spaces
        let cleaned: String = lower
            .chars()
            .filter(|c| !(*c == ','))
            .collect();

        for month in &months {
            if cleaned.contains(month) {
                // Check if it also has a year (4 digits)
                if cleaned.chars().filter(|c| c.is_ascii_digit()).count() >= 4 {
                    return true;
                }
            }
        }
        false
    }
}

impl Validator for DateFormatValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Only check date columns
            if col_schema.inferred_type != ColumnType::Date {
                continue;
            }

            // Count occurrences of each format
            let mut format_counts: IndexMap<DateFormat, (usize, Vec<String>)> = IndexMap::new();
            let mut rows_with_format: IndexMap<DateFormat, Vec<usize>> = IndexMap::new();

            for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
                if DataTable::is_null_value(value) {
                    continue;
                }

                if let Some(format) = Self::detect_format(value) {
                    let entry = format_counts.entry(format.clone()).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 3 {
                        entry.1.push(value.to_string());
                    }
                    rows_with_format
                        .entry(format)
                        .or_default()
                        .push(row_idx);
                }
            }

            // If more than one format detected, create an observation
            if format_counts.len() > 1 {
                let total: usize = format_counts.values().map(|(c, _)| c).sum();
                let pct = 100.0; // All dates have format issues if inconsistent

                // Format examples
                let format_examples: Vec<String> = format_counts
                    .iter()
                    .map(|(fmt, (count, examples))| {
                        format!(
                            "{}: {} ({} values)",
                            fmt.description(),
                            examples.first().unwrap_or(&String::new()),
                            count
                        )
                    })
                    .collect();

                // Recommend the most common format
                let most_common = format_counts
                    .iter()
                    .max_by_key(|(_, (count, _))| *count)
                    .map(|(fmt, _)| fmt.description())
                    .unwrap_or("ISO (YYYY-MM-DD)");

                // Get sample rows for non-dominant formats
                let dominant_format = format_counts
                    .iter()
                    .max_by_key(|(_, (count, _))| *count)
                    .map(|(fmt, _)| fmt.clone());

                let sample_rows: Vec<usize> = rows_with_format
                    .iter()
                    .filter(|(fmt, _)| Some(*fmt) != dominant_format.as_ref())
                    .flat_map(|(_, rows)| rows.iter().take(3).map(|r| r + 1)) // 1-indexed
                    .take(5)
                    .collect();

                let obs = Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "Mixed date formats detected ({}). Recommend standardizing to {}",
                        format_examples.join("; "),
                        most_common
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total)
                        .with_percentage(pct)
                        .with_sample_rows(sample_rows)
                        .with_value_counts(Some(json!(
                            format_counts
                                .iter()
                                .map(|(fmt, (count, examples))| {
                                    (
                                        fmt.description().to_string(),
                                        json!({"count": count, "examples": examples})
                                    )
                                })
                                .collect::<IndexMap<_, _>>()
                        ))),
                )
                .with_confidence(0.90)
                .with_detector("date_format_validator");

                observations.push(obs);
            }
        }

        observations
    }
}

// ============================================================================
// Regex Pattern Validator
// ============================================================================

/// Common patterns to check for based on column name or content.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)] // Variants kept for future pattern matching
enum PatternType {
    /// Email addresses
    Email,
    /// URLs
    Url,
    /// Identifier codes (alphanumeric with consistent format)
    Identifier,
    /// Phone numbers
    Phone,
    /// Postal/ZIP codes
    PostalCode,
}

impl PatternType {
    fn pattern(&self) -> &'static str {
        match self {
            PatternType::Email => r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$",
            PatternType::Url => r"^https?://[^\s]+$",
            PatternType::Identifier => r"^[A-Za-z]+[_-]?\d+$", // e.g., IBD001, SAMPLE_123
            PatternType::Phone => r"^\+?[\d\s\-\(\)]{7,}$",
            PatternType::PostalCode => r"^\d{5}(-\d{4})?$", // US ZIP codes
        }
    }

    fn description(&self) -> &'static str {
        match self {
            PatternType::Email => "email address",
            PatternType::Url => "URL",
            PatternType::Identifier => "identifier code",
            PatternType::Phone => "phone number",
            PatternType::PostalCode => "postal code",
        }
    }

    /// Infer pattern type from column name.
    fn from_column_name(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        if lower.contains("email") || lower.contains("e_mail") {
            Some(PatternType::Email)
        } else if lower.contains("url") || lower.contains("website") || lower.contains("link") {
            Some(PatternType::Url)
        } else if lower.contains("phone") || lower.contains("tel") || lower.contains("mobile") {
            Some(PatternType::Phone)
        } else if lower.contains("zip") || lower.contains("postal") {
            Some(PatternType::PostalCode)
        } else {
            None
        }
    }
}

/// Validates values against expected patterns (email, URL, identifiers, etc.).
pub struct RegexPatternValidator;

impl Validator for RegexPatternValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            // Skip non-string columns
            if col_schema.inferred_type != ColumnType::String {
                continue;
            }

            // Check if column name suggests a pattern
            if let Some(pattern_type) = PatternType::from_column_name(&col_schema.name) {
                let issues = self.check_pattern(table, col_schema, pattern_type);
                if let Some(obs) = issues {
                    observations.push(obs);
                }
            }

            // Check for identifier columns with inconsistent formats
            if col_schema.semantic_role == SemanticRole::Identifier {
                if let Some(obs) = self.check_identifier_consistency(table, col_schema) {
                    observations.push(obs);
                }
            }
        }

        observations
    }
}

impl RegexPatternValidator {
    /// Check values against an expected pattern.
    fn check_pattern(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
        pattern_type: PatternType,
    ) -> Option<Observation> {
        let pattern = regex::Regex::new(pattern_type.pattern()).ok()?;
        let mut invalid_rows = Vec::new();
        let mut invalid_examples = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            let trimmed = value.trim();
            if DataTable::is_null_value(trimmed) || trimmed.is_empty() {
                continue;
            }

            if !pattern.is_match(trimmed) {
                invalid_rows.push(row_idx);
                if invalid_examples.len() < 3 {
                    invalid_examples.push(trimmed.to_string());
                }
            }
        }

        if invalid_rows.is_empty() {
            return None;
        }

        let total_non_null: usize = table
            .column_values(col_schema.position)
            .filter(|v| !DataTable::is_null_value(v.trim()) && !v.trim().is_empty())
            .count();
        let pct = (invalid_rows.len() as f64 / total_non_null as f64) * 100.0;

        // Only report if < 50% match (otherwise, pattern might not be applicable)
        if pct > 50.0 {
            return None;
        }

        Some(
            Observation::new(
                ObservationType::PatternViolation,
                if pct > 10.0 {
                    Severity::Warning
                } else {
                    Severity::Info
                },
                &col_schema.name,
                format!(
                    "{} value(s) ({:.1}%) don't match expected {} format: {:?}",
                    invalid_rows.len(),
                    pct,
                    pattern_type.description(),
                    invalid_examples
                ),
            )
            .with_evidence(
                Evidence::new()
                    .with_occurrences(invalid_rows.len())
                    .with_percentage(pct)
                    .with_sample_rows(invalid_rows.into_iter().take(5).collect())
                    .with_pattern(pattern_type.pattern().to_string()),
            )
            .with_confidence(0.75)
            .with_detector("regex_pattern_validator"),
        )
    }

    /// Check identifier columns for inconsistent formats.
    fn check_identifier_consistency(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Option<Observation> {
        // Collect all unique patterns from the data
        let mut pattern_counts: IndexMap<String, (usize, Vec<String>)> = IndexMap::new();

        for value in table.column_values(col_schema.position) {
            let trimmed = value.trim();
            if DataTable::is_null_value(trimmed) || trimmed.is_empty() {
                continue;
            }

            // Extract pattern: replace digits with # and letters with @
            let pattern: String = trimmed
                .chars()
                .map(|c| {
                    if c.is_ascii_digit() {
                        '#'
                    } else if c.is_ascii_alphabetic() {
                        if c.is_uppercase() {
                            'A'
                        } else {
                            'a'
                        }
                    } else {
                        c
                    }
                })
                .collect();

            let entry = pattern_counts.entry(pattern).or_insert((0, Vec::new()));
            entry.0 += 1;
            if entry.1.len() < 2 {
                entry.1.push(trimmed.to_string());
            }
        }

        // If only one pattern or no patterns, no issue
        if pattern_counts.len() <= 1 {
            return None;
        }

        // Find dominant pattern
        let total: usize = pattern_counts.values().map(|(c, _)| c).sum();
        let (dominant_pattern, (dominant_count, _)) = pattern_counts
            .iter()
            .max_by_key(|(_, (c, _))| *c)?;

        let dominant_pct = (*dominant_count as f64 / total as f64) * 100.0;

        // Only report if there's a clear dominant pattern (>70%)
        if dominant_pct < 70.0 {
            return None;
        }

        // Collect non-dominant values
        let mut outlier_rows = Vec::new();
        let mut outlier_examples = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            let trimmed = value.trim();
            if DataTable::is_null_value(trimmed) || trimmed.is_empty() {
                continue;
            }

            let pattern: String = trimmed
                .chars()
                .map(|c| {
                    if c.is_ascii_digit() {
                        '#'
                    } else if c.is_ascii_alphabetic() {
                        if c.is_uppercase() {
                            'A'
                        } else {
                            'a'
                        }
                    } else {
                        c
                    }
                })
                .collect();

            if &pattern != dominant_pattern {
                outlier_rows.push(row_idx);
                if outlier_examples.len() < 3 {
                    outlier_examples.push(trimmed.to_string());
                }
            }
        }

        if outlier_rows.is_empty() {
            return None;
        }

        let outlier_pct = (outlier_rows.len() as f64 / total as f64) * 100.0;

        // Get example of dominant pattern
        let dominant_example = pattern_counts
            .get(dominant_pattern)
            .and_then(|(_, examples)| examples.first())
            .cloned()
            .unwrap_or_default();

        Some(
            Observation::new(
                ObservationType::Inconsistency,
                Severity::Warning,
                &col_schema.name,
                format!(
                    "Identifier format inconsistency: {} value(s) ({:.1}%) don't match dominant pattern (e.g., '{}'): {:?}",
                    outlier_rows.len(),
                    outlier_pct,
                    dominant_example,
                    outlier_examples
                ),
            )
            .with_evidence(
                Evidence::new()
                    .with_occurrences(outlier_rows.len())
                    .with_percentage(outlier_pct)
                    .with_sample_rows(outlier_rows.into_iter().take(5).collect())
                    .with_value_counts(Some(json!(
                        pattern_counts
                            .iter()
                            .map(|(p, (c, ex))| (p.clone(), json!({"count": c, "examples": ex})))
                            .collect::<IndexMap<_, _>>()
                    ))),
            )
            .with_confidence(0.80)
            .with_detector("regex_pattern_validator"),
        )
    }
}

// ============================================================================
// Cross-Column Validator
// ============================================================================

/// Validates logical relationships between columns.
pub struct CrossColumnValidator;

impl Validator for CrossColumnValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        // Check date relationships
        observations.extend(self.check_date_relationships(table, schema));

        // Check logical consistency (e.g., BMI vs weight/height)
        observations.extend(self.check_bmi_consistency(table, schema));

        // Check sex/pregnancy consistency
        observations.extend(self.check_sex_pregnancy(table, schema));

        // Check age-related constraints
        observations.extend(self.check_age_constraints(table, schema));

        observations
    }
}

impl CrossColumnValidator {
    /// Find column by name pattern (case-insensitive).
    fn find_column<'a>(
        schema: &'a TableSchema,
        patterns: &[&str],
    ) -> Option<&'a ColumnSchema> {
        for col in &schema.columns {
            let lower = col.name.to_lowercase();
            for pattern in patterns {
                if lower.contains(pattern) {
                    return Some(col);
                }
            }
        }
        None
    }

    /// Check date column relationships (start < end, birth < death, etc.).
    fn check_date_relationships(
        &self,
        table: &DataTable,
        schema: &TableSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        // Common date pairs to check
        let date_pairs = [
            (
                &["start_date", "start", "begin", "enrollment"][..],
                &["end_date", "end", "finish", "completion"][..],
                "start date should be before end date",
            ),
            (
                &["birth", "dob", "date_of_birth"][..],
                &["death", "dod", "date_of_death"][..],
                "birth date should be before death date",
            ),
            (
                &["admission", "admit"][..],
                &["discharge"][..],
                "admission should be before discharge",
            ),
            (
                &["diagnosis_date", "dx_date"][..],
                &["treatment_date", "tx_date"][..],
                "diagnosis should typically precede treatment",
            ),
        ];

        for (start_patterns, end_patterns, description) in &date_pairs {
            let start_col = Self::find_column(schema, start_patterns);
            let end_col = Self::find_column(schema, end_patterns);

            if let (Some(start), Some(end)) = (start_col, end_col) {
                let issues = self.compare_date_columns(table, start, end);
                if !issues.is_empty() {
                    let pct = (issues.len() as f64 / table.row_count() as f64) * 100.0;
                    observations.push(
                        Observation::new(
                            ObservationType::CrossColumnInconsistency,
                            Severity::Warning,
                            &format!("{} vs {}", start.name, end.name),
                            format!(
                                "{} row(s) ({:.1}%) where {}: {}",
                                issues.len(),
                                pct,
                                description,
                                issues
                                    .iter()
                                    .take(2)
                                    .map(|(r, s, e)| format!("row {}: {} > {}", r + 1, s, e))
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_occurrences(issues.len())
                                .with_percentage(pct)
                                .with_sample_rows(issues.iter().take(5).map(|(r, _, _)| *r).collect()),
                        )
                        .with_confidence(0.85)
                        .with_detector("cross_column_validator"),
                    );
                }
            }
        }

        observations
    }

    /// Compare two date columns and find violations.
    fn compare_date_columns(
        &self,
        table: &DataTable,
        start_col: &ColumnSchema,
        end_col: &ColumnSchema,
    ) -> Vec<(usize, String, String)> {
        let mut issues = Vec::new();

        for row_idx in 0..table.row_count() {
            let start_val = table.get(row_idx, start_col.position).unwrap_or_default();
            let end_val = table.get(row_idx, end_col.position).unwrap_or_default();

            if DataTable::is_null_value(start_val) || DataTable::is_null_value(end_val) {
                continue;
            }

            // Try to parse dates and compare
            if let (Some(start_date), Some(end_date)) =
                (self.parse_date(start_val), self.parse_date(end_val))
            {
                if start_date > end_date {
                    issues.push((row_idx, start_val.to_string(), end_val.to_string()));
                }
            }
        }

        issues
    }

    /// Simple date parser that returns a comparable string (YYYY-MM-DD format).
    fn parse_date(&self, value: &str) -> Option<String> {
        let trimmed = value.trim();

        // ISO format: YYYY-MM-DD
        if trimmed.len() == 10 && trimmed.chars().nth(4) == Some('-') {
            return Some(trimmed.to_string());
        }

        // Try other common formats
        // MM/DD/YYYY or DD/MM/YYYY - assume MM/DD/YYYY for US
        if let Some(parts) = trimmed.split('/').collect::<Vec<_>>().get(0..3) {
            if parts.len() == 3 {
                let (p1, p2, p3) = (parts[0], parts[1], parts[2]);
                if p3.len() == 4 {
                    return Some(format!("{}-{:0>2}-{:0>2}", p3, p1, p2));
                }
            }
        }

        // Just use string comparison as fallback
        Some(trimmed.to_string())
    }

    /// Check BMI vs weight/height consistency.
    fn check_bmi_consistency(
        &self,
        table: &DataTable,
        schema: &TableSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        let bmi_col = Self::find_column(schema, &["bmi"]);
        let weight_col = Self::find_column(schema, &["weight", "wt"]);
        let height_col = Self::find_column(schema, &["height", "ht"]);

        if let (Some(bmi), Some(weight), Some(height)) = (bmi_col, weight_col, height_col) {
            let issues = self.check_bmi_calculation(table, bmi, weight, height);
            if !issues.is_empty() {
                let pct = (issues.len() as f64 / table.row_count() as f64) * 100.0;
                observations.push(
                    Observation::new(
                        ObservationType::CrossColumnInconsistency,
                        Severity::Warning,
                        &format!("{} vs {}/{}", bmi.name, weight.name, height.name),
                        format!(
                            "{} row(s) ({:.1}%) where BMI doesn't match weight/height calculation (assuming kg/m)",
                            issues.len(),
                            pct
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(issues.len())
                            .with_percentage(pct)
                            .with_sample_rows(issues.into_iter().take(5).collect()),
                    )
                    .with_confidence(0.70)
                    .with_detector("cross_column_validator"),
                );
            }
        }

        observations
    }

    /// Check if BMI matches weight/height.
    fn check_bmi_calculation(
        &self,
        table: &DataTable,
        bmi_col: &ColumnSchema,
        weight_col: &ColumnSchema,
        height_col: &ColumnSchema,
    ) -> Vec<usize> {
        let mut issues = Vec::new();

        for row_idx in 0..table.row_count() {
            let bmi_str = table.get(row_idx, bmi_col.position).unwrap_or_default();
            let weight_str = table.get(row_idx, weight_col.position).unwrap_or_default();
            let height_str = table.get(row_idx, height_col.position).unwrap_or_default();

            if DataTable::is_null_value(bmi_str)
                || DataTable::is_null_value(weight_str)
                || DataTable::is_null_value(height_str)
            {
                continue;
            }

            if let (Ok(bmi), Ok(weight), Ok(height)) = (
                bmi_str.trim().parse::<f64>(),
                weight_str.trim().parse::<f64>(),
                height_str.trim().parse::<f64>(),
            ) {
                if height <= 0.0 || weight <= 0.0 {
                    continue;
                }

                // Calculate expected BMI (assuming height in meters, weight in kg)
                // If height > 3, assume it's in cm
                let height_m = if height > 3.0 { height / 100.0 } else { height };
                let expected_bmi = weight / (height_m * height_m);

                // Allow 10% tolerance
                let diff_pct = ((bmi - expected_bmi) / expected_bmi).abs() * 100.0;
                if diff_pct > 10.0 {
                    issues.push(row_idx);
                }
            }
        }

        issues
    }

    /// Check sex/pregnancy logical consistency.
    fn check_sex_pregnancy(
        &self,
        table: &DataTable,
        schema: &TableSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        let sex_col = Self::find_column(schema, &["sex", "gender"]);
        let pregnant_col = Self::find_column(schema, &["pregnant", "pregnancy"]);

        if let (Some(sex), Some(pregnant)) = (sex_col, pregnant_col) {
            let mut issues = Vec::new();

            for row_idx in 0..table.row_count() {
                let sex_val = table.get(row_idx, sex.position).unwrap_or_default().to_lowercase();
                let preg_val = table
                    .get(row_idx, pregnant.position)
                    .unwrap_or_default()
                    .to_lowercase();

                let is_male = sex_val.contains("male") && !sex_val.contains("female")
                    || sex_val == "m";
                let is_pregnant = preg_val == "yes"
                    || preg_val == "y"
                    || preg_val == "true"
                    || preg_val == "1";

                if is_male && is_pregnant {
                    issues.push(row_idx);
                }
            }

            if !issues.is_empty() {
                let pct = (issues.len() as f64 / table.row_count() as f64) * 100.0;
                observations.push(
                    Observation::new(
                        ObservationType::CrossColumnInconsistency,
                        Severity::Error,
                        &format!("{} vs {}", sex.name, pregnant.name),
                        format!(
                            "{} row(s) ({:.1}%) where male is marked as pregnant",
                            issues.len(),
                            pct
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(issues.len())
                            .with_percentage(pct)
                            .with_sample_rows(issues.into_iter().take(5).collect()),
                    )
                    .with_confidence(0.95)
                    .with_detector("cross_column_validator"),
                );
            }
        }

        observations
    }

    /// Check age-related constraints.
    fn check_age_constraints(
        &self,
        table: &DataTable,
        schema: &TableSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();

        let age_col = Self::find_column(schema, &["age"]);

        if let Some(age) = age_col {
            // Check for pediatric-only or adult-only conditions
            let diagnosis_col = Self::find_column(schema, &["diagnosis", "dx", "condition"]);

            if let Some(dx) = diagnosis_col {
                // Conditions that are unusual in certain age groups
                let adult_conditions = ["type 2 diabetes", "t2d", "menopause", "prostate"];
                let pediatric_max_age = 12;

                let mut issues = Vec::new();

                for row_idx in 0..table.row_count() {
                    let age_str = table.get(row_idx, age.position).unwrap_or_default();
                    let dx_val = table
                        .get(row_idx, dx.position)
                        .unwrap_or_default()
                        .to_lowercase();

                    if DataTable::is_null_value(age_str) {
                        continue;
                    }

                    if let Ok(age_val) = age_str.trim().parse::<i32>() {
                        // Check adult conditions in young children
                        if age_val <= pediatric_max_age {
                            for condition in &adult_conditions {
                                if dx_val.contains(condition) {
                                    issues.push((row_idx, age_val, dx_val.clone()));
                                    break;
                                }
                            }
                        }
                    }
                }

                if !issues.is_empty() {
                    let pct = (issues.len() as f64 / table.row_count() as f64) * 100.0;
                    observations.push(
                        Observation::new(
                            ObservationType::CrossColumnInconsistency,
                            Severity::Warning,
                            &format!("{} vs {}", age.name, dx.name),
                            format!(
                                "{} row(s) ({:.1}%) with unusual age/diagnosis combination: {}",
                                issues.len(),
                                pct,
                                issues
                                    .iter()
                                    .take(2)
                                    .map(|(_, a, d)| format!("age {} with '{}'", a, d))
                                    .collect::<Vec<_>>()
                                    .join("; ")
                            ),
                        )
                        .with_evidence(
                            Evidence::new()
                                .with_occurrences(issues.len())
                                .with_percentage(pct)
                                .with_sample_rows(issues.iter().take(5).map(|(r, _, _)| *r).collect()),
                        )
                        .with_confidence(0.70)
                        .with_detector("cross_column_validator"),
                    );
                }
            }
        }

        observations
    }
}

/// Validates that treatment/drug names use proper title case.
///
/// Drug names should typically be capitalized (e.g., "Infliximab" not "infliximab").
/// This validator flags lowercase values in treatment-related columns.
pub struct TitleCaseValidator;

impl TitleCaseValidator {
    /// Check if a column name suggests it contains treatment/drug names.
    fn is_treatment_column(name: &str) -> bool {
        let lower = name.to_lowercase();
        lower.contains("treatment")
            || lower.contains("drug")
            || lower.contains("medication")
            || lower.contains("medicine")
            || lower.contains("therapy")
            || lower.contains("rx")
    }

    /// Check if a value starts with lowercase when it should be title case.
    fn needs_title_case(value: &str) -> bool {
        let trimmed = value.trim();
        if trimmed.is_empty() || DataTable::is_null_value(trimmed) {
            return false;
        }
        // Check if first character is lowercase letter
        trimmed.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
    }

    /// Convert a value to title case (capitalize first letter).
    fn to_title_case(value: &str) -> String {
        let mut chars: Vec<char> = value.chars().collect();
        if let Some(first) = chars.first_mut() {
            *first = first.to_ascii_uppercase();
        }
        chars.into_iter().collect()
    }
}

impl Validator for TitleCaseValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            if !Self::is_treatment_column(&col_schema.name) {
                continue;
            }

            // Find values that need title case
            let mut lowercase_values: IndexMap<String, (usize, Vec<usize>)> = IndexMap::new();

            for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
                if Self::needs_title_case(value) {
                    let entry = lowercase_values
                        .entry(value.to_string())
                        .or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if entry.1.len() < 5 {
                        entry.1.push(row_idx + 1); // 1-indexed
                    }
                }
            }

            if lowercase_values.is_empty() {
                continue;
            }

            let total: usize = lowercase_values.values().map(|(c, _)| c).sum();
            let pct = (total as f64 / table.row_count() as f64) * 100.0;

            // Build mapping for suggestions
            let value_counts: serde_json::Value = lowercase_values
                .iter()
                .map(|(val, (count, _))| {
                    let title = Self::to_title_case(val);
                    (
                        val.clone(),
                        json!({
                            "count": count,
                            "suggestion": title
                        }),
                    )
                })
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();

            let sample_rows: Vec<usize> = lowercase_values
                .values()
                .flat_map(|(_, rows)| rows.iter().copied())
                .take(5)
                .collect();

            let examples: Vec<String> = lowercase_values
                .keys()
                .take(3)
                .map(|v| format!("'{}' → '{}'", v, Self::to_title_case(v)))
                .collect();

            observations.push(
                Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "{} value(s) need title case: {}",
                        total,
                        examples.join(", ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total)
                        .with_percentage(pct)
                        .with_sample_rows(sample_rows)
                        .with_value_counts(Some(value_counts)),
                )
                .with_confidence(0.85)
                .with_detector("title_case_validator"),
            );
        }

        observations
    }
}

// ============================================================================
// Coordinate Validator
// ============================================================================

/// Validates geographic coordinate columns (lat/lon).
///
/// Detects:
/// - Out-of-range latitude (must be -90 to 90)
/// - Out-of-range longitude (must be -180 to 180)
/// - Inconsistent coordinate formats (decimal degrees vs DMS)
/// - Swapped lat/lon values
/// - Coordinates in ocean when land expected (or vice versa)
pub struct CoordinateValidator;

impl CoordinateValidator {
    /// Column name patterns that indicate coordinate data.
    fn is_coordinate_column(name: &str) -> Option<CoordinateType> {
        let lower = name.to_lowercase();

        // Exclude location name columns (geo_loc_name is for place names, not coordinates)
        if lower.contains("_name") || lower.ends_with("name") {
            return None;
        }

        // Combined lat_lon column
        if lower.contains("lat_lon")
            || lower.contains("latlon")
            || lower.contains("coordinates")
            || lower == "geo_loc"
            || lower == "location"
        {
            return Some(CoordinateType::Combined);
        }

        // Latitude column
        if lower == "lat"
            || lower == "latitude"
            || lower.contains("_lat")
            || lower.starts_with("lat_")
        {
            return Some(CoordinateType::Latitude);
        }

        // Longitude column
        if lower == "lon"
            || lower == "lng"
            || lower == "long"
            || lower == "longitude"
            || lower.contains("_lon")
            || lower.contains("_lng")
            || lower.starts_with("lon_")
            || lower.starts_with("lng_")
        {
            return Some(CoordinateType::Longitude);
        }

        None
    }

    /// Parse a coordinate value and return (value, format).
    fn parse_coordinate(value: &str) -> Option<(f64, CoordinateFormat)> {
        let trimmed = value.trim();

        // Skip null/missing values
        if DataTable::is_null_value(trimmed)
            || trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("missing")
            || trimmed.eq_ignore_ascii_case("not collected")
            || trimmed.eq_ignore_ascii_case("not applicable")
        {
            return None;
        }

        // Try decimal degrees with optional direction suffix
        // e.g., "38.98", "38.98N", "-77.11", "77.11W"
        let (num_str, direction) = Self::extract_direction(trimmed);
        if let Ok(mut val) = num_str.parse::<f64>() {
            // Apply direction
            if matches!(direction, Some('S') | Some('W')) {
                val = -val.abs();
            } else if matches!(direction, Some('N') | Some('E')) {
                val = val.abs();
            }
            return Some((val, CoordinateFormat::DecimalDegrees));
        }

        // Try DMS format: 38°58'48"N or 38 58 48 N
        if let Some((val, _)) = Self::parse_dms(trimmed) {
            return Some((val, CoordinateFormat::DMS));
        }

        None
    }

    /// Extract direction suffix (N/S/E/W) from coordinate string.
    fn extract_direction(s: &str) -> (&str, Option<char>) {
        let s = s.trim();
        if let Some(last) = s.chars().last() {
            if matches!(last, 'N' | 'S' | 'E' | 'W' | 'n' | 's' | 'e' | 'w') {
                let num_part = s[..s.len() - 1].trim();
                return (num_part, Some(last.to_ascii_uppercase()));
            }
        }
        (s, None)
    }

    /// Parse DMS (Degrees Minutes Seconds) format.
    fn parse_dms(s: &str) -> Option<(f64, char)> {
        // Handle formats like: 38°58'48"N, 38 58 48 N, 38:58:48N
        let s = s
            .replace('°', " ")
            .replace("'", " ")
            .replace('"', " ")
            .replace(':', " ");

        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let degrees: f64 = parts[0].parse().ok()?;
        let minutes: f64 = parts[1].parse().ok()?;
        let seconds: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);

        let mut value = degrees + minutes / 60.0 + seconds / 3600.0;

        // Check for direction
        let direction = parts
            .last()
            .and_then(|s| s.chars().next())
            .filter(|c| matches!(c, 'N' | 'S' | 'E' | 'W' | 'n' | 's' | 'e' | 'w'))
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or('N');

        if matches!(direction, 'S' | 'W') {
            value = -value;
        }

        Some((value, direction))
    }

    /// Parse combined lat_lon format (e.g., "38.98 -77.11").
    fn parse_combined(value: &str) -> Option<(f64, f64, CoordinateFormat)> {
        let trimmed = value.trim();

        // Skip null/missing values
        if DataTable::is_null_value(trimmed)
            || trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("missing")
            || trimmed.eq_ignore_ascii_case("not collected")
            || trimmed.eq_ignore_ascii_case("not applicable")
        {
            return None;
        }

        // Split on whitespace, comma, or semicolon
        let parts: Vec<&str> = trimmed
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .collect();

        if parts.len() != 2 {
            return None;
        }

        let (lat, lat_fmt) = Self::parse_coordinate(parts[0])?;
        let (lon, lon_fmt) = Self::parse_coordinate(parts[1])?;

        // Use the more specific format
        let format = if lat_fmt == CoordinateFormat::DMS || lon_fmt == CoordinateFormat::DMS {
            CoordinateFormat::DMS
        } else {
            CoordinateFormat::DecimalDegrees
        };

        Some((lat, lon, format))
    }

    /// Check if latitude is in valid range.
    fn is_valid_latitude(lat: f64) -> bool {
        (-90.0..=90.0).contains(&lat)
    }

    /// Check if longitude is in valid range.
    fn is_valid_longitude(lon: f64) -> bool {
        (-180.0..=180.0).contains(&lon)
    }

    /// Check if coordinates might be swapped (lat looks like lon or vice versa).
    fn might_be_swapped(lat: f64, lon: f64) -> bool {
        // If lat is out of range but would be valid as lon, and vice versa
        !Self::is_valid_latitude(lat)
            && Self::is_valid_longitude(lat)
            && Self::is_valid_latitude(lon)
    }
}

/// Type of coordinate column.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CoordinateType {
    Latitude,
    Longitude,
    Combined,
}

/// Format of coordinate values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CoordinateFormat {
    DecimalDegrees,
    DMS,
}

impl CoordinateFormat {
    fn description(&self) -> &'static str {
        match self {
            CoordinateFormat::DecimalDegrees => "Decimal Degrees",
            CoordinateFormat::DMS => "Degrees Minutes Seconds",
        }
    }
}

impl Validator for CoordinateValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        for col_schema in &schema.columns {
            let coord_type = match Self::is_coordinate_column(&col_schema.name) {
                Some(ct) => ct,
                None => continue,
            };

            match coord_type {
                CoordinateType::Combined => {
                    observations.extend(self.validate_combined_column(table, col_schema));
                }
                CoordinateType::Latitude => {
                    observations.extend(self.validate_latitude_column(table, col_schema));
                }
                CoordinateType::Longitude => {
                    observations.extend(self.validate_longitude_column(table, col_schema));
                }
            }
        }

        observations
    }
}

impl CoordinateValidator {
    fn validate_combined_column(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();
        let mut out_of_range: Vec<(usize, String, String)> = Vec::new();
        let mut format_issues: IndexMap<CoordinateFormat, usize> = IndexMap::new();
        let mut parse_errors: Vec<(usize, String)> = Vec::new();
        let mut swapped: Vec<(usize, f64, f64)> = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            let trimmed = value.trim();
            if DataTable::is_null_value(trimmed) || trimmed.is_empty() {
                continue;
            }

            match Self::parse_combined(trimmed) {
                Some((lat, lon, format)) => {
                    *format_issues.entry(format).or_insert(0) += 1;

                    if !Self::is_valid_latitude(lat) || !Self::is_valid_longitude(lon) {
                        let issue = if !Self::is_valid_latitude(lat) {
                            format!("latitude {} out of range [-90, 90]", lat)
                        } else {
                            format!("longitude {} out of range [-180, 180]", lon)
                        };
                        out_of_range.push((row_idx, trimmed.to_string(), issue));
                    }

                    if Self::might_be_swapped(lat, lon) {
                        swapped.push((row_idx, lat, lon));
                    }
                }
                None => {
                    // Skip known null patterns
                    if !trimmed.eq_ignore_ascii_case("missing")
                        && !trimmed.eq_ignore_ascii_case("not collected")
                        && !trimmed.eq_ignore_ascii_case("not applicable")
                    {
                        parse_errors.push((row_idx, trimmed.to_string()));
                    }
                }
            }
        }

        // Report out-of-range coordinates
        if !out_of_range.is_empty() {
            let count = out_of_range.len();
            let pct = (count as f64 / table.row_count() as f64) * 100.0;

            observations.push(
                Observation::new(
                    ObservationType::Outlier,
                    Severity::Error,
                    &col_schema.name,
                    format!(
                        "{} coordinate(s) out of valid range: {}",
                        count,
                        out_of_range
                            .iter()
                            .take(2)
                            .map(|(_, v, issue)| format!("'{}' ({})", v, issue))
                            .collect::<Vec<_>>()
                            .join("; ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(out_of_range.iter().take(5).map(|(r, _, _)| *r).collect())
                        .with_expected(json!({
                            "latitude_range": [-90, 90],
                            "longitude_range": [-180, 180]
                        })),
                )
                .with_confidence(0.95)
                .with_detector("coordinate_validator"),
            );
        }

        // Report format inconsistencies
        if format_issues.len() > 1 {
            let total: usize = format_issues.values().sum();
            let format_desc: Vec<String> = format_issues
                .iter()
                .map(|(fmt, count)| format!("{}: {}", fmt.description(), count))
                .collect();

            observations.push(
                Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "Mixed coordinate formats detected: {}. Consider standardizing to decimal degrees.",
                        format_desc.join(", ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total)
                        .with_value_counts(Some(json!(format_issues
                            .iter()
                            .map(|(f, c)| (f.description().to_string(), *c))
                            .collect::<IndexMap<_, _>>()))),
                )
                .with_confidence(0.85)
                .with_detector("coordinate_validator"),
            );
        }

        // Report potentially swapped coordinates
        if !swapped.is_empty() {
            let count = swapped.len();
            let pct = (count as f64 / table.row_count() as f64) * 100.0;

            observations.push(
                Observation::new(
                    ObservationType::Inconsistency,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "{} coordinate(s) may have latitude and longitude swapped",
                        count
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(swapped.iter().take(5).map(|(r, _, _)| *r).collect()),
                )
                .with_confidence(0.75)
                .with_detector("coordinate_validator"),
            );
        }

        // Report parse errors
        if !parse_errors.is_empty() {
            let count = parse_errors.len();
            let pct = (count as f64 / table.row_count() as f64) * 100.0;

            observations.push(
                Observation::new(
                    ObservationType::PatternViolation,
                    Severity::Warning,
                    &col_schema.name,
                    format!(
                        "{} value(s) could not be parsed as coordinates: {:?}",
                        count,
                        parse_errors.iter().take(3).map(|(_, v)| v).collect::<Vec<_>>()
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(parse_errors.iter().take(5).map(|(r, _)| *r).collect())
                        .with_expected("Format: 'DD.DDDD DD.DDDD' or 'DD.DDDDN DD.DDDDW'"),
                )
                .with_confidence(0.80)
                .with_detector("coordinate_validator"),
            );
        }

        observations
    }

    fn validate_latitude_column(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();
        let mut out_of_range: Vec<(usize, f64)> = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if let Some((lat, _)) = Self::parse_coordinate(value) {
                if !Self::is_valid_latitude(lat) {
                    out_of_range.push((row_idx, lat));
                }
            }
        }

        if !out_of_range.is_empty() {
            let count = out_of_range.len();
            let pct = (count as f64 / table.row_count() as f64) * 100.0;

            observations.push(
                Observation::new(
                    ObservationType::Outlier,
                    Severity::Error,
                    &col_schema.name,
                    format!(
                        "{} latitude value(s) out of range [-90, 90]: {:?}",
                        count,
                        out_of_range.iter().take(3).map(|(_, v)| v).collect::<Vec<_>>()
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(out_of_range.iter().take(5).map(|(r, _)| *r).collect())
                        .with_expected(json!({"min": -90, "max": 90})),
                )
                .with_confidence(0.95)
                .with_detector("coordinate_validator"),
            );
        }

        observations
    }

    fn validate_longitude_column(
        &self,
        table: &DataTable,
        col_schema: &ColumnSchema,
    ) -> Vec<Observation> {
        let mut observations = Vec::new();
        let mut out_of_range: Vec<(usize, f64)> = Vec::new();

        for (row_idx, value) in table.column_values(col_schema.position).enumerate() {
            if let Some((lon, _)) = Self::parse_coordinate(value) {
                if !Self::is_valid_longitude(lon) {
                    out_of_range.push((row_idx, lon));
                }
            }
        }

        if !out_of_range.is_empty() {
            let count = out_of_range.len();
            let pct = (count as f64 / table.row_count() as f64) * 100.0;

            observations.push(
                Observation::new(
                    ObservationType::Outlier,
                    Severity::Error,
                    &col_schema.name,
                    format!(
                        "{} longitude value(s) out of range [-180, 180]: {:?}",
                        count,
                        out_of_range.iter().take(3).map(|(_, v)| v).collect::<Vec<_>>()
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(count)
                        .with_percentage(pct)
                        .with_sample_rows(out_of_range.iter().take(5).map(|(r, _)| *r).collect())
                        .with_expected(json!({"min": -180, "max": 180})),
                )
                .with_confidence(0.95)
                .with_detector("coordinate_validator"),
            );
        }

        observations
    }
}

// ============================================================================
// Duplicate Row Validator
// ============================================================================

/// Validates for duplicate or near-duplicate rows.
///
/// Detects:
/// - Exact duplicate rows (all values identical)
/// - Near-duplicates (same values except for identifier column)
/// - Conflicting duplicates (same ID but different values)
pub struct DuplicateRowValidator {
    /// Columns to exclude when checking for duplicates (typically identifiers).
    exclude_patterns: Vec<&'static str>,
}

impl Default for DuplicateRowValidator {
    fn default() -> Self {
        Self {
            exclude_patterns: vec![
                "id", "sample_id", "patient_id", "subject_id", "row_id",
                "index", "uuid", "guid", "key", "record_id",
            ],
        }
    }
}

impl DuplicateRowValidator {
    /// Check if a column should be excluded from duplicate detection.
    fn is_excluded_column(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.exclude_patterns.iter().any(|p| lower.contains(p))
    }

    /// Get a hash key for a row (excluding identifier columns).
    fn row_key(&self, table: &DataTable, schema: &TableSchema, row_idx: usize) -> String {
        let mut parts = Vec::new();
        for col in &schema.columns {
            if !self.is_excluded_column(&col.name) {
                let value = table.get(row_idx, col.position).unwrap_or_default();
                parts.push(value.trim().to_lowercase());
            }
        }
        parts.join("\0")
    }

    /// Get a full row key (including all columns).
    fn full_row_key(&self, table: &DataTable, row_idx: usize) -> String {
        let mut parts = Vec::new();
        for col_idx in 0..table.column_count() {
            let value = table.get(row_idx, col_idx).unwrap_or_default();
            parts.push(value.trim().to_lowercase());
        }
        parts.join("\0")
    }

    /// Find the primary identifier column.
    fn find_id_column<'a>(&self, schema: &'a TableSchema) -> Option<&'a ColumnSchema> {
        // Prefer columns explicitly marked as identifiers
        if let Some(col) = schema.columns.iter().find(|c| c.semantic_role == SemanticRole::Identifier) {
            return Some(col);
        }

        // Fall back to name-based detection
        for col in &schema.columns {
            let lower = col.name.to_lowercase();
            if lower == "sample_id"
                || lower == "id"
                || lower == "patient_id"
                || lower == "subject_id"
            {
                return Some(col);
            }
        }

        // Use first column if it looks like an ID
        if let Some(first) = schema.columns.first() {
            let lower = first.name.to_lowercase();
            if lower.contains("id") || lower.contains("sample") || lower.contains("subject") {
                return Some(first);
            }
        }

        None
    }
}

impl Validator for DuplicateRowValidator {
    fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut observations = Vec::new();

        if table.row_count() < 2 || schema.columns.is_empty() {
            return observations;
        }

        // Find exact duplicates
        let mut full_row_map: IndexMap<String, Vec<usize>> = IndexMap::new();
        for row_idx in 0..table.row_count() {
            let key = self.full_row_key(table, row_idx);
            full_row_map.entry(key).or_default().push(row_idx);
        }

        let exact_duplicates: Vec<_> = full_row_map
            .values()
            .filter(|rows| rows.len() > 1)
            .cloned()
            .collect();

        if !exact_duplicates.is_empty() {
            let total_dup_rows: usize = exact_duplicates.iter().map(|g| g.len() - 1).sum();
            let pct = (total_dup_rows as f64 / table.row_count() as f64) * 100.0;

            let sample_groups: Vec<String> = exact_duplicates
                .iter()
                .take(3)
                .map(|rows| {
                    format!(
                        "rows {}",
                        rows.iter()
                            .map(|r| (r + 1).to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect();

            observations.push(
                Observation::new(
                    ObservationType::Duplicate,
                    Severity::Error,
                    "_table",
                    format!(
                        "{} exact duplicate row(s) found in {} group(s): {}",
                        total_dup_rows,
                        exact_duplicates.len(),
                        sample_groups.join("; ")
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(total_dup_rows)
                        .with_percentage(pct)
                        .with_sample_rows(
                            exact_duplicates
                                .iter()
                                .flat_map(|g| g.iter().skip(1))
                                .take(5)
                                .copied()
                                .collect(),
                        ),
                )
                .with_confidence(0.98)
                .with_detector("duplicate_row_validator"),
            );
        }

        // Find near-duplicates (same content except ID column)
        let mut content_map: IndexMap<String, Vec<usize>> = IndexMap::new();
        for row_idx in 0..table.row_count() {
            let key = self.row_key(table, schema, row_idx);
            content_map.entry(key).or_default().push(row_idx);
        }

        let near_duplicates: Vec<_> = content_map
            .values()
            .filter(|rows| rows.len() > 1)
            .cloned()
            .collect();

        // Only report near-duplicates that aren't also exact duplicates
        let near_only: Vec<_> = near_duplicates
            .into_iter()
            .filter(|rows| {
                // Check if these rows have different IDs
                let id_col = self.find_id_column(schema);
                if let Some(id) = id_col {
                    let ids: std::collections::HashSet<_> = rows
                        .iter()
                        .map(|r| table.get(*r, id.position).unwrap_or_default().to_lowercase())
                        .collect();
                    ids.len() > 1 // Different IDs but same content
                } else {
                    false
                }
            })
            .collect();

        if !near_only.is_empty() {
            let total_near: usize = near_only.iter().map(|g| g.len()).sum();
            let pct = (total_near as f64 / table.row_count() as f64) * 100.0;

            let id_col_name = self
                .find_id_column(schema)
                .map(|c| c.name.as_str())
                .unwrap_or("ID");

            observations.push(
                Observation::new(
                    ObservationType::Duplicate,
                    Severity::Warning,
                    "_table",
                    format!(
                        "{} row(s) have identical content but different {} values (potential data entry duplicates)",
                        total_near,
                        id_col_name
                    ),
                )
                .with_evidence(
                    Evidence::new()
                        .with_occurrences(near_only.len())
                        .with_percentage(pct)
                        .with_sample_rows(
                            near_only
                                .iter()
                                .flat_map(|g| g.iter())
                                .take(5)
                                .copied()
                                .collect(),
                        ),
                )
                .with_confidence(0.85)
                .with_detector("duplicate_row_validator"),
            );
        }

        // Find ID conflicts (same ID but different content)
        if let Some(id_col) = self.find_id_column(schema) {
            let mut id_map: IndexMap<String, Vec<usize>> = IndexMap::new();
            for row_idx in 0..table.row_count() {
                let id = table.get(row_idx, id_col.position).unwrap_or_default();
                if !DataTable::is_null_value(id) && !id.is_empty() {
                    id_map.entry(id.to_lowercase()).or_default().push(row_idx);
                }
            }

            let mut conflicts: Vec<(String, Vec<usize>)> = Vec::new();
            for (id, rows) in &id_map {
                if rows.len() > 1 {
                    // Check if content differs
                    let contents: std::collections::HashSet<_> = rows
                        .iter()
                        .map(|r| self.row_key(table, schema, *r))
                        .collect();

                    if contents.len() > 1 {
                        conflicts.push((id.clone(), rows.clone()));
                    }
                }
            }

            if !conflicts.is_empty() {
                let total_conflicts: usize = conflicts.iter().map(|(_, r)| r.len()).sum();
                let pct = (total_conflicts as f64 / table.row_count() as f64) * 100.0;

                let sample_ids: Vec<String> = conflicts
                    .iter()
                    .take(3)
                    .map(|(id, rows)| format!("'{}' (rows {})", id, rows.len()))
                    .collect();

                observations.push(
                    Observation::new(
                        ObservationType::Inconsistency,
                        Severity::Error,
                        &id_col.name,
                        format!(
                            "{} {} value(s) appear multiple times with different data: {}",
                            conflicts.len(),
                            id_col.name,
                            sample_ids.join(", ")
                        ),
                    )
                    .with_evidence(
                        Evidence::new()
                            .with_occurrences(total_conflicts)
                            .with_percentage(pct)
                            .with_sample_rows(
                                conflicts
                                    .iter()
                                    .flat_map(|(_, r)| r.iter())
                                    .take(5)
                                    .copied()
                                    .collect(),
                            )
                            .with_value_counts(Some(json!(
                                conflicts
                                    .iter()
                                    .take(5)
                                    .map(|(id, rows)| (id.clone(), rows.len()))
                                    .collect::<IndexMap<_, _>>()
                            ))),
                    )
                    .with_confidence(0.95)
                    .with_detector("duplicate_row_validator"),
                );
            }
        }

        observations
    }
}

/// Composite validator that runs all validators.
pub struct ValidationEngine {
    validators: Vec<Box<dyn Validator>>,
}

impl ValidationEngine {
    /// Create a new validation engine with all default validators.
    pub fn new() -> Self {
        Self {
            validators: vec![
                Box::new(TypeValidator),
                Box::new(RangeValidator),
                Box::new(SetValidator),
                Box::new(UniquenessValidator),
                Box::new(IdentifierDuplicateValidator),
                Box::new(StatisticalOutlierValidator::default()),
                Box::new(CompletenessValidator::default()),
                Box::new(ConsistencyValidator),
                Box::new(CaseVariantValidator),
                Box::new(TypoValidator::default()),
                Box::new(SemanticEquivalenceValidator::default()),
                Box::new(DateFormatValidator),
                Box::new(MissingPatternValidator::default()),
                Box::new(RegexPatternValidator),
                Box::new(CrossColumnValidator),
                Box::new(TitleCaseValidator),
                Box::new(CoordinateValidator),
                Box::new(DuplicateRowValidator::default()),
            ],
        }
    }

    /// Run all validators and collect observations.
    pub fn validate(&self, table: &DataTable, schema: &TableSchema) -> Vec<Observation> {
        let mut all_observations = Vec::new();

        for validator in &self.validators {
            let observations = validator.validate(table, schema);
            all_observations.extend(observations);
        }

        // Sort by severity (errors first)
        all_observations.sort_by(|a, b| b.severity.cmp(&a.severity));

        all_observations
    }
}

impl Default for ValidationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColumnStatistics, SemanticRole, SemanticType};

    fn make_table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> DataTable {
        DataTable::new(
            headers.into_iter().map(String::from).collect(),
            rows.into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
            b',',
        )
    }

    fn make_simple_schema(columns: Vec<(&str, ColumnType)>) -> TableSchema {
        TableSchema::with_columns(
            columns
                .into_iter()
                .enumerate()
                .map(|(i, (name, typ))| ColumnSchema {
                    name: name.to_string(),
                    position: i,
                    inferred_type: typ,
                    semantic_type: SemanticType::Unknown,
                    semantic_role: SemanticRole::Unknown,
                    nullable: false,
                    unique: false,
                    expected_values: None,
                    expected_range: None,
                    constraints: Vec::new(),
                    statistics: ColumnStatistics::default(),
                    confidence: 0.9,
                    inference_sources: vec!["test".to_string()],
                    llm_insight: None,
                })
                .collect(),
        )
    }

    #[test]
    fn test_type_validator() {
        let table = make_table(
            vec!["age"],
            vec![vec!["25"], vec!["30"], vec!["invalid"], vec!["28"]],
        );
        let schema = make_simple_schema(vec![("age", ColumnType::Integer)]);

        let validator = TypeValidator;
        let observations = validator.validate(&table, &schema);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].observation_type, ObservationType::TypeMismatch);
    }

    #[test]
    fn test_missing_pattern_validator() {
        let table = make_table(
            vec!["status"],
            vec![
                vec!["active"],
                vec!["missing"],
                vec!["inactive"],
                vec!["missing"],
            ],
        );
        let schema = make_simple_schema(vec![("status", ColumnType::String)]);

        let validator = MissingPatternValidator::default();
        let observations = validator.validate(&table, &schema);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].observation_type, ObservationType::MissingPattern);
        assert!(observations[0].description.contains("missing"));
    }

    #[test]
    fn test_regex_pattern_validator_email() {
        let table = make_table(
            vec!["email"],
            vec![
                vec!["user@example.com"],
                vec!["test@domain.org"],
                vec!["invalid-email"],
                vec!["another@valid.net"],
            ],
        );
        let schema = make_simple_schema(vec![("email", ColumnType::String)]);

        let validator = RegexPatternValidator;
        let observations = validator.validate(&table, &schema);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].observation_type, ObservationType::PatternViolation);
        assert!(observations[0].description.contains("email"));
    }

    #[test]
    fn test_cross_column_validator_dates() {
        let table = make_table(
            vec!["start_date", "end_date"],
            vec![
                vec!["2024-01-01", "2024-12-31"],
                vec!["2024-06-15", "2024-03-01"],  // Invalid: end before start
                vec!["2024-02-01", "2024-05-15"],
            ],
        );

        let schema = TableSchema::with_columns(vec![
            ColumnSchema {
                name: "start_date".to_string(),
                position: 0,
                inferred_type: ColumnType::Date,
                semantic_type: SemanticType::Unknown,
                semantic_role: SemanticRole::Unknown,
                nullable: false,
                unique: false,
                expected_values: None,
                expected_range: None,
                constraints: Vec::new(),
                statistics: ColumnStatistics::default(),
                confidence: 0.9,
                inference_sources: vec!["test".to_string()],
                llm_insight: None,
            },
            ColumnSchema {
                name: "end_date".to_string(),
                position: 1,
                inferred_type: ColumnType::Date,
                semantic_type: SemanticType::Unknown,
                semantic_role: SemanticRole::Unknown,
                nullable: false,
                unique: false,
                expected_values: None,
                expected_range: None,
                constraints: Vec::new(),
                statistics: ColumnStatistics::default(),
                confidence: 0.9,
                inference_sources: vec!["test".to_string()],
                llm_insight: None,
            },
        ]);

        let validator = CrossColumnValidator;
        let observations = validator.validate(&table, &schema);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].observation_type, ObservationType::CrossColumnInconsistency);
        assert!(observations[0].description.contains("start date"));
    }
}
