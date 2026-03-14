//! MCP server main loop.

use std::io::{self, BufRead, Write};

use crate::app::App;
use crate::core::validation::MeriadocError;
use crate::mcp::handlers::McpHandlers;
use crate::mcp::protocol::*;
use crate::mcp::types::ToolCallParams;

/// MCP server that communicates over stdio
pub struct McpServer {
    app: App,
}

impl McpServer {
    /// Create a new MCP server with the given app state
    pub fn new(app: App) -> Self {
        Self { app }
    }

    /// Run the MCP server (blocking, reads from stdin)
    pub fn run(&mut self) -> Result<(), MeriadocError> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        // Print startup message to stderr (doesn't interfere with JSON-RPC on stdout)
        let task_count: usize = self
            .app
            .projects
            .iter()
            .flat_map(|p| &p.specs)
            .map(|s| s.spec.tasks.len())
            .sum();
        eprintln!(
            "Meriadoc MCP server v{} ready ({} tasks available)",
            env!("CARGO_PKG_VERSION"),
            task_count
        );
        eprintln!("Listening for JSON-RPC on stdin...");

        for line in stdin.lock().lines() {
            let line = line?;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse and handle request
            let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
                Ok(request) => self.handle_request(request),
                Err(e) => JsonRpcResponse::error(None, PARSE_ERROR, format!("Parse error: {}", e)),
            };

            // Write response (skip for notifications without id)
            if response.id.is_some() || response.error.is_some() {
                let output = serde_json::to_string(&response).unwrap_or_else(|_| {
                    r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Serialization error"}}"#
                        .to_string()
                });
                writeln!(stdout, "{}", output)?;
                stdout.flush()?;
            }
        }

        Ok(())
    }

    /// Handle a single JSON-RPC request
    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => {
                let result = McpHandlers::initialize();
                match serde_json::to_value(result) {
                    Ok(value) => JsonRpcResponse::success(request.id, value),
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        INTERNAL_ERROR,
                        format!("Serialization error: {}", e),
                    ),
                }
            }

            "notifications/initialized" => {
                // Client acknowledged initialization, no response needed
                JsonRpcResponse::empty()
            }

            "tools/list" => {
                let result = McpHandlers::list_tools(&self.app);
                match serde_json::to_value(result) {
                    Ok(value) => JsonRpcResponse::success(request.id, value),
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        INTERNAL_ERROR,
                        format!("Serialization error: {}", e),
                    ),
                }
            }

            "tools/call" => {
                match serde_json::from_value::<ToolCallParams>(request.params.clone()) {
                    Ok(params) => {
                        let result = McpHandlers::call_tool(&mut self.app, &params);
                        match serde_json::to_value(result) {
                            Ok(value) => JsonRpcResponse::success(request.id, value),
                            Err(e) => JsonRpcResponse::error(
                                request.id,
                                INTERNAL_ERROR,
                                format!("Serialization error: {}", e),
                            ),
                        }
                    }
                    Err(e) => JsonRpcResponse::error(
                        request.id,
                        INVALID_PARAMS,
                        format!("Invalid params: {}", e),
                    ),
                }
            }

            _ => JsonRpcResponse::error(
                request.id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
            ),
        }
    }
}
