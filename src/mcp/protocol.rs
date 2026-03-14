//! JSON-RPC 2.0 protocol types for MCP communication.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC request
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)] // Required by JSON-RPC spec, validated during deserialization
    pub jsonrpc: String,
    pub id: Option<Value>, // None for notifications
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

/// JSON-RPC response
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }

    /// Create an empty response (for notifications)
    pub fn empty() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            result: None,
            error: None,
        }
    }
}

/// JSON-RPC error object
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

// Standard JSON-RPC error codes
pub const PARSE_ERROR: i32 = -32700;
#[allow(dead_code)] // Standard error code, available for future use
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
