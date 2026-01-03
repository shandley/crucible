//! OpenAI GPT API provider implementation.

use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::{CrucibleError, Result};
use crate::input::ContextHints;
use crate::schema::ColumnSchema;
use crate::suggestion::{Suggestion, SuggestionAction};
use crate::validation::Observation;

use super::prompts;
use super::provider::{LlmConfig, LlmProvider, SchemaEnhancement};

/// OpenAI API endpoint.
const API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI GPT provider.
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    config: LlmConfig,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let mut config = LlmConfig::default();
        config.model = "gpt-4o".to_string();
        Self::with_config(api_key, config)
    }

    /// Create a new OpenAI provider with custom configuration.
    pub fn with_config(api_key: impl Into<String>, config: LlmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| CrucibleError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            api_key: api_key.into(),
            config,
        })
    }

    /// Create from environment variable.
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            CrucibleError::Config("OPENAI_API_KEY environment variable not set".to_string())
        })?;
        Self::new(api_key)
    }

    /// Build headers for API requests.
    fn build_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|e| CrucibleError::Config(format!("Invalid API key: {}", e)))?,
        );
        Ok(headers)
    }

    /// Send a message to the OpenAI API.
    fn send_message(&self, user_prompt: &str) -> Result<String> {
        let body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
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
            .post(API_URL)
            .headers(self.build_headers()?)
            .json(&body)
            .send()
            .map_err(|e| CrucibleError::Config(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            return Err(CrucibleError::Config(format!(
                "OpenAI API error ({}): {}",
                status, error_text
            )));
        }

        let api_response: OpenAIResponse = response
            .json()
            .map_err(|e| CrucibleError::Config(format!("Failed to parse API response: {}", e)))?;

        // Extract text from response
        api_response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| CrucibleError::Config("No response from OpenAI".to_string()))
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

impl LlmProvider for OpenAIProvider {
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
            .with_suggester("openai_llm");

        Ok(Some(suggestion))
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn name(&self) -> &str {
        "openai"
    }
}

/// OpenAI API response structure.
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
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
