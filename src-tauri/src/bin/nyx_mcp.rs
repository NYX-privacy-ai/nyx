// ---------------------------------------------------------------------------
// nyx-mcp — MCP server binary for Claude Code integration
// ---------------------------------------------------------------------------
// This binary implements the Model Context Protocol over stdio.
// Claude Code spawns it and communicates via JSON-RPC 2.0.
//
// Register with: claude mcp add --transport stdio nyx -- /path/to/nyx-mcp
// ---------------------------------------------------------------------------

use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // All logging goes to stderr — stdout is the MCP transport channel
    eprintln!("nyx-mcp v{} starting...", env!("CARGO_PKG_VERSION"));

    let server = nyx_lib::mcp::NyxMcpServer::new();

    let service = server
        .serve(stdio())
        .await
        .map_err(|e| {
            eprintln!("MCP server error: {:?}", e);
            e
        })?;

    eprintln!("nyx-mcp ready — waiting for MCP client");
    service.waiting().await?;

    eprintln!("nyx-mcp shutting down");
    Ok(())
}
