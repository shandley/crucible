//! CSV/TSV parser with delimiter detection.

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{CrucibleError, Result};
use super::source::{DataTable, SourceMetadata};

/// Delimiters to try when auto-detecting.
const DELIMITERS: &[u8] = &[b'\t', b',', b';', b'|'];

/// Parser configuration.
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Delimiter to use (None = auto-detect).
    pub delimiter: Option<u8>,
    /// Whether the file has a header row.
    pub has_header: bool,
    /// Maximum rows to read (None = all).
    pub max_rows: Option<usize>,
    /// Quote character.
    pub quote: u8,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            delimiter: None,
            has_header: true,
            max_rows: None,
            quote: b'"',
        }
    }
}

/// Parses tabular data files.
pub struct Parser {
    config: ParserConfig,
}

impl Parser {
    /// Create a new parser with default configuration.
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a parser with custom configuration.
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse a file and return the data table and metadata.
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<(DataTable, SourceMetadata)> {
        let path = path.as_ref();

        // Read file contents for hashing and parsing
        let mut file = File::open(path).map_err(|e| CrucibleError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        let metadata = file.metadata().map_err(|e| CrucibleError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let size_bytes = metadata.len();

        // Read entire file for hashing
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).map_err(|e| CrucibleError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Compute hash
        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let hash = format!("sha256:{:x}", hasher.finalize());

        // Detect delimiter if not specified
        let delimiter = match self.config.delimiter {
            Some(d) => d,
            None => detect_delimiter(&contents)?,
        };

        // Parse the CSV/TSV
        let data_table = self.parse_bytes(&contents, delimiter)?;

        // Determine format from delimiter
        let format = match delimiter {
            b'\t' => "tsv",
            b',' => "csv",
            b';' => "csv-semicolon",
            b'|' => "psv",
            _ => "delimited",
        }.to_string();

        let source_metadata = SourceMetadata::new(
            path.to_path_buf(),
            hash,
            size_bytes,
            format,
            data_table.row_count(),
            data_table.column_count(),
        );

        Ok((data_table, source_metadata))
    }

    /// Parse bytes directly.
    fn parse_bytes(&self, bytes: &[u8], delimiter: u8) -> Result<DataTable> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(self.config.has_header)
            .quote(self.config.quote)
            .flexible(true)
            .from_reader(bytes);

        // Get headers
        let headers: Vec<String> = if self.config.has_header {
            reader
                .headers()?
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            // Generate column names
            let first_record = reader.records().next();
            match first_record {
                Some(Ok(record)) => (0..record.len())
                    .map(|i| format!("column_{}", i + 1))
                    .collect(),
                Some(Err(e)) => return Err(e.into()),
                None => return Err(CrucibleError::EmptyData("No data rows found".to_string())),
            }
        };

        if headers.is_empty() {
            return Err(CrucibleError::EmptyData("No columns found".to_string()));
        }

        // Read rows
        let mut rows = Vec::new();
        let expected_cols = headers.len();

        // Need to re-create reader if we consumed it getting headers
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(self.config.has_header)
            .quote(self.config.quote)
            .flexible(true)
            .from_reader(bytes);

        for (row_idx, result) in reader.records().enumerate() {
            if let Some(max) = self.config.max_rows {
                if row_idx >= max {
                    break;
                }
            }

            let record = result?;
            let mut row: Vec<String> = record.iter().map(|s| s.to_string()).collect();

            // Pad row if needed
            while row.len() < expected_cols {
                row.push(String::new());
            }
            // Truncate if too many columns
            row.truncate(expected_cols);

            rows.push(row);
        }

        if rows.is_empty() {
            return Err(CrucibleError::EmptyData("No data rows found".to_string()));
        }

        Ok(DataTable::new(headers, rows, delimiter))
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect the delimiter by analyzing the first few lines.
fn detect_delimiter(bytes: &[u8]) -> Result<u8> {
    let reader = BufReader::new(bytes);
    let lines: Vec<String> = reader
        .lines()
        .take(10)
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .collect();

    if lines.is_empty() {
        return Err(CrucibleError::EmptyData("No lines to analyze".to_string()));
    }

    // Count occurrences of each delimiter in each line
    let mut best_delimiter = b',';
    let mut best_score = 0;

    for &delim in DELIMITERS {
        let counts: Vec<usize> = lines
            .iter()
            .map(|line| count_delimiter_in_line(line, delim))
            .collect();

        // Check if counts are consistent (same count in each line)
        if counts.is_empty() {
            continue;
        }

        let first_count = counts[0];
        if first_count == 0 {
            continue;
        }

        // Calculate consistency score
        let consistent = counts.iter().all(|&c| c == first_count);
        let variance: f64 = if counts.len() > 1 {
            let mean = counts.iter().sum::<usize>() as f64 / counts.len() as f64;
            counts.iter().map(|&c| (c as f64 - mean).powi(2)).sum::<f64>() / counts.len() as f64
        } else {
            0.0
        };

        // Score: higher count with lower variance is better
        // Tab delimiter gets a slight bonus as it's less common in actual data
        let score = if consistent {
            first_count * 1000 + (if delim == b'\t' { 100 } else { 0 })
        } else if variance < 1.0 {
            first_count * 100
        } else {
            first_count
        };

        if score > best_score {
            best_score = score;
            best_delimiter = delim;
        }
    }

    Ok(best_delimiter)
}

/// Count delimiter occurrences in a line, respecting quotes.
fn count_delimiter_in_line(line: &str, delimiter: u8) -> usize {
    let delim_char = delimiter as char;
    let mut count = 0;
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            c if c == delim_char && !in_quotes => count += 1,
            _ => {}
        }
    }

    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_delimiter_csv() {
        let data = b"a,b,c\n1,2,3\n4,5,6";
        assert_eq!(detect_delimiter(data).unwrap(), b',');
    }

    #[test]
    fn test_detect_delimiter_tsv() {
        let data = b"a\tb\tc\n1\t2\t3\n4\t5\t6";
        assert_eq!(detect_delimiter(data).unwrap(), b'\t');
    }

    #[test]
    fn test_parse_csv() {
        let parser = Parser::new();
        let data = b"name,age,city\nAlice,30,NYC\nBob,25,LA";
        let table = parser.parse_bytes(data, b',').unwrap();

        assert_eq!(table.headers, vec!["name", "age", "city"]);
        assert_eq!(table.row_count(), 2);
        assert_eq!(table.get(0, 0), Some("Alice"));
        assert_eq!(table.get(1, 1), Some("25"));
    }

    #[test]
    fn test_is_null_value() {
        assert!(DataTable::is_null_value(""));
        assert!(DataTable::is_null_value("NA"));
        assert!(DataTable::is_null_value("na"));
        assert!(DataTable::is_null_value("N/A"));
        assert!(DataTable::is_null_value("null"));
        assert!(DataTable::is_null_value("NULL"));
        assert!(DataTable::is_null_value("."));
        assert!(!DataTable::is_null_value("value"));
        assert!(!DataTable::is_null_value("0"));
    }
}
