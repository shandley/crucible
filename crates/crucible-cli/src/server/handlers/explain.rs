//! Interactive explanation handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crucible::{ContextHints, QuestionContext};

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// Request to ask a question about an observation or suggestion.
#[derive(Debug, Deserialize)]
pub struct AskQuestionRequest {
    /// The question to ask.
    pub question: String,

    /// Optional observation ID to ask about.
    #[serde(default)]
    pub observation_id: Option<String>,

    /// Optional suggestion ID to ask about.
    #[serde(default)]
    pub suggestion_id: Option<String>,
}

/// Response to an interactive question.
#[derive(Debug, Serialize)]
pub struct AskQuestionResponse {
    /// The answer to the question.
    pub answer: String,

    /// Confidence in the answer (0-100).
    pub confidence: f64,

    /// Suggested follow-up questions.
    pub follow_up_questions: Vec<String>,
}

/// Request to calibrate confidence for an observation.
#[derive(Debug, Deserialize)]
pub struct CalibrateConfidenceRequest {
    /// The observation ID to calibrate.
    pub observation_id: String,
}

/// Response with calibrated confidence.
#[derive(Debug, Serialize)]
pub struct CalibrateConfidenceResponse {
    /// The observation ID.
    pub observation_id: String,

    /// Original confidence (0-100).
    pub original_confidence: f64,

    /// Calibrated confidence (0-100).
    pub calibrated_confidence: f64,

    /// Reasoning for the calibration.
    pub reasoning: String,

    /// Factors that affected the calibration.
    pub factors: Vec<ConfidenceFactorInfo>,
}

/// Information about a confidence factor.
#[derive(Debug, Serialize)]
pub struct ConfidenceFactorInfo {
    /// Name of the factor.
    pub name: String,

    /// Impact on confidence (-100 to 100).
    pub impact: f64,

    /// Explanation of why this factor applies.
    pub explanation: String,
}

/// POST /api/explain/ask - Ask an interactive question about data quality.
pub async fn ask_question(
    State(state): State<AppState>,
    Json(request): Json<AskQuestionRequest>,
) -> Result<Json<AskQuestionResponse>, ApiError> {
    // Require LLM provider
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        ApiError::BadRequest(
            "LLM not configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable."
                .to_string(),
        )
    })?;

    let curation = state.curation.read().await;

    // Build question context
    let mut context = QuestionContext::new(&request.question);

    // Add observation if specified
    if let Some(ref obs_id) = request.observation_id {
        if let Some(obs) = curation.observations.iter().find(|o| o.id == *obs_id) {
            context = context.with_observation(obs.clone());

            // Add column schema if available
            if let Some(col) = curation.schema.columns.iter().find(|c| c.name == obs.column) {
                context = context.with_column(col.clone());
            }
        }
    }

    // Add suggestion if specified
    if let Some(ref sug_id) = request.suggestion_id {
        if let Some(sug) = curation.suggestions.iter().find(|s| s.id == *sug_id) {
            context = context.with_suggestion(sug.clone());
        }
    }

    // Get domain from curation context
    let hints = ContextHints::new().with_domain(
        curation
            .context
            .hints
            .domain
            .as_deref()
            .unwrap_or("general"),
    );

    let response = provider
        .answer_question(&context, &hints)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(AskQuestionResponse {
        answer: response.answer,
        confidence: response.confidence * 100.0,
        follow_up_questions: response.follow_up_questions,
    }))
}

/// POST /api/explain/calibrate - Calibrate confidence for an observation.
pub async fn calibrate_confidence(
    State(state): State<AppState>,
    Json(request): Json<CalibrateConfidenceRequest>,
) -> Result<Json<CalibrateConfidenceResponse>, ApiError> {
    // Require LLM provider
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        ApiError::BadRequest(
            "LLM not configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable."
                .to_string(),
        )
    })?;

    let curation = state.curation.read().await;

    // Find the observation
    let observation = curation
        .observations
        .iter()
        .find(|o| o.id == request.observation_id)
        .ok_or_else(|| ApiError::NotFound(format!("Observation {} not found", request.observation_id)))?;

    // Find the column schema
    let column = curation
        .schema
        .columns
        .iter()
        .find(|c| c.name == observation.column);

    // Get domain from curation context
    let hints = ContextHints::new().with_domain(
        curation
            .context
            .hints
            .domain
            .as_deref()
            .unwrap_or("general"),
    );

    let calibration = provider
        .calibrate_confidence(observation, column, &hints)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(CalibrateConfidenceResponse {
        observation_id: request.observation_id,
        original_confidence: calibration.original_confidence * 100.0,
        calibrated_confidence: calibration.confidence * 100.0,
        reasoning: calibration.reasoning,
        factors: calibration
            .factors
            .into_iter()
            .map(|f| ConfidenceFactorInfo {
                name: f.name,
                impact: f.impact * 100.0,
                explanation: f.explanation,
            })
            .collect(),
    }))
}

/// GET /api/explain/observation/:id - Get detailed explanation for an observation.
pub async fn get_observation_explanation(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ObservationExplanation>, ApiError> {
    // Require LLM provider
    let provider = state.llm_provider.as_ref().ok_or_else(|| {
        ApiError::BadRequest(
            "LLM not configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY environment variable."
                .to_string(),
        )
    })?;

    let curation = state.curation.read().await;

    // Find the observation
    let observation = curation
        .observations
        .iter()
        .find(|o| o.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Observation {} not found", id)))?;

    // Find the column schema
    let column = curation
        .schema
        .columns
        .iter()
        .find(|c| c.name == observation.column);

    // Get domain from curation context
    let hints = ContextHints::new().with_domain(
        curation
            .context
            .hints
            .domain
            .as_deref()
            .unwrap_or("general"),
    );

    let explanation = provider
        .explain_observation(observation, column, &hints)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Get calibrated confidence
    let calibration = provider
        .calibrate_confidence(observation, column, &hints)
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    Ok(Json(ObservationExplanation {
        observation_id: id,
        explanation,
        original_confidence: observation.confidence * 100.0,
        calibrated_confidence: calibration.confidence * 100.0,
        calibration_reasoning: calibration.reasoning,
        suggested_questions: vec![
            "Why was this flagged?".to_string(),
            "What would happen if I ignore this?".to_string(),
            "Are there similar issues elsewhere?".to_string(),
        ],
    }))
}

/// Detailed explanation for an observation.
#[derive(Debug, Serialize)]
pub struct ObservationExplanation {
    /// The observation ID.
    pub observation_id: String,

    /// Human-readable explanation.
    pub explanation: String,

    /// Original confidence (0-100).
    pub original_confidence: f64,

    /// Calibrated confidence (0-100).
    pub calibrated_confidence: f64,

    /// Reasoning for confidence calibration.
    pub calibration_reasoning: String,

    /// Suggested questions to ask.
    pub suggested_questions: Vec<String>,
}

/// Response for LLM status check.
#[derive(Debug, Serialize)]
pub struct LlmStatusResponse {
    /// Whether LLM is available.
    pub available: bool,

    /// Name of the configured provider (if any).
    pub provider: Option<String>,

    /// Message for the user.
    pub message: String,
}

/// GET /api/llm/status - Check if LLM is configured and available.
pub async fn get_llm_status(State(state): State<AppState>) -> Json<LlmStatusResponse> {
    if let Some(ref name) = state.llm_provider_name {
        Json(LlmStatusResponse {
            available: true,
            provider: Some(name.clone()),
            message: format!("LLM provider '{}' is configured and ready.", name),
        })
    } else {
        Json(LlmStatusResponse {
            available: false,
            provider: None,
            message: "No LLM configured. Set ANTHROPIC_API_KEY or OPENAI_API_KEY to enable AI-powered explanations.".to_string(),
        })
    }
}
