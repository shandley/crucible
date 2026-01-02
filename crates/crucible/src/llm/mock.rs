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

        let suggestion = match observation.observation_type {
            ObservationType::MissingPattern => {
                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::ConvertNa,
                        "Convert non-standard missing value representations to proper null values \
                         for correct statistical handling.",
                    )
                    .with_priority(2)
                    .with_confidence(0.9)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Inconsistency => {
                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Standardize,
                        "Standardize value formatting to ensure consistent representation \
                         across the dataset.",
                    )
                    .with_priority(3)
                    .with_confidence(0.85)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Outlier => {
                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Flag,
                        "Flag outlier values for manual review rather than automatic removal, \
                         as they may represent valid edge cases.",
                    )
                    .with_priority(4)
                    .with_confidence(0.7)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::TypeMismatch => {
                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Coerce,
                        "Attempt type coercion for mismatched values, or flag for review if \
                         coercion fails.",
                    )
                    .with_priority(3)
                    .with_confidence(0.75)
                    .with_suggester("mock_llm"),
                )
            }
            ObservationType::Duplicate => {
                Some(
                    Suggestion::new(
                        &observation.id,
                        SuggestionAction::Flag,
                        "Flag duplicate entries for manual review to determine if they should \
                         be merged or removed.",
                    )
                    .with_priority(2)
                    .with_confidence(0.8)
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
