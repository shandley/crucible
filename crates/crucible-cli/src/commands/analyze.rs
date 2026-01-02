//! Analyze command - analyze a data file and create curation layer.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{ContextHints, Crucible, CurationContext, CurationLayer, MockProvider, Severity};

pub fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    domain: Option<String>,
    mock_llm: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate input file exists
    if !file.exists() {
        return Err(format!("File not found: {}", file.display()).into());
    }

    println!(
        "{} {}",
        "Analyzing".cyan().bold(),
        file.display().to_string().white()
    );

    // Build Crucible instance
    let mut crucible = Crucible::new();

    if mock_llm {
        crucible = crucible.with_llm(MockProvider::new());
    }

    // Add domain context if provided
    if let Some(ref d) = domain {
        crucible = crucible.with_context(ContextHints::new().with_domain(d));
    }

    // Run analysis
    let result = crucible.analyze(&file)?;

    if verbose {
        println!();
        println!("{}", "Schema:".yellow().bold());
        for col in &result.schema.columns {
            println!(
                "  {:20} {:10} {:?}",
                col.name,
                format!("{:?}", col.inferred_type),
                col.semantic_role
            );
        }
        println!();
    }

    // Count by severity
    let error_count = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Error)
        .count();
    let warning_count = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Warning)
        .count();
    let info_count = result
        .observations
        .iter()
        .filter(|o| o.severity == Severity::Info)
        .count();

    println!(
        "Found {} observations ({} errors, {} warnings, {} info)",
        result.observations.len().to_string().white().bold(),
        error_count.to_string().red(),
        warning_count.to_string().yellow(),
        info_count.to_string().blue()
    );
    println!(
        "Generated {} suggestions",
        result.suggestions.len().to_string().white().bold()
    );

    // Create curation layer
    let mut context = CurationContext::new();
    if let Some(d) = domain {
        context = context.with_domain(d);
    }
    let curation = CurationLayer::from_analysis(result, context);

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut p = file.clone();
        let stem = p.file_stem().unwrap_or_default().to_string_lossy();
        p.set_file_name(format!("{}.curation.json", stem));
        p
    });

    // Save curation layer
    curation.save(&output_path)?;

    println!();
    println!(
        "{} {}",
        "Saved to".green().bold(),
        output_path.display().to_string().white()
    );

    // Print summary
    println!();
    println!(
        "Data quality score: {:.0}%",
        curation.summary.data_quality_score * 100.0
    );

    if curation.suggestions.is_empty() {
        println!("{}", "No issues found - data looks clean!".green());
    } else {
        println!(
            "Run {} to review suggestions",
            format!("crucible review {}", file.display())
                .cyan()
                .bold()
        );
    }

    Ok(())
}
