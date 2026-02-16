// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clawdtalk;
mod config;
mod docker;
mod gateway;
mod google;
mod ollama;
mod oneclick;
mod portfolio;
mod setup;
mod wallet;

// ---------------------------------------------------------------------------
// Docker commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_docker() -> Result<bool, String> {
    docker::is_docker_running().await
}

#[tauri::command]
async fn check_docker_detailed() -> Result<docker::DockerCheck, String> {
    docker::check_docker_detailed().await
}

#[tauri::command]
async fn install_docker() -> Result<String, String> {
    docker::install_docker().await
}

/// Pre-pull the OpenClaw Docker image in the background.
#[tauri::command]
async fn docker_prepull() -> Result<(), String> {
    docker::pull_image("ghcr.io/openclaw/openclaw:2026.2.9").await
}

// ---------------------------------------------------------------------------
// Setup commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_setup_complete() -> Result<bool, String> {
    setup::is_setup_complete().await
}

#[tauri::command]
async fn run_setup(
    app_handle: tauri::AppHandle,
    anthropic_key: String,
    openai_key: Option<String>,
    telegram_token: Option<String>,
) -> Result<String, String> {
    setup::run_setup(app_handle, anthropic_key, openai_key, telegram_token).await
}

/// Extended setup command that accepts the full v2 configuration.
#[tauri::command]
async fn run_setup_v2(
    app_handle: tauri::AppHandle,
    agent_name: Option<String>,
    anthropic_key: String,
    openai_key: Option<String>,
    venice_key: Option<String>,
    nearai_key: Option<String>,
    telegram_token: Option<String>,
    slack_token: Option<String>,
    whatsapp_phone: Option<String>,
    wallets: Vec<config::WalletConfig>,
    active_wallet_id: Option<String>,
    guardrails_preset: String,
    guardrails_custom: Option<config::GuardrailsConfig>,
    messaging: config::MessagingConfig,
    google_authenticated: bool,
    email_notifications: Option<config::EmailNotificationsConfig>,
    capabilities: Option<config::CapabilitiesConfig>,
) -> Result<String, String> {
    // Resolve guardrails from preset name or custom config
    let guardrails = match guardrails_custom {
        Some(custom) => custom,
        None => {
            let preset = match guardrails_preset.as_str() {
                "conservative" => config::SecurityPreset::Conservative,
                "autonomous" => config::SecurityPreset::Autonomous,
                _ => config::SecurityPreset::Balanced,
            };
            config::GuardrailsConfig::from_preset(preset)
        }
    };

    let email_config = email_notifications.unwrap_or_default();
    let caps = capabilities.unwrap_or_default();
    let name = agent_name.unwrap_or_else(|| "Nyx".to_string());

    setup::run_setup_v2(
        app_handle,
        name,
        anthropic_key,
        openai_key,
        venice_key,
        nearai_key,
        telegram_token,
        slack_token,
        whatsapp_phone,
        wallets,
        active_wallet_id,
        guardrails,
        messaging,
        google_authenticated,
        email_config,
        caps,
    )
    .await
}

// ---------------------------------------------------------------------------
// Wallet commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn generate_wallet() -> Result<wallet::WalletInfo, String> {
    let (info, _config) = wallet::generate_near_wallet().await?;
    Ok(info)
}

/// Generate a NEAR wallet and return both the info and config.
#[tauri::command]
async fn generate_near_wallet_full() -> Result<(wallet::WalletInfo, config::WalletConfig), String> {
    wallet::generate_near_wallet().await
}

/// Validate a wallet address for a given chain.
#[tauri::command]
fn validate_wallet_address(chain: config::Chain, address: String) -> Result<(), String> {
    wallet::validate_address(&chain, &address)
}

/// Import a wallet (address only, no private key).
#[tauri::command]
fn import_wallet(
    chain: config::Chain,
    address: String,
    label: String,
) -> Result<config::WalletConfig, String> {
    wallet::import_wallet(chain, address, label)
}

// ---------------------------------------------------------------------------
// Security preset commands
// ---------------------------------------------------------------------------

/// Get guardrails values for a named preset.
#[tauri::command]
fn get_guardrails_preset(preset: String) -> Result<config::GuardrailsConfig, String> {
    let p = match preset.as_str() {
        "conservative" => config::SecurityPreset::Conservative,
        "balanced" => config::SecurityPreset::Balanced,
        "autonomous" => config::SecurityPreset::Autonomous,
        _ => return Err(format!("Unknown preset: {}", preset)),
    };
    Ok(config::GuardrailsConfig::from_preset(p))
}

// ---------------------------------------------------------------------------
// Google Workspace commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_gog_available() -> Result<google::GogStatus, String> {
    google::check_gog_available().await
}

#[tauri::command]
async fn run_gog_auth() -> Result<bool, String> {
    google::run_gog_auth().await
}

#[tauri::command]
async fn check_gog_authenticated() -> Result<bool, String> {
    google::check_gog_authenticated().await
}

#[tauri::command]
async fn install_gog(app_handle: tauri::AppHandle) -> Result<String, String> {
    google::install_gog(&app_handle).await
}

// ---------------------------------------------------------------------------
// Portfolio & Chat
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_portfolio() -> Result<portfolio::PortfolioData, String> {
    portfolio::read_portfolio().await
}

#[tauri::command]
async fn send_chat_message(message: String) -> Result<String, String> {
    gateway::send_message(message).await
}

#[tauri::command]
async fn send_chat_message_to_session(message: String, session_key: String) -> Result<String, String> {
    gateway::send_message_to_session(message, session_key).await
}

// ---------------------------------------------------------------------------
// Session & folder management
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_chat_sessions() -> Result<Vec<gateway::SessionInfo>, String> {
    gateway::list_sessions()
}

#[tauri::command]
fn create_chat_session(title: Option<String>, folder: Option<String>) -> Result<String, String> {
    gateway::create_session(title, folder)
}

#[tauri::command]
fn rename_chat_session(session_key: String, title: String) -> Result<(), String> {
    gateway::rename_session(session_key, title)
}

#[tauri::command]
fn move_session_to_folder(session_key: String, folder_id: Option<String>) -> Result<(), String> {
    gateway::move_session_to_folder(session_key, folder_id)
}

#[tauri::command]
fn get_chat_folders() -> Result<gateway::ChatFolders, String> {
    gateway::get_chat_folders()
}

#[tauri::command]
fn create_chat_folder(name: String) -> Result<gateway::ChatFolder, String> {
    gateway::create_folder(name)
}

#[tauri::command]
fn rename_chat_folder(folder_id: String, name: String) -> Result<(), String> {
    gateway::rename_folder(folder_id, name)
}

#[tauri::command]
fn delete_chat_folder(folder_id: String) -> Result<(), String> {
    gateway::delete_folder(folder_id)
}

#[tauri::command]
async fn verify_source(url: String) -> Result<String, String> {
    gateway::verify_source(url).await
}

// ---------------------------------------------------------------------------
// 1Click API (cross-chain)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_supported_tokens() -> Result<Vec<oneclick::TokenInfo>, String> {
    oneclick::get_tokens().await
}

#[tauri::command]
async fn get_cross_chain_quote(
    asset_in: String,
    asset_out: String,
    amount_in: String,
    recipient: String,
    refund_to: String,
    dry_run: Option<bool>,
) -> Result<oneclick::QuoteResponse, String> {
    oneclick::get_quote(
        &asset_in,
        &asset_out,
        &amount_in,
        &recipient,
        &refund_to,
        dry_run.unwrap_or(true),
    )
    .await
}

#[tauri::command]
async fn get_swap_status(swap_id: String) -> Result<oneclick::SwapStatus, String> {
    oneclick::get_status(&swap_id).await
}

#[tauri::command]
fn resolve_asset_id(chain: String, symbol: String) -> Result<String, String> {
    oneclick::resolve_asset_id(&chain, &symbol)
}

/// Get a quote to shield assets into ZEC (any supported asset → ZEC).
#[tauri::command]
async fn get_zec_shield_quote(
    from_asset: String,
    amount: String,
) -> Result<oneclick::QuoteResponse, String> {
    let zec_address = config::get_zec_address()
        .ok_or_else(|| "No ZEC address configured. Add a ZEC wallet in Settings.".to_string())?;
    let refund_to = config::get_near_account()
        .unwrap_or_else(|| "nyx.near".to_string());
    oneclick::get_zec_quote(&from_asset, &amount, &zec_address, &refund_to).await
}

/// Get a quote to unshield from ZEC to any asset (ZEC → any supported asset).
#[tauri::command]
async fn get_zec_unshield_quote(
    to_asset: String,
    zec_amount: String,
    recipient: String,
) -> Result<oneclick::QuoteResponse, String> {
    let zec_refund = config::get_zec_address()
        .ok_or_else(|| "No ZEC address configured. Add a ZEC wallet in Settings.".to_string())?;
    oneclick::get_quote_from_zec(&to_asset, &zec_amount, &recipient, &zec_refund).await
}

/// Get the list of assets that can be shielded to ZEC.
#[tauri::command]
fn get_shieldable_assets() -> Vec<oneclick::ShieldableAsset> {
    oneclick::get_shieldable_assets()
}

// ---------------------------------------------------------------------------
// Container lifecycle
// ---------------------------------------------------------------------------

#[tauri::command]
async fn docker_start() -> Result<(), String> {
    docker::start_container().await
}

#[tauri::command]
async fn docker_stop() -> Result<(), String> {
    docker::stop_container().await
}

#[tauri::command]
async fn docker_status() -> Result<String, String> {
    docker::container_status().await
}

// ---------------------------------------------------------------------------
// Ollama (local models)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_ollama() -> Result<ollama::OllamaStatus, String> {
    ollama::check_ollama().await
}

#[tauri::command]
async fn install_ollama() -> Result<String, String> {
    ollama::install_ollama().await
}

#[tauri::command]
async fn list_ollama_models() -> Result<Vec<ollama::OllamaModel>, String> {
    ollama::list_models().await
}

#[tauri::command]
async fn pull_ollama_model(model: String) -> Result<String, String> {
    ollama::pull_model(model).await
}

#[tauri::command]
async fn delete_ollama_model(model: String) -> Result<String, String> {
    ollama::delete_model(model).await
}

#[tauri::command]
async fn chat_ollama(
    model: String,
    message: String,
    history: Vec<ollama::ChatMessage>,
) -> Result<String, String> {
    ollama::chat_ollama(model, message, history).await
}

#[tauri::command]
async fn get_system_ram() -> Result<u64, String> {
    ollama::get_system_ram().await
}

// ---------------------------------------------------------------------------
// Agent identity
// ---------------------------------------------------------------------------

/// Read the configured agent name from openclaw.json (fallback: "Nyx").
#[tauri::command]
fn get_agent_name() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let config_path = std::path::PathBuf::from(&home).join(".openclaw/openclaw.json");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Ok("Nyx".to_string()),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return Ok("Nyx".to_string()),
    };
    let name = json
        .pointer("/agents/list/0/identity/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Nyx")
        .to_string();
    Ok(name)
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

#[tauri::command]
fn read_current_config() -> Result<config::SettingsConfig, String> {
    config::read_current_config()
}

#[tauri::command]
fn save_settings(update: config::SettingsUpdate) -> Result<config::SettingsSaveResult, String> {
    config::save_settings(update)
}

#[tauri::command]
async fn restart_container() -> Result<(), String> {
    docker::restart_container().await
}

// ---------------------------------------------------------------------------
// ClawdTalk (voice calling)
// ---------------------------------------------------------------------------

#[tauri::command]
fn clawdtalk_status() -> Result<clawdtalk::ClawdTalkStatus, String> {
    clawdtalk::check_status()
}

#[tauri::command]
fn clawdtalk_configure(api_key: String) -> Result<(), String> {
    // Store the raw API key in docker.env, reference via env var in skill config
    let home = config::home_dir();
    let env_path = home.join("openclaw/docker.env");

    // Read existing docker.env
    let content = std::fs::read_to_string(&env_path).unwrap_or_default();

    // Check if CLAWDTALK_API_KEY already exists
    let has_key = content.lines().any(|l| l.trim().starts_with("CLAWDTALK_API_KEY="));

    let updated = if has_key {
        // Replace existing line
        content.lines()
            .map(|l| {
                if l.trim().starts_with("CLAWDTALK_API_KEY=") {
                    format!("CLAWDTALK_API_KEY={}", api_key)
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // Append to end
        format!("{}\n# ClawdTalk Voice\nCLAWDTALK_API_KEY={}\n", content.trim_end(), api_key)
    };

    std::fs::write(&env_path, updated)
        .map_err(|e| format!("Failed to update docker.env: {}", e))?;

    // chmod 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&env_path, std::fs::Permissions::from_mode(0o600));
    }

    // Get agent name for config
    let agent_name = get_agent_name().ok();

    // Write skill-config.json with actual API key (shell scripts use jq to
    // read this file and cannot resolve ${ENV_VAR} references)
    clawdtalk::write_config(
        &api_key,
        None, // Owner name auto-detected at runtime
        agent_name.as_deref(),
    )?;

    // Add voice agent to gateway config
    clawdtalk::configure_gateway_voice_agent()?;

    Ok(())
}

#[tauri::command]
fn clawdtalk_remove() -> Result<(), String> {
    clawdtalk::remove_config()?;
    clawdtalk::remove_gateway_voice_agent()?;

    // Remove key from docker.env
    let home = config::home_dir();
    let env_path = home.join("openclaw/docker.env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        let updated: Vec<&str> = content.lines()
            .filter(|l| !l.trim().starts_with("CLAWDTALK_API_KEY=") && l.trim() != "# ClawdTalk Voice")
            .collect();
        let _ = std::fs::write(&env_path, updated.join("\n") + "\n");
    }

    Ok(())
}

#[tauri::command]
async fn clawdtalk_start() -> Result<clawdtalk::ClawdTalkStatus, String> {
    clawdtalk::start_connection().await
}

#[tauri::command]
fn clawdtalk_stop() -> Result<clawdtalk::ClawdTalkStatus, String> {
    clawdtalk::stop_connection()
}

#[tauri::command]
fn clawdtalk_logs() -> Result<Vec<String>, String> {
    clawdtalk::get_logs(20)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            // Docker
            check_docker,
            check_docker_detailed,
            install_docker,
            docker_prepull,
            // Setup
            check_setup_complete,
            run_setup,
            run_setup_v2,
            // Wallets
            generate_wallet,
            generate_near_wallet_full,
            validate_wallet_address,
            import_wallet,
            // Security
            get_guardrails_preset,
            // Google
            check_gog_available,
            run_gog_auth,
            check_gog_authenticated,
            install_gog,
            // Portfolio & Chat
            get_portfolio,
            send_chat_message,
            send_chat_message_to_session,
            // Sessions & Folders
            list_chat_sessions,
            create_chat_session,
            rename_chat_session,
            move_session_to_folder,
            get_chat_folders,
            create_chat_folder,
            rename_chat_folder,
            delete_chat_folder,
            // Source Intelligence
            verify_source,
            // 1Click API
            get_supported_tokens,
            get_cross_chain_quote,
            get_swap_status,
            resolve_asset_id,
            // ZEC Privacy Shield
            get_zec_shield_quote,
            get_zec_unshield_quote,
            get_shieldable_assets,
            // Container
            docker_start,
            docker_stop,
            docker_status,
            // Ollama (local models)
            check_ollama,
            install_ollama,
            list_ollama_models,
            pull_ollama_model,
            delete_ollama_model,
            chat_ollama,
            get_system_ram,
            // Agent identity
            get_agent_name,
            // Settings
            read_current_config,
            save_settings,
            restart_container,
            // ClawdTalk (voice)
            clawdtalk_status,
            clawdtalk_configure,
            clawdtalk_remove,
            clawdtalk_start,
            clawdtalk_stop,
            clawdtalk_logs,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            // Start portfolio file watcher in background
            tauri::async_runtime::spawn(async move {
                if let Err(e) = portfolio::start_watcher(handle).await {
                    eprintln!("Portfolio watcher error: {}", e);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Nyx");
}
