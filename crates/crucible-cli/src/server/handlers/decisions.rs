//! Decision handlers for accept/reject/modify.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// Request body for accepting a suggestion.
#[derive(Deserialize)]
pub struct AcceptRequest {
    pub user: Option<String>,
    pub notes: Option<String>,
}

/// Request body for rejecting a suggestion.
#[derive(Deserialize)]
pub struct RejectRequest {
    pub notes: String,
    pub user: Option<String>,
}

/// Request body for modifying a suggestion.
#[derive(Deserialize)]
pub struct ModifyRequest {
    pub modifications: serde_json::Value,
    pub notes: String,
    pub user: Option<String>,
}

/// Response after making a decision.
#[derive(Serialize)]
pub struct DecisionResponse {
    pub id: String,
    pub suggestion_id: String,
    pub status: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub notes: Option<String>,
}

/// POST /api/decisions/:id/accept
pub async fn accept_decision(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
    Json(req): Json<AcceptRequest>,
) -> Result<Json<DecisionResponse>, ApiError> {
    let mut curation = state.curation.write().await;

    // Verify suggestion exists
    if curation.suggestion(&suggestion_id).is_none() {
        return Err(ApiError::NotFound(format!(
            "Suggestion not found: {}",
            suggestion_id
        )));
    }

    // Check if already decided
    if curation.decision_for(&suggestion_id).is_some() {
        return Err(ApiError::Conflict(format!(
            "Suggestion already has a decision: {}",
            suggestion_id
        )));
    }

    // Make the decision
    let decision = if let Some(user) = req.user {
        curation.accept_by(&suggestion_id, &user)?
    } else {
        curation.accept(&suggestion_id)?
    };

    let response = DecisionResponse {
        id: decision.id.clone(),
        suggestion_id: decision.suggestion_id.clone(),
        status: format!("{:?}", decision.status).to_lowercase(),
        decided_by: decision.decided_by.clone(),
        decided_at: decision.decided_at.map(|dt| dt.to_rfc3339()),
        notes: decision.notes.clone(),
    };

    // Auto-save if enabled
    if state.auto_save {
        drop(curation); // Release the lock before saving
        state.save().await?;
    }

    Ok(Json(response))
}

/// POST /api/decisions/:id/reject
pub async fn reject_decision(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
    Json(req): Json<RejectRequest>,
) -> Result<Json<DecisionResponse>, ApiError> {
    let mut curation = state.curation.write().await;

    // Verify suggestion exists
    if curation.suggestion(&suggestion_id).is_none() {
        return Err(ApiError::NotFound(format!(
            "Suggestion not found: {}",
            suggestion_id
        )));
    }

    // Check if already decided
    if curation.decision_for(&suggestion_id).is_some() {
        return Err(ApiError::Conflict(format!(
            "Suggestion already has a decision: {}",
            suggestion_id
        )));
    }

    // Make the decision
    let decision = if let Some(user) = req.user {
        curation.reject_by(&suggestion_id, &user, &req.notes)?
    } else {
        curation.reject(&suggestion_id, &req.notes)?
    };

    let response = DecisionResponse {
        id: decision.id.clone(),
        suggestion_id: decision.suggestion_id.clone(),
        status: format!("{:?}", decision.status).to_lowercase(),
        decided_by: decision.decided_by.clone(),
        decided_at: decision.decided_at.map(|dt| dt.to_rfc3339()),
        notes: decision.notes.clone(),
    };

    // Auto-save if enabled
    if state.auto_save {
        drop(curation);
        state.save().await?;
    }

    Ok(Json(response))
}

/// POST /api/decisions/:id/modify
pub async fn modify_decision(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
    Json(req): Json<ModifyRequest>,
) -> Result<Json<DecisionResponse>, ApiError> {
    let mut curation = state.curation.write().await;

    // Verify suggestion exists
    if curation.suggestion(&suggestion_id).is_none() {
        return Err(ApiError::NotFound(format!(
            "Suggestion not found: {}",
            suggestion_id
        )));
    }

    // Check if already decided
    if curation.decision_for(&suggestion_id).is_some() {
        return Err(ApiError::Conflict(format!(
            "Suggestion already has a decision: {}",
            suggestion_id
        )));
    }

    // Make the decision
    let decision = if let Some(user) = req.user {
        curation.modify_by(&suggestion_id, &user, req.modifications, &req.notes)?
    } else {
        curation.modify(&suggestion_id, req.modifications, &req.notes)?
    };

    let response = DecisionResponse {
        id: decision.id.clone(),
        suggestion_id: decision.suggestion_id.clone(),
        status: format!("{:?}", decision.status).to_lowercase(),
        decided_by: decision.decided_by.clone(),
        decided_at: decision.decided_at.map(|dt| dt.to_rfc3339()),
        notes: decision.notes.clone(),
    };

    // Auto-save if enabled
    if state.auto_save {
        drop(curation);
        state.save().await?;
    }

    Ok(Json(response))
}
