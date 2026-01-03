//! Transformation engine that applies curation decisions to data.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::curation::{CurationLayer, DecisionStatus};
use crate::error::{CrucibleError, Result};
use crate::input::DataTable;
use crate::suggestion::SuggestionAction;
use crate::validation::ObservationType;

use super::operations::{RowAudit, TransformChange, TransformOperation, TransformResult};

/// Engine for applying transformations to data based on curation decisions.
pub struct TransformEngine;

impl TransformEngine {
    /// Create a new transform engine.
    pub fn new() -> Self {
        Self
    }

    /// Apply all accepted decisions from a curation layer to the source data.
    pub fn apply(&self, curation: &CurationLayer, data: &mut DataTable) -> Result<TransformResult> {
        let mut result = TransformResult::new();

        // Get all accepted/modified decisions
        let approved_decisions: Vec<_> = curation
            .decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Accepted || d.status == DecisionStatus::Modified)
            .collect();

        for decision in approved_decisions {
            // Find the corresponding suggestion
            let suggestion = curation
                .suggestion(&decision.suggestion_id)
                .ok_or_else(|| {
                    CrucibleError::Validation(format!(
                        "Suggestion '{}' not found",
                        decision.suggestion_id
                    ))
                })?;

            // Find the corresponding observation
            let observation = curation
                .observation(&suggestion.observation_id)
                .ok_or_else(|| {
                    CrucibleError::Validation(format!(
                        "Observation '{}' not found",
                        suggestion.observation_id
                    ))
                })?;

            // Generate and apply the transformation
            let operation = self.create_operation(suggestion, observation, curation, data)?;

            if let Some(op) = operation {
                let change = self.apply_operation(&op, data)?;
                result.add_change(change);
            }
        }

        Ok(result)
    }

    /// Create a transformation operation from a suggestion and observation.
    fn create_operation(
        &self,
        suggestion: &crate::suggestion::Suggestion,
        observation: &crate::validation::Observation,
        _curation: &CurationLayer,
        data: &DataTable,
    ) -> Result<Option<TransformOperation>> {
        match suggestion.action {
            SuggestionAction::Standardize => {
                self.create_standardize_operation(suggestion, observation, data)
            }
            SuggestionAction::Flag => self.create_flag_operation(suggestion, observation),
            SuggestionAction::ConvertNa => self.create_convert_na_operation(suggestion, observation),
            SuggestionAction::Coerce => self.create_coerce_operation(suggestion, observation, data),
            SuggestionAction::Remove => Ok(Some(TransformOperation::NoOp {
                reason: "Remove operations require manual review".to_string(),
            })),
            SuggestionAction::Merge => Ok(Some(TransformOperation::NoOp {
                reason: "Merge operations require manual review".to_string(),
            })),
            SuggestionAction::Rename => Ok(Some(TransformOperation::NoOp {
                reason: "Rename operations not yet implemented".to_string(),
            })),
            SuggestionAction::Split => Ok(Some(TransformOperation::NoOp {
                reason: "Split operations not yet implemented".to_string(),
            })),
            SuggestionAction::Derive => Ok(Some(TransformOperation::NoOp {
                reason: "Derive operations not yet implemented".to_string(),
            })),
        }
    }

    /// Create a standardize operation from a suggestion and observation.
    fn create_standardize_operation(
        &self,
        suggestion: &crate::suggestion::Suggestion,
        observation: &crate::validation::Observation,
        data: &DataTable,
    ) -> Result<Option<TransformOperation>> {
        // Get column from suggestion parameters or observation
        let column = suggestion
            .parameters
            .get("column")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| observation.column.clone());

        // Try to get mapping from suggestion parameters first
        let mut mapping = HashMap::new();
        if let Some(param_mapping) = suggestion.parameters.get("mapping") {
            if let Some(obj) = param_mapping.as_object() {
                for (from, to) in obj {
                    if let Some(to_str) = to.as_str() {
                        mapping.insert(from.clone(), to_str.to_string());
                    }
                }
            }
        }

        // Fall back to extracting from observation evidence
        if mapping.is_empty() {
            mapping = match observation.observation_type {
                ObservationType::Inconsistency => {
                    self.extract_standardize_mapping(observation, data)?
                }
                _ => HashMap::new(),
            };
        }

        if mapping.is_empty() {
            return Ok(None);
        }

        Ok(Some(TransformOperation::Standardize { column, mapping }))
    }

    /// Extract a value mapping from inconsistency observation evidence.
    fn extract_standardize_mapping(
        &self,
        observation: &crate::validation::Observation,
        data: &DataTable,
    ) -> Result<HashMap<String, String>> {
        let mut mapping = HashMap::new();

        // Parse the value_counts from evidence to find variant groups
        if let Some(ref value_counts) = observation.evidence.value_counts {
            // Handle case variants: {"lowercase": {"Original1": count, "Original2": count}}
            if let Some(groups) = value_counts.as_object() {
                for (canonical, variants) in groups {
                    if let Some(variant_map) = variants.as_object() {
                        // Find the most common variant to use as canonical
                        let mut max_count = 0;
                        let mut best_canonical = canonical.clone();

                        for (variant, count) in variant_map {
                            let c = count.as_u64().unwrap_or(0);
                            if c > max_count {
                                max_count = c;
                                best_canonical = variant.clone();
                            }
                        }

                        // Map all variants to the canonical form
                        for variant in variant_map.keys() {
                            if variant != &best_canonical {
                                mapping.insert(variant.clone(), best_canonical.clone());
                            }
                        }
                    }
                }
            }
        }

        // If no mapping found from evidence, try to infer from column values
        if mapping.is_empty() {
            // Check if this is about case variants or typos
            let desc = &observation.description;

            if desc.contains("Case variants") || desc.contains("case variant") {
                // Find column index
                if let Some(col_idx) = data.column_index(&observation.column) {
                    mapping = self.infer_case_variant_mapping(data, col_idx);
                }
            } else if desc.contains("typo") {
                // Parse typo suggestions from description
                mapping = self.parse_typo_mapping(desc);
            } else if desc.contains("semantic equivalent") {
                // Parse semantic equivalents
                mapping = self.parse_semantic_mapping(desc, observation);
            } else if desc.contains("date format") || desc.contains("Mixed date") {
                // For date formats, we'll need special handling
                // For now, skip - would need actual date parsing
                return Ok(HashMap::new());
            }
        }

        Ok(mapping)
    }

    /// Infer a case variant mapping from column data.
    fn infer_case_variant_mapping(&self, data: &DataTable, col_idx: usize) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        // Group values by lowercase form
        let mut groups: IndexMap<String, IndexMap<String, usize>> = IndexMap::new();
        for value in data.column_values(col_idx) {
            let trimmed = value.trim();
            if trimmed.is_empty() || DataTable::is_null_value(trimmed) {
                continue;
            }
            let lower = trimmed.to_lowercase();
            *groups
                .entry(lower)
                .or_default()
                .entry(trimmed.to_string())
                .or_insert(0) += 1;
        }

        // For groups with multiple case variants, map to the most common one
        for (_lower, variants) in groups {
            if variants.len() <= 1 {
                continue;
            }

            // Find most common variant
            let canonical = variants
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(val, _)| val.clone())
                .unwrap_or_default();

            // Map others to canonical
            for (variant, _) in variants {
                if variant != canonical {
                    mapping.insert(variant, canonical.clone());
                }
            }
        }

        mapping
    }

    /// Parse typo corrections from the description.
    fn parse_typo_mapping(&self, desc: &str) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        // Look for patterns like 'typo' → 'correct'
        // The description format is: "1 potential typo(s) detected: 'infliximab' → 'Infliximab'"
        for segment in desc.split(", ") {
            if let Some(arrow_pos) = segment.find(" → ") {
                // Find the quoted strings
                if let Some(start) = segment.find('\'') {
                    let rest = &segment[start + 1..];
                    if let Some(end) = rest.find('\'') {
                        let typo = &rest[..end];
                        let after_arrow = &segment[arrow_pos + 5..]; // " → '" is 5 chars
                        if let Some(correct_start) = after_arrow.find('\'') {
                            let after_quote = &after_arrow[correct_start + 1..];
                            if let Some(correct_end) = after_quote.find('\'') {
                                let correct = &after_quote[..correct_end];
                                mapping.insert(typo.to_string(), correct.to_string());
                            }
                        }
                    }
                }
            }
        }

        mapping
    }

    /// Parse semantic equivalent mapping from observation.
    fn parse_semantic_mapping(
        &self,
        _desc: &str,
        observation: &crate::validation::Observation,
    ) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        // Parse from evidence.value_counts which has format:
        // {"canonical": {"variant1": count, "variant2": count}}
        if let Some(ref value_counts) = observation.evidence.value_counts {
            if let Some(groups) = value_counts.as_object() {
                for (canonical, variants) in groups {
                    if let Some(variant_map) = variants.as_object() {
                        for variant in variant_map.keys() {
                            if variant != canonical {
                                mapping.insert(variant.clone(), canonical.clone());
                            }
                        }
                    }
                }
            }
        }

        mapping
    }

    /// Create a flag operation from a suggestion and observation.
    fn create_flag_operation(
        &self,
        suggestion: &crate::suggestion::Suggestion,
        observation: &crate::validation::Observation,
    ) -> Result<Option<TransformOperation>> {
        // Get parameters from suggestion or defaults
        let source_column = suggestion
            .parameters
            .get("column")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| observation.column.clone());

        let flag_column = suggestion
            .parameters
            .get("flag_column")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}_flagged", source_column));

        let flag_value = suggestion
            .parameters
            .get("flag_value")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "REVIEW".to_string());

        // Get rows from suggestion parameters or observation evidence
        let rows: Vec<usize> = suggestion
            .parameters
            .get("rows")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_else(|| observation.evidence.sample_rows.clone());

        if rows.is_empty() {
            return Ok(Some(TransformOperation::NoOp {
                reason: "No rows to flag".to_string(),
            }));
        }

        Ok(Some(TransformOperation::Flag {
            source_column,
            flag_column,
            rows,
            flag_value,
        }))
    }

    /// Create a convert NA operation from a suggestion and observation.
    fn create_convert_na_operation(
        &self,
        suggestion: &crate::suggestion::Suggestion,
        observation: &crate::validation::Observation,
    ) -> Result<Option<TransformOperation>> {
        // Get column from suggestion parameters or observation
        let column = suggestion
            .parameters
            .get("column")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| observation.column.clone());

        // Get values to convert from suggestion parameters first
        let values: Vec<String> = suggestion
            .parameters
            .get("from_values")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| {
                // Fall back to observation evidence
                if let Some(ref pattern) = observation.evidence.pattern {
                    vec![pattern.clone()]
                } else {
                    Vec::new()
                }
            });

        if values.is_empty() {
            return Ok(Some(TransformOperation::NoOp {
                reason: "No values to convert to NA".to_string(),
            }));
        }

        Ok(Some(TransformOperation::ConvertNa { column, values }))
    }

    /// Create a coerce operation for type conversion.
    fn create_coerce_operation(
        &self,
        suggestion: &crate::suggestion::Suggestion,
        observation: &crate::validation::Observation,
        _data: &DataTable,
    ) -> Result<Option<TransformOperation>> {
        // Get column from suggestion parameters or observation
        let column = suggestion
            .parameters
            .get("column")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| observation.column.clone());

        // Get target type from suggestion parameters
        let target_type = suggestion
            .parameters
            .get("target_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "string".to_string());

        // Get rows with type issues from suggestion parameters or observation evidence
        let rows: Vec<usize> = suggestion
            .parameters
            .get("rows")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_else(|| observation.evidence.sample_rows.clone());

        if rows.is_empty() {
            return Ok(Some(TransformOperation::NoOp {
                reason: "No values to coerce".to_string(),
            }));
        }

        Ok(Some(TransformOperation::Coerce {
            column,
            target_type,
            rows,
        }))
    }

    /// Apply a transformation operation to the data.
    fn apply_operation(
        &self,
        operation: &TransformOperation,
        data: &mut DataTable,
    ) -> Result<TransformChange> {
        match operation {
            TransformOperation::Standardize { column, mapping } => {
                self.apply_standardize(column, mapping, data)
            }
            TransformOperation::Flag {
                source_column,
                flag_column,
                rows,
                flag_value,
            } => self.apply_flag(source_column, flag_column, rows, flag_value, data),
            TransformOperation::ConvertNa { column, values } => {
                self.apply_convert_na(column, values, data)
            }
            TransformOperation::Coerce {
                column,
                target_type,
                rows,
            } => self.apply_coerce(column, target_type, rows, data),
            TransformOperation::NoOp { reason } => Ok(TransformChange {
                description: format!("Skipped: {}", reason),
                column: String::new(),
                values_changed: 0,
                row_audits: Vec::new(),
            }),
        }
    }

    /// Apply a standardize transformation.
    fn apply_standardize(
        &self,
        column: &str,
        mapping: &HashMap<String, String>,
        data: &mut DataTable,
    ) -> Result<TransformChange> {
        let col_idx = data.column_index(column).ok_or_else(|| {
            CrucibleError::Validation(format!("Column '{}' not found", column))
        })?;

        let mut changed = 0;
        let mut row_audits = Vec::new();

        for row_idx in 0..data.row_count() {
            let value = data.get(row_idx, col_idx).unwrap_or_default().to_string();
            if let Some(new_value) = mapping.get(&value) {
                row_audits.push(RowAudit {
                    row: row_idx,
                    column: column.to_string(),
                    original_value: value.clone(),
                    new_value: new_value.clone(),
                    transform_type: "standardize".to_string(),
                    reason: format!("Normalized '{}' to '{}'", value, new_value),
                });
                data.set(row_idx, col_idx, new_value.clone());
                changed += 1;
            }
        }

        let examples: Vec<String> = mapping
            .iter()
            .take(2)
            .map(|(from, to)| format!("'{}' → '{}'", from, to))
            .collect();

        Ok(TransformChange {
            description: format!("Standardized '{}': {}", column, examples.join(", ")),
            column: column.to_string(),
            values_changed: changed,
            row_audits,
        })
    }

    /// Apply a flag transformation.
    fn apply_flag(
        &self,
        source_column: &str,
        flag_column: &str,
        rows: &[usize],
        flag_value: &str,
        data: &mut DataTable,
    ) -> Result<TransformChange> {
        // Add the flag column if it doesn't exist
        if data.column_index(flag_column).is_none() {
            data.add_column(flag_column.to_string(), String::new());
        }

        let flag_col_idx = data.column_index(flag_column).unwrap();
        let mut row_audits = Vec::new();

        // Set flag values for specified rows
        for &row_idx in rows {
            if row_idx < data.row_count() {
                row_audits.push(RowAudit {
                    row: row_idx,
                    column: flag_column.to_string(),
                    original_value: String::new(),
                    new_value: flag_value.to_string(),
                    transform_type: "flag".to_string(),
                    reason: format!("Flagged for review: issue in '{}'", source_column),
                });
                data.set(row_idx, flag_col_idx, flag_value.to_string());
            }
        }

        Ok(TransformChange {
            description: format!(
                "Flagged {} rows in '{}' → '{}'",
                rows.len(),
                source_column,
                flag_column
            ),
            column: flag_column.to_string(),
            values_changed: rows.len(),
            row_audits,
        })
    }

    /// Apply a convert NA transformation.
    fn apply_convert_na(
        &self,
        column: &str,
        values: &[String],
        data: &mut DataTable,
    ) -> Result<TransformChange> {
        let col_idx = data.column_index(column).ok_or_else(|| {
            CrucibleError::Validation(format!("Column '{}' not found", column))
        })?;

        let mut changed = 0;
        let mut row_audits = Vec::new();

        for row_idx in 0..data.row_count() {
            let value = data.get(row_idx, col_idx).unwrap_or_default().to_string();
            let lower = value.to_lowercase();
            if values.iter().any(|v| v.to_lowercase() == lower) {
                row_audits.push(RowAudit {
                    row: row_idx,
                    column: column.to_string(),
                    original_value: value.clone(),
                    new_value: String::new(),
                    transform_type: "convert_na".to_string(),
                    reason: format!("Converted '{}' to NA (missing value pattern)", value),
                });
                data.set(row_idx, col_idx, String::new());
                changed += 1;
            }
        }

        Ok(TransformChange {
            description: format!("Converted {:?} to NA in '{}'", values, column),
            column: column.to_string(),
            values_changed: changed,
            row_audits,
        })
    }

    /// Apply a type coercion transformation.
    fn apply_coerce(
        &self,
        column: &str,
        target_type: &str,
        rows: &[usize],
        data: &mut DataTable,
    ) -> Result<TransformChange> {
        let col_idx = data.column_index(column).ok_or_else(|| {
            CrucibleError::Validation(format!("Column '{}' not found", column))
        })?;

        let mut changed = 0;
        let mut row_audits = Vec::new();

        for &row_idx in rows {
            if row_idx >= data.row_count() {
                continue;
            }

            let value = data.get(row_idx, col_idx).unwrap_or_default().to_string();
            let trimmed = value.trim();

            // Skip empty/null values
            if trimmed.is_empty() || DataTable::is_null_value(trimmed) {
                continue;
            }

            // Try to coerce based on target type
            let coerced = match target_type {
                "integer" | "Integer" => {
                    if trimmed.parse::<i64>().is_ok() {
                        Some(trimmed.to_string())
                    } else if let Some(f) = trimmed.parse::<f64>().ok() {
                        // Try to convert float to int if it's a whole number
                        if f.fract() == 0.0 {
                            Some(format!("{}", f as i64))
                        } else {
                            None // Can't cleanly convert
                        }
                    } else {
                        None // Convert to NA
                    }
                }
                "float" | "Float" => {
                    if trimmed.parse::<f64>().is_ok() {
                        Some(trimmed.to_string())
                    } else {
                        None // Convert to NA
                    }
                }
                "boolean" | "Boolean" => {
                    let lower = trimmed.to_lowercase();
                    if matches!(
                        lower.as_str(),
                        "true" | "false" | "yes" | "no" | "1" | "0" | "t" | "f" | "y" | "n"
                    ) {
                        // Standardize to true/false
                        let is_true = matches!(lower.as_str(), "true" | "yes" | "1" | "t" | "y");
                        Some(if is_true { "true" } else { "false" }.to_string())
                    } else {
                        None
                    }
                }
                _ => Some(trimmed.to_string()), // String type - keep as is
            };

            match coerced {
                Some(ref new_value) if new_value != trimmed => {
                    row_audits.push(RowAudit {
                        row: row_idx,
                        column: column.to_string(),
                        original_value: trimmed.to_string(),
                        new_value: new_value.clone(),
                        transform_type: "coerce".to_string(),
                        reason: format!("Coerced '{}' to {} type", trimmed, target_type),
                    });
                    data.set(row_idx, col_idx, new_value.clone());
                    changed += 1;
                }
                None => {
                    row_audits.push(RowAudit {
                        row: row_idx,
                        column: column.to_string(),
                        original_value: trimmed.to_string(),
                        new_value: String::new(),
                        transform_type: "coerce".to_string(),
                        reason: format!(
                            "Converted '{}' to NA (could not coerce to {})",
                            trimmed, target_type
                        ),
                    });
                    // Convert non-coercible values to NA
                    data.set(row_idx, col_idx, String::new());
                    changed += 1;
                }
                _ => {} // Value unchanged
            }
        }

        Ok(TransformChange {
            description: format!(
                "Coerced {} value(s) in '{}' to {}",
                changed, column, target_type
            ),
            column: column.to_string(),
            values_changed: changed,
            row_audits,
        })
    }
}

impl Default for TransformEngine {
    fn default() -> Self {
        Self::new()
    }
}
