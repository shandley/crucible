//! Diff command - preview changes that would be applied.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{CurationLayer, DecisionStatus};

pub fn run(
    file: PathBuf,
    _context: usize,
    _changed_only: bool,
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
        println!("{}", "No changes to preview.".yellow());
        println!("Accept some suggestions first using 'crucible review'.");
        return Ok(());
    }

    println!(
        "{} {} accepted changes",
        "Previewing".cyan().bold(),
        approved.len().to_string().white().bold()
    );
    println!();

    // Show summary of what would change
    for decision in &approved {
        if let Some(suggestion) = curation.suggestion(&decision.suggestion_id) {
            if let Some(observation) = curation.observation(&suggestion.observation_id) {
                let action = format!("{:?}", suggestion.action).to_uppercase();
                let status = match decision.status {
                    DecisionStatus::Accepted => "ACCEPT".green(),
                    DecisionStatus::Modified => "MODIFY".blue(),
                    _ => "".normal(),
                };

                println!(
                    "  {} [{}] {} - {}",
                    status,
                    action.cyan(),
                    observation.column.white().bold(),
                    observation.description
                );

                if let Some(notes) = &decision.notes {
                    println!("    Notes: {}", notes.dimmed());
                }
            }
        }
    }

    println!();
    println!(
        "Run {} to apply these changes.",
        format!("crucible apply {}", file.display()).cyan().bold()
    );

    // TODO: Implement actual diff display with row-by-row changes

    Ok(())
}
