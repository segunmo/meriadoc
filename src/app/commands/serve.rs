//! Serve command handler - starts the MCP server.

use crate::app::App;
use crate::core::validation::MeriadocError;
use crate::mcp::McpServer;

/// Start the MCP server
pub fn handle_serve(app: App) -> Result<(), MeriadocError> {
    let mut server = McpServer::new(app);
    server.run()
}
