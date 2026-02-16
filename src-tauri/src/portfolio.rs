// ---------------------------------------------------------------------------
// Portfolio — Tauri file watcher for real-time updates
// Re-exports types and read function from portfolio_data (shared lib).
// ---------------------------------------------------------------------------

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

// Re-export shared types so existing code (`portfolio::PortfolioData`) still works
pub use nyx_lib::portfolio_data::*;

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
