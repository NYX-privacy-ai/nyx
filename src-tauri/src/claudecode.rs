// ---------------------------------------------------------------------------
// Claude Code — detection, MCP registration, and status
// ---------------------------------------------------------------------------
// Follows the clawdtalk.rs pattern: status struct, path detection, process
// management. Detects Claude Code CLI and manages Nyx MCP server registration.
// ---------------------------------------------------------------------------

use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug)]
pub struct ClaudeCodeStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub mcp_registered: bool,
    pub binary_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Known paths where the Claude Code CLI might be installed.
fn known_claude_paths() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    vec![
        PathBuf::from("/usr/local/bin/claude"),
        PathBuf::from(format!("{}/.local/bin/claude", home)),
        PathBuf::from(format!("{}/.claude/local/claude", home)),
        PathBuf::from(format!("{}/.nvm/versions/node/default/bin/claude", home)),
    ]
}

/// Find the Claude Code binary path.
fn find_claude_binary() -> Option<String> {
    // Try `which claude` first
    if let Ok(output) = Command::new("which").arg("claude").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    // Check known paths
    for path in known_claude_paths() {
        if path.exists() {
            return Some(path.to_string_lossy().to_string());
        }
    }

    None
}

/// Get Claude Code version string.
fn get_claude_version(binary_path: &str) -> Option<String> {
    let output = Command::new(binary_path)
        .arg("--version")
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Some(version);
        }
    }

    // Some versions output to stderr
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return Some(stderr);
    }

    None
}

/// Check if "nyx" is registered as an MCP server in Claude Code settings.
fn check_mcp_registered() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();

    // Check ~/.claude/settings.json (newer Claude Code)
    let settings_path = PathBuf::from(&home).join(".claude/settings.json");
    if let Ok(content) = std::fs::read_to_string(&settings_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            // Look for mcpServers.nyx or similar
            if let Some(servers) = json.get("mcpServers") {
                if servers.get("nyx").is_some() {
                    return true;
                }
            }
        }
    }

    // Check ~/.claude.json (older Claude Code)
    let config_path = PathBuf::from(&home).join(".claude.json");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(servers) = json.get("mcpServers") {
                if servers.get("nyx").is_some() {
                    return true;
                }
            }
        }
    }

    false
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check Claude Code installation and MCP registration status.
pub fn check_status() -> Result<ClaudeCodeStatus, String> {
    let binary_path = find_claude_binary();
    let installed = binary_path.is_some();

    let version = binary_path
        .as_ref()
        .and_then(|p| get_claude_version(p));

    let mcp_registered = if installed {
        check_mcp_registered()
    } else {
        false
    };

    Ok(ClaudeCodeStatus {
        installed,
        version,
        mcp_registered,
        binary_path,
    })
}

/// Get the path to the bundled nyx-mcp binary.
/// In development: looks for it in the target directory.
/// In production: looks for it alongside the main app binary.
pub fn get_mcp_binary_path() -> Result<String, String> {
    // Try alongside the current executable first (production)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent().unwrap_or(&exe).join("nyx-mcp");
        if sibling.exists() {
            return Ok(sibling.to_string_lossy().to_string());
        }
    }

    // Development: try target/debug or target/release
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let debug_path = PathBuf::from(manifest_dir).join("target/debug/nyx-mcp");
    if debug_path.exists() {
        return Ok(debug_path.to_string_lossy().to_string());
    }

    let release_path = PathBuf::from(manifest_dir).join("target/release/nyx-mcp");
    if release_path.exists() {
        return Ok(release_path.to_string_lossy().to_string());
    }

    Err("nyx-mcp binary not found. Build it with: cargo build --bin nyx-mcp".to_string())
}

/// Register Nyx as an MCP server with Claude Code.
pub async fn register_mcp_server() -> Result<String, String> {
    let mcp_path = get_mcp_binary_path()?;
    let claude_path = find_claude_binary()
        .ok_or_else(|| "Claude Code CLI not found. Install it first.".to_string())?;

    let output = Command::new(&claude_path)
        .args(["mcp", "add", "--transport", "stdio", "nyx", "--", &mcp_path])
        .output()
        .map_err(|e| format!("Failed to run claude mcp add: {}", e))?;

    if output.status.success() {
        let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(if msg.is_empty() {
            format!("Nyx MCP server registered at {}", mcp_path)
        } else {
            msg
        })
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Registration failed: {}", err))
    }
}

/// Unregister Nyx MCP server from Claude Code.
pub async fn unregister_mcp_server() -> Result<(), String> {
    let claude_path = find_claude_binary()
        .ok_or_else(|| "Claude Code CLI not found.".to_string())?;

    let output = Command::new(&claude_path)
        .args(["mcp", "remove", "nyx"])
        .output()
        .map_err(|e| format!("Failed to run claude mcp remove: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("Unregister failed: {}", err))
    }
}
