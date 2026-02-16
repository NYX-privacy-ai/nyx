use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Chain {
    NEAR,
    ETH,
    SOL,
    BTC,
    ZEC,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::NEAR => write!(f, "near"),
            Chain::ETH => write!(f, "eth"),
            Chain::SOL => write!(f, "sol"),
            Chain::BTC => write!(f, "btc"),
            Chain::ZEC => write!(f, "zec"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletConfig {
    pub id: String,
    pub chain: Chain,
    pub address: String,
    pub label: String,
    pub has_private_key: bool,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SecurityPreset {
    Conservative,
    Balanced,
    Autonomous,
    Custom,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GuardrailsConfig {
    pub preset: SecurityPreset,
    pub max_transaction_usd: f64,
    pub daily_loss_percent: f64,
    pub weekly_loss_percent: f64,
    pub daily_tx_limit: u32,
    pub require_confirmation: bool,
    pub max_slippage_percent: f64,
    pub max_concentration_percent: f64,
    pub min_health_factor: f64,
}

impl GuardrailsConfig {
    pub fn from_preset(preset: SecurityPreset) -> Self {
        match preset {
            SecurityPreset::Conservative => GuardrailsConfig {
                preset: SecurityPreset::Conservative,
                max_transaction_usd: 100.0,
                daily_loss_percent: 2.0,
                weekly_loss_percent: 5.0,
                daily_tx_limit: 10,
                require_confirmation: true,
                max_slippage_percent: 1.0,
                max_concentration_percent: 25.0,
                min_health_factor: 2.0,
            },
            SecurityPreset::Balanced => GuardrailsConfig {
                preset: SecurityPreset::Balanced,
                max_transaction_usd: 500.0,
                daily_loss_percent: 5.0,
                weekly_loss_percent: 15.0,
                daily_tx_limit: 20,
                require_confirmation: false,
                max_slippage_percent: 2.0,
                max_concentration_percent: 40.0,
                min_health_factor: 1.5,
            },
            SecurityPreset::Autonomous => GuardrailsConfig {
                preset: SecurityPreset::Autonomous,
                max_transaction_usd: 1_000_000.0,
                daily_loss_percent: 100.0,
                weekly_loss_percent: 100.0,
                daily_tx_limit: 1000,
                require_confirmation: false,
                max_slippage_percent: 50.0,
                max_concentration_percent: 100.0,
                min_health_factor: 1.0,
            },
            SecurityPreset::Custom => GuardrailsConfig {
                preset: SecurityPreset::Custom,
                max_transaction_usd: 500.0,
                daily_loss_percent: 5.0,
                weekly_loss_percent: 15.0,
                daily_tx_limit: 20,
                require_confirmation: false,
                max_slippage_percent: 2.0,
                max_concentration_percent: 40.0,
                min_health_factor: 1.5,
            },
        }
    }
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        GuardrailsConfig::from_preset(SecurityPreset::Balanced)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessagingAutonomy {
    DraftOnly,
    SendWithConfirm,
    Autonomous,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub autonomy: MessagingAutonomy,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        ChannelConfig {
            enabled: false,
            autonomy: MessagingAutonomy::DraftOnly,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessagingConfig {
    pub gmail: ChannelConfig,
    pub whatsapp: ChannelConfig,
    pub telegram: ChannelConfig,
    pub slack: ChannelConfig,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        MessagingConfig {
            gmail: ChannelConfig::default(),
            whatsapp: ChannelConfig::default(),
            telegram: ChannelConfig::default(),
            slack: ChannelConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Email Notifications Config
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EmailNotificationsConfig {
    /// Whether email notifications are enabled at all
    pub enabled: bool,
    /// IANA timezone string, e.g. "Europe/London", "America/New_York"
    pub timezone: String,
    /// Hour (0-23) to send the daily email digest
    pub digest_hour: u8,
    /// Minute (0-59) to send the daily email digest
    pub digest_minute: u8,
    /// Start hour (0-23) for hourly triage window (inclusive)
    pub triage_start_hour: u8,
    /// End hour (0-23) for hourly triage window (inclusive)
    pub triage_end_hour: u8,
}

impl Default for EmailNotificationsConfig {
    fn default() -> Self {
        EmailNotificationsConfig {
            enabled: true,
            timezone: "Europe/London".to_string(),
            digest_hour: 8,
            digest_minute: 30,
            triage_start_hour: 8,
            triage_end_hour: 22,
        }
    }
}

// ---------------------------------------------------------------------------
// Capabilities Config
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapabilitiesConfig {
    pub defi_crypto: bool,
    pub travel: bool,
    pub google_workspace: bool,
    pub email_intelligence: bool,
    pub communications: bool,
    pub source_intelligence: bool,
    /// Default LLM provider: "anthropic", "venice", "openai", "nearai", or "ollama"
    pub default_llm_provider: String,
    /// Selected Ollama model tag (e.g. "qwen3:4b"), None if not using local models
    #[serde(default)]
    pub ollama_model: Option<String>,
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        CapabilitiesConfig {
            defi_crypto: true,
            travel: true,
            google_workspace: true,
            email_intelligence: true,
            communications: true,
            source_intelligence: true,
            default_llm_provider: "anthropic".to_string(),
            ollama_model: None,
        }
    }
}

// ---------------------------------------------------------------------------
// SetupConfig
// ---------------------------------------------------------------------------

pub struct SetupConfig {
    pub agent_name: String,
    pub anthropic_key: String,
    pub openai_key: Option<String>,
    pub venice_key: Option<String>,
    pub nearai_key: Option<String>,
    pub telegram_token: Option<String>,
    pub slack_token: Option<String>,
    pub whatsapp_phone: Option<String>,
    pub gateway_token: String,
    pub wallets: Vec<WalletConfig>,
    pub active_wallet_id: Option<String>,
    pub guardrails: GuardrailsConfig,
    pub messaging: MessagingConfig,
    pub google_authenticated: bool,
    pub email_notifications: EmailNotificationsConfig,
    pub capabilities: CapabilitiesConfig,
}

// ---------------------------------------------------------------------------
// Settings (read & update)
// ---------------------------------------------------------------------------

/// Config returned to the frontend for the Settings page.
/// API keys are NEVER returned — only boolean flags indicating presence.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettingsConfig {
    pub agent_name: String,
    pub has_anthropic_key: bool,
    pub has_openai_key: bool,
    pub has_venice_key: bool,
    pub has_nearai_key: bool,
    pub has_telegram_token: bool,
    pub has_slack_token: bool,
    pub whatsapp_phone: Option<String>,
    pub guardrails: GuardrailsConfig,
    pub messaging: MessagingConfig,
    pub google_authenticated: bool,
    pub email_notifications: EmailNotificationsConfig,
    pub capabilities: CapabilitiesConfig,
    pub default_llm_provider: String,
}

/// Partial update struct — None fields are preserved from existing config.
/// For keys: None = keep existing, Some("") = clear, Some(val) = update.
#[derive(Deserialize, Clone, Debug)]
pub struct SettingsUpdate {
    pub agent_name: Option<String>,
    pub anthropic_key: Option<String>,
    pub openai_key: Option<String>,
    pub venice_key: Option<String>,
    pub nearai_key: Option<String>,
    pub telegram_token: Option<String>,
    pub slack_token: Option<String>,
    pub whatsapp_phone: Option<String>,
    pub guardrails: Option<GuardrailsConfig>,
    pub messaging: Option<MessagingConfig>,
    pub email_notifications: Option<EmailNotificationsConfig>,
    pub capabilities: Option<CapabilitiesConfig>,
}

#[derive(Serialize, Clone, Debug)]
pub struct SettingsSaveResult {
    pub success: bool,
    pub restart_required: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get the home directory.
pub fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME not set"))
}

/// Generate a random 32-byte hex token.
pub fn generate_token() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

// ---------------------------------------------------------------------------
// Settings helpers
// ---------------------------------------------------------------------------

/// Parse a KEY=VALUE env file into a HashMap. Skips comments and empty lines.
fn parse_env_file(path: &Path) -> Result<std::collections::HashMap<String, String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut map = std::collections::HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            map.insert(key, value);
        }
    }
    Ok(map)
}

/// Read current configuration from config files. Returns SettingsConfig for the frontend.
pub fn read_current_config() -> Result<SettingsConfig, String> {
    let home = home_dir();

    // Parse docker.env
    let env_path = home.join("openclaw/docker.env");
    let env = parse_env_file(&env_path)?;

    // Parse openclaw.json
    let config_path = home.join(".openclaw/openclaw.json");
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read openclaw.json: {}", e))?;
    let config_json: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse openclaw.json: {}", e))?;

    // Agent name
    let agent_name = config_json
        .pointer("/agents/list/0/identity/name")
        .and_then(|v| v.as_str())
        .unwrap_or("Nyx")
        .to_string();

    // Key presence (never expose actual values)
    let has_key = |k: &str| env.get(k).map_or(false, |v| !v.is_empty());

    // Default LLM provider
    let default_llm_provider = env.get("DEFAULT_LLM_PROVIDER")
        .cloned()
        .unwrap_or_else(|| "anthropic".to_string());

    // Guardrails
    let guardrails = GuardrailsConfig {
        preset: SecurityPreset::Custom, // When reading back, always treat as custom
        max_transaction_usd: env.get("MAX_SINGLE_TX_USD")
            .and_then(|v| v.parse().ok()).unwrap_or(500.0),
        daily_loss_percent: env.get("DAILY_LOSS_LIMIT_PCT")
            .and_then(|v| v.parse().ok()).unwrap_or(5.0),
        weekly_loss_percent: env.get("WEEKLY_LOSS_LIMIT_PCT")
            .and_then(|v| v.parse().ok()).unwrap_or(15.0),
        daily_tx_limit: env.get("MAX_DAILY_TXS")
            .and_then(|v| v.parse().ok()).unwrap_or(20),
        require_confirmation: env.get("REQUIRE_CONFIRMATION")
            .map_or(false, |v| v == "true"),
        max_slippage_percent: env.get("MAX_SLIPPAGE_PCT")
            .and_then(|v| v.parse().ok()).unwrap_or(2.0),
        max_concentration_percent: env.get("MAX_CONCENTRATION_PCT")
            .and_then(|v| v.parse().ok()).unwrap_or(40.0),
        min_health_factor: env.get("BURROW_MIN_HEALTH_FACTOR")
            .and_then(|v| v.parse().ok()).unwrap_or(1.5),
    };

    // Messaging
    let parse_bool = |k: &str| env.get(k).map_or(false, |v| v == "true");

    let messaging = MessagingConfig {
        gmail: ChannelConfig {
            enabled: parse_bool("MESSAGING_GMAIL_ENABLED"),
            autonomy: MessagingAutonomy::DraftOnly,
        },
        whatsapp: ChannelConfig {
            enabled: parse_bool("MESSAGING_WHATSAPP_ENABLED"),
            autonomy: MessagingAutonomy::DraftOnly,
        },
        telegram: ChannelConfig {
            enabled: parse_bool("MESSAGING_TELEGRAM_ENABLED"),
            autonomy: MessagingAutonomy::DraftOnly,
        },
        slack: ChannelConfig {
            enabled: parse_bool("MESSAGING_SLACK_ENABLED"),
            autonomy: MessagingAutonomy::DraftOnly,
        },
    };

    // Email notifications — parse from cron/jobs.json
    let email_notifications = read_email_config(&home);

    // Capabilities
    let capabilities = CapabilitiesConfig {
        defi_crypto: parse_bool("CAPABILITY_DEFI"),
        travel: parse_bool("CAPABILITY_TRAVEL"),
        google_workspace: parse_bool("CAPABILITY_GOOGLE"),
        email_intelligence: parse_bool("CAPABILITY_EMAIL_INTEL"),
        communications: parse_bool("CAPABILITY_COMMS"),
        source_intelligence: parse_bool("CAPABILITY_SOURCE_INTEL"),
        default_llm_provider: default_llm_provider.clone(),
        ollama_model: env.get("OLLAMA_MODEL")
            .filter(|v| !v.is_empty())
            .cloned(),
    };

    // WhatsApp phone from openclaw.json
    let whatsapp_phone = config_json
        .pointer("/channels/whatsapp/allowFrom/0")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(SettingsConfig {
        agent_name,
        has_anthropic_key: has_key("ANTHROPIC_API_KEY"),
        has_openai_key: has_key("OPENAI_API_KEY"),
        has_venice_key: has_key("VENICE_API_KEY"),
        has_nearai_key: has_key("NEARAI_API_KEY"),
        has_telegram_token: has_key("TELEGRAM_BOT_TOKEN"),
        has_slack_token: has_key("SLACK_BOT_TOKEN"),
        whatsapp_phone,
        guardrails,
        messaging,
        google_authenticated: parse_bool("GOOGLE_AUTHENTICATED"),
        email_notifications,
        capabilities,
        default_llm_provider,
    })
}

/// Read email notification config from cron/jobs.json.
fn read_email_config(home: &Path) -> EmailNotificationsConfig {
    let cron_path = home.join(".openclaw/cron/jobs.json");
    let content = match fs::read_to_string(&cron_path) {
        Ok(c) => c,
        Err(_) => return EmailNotificationsConfig::default(),
    };
    let jobs: serde_json::Value = match serde_json::from_str(&content) {
        Ok(j) => j,
        Err(_) => return EmailNotificationsConfig::default(),
    };

    let mut config = EmailNotificationsConfig::default();

    if let Some(arr) = jobs.as_array() {
        for job in arr {
            let id = job.get("id").and_then(|v| v.as_str()).unwrap_or("");
            match id {
                "daily-email-digest" => {
                    if let Some(sched) = job.get("schedule") {
                        if let Some(tz) = sched.get("timezone").and_then(|v| v.as_str()) {
                            config.timezone = tz.to_string();
                        }
                        // Parse cron: "30 8 * * *" -> minute=30, hour=8
                        if let Some(cron_str) = sched.get("cron").and_then(|v| v.as_str()) {
                            let parts: Vec<&str> = cron_str.split_whitespace().collect();
                            if parts.len() >= 2 {
                                config.digest_minute = parts[0].parse().unwrap_or(30);
                                config.digest_hour = parts[1].parse().unwrap_or(8);
                            }
                        }
                    }
                    config.enabled = job.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                }
                "hourly-email-triage" => {
                    if let Some(sched) = job.get("schedule") {
                        if let Some(cron_str) = sched.get("cron").and_then(|v| v.as_str()) {
                            // Parse "0 8-22 * * *" -> start=8, end=22
                            let parts: Vec<&str> = cron_str.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let range = parts[1];
                                if let Some(dash) = range.find('-') {
                                    config.triage_start_hour = range[..dash].parse().unwrap_or(8);
                                    config.triage_end_hour = range[dash + 1..].parse().unwrap_or(22);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    config
}

/// Apply settings update. Reads existing config, merges changes, writes all files.
pub fn save_settings(update: SettingsUpdate) -> Result<SettingsSaveResult, String> {
    let home = home_dir();
    let env_path = home.join("openclaw/docker.env");
    let env = parse_env_file(&env_path)?;

    // Read existing values to preserve unchanged fields
    let existing = read_current_config()?;
    let mut restart_required = false;

    // Determine final values by merging update with existing
    let agent_name = update.agent_name.clone().unwrap_or(existing.agent_name.clone());
    if update.agent_name.is_some() && update.agent_name.as_deref() != Some(&existing.agent_name) {
        restart_required = true;
    }

    // API keys: None = preserve, Some("") = clear, Some(val) = new
    let anthropic_key = match &update.anthropic_key {
        Some(k) => { restart_required = true; k.clone() }
        None => env.get("ANTHROPIC_API_KEY").cloned().unwrap_or_default(),
    };
    let openai_key = match &update.openai_key {
        Some(k) if !k.is_empty() => { restart_required = true; Some(k.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("OPENAI_API_KEY").filter(|v| !v.is_empty()).cloned(),
    };
    let venice_key = match &update.venice_key {
        Some(k) if !k.is_empty() => { restart_required = true; Some(k.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("VENICE_API_KEY").filter(|v| !v.is_empty()).cloned(),
    };
    let nearai_key = match &update.nearai_key {
        Some(k) if !k.is_empty() => { restart_required = true; Some(k.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("NEARAI_API_KEY").filter(|v| !v.is_empty()).cloned(),
    };
    let telegram_token = match &update.telegram_token {
        Some(t) if !t.is_empty() => { restart_required = true; Some(t.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("TELEGRAM_BOT_TOKEN").filter(|v| !v.is_empty()).cloned(),
    };
    let slack_token = match &update.slack_token {
        Some(t) if !t.is_empty() => { restart_required = true; Some(t.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("SLACK_BOT_TOKEN").filter(|v| !v.is_empty()).cloned(),
    };
    let whatsapp_phone = match &update.whatsapp_phone {
        Some(p) if !p.is_empty() => Some(p.clone()),
        Some(_) => None,
        None => existing.whatsapp_phone.clone(),
    };

    let guardrails = update.guardrails.clone().unwrap_or(existing.guardrails.clone());
    if update.guardrails.is_some() { restart_required = true; }

    let messaging = update.messaging.clone().unwrap_or(existing.messaging.clone());
    if update.messaging.is_some() { restart_required = true; }

    let email_notifications = update.email_notifications.clone()
        .unwrap_or(existing.email_notifications.clone());

    let capabilities = update.capabilities.clone().unwrap_or(existing.capabilities.clone());
    if update.capabilities.is_some() { restart_required = true; }

    // Preserve gateway token from existing env
    let gateway_token = env.get("OPENCLAW_GATEWAY_TOKEN")
        .cloned()
        .unwrap_or_else(generate_token);

    // Reconstruct wallets from existing env
    let wallet_count: usize = env.get("WALLET_COUNT")
        .and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut wallets = Vec::new();
    for i in 0..wallet_count {
        let chain_str = env.get(&format!("WALLET_{}_CHAIN", i))
            .cloned().unwrap_or_default();
        let chain = match chain_str.as_str() {
            "near" => Chain::NEAR,
            "eth" => Chain::ETH,
            "sol" => Chain::SOL,
            "btc" => Chain::BTC,
            "zec" => Chain::ZEC,
            _ => Chain::NEAR,
        };
        wallets.push(WalletConfig {
            id: format!("wallet_{}", i),
            chain,
            address: env.get(&format!("WALLET_{}_ADDRESS", i)).cloned().unwrap_or_default(),
            label: env.get(&format!("WALLET_{}_LABEL", i)).cloned().unwrap_or_default(),
            has_private_key: true,
            is_active: env.get(&format!("WALLET_{}_ACTIVE", i))
                .map_or(false, |v| v == "true"),
        });
    }
    let active_wallet_id = env.get("ACTIVE_WALLET_ID").cloned();

    // Build full SetupConfig
    let setup_config = SetupConfig {
        agent_name: agent_name.clone(),
        anthropic_key,
        openai_key,
        venice_key,
        nearai_key,
        telegram_token,
        slack_token,
        whatsapp_phone,
        gateway_token,
        wallets,
        active_wallet_id,
        guardrails,
        messaging,
        google_authenticated: existing.google_authenticated,
        email_notifications,
        capabilities,
    };

    // Write all config files
    write_docker_env(&setup_config)?;
    write_openclaw_config(&setup_config)?;
    write_guardrails(&setup_config.guardrails)?;
    write_cron_jobs(&setup_config)?;

    // Update SOUL.md if agent name changed
    if update.agent_name.is_some() && update.agent_name.as_deref() != Some(&existing.agent_name) {
        let soul_path = home.join("openclaw/workspace/SOUL.md");
        if let Ok(soul_content) = fs::read_to_string(&soul_path) {
            let updated_soul = soul_content.replace(
                &format!("You're {}", existing.agent_name),
                &format!("You're {}", agent_name),
            );
            let _ = fs::write(&soul_path, updated_soul);
        }
    }

    Ok(SettingsSaveResult {
        success: true,
        restart_required,
        message: if restart_required {
            "Settings saved. Container restart required for changes to take effect.".to_string()
        } else {
            "Settings saved.".to_string()
        },
    })
}

// ---------------------------------------------------------------------------
// ZEC / NEAR address helpers (used by shield/unshield commands)
// ---------------------------------------------------------------------------

/// Get the configured ZEC wallet address from docker.env wallets.
pub fn get_zec_address() -> Option<String> {
    let home = home_dir();
    let env_path = home.join("openclaw/docker.env");
    let env = parse_env_file(&env_path).ok()?;

    let wallet_count: usize = env.get("WALLET_COUNT")
        .and_then(|v| v.parse().ok()).unwrap_or(0);

    for i in 0..wallet_count {
        let chain = env.get(&format!("WALLET_{}_CHAIN", i)).cloned().unwrap_or_default();
        if chain == "zec" {
            if let Some(addr) = env.get(&format!("WALLET_{}_ADDRESS", i)) {
                if !addr.is_empty() {
                    return Some(addr.clone());
                }
            }
        }
    }
    None
}

/// Get the configured NEAR account ID from docker.env wallets.
pub fn get_near_account() -> Option<String> {
    let home = home_dir();
    let env_path = home.join("openclaw/docker.env");
    let env = parse_env_file(&env_path).ok()?;

    // Check for explicit NEAR_ACCOUNT_ID first
    if let Some(account) = env.get("NEAR_ACCOUNT_ID") {
        if !account.is_empty() {
            return Some(account.clone());
        }
    }

    // Fall back to first NEAR wallet address
    let wallet_count: usize = env.get("WALLET_COUNT")
        .and_then(|v| v.parse().ok()).unwrap_or(0);

    for i in 0..wallet_count {
        let chain = env.get(&format!("WALLET_{}_CHAIN", i)).cloned().unwrap_or_default();
        if chain == "near" {
            if let Some(addr) = env.get(&format!("WALLET_{}_ADDRESS", i)) {
                if !addr.is_empty() {
                    return Some(addr.clone());
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Directory creation
// ---------------------------------------------------------------------------

/// Create the full directory structure.
pub fn create_directories() -> Result<(), String> {
    let home = home_dir();
    let dirs = vec![
        home.join("openclaw/workspace"),
        home.join("openclaw/local-skills/near-intents"),
        home.join("openclaw/local-skills/gog"),
        home.join("openclaw/near-intents-helper"),
        home.join("openclaw/bin"),
        home.join("openclaw/patches/dist"),
        home.join(".openclaw/secrets"),
        home.join(".openclaw/cron"),
        home.join(".openclaw/defi-state/logs"),
        home.join(".openclaw/agents/default/sessions"),
    ];

    for dir in dirs {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    }

    // chmod 700 on secrets dir
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let secrets = home.join(".openclaw/secrets");
        fs::set_permissions(&secrets, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Failed to set secrets permissions: {}", e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// docker.env
// ---------------------------------------------------------------------------

/// Generate docker.env from config.
pub fn write_docker_env(config: &SetupConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join("openclaw/docker.env");

    let mut content = format!(
        "# Nyx Docker Environment\n\
         OPENCLAW_GATEWAY_TOKEN={}\n\
         OPENCLAW_IMAGE=ghcr.io/openclaw/openclaw:2026.2.9\n\
         ANTHROPIC_API_KEY={}\n",
        config.gateway_token, config.anthropic_key
    );

    if let Some(ref key) = config.openai_key {
        content.push_str(&format!("OPENAI_API_KEY={}\n", key));
    }
    if let Some(ref key) = config.venice_key {
        content.push_str(&format!("VENICE_API_KEY={}\n", key));
    }
    if let Some(ref key) = config.nearai_key {
        content.push_str(&format!("NEARAI_API_KEY={}\n", key));
    }
    if let Some(ref token) = config.telegram_token {
        content.push_str(&format!("TELEGRAM_BOT_TOKEN={}\n", token));
    }
    if let Some(ref token) = config.slack_token {
        content.push_str(&format!("SLACK_BOT_TOKEN={}\n", token));
    }

    // Wallet credentials — injected at container boundary, never mounted as files
    content.push_str(&format!(
        "\n# Wallet credentials (boundary injection)\nWALLET_COUNT={}\n",
        config.wallets.len()
    ));
    for (i, w) in config.wallets.iter().enumerate() {
        content.push_str(&format!("WALLET_{}_CHAIN={}\n", i, w.chain));
        content.push_str(&format!("WALLET_{}_ADDRESS={}\n", i, w.address));
        content.push_str(&format!("WALLET_{}_LABEL={}\n", i, w.label));
        content.push_str(&format!("WALLET_{}_ACTIVE={}\n", i, w.is_active));
    }
    if let Some(ref active_id) = config.active_wallet_id {
        content.push_str(&format!("ACTIVE_WALLET_ID={}\n", active_id));
    }

    // DeFi guardrails — from config (only if DeFi capability enabled)
    let caps = &config.capabilities;
    if caps.defi_crypto {
        let g = &config.guardrails;
        content.push_str(&format!(
            "\n# DeFi guardrails\n\
             MAX_SINGLE_TX_USD={}\n\
             DAILY_LOSS_LIMIT_PCT={}\n\
             WEEKLY_LOSS_LIMIT_PCT={}\n\
             MAX_CONCENTRATION_PCT={}\n\
             BURROW_MIN_HEALTH_FACTOR={}\n\
             MAX_SLIPPAGE_PCT={}\n\
             MAX_DAILY_TXS={}\n\
             REQUIRE_CONFIRMATION={}\n",
            g.max_transaction_usd,
            g.daily_loss_percent,
            g.weekly_loss_percent,
            g.max_concentration_percent,
            g.min_health_factor,
            g.max_slippage_percent,
            g.daily_tx_limit,
            g.require_confirmation,
        ));
    }

    // Messaging env vars
    let m = &config.messaging;
    content.push_str(&format!(
        "\n# Messaging\n\
         MESSAGING_GMAIL_ENABLED={}\n\
         MESSAGING_WHATSAPP_ENABLED={}\n\
         MESSAGING_TELEGRAM_ENABLED={}\n\
         MESSAGING_SLACK_ENABLED={}\n\
         GOOGLE_AUTHENTICATED={}\n\
         \n# Privacy\n\
         ZEC_PRIVACY_DEFAULT=true\n\
         CROSS_CHAIN_ENABLED=true\n\
         \n# Capabilities\n\
         CAPABILITY_DEFI={}\n\
         CAPABILITY_TRAVEL={}\n\
         CAPABILITY_GOOGLE={}\n\
         CAPABILITY_EMAIL_INTEL={}\n\
         CAPABILITY_COMMS={}\n\
         CAPABILITY_SOURCE_INTEL={}\n\
         DEFAULT_LLM_PROVIDER={}\n\
         OLLAMA_MODEL={}\n",
        m.gmail.enabled,
        m.whatsapp.enabled,
        m.telegram.enabled,
        m.slack.enabled,
        config.google_authenticated,
        caps.defi_crypto,
        caps.travel,
        caps.google_workspace,
        caps.email_intelligence,
        caps.communications,
        caps.source_intelligence,
        caps.default_llm_provider,
        caps.ollama_model.as_deref().unwrap_or(""),
    ));

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write docker.env: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set docker.env permissions: {}", e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// openclaw.json
// ---------------------------------------------------------------------------

/// Generate openclaw.json.
pub fn write_openclaw_config(config: &SetupConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".openclaw/openclaw.json");

    let has_telegram = config.telegram_token.is_some();
    let has_slack = config.slack_token.is_some();
    let has_openai = config.openai_key.is_some();
    let has_venice = config.venice_key.is_some();
    let has_nearai = config.nearai_key.is_some();
    let caps = &config.capabilities;
    let default_provider = &caps.default_llm_provider;

    // WhatsApp allowFrom: populate with user's phone if provided
    let whatsapp_allow_from = match &config.whatsapp_phone {
        Some(phone) => json!([phone]),
        None => json!([]),
    };

    let mut channels = json!({
        "whatsapp": {
            "sendReadReceipts": true,
            "dmPolicy": "allowlist",
            "responsePrefix": "\u{1f310}",
            "allowFrom": whatsapp_allow_from,
            "groupPolicy": "disabled",
            "textChunkLimit": 4000,
            "mediaMaxMb": 50,
            "debounceMs": 0
        }
    });

    if has_telegram {
        channels["telegram"] = json!({
            "dmPolicy": "pairing",
            "groupPolicy": "disabled",
            "textChunkLimit": 4000,
            "streamMode": "partial"
        });
    }

    if has_slack {
        channels["slack"] = json!({
            "dmPolicy": "allowlist",
            "groupPolicy": "disabled",
            "textChunkLimit": 4000
        });
    }

    let mut plugins = json!({
        "whatsapp": { "enabled": true }
    });
    if has_telegram {
        plugins["telegram"] = json!({ "enabled": true });
    }
    if has_slack {
        plugins["slack"] = json!({ "enabled": true });
    }

    let mut tts = json!({});
    if has_openai {
        tts = json!({
            "auto": "inbound",
            "provider": "openai",
            "openai": {
                "model": "gpt-4o-mini-tts",
                "voice": "alloy"
            }
        });
    }

    // Build dynamic lists based on capabilities
    let mut safe_bins: Vec<&str> = vec!["ls", "find", "wc", "date", "openclaw", "curl", "cat", "grep", "head", "tail"];
    if caps.google_workspace {
        safe_bins.push("gog");
    }
    if caps.defi_crypto {
        safe_bins.push("/opt/near-intents-helper/run_near_intents.sh");
    }

    // ClawdTalk voice — add jq + shell scripts to safeBins when configured
    if caps.communications {
        let clawdtalk_config = home.join("openclaw/local-skills/clawdtalk/skill-config.json");
        if clawdtalk_config.exists() {
            safe_bins.push("jq");
            safe_bins.push("/home/node/.openclaw/local-skills/clawdtalk/scripts/connect.sh");
            safe_bins.push("/home/node/.openclaw/local-skills/clawdtalk/scripts/call.sh");
            safe_bins.push("/home/node/.openclaw/local-skills/clawdtalk/scripts/sms.sh");
            safe_bins.push("/home/node/.openclaw/local-skills/clawdtalk/scripts/missions.sh");
        }
    }

    let mut allow_bundled: Vec<&str> = vec![];
    if caps.google_workspace {
        allow_bundled.push("gog");
    }

    let mut skill_entries = serde_json::Map::new();
    if caps.defi_crypto {
        skill_entries.insert("near-intents".to_string(), json!({ "enabled": true }));
    }
    if caps.travel {
        skill_entries.insert("travel".to_string(), json!({ "enabled": true }));
    }
    if caps.communications {
        let clawdtalk_config = home.join("openclaw/local-skills/clawdtalk/skill-config.json");
        if clawdtalk_config.exists() {
            skill_entries.insert("clawdtalk-client".to_string(), json!({ "enabled": true }));
        }
    }

    // Build LLM provider configuration
    let mut providers = serde_json::Map::new();
    providers.insert("anthropic".to_string(), json!({
        "enabled": true,
        "model": "claude-sonnet-4-20250514"
    }));
    if has_venice {
        providers.insert("venice".to_string(), json!({
            "enabled": true,
            "baseUrl": "https://api.venice.ai/api/v1",
            "model": "llama-3.3-70b"
        }));
    }
    if has_openai {
        providers.insert("openai".to_string(), json!({
            "enabled": true,
            "model": "gpt-4o"
        }));
    }
    if has_nearai {
        providers.insert("nearai".to_string(), json!({
            "enabled": true,
            "baseUrl": "https://cloud-api.near.ai/v1",
            "model": "qwen3-30b-a3b"
        }));
    }

    let config_json = json!({
        "agents": {
            "defaults": {
                "workspace": "/home/node/.openclaw/workspace",
                "maxConcurrent": 2,
                "subagents": { "maxConcurrent": 4 },
                "sandbox": { "mode": "off" }
            },
            "list": [{
                "id": "default",
                "default": true,
                "workspace": "/home/node/.openclaw/workspace",
                "identity": {
                    "name": &config.agent_name,
                    "theme": "private AI chief of staff",
                    "emoji": "\u{1f3db}\u{fe0f}"
                }
            }]
        },
        "llm": {
            "default": default_provider,
            "providers": providers
        },
        "tools": {
            "profile": "coding",
            "deny": ["group:ui"],
            "media": { "audio": { "enabled": has_openai } },
            "exec": {
                "host": "gateway",
                "ask": "off",
                "security": "full",
                "safeBins": safe_bins
            }
        },
        "logging": {
            "level": "info",
            "consoleLevel": "info",
            "redactSensitive": "tools",
            "redactPatterns": [
                "sk-ant-[A-Za-z0-9_\\-]+",
                "sk-proj-[A-Za-z0-9_\\-]+",
                "ed25519:[A-Za-z0-9]{40,}",
                "AKIA[0-9A-Z]{16}",
                "ghp_[A-Za-z0-9]{36}",
                "[0-9a-f]{64}"
            ]
        },
        "messages": {
            "ackReactionScope": "group-mentions",
            "tts": tts
        },
        "commands": { "native": "auto", "nativeSkills": "auto" },
        "session": {},
        "cron": { "enabled": true, "maxConcurrentRuns": 1 },
        "channels": channels,
        "gateway": {
            "port": 18789,
            "mode": "local",
            "bind": "lan",
            "auth": {
                "mode": "token",
                "token": &config.gateway_token
            },
            "http": {
                "endpoints": {
                    "chatCompletions": { "enabled": true }
                }
            },
            "tailscale": { "mode": "off", "resetOnExit": false },
            "controlUi": { "enabled": false }
        },
        "skills": {
            "allowBundled": allow_bundled,
            "load": {
                "extraDirs": ["/home/node/.openclaw/local-skills"],
                "watch": false
            },
            "entries": skill_entries
        },
        "plugins": { "entries": plugins }
    });

    let content = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write openclaw.json: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Guardrails
// ---------------------------------------------------------------------------

/// Write guardrails config from the provided GuardrailsConfig.
pub fn write_guardrails(guardrails: &GuardrailsConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".openclaw/secrets/defi_guardrails.env");

    let content = format!(
        "# Nyx DeFi Guardrails\n\
         MAX_TX_USD={}\n\
         DAILY_LOSS_PCT={}\n\
         WEEKLY_LOSS_PCT={}\n\
         MAX_CONCENTRATION_PCT={}\n\
         BURROW_MIN_HEALTH={}\n\
         MAX_SLIPPAGE_PCT={}\n\
         MAX_DAILY_TXS={}\n\
         REQUIRE_CONFIRMATION={}\n",
        guardrails.max_transaction_usd,
        guardrails.daily_loss_percent,
        guardrails.weekly_loss_percent,
        guardrails.max_concentration_percent,
        guardrails.min_health_factor,
        guardrails.max_slippage_percent,
        guardrails.daily_tx_limit,
        guardrails.require_confirmation,
    );

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write guardrails: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set guardrails permissions: {}", e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Cron jobs
// ---------------------------------------------------------------------------

/// Write cron jobs. Email schedules are user-configurable (timezone, hours).
pub fn write_cron_jobs(config: &SetupConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".openclaw/cron/jobs.json");

    let e = &config.email_notifications;
    let caps = &config.capabilities;
    let email_enabled = config.google_authenticated && e.enabled && caps.email_intelligence;
    let defi_enabled = caps.defi_crypto;
    let tz = &e.timezone;

    // Determine delivery channel: priority WhatsApp > Telegram > Slack > gateway
    let delivery_channel = if config.messaging.whatsapp.enabled {
        "whatsapp"
    } else if config.messaging.telegram.enabled {
        "telegram"
    } else if config.messaging.slack.enabled {
        "slack"
    } else {
        "gateway"
    };

    // Build cron expressions from user preferences
    let triage_cron = format!("0 {}-{} * * *", e.triage_start_hour, e.triage_end_hour);
    let digest_cron = format!("{} {} * * *", e.digest_minute, e.digest_hour);

    let jobs = json!([
        {
            "id": "nyx-heartbeat",
            "name": format!("{} Heartbeat", &config.agent_name),
            "schedule": { "intervalMs": 14400000 },
            "prompt": "/opt/near-intents-helper/run_near_intents.sh heartbeat --risk medium",
            "delivery": { "channel": delivery_channel },
            "enabled": defi_enabled
        },
        {
            "id": "daily-defi-report",
            "name": "Daily DeFi Report",
            "schedule": { "cron": "0 9 * * *", "timezone": tz },
            "prompt": "/opt/near-intents-helper/run_near_intents.sh daily-report",
            "delivery": { "channel": delivery_channel },
            "enabled": defi_enabled
        },
        {
            "id": "hourly-email-triage",
            "name": "Hourly Email Triage",
            "schedule": { "cron": triage_cron, "timezone": tz },
            "prompt": "Quick email triage across all gog accounts. Search for unread emails in the last hour. Only message me if something is 🔴 URGENT.",
            "delivery": { "channel": delivery_channel },
            "enabled": email_enabled
        },
        {
            "id": "daily-email-digest",
            "name": "Daily Email Digest",
            "schedule": { "cron": digest_cron, "timezone": tz },
            "prompt": "Generate daily email digest across all gog accounts. Last 24h summary grouped by priority. Always send this morning briefing.",
            "delivery": { "channel": delivery_channel },
            "enabled": email_enabled
        }
    ]);

    let content = serde_json::to_string_pretty(&jobs)
        .map_err(|e| format!("Failed to serialize cron jobs: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write cron jobs: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Resource copying
// ---------------------------------------------------------------------------

/// Copy bundled resources to user directories.
pub fn copy_resources(resources_dir: &Path) -> Result<(), String> {
    let home = home_dir();

    // Copy workspace files
    copy_dir_contents(
        &resources_dir.join("workspace"),
        &home.join("openclaw/workspace"),
    )?;

    // Copy skills
    copy_dir_contents(
        &resources_dir.join("local-skills"),
        &home.join("openclaw/local-skills"),
    )?;

    // Copy Python modules
    copy_dir_contents(
        &resources_dir.join("near-intents-helper"),
        &home.join("openclaw/near-intents-helper"),
    )?;

    // Copy gog binary
    let gog_src = resources_dir.join("bin/gog");
    let gog_dst = home.join("openclaw/bin/gog");
    if gog_src.exists() {
        fs::copy(&gog_src, &gog_dst)
            .map_err(|e| format!("Failed to copy gog: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&gog_dst, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set gog permissions: {}", e))?;
        }
    }

    // Copy jq binary (needed by ClawdTalk shell scripts inside Docker container)
    let jq_src = resources_dir.join("bin/jq");
    let jq_dst = home.join("openclaw/bin/jq");
    if jq_src.exists() {
        fs::copy(&jq_src, &jq_dst)
            .map_err(|e| format!("Failed to copy jq: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&jq_dst, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set jq permissions: {}", e))?;
        }
    }

    // Copy OpenClaw runtime patches (e.g. billing error false-positive fix)
    copy_dir_contents(
        &resources_dir.join("patches"),
        &home.join("openclaw/patches"),
    )?;

    // Copy docker-compose.yml
    let compose_src = resources_dir.join("docker-compose.yml");
    let compose_dst = home.join("openclaw/docker-compose.yml");
    if compose_src.exists() {
        fs::copy(&compose_src, &compose_dst)
            .map_err(|e| format!("Failed to copy docker-compose.yml: {}", e))?;
    }

    // Copy start script (invoked by LaunchAgent at login)
    let start_src = resources_dir.join("start-nyx.sh");
    let start_dst = home.join("openclaw/start-nyx.sh");
    if start_src.exists() {
        fs::copy(&start_src, &start_dst)
            .map_err(|e| format!("Failed to copy start-nyx.sh: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&start_dst, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set start-nyx.sh permissions: {}", e))?;
        }
    }

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {}: {}", src_path.display(), e))?;
        }
    }

    Ok(())
}
