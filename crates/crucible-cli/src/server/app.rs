//! Axum application setup.

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use super::handlers;
use super::state::AppState;
use crate::web::static_handler;

/// Create the Axum router with all routes.
pub fn create_router(state: AppState) -> Router {
    // CORS configuration for local development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes
    let api_routes = Router::new()
        // Curation layer
        .route("/curation", get(handlers::get_curation))
        .route("/save", post(handlers::save_curation))
        // Data preview
        .route("/data", get(handlers::get_data_preview))
        // Decisions
        .route("/decisions/:id/accept", post(handlers::accept_decision))
        .route("/decisions/:id/reject", post(handlers::reject_decision))
        .route("/decisions/:id/modify", post(handlers::modify_decision))
        .route("/decisions/:id/reset", post(handlers::reset_decision))
        // Batch operations
        .route("/batch/accept", post(handlers::batch_accept))
        .route("/batch/reject", post(handlers::batch_reject))
        // Interactive explanations (LLM-powered)
        .route("/explain/ask", post(handlers::ask_question))
        .route("/explain/calibrate", post(handlers::calibrate_confidence))
        .route(
            "/explain/observation/:id",
            get(handlers::get_observation_explanation),
        );

    Router::new()
        .nest("/api", api_routes)
        .fallback(static_handler)
        .layer(cors)
        .with_state(state)
}

/// Start the web server.
pub async fn run_server(state: AppState, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

    println!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
