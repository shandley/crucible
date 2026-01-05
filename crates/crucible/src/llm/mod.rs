//! LLM provider integration for enhanced inference and explanations.
//!
//! This module provides optional LLM capabilities for:
//! - Enhancing schema inference with semantic insights
//! - Explaining data quality observations in natural language
//! - Generating actionable suggestions for data issues
//!
//! The LLM integration is optional - Crucible works fully without it.
//!
//! # Supported Providers
//!
//! - **Anthropic** - Claude models via API (requires `ANTHROPIC_API_KEY`)
//! - **OpenAI** - GPT models via API (requires `OPENAI_API_KEY`)
//! - **Ollama** - Local models, no API key needed (requires Ollama installed)
//!
//! # Example
//!
//! ```no_run
//! use crucible::{Crucible, OllamaProvider};
//!
//! // Use a free local model
//! let crucible = Crucible::new()
//!     .with_llm(OllamaProvider::new().unwrap());
//!
//! // Or use Anthropic API
//! // let crucible = Crucible::new()
//! //     .with_llm(AnthropicProvider::from_env().unwrap());
//! ```

mod anthropic;
mod mock;
mod ollama;
mod openai;
mod prompts;
mod provider;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use provider::{
    CalibratedConfidence, ConfidenceFactor, LlmConfig, LlmProvider, QuestionContext,
    QuestionResponse, SchemaEnhancement,
};
