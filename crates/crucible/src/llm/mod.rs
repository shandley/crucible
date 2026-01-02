//! LLM provider integration for enhanced inference and explanations.
//!
//! This module provides optional LLM capabilities for:
//! - Enhancing schema inference with semantic insights
//! - Explaining data quality observations in natural language
//! - Generating actionable suggestions for data issues
//!
//! The LLM integration is optional - Crucible works fully without it.

mod anthropic;
mod mock;
mod prompts;
mod provider;

pub use anthropic::AnthropicProvider;
pub use mock::MockProvider;
pub use provider::{LlmConfig, LlmProvider, SchemaEnhancement};
