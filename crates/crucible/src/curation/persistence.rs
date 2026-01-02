//! Persistence for curation layers - save/load JSON files.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use crate::error::{CrucibleError, Result};

use super::layer::CurationLayer;

impl CurationLayer {
    /// Save the curation layer to a JSON file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crucible::curation::CurationLayer;
    /// # fn example(curation: &CurationLayer) -> crucible::Result<()> {
    /// curation.save("metadata.curation.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    CrucibleError::Persistence(format!(
                        "Failed to create directory '{}': {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }

        let file = File::create(path).map_err(|e| {
            CrucibleError::Persistence(format!(
                "Failed to create file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).map_err(|e| {
            CrucibleError::Persistence(format!("Failed to serialize curation layer: {}", e))
        })?;

        Ok(())
    }

    /// Load a curation layer from a JSON file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crucible::curation::CurationLayer;
    /// let curation = CurationLayer::load("metadata.curation.json").unwrap();
    /// println!("Pending: {}", curation.pending_suggestions().len());
    /// ```
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let file = File::open(path).map_err(|e| {
            CrucibleError::Persistence(format!(
                "Failed to open file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let reader = BufReader::new(file);
        let layer: CurationLayer = serde_json::from_reader(reader).map_err(|e| {
            CrucibleError::Persistence(format!(
                "Failed to parse curation layer '{}': {}",
                path.display(),
                e
            ))
        })?;

        Ok(layer)
    }

    /// Save with version history.
    ///
    /// Creates a timestamped backup in a `.history` subdirectory before saving.
    ///
    /// # Example
    ///
    /// File structure after calling:
    /// ```text
    /// data/
    /// ├── metadata.curation.json           # Current version
    /// └── metadata.curation.history/
    ///     └── 2024-12-30T10-00-00.json      # Previous version
    /// ```
    pub fn save_with_history(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // If file exists, save to history first
        if path.exists() {
            self.save_to_history(path)?;
        }

        // Save current version
        self.save(path)
    }

    /// Save the current file to history directory.
    fn save_to_history(&self, path: &Path) -> Result<()> {
        let history_dir = history_directory(path);

        // Create history directory if needed
        if !history_dir.exists() {
            fs::create_dir_all(&history_dir).map_err(|e| {
                CrucibleError::Persistence(format!(
                    "Failed to create history directory '{}': {}",
                    history_dir.display(),
                    e
                ))
            })?;
        }

        // Load existing file
        let existing = Self::load(path)?;

        // Save with timestamp
        let timestamp = existing.updated_at.format("%Y-%m-%dT%H-%M-%S").to_string();
        let history_file = history_dir.join(format!("{}.json", timestamp));

        existing.save(&history_file)
    }

    /// List all historical versions of a curation layer.
    ///
    /// Returns paths sorted by timestamp (newest first).
    pub fn list_history(path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let path = path.as_ref();
        let history_dir = history_directory(path);

        if !history_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries: Vec<PathBuf> = fs::read_dir(&history_dir)
            .map_err(|e| {
                CrucibleError::Persistence(format!(
                    "Failed to read history directory '{}': {}",
                    history_dir.display(),
                    e
                ))
            })?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
            .collect();

        // Sort by filename (timestamp) descending
        entries.sort_by(|a, b| b.cmp(a));

        Ok(entries)
    }

    /// Load a specific historical version.
    pub fn load_history(path: impl AsRef<Path>, index: usize) -> Result<Self> {
        let history = Self::list_history(&path)?;

        let history_path = history.get(index).ok_or_else(|| {
            CrucibleError::Persistence(format!(
                "History version {} not found (only {} versions available)",
                index,
                history.len()
            ))
        })?;

        Self::load(history_path)
    }
}

/// Get the history directory for a curation file.
fn history_directory(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(Path::new("."));

    parent.join(format!("{}.history", stem))
}

/// Generate a curation file path for a data file.
///
/// # Example
///
/// ```
/// use crucible::curation::curation_path;
///
/// let path = curation_path("data/metadata.tsv");
/// assert_eq!(path.to_string_lossy(), "data/metadata.curation.json");
/// ```
pub fn curation_path(data_path: impl AsRef<Path>) -> PathBuf {
    let data_path = data_path.as_ref();
    let stem = data_path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = data_path.parent().unwrap_or(Path::new("."));

    parent.join(format!("{}.curation.json", stem))
}

/// Generate a curation file path in a .crucible subdirectory.
///
/// # Example
///
/// ```
/// use crucible::curation::crucible_curation_path;
///
/// let path = crucible_curation_path("data/metadata.tsv");
/// assert_eq!(path.to_string_lossy(), "data/.crucible/metadata.curation.json");
/// ```
pub fn crucible_curation_path(data_path: impl AsRef<Path>) -> PathBuf {
    let data_path = data_path.as_ref();
    let stem = data_path.file_stem().unwrap_or_default().to_string_lossy();
    let parent = data_path.parent().unwrap_or(Path::new("."));

    parent
        .join(".crucible")
        .join(format!("{}.curation.json", stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curation_path() {
        assert_eq!(
            curation_path("data/metadata.tsv").to_string_lossy(),
            "data/metadata.curation.json"
        );
        assert_eq!(
            curation_path("test.csv").to_string_lossy(),
            "test.curation.json"
        );
    }

    #[test]
    fn test_crucible_curation_path() {
        assert_eq!(
            crucible_curation_path("data/metadata.tsv").to_string_lossy(),
            "data/.crucible/metadata.curation.json"
        );
    }

    #[test]
    fn test_history_directory() {
        let path = Path::new("data/metadata.curation.json");
        assert_eq!(
            history_directory(path).to_string_lossy(),
            "data/metadata.curation.history"
        );
    }
}
