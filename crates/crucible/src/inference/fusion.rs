//! Inference fusion - combining statistical and semantic analysis.

use crate::input::DataTable;
use crate::schema::{ColumnSchema, Constraint, SemanticRole, TableSchema};

use super::semantic::{SemanticAnalysis, SemanticAnalyzer};
use super::statistical::{StatisticalAnalysis, StatisticalAnalyzer};

/// Combined inference result for a column.
#[derive(Debug)]
pub struct FusedInference {
    /// Statistical analysis results.
    pub statistical: StatisticalAnalysis,
    /// Semantic analysis results.
    pub semantic: SemanticAnalysis,
    /// Final fused column schema.
    pub schema: ColumnSchema,
}

/// Configuration for inference fusion.
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// Weight for statistical analysis (0.0-1.0).
    pub statistical_weight: f64,
    /// Weight for semantic analysis (0.0-1.0).
    pub semantic_weight: f64,
    /// Minimum confidence threshold for constraints.
    pub constraint_threshold: f64,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            statistical_weight: 0.6,
            semantic_weight: 0.4,
            constraint_threshold: 0.7,
        }
    }
}

/// Combines statistical and semantic analysis to produce final schema.
pub struct InferenceFusion {
    statistical_analyzer: StatisticalAnalyzer,
    semantic_analyzer: SemanticAnalyzer,
    config: FusionConfig,
}

impl InferenceFusion {
    /// Create a new inference fusion engine.
    pub fn new() -> Self {
        Self {
            statistical_analyzer: StatisticalAnalyzer::new(),
            semantic_analyzer: SemanticAnalyzer::new(),
            config: FusionConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: FusionConfig) -> Self {
        Self {
            statistical_analyzer: StatisticalAnalyzer::new(),
            semantic_analyzer: SemanticAnalyzer::new(),
            config,
        }
    }

    /// Analyze a table and produce a fused schema.
    pub fn analyze_table(&self, table: &DataTable) -> TableSchema {
        let columns: Vec<ColumnSchema> = table
            .headers
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let fused = self.analyze_column(table, idx, name);
                fused.schema
            })
            .collect();

        TableSchema::with_columns(columns)
    }

    /// Analyze a single column with both analyzers and fuse results.
    pub fn analyze_column(
        &self,
        table: &DataTable,
        col_index: usize,
        col_name: &str,
    ) -> FusedInference {
        // Run both analyzers
        let statistical = self.statistical_analyzer.analyze_column(table, col_index);
        let semantic = self.semantic_analyzer.analyze_column(table, col_index, col_name);

        // Fuse results into final schema
        let schema = self.fuse_results(col_name, col_index, &statistical, &semantic);

        FusedInference {
            statistical,
            semantic,
            schema,
        }
    }

    /// Fuse statistical and semantic results into a column schema.
    fn fuse_results(
        &self,
        col_name: &str,
        col_index: usize,
        statistical: &StatisticalAnalysis,
        semantic: &SemanticAnalysis,
    ) -> ColumnSchema {
        // Type comes from statistical analysis (more reliable)
        let inferred_type = statistical.inferred_type;

        // Semantic type from statistical analysis
        let semantic_type = statistical.semantic_type;

        // Semantic role: prefer semantic analyzer if confident, else use statistical hints
        let semantic_role = if semantic.confidence > 0.6 {
            semantic.semantic_role
        } else {
            // Infer from statistical uniqueness patterns
            if statistical.unique && !statistical.nullable {
                SemanticRole::Identifier
            } else {
                semantic.semantic_role
            }
        };

        // Combine constraints, deduplicating and filtering by confidence
        let mut constraints = Vec::new();
        let mut seen_types = std::collections::HashSet::new();

        // Add statistical constraints first (higher priority)
        for constraint in &statistical.constraints {
            if constraint.confidence() >= self.config.constraint_threshold {
                let type_key = self.constraint_type_key(constraint);
                if seen_types.insert(type_key) {
                    constraints.push(constraint.clone());
                }
            }
        }

        // Add semantic constraints if not duplicates
        for constraint in &semantic.constraints {
            if constraint.confidence() >= self.config.constraint_threshold {
                let type_key = self.constraint_type_key(constraint);
                if seen_types.insert(type_key) {
                    constraints.push(constraint.clone());
                }
            }
        }

        // Calculate fused confidence
        let confidence = (statistical.confidence * self.config.statistical_weight
            + semantic.confidence * self.config.semantic_weight)
            / (self.config.statistical_weight + self.config.semantic_weight);

        // Build inference sources
        let mut inference_sources = vec!["statistical".to_string()];
        if semantic.confidence > 0.3 {
            inference_sources.push("semantic".to_string());
        }

        ColumnSchema {
            name: col_name.to_string(),
            position: col_index,
            inferred_type,
            semantic_type,
            semantic_role,
            nullable: statistical.nullable,
            unique: statistical.unique,
            expected_values: statistical.expected_values.clone(),
            expected_range: statistical.expected_range,
            constraints,
            statistics: statistical.statistics.clone(),
            confidence,
            inference_sources,
            llm_insight: None, // No LLM in Phase 1
        }
    }

    /// Get a key for constraint deduplication.
    fn constraint_type_key(&self, constraint: &Constraint) -> String {
        match constraint {
            Constraint::Pattern { .. } => "pattern".to_string(),
            Constraint::SetMembership { .. } => "set".to_string(),
            Constraint::Range { .. } => "range".to_string(),
            Constraint::Length { .. } => "length".to_string(),
            Constraint::Unique { .. } => "unique".to_string(),
            Constraint::NotNull { .. } => "not_null".to_string(),
        }
    }

    /// Get detected outliers from the analysis.
    pub fn get_outliers(
        &self,
        table: &DataTable,
        col_index: usize,
    ) -> Vec<usize> {
        let statistical = self.statistical_analyzer.analyze_column(table, col_index);
        statistical.outliers
    }

    /// Get detected missing value patterns.
    pub fn get_missing_patterns(
        &self,
        table: &DataTable,
        col_index: usize,
    ) -> Vec<String> {
        let statistical = self.statistical_analyzer.analyze_column(table, col_index);
        statistical.missing_patterns
    }
}

impl Default for InferenceFusion {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> DataTable {
        DataTable::new(
            headers.into_iter().map(String::from).collect(),
            rows.into_iter()
                .map(|r| r.into_iter().map(String::from).collect())
                .collect(),
            b',',
        )
    }

    #[test]
    fn test_fuse_identifier_column() {
        let table = make_table(
            vec!["sample_id"],
            vec![
                vec!["S001"],
                vec!["S002"],
                vec!["S003"],
                vec!["S004"],
            ],
        );

        let fusion = InferenceFusion::new();
        let result = fusion.analyze_column(&table, 0, "sample_id");

        assert_eq!(result.schema.semantic_role, SemanticRole::Identifier);
        assert!(result.schema.unique);
        assert!(!result.schema.nullable);
    }

    #[test]
    fn test_fuse_categorical_column() {
        let table = make_table(
            vec!["diagnosis"],
            vec![
                vec!["CD"],
                vec!["UC"],
                vec!["CD"],
                vec!["Control"],
            ],
        );

        let fusion = InferenceFusion::new();
        let result = fusion.analyze_column(&table, 0, "diagnosis");

        assert_eq!(result.schema.semantic_role, SemanticRole::Grouping);
        assert!(result.schema.expected_values.is_some());
    }

    #[test]
    fn test_analyze_full_table() {
        let table = make_table(
            vec!["sample_id", "age", "diagnosis"],
            vec![
                vec!["S001", "25", "CD"],
                vec!["S002", "30", "UC"],
                vec!["S003", "28", "CD"],
            ],
        );

        let fusion = InferenceFusion::new();
        let schema = fusion.analyze_table(&table);

        assert_eq!(schema.columns.len(), 3);
        assert_eq!(schema.columns[0].name, "sample_id");
        assert_eq!(schema.columns[1].name, "age");
        assert_eq!(schema.columns[2].name, "diagnosis");
    }
}
