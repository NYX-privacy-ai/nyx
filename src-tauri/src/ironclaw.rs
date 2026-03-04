use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// Constants — Nyx runs its own IronClaw instance under ~/.nyx/
// If another IronClaw instance exists (e.g. Atlas on port 3000), Nyx
// auto-detects and uses a different port to avoid collision.
// ---------------------------------------------------------------------------

/// Standard IronClaw gateway port. Used unless another instance occupies it.
pub const DEFAULT_GATEWAY_PORT: u16 = 3000;

/// Fallback port if another IronClaw instance already occupies the default.
pub const FALLBACK_GATEWAY_PORT: u16 = 3001;

/// Nyx config directory — branded to Nyx, not shared with other IronClaw
/// installations (e.g. a pre-existing ~/.ironclaw/ belongs to someone else).
pub fn config_dir() -> PathBuf {
    PathBuf::from(home_dir()).join(".nyx")
}

/// LaunchAgent plist for the Nyx daemon.
const PLIST_NAME: &str = "com.nyx.daemon.plist";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IronClawCheck {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub config_path: Option<String>,
    pub gateway_port: u16,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn ironclaw_bin() -> Option<PathBuf> {
    let home = home_dir();
    let candidates = [
        PathBuf::from(&home).join(".cargo/bin/ironclaw"),
        PathBuf::from("/usr/local/bin/ironclaw"),
        PathBuf::from("/opt/homebrew/bin/ironclaw"),
    ];
    for p in &candidates {
        if p.exists() {
            return Some(p.clone());
        }
    }
    // Try PATH
    Command::new("which")
        .arg("ironclaw")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
}

fn plist_path() -> PathBuf {
    PathBuf::from(home_dir())
        .join("Library/LaunchAgents")
        .join(PLIST_NAME)
}

/// Read the gateway port from Nyx's .env file, falling back to the default.
pub fn gateway_port() -> u16 {
    let env_file = config_dir().join(".env");
    if let Ok(contents) = std::fs::read_to_string(&env_file) {
        for line in contents.lines() {
            if let Some(val) = line.strip_prefix("GATEWAY_PORT=") {
                if let Ok(port) = val.trim().parse::<u16>() {
                    return port;
                }
            }
        }
    }
    DEFAULT_GATEWAY_PORT
}

/// Pick a gateway port during setup, avoiding collisions with any existing
/// IronClaw instance (e.g. Atlas). Called once during first-run setup.
pub fn pick_available_port() -> u16 {
    // If port 3000 is free, use it (most users)
    if !is_port_in_use(DEFAULT_GATEWAY_PORT) {
        return DEFAULT_GATEWAY_PORT;
    }
    // Another IronClaw instance is on 3000 — use fallback
    FALLBACK_GATEWAY_PORT
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        std::time::Duration::from_millis(200),
    )
    .is_ok()
}

/// Base URL for Nyx's IronClaw gateway.
pub fn gateway_url() -> String {
    format!("http://127.0.0.1:{}", gateway_port())
}

// ---------------------------------------------------------------------------
// Status checks
// ---------------------------------------------------------------------------

/// Detailed IronClaw status: binary installed, daemon running, version.
pub async fn check_ironclaw_detailed() -> Result<IronClawCheck, String> {
    let bin = ironclaw_bin();
    let installed = bin.is_some();

    let version = bin.as_ref().and_then(|b| {
        Command::new(b)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    });

    let running = is_daemon_running_sync();

    let config_path = {
        let p = config_dir().join("config.toml");
        if p.exists() {
            Some(p.display().to_string())
        } else {
            None
        }
    };

    Ok(IronClawCheck {
        installed,
        running,
        version,
        config_path,
        gateway_port: gateway_port(),
    })
}

/// Check if the Nyx IronClaw daemon process is running.
pub async fn is_daemon_running() -> Result<bool, String> {
    Ok(is_daemon_running_sync())
}

fn is_daemon_running_sync() -> bool {
    // Match specifically the Nyx daemon (uses --config pointing to ~/.nyx/)
    let nyx_dir = config_dir();
    let pattern = format!("ironclaw run.*{}", nyx_dir.display());
    Command::new("pgrep")
        .args(["-f", &pattern])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Daemon lifecycle
// ---------------------------------------------------------------------------

/// Start the Nyx IronClaw daemon via launchctl.
pub async fn start_daemon() -> Result<(), String> {
    let plist = plist_path();
    if !plist.exists() {
        return Err("LaunchAgent plist not found. Run setup first.".to_string());
    }

    if is_daemon_running_sync() {
        return Ok(());
    }

    let output = Command::new("launchctl")
        .args(["load", "-w", &plist.display().to_string()])
        .output()
        .map_err(|e| format!("Failed to load LaunchAgent: {}", e))?;

    if output.status.success() {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if is_daemon_running_sync() {
            Ok(())
        } else {
            Err("LaunchAgent loaded but daemon did not start. Check logs.".to_string())
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already loaded") || stderr.contains("service already loaded") {
            Ok(())
        } else {
            Err(format!("Failed to start daemon: {}", stderr))
        }
    }
}

/// Stop the Nyx IronClaw daemon via launchctl.
pub async fn stop_daemon() -> Result<(), String> {
    let plist = plist_path();

    let output = Command::new("launchctl")
        .args(["unload", &plist.display().to_string()])
        .output()
        .map_err(|e| format!("Failed to unload LaunchAgent: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Could not find") || stderr.contains("not loaded") {
            Ok(())
        } else {
            Err(format!("Failed to stop daemon: {}", stderr))
        }
    }
}

/// Restart the Nyx IronClaw daemon (stop + start).
pub async fn restart_daemon() -> Result<(), String> {
    stop_daemon().await?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    start_daemon().await
}

/// Get daemon status as a human-readable string.
pub async fn daemon_status() -> Result<String, String> {
    if is_daemon_running_sync() {
        let port = gateway_port();
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        match client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer ping")
            .body("{}")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status == 401 {
                    Ok("running".to_string())
                } else {
                    Ok(format!("running (gateway responded {})", status))
                }
            }
            Err(_) => Ok("running (gateway not responding yet)".to_string()),
        }
    } else {
        Ok("stopped".to_string())
    }
}

/// Install IronClaw via cargo install.
pub async fn install_ironclaw() -> Result<String, String> {
    if let Some(bin) = ironclaw_bin() {
        let version = Command::new(&bin)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        return Ok(format!("IronClaw already installed: {}", version));
    }

    let output = Command::new("cargo")
        .args(["install", "ironclaw"])
        .output()
        .map_err(|e| format!("Failed to install IronClaw: {}", e))?;

    if output.status.success() {
        Ok("IronClaw installed successfully".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Installation failed: {}", stderr))
    }
}
