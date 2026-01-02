//! Semantic analysis for column name and value pattern inference.

use std::collections::HashMap;

use regex::Regex;

use crate::input::DataTable;
use crate::schema::{Constraint, SemanticRole};

/// Results from semantic analysis of a column.
#[derive(Debug, Clone)]
pub struct SemanticAnalysis {
    /// Inferred semantic role.
    pub semantic_role: SemanticRole,
    /// Detected value pattern (regex).
    pub value_pattern: Option<String>,
    /// Detected format (date, identifier, etc.).
    pub detected_format: Option<String>,
    /// Constraints inferred from semantics.
    pub constraints: Vec<Constraint>,
    /// Confidence in the analysis.
    pub confidence: f64,
    /// Hints extracted from column name.
    pub name_hints: Vec<String>,
}

/// Performs semantic analysis on column names and values.
pub struct SemanticAnalyzer {
    /// Patterns for identifying column roles by name.
    role_patterns: Vec<(Regex, SemanticRole)>,
    /// Patterns for identifying value formats.
    format_patterns: Vec<(Regex, &'static str)>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer.
    pub fn new() -> Self {
        Self {
            role_patterns: Self::build_role_patterns(),
            format_patterns: Self::build_format_patterns(),
        }
    }

    /// Build patterns for inferring semantic role from column name.
    fn build_role_patterns() -> Vec<(Regex, SemanticRole)> {
        vec![
            // Identifier patterns
            (Regex::new(r"(?i)^(id|_id$|identifier|key|uuid|guid)").unwrap(), SemanticRole::Identifier),
            (Regex::new(r"(?i)(sample[_\s]?id|patient[_\s]?id|subject[_\s]?id|record[_\s]?id)").unwrap(), SemanticRole::Identifier),
            (Regex::new(r"(?i)^(name|title|label)$").unwrap(), SemanticRole::Identifier),

            // Grouping patterns
            (Regex::new(r"(?i)(group|category|class|type|status|state|phase)").unwrap(), SemanticRole::Grouping),
            (Regex::new(r"(?i)(diagnosis|treatment|condition|cohort|arm)").unwrap(), SemanticRole::Grouping),
            (Regex::new(r"(?i)(sex|gender|race|ethnicity)").unwrap(), SemanticRole::Grouping),

            // Covariate patterns
            (Regex::new(r"(?i)(age|weight|height|bmi|score)").unwrap(), SemanticRole::Covariate),
            (Regex::new(r"(?i)(count|number|amount|quantity|level|value)").unwrap(), SemanticRole::Covariate),
            (Regex::new(r"(?i)(percent|percentage|ratio|rate|proportion)").unwrap(), SemanticRole::Covariate),

            // Outcome patterns
            (Regex::new(r"(?i)(outcome|result|response|endpoint|survival)").unwrap(), SemanticRole::Outcome),
            (Regex::new(r"(?i)(death|event|relapse|recurrence)").unwrap(), SemanticRole::Outcome),

            // Metadata patterns
            (Regex::new(r"(?i)(date|time|timestamp|created|updated|modified)").unwrap(), SemanticRole::Metadata),
            (Regex::new(r"(?i)(version|batch|run|file|source|origin)").unwrap(), SemanticRole::Metadata),
            (Regex::new(r"(?i)(note|comment|description|remark)").unwrap(), SemanticRole::Metadata),
        ]
    }

    /// Build patterns for identifying value formats.
    fn build_format_patterns() -> Vec<(Regex, &'static str)> {
        vec![
            // Email
            (Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(), "email"),

            // URL
            (Regex::new(r"^https?://").unwrap(), "url"),

            // UUID
            (Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap(), "uuid"),

            // ISO date
            (Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap(), "iso_date"),

            // ISO datetime
            (Regex::new(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}").unwrap(), "iso_datetime"),

            // US date
            (Regex::new(r"^\d{1,2}/\d{1,2}/\d{4}$").unwrap(), "us_date"),

            // Phone (US)
            (Regex::new(r"^\+?1?[-.\s]?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$").unwrap(), "phone_us"),

            // ZIP code
            (Regex::new(r"^\d{5}(-\d{4})?$").unwrap(), "zip_code"),

            // Scientific sample ID patterns (common in bioinformatics)
            (Regex::new(r"^[A-Z]{2,4}[_-]?\d{3,}").unwrap(), "sample_id"),
            (Regex::new(r"^\d{4}\.[A-Z]+\.\d+").unwrap(), "sample_id_dotted"),

            // Gene symbols
            (Regex::new(r"^[A-Z][A-Z0-9]{1,5}$").unwrap(), "gene_symbol_candidate"),

            // Boolean-like
            (Regex::new(r"(?i)^(true|false|yes|no|y|n|t|f)$").unwrap(), "boolean"),
        ]
    }

    /// Analyze a column's semantics.
    pub fn analyze_column(
        &self,
        table: &DataTable,
        col_index: usize,
        col_name: &str,
    ) -> SemanticAnalysis {
        let values: Vec<&str> = table.column_values(col_index).collect();

        // Analyze column name
        let (role_from_name, name_confidence) = self.infer_role_from_name(col_name);
        let name_hints = self.extract_name_hints(col_name);

        // Analyze value patterns
        let (value_pattern, pattern_confidence) = self.infer_value_pattern(&values);
        let detected_format = self.detect_value_format(&values);

        // Combine role inference
        let (semantic_role, role_confidence) = if name_confidence > 0.7 {
            (role_from_name, name_confidence)
        } else {
            // Try to infer from values
            let role_from_values = self.infer_role_from_values(&values, &detected_format);
            if role_from_values != SemanticRole::Unknown {
                (role_from_values, 0.6)
            } else {
                (role_from_name, name_confidence)
            }
        };

        // Build constraints from patterns
        let mut constraints = Vec::new();
        if let Some(ref pattern) = value_pattern {
            constraints.push(Constraint::Pattern {
                value: pattern.clone(),
                confidence: pattern_confidence,
            });
        }

        // Overall confidence
        let confidence = (role_confidence + pattern_confidence) / 2.0;

        SemanticAnalysis {
            semantic_role,
            value_pattern,
            detected_format,
            constraints,
            confidence,
            name_hints,
        }
    }

    /// Infer semantic role from column name.
    fn infer_role_from_name(&self, name: &str) -> (SemanticRole, f64) {
        for (pattern, role) in &self.role_patterns {
            if pattern.is_match(name) {
                return (*role, 0.85);
            }
        }
        (SemanticRole::Unknown, 0.0)
    }

    /// Extract semantic hints from column name.
    fn extract_name_hints(&self, name: &str) -> Vec<String> {
        let mut hints = Vec::new();

        // Split on common separators
        let parts: Vec<&str> = name
            .split(|c: char| c == '_' || c == '-' || c == '.' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        for part in parts {
            let lower = part.to_lowercase();

            // Check for common meaningful terms
            if matches!(
                lower.as_str(),
                "id" | "date" | "time" | "count" | "total" | "mean" | "avg" |
                "sum" | "min" | "max" | "std" | "var" | "pct" | "percent" |
                "num" | "name" | "type" | "status" | "group" | "category"
            ) {
                hints.push(lower);
            }
        }

        hints
    }

    /// Infer a common value pattern.
    fn infer_value_pattern(&self, values: &[&str]) -> (Option<String>, f64) {
        let non_empty: Vec<&str> = values
            .iter()
            .filter(|v| !DataTable::is_null_value(v))
            .copied()
            .collect();

        if non_empty.is_empty() {
            return (None, 0.0);
        }

        // Try to find a common pattern
        let patterns = self.generate_candidate_patterns(&non_empty);

        // Score patterns by how many values they match
        let mut best_pattern: Option<String> = None;
        let mut best_score = 0.0;

        for pattern_str in patterns {
            if let Ok(pattern) = Regex::new(&pattern_str) {
                let matches = non_empty.iter().filter(|v| pattern.is_match(v)).count();
                let score = matches as f64 / non_empty.len() as f64;

                if score > best_score && score >= 0.9 {
                    best_score = score;
                    best_pattern = Some(pattern_str);
                }
            }
        }

        (best_pattern, best_score)
    }

    /// Generate candidate regex patterns from sample values.
    fn generate_candidate_patterns(&self, values: &[&str]) -> Vec<String> {
        let mut patterns = Vec::new();

        if values.is_empty() {
            return patterns;
        }

        // Check for consistent length
        let lengths: Vec<usize> = values.iter().map(|v| v.len()).collect();
        let all_same_length = lengths.iter().all(|&l| l == lengths[0]);

        // Check character classes
        let all_alphanumeric = values.iter().all(|v| v.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
        let all_numeric = values.iter().all(|v| v.chars().all(|c| c.is_ascii_digit()));
        let all_alpha = values.iter().all(|v| v.chars().all(|c| c.is_alphabetic()));
        let all_uppercase = values.iter().all(|v| v.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()));

        // Generate patterns based on observations
        if all_numeric {
            if all_same_length {
                patterns.push(format!(r"^\d{{{}}}$", lengths[0]));
            } else {
                let min_len = lengths.iter().min().unwrap_or(&1);
                let max_len = lengths.iter().max().unwrap_or(&20);
                patterns.push(format!(r"^\d{{{},{}}}$", min_len, max_len));
            }
        } else if all_alpha && all_uppercase && all_same_length {
            patterns.push(format!(r"^[A-Z]{{{}}}$", lengths[0]));
        } else if all_alphanumeric {
            // Try to build a more specific pattern from first value
            if let Some(first) = values.first() {
                let pattern = self.build_pattern_from_sample(first);
                patterns.push(pattern);
            }
        }

        patterns
    }

    /// Build a regex pattern from a sample value.
    fn build_pattern_from_sample(&self, sample: &str) -> String {
        let mut pattern = String::from("^");
        let mut chars = sample.chars().peekable();

        while let Some(c) = chars.next() {
            if c.is_ascii_digit() {
                // Count consecutive digits
                let mut count = 1;
                while chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    chars.next();
                    count += 1;
                }
                if count > 1 {
                    pattern.push_str(&format!(r"\d{{{}}}", count));
                } else {
                    pattern.push_str(r"\d");
                }
            } else if c.is_alphabetic() {
                // Count consecutive letters
                let mut count = 1;
                let is_upper = c.is_uppercase();
                while chars
                    .peek()
                    .map(|c| c.is_alphabetic() && c.is_uppercase() == is_upper)
                    .unwrap_or(false)
                {
                    chars.next();
                    count += 1;
                }
                let char_class = if is_upper { "[A-Z]" } else { "[a-z]" };
                if count > 1 {
                    pattern.push_str(&format!("{}{{{}}}", char_class, count));
                } else {
                    pattern.push_str(char_class);
                }
            } else {
                // Escape special regex characters
                if "[](){}.*+?^$\\|".contains(c) {
                    pattern.push('\\');
                }
                pattern.push(c);
            }
        }

        pattern.push('$');
        pattern
    }

    /// Detect a known format from values.
    fn detect_value_format(&self, values: &[&str]) -> Option<String> {
        let non_empty: Vec<&str> = values
            .iter()
            .filter(|v| !DataTable::is_null_value(v))
            .copied()
            .collect();

        if non_empty.is_empty() {
            return None;
        }

        // Count format matches
        let mut format_counts: HashMap<&str, usize> = HashMap::new();

        for value in &non_empty {
            for (pattern, format) in &self.format_patterns {
                if pattern.is_match(value) {
                    *format_counts.entry(format).or_insert(0) += 1;
                    break; // Only count first match per value
                }
            }
        }

        // Find format that matches most values (with threshold)
        let threshold = (non_empty.len() as f64 * 0.8) as usize;

        format_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .max_by_key(|(_, count)| *count)
            .map(|(format, _)| format.to_string())
    }

    /// Infer role from value patterns.
    fn infer_role_from_values(
        &self,
        _values: &[&str],
        detected_format: &Option<String>,
    ) -> SemanticRole {
        if let Some(format) = detected_format {
            match format.as_str() {
                "email" | "uuid" | "sample_id" | "sample_id_dotted" => {
                    return SemanticRole::Identifier;
                }
                "iso_date" | "iso_datetime" | "us_date" => {
                    return SemanticRole::Metadata;
                }
                "boolean" => {
                    return SemanticRole::Grouping;
                }
                _ => {}
            }
        }

        SemanticRole::Unknown
    }
}

impl Default for SemanticAnalyzer {
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
    fn test_identifier_role_from_name() {
        let analyzer = SemanticAnalyzer::new();
        let (role, conf) = analyzer.infer_role_from_name("sample_id");
        assert_eq!(role, SemanticRole::Identifier);
        assert!(conf > 0.8);
    }

    #[test]
    fn test_grouping_role_from_name() {
        let analyzer = SemanticAnalyzer::new();
        let (role, _) = analyzer.infer_role_from_name("diagnosis");
        assert_eq!(role, SemanticRole::Grouping);
    }

    #[test]
    fn test_covariate_role_from_name() {
        let analyzer = SemanticAnalyzer::new();
        let (role, _) = analyzer.infer_role_from_name("age");
        assert_eq!(role, SemanticRole::Covariate);
    }

    #[test]
    fn test_detect_email_format() {
        let table = make_table(
            vec!["email"],
            vec![
                vec!["user@example.com"],
                vec!["another@test.org"],
                vec!["third@domain.net"],
            ],
        );
        let analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0, "email");

        assert_eq!(result.detected_format, Some("email".to_string()));
    }

    #[test]
    fn test_detect_date_format() {
        let table = make_table(
            vec!["date"],
            vec![
                vec!["2024-01-15"],
                vec!["2024-02-20"],
                vec!["2024-03-25"],
            ],
        );
        let analyzer = SemanticAnalyzer::new();
        let result = analyzer.analyze_column(&table, 0, "collection_date");

        assert_eq!(result.detected_format, Some("iso_date".to_string()));
        assert_eq!(result.semantic_role, SemanticRole::Metadata);
    }
}
