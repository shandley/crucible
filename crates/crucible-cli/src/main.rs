//! Crucible CLI - LLM-native data curation tool.

mod cli;
mod commands;
mod server;
mod web;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Analyze {
            file,
            output,
            domain,
            llm,
            model,
        } => commands::analyze::run(file, output, domain, llm, model, cli.verbose),

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

        Commands::Batch {
            file,
            accept,
            reject,
            action_type,
            column,
            all,
            user,
        } => commands::batch::run(file, accept, reject, action_type, column, all, user, cli.verbose),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
