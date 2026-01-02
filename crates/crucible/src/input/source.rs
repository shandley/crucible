//! Data source abstraction and metadata.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata about the source data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata {
    /// File name without path.
    pub file: String,
    /// Full path to the file.
    pub path: PathBuf,
    /// SHA-256 hash of the file contents.
    pub hash: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Detected format (csv, tsv, etc.).
    pub format: String,
    /// Detected encoding.
    pub encoding: String,
    /// Number of data rows (excluding header).
    pub row_count: usize,
    /// Number of columns.
    pub column_count: usize,
    /// When the analysis was performed.
    pub analyzed_at: DateTime<Utc>,
}

impl SourceMetadata {
    /// Create metadata for a file that has been analyzed.
    pub fn new(
        path: PathBuf,
        hash: String,
        size_bytes: u64,
        format: String,
        row_count: usize,
        column_count: usize,
    ) -> Self {
        let file = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        Self {
            file,
            path,
            hash,
            size_bytes,
            format,
            encoding: "utf-8".to_string(),
            row_count,
            column_count,
            analyzed_at: Utc::now(),
        }
    }
}

/// Represents parsed tabular data.
#[derive(Debug, Clone)]
pub struct DataTable {
    /// Column headers.
    pub headers: Vec<String>,
    /// Row data as strings (row-major order).
    pub rows: Vec<Vec<String>>,
    /// The delimiter used.
    pub delimiter: u8,
}

impl DataTable {
    /// Create a new data table.
    pub fn new(headers: Vec<String>, rows: Vec<Vec<String>>, delimiter: u8) -> Self {
        Self {
            headers,
            rows,
            delimiter,
        }
    }

    /// Get the number of columns.
    pub fn column_count(&self) -> usize {
        self.headers.len()
    }

    /// Get the number of rows (excluding header).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get all values for a column by index.
    pub fn column_values(&self, index: usize) -> impl Iterator<Item = &str> {
        self.rows.iter().map(move |row| {
            row.get(index)
                .map(|s| s.as_str())
                .unwrap_or("")
        })
    }

    /// Get a column by name.
    pub fn column_by_name(&self, name: &str) -> Option<Vec<&str>> {
        let index = self.headers.iter().position(|h| h == name)?;
        Some(self.column_values(index).collect())
    }

    /// Get a specific cell value.
    pub fn get(&self, row: usize, col: usize) -> Option<&str> {
        self.rows.get(row).and_then(|r| r.get(col).map(|s| s.as_str()))
    }

    /// Check if a value represents a missing/null value.
    pub fn is_null_value(value: &str) -> bool {
        let trimmed = value.trim();
        trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("na")
            || trimmed.eq_ignore_ascii_case("n/a")
            || trimmed.eq_ignore_ascii_case("null")
            || trimmed.eq_ignore_ascii_case("none")
            || trimmed.eq_ignore_ascii_case("nil")
            || trimmed == "."
            || trimmed == "-"
    }
}
