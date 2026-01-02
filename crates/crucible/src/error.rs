//! Error types for the Crucible library.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for Crucible operations.
#[derive(Debug, Error)]
pub enum CrucibleError {
    /// Error reading or accessing a file.
    #[error("IO error for '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Error parsing CSV/TSV data.
    #[error("Parse error at row {row}, column {column}: {message}")]
    Parse {
        row: usize,
        column: usize,
        message: String,
    },

    /// Error from the CSV library.
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    /// Invalid delimiter detected or specified.
    #[error("Invalid delimiter: {0}")]
    InvalidDelimiter(String),

    /// File format not supported.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Empty file or no data to analyze.
    #[error("Empty data: {0}")]
    EmptyData(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Regex compilation error.
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

/// Result type alias for Crucible operations.
pub type Result<T> = std::result::Result<T, CrucibleError>;
