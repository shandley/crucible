//! Review command - open web UI for interactive curation.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{Crucible, CurationContext, CurationLayer, MockProvider};

use crate::server::{app, state::AppState};

pub fn run(
    file: PathBuf,
    port: u16,
    no_open: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine if file is data or curation file
    let (data_path, curation_path) = if file
        .extension()
        .map(|e| e == "json")
        .unwrap_or(false)
        && file.to_string_lossy().contains(".curation.")
    {
        // It's a curation file, derive data path
        let stem = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .replace(".curation.json", "");
        let data_path = file.with_file_name(format!("{}.tsv", stem));
        (data_path, file.clone())
    } else {
        // It's a data file, derive curation path
        let stem = file.file_stem().unwrap_or_default().to_string_lossy();
        let curation_path = file.with_file_name(format!("{}.curation.json", stem));
        (file.clone(), curation_path)
    };

    // Load or create curation layer
    let curation = if curation_path.exists() {
        if verbose {
            println!("Loading existing curation from {}", curation_path.display());
        }
        CurationLayer::load(&curation_path)?
    } else {
        println!(
            "{} No curation file found, analyzing {}...",
            "Note:".yellow(),
            data_path.display()
        );

        // Analyze the data file
        let crucible = Crucible::new().with_llm(MockProvider::new());
        let result = crucible.analyze(&data_path)?;

        let context = CurationContext::new();
        let curation = CurationLayer::from_analysis(result, context);

        // Save the new curation file
        curation.save(&curation_path)?;
        println!("Created {}", curation_path.display());

        curation
    };

    // Create app state
    let state = AppState::new(curation, curation_path.clone(), data_path.clone());

    // Print server info
    let url = format!("http://localhost:{}", port);
    println!();
    println!(
        "{} {}",
        "Starting review server at".cyan().bold(),
        url.white().bold()
    );
    println!();
    println!("  File: {}", data_path.display());
    println!("  Curation: {}", curation_path.display());
    println!();
    println!("Press {} to stop the server", "Ctrl+C".yellow().bold());
    println!();

    // Open browser if requested
    if !no_open {
        if let Err(e) = open::that(&url) {
            eprintln!(
                "{} Could not open browser: {}",
                "Warning:".yellow(),
                e
            );
        }
    }

    // Run the server
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async {
        // Set up Ctrl+C handler
        let state_clone = state.clone();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            println!();
            println!("{}", "Shutting down...".yellow());
            if let Err(e) = state_clone.save().await {
                eprintln!("Error saving: {}", e);
            }
            std::process::exit(0);
        });

        if let Err(e) = app::run_server(state, port).await {
            eprintln!("Server error: {}", e);
        }
    });

    Ok(())
}
