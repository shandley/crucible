//! Validators for checking data against inferred schema.

use indexmap::IndexMap;
use serde_json::json;

use crate::input::DataTable;
use crate::schema::{ColumnSchema, ColumnType, Constraint, TableSchema};

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
                Box::new(CompletenessValidator::default()),
                Box::new(ConsistencyValidator),
                Box::new(MissingPatternValidator::default()),
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
}
