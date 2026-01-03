//! CLI argument definitions using clap.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Crucible: LLM-native data curation tool
#[derive(Parser)]
#[command(name = "crucible")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze a data file and create a curation layer
    Analyze {
        /// Path to the data file (CSV/TSV)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output path for curation file (default: <file>.curation.json)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Domain context for analysis (e.g., "biomedical", "genomics")
        #[arg(short, long)]
        domain: Option<String>,

        /// LLM provider to use for enhanced analysis
        #[arg(long, default_value = "none")]
        llm: LlmProviderChoice,

        /// Model to use (provider-specific, e.g., "gpt-4o", "llama3.2")
        #[arg(long)]
        model: Option<String>,

        /// MIxS environmental package for bioinformatics validation
        #[arg(long)]
        mixs_package: Option<MixsPackageChoice>,
    },

    /// Open web UI for interactive curation review
    Review {
        /// Path to data file or curation file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Port for web server
        #[arg(short, long, default_value = "3141")]
        port: u16,

        /// Don't automatically open browser
        #[arg(long)]
        no_open: bool,
    },

    /// Apply accepted decisions and export curated data
    Apply {
        /// Path to curation file
        #[arg(value_name = "CURATION_FILE")]
        file: PathBuf,

        /// Output path for curated data
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "tsv")]
        format: OutputFormat,

        /// Include audit metadata columns
        #[arg(long)]
        with_audit: bool,
    },

    /// Show curation progress and summary
    Status {
        /// Path to curation file or data file
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Preview changes that would be applied
    Diff {
        /// Path to curation file
        #[arg(value_name = "CURATION_FILE")]
        file: PathBuf,

        /// Number of context lines around changes
        #[arg(short = 'C', long, default_value = "3")]
        context: usize,

        /// Show only changed rows
        #[arg(long)]
        changed_only: bool,
    },

    /// Batch accept or reject suggestions by type
    Batch {
        /// Path to curation file
        #[arg(value_name = "CURATION_FILE")]
        file: PathBuf,

        /// Accept suggestions (cannot use with --reject)
        #[arg(long, conflicts_with = "reject")]
        accept: bool,

        /// Reject suggestions (cannot use with --accept)
        #[arg(long, conflicts_with = "accept")]
        reject: bool,

        /// Filter by action type (standardize, convert_na, flag, coerce, convert_date)
        #[arg(long, short = 't')]
        action_type: Option<String>,

        /// Filter by column name
        #[arg(long, short = 'c')]
        column: Option<String>,

        /// Accept/reject all pending suggestions
        #[arg(long)]
        all: bool,

        /// User name for the decision
        #[arg(long, default_value = "batch")]
        user: String,
    },
}

#[derive(Clone, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Tsv,
    Csv,
    Json,
    #[cfg(feature = "parquet")]
    Parquet,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tsv" => Ok(OutputFormat::Tsv),
            "csv" => Ok(OutputFormat::Csv),
            "json" => Ok(OutputFormat::Json),
            #[cfg(feature = "parquet")]
            "parquet" => Ok(OutputFormat::Parquet),
            #[cfg(not(feature = "parquet"))]
            "parquet" => Err("Parquet support not enabled. Rebuild with --features parquet".to_string()),
            _ => Err(format!("Unknown format: {}. Use tsv, csv, or json.", s)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Tsv => write!(f, "tsv"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Json => write!(f, "json"),
            #[cfg(feature = "parquet")]
            OutputFormat::Parquet => write!(f, "parquet"),
        }
    }
}

/// LLM provider choice for analysis
#[derive(Clone, Debug, Default)]
pub enum LlmProviderChoice {
    /// No LLM - use rule-based analysis only
    #[default]
    None,
    /// Anthropic Claude API (requires ANTHROPIC_API_KEY)
    Anthropic,
    /// OpenAI GPT API (requires OPENAI_API_KEY)
    OpenAI,
    /// Ollama local models (requires Ollama running)
    Ollama,
    /// Mock provider for testing
    Mock,
}

impl std::str::FromStr for LlmProviderChoice {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(LlmProviderChoice::None),
            "anthropic" | "claude" => Ok(LlmProviderChoice::Anthropic),
            "openai" | "gpt" => Ok(LlmProviderChoice::OpenAI),
            "ollama" | "local" => Ok(LlmProviderChoice::Ollama),
            "mock" | "test" => Ok(LlmProviderChoice::Mock),
            _ => Err(format!(
                "Unknown provider: {}. Use: none, anthropic, openai, ollama, or mock.",
                s
            )),
        }
    }
}

impl std::fmt::Display for LlmProviderChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProviderChoice::None => write!(f, "none"),
            LlmProviderChoice::Anthropic => write!(f, "anthropic"),
            LlmProviderChoice::OpenAI => write!(f, "openai"),
            LlmProviderChoice::Ollama => write!(f, "ollama"),
            LlmProviderChoice::Mock => write!(f, "mock"),
        }
    }
}

/// MIxS environmental package for bioinformatics validation
#[derive(Clone, Debug)]
pub enum MixsPackageChoice {
    /// Human gut microbiome
    HumanGut,
    /// Human oral microbiome
    HumanOral,
    /// Human skin microbiome
    HumanSkin,
    /// Human vaginal microbiome
    HumanVaginal,
    /// Human-associated (general)
    HumanAssociated,
    /// Soil microbiome
    Soil,
    /// Water microbiome
    Water,
    /// Marine microbiome
    Marine,
    /// Air microbiome
    Air,
    /// Sediment microbiome
    Sediment,
    /// Plant-associated microbiome
    PlantAssociated,
    /// Built environment
    BuiltEnvironment,
    /// Host-associated (general)
    HostAssociated,
    /// Microbial mat/biofilm
    MicrobialMat,
    /// Miscellaneous natural environment
    MiscNatural,
    /// Auto-detect based on data
    Auto,
}

impl std::str::FromStr for MixsPackageChoice {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "human_gut" | "humangut" | "gut" => Ok(MixsPackageChoice::HumanGut),
            "human_oral" | "humanoral" | "oral" => Ok(MixsPackageChoice::HumanOral),
            "human_skin" | "humanskin" | "skin" => Ok(MixsPackageChoice::HumanSkin),
            "human_vaginal" | "humanvaginal" | "vaginal" => Ok(MixsPackageChoice::HumanVaginal),
            "human_associated" | "humanassociated" | "human" => Ok(MixsPackageChoice::HumanAssociated),
            "soil" => Ok(MixsPackageChoice::Soil),
            "water" | "freshwater" => Ok(MixsPackageChoice::Water),
            "marine" | "ocean" => Ok(MixsPackageChoice::Marine),
            "air" => Ok(MixsPackageChoice::Air),
            "sediment" => Ok(MixsPackageChoice::Sediment),
            "plant_associated" | "plantassociated" | "plant" => Ok(MixsPackageChoice::PlantAssociated),
            "built_environment" | "builtenvironment" | "built" => Ok(MixsPackageChoice::BuiltEnvironment),
            "host_associated" | "hostassociated" | "host" => Ok(MixsPackageChoice::HostAssociated),
            "microbial_mat" | "microbialmat" | "biofilm" => Ok(MixsPackageChoice::MicrobialMat),
            "misc_natural" | "miscnatural" | "misc" => Ok(MixsPackageChoice::MiscNatural),
            "auto" | "detect" => Ok(MixsPackageChoice::Auto),
            _ => Err(format!(
                "Unknown MIxS package: {}. Use: human-gut, soil, water, marine, etc.",
                s
            )),
        }
    }
}

impl std::fmt::Display for MixsPackageChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MixsPackageChoice::HumanGut => write!(f, "human-gut"),
            MixsPackageChoice::HumanOral => write!(f, "human-oral"),
            MixsPackageChoice::HumanSkin => write!(f, "human-skin"),
            MixsPackageChoice::HumanVaginal => write!(f, "human-vaginal"),
            MixsPackageChoice::HumanAssociated => write!(f, "human-associated"),
            MixsPackageChoice::Soil => write!(f, "soil"),
            MixsPackageChoice::Water => write!(f, "water"),
            MixsPackageChoice::Marine => write!(f, "marine"),
            MixsPackageChoice::Air => write!(f, "air"),
            MixsPackageChoice::Sediment => write!(f, "sediment"),
            MixsPackageChoice::PlantAssociated => write!(f, "plant-associated"),
            MixsPackageChoice::BuiltEnvironment => write!(f, "built-environment"),
            MixsPackageChoice::HostAssociated => write!(f, "host-associated"),
            MixsPackageChoice::MicrobialMat => write!(f, "microbial-mat"),
            MixsPackageChoice::MiscNatural => write!(f, "misc-natural"),
            MixsPackageChoice::Auto => write!(f, "auto"),
        }
    }
}
