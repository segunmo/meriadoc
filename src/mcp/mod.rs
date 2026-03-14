//! MCP (Model Context Protocol) server implementation.
//!
//! This module implements an MCP server that exposes Meriadoc tasks as tools
//! that AI agents can discover and execute.

pub mod handlers;
pub mod protocol;
pub mod server;
pub mod types;

pub use server::McpServer;
