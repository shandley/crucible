//! Curation layer handlers.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crucible::{DecisionStatus, Observation, Severity, Suggestion, SuggestionAction};

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// Response containing the full curation state.
#[derive(Serialize)]
pub struct CurationResponse {
    pub source: SourceInfo,
    pub schema: SchemaInfo,
    pub observations: Vec<ObservationInfo>,
    pub suggestions: Vec<SuggestionInfo>,
    pub decisions: Vec<DecisionInfo>,
    pub summary: SummaryInfo,
    pub progress: f64,
}

#[derive(Serialize)]
pub struct SourceInfo {
    pub file: String,
    pub format: String,
    pub row_count: usize,
    pub column_count: usize,
}

#[derive(Serialize)]
pub struct SchemaInfo {
    pub columns: Vec<ColumnInfo>,
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub inferred_type: String,
    pub semantic_role: String,
    pub nullable: bool,
    pub unique: bool,
}

#[derive(Serialize)]
pub struct ObservationInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub observation_type: String,
    pub severity: String,
    pub column: String,
    pub description: String,
    pub confidence: f64,
    pub evidence: serde_json::Value,
}

#[derive(Serialize)]
pub struct SuggestionInfo {
    pub id: String,
    pub observation_id: String,
    pub action: String,
    pub priority: u8,
    pub rationale: String,
    pub affected_rows: usize,
    pub confidence: f64,
    pub parameters: serde_json::Value,
}

#[derive(Serialize)]
pub struct DecisionInfo {
    pub id: String,
    pub suggestion_id: String,
    pub status: String,
    pub decided_by: Option<String>,
    pub decided_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct SummaryInfo {
    pub total_columns: usize,
    pub columns_with_issues: usize,
    pub total_observations: usize,
    pub observations_by_severity: ObservationCounts,
    pub data_quality_score: f64,
    pub total_suggestions: usize,
    pub pending_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
}

#[derive(Serialize)]
pub struct ObservationCounts {
    pub error: usize,
    pub warning: usize,
    pub info: usize,
}

/// GET /api/curation - Get the full curation state.
pub async fn get_curation(State(state): State<AppState>) -> Result<Json<CurationResponse>, ApiError> {
    let curation = state.curation.read().await;

    let response = CurationResponse {
        source: SourceInfo {
            file: curation.source.file.clone(),
            format: curation.source.format.clone(),
            row_count: curation.source.row_count,
            column_count: curation.source.column_count,
        },
        schema: SchemaInfo {
            columns: curation
                .schema
                .columns
                .iter()
                .map(|c| ColumnInfo {
                    name: c.name.clone(),
                    inferred_type: format!("{:?}", c.inferred_type),
                    semantic_role: format!("{:?}", c.semantic_role),
                    nullable: c.nullable,
                    unique: c.unique,
                })
                .collect(),
        },
        observations: curation
            .observations
            .iter()
            .map(|o| ObservationInfo {
                id: o.id.clone(),
                observation_type: format!("{:?}", o.observation_type),
                severity: format!("{:?}", o.severity).to_lowercase(),
                column: o.column.clone(),
                description: o.description.clone(),
                confidence: o.confidence,
                evidence: serde_json::to_value(&o.evidence).unwrap_or_default(),
            })
            .collect(),
        suggestions: curation
            .suggestions
            .iter()
            .map(|s| SuggestionInfo {
                id: s.id.clone(),
                observation_id: s.observation_id.clone(),
                action: format!("{:?}", s.action),
                priority: s.priority,
                rationale: s.rationale.clone(),
                affected_rows: s.affected_rows,
                confidence: s.confidence,
                parameters: s.parameters.clone(),
            })
            .collect(),
        decisions: curation
            .decisions
            .iter()
            .map(|d| DecisionInfo {
                id: d.id.clone(),
                suggestion_id: d.suggestion_id.clone(),
                status: format!("{:?}", d.status).to_lowercase(),
                decided_by: d.decided_by.clone(),
                decided_at: d.decided_at.map(|dt| dt.to_rfc3339()),
                notes: d.notes.clone(),
            })
            .collect(),
        summary: SummaryInfo {
            total_columns: curation.summary.total_columns,
            columns_with_issues: curation.summary.columns_with_issues,
            total_observations: curation.summary.total_observations,
            observations_by_severity: ObservationCounts {
                error: curation.summary.observations_by_severity.error,
                warning: curation.summary.observations_by_severity.warning,
                info: curation.summary.observations_by_severity.info,
            },
            data_quality_score: curation.summary.data_quality_score,
            total_suggestions: curation.suggestions.len(),
            pending_count: curation.pending_suggestions().len(),
            accepted_count: curation
                .decisions
                .iter()
                .filter(|d| {
                    d.status == DecisionStatus::Accepted || d.status == DecisionStatus::Modified
                })
                .count(),
            rejected_count: curation
                .decisions
                .iter()
                .filter(|d| d.status == DecisionStatus::Rejected)
                .count(),
        },
        progress: curation.progress(),
    };

    Ok(Json(response))
}

/// POST /api/save - Force save the curation layer.
pub async fn save_curation(State(state): State<AppState>) -> Result<Json<SaveResponse>, ApiError> {
    state.save().await?;
    Ok(Json(SaveResponse {
        success: true,
        path: state.curation_path.display().to_string(),
    }))
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub success: bool,
    pub path: String,
}
