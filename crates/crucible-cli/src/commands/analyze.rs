//! Analyze command - analyze a data file and create curation layer.

use std::path::PathBuf;

use colored::Colorize;
use crucible::{
    bio::{BioSampleValidator, BioValidator, MixsComplianceValidator, MixsPackage},
    AnthropicProvider, ContextHints, Crucible, CurationContext, CurationLayer, LlmConfig,
    MockProvider, OllamaProvider, OpenAIProvider, Parser, Severity,
};

use crate::cli::{LlmProviderChoice, MixsPackageChoice};

pub fn run(
    file: PathBuf,
    output: Option<PathBuf>,
    domain: Option<String>,
    llm: LlmProviderChoice,
    model: Option<String>,
    mixs_package: Option<MixsPackageChoice>,
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

    // Build Crucible instance with selected LLM provider
    let crucible = create_crucible_with_provider(llm, model, verbose)?;

    // Add domain context if provided
    let crucible = if let Some(ref d) = domain {
        crucible.with_context(ContextHints::new().with_domain(d))
    } else {
        crucible
    };

    // Run analysis
    let mut result = crucible.analyze(&file)?;

    // Run MIxS compliance validation if requested
    if let Some(ref pkg) = mixs_package {
        let is_auto = matches!(pkg, MixsPackageChoice::Auto);
        let mixs_pkg = convert_mixs_package(pkg);

        // Parse the file to get data for bio validation
        let parser = Parser::new();
        let (table, _) = parser.parse_file(&file)?;

        let mut validator = MixsComplianceValidator::new();
        if !is_auto {
            validator = validator.with_package(mixs_pkg);
        }

        let bio_observations = validator.validate(&table, &result.schema);

        if verbose {
            println!();
            println!("{}", "MIxS Compliance:".yellow().bold());
            let detected = validator.detect_package(&table, &result.schema);
            println!("  Package: {:?}", detected.unwrap_or(mixs_pkg));
            let score = validator.compliance_score(&table, &result.schema);
            println!("  Compliance score: {:.0}%", score * 100.0);
        }

        // Count bio observations by severity
        let bio_errors = bio_observations.iter().filter(|o| o.severity == Severity::Error).count();
        let bio_warnings = bio_observations.iter().filter(|o| o.severity == Severity::Warning).count();

        if bio_errors > 0 || bio_warnings > 0 {
            println!(
                "{} MIxS issues: {} errors, {} warnings",
                "Found".cyan(),
                bio_errors.to_string().red(),
                bio_warnings.to_string().yellow()
            );
        }

        // Merge bio observations into result
        result.observations.extend(bio_observations);

        // Run NCBI BioSample pre-validation
        let biosample_validator = BioSampleValidator::new();
        let detected_pkg = if is_auto {
            validator.detect_package(&table, &result.schema)
        } else {
            Some(mixs_pkg)
        };
        let readiness = biosample_validator.check_readiness(&table, &result.schema, detected_pkg);

        println!();
        let ready_status = if readiness.is_ready {
            "READY".green().bold()
        } else {
            "NOT READY".red().bold()
        };
        println!(
            "{} {}% ({})",
            "NCBI Readiness:".yellow().bold(),
            readiness.score,
            ready_status
        );

        if verbose && !readiness.blocking_issues.is_empty() {
            println!();
            println!("{}", "Blocking issues (must fix):".red());
            for issue in &readiness.blocking_issues {
                println!("  {} {}", "✗".red(), issue.description);
            }
        }

        if verbose && !readiness.warning_issues.is_empty() {
            println!();
            println!("{}", "Warnings (should fix):".yellow());
            for issue in readiness.warning_issues.iter().take(5) {
                println!("  {} {}", "⚠".yellow(), issue.description);
            }
            if readiness.warning_issues.len() > 5 {
                println!(
                    "  {} ... and {} more",
                    "⚠".yellow(),
                    readiness.warning_issues.len() - 5
                );
            }
        }

        // Add BioSample observations to result
        let biosample_observations = biosample_validator.to_observations(&readiness);
        result.observations.extend(biosample_observations);
    }

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

/// Create a Crucible instance with the selected LLM provider.
fn create_crucible_with_provider(
    provider: LlmProviderChoice,
    model: Option<String>,
    verbose: bool,
) -> Result<Crucible, Box<dyn std::error::Error>> {
    let crucible = Crucible::new();

    match provider {
        LlmProviderChoice::None => {
            if verbose {
                println!("  {} rule-based analysis (no LLM)", "Using".dimmed());
            }
            Ok(crucible)
        }
        LlmProviderChoice::Anthropic => {
            if verbose {
                println!("  {} Anthropic Claude API", "Using".dimmed());
            }
            let mut provider = AnthropicProvider::from_env()?;
            if let Some(m) = model {
                let mut config = LlmConfig::default();
                config.model = m;
                provider = AnthropicProvider::with_config(
                    std::env::var("ANTHROPIC_API_KEY")?,
                    config,
                )?;
            }
            Ok(crucible.with_llm(provider))
        }
        LlmProviderChoice::OpenAI => {
            if verbose {
                println!("  {} OpenAI API", "Using".dimmed());
            }
            let mut provider = OpenAIProvider::from_env()?;
            if let Some(m) = model {
                let mut config = LlmConfig::default();
                config.model = m;
                provider = OpenAIProvider::with_config(
                    std::env::var("OPENAI_API_KEY")?,
                    config,
                )?;
            }
            Ok(crucible.with_llm(provider))
        }
        LlmProviderChoice::Ollama => {
            let model_name = model.as_deref().unwrap_or("llama3.2");
            if verbose {
                println!("  {} Ollama local model: {}", "Using".dimmed(), model_name);
            }
            let provider = OllamaProvider::with_model(model_name)?;
            Ok(crucible.with_llm(provider))
        }
        LlmProviderChoice::Mock => {
            if verbose {
                println!("  {} mock LLM (for testing)", "Using".dimmed());
            }
            Ok(crucible.with_llm(MockProvider::new()))
        }
    }
}

/// Convert CLI MixsPackageChoice to library MixsPackage.
fn convert_mixs_package(choice: &MixsPackageChoice) -> MixsPackage {
    match choice {
        MixsPackageChoice::HumanGut => MixsPackage::HumanGut,
        MixsPackageChoice::HumanOral => MixsPackage::HumanOral,
        MixsPackageChoice::HumanSkin => MixsPackage::HumanSkin,
        MixsPackageChoice::HumanVaginal => MixsPackage::HumanVaginal,
        MixsPackageChoice::HumanAssociated => MixsPackage::HumanAssociated,
        MixsPackageChoice::Soil => MixsPackage::Soil,
        MixsPackageChoice::Water => MixsPackage::Water,
        MixsPackageChoice::Marine => MixsPackage::Water, // Marine uses water package
        MixsPackageChoice::Air => MixsPackage::Air,
        MixsPackageChoice::Sediment => MixsPackage::Sediment,
        MixsPackageChoice::PlantAssociated => MixsPackage::PlantAssociated,
        MixsPackageChoice::BuiltEnvironment => MixsPackage::BuiltEnvironment,
        MixsPackageChoice::HostAssociated => MixsPackage::HostAssociated,
        MixsPackageChoice::MicrobialMat => MixsPackage::MicrobialMatBiofilm,
        MixsPackageChoice::MiscNatural => MixsPackage::MiscellaneousNaturalOrArtificialEnvironment,
        MixsPackageChoice::Auto => MixsPackage::HumanGut, // Default for auto-detect start
    }
}
