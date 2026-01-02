//! Crucible CLI - LLM-native data curation tool.

mod cli;
mod commands;
mod server;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Analyze {
            file,
            output,
            domain,
            mock_llm,
        } => commands::analyze::run(file, output, domain, mock_llm, cli.verbose),

        Commands::Review {
            file,
            port,
            no_open,
        } => commands::review::run(file, port, no_open, cli.verbose),

        Commands::Apply {
            file,
            output,
            format,
            with_audit,
        } => commands::apply::run(file, output, format, with_audit, cli.verbose),

        Commands::Status { file, json } => commands::status::run(file, json, cli.verbose),

        Commands::Diff {
            file,
            context,
            changed_only,
        } => commands::diff::run(file, context, changed_only, cli.verbose),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
