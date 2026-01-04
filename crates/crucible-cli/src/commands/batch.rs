//! Batch command - accept or reject multiple suggestions at once.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{CurationLayer, DecisionStatus};

pub fn run(
    file: PathBuf,
    accept: bool,
    reject: bool,
    action_type: Option<String>,
    column: Option<String>,
    all: bool,
    user: String,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("Curation file not found: {}", file.display()).into());
    }

    if !accept && !reject {
        return Err("Must specify --accept or --reject".into());
    }

    if !all && action_type.is_none() && column.is_none() {
        return Err("Must specify --all, --action-type, or --column to filter suggestions".into());
    }

    // Load curation layer
    let mut curation = CurationLayer::load(&file)?;

    // Find matching suggestions
    let matching_suggestions: Vec<String> = curation
        .suggestions
        .iter()
        .filter(|s| {
            // Check if already decided
            let already_decided = curation
                .decisions
                .iter()
                .any(|d| d.suggestion_id == s.id);
            if already_decided {
                return false;
            }

            // Filter by action type
            if let Some(ref action_filter) = action_type {
                let action_str = format!("{:?}", s.action).to_lowercase();
                let filter_lower = action_filter.to_lowercase();
                if !action_str.contains(&filter_lower) && !filter_lower.contains(&action_str) {
                    // Also check snake_case format
                    let snake = match s.action {
                        crucible::SuggestionAction::Standardize => "standardize",
                        crucible::SuggestionAction::ConvertNa => "convert_na",
                        crucible::SuggestionAction::Coerce => "coerce",
                        crucible::SuggestionAction::ConvertDate => "convert_date",
                        crucible::SuggestionAction::Flag => "flag",
                        crucible::SuggestionAction::Remove => "remove",
                        crucible::SuggestionAction::Merge => "merge",
                        crucible::SuggestionAction::Rename => "rename",
                        crucible::SuggestionAction::Split => "split",
                        crucible::SuggestionAction::Derive => "derive",
                    };
                    if !filter_lower.contains(snake) && snake != filter_lower {
                        return false;
                    }
                }
            }

            // Filter by column
            if let Some(ref col_filter) = column {
                let suggestion_col = s
                    .parameters
                    .get("column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !suggestion_col.eq_ignore_ascii_case(col_filter) {
                    return false;
                }
            }

            true
        })
        .map(|s| s.id.clone())
        .collect();

    if matching_suggestions.is_empty() {
        println!(
            "{} No pending suggestions match the filter criteria.",
            "Note:".yellow()
        );
        return Ok(());
    }

    let action_word = if accept { "Accepting" } else { "Rejecting" };
    let _status = if accept {
        DecisionStatus::Accepted
    } else {
        DecisionStatus::Rejected
    };

    println!(
        "{} {} suggestion(s)...",
        action_word.cyan().bold(),
        matching_suggestions.len().to_string().white().bold()
    );

    // Apply decisions
    let mut count = 0;
    for suggestion_id in &matching_suggestions {
        if accept {
            curation.accept_by(suggestion_id, &user)?;
        } else {
            curation.reject_by(suggestion_id, &user, "Batch rejected")?;
        }
        count += 1;

        if verbose {
            // Find the suggestion to show details
            if let Some(s) = curation.suggestions.iter().find(|s| s.id == *suggestion_id) {
                let col = s
                    .parameters
                    .get("column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!(
                    "  {} {} [{}] {}",
                    "•".dimmed(),
                    suggestion_id,
                    col.cyan(),
                    s.action.label()
                );
            }
        }
    }

    // Save the updated curation layer
    curation.save(&file)?;

    println!();
    println!(
        "{} {} suggestion(s) {}",
        "Done:".green().bold(),
        count.to_string().white().bold(),
        if accept { "accepted" } else { "rejected" }
    );

    // Show remaining pending count
    let pending: usize = curation
        .suggestions
        .iter()
        .filter(|s| {
            !curation
                .decisions
                .iter()
                .any(|d| d.suggestion_id == s.id)
        })
        .count();

    if pending > 0 {
        println!(
            "  {} pending suggestion(s) remaining",
            pending.to_string().yellow()
        );
    } else {
        println!("  {} All suggestions have been decided!", "✓".green());
    }

    Ok(())
}
