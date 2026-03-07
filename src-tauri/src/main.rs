// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Shared modules from nyx_lib (used by both Tauri GUI and MCP server)
use nyx_lib::config;
use nyx_lib::docker;
use nyx_lib::gateway;
use nyx_lib::ironclaw;
use nyx_lib::oneclick;
use nyx_lib::wallet;

// Tauri-only modules (UI-specific or have Tauri dependencies)
mod browser;
mod clawdtalk;
mod claudecode;
mod google;
mod intelligence;
mod ollama;
mod portfolio;
mod pty;
mod setup;

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
    docker::pull_image("ghcr.io/openclaw/openclaw:2026.2.21").await
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
    perplexity_key: Option<String>,
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
        perplexity_key,
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
        false,
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

/// Execute a shield swap (any supported asset → shielded ZEC). Live, not dry run.
#[tauri::command]
async fn execute_zec_shield(
    from_asset: String,
    amount: String,
) -> Result<oneclick::QuoteResponse, String> {
    let zec_address = config::get_zec_address()
        .ok_or_else(|| "No ZEC address configured. Add a ZEC wallet in Settings.".to_string())?;
    let refund_to = config::get_near_account()
        .unwrap_or_else(|| "nyx.near".to_string());
    oneclick::execute_zec_shield(&from_asset, &amount, &zec_address, &refund_to).await
}

/// Execute an unshield swap (ZEC → any supported asset). Live, not dry run.
#[tauri::command]
async fn execute_zec_unshield(
    to_asset: String,
    zec_amount: String,
    recipient: String,
) -> Result<oneclick::QuoteResponse, String> {
    let zec_refund = config::get_zec_address()
        .ok_or_else(|| "No ZEC address configured. Add a ZEC wallet in Settings.".to_string())?;
    oneclick::execute_zec_unshield(&to_asset, &zec_amount, &recipient, &zec_refund).await
}

/// Get the list of assets that can be shielded to ZEC.
#[tauri::command]
fn get_shieldable_assets() -> Vec<oneclick::ShieldableAsset> {
    oneclick::get_shieldable_assets()
}

// ---------------------------------------------------------------------------
// Confidential Intents (NEAR private shard / TEE)
// ---------------------------------------------------------------------------

/// Get a confidential cross-chain swap quote. Executes via NEAR's TEE-secured
/// private shard — prevents MEV, frontrunning, and strategy leakage.
#[tauri::command]
async fn get_confidential_quote(
    origin_asset: String,
    destination_asset: String,
    amount: String,
) -> Result<oneclick::QuoteResponse, String> {
    let near_account = config::get_near_account()
        .unwrap_or_else(|| "nyx.near".to_string());
    oneclick::get_confidential_quote(
        &origin_asset,
        &destination_asset,
        &amount,
        &near_account,
        &near_account,
    )
    .await
}

/// Execute a confidential cross-chain swap (live, not dry run).
#[tauri::command]
async fn execute_confidential_swap(
    origin_asset: String,
    destination_asset: String,
    amount: String,
) -> Result<oneclick::QuoteResponse, String> {
    let near_account = config::get_near_account()
        .unwrap_or_else(|| "nyx.near".to_string());
    oneclick::execute_confidential_swap(
        &origin_asset,
        &destination_asset,
        &amount,
        &near_account,
        &near_account,
    )
    .await
}

// ---------------------------------------------------------------------------
// Container lifecycle (legacy Docker — kept for rollback)
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
// IronClaw daemon lifecycle
// ---------------------------------------------------------------------------

#[tauri::command]
async fn check_ironclaw() -> Result<bool, String> {
    ironclaw::is_daemon_running().await
}

#[tauri::command]
async fn check_ironclaw_detailed() -> Result<ironclaw::IronClawCheck, String> {
    ironclaw::check_ironclaw_detailed().await
}

#[tauri::command]
async fn install_ironclaw() -> Result<String, String> {
    ironclaw::install_ironclaw().await
}

#[tauri::command]
async fn upgrade_ironclaw() -> Result<String, String> {
    ironclaw::upgrade_ironclaw().await
}

#[tauri::command]
async fn ironclaw_start() -> Result<(), String> {
    ironclaw::start_daemon().await
}

#[tauri::command]
async fn ironclaw_stop() -> Result<(), String> {
    ironclaw::stop_daemon().await
}

#[tauri::command]
async fn ironclaw_status() -> Result<String, String> {
    ironclaw::daemon_status().await
}

#[tauri::command]
async fn restart_ironclaw() -> Result<(), String> {
    ironclaw::restart_daemon().await
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

/// Read the configured agent name from IronClaw config.toml (fallback: "Nyx").
#[tauri::command]
fn get_agent_name() -> Result<String, String> {
    let config_path = ironclaw::config_dir().join("config.toml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Ok("Nyx".to_string()),
    };
    // Simple TOML parsing — look for agent.name
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains('=') {
            if let Some(val) = trimmed.split('=').nth(1) {
                let name = val.trim().trim_matches('"').to_string();
                if !name.is_empty() {
                    return Ok(name);
                }
            }
        }
    }
    Ok("Nyx".to_string())
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
    // Now restarts IronClaw daemon instead of Docker container
    ironclaw::restart_daemon().await
}

// ---------------------------------------------------------------------------
// Claude Code integration
// ---------------------------------------------------------------------------

#[tauri::command]
fn claude_code_status() -> Result<claudecode::ClaudeCodeStatus, String> {
    claudecode::check_status()
}

#[tauri::command]
async fn claude_code_register_mcp() -> Result<String, String> {
    claudecode::register_mcp_server().await
}

#[tauri::command]
async fn claude_code_unregister_mcp() -> Result<(), String> {
    claudecode::unregister_mcp_server().await
}

// ---------------------------------------------------------------------------
// PTY (embedded terminal)
// ---------------------------------------------------------------------------

#[tauri::command]
fn pty_spawn(
    app: tauri::AppHandle,
    command: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<String, String> {
    pty::spawn(app, command, cols.unwrap_or(120), rows.unwrap_or(36))
}

#[tauri::command]
fn pty_write(session_id: String, data: String) -> Result<(), String> {
    pty::write_to(&session_id, &data)
}

#[tauri::command]
fn pty_resize(session_id: String, cols: u16, rows: u16) -> Result<(), String> {
    pty::resize(&session_id, cols, rows)
}

#[tauri::command]
fn pty_kill(session_id: String) -> Result<(), String> {
    pty::kill(&session_id)
}

// ---------------------------------------------------------------------------
// Activity Intelligence
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_intelligence_suggestions() -> Result<Vec<intelligence::Suggestion>, String> {
    intelligence::get_suggestions()
}

#[tauri::command]
fn dismiss_intelligence_suggestion(id: i64) -> Result<(), String> {
    intelligence::dismiss_suggestion(id)
}

#[tauri::command]
fn accept_intelligence_suggestion(id: i64) -> Result<intelligence::Suggestion, String> {
    intelligence::accept_suggestion(id)
}

#[tauri::command]
fn get_contact_insights(email: String) -> Result<intelligence::ContactInsight, String> {
    intelligence::get_contact_insights(&email)
}

#[tauri::command]
fn get_activity_stats() -> Result<intelligence::ActivityStats, String> {
    intelligence::get_activity_stats()
}

#[tauri::command]
fn get_autonomy_settings() -> Result<Vec<intelligence::AutonomySetting>, String> {
    intelligence::get_autonomy_settings()
}

#[tauri::command]
fn set_autonomy_level(activity_type: String, level: String) -> Result<(), String> {
    intelligence::set_autonomy_level(&activity_type, &level)
}

#[tauri::command]
fn clear_intelligence_data() -> Result<(), String> {
    intelligence::clear_all_data()
}

// ---------------------------------------------------------------------------
// Web Browser (agent-controlled browsing)
// ---------------------------------------------------------------------------

#[tauri::command]
fn browser_open(app: tauri::AppHandle) -> Result<(), String> {
    browser::open(&app)
}

#[tauri::command]
fn browser_close(app: tauri::AppHandle) -> Result<(), String> {
    browser::close(&app)
}

#[tauri::command]
fn browser_state() -> Result<Option<browser::BrowserState>, String> {
    browser::get_state()
}

#[tauri::command]
fn browser_navigate(app: tauri::AppHandle, url: String) -> Result<(), String> {
    browser::navigate(&app, &url)
}

#[tauri::command]
fn browser_go_back(app: tauri::AppHandle) -> Result<(), String> {
    browser::go_back(&app)
}

#[tauri::command]
fn browser_go_forward(app: tauri::AppHandle) -> Result<(), String> {
    browser::go_forward(&app)
}

#[tauri::command]
fn browser_click(app: tauri::AppHandle, selector: String) -> Result<String, String> {
    browser::click(&app, &selector)
}

#[tauri::command]
fn browser_type_text(app: tauri::AppHandle, selector: String, text: String) -> Result<String, String> {
    browser::type_text(&app, &selector, &text)
}

#[tauri::command]
fn browser_scroll(app: tauri::AppHandle, direction: String, amount: Option<i32>) -> Result<String, String> {
    browser::scroll(&app, &direction, amount.unwrap_or(3))
}

#[tauri::command]
fn browser_read_page(app: tauri::AppHandle) -> Result<String, String> {
    browser::read_page(&app)
}

#[tauri::command]
fn browser_read_links(app: tauri::AppHandle) -> Result<String, String> {
    browser::read_links(&app)
}

#[tauri::command]
fn browser_read_forms(app: tauri::AppHandle) -> Result<String, String> {
    browser::read_forms(&app)
}

#[tauri::command]
fn browser_select_option(app: tauri::AppHandle, selector: String, value: String) -> Result<String, String> {
    browser::select_option(&app, &selector, &value)
}

#[tauri::command]
fn browser_execute_js(app: tauri::AppHandle, code: String) -> Result<String, String> {
    browser::execute_js(&app, &code)
}

#[tauri::command]
async fn browser_execute_action(
    app: tauri::AppHandle,
    action: browser::BrowserAction,
) -> Result<browser::BrowserActionResult, String> {
    Ok(browser::execute_action(&app, &action).await)
}

/// Send a message with browser tool to the agent and run the full agent loop.
#[tauri::command]
async fn browser_send_message(
    app: tauri::AppHandle,
    message: String,
    session_key: Option<String>,
) -> Result<String, String> {
    let key = session_key.unwrap_or_else(|| "agent:default:browse".to_string());
    browser::send_browse_message(&app, message, key).await
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
    // Store the raw API key in .env
    let env_path = ironclaw::config_dir().join(".env");

    // Read existing .env
    let content = std::fs::read_to_string(&env_path).unwrap_or_default();

    // Check if CLAWDTALK_API_KEY already exists
    let has_key = content.lines().any(|l| l.trim().starts_with("CLAWDTALK_API_KEY="));

    let updated = if has_key {
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
        format!("{}\n# ClawdTalk Voice\nCLAWDTALK_API_KEY={}\n", content.trim_end(), api_key)
    };

    std::fs::write(&env_path, updated)
        .map_err(|e| format!("Failed to update .env: {}", e))?;

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

    // Remove key from .env
    let env_path = ironclaw::config_dir().join(".env");
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
// Session history
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_session_history(
    session_key: String,
    limit: Option<usize>,
) -> Result<Vec<gateway::HistoryMessage>, String> {
    gateway::load_session_history(&session_key, limit.unwrap_or(50))
}

// ---------------------------------------------------------------------------
// Scheduling (cron job CRUD)
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_scheduled_tasks() -> Result<Vec<config::CronJob>, String> {
    let file = config::read_cron_jobs()?;
    Ok(file.jobs)
}

#[tauri::command]
fn create_scheduled_task(
    name: String,
    schedule_kind: String,
    schedule_value: String,
    timezone: Option<String>,
    message: String,
    enabled: Option<bool>,
) -> Result<config::CronJob, String> {
    let schedule = match schedule_kind.as_str() {
        "every" => {
            let ms: u64 = schedule_value
                .parse()
                .map_err(|_| "schedule_value must be a number (milliseconds) for 'every' kind")?;
            config::CronSchedule::Every { every_ms: ms, anchor_ms: None }
        }
        "cron" => config::CronSchedule::Cron {
            expr: schedule_value,
            tz: timezone,
        },
        other => return Err(format!("Unknown schedule_kind '{}'. Use 'every' or 'cron'.", other)),
    };

    config::create_cron_job(name, schedule, message, enabled.unwrap_or(true))
}

#[tauri::command]
fn update_scheduled_task(
    id: String,
    name: Option<String>,
    schedule_kind: Option<String>,
    schedule_value: Option<String>,
    timezone: Option<String>,
    message: Option<String>,
    enabled: Option<bool>,
) -> Result<config::CronJob, String> {
    let schedule = match (schedule_kind.as_deref(), schedule_value) {
        (Some("every"), Some(val)) => {
            let ms: u64 = val
                .parse()
                .map_err(|_| "schedule_value must be a number (milliseconds) for 'every' kind")?;
            Some(config::CronSchedule::Every { every_ms: ms, anchor_ms: None })
        }
        (Some("cron"), Some(val)) => Some(config::CronSchedule::Cron {
            expr: val,
            tz: timezone,
        }),
        (Some(other), _) => {
            return Err(format!("Unknown schedule_kind '{}'. Use 'every' or 'cron'.", other))
        }
        _ => None,
    };

    config::update_cron_job(
        &id,
        config::CronJobUpdate {
            name,
            schedule,
            message,
            enabled,
        },
    )
}

#[tauri::command]
fn delete_scheduled_task(id: String) -> Result<(), String> {
    config::delete_cron_job(&id)
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
            get_session_history,
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
            execute_zec_shield,
            execute_zec_unshield,
            // Confidential Intents (NEAR TEE)
            get_confidential_quote,
            execute_confidential_swap,
            // Container (legacy Docker)
            docker_start,
            docker_stop,
            docker_status,
            // IronClaw daemon
            check_ironclaw,
            check_ironclaw_detailed,
            install_ironclaw,
            upgrade_ironclaw,
            ironclaw_start,
            ironclaw_stop,
            ironclaw_status,
            restart_ironclaw,
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
            // Scheduling
            list_scheduled_tasks,
            create_scheduled_task,
            update_scheduled_task,
            delete_scheduled_task,
            // ClawdTalk (voice)
            clawdtalk_status,
            clawdtalk_configure,
            clawdtalk_remove,
            clawdtalk_start,
            clawdtalk_stop,
            clawdtalk_logs,
            // Claude Code
            claude_code_status,
            claude_code_register_mcp,
            claude_code_unregister_mcp,
            // PTY (embedded terminal)
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            // Activity Intelligence
            get_intelligence_suggestions,
            dismiss_intelligence_suggestion,
            accept_intelligence_suggestion,
            get_contact_insights,
            get_activity_stats,
            get_autonomy_settings,
            set_autonomy_level,
            clear_intelligence_data,
            // Web Browser
            browser_open,
            browser_close,
            browser_state,
            browser_navigate,
            browser_go_back,
            browser_go_forward,
            browser_click,
            browser_type_text,
            browser_scroll,
            browser_read_page,
            browser_read_links,
            browser_read_forms,
            browser_select_option,
            browser_execute_js,
            browser_execute_action,
            browser_send_message,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            // Start portfolio file watcher in background
            tauri::async_runtime::spawn(async move {
                if let Err(e) = portfolio::start_watcher(handle).await {
                    eprintln!("Portfolio watcher error: {}", e);
                }
            });

            // Auto-upgrade IronClaw binary if the Nyx app version changed
            // (e.g. after auto-updater delivered a new Nyx release).
            // Runs in background — never blocks startup or crashes on failure.
            tauri::async_runtime::spawn(async {
                ironclaw::check_and_auto_upgrade().await;
            });

            // Start Activity Intelligence observer in background (only if enabled)
            let intel_handle = app.handle().clone();
            if config::read_current_config()
                .map(|c| c.capabilities.activity_intelligence)
                .unwrap_or(false)
            {
                intelligence::start_observer(intel_handle);
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Nyx");
}
