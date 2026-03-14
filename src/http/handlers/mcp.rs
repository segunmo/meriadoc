//! MCP JSON-RPC handler over HTTP.

use axum::{Json, extract::State};
use serde_json::Value;

use crate::mcp::handlers::McpHandlers;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::types::ToolCallParams;

use super::super::state::AppState;

/// Handle MCP JSON-RPC requests over HTTP.
pub async fn handle_mcp(
    State(state): State<AppState>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let result = match request.method.as_str() {
        "initialize" => {
            let init_result = McpHandlers::initialize();
            serde_json::to_value(init_result).unwrap_or(Value::Null)
        }
        "tools/list" => {
            let app = state.app.read();
            let tools = McpHandlers::list_tools(&app);
            serde_json::to_value(tools).unwrap_or(Value::Null)
        }
        "tools/call" => {
            let params: ToolCallParams = serde_json::from_value(request.params.clone())
                .unwrap_or_else(|_| ToolCallParams {
                    name: String::new(),
                    arguments: std::collections::HashMap::new(),
                });
            let mut app = state.app.write();
            let result = McpHandlers::call_tool(&mut app, &params);
            serde_json::to_value(result).unwrap_or(Value::Null)
        }
        _ => serde_json::json!({"error": format!("Unknown method: {}", request.method)}),
    };

    Json(JsonRpcResponse::success(request.id, result))
}
