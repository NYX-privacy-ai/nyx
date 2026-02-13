use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PortfolioData {
    pub total_value_usd: f64,
    pub change_24h_pct: f64,
    pub change_24h_usd: f64,
    pub positions: Vec<Position>,
    pub allocation: Vec<Allocation>,
    pub recent_activity: Vec<Activity>,
    pub health: HealthStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Position {
    pub asset: String,
    pub protocol: String,
    pub position_type: String,
    pub amount: f64,
    pub value_usd: f64,
    pub apy: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Allocation {
    pub asset: String,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Activity {
    pub timestamp: String,
    pub action: String,
    pub protocol: String,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HealthStatus {
    pub burrow_health_factor: Option<f64>,
    pub guardrails_active: bool,
    pub daily_loss_pct: f64,
    pub daily_loss_limit_pct: f64,
}

fn defi_state_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".openclaw/defi-state")
}

/// Read current portfolio data from defi-state files.
pub async fn read_portfolio() -> Result<PortfolioData, String> {
    let dir = defi_state_dir();

    // Try to read portfolio.json
    let portfolio_path = dir.join("portfolio.json");
    if portfolio_path.exists() {
        let content = fs::read_to_string(&portfolio_path)
            .map_err(|e| format!("Failed to read portfolio: {}", e))?;
        let data: PortfolioData = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse portfolio: {}", e))?;
        return Ok(data);
    }

    // Return demo data if no portfolio file exists yet
    Ok(demo_portfolio())
}

/// Start file watcher for real-time updates.
pub async fn start_watcher(app: AppHandle) -> Result<(), String> {
    let dir = defi_state_dir();

    // Create dir if it doesn't exist
    fs::create_dir_all(&dir).ok();

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())
        .map_err(|e| format!("Watcher creation failed: {}", e))?;

    watcher
        .watch(&dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Watch failed: {}", e))?;

    // Also poll every 30 seconds as fallback
    let app_clone = app.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Ok(data) = read_portfolio().await {
                let _ = app_clone.emit("portfolio-update", &data);
            }
        }
    });

    // Process file change events
    loop {
        match rx.recv() {
            Ok(Ok(_event)) => {
                // Small debounce
                std::thread::sleep(std::time::Duration::from_millis(200));
                if let Ok(data) = read_portfolio().await {
                    let _ = app.emit("portfolio-update", &data);
                }
            }
            Ok(Err(e)) => {
                eprintln!("Watch error: {}", e);
            }
            Err(_) => break,
        }
    }

    Ok(())
}

fn demo_portfolio() -> PortfolioData {
    PortfolioData {
        total_value_usd: 0.0,
        change_24h_pct: 0.0,
        change_24h_usd: 0.0,
        positions: vec![],
        allocation: vec![],
        recent_activity: vec![],
        health: HealthStatus {
            burrow_health_factor: None,
            guardrails_active: true,
            daily_loss_pct: 0.0,
            daily_loss_limit_pct: 5.0,
        },
    }
}
