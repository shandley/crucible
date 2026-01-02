//! Mock LLM provider for testing.

use crate::error::Result;
use crate::input::ContextHints;
use crate::schema::ColumnSchema;
use crate::suggestion::{Suggestion, SuggestionAction};
use crate::validation::{Observation, ObservationType};

use super::provider::{LlmConfig, LlmProvider, SchemaEnhancement};

/// Mock LLM provider that returns predictable responses for testing.
pub struct MockProvider {
    config: LlmConfig,
}

impl MockProvider {
    /// Create a new mock provider.
    pub fn new() -> Self {
        Self {
            config: LlmConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: LlmConfig) -> Self {
        Self { config }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmProvider for MockProvider {
    fn enhance_schema(
        &self,
        column: &ColumnSchema,
        _samples: &[String],
        context: &ContextHints,
    ) -> Result<SchemaEnhancement> {
        if !self.config.enhance_schema {
            return Ok(SchemaEnhancement {
                insight: String::new(),
                suggested_role: None,
                suggested_constraints: None,
                confidence: 0.0,
            });
        }

        // Generate mock insight based on column name and context
        let domain = context.domain.as_deref().unwrap_or("general");
        let insight = format!(
            "The '{}' column appears to be a {:?} in a {} context. \
             It contains {:?} data with {} unique values.",
            column.name,
            column.semantic_role,
            domain,
            column.inferred_type,
            column.statistics.unique_count
        );

        Ok(SchemaEnhancement {
            insight,
            suggested_role: None,
            suggested_constraints: None,
            confidence: 0.85,
        })
    }

    fn explain_observation(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        context: &ContextHints,
    ) -> Result<String> {
        if !self.config.explain_observations {
            return Ok(String::new());
        }

        let domain = context.domain.as_deref().unwrap_or("the dataset");
        let col_info = column
            .map(|c| format!("{:?}", c.inferred_type))
            .unwrap_or_else(|| "unknown".to_string());

        let explanation = match observation.observation_type {
            ObservationType::MissingPattern => {
                format!(
                    "The column '{}' contains values like 'missing' or 'NA' that represent \
                     missing data but aren't encoded as proper null values. In {}, this could \
                     cause issues with statistical analysis as these string values may be \
                     treated as valid categories.",
                    observation.column, domain
                )
            }
            ObservationType::Inconsistency => {
                format!(
                    "The column '{}' has inconsistent value formatting (e.g., mixed case or \
                     varying representations of the same concept). This {} column should have \
                     consistent formatting for reliable analysis.",
                    observation.column, col_info
                )
            }
            ObservationType::Outlier => {
                format!(
                    "The column '{}' contains values that fall significantly outside the \
                     expected range for this {} data. These outliers may represent data entry \
                     errors or legitimate edge cases that warrant investigation.",
                    observation.column, col_info
                )
            }
            ObservationType::TypeMismatch => {
                format!(
                    "The column '{}' was inferred as {} type, but contains values that don't \
                     match this type. This may indicate mixed data or encoding issues.",
                    observation.column, col_info
                )
            }
            ObservationType::Duplicate => {
                format!(
                    "The column '{}' appears to be an identifier but contains duplicate values. \
                     This could indicate data entry errors or issues with the data collection \
                     process.",
                    observation.column
                )
            }
            _ => {
                format!(
                    "The column '{}' has a {:?} issue that may affect data quality. \
                     Review the evidence to determine the appropriate action.",
                    observation.column, observation.observation_type
                )
            }
        };

        Ok(explanation)
    }

    fn generate_suggestion(
        &self,
        observation: &Observation,
        _column: Option<&ColumnSchema>,
        _context: &ContextHints,
    ) -> Result<Option<Suggestion>> {
        if !self.config.generate_suggestions {
            return Ok(None);
        }

        // Helper to extract specific details from the observation
        let col_name = &observation.column;
        let desc = &observation.description;

        let suggestion = match observation.observation_type {
            ObservationType::MissingPattern => {
                // Extract pattern from evidence if available
                let pattern_info = observation
                    .evidence
                    .pattern
                    .as_ref()
                    .map(|p| format!(" (pattern: '{}')", p))
                    .unwrap_or_default();

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::ConvertNa,
                        format!(
                            "In column '{}': Convert non-standard missing values{} to proper \
                             null values for consistent handling.",
                            col_name, pattern_info
                        ),
                    )
                    .with_priority(2)
                    .with_confidence(0.9)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Inconsistency => {
                // Build specific rationale based on the description
                let rationale = if desc.contains("Case variants") {
                    format!(
                        "In column '{}': Standardize case variants to a consistent format. {}",
                        col_name, desc
                    )
                } else if desc.contains("typo") {
                    format!(
                        "In column '{}': Review and correct potential typos. {}",
                        col_name, desc
                    )
                } else if desc.contains("semantic equivalent") {
                    format!(
                        "In column '{}': Standardize equivalent terms to canonical values. {}",
                        col_name, desc
                    )
                } else if desc.contains("date format") || desc.contains("Mixed date") {
                    format!(
                        "In column '{}': Standardize to a single date format (ISO recommended). {}",
                        col_name, desc
                    )
                } else if desc.contains("boolean") {
                    format!(
                        "In column '{}': Standardize boolean representations (recommend: true/false). {}",
                        col_name, desc
                    )
                } else {
                    format!(
                        "In column '{}': Standardize value formatting. {}",
                        col_name, desc
                    )
                };

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Standardize,
                        rationale,
                    )
                    .with_priority(3)
                    .with_confidence(0.85)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Outlier => {
                // Include sample values if available
                let value_info = observation
                    .evidence
                    .value
                    .as_ref()
                    .map(|v| format!(" Values: {}", v))
                    .unwrap_or_default();

                let row_info = if !observation.evidence.sample_rows.is_empty() {
                    format!(" (rows: {:?})", observation.evidence.sample_rows)
                } else {
                    String::new()
                };

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Flag,
                        format!(
                            "In column '{}': Review outlier values for validity.{}{} \
                             These may be data entry errors or valid edge cases.",
                            col_name, value_info, row_info
                        ),
                    )
                    .with_priority(4)
                    .with_confidence(0.7)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::TypeMismatch => {
                let row_info = if !observation.evidence.sample_rows.is_empty() {
                    format!(" (rows: {:?})", observation.evidence.sample_rows)
                } else {
                    String::new()
                };

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Coerce,
                        format!(
                            "In column '{}': {} Attempt type coercion or flag for manual review.",
                            col_name, desc
                        ) + &row_info,
                    )
                    .with_priority(3)
                    .with_confidence(0.75)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Duplicate => {
                // Extract duplicate values from evidence
                let dup_info = observation
                    .evidence
                    .value
                    .as_ref()
                    .map(|v| format!(" Duplicates: {}", v))
                    .unwrap_or_default();

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Flag,
                        format!(
                            "In column '{}': Review duplicate values.{} Determine if entries \
                             should be merged, removed, or are valid repeats.",
                            col_name, dup_info
                        ),
                    )
                    .with_priority(2)
                    .with_confidence(0.8)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Completeness => {
                let pct = observation
                    .evidence
                    .percentage
                    .map(|p| format!(" ({:.1}% missing)", p))
                    .unwrap_or_default();

                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Flag,
                        format!(
                            "In column '{}': High rate of missing values.{} Consider \
                             imputation strategy or documenting missingness pattern.",
                            col_name, pct
                        ),
                    )
                    .with_priority(3)
                    .with_confidence(0.7)
                    .with_suggester("mock_llm"),
                )
            }
            _ => None,
        };

        Ok(suggestion)
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{ColumnStatistics, ColumnType, SemanticRole, SemanticType};
    use crate::validation::Severity;

    fn make_test_column() -> ColumnSchema {
        ColumnSchema {
            name: "test_column".to_string(),
            position: 0,
            inferred_type: ColumnType::String,
            semantic_type: SemanticType::Categorical,
            semantic_role: SemanticRole::Grouping,
            nullable: false,
            unique: false,
            expected_values: None,
            expected_range: None,
            constraints: vec![],
            statistics: ColumnStatistics {
                count: 100,
                null_count: 0,
                unique_count: 5,
                ..Default::default()
            },
            confidence: 0.9,
            inference_sources: vec!["statistical".to_string()],
            llm_insight: None,
        }
    }

    fn make_test_observation() -> Observation {
        Observation::new(
            ObservationType::MissingPattern,
            Severity::Warning,
            "status",
            "String 'missing' appears to represent NA values",
        )
        .with_confidence(0.9)
        .with_detector("test")
    }

    #[test]
    fn test_mock_enhance_schema() {
        let provider = MockProvider::new();
        let column = make_test_column();
        let context = ContextHints::new().with_domain("biomedical");

        let result = provider
            .enhance_schema(&column, &["A".to_string(), "B".to_string()], &context)
            .unwrap();

        assert!(!result.insight.is_empty());
        assert!(result.insight.contains("test_column"));
        assert!(result.insight.contains("biomedical"));
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn test_mock_explain_observation() {
        let provider = MockProvider::new();
        let observation = make_test_observation();
        let context = ContextHints::new();

        let explanation = provider
            .explain_observation(&observation, None, &context)
            .unwrap();

        assert!(!explanation.is_empty());
        assert!(explanation.contains("status"));
        assert!(explanation.contains("missing"));
    }

    #[test]
    fn test_mock_generate_suggestion() {
        let provider = MockProvider::new();
        let observation = make_test_observation();
        let context = ContextHints::new();

        let suggestion = provider
            .generate_suggestion(&observation, None, &context)
            .unwrap();

        assert!(suggestion.is_some());
        let sug = suggestion.unwrap();
        assert_eq!(sug.action, SuggestionAction::ConvertNa);
        assert!(!sug.rationale.is_empty());
    }

    #[test]
    fn test_disabled_features() {
        let config = LlmConfig {
            enhance_schema: false,
            explain_observations: false,
            generate_suggestions: false,
            ..Default::default()
        };
        let provider = MockProvider::with_config(config);
        let column = make_test_column();
        let observation = make_test_observation();
        let context = ContextHints::new();

        let enhancement = provider
            .enhance_schema(&column, &[], &context)
            .unwrap();
        assert!(enhancement.insight.is_empty());

        let explanation = provider
            .explain_observation(&observation, None, &context)
            .unwrap();
        assert!(explanation.is_empty());

        let suggestion = provider
            .generate_suggestion(&observation, None, &context)
            .unwrap();
        assert!(suggestion.is_none());
    }
}
