// ---------------------------------------------------------------------------
// nyx_lib — shared modules used by both the Tauri GUI and the MCP server
// ---------------------------------------------------------------------------

pub mod audit;
pub mod config;
pub mod docker;
pub mod gateway;
pub mod ironclaw;
pub mod oneclick;
pub mod wallet;

// Portfolio types + read function (no Tauri dependency).
// The Tauri binary adds the file-watcher on top of these.
pub mod portfolio_data;

// MCP server implementation
pub mod mcp;
