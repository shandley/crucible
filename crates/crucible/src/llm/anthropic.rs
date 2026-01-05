//! Anthropic Claude API provider implementation.

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

/// Anthropic API endpoint.
const API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Anthropic API version.
const API_VERSION: &str = "2023-06-01";

/// Anthropic Claude provider.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    config: LlmConfig,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        Self::with_config(api_key, LlmConfig::default())
    }

    /// Create a new Anthropic provider with custom configuration.
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
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            CrucibleError::Config(
                "ANTHROPIC_API_KEY environment variable not set".to_string(),
            )
        })?;
        Self::new(api_key)
    }

    /// Build headers for API requests.
    fn build_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| CrucibleError::Config(format!("Invalid API key: {}", e)))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(API_VERSION),
        );
        Ok(headers)
    }

    /// Send a message to the Claude API.
    fn send_message(&self, user_prompt: &str) -> Result<String> {
        let body = json!({
            "model": self.config.model,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
            "system": prompts::system_prompt(),
            "messages": [
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
                "API error ({}): {}",
                status, error_text
            )));
        }

        let api_response: ApiResponse = response
            .json()
            .map_err(|e| CrucibleError::Config(format!("Failed to parse API response: {}", e)))?;

        // Extract text from response
        api_response
            .content
            .into_iter()
            .find_map(|block| {
                if block.content_type == "text" {
                    Some(block.text)
                } else {
                    None
                }
            })
            .ok_or_else(|| CrucibleError::Config("No text in API response".to_string()))
    }

    /// Parse JSON from LLM response, handling markdown code blocks.
    fn parse_json_response<T: for<'de> Deserialize<'de>>(&self, response: &str) -> Result<T> {
        // Try to extract JSON from markdown code block if present
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

impl LlmProvider for AnthropicProvider {
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

        // For explanations, we just return the text directly
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

        // If no action suggested, return None
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
            .with_suggester("anthropic_llm");

        Ok(Some(suggestion))
    }

    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn answer_question(
        &self,
        question_context: &super::provider::QuestionContext,
        hints: &ContextHints,
    ) -> Result<super::provider::QuestionResponse> {
        let prompt = prompts::question_prompt(
            &question_context.question,
            question_context.observation.as_ref(),
            question_context.suggestion.as_ref(),
            question_context.column.as_ref(),
            &question_context.sample_values,
            hints,
        );

        let response = self.send_message(&prompt)?;
        let parsed: QuestionResponseParsed = self.parse_json_response(&response)?;

        Ok(super::provider::QuestionResponse {
            answer: parsed.answer,
            confidence: parsed.confidence,
            follow_up_questions: parsed.follow_up_questions,
        })
    }

    fn calibrate_confidence(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        hints: &ContextHints,
    ) -> Result<super::provider::CalibratedConfidence> {
        let prompt = prompts::confidence_calibration_prompt(observation, column, hints);
        let response = self.send_message(&prompt)?;
        let parsed: ConfidenceCalibrationResponse = self.parse_json_response(&response)?;

        Ok(super::provider::CalibratedConfidence {
            confidence: parsed.calibrated_confidence,
            original_confidence: observation.confidence,
            reasoning: parsed.reasoning,
            factors: parsed
                .factors
                .into_iter()
                .map(|f| super::provider::ConfidenceFactor {
                    name: f.name,
                    impact: f.impact,
                    explanation: f.explanation,
                })
                .collect(),
        })
    }
}

/// Anthropic API response structure.
#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

/// Content block in API response.
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
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

/// Parsed question response.
#[derive(Debug, Deserialize)]
struct QuestionResponseParsed {
    answer: String,
    #[serde(default)]
    confidence: f64,
    #[serde(default)]
    follow_up_questions: Vec<String>,
}

/// Parsed confidence calibration response.
#[derive(Debug, Deserialize)]
struct ConfidenceCalibrationResponse {
    calibrated_confidence: f64,
    reasoning: String,
    #[serde(default)]
    factors: Vec<ConfidenceFactorParsed>,
}

/// Parsed confidence factor.
#[derive(Debug, Deserialize)]
struct ConfidenceFactorParsed {
    name: String,
    #[serde(default)]
    impact: f64,
    #[serde(default)]
    explanation: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_from_markdown() {
        let provider = AnthropicProvider {
            client: Client::new(),
            api_key: "test".to_string(),
            config: LlmConfig::default(),
        };

        let response = r#"```json
{
    "insight": "Test insight",
    "confidence": 0.9
}
```"#;

        let parsed: SchemaEnhancementResponse = provider.parse_json_response(response).unwrap();
        assert_eq!(parsed.insight, "Test insight");
        assert_eq!(parsed.confidence, 0.9);
    }

    #[test]
    fn test_parse_plain_json() {
        let provider = AnthropicProvider {
            client: Client::new(),
            api_key: "test".to_string(),
            config: LlmConfig::default(),
        };

        let response = r#"{"insight": "Test", "confidence": 0.8}"#;

        let parsed: SchemaEnhancementResponse = provider.parse_json_response(response).unwrap();
        assert_eq!(parsed.insight, "Test");
    }
}
