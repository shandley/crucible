//! Status command - show curation progress and summary.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{CurationLayer, DecisionStatus};

pub fn run(
    file: PathBuf,
    json_output: bool,
    _verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine curation file path
    let curation_path = if file.extension().map(|e| e == "json").unwrap_or(false) {
        file.clone()
    } else {
        // Assume it's a data file, look for .curation.json
        let mut p = file.clone();
        let stem = p.file_stem().unwrap_or_default().to_string_lossy();
        p.set_file_name(format!("{}.curation.json", stem));
        p
    };

    if !curation_path.exists() {
        return Err(format!(
            "Curation file not found: {}\nRun 'crucible analyze {}' first.",
            curation_path.display(),
            file.display()
        )
        .into());
    }

    // Load curation layer
    let curation = CurationLayer::load(&curation_path)?;

    if json_output {
        // JSON output
        let status = serde_json::json!({
            "file": curation.source.file,
            "progress": curation.progress(),
            "total_suggestions": curation.suggestions.len(),
            "decisions": {
                "pending": curation.pending_suggestions().len(),
                "accepted": curation.decisions.iter().filter(|d| d.status == DecisionStatus::Accepted).count(),
                "modified": curation.decisions.iter().filter(|d| d.status == DecisionStatus::Modified).count(),
                "rejected": curation.decisions.iter().filter(|d| d.status == DecisionStatus::Rejected).count(),
                "applied": curation.decisions.iter().filter(|d| d.status == DecisionStatus::Applied).count(),
            },
            "observations": {
                "total": curation.observations.len(),
                "by_severity": curation.summary.observations_by_severity,
            },
            "data_quality_score": curation.summary.data_quality_score,
            "is_complete": curation.is_complete(),
        });
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        // Human-readable output
        println!(
            "{} {}",
            "Curation status for".cyan().bold(),
            curation.source.file.white()
        );
        println!();

        // Progress bar
        let progress = curation.progress();
        let total = curation.suggestions.len();
        let decided = (progress * total as f64).round() as usize;
        let bar_width = 30;
        let filled = (progress * bar_width as f64).round() as usize;
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        println!(
            "Progress: {} {}/{} ({:.0}%)",
            bar.cyan(),
            decided.to_string().white().bold(),
            total,
            progress * 100.0
        );
        println!();

        // Decision counts
        let pending = curation.pending_suggestions().len();
        let accepted = curation
            .decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Accepted)
            .count();
        let modified = curation
            .decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Modified)
            .count();
        let rejected = curation
            .decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Rejected)
            .count();
        let applied = curation
            .decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Applied)
            .count();

        println!("{}", "Decisions:".yellow().bold());
        println!("  Pending:  {}", pending.to_string().white());
        println!("  Accepted: {}", accepted.to_string().green());
        println!("  Modified: {}", modified.to_string().blue());
        println!("  Rejected: {}", rejected.to_string().red());
        if applied > 0 {
            println!("  Applied:  {}", applied.to_string().magenta());
        }
        println!();

        // Observation summary
        println!("{}", "Observations:".yellow().bold());
        println!(
            "  Errors:   {}",
            curation
                .summary
                .observations_by_severity
                .error
                .to_string()
                .red()
        );
        println!(
            "  Warnings: {}",
            curation
                .summary
                .observations_by_severity
                .warning
                .to_string()
                .yellow()
        );
        println!(
            "  Info:     {}",
            curation
                .summary
                .observations_by_severity
                .info
                .to_string()
                .blue()
        );
        println!();

        // Quality score
        let score = curation.summary.data_quality_score * 100.0;
        let score_color = if score >= 80.0 {
            score.to_string().green()
        } else if score >= 50.0 {
            score.to_string().yellow()
        } else {
            score.to_string().red()
        };
        println!("Data quality score: {}%", score_color);
        println!();

        // Next steps
        if curation.is_complete() {
            if applied == total {
                println!("{}", "All decisions have been applied!".green().bold());
            } else {
                println!(
                    "All suggestions reviewed. Run {} to apply changes.",
                    format!("crucible apply {}", curation_path.display())
                        .cyan()
                        .bold()
                );
            }
        } else {
            println!(
                "Run {} to continue reviewing.",
                format!("crucible review {}", curation_path.display())
                    .cyan()
                    .bold()
            );
        }
    }

    Ok(())
}
