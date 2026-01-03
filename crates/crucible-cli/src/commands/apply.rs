//! Apply command - apply accepted decisions and export curated data.

use std::collections::HashMap;
use std::path::PathBuf;

use colored::Colorize;
use crucible::{CurationLayer, DecisionStatus, Parser, TransformEngine, TransformResult};

use crate::cli::OutputFormat;

pub fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    format: OutputFormat,
    with_audit: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("Curation file not found: {}", file.display()).into());
    }

    // Load curation layer
    let curation = CurationLayer::load(&file)?;

    // Count approved decisions
    let approved: Vec<_> = curation
        .decisions
        .iter()
        .filter(|d| d.status == DecisionStatus::Accepted || d.status == DecisionStatus::Modified)
        .collect();

    if approved.is_empty() {
        println!(
            "{} No accepted decisions to apply.",
            "Warning:".yellow().bold()
        );
        println!(
            "Run {} to review and accept suggestions first.",
            format!("crucible review {}", file.display()).cyan()
        );
        return Ok(());
    }

    println!(
        "{} {} decisions",
        "Applying".cyan().bold(),
        approved.len().to_string().white().bold()
    );

    // Find and load the source data file
    let source_path = resolve_source_path(&file, &curation)?;

    if verbose {
        println!(
            "  {} {}",
            "Source:".dimmed(),
            source_path.display().to_string().dimmed()
        );
    }

    // Parse the source data
    let parser = Parser::new();
    let (mut data, _source_metadata) = parser.parse_file(&source_path)?;

    if verbose {
        println!(
            "  {} {} rows × {} columns",
            "Loaded:".dimmed(),
            data.row_count().to_string().dimmed(),
            data.column_count().to_string().dimmed()
        );
    }

    // Apply transformations
    let engine = TransformEngine::new();
    let result = engine.apply(&curation, &mut data)?;

    // Report changes
    if result.operations_applied > 0 {
        println!();
        println!(
            "{} {} transformations",
            "Applied".green().bold(),
            result.operations_applied.to_string().white().bold()
        );

        for change in &result.changes {
            if change.values_changed > 0 || verbose {
                println!(
                    "  {} {} ({} values)",
                    "•".dimmed(),
                    change.description,
                    change.values_changed
                );
            }
        }
    } else {
        println!();
        println!(
            "{} No data changes were needed.",
            "Note:".yellow()
        );
    }

    // Add audit columns if requested
    if with_audit {
        add_audit_columns(&mut data, &result, verbose)?;
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let source_file = PathBuf::from(&curation.source.file);
        let stem = source_file.file_stem().unwrap_or_default().to_string_lossy();
        let ext = match format {
            OutputFormat::Tsv => "tsv",
            OutputFormat::Csv => "csv",
            OutputFormat::Json => "json",
        };
        file.with_file_name(format!("{}_curated.{}", stem, ext))
    });

    // Write the transformed data
    match format {
        OutputFormat::Json => {
            data.write_to_json(&output_path)?;
        }
        OutputFormat::Tsv => {
            data.write_to_file(&output_path, b'\t')?;
        }
        OutputFormat::Csv => {
            data.write_to_file(&output_path, b',')?;
        }
    }

    println!();
    println!(
        "{} {}",
        "Saved:".green().bold(),
        output_path.display().to_string().cyan()
    );

    // Show summary
    println!();
    println!("{}", "Summary:".white().bold());
    println!(
        "  {} rows processed",
        data.row_count().to_string().cyan()
    );
    println!(
        "  {} values modified",
        result.rows_modified.to_string().cyan()
    );
    if with_audit {
        println!(
            "  {} audit columns added",
            "3".to_string().cyan()
        );
    }

    Ok(())
}

/// Add audit columns to track what was changed and why.
fn add_audit_columns(
    data: &mut crucible::DataTable,
    result: &TransformResult,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build per-row audit information
    let mut row_changes: HashMap<usize, Vec<String>> = HashMap::new();
    let mut row_originals: HashMap<usize, Vec<String>> = HashMap::new();
    let mut row_reasons: HashMap<usize, Vec<String>> = HashMap::new();

    for change in &result.changes {
        for audit in &change.row_audits {
            row_changes
                .entry(audit.row)
                .or_default()
                .push(format!("{}={}", audit.column, audit.new_value));
            row_originals
                .entry(audit.row)
                .or_default()
                .push(format!("{}={}", audit.column, audit.original_value));
            row_reasons
                .entry(audit.row)
                .or_default()
                .push(audit.reason.clone());
        }
    }

    // Add audit columns
    data.add_column("_crucible_modified".to_string(), String::new());
    data.add_column("_crucible_original".to_string(), String::new());
    data.add_column("_crucible_reason".to_string(), String::new());

    let mod_col = data.column_index("_crucible_modified").unwrap();
    let orig_col = data.column_index("_crucible_original").unwrap();
    let reason_col = data.column_index("_crucible_reason").unwrap();

    // Populate audit columns
    for row_idx in 0..data.row_count() {
        if let Some(changes) = row_changes.get(&row_idx) {
            data.set(row_idx, mod_col, changes.join("; "));
        }
        if let Some(originals) = row_originals.get(&row_idx) {
            data.set(row_idx, orig_col, originals.join("; "));
        }
        if let Some(reasons) = row_reasons.get(&row_idx) {
            data.set(row_idx, reason_col, reasons.join("; "));
        }
    }

    if verbose {
        println!();
        println!(
            "  {} Added audit columns: _crucible_modified, _crucible_original, _crucible_reason",
            "Audit:".dimmed()
        );
    }

    Ok(())
}

/// Resolve the source data file path from the curation layer.
fn resolve_source_path(
    curation_file: &PathBuf,
    curation: &CurationLayer,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    // First, try the path stored in the curation layer
    let stored_path = &curation.source.path;
    if stored_path.exists() {
        return Ok(stored_path.clone());
    }

    // If the stored path doesn't exist, try relative to the curation file
    let curation_dir = curation_file.parent().unwrap_or(std::path::Path::new("."));

    // Try the filename from the curation layer
    let relative_path = curation_dir.join(&curation.source.file);
    if relative_path.exists() {
        return Ok(relative_path);
    }

    // Try common patterns
    let stem = curation_file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();

    // If the curation file is "foo.curation.json", try "foo.tsv", "foo.csv"
    let data_stem = stem.trim_end_matches(".curation");
    for ext in &["tsv", "csv", "txt"] {
        let data_file = curation_dir.join(format!("{}.{}", data_stem, ext));
        if data_file.exists() {
            return Ok(data_file);
        }
    }

    Err(format!(
        "Could not find source data file. Tried:\n  - {}\n  - {}\nPlease ensure the source file exists.",
        stored_path.display(),
        relative_path.display()
    )
    .into())
}
