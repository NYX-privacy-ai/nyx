use crate::config;
use crate::docker;
use crate::wallet;
use std::path::PathBuf;
use tauri::Manager;

/// Check if Nyx has been set up (openclaw.json exists).
pub async fn is_setup_complete() -> Result<bool, String> {
    let home = config::home_dir();
    let config_path = home.join(".openclaw/openclaw.json");
    let env_path = home.join("openclaw/docker.env");
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
    // relative to the current executable
    let dev_path = std::env::current_dir()
        .unwrap_or_default()
        .join("src-tauri/resources");
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // Also check if resource_path itself contains the expected files
    if resource_path.join("docker-compose.yml").exists() {
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
    let home = config::home_dir();
    wallet::save_wallet(&wallet_info, &home.join(".openclaw/secrets"))?;
    wallet::save_wallet_key(&wallet_config.id, &wallet_info)?;

    // Step 3: Write config files — credentials injected via env vars (IronClaw pattern)
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

    config::write_docker_env(&setup_config)?;
    config::write_openclaw_config(&setup_config)?;
    config::write_guardrails(&guardrails)?;
    config::write_cron_jobs(&setup_config)?;

    // Step 4: Write empty function call keys
    let keys_path = home.join(".openclaw/secrets/function_call_keys.json");
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

    // Step 6: Pull Docker image
    docker::pull_image("ghcr.io/openclaw/openclaw:2026.2.17").await?;

    // Step 7: Start container
    docker::start_container().await?;

    // Step 8: Write LaunchAgent
    write_launch_agent()?;

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
    let gateway_token = config::generate_token();
    let home = config::home_dir();

    // Step 1: Create directory structure
    config::create_directories()?;

    // Step 2: Save private keys for any NEAR wallets that were generated
    // (The UI calls generate_near_wallet_full which gives us WalletInfo + WalletConfig.
    //  Private keys are saved per-wallet at generation time, so nothing extra here.)

    // Step 3: Write config files
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

    config::write_docker_env(&setup_config)?;
    config::write_openclaw_config(&setup_config)?;
    config::write_guardrails(&guardrails)?;
    config::write_cron_jobs(&setup_config)?;

    // Step 4: Write empty function call keys
    let keys_path = home.join(".openclaw/secrets/function_call_keys.json");
    std::fs::write(&keys_path, "{}").map_err(|e| format!("Failed to write keys: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&keys_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set keys permissions: {}", e))?;
    }

    // Step 5: Copy bundled resources
    let resources_dir = resolve_resources_dir(&app_handle)?;
    config::copy_resources(&resources_dir)?;

    // Step 5b: Personalize SOUL.md with the configured agent name
    let soul_path = home.join("openclaw/workspace/SOUL.md");
    if soul_path.exists() {
        let soul_content = std::fs::read_to_string(&soul_path)
            .map_err(|e| format!("Failed to read SOUL.md: {}", e))?;
        let personalized = soul_content.replace("You're Nyx", &format!("You're {}", agent_name));
        std::fs::write(&soul_path, personalized)
            .map_err(|e| format!("Failed to write SOUL.md: {}", e))?;
    }

    // Step 6: Pull Docker image
    docker::pull_image("ghcr.io/openclaw/openclaw:2026.2.17").await?;

    // Step 7: Start container
    docker::start_container().await?;

    // Step 8: Write LaunchAgent
    write_launch_agent()?;

    // Return the active wallet address as confirmation
    let active_address = setup_config
        .wallets
        .iter()
        .find(|w| Some(&w.id) == setup_config.active_wallet_id.as_ref())
        .map(|w| w.address.clone())
        .unwrap_or_else(|| "setup_complete".to_string());

    Ok(active_address)
}

fn write_launch_agent() -> Result<(), String> {
    let home = config::home_dir();
    let plist_dir = home.join("Library/LaunchAgents");
    std::fs::create_dir_all(&plist_dir)
        .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.nyx.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/bash</string>
        <string>{}/openclaw/start-nyx.sh</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>{}</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin</string>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/nyx-launchagent-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/nyx-launchagent-stderr.log</string>
</dict>
</plist>"#,
        home.display(),
        home.display()
    );

    let path = plist_dir.join("com.nyx.agent.plist");
    std::fs::write(&path, plist)
        .map_err(|e| format!("Failed to write LaunchAgent: {}", e))?;

    Ok(())
}
