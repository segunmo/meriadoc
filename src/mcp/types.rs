//! MCP-specific types for tools and capabilities.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Server capabilities returned during initialization
#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

/// Tools capability configuration
#[derive(Debug, Serialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Server info returned during initialization
#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Initialize result sent to client
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

/// Tool definition exposed to clients
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub description: String,
    pub input_schema: Value,
}

/// Result of tools/list request
#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

/// Parameters for tools/call request
#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: HashMap<String, Value>,
}

/// Text content in tool result
#[derive(Debug, Serialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl TextContent {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: text.into(),
        }
    }
}

/// Result of tools/call request
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    pub content: Vec<TextContent>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl ToolCallResult {
    /// Create a successful result with text content
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![TextContent::new(text)],
            is_error: false,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![TextContent::new(message)],
            is_error: true,
        }
    }
}
