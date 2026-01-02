//! Prompt templates for LLM interactions.

use crate::input::ContextHints;
use crate::schema::ColumnSchema;
use crate::validation::Observation;

/// Build a prompt for schema enhancement.
pub fn schema_enhancement_prompt(
    column: &ColumnSchema,
    samples: &[String],
    context: &ContextHints,
) -> String {
    let sample_str = if samples.is_empty() {
        "No samples available".to_string()
    } else {
        samples
            .iter()
            .take(10)
            .map(|s| format!("  - \"{}\"", s))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let stats_str = if let Some(ref numeric) = column.statistics.numeric {
        format!(
            "Numeric stats: min={:.2}, max={:.2}, mean={:.2}, median={:.2}",
            numeric.min, numeric.max, numeric.mean, numeric.median
        )
    } else {
        format!(
            "Cardinality: {} unique values out of {} total",
            column.statistics.unique_count, column.statistics.count
        )
    };

    format!(
        r#"Analyze this column from a tabular dataset and provide insights.

## Column Information
- Name: {}
- Inferred type: {:?}
- Inferred semantic role: {:?}
- Nullable: {}
- Unique: {}
- {}

## Sample Values
{}

## Context
{}

## Task
Provide a concise insight (1-2 sentences) about what this column likely represents,
considering the column name, data type, sample values, and any context provided.
Focus on domain-specific meaning that would help a data analyst understand the column.

If the semantic role seems incorrect based on your analysis, suggest a better role.
If you notice potential data quality issues, briefly mention them.

Respond with a JSON object:
{{
  "insight": "Your insight about the column...",
  "suggested_role": null or "Identifier|Grouping|Covariate|Outcome|Metadata",
  "potential_issues": null or "Brief description of any issues noticed",
  "confidence": 0.0-1.0
}}"#,
        column.name,
        column.inferred_type,
        column.semantic_role,
        column.nullable,
        column.unique,
        stats_str,
        sample_str,
        context.to_prompt_string()
    )
}

/// Build a prompt for observation explanation.
pub fn observation_explanation_prompt(
    observation: &Observation,
    column: Option<&ColumnSchema>,
    context: &ContextHints,
) -> String {
    let column_info = if let Some(col) = column {
        format!(
            "Column '{}' (type: {:?}, role: {:?})",
            col.name, col.inferred_type, col.semantic_role
        )
    } else {
        format!("Column '{}'", observation.column)
    };

    let evidence_str = serde_json::to_string_pretty(&observation.evidence)
        .unwrap_or_else(|_| "Unable to serialize evidence".to_string());

    format!(
        r#"Explain this data quality issue in clear, actionable language.

## Issue Details
- Type: {:?}
- Severity: {:?}
- {column_info}
- Description: {}

## Evidence
{}

## Context
{}

## Task
Provide a clear, 2-3 sentence explanation of this issue that:
1. Explains what the problem is in plain language
2. Suggests why it might have occurred
3. Indicates the potential impact if not addressed

Be specific and reference the actual data when possible.
Write for a data analyst who needs to decide whether to fix this issue."#,
        observation.observation_type,
        observation.severity,
        observation.description,
        evidence_str,
        context.to_prompt_string()
    )
}

/// Build a prompt for suggestion generation.
pub fn suggestion_prompt(
    observation: &Observation,
    column: Option<&ColumnSchema>,
    context: &ContextHints,
) -> String {
    let column_info = if let Some(col) = column {
        format!(
            "Column '{}' (type: {:?}, role: {:?})",
            col.name, col.inferred_type, col.semantic_role
        )
    } else {
        format!("Column '{}'", observation.column)
    };

    let evidence_str = serde_json::to_string_pretty(&observation.evidence)
        .unwrap_or_else(|_| "Unable to serialize evidence".to_string());

    format!(
        r#"Suggest a fix for this data quality issue.

## Issue Details
- Type: {:?}
- Severity: {:?}
- {column_info}
- Description: {}

## Evidence
{}

## Context
{}

## Available Actions
- standardize: Normalize format, case, or encoding
- convert_na: Convert string values to proper NA/null
- coerce: Type conversion (e.g., string to number)
- flag: Add a flag column for human review
- remove: Remove problematic rows
- merge: Combine duplicate entries

## Task
Suggest the most appropriate fix for this issue. Consider:
1. The severity and impact of the issue
2. Whether the fix is reversible
3. How many rows are affected
4. The domain context

Respond with a JSON object:
{{
  "action": "standardize|convert_na|coerce|flag|remove|merge",
  "rationale": "Clear explanation of why this fix is recommended...",
  "parameters": {{ action-specific parameters }},
  "confidence": 0.0-1.0,
  "priority": 1-10 (1=highest)
}}

If no fix is appropriate (e.g., the issue is informational only), respond with:
{{
  "action": null,
  "rationale": "Explanation of why no fix is needed..."
}}"#,
        observation.observation_type,
        observation.severity,
        observation.description,
        evidence_str,
        context.to_prompt_string()
    )
}

/// System prompt for all Crucible LLM interactions.
pub fn system_prompt() -> &'static str {
    r#"You are a data quality expert assistant for Crucible, an LLM-native data curation tool.

Your role is to:
1. Analyze tabular data columns and provide semantic insights
2. Explain data quality issues in clear, actionable language
3. Suggest appropriate fixes for data problems

Guidelines:
- Be concise and specific
- Reference actual data values when explaining issues
- Consider domain context when making suggestions
- Prioritize data integrity and analyst productivity
- When uncertain, recommend flagging for human review rather than automatic fixes
- Always respond with valid JSON when requested

You have expertise in:
- Biomedical/clinical research data
- Survey and questionnaire data
- Financial and business data
- General tabular data patterns"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColumnStatistics, ColumnType, SemanticRole, SemanticType};

    #[test]
    fn test_schema_prompt_generation() {
        let column = ColumnSchema {
            name: "age".to_string(),
            position: 0,
            inferred_type: ColumnType::Integer,
            semantic_type: SemanticType::Continuous,
            semantic_role: SemanticRole::Covariate,
            nullable: false,
            unique: false,
            expected_values: None,
            expected_range: Some((0.0, 100.0)),
            constraints: vec![],
            statistics: ColumnStatistics::default(),
            confidence: 0.9,
            inference_sources: vec!["statistical".to_string()],
            llm_insight: None,
        };

        let samples = vec!["25".to_string(), "30".to_string(), "28".to_string()];
        let context = ContextHints::new().with_domain("biomedical");

        let prompt = schema_enhancement_prompt(&column, &samples, &context);

        assert!(prompt.contains("age"));
        assert!(prompt.contains("Integer"));
        assert!(prompt.contains("biomedical"));
    }
}
