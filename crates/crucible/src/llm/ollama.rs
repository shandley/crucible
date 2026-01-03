//! Ollama local LLM provider implementation.
//!
//! Ollama allows running LLMs locally without API keys.
//! Install from: https://ollama.ai

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{CrucibleError, Result};
use crate::input::ContextHints;
use crate::schema::ColumnSchema;
use crate::suggestion::{Suggestion, SuggestionAction};
use crate::validation::Observation;

use super::prompts;
use super::provider::{LlmConfig, LlmProvider, SchemaEnhancement};

/// Default Ollama API endpoint.
const DEFAULT_API_URL: &str = "http://localhost:11434/api/chat";

/// Ollama local LLM provider.
pub struct OllamaProvider {
    client: Client,
    api_url: String,
    config: LlmConfig,
}

impl OllamaProvider {
    /// Create a new Ollama provider with default settings.
    ///
    /// Uses llama3.2 model by default. Make sure you've pulled it:
    /// `ollama pull llama3.2`
    pub fn new() -> Result<Self> {
        let mut config = LlmConfig::default();
        config.model = "llama3.2".to_string();
        Self::with_config(config)
    }

    /// Create with a specific model.
    ///
    /// Popular models for data analysis:
    /// - `llama3.2` - Good balance of speed and quality
    /// - `llama3.1:70b` - Higher quality, slower
    /// - `mistral` - Fast, good for simple tasks
    /// - `codellama` - Good for technical/structured output
    pub fn with_model(model: impl Into<String>) -> Result<Self> {
        let mut config = LlmConfig::default();
        config.model = model.into();
        Self::with_config(config)
    }

    /// Create with custom configuration.
    pub fn with_config(config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120)) // Local models can be slower
            .build()
            .map_err(|e| CrucibleError::Config(format!("Failed to create HTTP client: {}", e)))?;

        let api_url = std::env::var("OLLAMA_HOST")
            .map(|host| format!("{}/api/chat", host.trim_end_matches('/')))
            .unwrap_or_else(|_| DEFAULT_API_URL.to_string());

        Ok(Self {
            client,
            api_url,
            config,
        })
    }

    /// Build headers for API requests.
    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    /// Send a message to Ollama.
    fn send_message(&self, user_prompt: &str) -> Result<String> {
        let body = json!({
            "model": self.config.model,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens
            },
            "messages": [
                {
                    "role": "system",
                    "content": prompts::system_prompt()
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

        let response = self
            .client
            .post(&self.api_url)
            .headers(self.build_headers())
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_connect() {
                    CrucibleError::Config(
                        "Failed to connect to Ollama. Is it running? Start with: ollama serve"
                            .to_string(),
                    )
                } else {
                    CrucibleError::Config(format!("Ollama request failed: {}", e))
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();

            // Check for model not found error
            if error_text.contains("not found") {
                return Err(CrucibleError::Config(format!(
                    "Model '{}' not found. Pull it with: ollama pull {}",
                    self.config.model, self.config.model
                )));
            }

            return Err(CrucibleError::Config(format!(
                "Ollama error ({}): {}",
                status, error_text
            )));
        }

        let api_response: OllamaResponse = response
            .json()
            .map_err(|e| CrucibleError::Config(format!("Failed to parse Ollama response: {}", e)))?;

        Ok(api_response.message.content)
    }

    /// Parse JSON from LLM response, handling markdown code blocks.
    fn parse_json_response<T: for<'de> Deserialize<'de>>(&self, response: &str) -> Result<T> {
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .map(|s| s.trim())
                .unwrap_or(response)
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .map(|s| s.trim())
                .unwrap_or(response)
        } else {
            response.trim()
        };

        serde_json::from_str(json_str)
            .map_err(|e| CrucibleError::Config(format!("Failed to parse LLM JSON response: {}", e)))
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create default Ollama provider")
    }
}

impl LlmProvider for OllamaProvider {
    fn enhance_schema(
        &self,
        column: &ColumnSchema,
        samples: &[String],
        context: &ContextHints,
    ) -> Result<SchemaEnhancement> {
        if !self.config.enhance_schema {
            return Ok(SchemaEnhancement {
                insight: String::new(),
                suggested_role: None,
                suggested_constraints: None,
                confidence: 0.0,
            });
        }

        let prompt = prompts::schema_enhancement_prompt(column, samples, context);
        let response = self.send_message(&prompt)?;

        let parsed: SchemaEnhancementResponse = self.parse_json_response(&response)?;

        Ok(SchemaEnhancement {
            insight: parsed.insight,
            suggested_role: parsed.suggested_role,
            suggested_constraints: parsed.potential_issues,
            confidence: parsed.confidence,
        })
    }

    fn explain_observation(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        context: &ContextHints,
    ) -> Result<String> {
        if !self.config.explain_observations {
            return Ok(String::new());
        }

        let prompt = prompts::observation_explanation_prompt(observation, column, context);
        let response = self.send_message(&prompt)?;

        Ok(response.trim().to_string())
    }

    fn generate_suggestion(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        context: &ContextHints,
    ) -> Result<Option<Suggestion>> {
        if !self.config.generate_suggestions {
            return Ok(None);
        }

        let prompt = prompts::suggestion_prompt(observation, column, context);
        let response = self.send_message(&prompt)?;

        let parsed: SuggestionResponse = self.parse_json_response(&response)?;

        let action = match parsed.action.as_deref() {
            Some("standardize") => SuggestionAction::Standardize,
            Some("convert_na") => SuggestionAction::ConvertNa,
            Some("coerce") => SuggestionAction::Coerce,
            Some("flag") => SuggestionAction::Flag,
            Some("remove") => SuggestionAction::Remove,
            Some("merge") => SuggestionAction::Merge,
            _ => return Ok(None),
        };

        let suggestion = Suggestion::new(&observation.id, action, parsed.rationale)
            .with_parameters(parsed.parameters.unwrap_or(Value::Null))
            .with_confidence(parsed.confidence.unwrap_or(0.5))
            .with_priority(parsed.priority.unwrap_or(5))
            .with_suggester("ollama_llm");

        Ok(Some(suggestion))
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn name(&self) -> &str {
        "ollama"
    }
}

/// Ollama API response structure.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

/// Parsed schema enhancement response.
#[derive(Debug, Deserialize)]
struct SchemaEnhancementResponse {
    insight: String,
    #[serde(default)]
    suggested_role: Option<String>,
    #[serde(default)]
    potential_issues: Option<String>,
    #[serde(default)]
    confidence: f64,
}

/// Parsed suggestion response.
#[derive(Debug, Deserialize)]
struct SuggestionResponse {
    action: Option<String>,
    rationale: String,
    #[serde(default)]
    parameters: Option<Value>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    priority: Option<u8>,
}
