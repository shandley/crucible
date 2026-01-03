//! Main Crucible struct and public API.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::inference::{FusionConfig, InferenceFusion};
use crate::input::{ContextHints, Parser, ParserConfig, SourceMetadata};
use crate::llm::LlmProvider;
use crate::schema::TableSchema;
use crate::suggestion::{Suggestion, SuggestionEngine};
use crate::validation::{Observation, ValidationEngine};

/// Configuration for Crucible analysis.
#[derive(Debug, Clone)]
pub struct CrucibleConfig {
    /// Parser configuration.
    pub parser: ParserConfig,
    /// Inference fusion configuration.
    pub fusion: FusionConfig,
    /// Maximum rows to analyze (None = all).
    pub max_rows: Option<usize>,
    /// Context hints for LLM enhancement.
    pub context: ContextHints,
}

impl Default for CrucibleConfig {
    fn default() -> Self {
        Self {
            parser: ParserConfig::default(),
            fusion: FusionConfig::default(),
            max_rows: None,
            context: ContextHints::default(),
        }
    }
}

/// Result of analyzing a data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Metadata about the source file.
    pub source: SourceMetadata,
    /// Inferred schema for the table.
    pub schema: TableSchema,
    /// Data quality observations.
    pub observations: Vec<Observation>,
    /// Suggested fixes for observations (LLM-generated if enabled).
    pub suggestions: Vec<Suggestion>,
    /// Summary statistics.
    pub summary: AnalysisSummary,
}

/// Summary of the analysis results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Total number of columns.
    pub total_columns: usize,
    /// Number of columns with at least one observation.
    pub columns_with_issues: usize,
    /// Total number of observations.
    pub total_observations: usize,
    /// Observations by severity.
    pub observations_by_severity: ObservationCounts,
    /// Observations by type.
    pub observations_by_type: std::collections::HashMap<String, usize>,
    /// Data quality score (0.0-1.0).
    pub data_quality_score: f64,
    /// Human-readable recommendation.
    pub recommendation: String,
}

/// Counts of observations by severity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservationCounts {
    pub error: usize,
    pub warning: usize,
    pub info: usize,
}

/// The main Crucible analysis engine.
pub struct Crucible {
    config: CrucibleConfig,
    parser: Parser,
    inference: InferenceFusion,
    validation: ValidationEngine,
    llm_provider: Option<Arc<dyn LlmProvider>>,
}

impl Crucible {
    /// Create a new Crucible instance with default configuration.
    pub fn new() -> Self {
        Self::with_config(CrucibleConfig::default())
    }

    /// Create a Crucible instance with custom configuration.
    pub fn with_config(config: CrucibleConfig) -> Self {
        let parser = Parser::with_config(config.parser.clone());
        let inference = InferenceFusion::with_config(config.fusion.clone());
        let validation = ValidationEngine::new();

        Self {
            config,
            parser,
            inference,
            validation,
            llm_provider: None,
        }
    }

    /// Add an LLM provider for enhanced inference and explanations.
    ///
    /// When an LLM provider is configured, Crucible will:
    /// - Enhance column schemas with semantic insights
    /// - Generate human-readable explanations for observations
    /// - Produce actionable suggestions for data quality issues
    pub fn with_llm(mut self, provider: impl LlmProvider + 'static) -> Self {
        self.llm_provider = Some(Arc::new(provider));
        self
    }

    /// Set context hints for LLM enhancement.
    ///
    /// Context hints help the LLM provide more relevant insights
    /// by understanding the domain and purpose of the data.
    pub fn with_context(mut self, context: ContextHints) -> Self {
        self.config.context = context;
        self
    }

    /// Analyze a data file and produce observations.
    pub fn analyze(&self, path: impl AsRef<Path>) -> Result<AnalysisResult> {
        let path = path.as_ref();

        // Parse the file
        let (table, source) = self.parser.parse_file(path)?;

        // Run inference to get schema
        let mut schema = self.inference.analyze_table(&table);

        // Enhance schema with LLM if available
        if let Some(ref llm) = self.llm_provider {
            self.enhance_schema(&mut schema, &table, llm.as_ref());
        }

        // Run validation to get observations
        let mut observations = self.validation.validate(&table, &schema);

        // Enhance observations with LLM explanations
        if let Some(ref llm) = self.llm_provider {
            self.enhance_observations(&mut observations, &schema, llm.as_ref());
        }

        // Generate suggestions
        // First, generate rule-based suggestions from observations
        let mut suggestions = SuggestionEngine::generate(&observations);

        // If LLM is available, enhance or add LLM-generated suggestions
        if let Some(ref llm) = self.llm_provider {
            let llm_suggestions = self.generate_llm_suggestions(&observations, &schema, llm.as_ref());
            // Merge LLM suggestions with rule-based ones
            // LLM suggestions can provide better rationale for existing suggestions
            // or add new suggestions that rules didn't catch
            for llm_sug in llm_suggestions {
                // Check if we already have a suggestion for this observation
                if let Some(existing) = suggestions.iter_mut().find(|s| s.observation_id == llm_sug.observation_id) {
                    // Use LLM rationale if it's more detailed
                    if llm_sug.rationale.len() > existing.rationale.len() {
                        existing.rationale = llm_sug.rationale;
                    }
                } else {
                    suggestions.push(llm_sug);
                }
            }
        }

        // Compute summary
        let summary = self.compute_summary(&schema, &observations);

        Ok(AnalysisResult {
            source,
            schema,
            observations,
            suggestions,
            summary,
        })
    }

    /// Enhance column schemas with LLM-generated insights.
    fn enhance_schema(
        &self,
        schema: &mut TableSchema,
        table: &crate::input::DataTable,
        llm: &dyn LlmProvider,
    ) {
        if !llm.config().enhance_schema {
            return;
        }

        for column in &mut schema.columns {
            // Get sample values for this column
            let samples: Vec<String> = table
                .column_values(column.position)
                .take(10)
                .map(|s| s.to_string())
                .collect();

            // Get LLM enhancement
            if let Ok(enhancement) = llm.enhance_schema(column, &samples, &self.config.context) {
                if !enhancement.insight.is_empty() {
                    column.llm_insight = Some(enhancement.insight);
                }
            }
        }
    }

    /// Enhance observations with LLM-generated explanations.
    fn enhance_observations(
        &self,
        observations: &mut [Observation],
        schema: &TableSchema,
        llm: &dyn LlmProvider,
    ) {
        if !llm.config().explain_observations {
            return;
        }

        for obs in observations {
            let column = schema.columns.iter().find(|c| c.name == obs.column);
            if let Ok(explanation) = llm.explain_observation(obs, column, &self.config.context) {
                if !explanation.is_empty() {
                    obs.llm_explanation = Some(explanation);
                }
            }
        }
    }

    /// Generate LLM-enhanced suggestions for observations.
    fn generate_llm_suggestions(
        &self,
        observations: &[Observation],
        schema: &TableSchema,
        llm: &dyn LlmProvider,
    ) -> Vec<Suggestion> {
        if !llm.config().generate_suggestions {
            return Vec::new();
        }

        let mut suggestions = Vec::new();
        for obs in observations {
            let column = schema.columns.iter().find(|c| c.name == obs.column);
            if let Ok(Some(suggestion)) =
                llm.generate_suggestion(obs, column, &self.config.context)
            {
                suggestions.push(suggestion);
            }
        }
        suggestions
    }

    /// Compute summary statistics from analysis results.
    fn compute_summary(
        &self,
        schema: &TableSchema,
        observations: &[Observation],
    ) -> AnalysisSummary {
        let total_columns = schema.columns.len();

        // Count columns with issues
        let columns_with_issues = {
            let mut affected: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for obs in observations {
                affected.insert(&obs.column);
            }
            affected.len()
        };

        // Count by severity
        let mut observations_by_severity = ObservationCounts::default();
        for obs in observations {
            match obs.severity {
                crate::validation::Severity::Error => observations_by_severity.error += 1,
                crate::validation::Severity::Warning => observations_by_severity.warning += 1,
                crate::validation::Severity::Info => observations_by_severity.info += 1,
            }
        }

        // Count by type
        let mut observations_by_type: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for obs in observations {
            *observations_by_type
                .entry(format!("{:?}", obs.observation_type).to_lowercase())
                .or_insert(0) += 1;
        }

        // Calculate quality score
        let data_quality_score = self.calculate_quality_score(
            total_columns,
            columns_with_issues,
            &observations_by_severity,
        );

        // Generate recommendation
        let recommendation = self.generate_recommendation(
            &observations_by_severity,
            data_quality_score,
        );

        AnalysisSummary {
            total_columns,
            columns_with_issues,
            total_observations: observations.len(),
            observations_by_severity,
            observations_by_type,
            data_quality_score,
            recommendation,
        }
    }

    /// Calculate a data quality score.
    fn calculate_quality_score(
        &self,
        total_columns: usize,
        columns_with_issues: usize,
        severity_counts: &ObservationCounts,
    ) -> f64 {
        if total_columns == 0 {
            return 1.0;
        }

        // Base score from percentage of clean columns
        let column_score = 1.0 - (columns_with_issues as f64 / total_columns as f64);

        // Penalty for severity
        let error_penalty = severity_counts.error as f64 * 0.1;
        let warning_penalty = severity_counts.warning as f64 * 0.02;
        let info_penalty = severity_counts.info as f64 * 0.005;

        let total_penalty = (error_penalty + warning_penalty + info_penalty).min(0.5);

        (column_score - total_penalty).max(0.0).min(1.0)
    }

    /// Generate a recommendation based on the analysis.
    fn generate_recommendation(
        &self,
        severity_counts: &ObservationCounts,
        quality_score: f64,
    ) -> String {
        if severity_counts.error > 0 {
            format!(
                "Address {} error-level issues before proceeding with analysis.",
                severity_counts.error
            )
        } else if severity_counts.warning > 5 {
            format!(
                "Review {} warning-level issues to improve data quality (score: {:.0}%).",
                severity_counts.warning,
                quality_score * 100.0
            )
        } else if quality_score >= 0.9 {
            "Data quality is good. Minor issues detected for review.".to_string()
        } else if quality_score >= 0.7 {
            "Data quality is acceptable. Consider addressing warnings.".to_string()
        } else {
            "Data quality needs attention. Review all observations.".to_string()
        }
    }
}

impl Default for Crucible {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_analyze_simple_csv() {
        let content = "sample_id,age,diagnosis\nS001,25,CD\nS002,30,UC\nS003,28,CD\n";
        let file = create_test_file(content);

        let crucible = Crucible::new();
        let result = crucible.analyze(file.path()).unwrap();

        assert_eq!(result.schema.columns.len(), 3);
        assert_eq!(result.source.row_count, 3);
        assert_eq!(result.source.column_count, 3);
    }

    #[test]
    fn test_analyze_with_issues() {
        let content = "id,status\n1,active\n2,missing\n3,active\n4,missing\n";
        let file = create_test_file(content);

        let crucible = Crucible::new();
        let result = crucible.analyze(file.path()).unwrap();

        // Should detect "missing" as a potential NA pattern
        let missing_obs = result
            .observations
            .iter()
            .any(|o| o.description.contains("missing"));
        assert!(missing_obs);
    }

    #[test]
    fn test_quality_score() {
        let crucible = Crucible::new();

        // Perfect data
        let score1 = crucible.calculate_quality_score(
            10,
            0,
            &ObservationCounts::default(),
        );
        assert_eq!(score1, 1.0);

        // Some issues
        let score2 = crucible.calculate_quality_score(
            10,
            2,
            &ObservationCounts {
                error: 0,
                warning: 3,
                info: 2,
            },
        );
        assert!(score2 > 0.7 && score2 < 0.9);
    }
}
