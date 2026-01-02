//! Apply command - apply accepted decisions and export curated data.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{CurationLayer, DecisionStatus};

use crate::cli::OutputFormat;

pub fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    format: OutputFormat,
    _with_audit: bool,
    _verbose: bool,
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

    // TODO: Implement actual transformation logic
    println!();
    println!(
        "{} Transformation logic not yet implemented.",
        "Note:".yellow()
    );
    println!(
        "Would save curated data to: {}",
        output_path.display().to_string().cyan()
    );

    Ok(())
}
