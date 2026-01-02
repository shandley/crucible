//! Example: Analyze a tabular data file with Crucible.
//!
//! Usage:
//!   cargo run --example analyze -- <file_path>
//!
//! Example:
//!   cargo run --example analyze -- test_data/ibd_cohort_metadata.tsv

use std::env;
use std::path::Path;

use crucible::{ContextHints, Crucible, MockProvider, Severity};

fn main() -> crucible::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example analyze -- <file_path>");
        eprintln!("\nExample:");
        eprintln!("  cargo run --example analyze -- test_data/ibd_cohort_metadata.tsv");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    if !path.exists() {
        eprintln!("Error: File not found: {}", file_path);
        std::process::exit(1);
    }

    let separator = "=".repeat(80);
    println!("{}", separator);
    println!("Crucible Analysis: {}", file_path);
    println!("{}", separator);
    println!();

    // Create Crucible with mock LLM (for testing without API key)
    let crucible = Crucible::new()
        .with_llm(MockProvider::new())
        .with_context(ContextHints::new().with_domain("biomedical"));

    let result = crucible.analyze(path)?;

    // Print source metadata
    println!("## Source Metadata");
    println!("  File: {}", result.source.file);
    println!("  Format: {}", result.source.format);
    println!("  Rows: {}", result.source.row_count);
    println!("  Columns: {}", result.source.column_count);
    println!();

    // Print schema summary
    println!("## Schema ({} columns)", result.schema.columns.len());
    println!();
    for col in &result.schema.columns {
        println!(
            "  {:20} {:10} {:15} unique={:<5} nullable={}",
            col.name,
            format!("{:?}", col.inferred_type),
            format!("{:?}", col.semantic_role),
            col.unique,
            col.nullable
        );
        if let Some(expected) = &col.expected_values {
            if expected.len() <= 10 {
                println!("                       expected: {:?}", expected);
            } else {
                println!(
                    "                       expected: [{} values]",
                    expected.len()
                );
            }
        }
    }
    println!();

    // Print observations
    println!(
        "## Observations ({} total)",
        result.observations.len()
    );
    println!();

    let mut errors: Vec<_> = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Error)
        .collect();
    let mut warnings: Vec<_> = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Warning)
        .collect();
    let mut infos: Vec<_> = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Info)
        .collect();

    errors.sort_by(|a, b| a.column.cmp(&b.column));
    warnings.sort_by(|a, b| a.column.cmp(&b.column));
    infos.sort_by(|a, b| a.column.cmp(&b.column));

    if !errors.is_empty() {
        println!("### Errors ({}):", errors.len());
        for obs in &errors {
            println!(
                "  [{}] {} - {:?}",
                obs.column, obs.description, obs.observation_type
            );
            if !obs.evidence.sample_rows.is_empty() {
                println!("       Sample rows: {:?}", obs.evidence.sample_rows);
            }
        }
        println!();
    }

    if !warnings.is_empty() {
        println!("### Warnings ({}):", warnings.len());
        for obs in &warnings {
            println!(
                "  [{}] {} - {:?}",
                obs.column, obs.description, obs.observation_type
            );
            if !obs.evidence.sample_rows.is_empty() {
                println!("       Sample rows: {:?}", obs.evidence.sample_rows);
            }
        }
        println!();
    }

    if !infos.is_empty() {
        println!("### Info ({}):", infos.len());
        for obs in &infos {
            println!(
                "  [{}] {} - {:?}",
                obs.column, obs.description, obs.observation_type
            );
        }
        println!();
    }

    // Print suggestions
    println!(
        "## Suggestions ({} total)",
        result.suggestions.len()
    );
    println!();

    for (i, sug) in result.suggestions.iter().enumerate() {
        println!(
            "  {}. [{}] {:?} (confidence: {:.0}%, priority: {})",
            i + 1,
            sug.id,
            sug.action,
            sug.confidence * 100.0,
            sug.priority
        );
        println!("     {}", sug.rationale);
        println!();
    }

    // Print summary
    println!("## Summary");
    println!(
        "  Data Quality Score: {:.1}%",
        result.summary.data_quality_score * 100.0
    );
    println!("  Columns with issues: {}", result.summary.columns_with_issues);
    println!("  Recommendation: {}", result.summary.recommendation);
    println!();

    println!("{}", separator);

    Ok(())
}
