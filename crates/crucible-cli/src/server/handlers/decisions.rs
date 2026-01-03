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
    #[allow(dead_code)]
    pub notes: Option<String>,
}

/// Request body for batch operations.
#[derive(Deserialize)]
pub struct BatchRequest {
    /// Filter by action type (e.g., "standardize", "flag")
    pub action_type: Option<String>,
    /// Filter by column name
    pub column: Option<String>,
    /// Process all pending suggestions
    pub all: Option<bool>,
    /// User name for the decisions
    pub user: Option<String>,
    /// Notes for rejected suggestions
    pub notes: Option<String>,
}

/// Response for batch operations.
#[derive(Serialize)]
pub struct BatchResponse {
    pub processed: usize,
    pub remaining: usize,
    pub decisions: Vec<DecisionResponse>,
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

/// Response after resetting a decision.
#[derive(Serialize)]
pub struct ResetResponse {
    pub suggestion_id: String,
    pub was_reset: bool,
    pub previous_status: Option<String>,
}

/// POST /api/decisions/:id/reset
pub async fn reset_decision(
    State(state): State<AppState>,
    Path(suggestion_id): Path<String>,
) -> Result<Json<ResetResponse>, ApiError> {
    let mut curation = state.curation.write().await;

    // Verify suggestion exists
    if curation.suggestion(&suggestion_id).is_none() {
        return Err(ApiError::NotFound(format!(
            "Suggestion not found: {}",
            suggestion_id
        )));
    }

    // Reset the decision
    let removed = curation.reset(&suggestion_id)?;

    let response = ResetResponse {
        suggestion_id: suggestion_id.clone(),
        was_reset: removed.is_some(),
        previous_status: removed.map(|d| format!("{:?}", d.status).to_lowercase()),
    };

    // Auto-save if enabled
    if state.auto_save && response.was_reset {
        drop(curation);
        state.save().await?;
    }

    Ok(Json(response))
}

/// POST /api/batch/accept
pub async fn batch_accept(
    State(state): State<AppState>,
    Json(req): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, ApiError> {
    batch_operation(state, req, true).await
}

/// POST /api/batch/reject
pub async fn batch_reject(
    State(state): State<AppState>,
    Json(req): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, ApiError> {
    batch_operation(state, req, false).await
}

/// Internal batch operation handler.
async fn batch_operation(
    state: AppState,
    req: BatchRequest,
    accept: bool,
) -> Result<Json<BatchResponse>, ApiError> {
    let mut curation = state.curation.write().await;

    // Find matching pending suggestions
    let matching: Vec<String> = curation
        .suggestions
        .iter()
        .filter(|s| {
            // Check if already decided
            let already_decided = curation
                .decisions
                .iter()
                .any(|d| d.suggestion_id == s.id);
            if already_decided {
                return false;
            }

            // Must specify --all or a filter
            if req.all != Some(true) && req.action_type.is_none() && req.column.is_none() {
                return false;
            }

            // Filter by action type
            if let Some(ref action_filter) = req.action_type {
                let action_str = s.action.label().to_lowercase();
                let filter_lower = action_filter.to_lowercase();
                // Check various matching strategies
                let snake = match s.action {
                    crucible::SuggestionAction::Standardize => "standardize",
                    crucible::SuggestionAction::ConvertNa => "convert_na",
                    crucible::SuggestionAction::Coerce => "coerce",
                    crucible::SuggestionAction::ConvertDate => "convert_date",
                    crucible::SuggestionAction::Flag => "flag",
                    crucible::SuggestionAction::Remove => "remove",
                    crucible::SuggestionAction::Merge => "merge",
                    crucible::SuggestionAction::Rename => "rename",
                    crucible::SuggestionAction::Split => "split",
                    crucible::SuggestionAction::Derive => "derive",
                };
                if !action_str.contains(&filter_lower)
                    && !filter_lower.contains(&action_str)
                    && !filter_lower.contains(snake)
                    && snake != filter_lower
                {
                    return false;
                }
            }

            // Filter by column
            if let Some(ref col_filter) = req.column {
                let suggestion_col = s
                    .parameters
                    .get("column")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !suggestion_col.eq_ignore_ascii_case(col_filter) {
                    return false;
                }
            }

            true
        })
        .map(|s| s.id.clone())
        .collect();

    if matching.is_empty() {
        return Ok(Json(BatchResponse {
            processed: 0,
            remaining: curation
                .suggestions
                .iter()
                .filter(|s| !curation.decisions.iter().any(|d| d.suggestion_id == s.id))
                .count(),
            decisions: vec![],
        }));
    }

    // Apply decisions
    let user = req.user.as_deref().unwrap_or("batch");
    let notes = req.notes.as_deref().unwrap_or("Batch operation");
    let mut decisions = Vec::new();

    for suggestion_id in &matching {
        let decision = if accept {
            curation.accept_by(suggestion_id, user)?
        } else {
            curation.reject_by(suggestion_id, user, notes)?
        };

        decisions.push(DecisionResponse {
            id: decision.id.clone(),
            suggestion_id: decision.suggestion_id.clone(),
            status: format!("{:?}", decision.status).to_lowercase(),
            decided_by: decision.decided_by.clone(),
            decided_at: decision.decided_at.map(|dt| dt.to_rfc3339()),
            notes: decision.notes.clone(),
        });
    }

    let remaining = curation
        .suggestions
        .iter()
        .filter(|s| !curation.decisions.iter().any(|d| d.suggestion_id == s.id))
        .count();

    // Auto-save if enabled
    if state.auto_save {
        drop(curation);
        state.save().await?;
    }

    Ok(Json(BatchResponse {
        processed: decisions.len(),
        remaining,
        decisions,
    }))
}
