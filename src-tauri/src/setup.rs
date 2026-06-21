use crate::config;
use crate::ironclaw;
use crate::wallet;
use std::path::PathBuf;
use tauri::Manager;

/// Check if Nyx has been set up (config.toml + .env exist).
pub async fn is_setup_complete() -> Result<bool, String> {
    let nyx_dir = ironclaw::config_dir();
    let config_path = nyx_dir.join("config.toml");
    let env_path = nyx_dir.join(".env");
    Ok(config_path.exists() && env_path.exists())
}

/// Resolve the bundled resources directory.
/// In production: Tauri bundles resources into the app bundle.
/// In development: resources are in src-tauri/resources/ relative to the project.
pub fn resolve_resources_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    // Tauri v2: resource_dir() returns the path to bundled resources
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to resolve resource dir: {}", e))?;

    // In production, resources are at <app>/Contents/Resources/resources/
    let bundled = resource_path.join("resources");
    if bundled.exists() {
        return Ok(bundled);
    }

    // In development, Tauri may not resolve properly — check src-tauri/resources/
    let dev_path = std::env::current_dir()
        .unwrap_or_default()
        .join("src-tauri/resources");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Also check if resource_path itself contains the expected files
    if resource_path.join("workspace").exists() {
        return Ok(resource_path);
    }

    Err("Could not find bundled resources directory".to_string())
}

/// Run the full setup process.
pub async fn run_setup(
    app_handle: tauri::AppHandle,
    anthropic_key: String,
    openai_key: Option<String>,
    telegram_token: Option<String>,
) -> Result<String, String> {
    let gateway_token = config::generate_token();

    // Step 1: Create directory structure
    config::create_directories()?;

    // Step 2: Generate NEAR wallet
    let (wallet_info, wallet_config) = wallet::generate_near_wallet().await?;
    let nyx_dir = ironclaw::config_dir();
    wallet::save_wallet(&wallet_info, &nyx_dir.join("secrets"))?;
    wallet::save_wallet_key(&wallet_config.id, &wallet_info)?;

    // Step 3: Write config files
    let guardrails = config::GuardrailsConfig::default();

    let setup_config = config::SetupConfig {
        agent_name: "Nyx".to_string(),
        anthropic_key,
        openai_key,
        venice_key: None,
        nearai_key: None,
        perplexity_key: None,
        telegram_token,
        slack_token: None,
        whatsapp_phone: None,
        gateway_token: gateway_token.clone(),
        wallets: vec![wallet_config],
        active_wallet_id: Some(wallet_info.account_id.clone()),
        guardrails: guardrails.clone(),
        messaging: config::MessagingConfig::default(),
        google_authenticated: false,
        email_notifications: config::EmailNotificationsConfig::default(),
        capabilities: config::CapabilitiesConfig::default(),
    };

    config::write_nyx_env(&setup_config)?;
    config::write_ironclaw_config(&setup_config)?;
    config::write_guardrails(&guardrails)?;
    config::write_cron_jobs(&setup_config)?;

    // Step 4: Write empty function call keys
    let keys_path = nyx_dir.join("secrets/function_call_keys.json");
    std::fs::write(&keys_path, "{}").map_err(|e| format!("Failed to write keys: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keys_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set keys permissions: {}", e))?;
    }

    // Step 5: Copy bundled resources (resolved via Tauri at runtime)
    let resources_dir = resolve_resources_dir(&app_handle)?;
    config::copy_resources(&resources_dir)?;

    // Step 6: Write daemon wrapper script
    write_daemon_script()?;

    // Step 7: Write LaunchAgent plist
    write_launch_agent()?;

    // Step 8: Start IronClaw daemon
    ironclaw::start_daemon().await?;

    Ok(wallet_info.account_id)
}

/// Extended setup that accepts the full v2 configuration from the setup wizard.
/// Wallets are passed in directly (already generated/imported by the UI).
pub async fn run_setup_v2(
    app_handle: tauri::AppHandle,
    agent_name: String,
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
    guardrails: config::GuardrailsConfig,
    messaging: config::MessagingConfig,
    google_authenticated: bool,
    email_notifications: config::EmailNotificationsConfig,
    capabilities: config::CapabilitiesConfig,
) -> Result<String, String> {
    // Validate guardrails up front, before anything is written to disk. The
    // wizard's number inputs are not trusted; reject bad values before they
    // can land in .env / config.toml / defi_guardrails.env.
    guardrails.validate()?;

    let gateway_token = config::generate_token();
    let nyx_dir = ironclaw::config_dir();

    // Step 1: Create directory structure
    config::create_directories()?;

    // Step 2: Write config files
    let active_id = active_wallet_id.or_else(|| {
        wallets.first().map(|w| w.id.clone())
    });

    let setup_config = config::SetupConfig {
        agent_name: agent_name.clone(),
        anthropic_key,
        openai_key,
        venice_key,
        nearai_key,
        perplexity_key,
        telegram_token,
        slack_token,
        whatsapp_phone,
        gateway_token: gateway_token.clone(),
        wallets,
        active_wallet_id: active_id,
        guardrails: guardrails.clone(),
        messaging,
        google_authenticated,
        email_notifications,
        capabilities,
    };

    config::write_nyx_env(&setup_config)?;
    config::write_ironclaw_config(&setup_config)?;
    config::write_guardrails(&guardrails)?;
    config::write_cron_jobs(&setup_config)?;

    // Step 3: Write empty function call keys
    let keys_path = nyx_dir.join("secrets/function_call_keys.json");
    std::fs::write(&keys_path, "{}").map_err(|e| format!("Failed to write keys: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keys_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set keys permissions: {}", e))?;
    }

    // Step 4: Copy bundled resources
    let resources_dir = resolve_resources_dir(&app_handle)?;
    config::copy_resources(&resources_dir)?;

    // Step 4b: Personalize SOUL.md with the configured agent name
    let soul_path = nyx_dir.join("workspace/SOUL.md");
    if soul_path.exists() {
        let soul_content = std::fs::read_to_string(&soul_path)
            .map_err(|e| format!("Failed to read SOUL.md: {}", e))?;
        let personalized = soul_content.replace("You're Nyx", &format!("You're {}", agent_name));
        std::fs::write(&soul_path, personalized)
            .map_err(|e| format!("Failed to write SOUL.md: {}", e))?;
    }

    // Step 5: Write daemon wrapper script
    write_daemon_script()?;

    // Step 6: Write LaunchAgent plist
    write_launch_agent()?;

    // Step 7: Start IronClaw daemon
    ironclaw::start_daemon().await?;

    // Return the active wallet address as confirmation
    let active_address = setup_config
        .wallets
        .iter()
        .find(|w| Some(&w.id) == setup_config.active_wallet_id.as_ref())
        .map(|w| w.address.clone())
        .unwrap_or_else(|| "setup_complete".to_string());

    Ok(active_address)
}

/// Write the daemon wrapper script that ensures proper PATH and keeps stdin open.
/// (Atlas learning #2: launchd provides minimal PATH)
fn write_daemon_script() -> Result<(), String> {
    let home = config::home_dir();
    let nyx_dir = ironclaw::config_dir();
    let script_path = nyx_dir.join("run-daemon.sh");

    let content = format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
export PATH="/usr/local/bin:/opt/homebrew/bin:{home}/.cargo/bin:{home}/.nyx/bin:$PATH"
export HOME="{home}"

# Source IronClaw environment
set -a
source "{nyx_dir}/.env"
set +a

# Export required vars for IronClaw
export LLM_BACKEND
export ANTHROPIC_API_KEY
export DATABASE_BACKEND
export LIBSQL_PATH
export GATEWAY_AUTH_TOKEN

# Run IronClaw daemon with config pointing to Nyx directory
# Uses PATH resolution (binary may be in ~/.cargo/bin, /opt/homebrew/bin, etc.)
# Process substitution keeps stdin open (Atlas learning: REPL channel needs stdin)
exec ironclaw run --config "{nyx_dir}/config.toml" < <(
    while true; do sleep 86400; done
)
"#,
        home = home.display(),
        nyx_dir = nyx_dir.display(),
    );

    std::fs::write(&script_path, content)
        .map_err(|e| format!("Failed to write run-daemon.sh: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set run-daemon.sh permissions: {}", e))?;
    }

    Ok(())
}

/// Write the macOS LaunchAgent plist for the Nyx IronClaw daemon.
fn write_launch_agent() -> Result<(), String> {
    let home = config::home_dir();
    let nyx_dir = ironclaw::config_dir();
    let plist_dir = home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&plist_dir)
        .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.nyx.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>{}/run-daemon.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>{}</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:{}/.cargo/bin</string>
    </dict>
    <key>StandardOutPath</key>
    <string>{}/logs/daemon-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{}/logs/daemon-stderr.log</string>
</dict>
</plist>"#,
        nyx_dir.display(),
        home.display(),
        home.display(),
        nyx_dir.display(),
        nyx_dir.display(),
    );

    let path = plist_dir.join("com.nyx.daemon.plist");
    std::fs::write(&path, plist)
        .map_err(|e| format!("Failed to write LaunchAgent: {}", e))?;

    Ok(())
}
