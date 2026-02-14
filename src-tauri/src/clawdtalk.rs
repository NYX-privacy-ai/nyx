use crate::config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClawdTalkConfig {
    pub api_key: String,
    pub server: String,
    pub owner_name: Option<String>,
    pub agent_name: Option<String>,
    pub greeting: String,
    pub max_conversation_turns: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct ClawdTalkStatus {
    pub configured: bool,
    pub connected: bool,
    pub has_api_key: bool,
    pub server: String,
    pub pid: Option<u32>,
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// ClawdTalk skill directory
fn skill_dir() -> PathBuf {
    config::home_dir().join("openclaw/local-skills/clawdtalk")
}

fn config_path() -> PathBuf {
    skill_dir().join("skill-config.json")
}

fn pid_file() -> PathBuf {
    skill_dir().join(".connect.pid")
}

fn log_file() -> PathBuf {
    skill_dir().join(".connect.log")
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Check ClawdTalk status: configured, running, has key.
pub fn check_status() -> Result<ClawdTalkStatus, String> {
    let config = config_path();
    let configured = config.exists();

    let mut has_api_key = false;
    let mut server = "https://clawdtalk.com".to_string();

    if configured {
        if let Ok(content) = fs::read_to_string(&config) {
            // Resolve env vars from docker.env
            let resolved = resolve_env_vars(&content);
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&resolved) {
                let key = cfg.get("api_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                has_api_key = !key.is_empty() && key != "YOUR_API_KEY_HERE" && !key.starts_with("${");
                if let Some(s) = cfg.get("server").and_then(|v| v.as_str()) {
                    server = s.to_string();
                }
            }
        }
    }

    // Check if WebSocket client process is running
    let (connected, pid) = check_process_running();

    Ok(ClawdTalkStatus {
        configured,
        connected,
        has_api_key,
        server,
        pid,
    })
}

/// Check if the ws-client process is running via PID file.
fn check_process_running() -> (bool, Option<u32>) {
    let pidfile = pid_file();
    if !pidfile.exists() {
        return (false, None);
    }
    if let Ok(content) = fs::read_to_string(&pidfile) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            // Check if process exists
            let output = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output();
            if let Ok(o) = output {
                if o.status.success() {
                    return (true, Some(pid));
                }
            }
            // Stale PID file — clean up
            let _ = fs::remove_file(&pidfile);
        }
    }
    (false, None)
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Write ClawdTalk skill-config.json with env var reference for API key.
pub fn write_config(api_key_ref: &str, owner_name: Option<&str>, agent_name: Option<&str>) -> Result<(), String> {
    let dir = skill_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create ClawdTalk dir: {}", e))?;

    // Build greeting
    let greeting = match owner_name {
        Some(name) if !name.is_empty() => format!("Hey {}, what's up?", name),
        _ => "Hey, what's up?".to_string(),
    };

    let config = serde_json::json!({
        "api_key": api_key_ref,
        "server": "https://clawdtalk.com",
        "owner_name": owner_name,
        "agent_name": agent_name,
        "greeting": greeting,
        "max_conversation_turns": 20
    });

    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(config_path(), content)
        .map_err(|e| format!("Failed to write config: {}", e))?;

    // chmod 600 on config file (contains env var reference, defence in depth)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(config_path(), fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

/// Remove ClawdTalk configuration and stop if running.
pub fn remove_config() -> Result<(), String> {
    // Stop if running
    let _ = stop_connection();
    // Remove config file
    let config = config_path();
    if config.exists() {
        fs::remove_file(&config)
            .map_err(|e| format!("Failed to remove config: {}", e))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Connection management
// ---------------------------------------------------------------------------

/// Start the WebSocket connection (ws-client.js in background).
pub async fn start_connection() -> Result<ClawdTalkStatus, String> {
    let dir = skill_dir();
    let scripts_dir = dir.join("scripts");
    let ws_client = scripts_dir.join("ws-client.js");

    if !ws_client.exists() {
        return Err("ClawdTalk client files not found. Please reinstall.".to_string());
    }

    // Check not already running
    let (running, _) = check_process_running();
    if running {
        return check_status();
    }

    // Install npm dependencies if needed (ws package)
    let node_modules = dir.join("node_modules/ws");
    if !node_modules.exists() {
        let install = std::process::Command::new("npm")
            .args(["install", "--production"])
            .current_dir(&dir)
            .output();
        if let Ok(o) = install {
            if !o.status.success() {
                return Err("Failed to install ClawdTalk npm dependencies. Ensure npm is available.".to_string());
            }
        } else {
            return Err("npm not found. Install Node.js to use voice calling.".to_string());
        }
    }

    // Source env vars — we need to resolve ${VAR} in skill-config.json
    // The ws-client.js handles this itself via resolve_config, but we need
    // the env vars available in the process environment
    let env_path = config::home_dir().join("openclaw/docker.env");
    let mut env_vars: Vec<(String, String)> = Vec::new();
    if let Ok(content) = fs::read_to_string(&env_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();
                env_vars.push((key, value));
            }
        }
    }

    // Start ws-client.js via node
    let log = log_file();
    let log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|e| format!("Failed to open log file: {}", e))?;

    let log_err = log_handle.try_clone()
        .map_err(|e| format!("Failed to clone log handle: {}", e))?;

    let mut cmd = std::process::Command::new("node");
    cmd.arg(&ws_client)
        .current_dir(&dir)
        .stdout(log_handle)
        .stderr(log_err);

    // Inject env vars
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    let child = cmd.spawn()
        .map_err(|e| format!("Failed to start ClawdTalk: {}", e))?;

    let pid = child.id();
    fs::write(pid_file(), pid.to_string())
        .map_err(|e| format!("Failed to write PID file: {}", e))?;

    // Brief pause to let it connect
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    check_status()
}

/// Stop the WebSocket connection.
pub fn stop_connection() -> Result<ClawdTalkStatus, String> {
    let pidfile = pid_file();
    if let Ok(content) = fs::read_to_string(&pidfile) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            // Graceful kill
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();
            // Wait briefly
            std::thread::sleep(std::time::Duration::from_millis(500));
            // Force kill if still running
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .output();
        }
    }
    let _ = fs::remove_file(&pidfile);
    check_status()
}

/// Get recent log lines.
pub fn get_logs(lines: usize) -> Result<Vec<String>, String> {
    let log = log_file();
    if !log.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&log)
        .map_err(|e| format!("Failed to read log: {}", e))?;
    let all_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let start = if all_lines.len() > lines { all_lines.len() - lines } else { 0 };
    Ok(all_lines[start..].to_vec())
}

// ---------------------------------------------------------------------------
// Voice agent config for OpenClaw gateway
// ---------------------------------------------------------------------------

/// Add voice agent to openclaw.json if not already present.
/// Also enables chatCompletions endpoint.
pub fn configure_gateway_voice_agent() -> Result<(), String> {
    let home = config::home_dir();
    let config_path = home.join(".openclaw/openclaw.json");

    if !config_path.exists() {
        return Err("openclaw.json not found — run setup first".to_string());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read openclaw.json: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse openclaw.json: {}", e))?;

    // Check if voice agent already exists
    let has_voice = config
        .pointer("/agents/list")
        .and_then(|list| list.as_array())
        .map_or(false, |list| {
            list.iter().any(|a| a.get("id").and_then(|v| v.as_str()) == Some("voice"))
        });

    if !has_voice {
        // Get main agent name for voice agent naming
        let main_name = config
            .pointer("/agents/list/0/identity/name")
            .and_then(|v| v.as_str())
            .unwrap_or("Nyx");
        let voice_name = format!("{} Voice", main_name);

        // Get workspace from main agent
        let workspace = config
            .pointer("/agents/list/0/workspace")
            .and_then(|v| v.as_str())
            .unwrap_or("/home/node/.openclaw/workspace");

        let voice_agent = serde_json::json!({
            "id": "voice",
            "name": voice_name,
            "workspace": workspace
        });

        // Add to agents.list
        if let Some(list) = config.pointer_mut("/agents/list").and_then(|v| v.as_array_mut()) {
            list.push(voice_agent);
        }
    }

    // Enable chatCompletions endpoint
    // Ensure gateway.http.endpoints.chatCompletions.enabled = true
    if config.pointer("/gateway/http").is_none() {
        if let Some(gw) = config.pointer_mut("/gateway") {
            if let Some(obj) = gw.as_object_mut() {
                obj.insert("http".to_string(), serde_json::json!({
                    "endpoints": {
                        "chatCompletions": { "enabled": true }
                    }
                }));
            }
        }
    } else {
        // Navigate/create the path
        let gw = config.pointer_mut("/gateway").unwrap();
        let http = gw.as_object_mut().unwrap()
            .entry("http").or_insert_with(|| serde_json::json!({}));
        let endpoints = http.as_object_mut()
            .ok_or("Invalid gateway.http")?
            .entry("endpoints").or_insert_with(|| serde_json::json!({}));
        let chat = endpoints.as_object_mut()
            .ok_or("Invalid endpoints")?
            .entry("chatCompletions").or_insert_with(|| serde_json::json!({}));
        if let Some(obj) = chat.as_object_mut() {
            obj.insert("enabled".to_string(), serde_json::json!(true));
        }
    }

    // Write back
    let updated = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&config_path, updated)
        .map_err(|e| format!("Failed to write openclaw.json: {}", e))?;

    Ok(())
}

/// Remove voice agent from openclaw.json.
pub fn remove_gateway_voice_agent() -> Result<(), String> {
    let home = config::home_dir();
    let config_path = home.join(".openclaw/openclaw.json");

    if !config_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse: {}", e))?;

    // Remove voice agent from list
    if let Some(list) = config.pointer_mut("/agents/list").and_then(|v| v.as_array_mut()) {
        list.retain(|a| a.get("id").and_then(|v| v.as_str()) != Some("voice"));
    }

    let updated = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize: {}", e))?;
    fs::write(&config_path, updated)
        .map_err(|e| format!("Failed to write: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve ${VAR} references in a string using docker.env values.
fn resolve_env_vars(content: &str) -> String {
    let env_path = config::home_dir().join("openclaw/docker.env");
    let mut resolved = content.to_string();

    if let Ok(env_content) = fs::read_to_string(&env_path) {
        for line in env_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim();
                let value = trimmed[pos + 1..].trim();
                resolved = resolved.replace(&format!("${{{}}}", key), value);
            }
        }
    }

    resolved
}
