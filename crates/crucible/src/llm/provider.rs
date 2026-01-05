//! LLM provider trait and types.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::input::ContextHints;
use crate::schema::ColumnSchema;
use crate::suggestion::Suggestion;
use crate::validation::Observation;

/// Enhancement result for a column schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaEnhancement {
    /// Human-readable insight about the column.
    pub insight: String,

    /// Suggested semantic role if different from inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_role: Option<String>,

    /// Additional constraints suggested by LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_constraints: Option<String>,

    /// Confidence in the enhancement (0.0-1.0).
    pub confidence: f64,
}

/// Context for an interactive question about data quality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionContext {
    /// The user's question.
    pub question: String,

    /// The observation being asked about (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation: Option<Observation>,

    /// The suggestion being asked about (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<Suggestion>,

    /// The column schema (if relevant).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<ColumnSchema>,

    /// Sample values from the data.
    #[serde(default)]
    pub sample_values: Vec<String>,
}

impl QuestionContext {
    /// Create a new question context.
    pub fn new(question: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            observation: None,
            suggestion: None,
            column: None,
            sample_values: Vec::new(),
        }
    }

    /// Add an observation to the context.
    pub fn with_observation(mut self, obs: Observation) -> Self {
        self.observation = Some(obs);
        self
    }

    /// Add a suggestion to the context.
    pub fn with_suggestion(mut self, sug: Suggestion) -> Self {
        self.suggestion = Some(sug);
        self
    }

    /// Add column schema to the context.
    pub fn with_column(mut self, col: ColumnSchema) -> Self {
        self.column = Some(col);
        self
    }

    /// Add sample values to the context.
    pub fn with_samples(mut self, samples: Vec<String>) -> Self {
        self.sample_values = samples;
        self
    }
}

/// Response to an interactive question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionResponse {
    /// The answer to the question.
    pub answer: String,

    /// Confidence in the answer (0.0-1.0).
    pub confidence: f64,

    /// Suggested follow-up questions.
    #[serde(default)]
    pub follow_up_questions: Vec<String>,
}

/// Calibrated confidence with reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibratedConfidence {
    /// The calibrated confidence score (0.0-1.0).
    pub confidence: f64,

    /// Original confidence before calibration.
    pub original_confidence: f64,

    /// Reasoning for the calibration.
    pub reasoning: String,

    /// Domain-specific factors that affected confidence.
    #[serde(default)]
    pub factors: Vec<ConfidenceFactor>,
}

/// A factor that affects confidence calibration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceFactor {
    /// Name of the factor.
    pub name: String,

    /// Impact on confidence (-1.0 to 1.0).
    pub impact: f64,

    /// Explanation of why this factor applies.
    pub explanation: String,
}

/// Configuration for LLM providers.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Model to use (e.g., "claude-sonnet-4-20250514").
    pub model: String,

    /// Maximum tokens in response.
    pub max_tokens: usize,

    /// Temperature for generation (0.0-1.0).
    pub temperature: f64,

    /// Whether to enhance schema columns.
    pub enhance_schema: bool,

    /// Whether to generate observation explanations.
    pub explain_observations: bool,

    /// Whether to generate suggestions.
    pub generate_suggestions: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 1024,
            temperature: 0.3,
            enhance_schema: true,
            explain_observations: true,
            generate_suggestions: true,
        }
    }
}

/// Trait for LLM providers.
///
/// Implementations must be thread-safe (Send + Sync) to allow
/// sharing across analysis operations.
pub trait LlmProvider: Send + Sync {
    /// Enhance a column schema with LLM-generated insights.
    ///
    /// # Arguments
    /// * `column` - The inferred column schema
    /// * `samples` - Sample values from the column
    /// * `context` - User-provided context hints
    ///
    /// # Returns
    /// Enhancement with insight and optional suggestions
    fn enhance_schema(
        &self,
        column: &ColumnSchema,
        samples: &[String],
        context: &ContextHints,
    ) -> Result<SchemaEnhancement>;

    /// Generate a human-readable explanation for an observation.
    ///
    /// # Arguments
    /// * `observation` - The detected issue
    /// * `column` - Schema of the affected column
    /// * `context` - User-provided context hints
    ///
    /// # Returns
    /// Natural language explanation of the issue
    fn explain_observation(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        context: &ContextHints,
    ) -> Result<String>;

    /// Generate a suggestion for fixing an observation.
    ///
    /// # Arguments
    /// * `observation` - The detected issue
    /// * `column` - Schema of the affected column
    /// * `context` - User-provided context hints
    ///
    /// # Returns
    /// Optional suggestion with rationale
    fn generate_suggestion(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        context: &ContextHints,
    ) -> Result<Option<Suggestion>>;

    /// Get the configuration for this provider.
    fn config(&self) -> &LlmConfig;

    /// Get the name of this provider (for logging/debugging).
    fn name(&self) -> &str;

    /// Answer an interactive question about data quality.
    ///
    /// This enables chat-based clarification in the web UI, allowing users
    /// to ask follow-up questions like "Why was this flagged?" or
    /// "What would happen if I change X?".
    ///
    /// # Arguments
    /// * `question_context` - The question and relevant context
    /// * `hints` - User-provided context hints
    ///
    /// # Returns
    /// Response with answer, confidence, and suggested follow-ups
    fn answer_question(
        &self,
        question_context: &QuestionContext,
        hints: &ContextHints,
    ) -> Result<QuestionResponse>;

    /// Calibrate confidence for an observation based on domain context.
    ///
    /// This adjusts confidence scores based on domain-specific knowledge,
    /// such as recognizing that certain patterns are more/less reliable
    /// in specific contexts (e.g., biomedical vs financial data).
    ///
    /// # Arguments
    /// * `observation` - The observation to calibrate
    /// * `column` - The affected column schema
    /// * `hints` - User-provided context hints
    ///
    /// # Returns
    /// Calibrated confidence with reasoning and factors
    fn calibrate_confidence(
        &self,
        observation: &Observation,
        column: Option<&ColumnSchema>,
        hints: &ContextHints,
    ) -> Result<CalibratedConfidence>;
}
