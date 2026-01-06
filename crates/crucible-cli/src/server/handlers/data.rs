//! Data preview handler.

use axum::{
    extract::{Query, State},
    Json,
};
use crucible::Parser;
use serde::{Deserialize, Serialize};

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// Query parameters for the data preview endpoint.
#[derive(Deserialize, Default)]
pub struct DataPreviewQuery {
    /// Number of rows to skip (default: 0).
    #[serde(default)]
    pub offset: usize,
    /// Maximum number of rows to return (default: 100, max: 500).
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

/// Response for the data preview endpoint.
#[derive(Serialize)]
pub struct DataPreviewResponse {
    /// Column headers.
    pub headers: Vec<String>,
    /// Data rows for the requested page.
    pub rows: Vec<Vec<String>>,
    /// Total row count in the file.
    pub total_rows: usize,
    /// Current offset (0-based).
    pub offset: usize,
    /// Current limit.
    pub limit: usize,
    /// Whether there are more rows after this page.
    pub has_more: bool,
    /// Whether the data was truncated (legacy field for backwards compatibility).
    pub truncated: bool,
}

/// Maximum allowed limit to prevent abuse.
const MAX_LIMIT: usize = 500;

/// Get a preview of the source data with pagination.
///
/// Query parameters:
/// - `offset`: Number of rows to skip (default: 0)
/// - `limit`: Maximum rows to return (default: 100, max: 500)
pub async fn get_data_preview(
    State(state): State<AppState>,
    Query(params): Query<DataPreviewQuery>,
) -> Result<Json<DataPreviewResponse>, ApiError> {
    // Clamp limit to MAX_LIMIT
    let limit = params.limit.min(MAX_LIMIT);
    let offset = params.offset;

    // Load and parse the source data
    let parser = Parser::new();
    let (data, _metadata) = parser
        .parse_file(&state.data_path)
        .map_err(|e| ApiError::NotFound(format!("Failed to load data: {}", e)))?;

    let total_rows = data.row_count();

    // Apply pagination: skip `offset` rows, take `limit` rows
    let rows: Vec<Vec<String>> = data
        .rows
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    let rows_returned = rows.len();
    let has_more = offset + rows_returned < total_rows;

    Ok(Json(DataPreviewResponse {
        headers: data.headers,
        rows,
        total_rows,
        offset,
        limit,
        has_more,
        truncated: has_more, // Legacy field
    }))
}
