//! Rule-based suggestion generation from observations.
//!
//! This module generates actionable suggestions from observations without
//! requiring an LLM. Each observation type has specific logic to create
//! appropriate fix suggestions.

use indexmap::IndexMap;
use serde_json::{json, Value};

use crate::validation::{Observation, ObservationType, Severity};

use super::{Suggestion, SuggestionAction};

/// Generates suggestions from observations using rule-based logic.
pub struct SuggestionEngine;

impl SuggestionEngine {
    /// Generate suggestions for a list of observations.
    pub fn generate(observations: &[Observation]) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for obs in observations {
            if let Some(suggestion) = Self::generate_for_observation(obs) {
                suggestions.push(suggestion);
            }
        }

        // Sort by priority (lower = higher priority)
        suggestions.sort_by_key(|s| s.priority);

        suggestions
    }

    /// Generate a suggestion for a single observation.
    fn generate_for_observation(obs: &Observation) -> Option<Suggestion> {
        match obs.observation_type {
            ObservationType::MissingPattern => Self::suggest_convert_na(obs),
            ObservationType::Inconsistency => {
                // Check if this is a date format inconsistency
                if Self::is_date_format_issue(obs) {
                    Self::suggest_convert_date(obs)
                } else {
                    Self::suggest_standardize(obs)
                }
            }
            ObservationType::Outlier => Self::suggest_flag_outlier(obs),
            ObservationType::Duplicate => Self::suggest_handle_duplicate(obs),
            ObservationType::TypeMismatch => Self::suggest_handle_type_mismatch(obs),
            ObservationType::ConstraintViolation => Self::suggest_flag_constraint(obs),
            ObservationType::Completeness => Self::suggest_flag_completeness(obs),
            ObservationType::Cardinality => None, // Informational, no action needed
            ObservationType::CrossColumn => Self::suggest_flag_cross_column(obs),
            ObservationType::PatternViolation => Self::suggest_flag_pattern(obs),
            ObservationType::CrossColumnInconsistency => Self::suggest_flag_cross_column(obs),
        }
    }

    /// Check if an observation is about date format inconsistencies.
    fn is_date_format_issue(obs: &Observation) -> bool {
        let desc = &obs.description;
        desc.contains("date format") || desc.contains("Mixed date")
    }

    /// Generate suggestion to standardize date formats.
    fn suggest_convert_date(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        let params = json!({
            "column": obs.column,
            "target_format": "ISO (YYYY-MM-DD)",
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::ConvertDate,
                format!(
                    "Standardize {} date value(s) in column '{}' to ISO format (YYYY-MM-DD). This ensures consistent date handling across all rows.",
                    occurrences, obs.column
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(2) // High priority - format consistency is important
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to convert missing patterns to NA.
    fn suggest_convert_na(obs: &Observation) -> Option<Suggestion> {
        let pattern = obs.evidence.pattern.as_ref()?;
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        let params = json!({
            "column": obs.column,
            "from_values": [pattern],
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::ConvertNa,
                format!(
                    "Convert '{}' to proper NA values in column '{}'. This will standardize {} occurrence(s) as missing data.",
                    pattern, obs.column, occurrences
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(3) // Medium-high priority
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to standardize inconsistent values.
    fn suggest_standardize(obs: &Observation) -> Option<Suggestion> {
        let value_counts = obs.evidence.value_counts.as_ref()?;

        // Try to extract mapping from value_counts
        let mapping = Self::extract_standardization_mapping(value_counts, &obs.column);

        if mapping.is_empty() {
            return None;
        }

        let affected_rows = obs.evidence.occurrences.unwrap_or(0);
        let variants: Vec<String> = mapping.keys().cloned().collect();
        let target = mapping.values().next().cloned().unwrap_or_default();

        let params = json!({
            "column": obs.column,
            "mapping": mapping,
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Standardize,
                format!(
                    "Standardize {} variant(s) {:?} to '{}' in column '{}'.",
                    variants.len(),
                    variants,
                    target,
                    obs.column
                ),
            )
            .with_parameters(params)
            .with_affected_rows(affected_rows)
            .with_confidence(obs.confidence)
            .with_priority(2) // High priority - consistency is important
            .with_suggester("rule_engine"),
        )
    }

    /// Extract a standardization mapping from value_counts evidence.
    fn extract_standardization_mapping(value_counts: &Value, _column: &str) -> IndexMap<String, String> {
        let mut mapping = IndexMap::new();

        // Handle different evidence formats from validators
        if let Some(obj) = value_counts.as_object() {
            for (key, value) in obj {
                // Format 1: Case variant format - { "canonical": { "Variant1": count, "Variant2": count } }
                // The inner values should be integers (counts)
                if let Some(variant_obj) = value.as_object() {
                    // Check if this looks like a case variant format (values are counts)
                    let is_case_variant = variant_obj.values().all(|v| v.is_u64() || v.is_i64());

                    if is_case_variant && !variant_obj.is_empty() {
                        // Find the most common variant as the target
                        let mut max_count = 0u64;
                        let mut target = String::new();

                        for (variant, count_val) in variant_obj {
                            let count = count_val.as_u64().unwrap_or(0);
                            if count > max_count {
                                max_count = count;
                                target = variant.clone();
                            }
                        }

                        // Map all other variants to the most common one
                        for variant in variant_obj.keys() {
                            if variant != &target && !target.is_empty() {
                                mapping.insert(variant.clone(), target.clone());
                            }
                        }
                    }
                    // Format 2: Typo format - { "typoValue": { "suggestion": "correct", "count": N } }
                    else if let Some(suggestion) = variant_obj.get("suggestion").and_then(|s| s.as_str()) {
                        // Only add if it's actually a different value
                        if key != suggestion {
                            mapping.insert(key.clone(), suggestion.to_string());
                        }
                    }
                }
            }
        }

        mapping
    }

    /// Generate suggestion to flag outliers for review.
    fn suggest_flag_outlier(obs: &Observation) -> Option<Suggestion> {
        let sample_rows = &obs.evidence.sample_rows;
        let occurrences = obs.evidence.occurrences.unwrap_or(sample_rows.len());

        // Use severity to determine if this is a data entry error vs natural variation
        let is_error = obs.severity == Severity::Error;

        let params = json!({
            "column": obs.column,
            "rows": sample_rows,
            "flag_column": format!("{}_flagged", obs.column),
            "flag_value": if is_error { "invalid_value" } else { "outlier" },
        });

        let rationale = if is_error {
            format!(
                "Flag {} invalid value(s) in column '{}' for manual review. These values are likely data entry errors.",
                occurrences, obs.column
            )
        } else {
            format!(
                "Flag {} statistical outlier(s) in column '{}' for review. These may be valid extreme values or errors.",
                occurrences, obs.column
            )
        };

        Some(
            Suggestion::new(&obs.id, SuggestionAction::Flag, rationale)
                .with_parameters(params)
                .with_affected_rows(occurrences)
                .with_confidence(obs.confidence)
                .with_priority(if is_error { 2 } else { 4 })
                .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to handle duplicates.
    fn suggest_handle_duplicate(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        // For identifier columns, duplicates are errors - suggest flagging
        let params = json!({
            "column": obs.column,
            "rows": obs.evidence.sample_rows,
            "flag_column": format!("{}_duplicate", obs.column),
            "flag_value": "duplicate",
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Flag,
                format!(
                    "Flag {} duplicate value(s) in column '{}' for review. Duplicates in identifier columns may indicate data entry errors or need deduplication.",
                    occurrences, obs.column
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(1) // Highest priority - duplicates are often serious
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to handle type mismatches.
    fn suggest_handle_type_mismatch(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);
        let expected_type = obs
            .evidence
            .expected
            .as_ref()
            .and_then(|e| e.as_str())
            .unwrap_or("expected type");

        let pct = obs.evidence.percentage.unwrap_or(0.0);

        // If small percentage, suggest coercion; if large, suggest flagging
        if pct < 10.0 {
            let params = json!({
                "column": obs.column,
                "target_type": expected_type,
                "rows": obs.evidence.sample_rows,
            });

            Some(
                Suggestion::new(
                    &obs.id,
                    SuggestionAction::Coerce,
                    format!(
                        "Convert {} value(s) ({:.1}%) in column '{}' to {}. Non-convertible values will become NA.",
                        occurrences, pct, obs.column, expected_type
                    ),
                )
                .with_parameters(params)
                .with_affected_rows(occurrences)
                .with_confidence(obs.confidence * 0.9)
                .with_priority(3)
                .with_suggester("rule_engine"),
            )
        } else {
            let params = json!({
                "column": obs.column,
                "rows": obs.evidence.sample_rows,
                "flag_column": format!("{}_type_error", obs.column),
                "flag_value": "type_mismatch",
            });

            Some(
                Suggestion::new(
                    &obs.id,
                    SuggestionAction::Flag,
                    format!(
                        "Flag {} value(s) ({:.1}%) with type mismatches in column '{}'. Review before type coercion due to high error rate.",
                        occurrences, pct, obs.column
                    ),
                )
                .with_parameters(params)
                .with_affected_rows(occurrences)
                .with_confidence(obs.confidence)
                .with_priority(2)
                .with_suggester("rule_engine"),
            )
        }
    }

    /// Generate suggestion to flag constraint violations.
    fn suggest_flag_constraint(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        let params = json!({
            "column": obs.column,
            "rows": obs.evidence.sample_rows,
            "flag_column": format!("{}_constraint_violation", obs.column),
            "flag_value": "constraint_violation",
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Flag,
                format!(
                    "Flag {} value(s) violating constraints in column '{}' for review.",
                    occurrences, obs.column
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(2)
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion for completeness issues.
    fn suggest_flag_completeness(obs: &Observation) -> Option<Suggestion> {
        let pct = obs.evidence.percentage.unwrap_or(0.0);
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        // Only suggest action for high missing rates
        if pct < 20.0 {
            return None;
        }

        let params = json!({
            "column": obs.column,
            "missing_percentage": pct,
            "missing_count": occurrences,
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Flag,
                format!(
                    "Column '{}' has {:.1}% missing values ({} rows). Consider whether this column should be included in analysis or if missing data can be imputed.",
                    obs.column, pct, occurrences
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(4) // Lower priority - informational
            .with_reversible(true)
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to flag cross-column issues.
    fn suggest_flag_cross_column(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);

        let params = json!({
            "columns": obs.column, // May contain multiple column names
            "rows": obs.evidence.sample_rows,
            "flag_column": "cross_column_issue",
            "flag_value": "inconsistency",
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Flag,
                format!(
                    "Flag {} row(s) with cross-column inconsistency in {}. {}",
                    occurrences, obs.column, obs.description
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(2) // High priority - logical errors
            .with_suggester("rule_engine"),
        )
    }

    /// Generate suggestion to flag pattern violations.
    fn suggest_flag_pattern(obs: &Observation) -> Option<Suggestion> {
        let occurrences = obs.evidence.occurrences.unwrap_or(0);
        let pattern = obs
            .evidence
            .pattern
            .as_ref()
            .map(|p| p.as_str())
            .unwrap_or("expected pattern");

        let params = json!({
            "column": obs.column,
            "rows": obs.evidence.sample_rows,
            "expected_pattern": pattern,
            "flag_column": format!("{}_invalid_format", obs.column),
            "flag_value": "invalid_format",
        });

        Some(
            Suggestion::new(
                &obs.id,
                SuggestionAction::Flag,
                format!(
                    "Flag {} value(s) in column '{}' that don't match the expected format. Review and correct these entries.",
                    occurrences, obs.column
                ),
            )
            .with_parameters(params)
            .with_affected_rows(occurrences)
            .with_confidence(obs.confidence)
            .with_priority(3)
            .with_suggester("rule_engine"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::Evidence;

    #[test]
    fn test_generate_convert_na_suggestion() {
        let obs = Observation::new(
            ObservationType::MissingPattern,
            Severity::Warning,
            "status",
            "String 'missing' appears to represent NA values",
        )
        .with_evidence(
            Evidence::new()
                .with_pattern("missing")
                .with_occurrences(10)
                .with_percentage(5.0),
        )
        .with_confidence(0.9);

        let suggestion = SuggestionEngine::generate_for_observation(&obs).unwrap();

        assert_eq!(suggestion.action, SuggestionAction::ConvertNa);
        assert_eq!(suggestion.affected_rows, 10);
        assert!(suggestion.rationale.contains("missing"));
    }

    #[test]
    fn test_generate_standardize_suggestion() {
        let obs = Observation::new(
            ObservationType::Inconsistency,
            Severity::Warning,
            "diagnosis",
            "Case variants detected",
        )
        .with_evidence(
            Evidence::new()
                .with_occurrences(5)
                .with_value_counts(Some(json!({
                    "cd": { "CD": 10, "cd": 5 }
                }))),
        )
        .with_confidence(0.85);

        let suggestion = SuggestionEngine::generate_for_observation(&obs).unwrap();

        assert_eq!(suggestion.action, SuggestionAction::Standardize);
        assert!(suggestion.rationale.contains("Standardize"));
    }

    #[test]
    fn test_generate_flag_outlier_suggestion() {
        let obs = Observation::new(
            ObservationType::Outlier,
            Severity::Warning,
            "age",
            "Statistical outlier detected",
        )
        .with_evidence(
            Evidence::new()
                .with_occurrences(2)
                .with_sample_rows(vec![5, 12]),
        )
        .with_confidence(0.8);

        let suggestion = SuggestionEngine::generate_for_observation(&obs).unwrap();

        assert_eq!(suggestion.action, SuggestionAction::Flag);
        assert!(suggestion.rationale.contains("outlier"));
    }

    #[test]
    fn test_generate_multiple_suggestions() {
        let observations = vec![
            Observation::new(
                ObservationType::MissingPattern,
                Severity::Warning,
                "col1",
                "Missing pattern",
            )
            .with_evidence(Evidence::new().with_pattern("NA").with_occurrences(5)),
            Observation::new(
                ObservationType::Duplicate,
                Severity::Error,
                "col2",
                "Duplicates found",
            )
            .with_evidence(Evidence::new().with_occurrences(3)),
        ];

        let suggestions = SuggestionEngine::generate(&observations);

        assert_eq!(suggestions.len(), 2);
        // Should be sorted by priority - duplicates (priority 1) before NA conversion (priority 3)
        assert_eq!(suggestions[0].action, SuggestionAction::Flag); // Duplicate
        assert_eq!(suggestions[1].action, SuggestionAction::ConvertNa); // Missing pattern
    }
}
