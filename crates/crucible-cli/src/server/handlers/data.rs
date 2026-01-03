//! Data preview handler.

use axum::{extract::State, Json};
use crucible::Parser;
use serde::Serialize;

use crate::server::error::ApiError;
use crate::server::state::AppState;

/// Response for the data preview endpoint.
#[derive(Serialize)]
pub struct DataPreviewResponse {
    /// Column headers.
    pub headers: Vec<String>,
    /// Data rows (first N rows).
    pub rows: Vec<Vec<String>>,
    /// Total row count in the file.
    pub total_rows: usize,
    /// Whether the data was truncated.
    pub truncated: bool,
}

/// Maximum number of rows to return in preview.
const MAX_PREVIEW_ROWS: usize = 100;

/// Get a preview of the source data.
pub async fn get_data_preview(
    State(state): State<AppState>,
) -> Result<Json<DataPreviewResponse>, ApiError> {
    // Load and parse the source data
    let parser = Parser::new();
    let (data, _metadata) = parser
        .parse_file(&state.data_path)
        .map_err(|e| ApiError::NotFound(format!("Failed to load data: {}", e)))?;

    let total_rows = data.row_count();
    let truncated = total_rows > MAX_PREVIEW_ROWS;

    // Get rows (up to MAX_PREVIEW_ROWS)
    let rows: Vec<Vec<String>> = data
        .rows
        .into_iter()
        .take(MAX_PREVIEW_ROWS)
        .collect();

    Ok(Json(DataPreviewResponse {
        headers: data.headers,
        rows,
        total_rows,
        truncated,
    }))
}
