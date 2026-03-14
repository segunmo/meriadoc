//! HTTP server setup and routing.

use axum::{
    Router,
    routing::{get, post},
};
use tower_http::cors::CorsLayer;

use crate::app::App;
use crate::core::validation::MeriadocError;

use super::handlers::{api, mcp, ui};
use super::state::AppState;

/// Run the HTTP server with web UI and MCP endpoint.
pub async fn run_server(app: App, port: u16) -> Result<(), MeriadocError> {
    let state = AppState::new(app);

    let router = Router::new()
        // MCP endpoint (JSON-RPC over HTTP)
        .route("/mcp", post(mcp::handle_mcp))
        // REST API
        .route("/api/tasks", get(api::list_tasks))
        .route("/api/tasks/:name/info", get(api::task_info))
        .route("/api/tasks/:name/run", post(api::run_task))
        .route("/api/tasks/:name/stream", get(api::run_task_stream))
        .route("/api/projects", get(api::list_projects))
        // Static UI
        .route("/", get(ui::index))
        .route("/static/*path", get(ui::static_file))
        // Shared state
        .with_state(state)
        // CORS for local development
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", port);
    println!("Meriadoc server running at http://localhost:{}", port);
    println!("  - Web UI:       http://localhost:{}/", port);
    println!("  - MCP endpoint: http://localhost:{}/mcp", port);
    println!("  - REST API:     http://localhost:{}/api/", port);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| MeriadocError::Execution(format!("Failed to bind to {}: {}", addr, e)))?;

    axum::serve(listener, router)
        .await
        .map_err(|e| MeriadocError::Execution(format!("Server error: {}", e)))?;

    Ok(())
}
