//! Statistical analysis for column type and distribution inference.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::input::DataTable;
use crate::schema::{
    ColumnStatistics, ColumnType, Constraint, NumericStatistics,
    SemanticType, StringStatistics,
};

/// Results from statistical analysis of a column.
#[derive(Debug, Clone)]
pub struct StatisticalAnalysis {
    /// Inferred data type.
    pub inferred_type: ColumnType,
    /// Semantic type classification.
    pub semantic_type: SemanticType,
    /// Whether null values exist.
    pub nullable: bool,
    /// Whether all values are unique.
    pub unique: bool,
    /// Expected values for categorical columns.
    pub expected_values: Option<Vec<String>>,
    /// Expected range for numeric columns.
    pub expected_range: Option<(f64, f64)>,
    /// Inferred constraints.
    pub constraints: Vec<Constraint>,
    /// Computed statistics.
    pub statistics: ColumnStatistics,
    /// Confidence in the analysis.
    pub confidence: f64,
    /// Detected outliers (row indices).
    pub outliers: Vec<usize>,
    /// Detected missing value patterns.
    pub missing_patterns: Vec<String>,
}

/// Performs statistical analysis on data columns.
pub struct StatisticalAnalyzer {
    /// Maximum unique values to consider "categorical".
    categorical_threshold: usize,
    /// Outlier detection multiplier for IQR method.
    iqr_multiplier: f64,
    /// Z-score threshold for outlier detection.
    z_score_threshold: f64,
}

impl StatisticalAnalyzer {
    /// Create a new statistical analyzer with default settings.
    pub fn new() -> Self {
        Self {
            categorical_threshold: 20,
            iqr_multiplier: 1.5,
            z_score_threshold: 3.0,
        }
    }

    /// Analyze a column and return statistical analysis.
    pub fn analyze_column(&self, table: &DataTable, col_index: usize) -> StatisticalAnalysis {
        let values: Vec<&str> = table.column_values(col_index).collect();
        let total_count = values.len();

        // Separate null and non-null values
        let (null_values, non_null_values): (Vec<&&str>, Vec<&&str>) = values
            .iter()
            .partition(|v| DataTable::is_null_value(v));

        let null_count = null_values.len();
        let nullable = null_count > 0;

        // Detect missing value patterns (non-standard NA representations)
        let missing_patterns = self.detect_missing_patterns(&values);

        // Get unique non-null values
        let mut value_counts: IndexMap<String, usize> = IndexMap::new();
        for v in &non_null_values {
            *value_counts.entry(v.to_string()).or_insert(0) += 1;
        }
        let unique_count = value_counts.len();
        let unique = unique_count == non_null_values.len() && !non_null_values.is_empty();

        // Infer type
        let (inferred_type, type_confidence) = self.infer_type(&non_null_values);

        // Compute statistics based on type
        let (numeric_stats, string_stats) = self.compute_statistics(&non_null_values, inferred_type);

        // Detect outliers
        let outliers = self.detect_outliers(&values, &numeric_stats);

        // Build constraints
        let mut constraints = Vec::new();

        // Determine semantic type and constraints
        let semantic_type = self.infer_semantic_type(
            inferred_type,
            unique_count,
            total_count,
            &value_counts,
            &numeric_stats,
        );

        let expected_values = if matches!(semantic_type, SemanticType::Categorical | SemanticType::Binary)
            && unique_count <= self.categorical_threshold
        {
            let values: Vec<String> = value_counts.keys().cloned().collect();
            constraints.push(Constraint::SetMembership {
                values: values.clone(),
                confidence: 0.9,
            });
            Some(values)
        } else {
            None
        };

        let expected_range = if let Some(ref stats) = numeric_stats {
            constraints.push(Constraint::Range {
                min: Some(stats.min),
                max: Some(stats.max),
                confidence: 0.85,
            });
            Some((stats.min, stats.max))
        } else {
            None
        };

        if unique {
            constraints.push(Constraint::Unique { confidence: 0.95 });
        }

        if !nullable {
            constraints.push(Constraint::NotNull { confidence: 0.90 });
        }

        // Sample values
        let sample_values: Vec<String> = value_counts
            .keys()
            .take(5)
            .cloned()
            .collect();

        // Build statistics
        let statistics = ColumnStatistics {
            count: total_count,
            null_count,
            unique_count,
            sample_values,
            value_counts: if unique_count <= self.categorical_threshold * 2 {
                Some(value_counts)
            } else {
                None
            },
            numeric: numeric_stats,
            string: string_stats,
        };

        StatisticalAnalysis {
            inferred_type,
            semantic_type,
            nullable,
            unique,
            expected_values,
            expected_range,
            constraints,
            statistics,
            confidence: type_confidence,
            outliers,
            missing_patterns,
        }
    }

    /// Infer the data type from values.
    fn infer_type(&self, values: &[&&str]) -> (ColumnType, f64) {
        if values.is_empty() {
            return (ColumnType::Unknown, 0.0);
        }

        let mut type_counts = HashMap::new();

        for &value in values {
            let detected = self.detect_value_type(value);
            *type_counts.entry(detected).or_insert(0usize) += 1;
        }

        // Find the most common type
        let total = values.len() as f64;
        let (best_type, count) = type_counts
            .iter()
            .max_by_key(|&(_, count)| *count)
            .map(|(t, c)| (*t, *c))
            .unwrap_or((ColumnType::String, 0));

        let confidence = count as f64 / total;

        // Special handling: if all integers, but some could be floats, stay integer
        // If mostly integers with some floats, promote to float
        if best_type == ColumnType::Integer {
            let float_count = type_counts.get(&ColumnType::Float).unwrap_or(&0);
            if *float_count > 0 {
                return (ColumnType::Float, confidence * 0.95);
            }
        }

        (best_type, confidence)
    }

    /// Detect the type of a single value.
    fn detect_value_type(&self, value: &str) -> ColumnType {
        let trimmed = value.trim();

        // Boolean check
        if matches!(
            trimmed.to_lowercase().as_str(),
            "true" | "false" | "yes" | "no" | "t" | "f" | "y" | "n" | "1" | "0"
        ) {
            // Only consider it boolean if it's clearly a boolean word
            if matches!(
                trimmed.to_lowercase().as_str(),
                "true" | "false" | "yes" | "no"
            ) {
                return ColumnType::Boolean;
            }
        }

        // Integer check
        if trimmed.parse::<i64>().is_ok() {
            return ColumnType::Integer;
        }

        // Float check
        if trimmed.parse::<f64>().is_ok() {
            return ColumnType::Float;
        }

        // Date/DateTime check
        if self.looks_like_date(trimmed) {
            if trimmed.contains(':') || trimmed.contains('T') {
                return ColumnType::DateTime;
            }
            return ColumnType::Date;
        }

        ColumnType::String
    }

    /// Check if a value looks like a date.
    fn looks_like_date(&self, value: &str) -> bool {
        // Common date patterns
        let date_patterns = [
            r"^\d{4}-\d{2}-\d{2}",           // ISO date
            r"^\d{2}/\d{2}/\d{4}",           // US date
            r"^\d{2}-\d{2}-\d{4}",           // European date
            r"^\d{4}/\d{2}/\d{2}",           // Alt ISO
        ];

        for pattern in &date_patterns {
            if regex::Regex::new(pattern)
                .map(|r| r.is_match(value))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    /// Compute numeric and string statistics.
    fn compute_statistics(
        &self,
        values: &[&&str],
        column_type: ColumnType,
    ) -> (Option<NumericStatistics>, Option<StringStatistics>) {
        match column_type {
            ColumnType::Integer | ColumnType::Float => {
                let numeric_values: Vec<f64> = values
                    .iter()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect();

                if numeric_values.is_empty() {
                    return (None, None);
                }

                let stats = self.compute_numeric_stats(&numeric_values);
                (Some(stats), None)
            }
            ColumnType::String => {
                let lengths: Vec<usize> = values.iter().map(|v| v.len()).collect();
                if lengths.is_empty() {
                    return (None, None);
                }

                let min_length = *lengths.iter().min().unwrap_or(&0);
                let max_length = *lengths.iter().max().unwrap_or(&0);
                let avg_length = lengths.iter().sum::<usize>() as f64 / lengths.len() as f64;

                (None, Some(StringStatistics {
                    min_length,
                    max_length,
                    avg_length,
                }))
            }
            _ => (None, None),
        }
    }

    /// Compute numeric statistics.
    fn compute_numeric_stats(&self, values: &[f64]) -> NumericStatistics {
        let n = values.len();
        if n == 0 {
            return NumericStatistics {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                std: 0.0,
                median: 0.0,
                q1: 0.0,
                q3: 0.0,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = sorted[0];
        let max = sorted[n - 1];
        let mean = values.iter().sum::<f64>() / n as f64;

        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();

        let median = if n % 2 == 0 {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };

        let q1 = sorted[n / 4];
        let q3 = sorted[3 * n / 4];

        NumericStatistics {
            min,
            max,
            mean,
            std,
            median,
            q1,
            q3,
        }
    }

    /// Infer semantic type from statistics.
    fn infer_semantic_type(
        &self,
        column_type: ColumnType,
        unique_count: usize,
        total_count: usize,
        _value_counts: &IndexMap<String, usize>,
        numeric_stats: &Option<NumericStatistics>,
    ) -> SemanticType {
        match column_type {
            ColumnType::Boolean => SemanticType::Binary,
            ColumnType::Integer | ColumnType::Float => {
                // Check for count data (non-negative integers)
                if column_type == ColumnType::Integer {
                    if let Some(stats) = numeric_stats {
                        if stats.min >= 0.0 {
                            return SemanticType::Count;
                        }
                    }
                }

                // Check for proportion (0-1 or 0-100)
                if let Some(stats) = numeric_stats {
                    if stats.min >= 0.0 && stats.max <= 1.0 {
                        return SemanticType::Proportion;
                    }
                    if stats.min >= 0.0 && stats.max <= 100.0 && stats.mean < 100.0 {
                        // Could be percentage, but not certain
                    }
                }

                SemanticType::Continuous
            }
            ColumnType::String => {
                // Binary if exactly 2 unique values
                if unique_count == 2 {
                    return SemanticType::Binary;
                }

                // Categorical if low cardinality
                if unique_count <= self.categorical_threshold {
                    return SemanticType::Categorical;
                }

                // Check if unique (identifier-like)
                if unique_count == total_count {
                    return SemanticType::Identifier;
                }

                SemanticType::FreeText
            }
            ColumnType::DateTime | ColumnType::Date | ColumnType::Time => {
                SemanticType::Continuous
            }
            ColumnType::Unknown => SemanticType::Unknown,
        }
    }

    /// Detect outliers using IQR and z-score methods.
    fn detect_outliers(
        &self,
        values: &[&str],
        numeric_stats: &Option<NumericStatistics>,
    ) -> Vec<usize> {
        let Some(stats) = numeric_stats else {
            return Vec::new();
        };

        let mut outliers = Vec::new();

        for (idx, value) in values.iter().enumerate() {
            if DataTable::is_null_value(value) {
                continue;
            }

            if let Ok(num) = value.parse::<f64>() {
                // IQR method
                if stats.is_outlier_iqr(num, self.iqr_multiplier) {
                    outliers.push(idx);
                    continue;
                }

                // Z-score method
                if stats.z_score(num).abs() > self.z_score_threshold {
                    outliers.push(idx);
                }
            }
        }

        outliers
    }

    /// Detect non-standard missing value patterns.
    fn detect_missing_patterns(&self, values: &[&str]) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut pattern_counts: HashMap<String, usize> = HashMap::new();

        // Common missing value indicators that might not be caught by is_null_value
        let suspicious_patterns = [
            "missing", "unknown", "not available", "not recorded",
            "n.a.", "n.a", "na.", "#n/a", "#null", "undefined",
            "-999", "-9999", "999", "9999", "-1",
        ];

        for &value in values {
            let lower = value.trim().to_lowercase();
            for &pattern in &suspicious_patterns {
                if lower == pattern {
                    *pattern_counts.entry(pattern.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Only report patterns that appear multiple times
        for (pattern, count) in pattern_counts {
            if count >= 2 {
                patterns.push(pattern);
            }
        }

        patterns
    }
}

impl Default for StatisticalAnalyzer {
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
    fn test_infer_integer_type() {
        let table = make_table(
            vec!["count"],
            vec![vec!["1"], vec!["2"], vec!["3"], vec!["100"]],
        );
        let analyzer = StatisticalAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0);

        assert_eq!(result.inferred_type, ColumnType::Integer);
    }

    #[test]
    fn test_infer_float_type() {
        let table = make_table(
            vec!["value"],
            vec![vec!["1.5"], vec!["2.7"], vec!["3.14"], vec!["0.5"]],
        );
        let analyzer = StatisticalAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0);

        assert_eq!(result.inferred_type, ColumnType::Float);
    }

    #[test]
    fn test_detect_categorical() {
        let table = make_table(
            vec!["category"],
            vec![vec!["A"], vec!["B"], vec!["A"], vec!["C"], vec!["B"]],
        );
        let analyzer = StatisticalAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0);

        assert_eq!(result.semantic_type, SemanticType::Categorical);
        assert!(result.expected_values.is_some());
    }

    #[test]
    fn test_detect_nulls() {
        let table = make_table(
            vec!["value"],
            vec![vec!["1"], vec!["NA"], vec!["3"], vec![""], vec!["5"]],
        );
        let analyzer = StatisticalAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0);

        assert!(result.nullable);
        assert_eq!(result.statistics.null_count, 2);
    }
}
