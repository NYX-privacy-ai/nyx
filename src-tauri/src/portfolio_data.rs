// ---------------------------------------------------------------------------
// Portfolio data types and read function (shared, no Tauri dependency)
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

pub fn defi_state_dir() -> PathBuf {
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

    // Return empty data if no portfolio file exists yet
    Ok(demo_portfolio())
}

pub fn demo_portfolio() -> PortfolioData {
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
