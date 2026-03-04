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
    #[serde(default)]
    pub signal: ChannelConfig,
}

impl Default for MessagingConfig {
    fn default() -> Self {
        MessagingConfig {
            gmail: ChannelConfig::default(),
            whatsapp: ChannelConfig::default(),
            telegram: ChannelConfig::default(),
            slack: ChannelConfig::default(),
            signal: ChannelConfig::default(),
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
    /// Observe calendar + email patterns to offer proactive suggestions
    #[serde(default)]
    pub activity_intelligence: bool,
    /// Agent-controlled web browsing (navigate sites, fill forms, book travel)
    #[serde(default = "default_true")]
    pub web_browsing: bool,
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
            activity_intelligence: false, // opt-in — requires explicit enable
            web_browsing: true, // on by default
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
    pub perplexity_key: Option<String>,
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
    pub has_perplexity_key: bool,
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
    pub perplexity_key: Option<String>,
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

/// Serde default helper for fields that default to true.
fn default_true() -> bool {
    true
}

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

    // Parse .env
    let env_path = home.join(".nyx/.env");
    let env = parse_env_file(&env_path)?;

    // Read agent name from config.toml
    let config_path = home.join(".nyx/config.toml");
    let agent_name = if let Ok(content) = fs::read_to_string(&config_path) {
        content.lines()
            .find(|l| l.trim().starts_with("name") && l.contains('='))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Nyx".to_string())
    } else {
        "Nyx".to_string()
    };

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

    let parse_autonomy = |k: &str| -> MessagingAutonomy {
        match env.get(k).map(|s| s.as_str()) {
            Some("SendWithConfirm") | Some("send_with_confirm") => MessagingAutonomy::SendWithConfirm,
            Some("Autonomous") | Some("autonomous") => MessagingAutonomy::Autonomous,
            _ => MessagingAutonomy::DraftOnly,
        }
    };

    let messaging = MessagingConfig {
        gmail: ChannelConfig {
            enabled: parse_bool("MESSAGING_GMAIL_ENABLED"),
            autonomy: parse_autonomy("MESSAGING_GMAIL_AUTONOMY"),
        },
        whatsapp: ChannelConfig {
            enabled: parse_bool("MESSAGING_WHATSAPP_ENABLED"),
            autonomy: parse_autonomy("MESSAGING_WHATSAPP_AUTONOMY"),
        },
        telegram: ChannelConfig {
            enabled: parse_bool("MESSAGING_TELEGRAM_ENABLED"),
            autonomy: parse_autonomy("MESSAGING_TELEGRAM_AUTONOMY"),
        },
        slack: ChannelConfig {
            enabled: parse_bool("MESSAGING_SLACK_ENABLED"),
            autonomy: parse_autonomy("MESSAGING_SLACK_AUTONOMY"),
        },
        signal: ChannelConfig {
            enabled: parse_bool("MESSAGING_SIGNAL_ENABLED"),
            autonomy: parse_autonomy("MESSAGING_SIGNAL_AUTONOMY"),
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
        activity_intelligence: env
            .get("CAPABILITY_ACTIVITY_INTEL")
            .map(|v| v == "true")
            .unwrap_or(false),
        web_browsing: env
            .get("CAPABILITY_WEB_BROWSING")
            .map(|v| v == "true")
            .unwrap_or(true),
        default_llm_provider: default_llm_provider.clone(),
        ollama_model: env.get("OLLAMA_MODEL")
            .filter(|v| !v.is_empty())
            .cloned(),
    };

    // WhatsApp phone from env
    let whatsapp_phone = env.get("WHATSAPP_PHONE")
        .filter(|v| !v.is_empty())
        .cloned();

    Ok(SettingsConfig {
        agent_name,
        has_anthropic_key: has_key("ANTHROPIC_API_KEY"),
        has_openai_key: has_key("OPENAI_API_KEY"),
        has_venice_key: has_key("VENICE_API_KEY"),
        has_nearai_key: has_key("NEARAI_API_KEY"),
        has_perplexity_key: has_key("PERPLEXITY_API_KEY"),
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
    let cron_path = home.join(".nyx/cron/jobs.json");
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
    let env_path = home.join(".nyx/.env");
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
    let perplexity_key = match &update.perplexity_key {
        Some(k) if !k.is_empty() => { restart_required = true; Some(k.clone()) }
        Some(_) => { restart_required = true; None }
        None => env.get("PERPLEXITY_API_KEY").filter(|v| !v.is_empty()).cloned(),
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
    let gateway_token = env.get("GATEWAY_AUTH_TOKEN")
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
        perplexity_key,
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
    write_nyx_env(&setup_config)?;
    write_ironclaw_config(&setup_config)?;
    write_guardrails(&setup_config.guardrails)?;
    write_cron_jobs(&setup_config)?;

    // Update SOUL.md if agent name changed
    if update.agent_name.is_some() && update.agent_name.as_deref() != Some(&existing.agent_name) {
        let soul_path = home.join(".nyx/workspace/SOUL.md");
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
            "Settings saved. Daemon restart required for changes to take effect.".to_string()
        } else {
            "Settings saved.".to_string()
        },
    })
}

// ---------------------------------------------------------------------------
// ZEC / NEAR address helpers (used by shield/unshield commands)
// ---------------------------------------------------------------------------

/// Get the configured ZEC wallet address from .env wallets.
pub fn get_zec_address() -> Option<String> {
    let home = home_dir();
    let env_path = home.join(".nyx/.env");
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

/// Get the configured NEAR account ID from .env wallets.
pub fn get_near_account() -> Option<String> {
    let home = home_dir();
    let env_path = home.join(".nyx/.env");
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
        home.join(".nyx/workspace"),
        home.join(".nyx/local-skills/near-intents"),
        home.join(".nyx/local-skills/gog"),
        home.join(".nyx/near-intents-helper"),
        home.join(".nyx/bin"),
        home.join(".nyx/secrets"),
        home.join(".nyx/cron"),
        home.join(".nyx/gogcli"),
        home.join(".nyx/defi-state/logs"),
        home.join(".nyx/logs"),
    ];

    for dir in dirs {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
    }

    // chmod 700 on secrets dir
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let secrets = home.join(".nyx/secrets");
        fs::set_permissions(&secrets, fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("Failed to set secrets permissions: {}", e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// .env (IronClaw environment)
// ---------------------------------------------------------------------------

/// Generate .env from config.
pub fn write_nyx_env(config: &SetupConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".nyx/.env");

    // Determine gateway port: preserve existing value from .env, or pick an
    // available port (avoids collision with other IronClaw instances like Atlas).
    let gateway_port = {
        let existing = if let Ok(contents) = fs::read_to_string(&path) {
            contents.lines()
                .find(|l| l.starts_with("GATEWAY_PORT="))
                .and_then(|l| l.strip_prefix("GATEWAY_PORT="))
                .and_then(|v| v.trim().parse::<u16>().ok())
        } else {
            None
        };
        existing.unwrap_or_else(crate::ironclaw::pick_available_port)
    };

    let mut content = format!(
        "# Nyx IronClaw Environment\n\
         GATEWAY_AUTH_TOKEN={}\n\
         ANTHROPIC_API_KEY={}\n\
         LLM_BACKEND=anthropic\n\
         DATABASE_BACKEND=libsql\n\
         LIBSQL_PATH={}/.nyx/ironclaw.db\n\
         GATEWAY_PORT={}\n",
        config.gateway_token, config.anthropic_key, home.display(), gateway_port
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
    if let Some(ref key) = config.perplexity_key {
        content.push_str(&format!("PERPLEXITY_API_KEY={}\n", key));
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

    // NEAR credentials — injected as env vars (IronClaw boundary injection)
    // Find the active NEAR wallet and write account ID + private key path
    let active_near = config.wallets.iter()
        .find(|w| matches!(w.chain, Chain::NEAR) && w.is_active);
    if let Some(near_wallet) = active_near {
        content.push_str(&format!(
            "\n# NEAR credentials (boundary injection)\n\
             NEAR_ACCOUNT_ID={}\n\
             NEAR_NETWORK_ID=mainnet\n\
             SOLVER_RELAY_URL=https://solver-relay.near.org\n",
            near_wallet.address
        ));
    }

    // WhatsApp phone
    if let Some(ref phone) = config.whatsapp_phone {
        content.push_str(&format!("\n# WhatsApp\nWHATSAPP_PHONE={}\n", phone));
    }

    // Messaging env vars (with autonomy)
    let m = &config.messaging;
    content.push_str(&format!(
        "\n# Messaging\n\
         MESSAGING_GMAIL_ENABLED={}\n\
         MESSAGING_GMAIL_AUTONOMY={:?}\n\
         MESSAGING_WHATSAPP_ENABLED={}\n\
         MESSAGING_WHATSAPP_AUTONOMY={:?}\n\
         MESSAGING_TELEGRAM_ENABLED={}\n\
         MESSAGING_TELEGRAM_AUTONOMY={:?}\n\
         MESSAGING_SLACK_ENABLED={}\n\
         MESSAGING_SLACK_AUTONOMY={:?}\n\
         MESSAGING_SIGNAL_ENABLED={}\n\
         MESSAGING_SIGNAL_AUTONOMY={:?}\n\
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
         CAPABILITY_ACTIVITY_INTEL={}\n\
         CAPABILITY_WEB_BROWSING={}\n\
         DEFAULT_LLM_PROVIDER={}\n\
         OLLAMA_MODEL={}\n",
        m.gmail.enabled, m.gmail.autonomy,
        m.whatsapp.enabled, m.whatsapp.autonomy,
        m.telegram.enabled, m.telegram.autonomy,
        m.slack.enabled, m.slack.autonomy,
        m.signal.enabled, m.signal.autonomy,
        config.google_authenticated,
        caps.defi_crypto,
        caps.travel,
        caps.google_workspace,
        caps.email_intelligence,
        caps.communications,
        caps.source_intelligence,
        caps.activity_intelligence,
        caps.web_browsing,
        caps.default_llm_provider,
        caps.ollama_model.as_deref().unwrap_or(""),
    ));

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write .env: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set .env permissions: {}", e))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// config.toml (IronClaw daemon configuration)
// ---------------------------------------------------------------------------

/// Generate config.toml for the IronClaw daemon.
pub fn write_ironclaw_config(config: &SetupConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".nyx/config.toml");
    let caps = &config.capabilities;

    // Resolve default model
    let default_model = match caps.default_llm_provider.as_str() {
        "openai" => "gpt-4o",
        "venice" => "llama-3.3-70b",
        "nearai" => "qwen3-30b-a3b",
        "ollama" => caps.ollama_model.as_deref().unwrap_or("qwen3:4b"),
        _ => "claude-sonnet-4-20250514",
    };

    // Determine gateway port (avoid collision with other IronClaw instances)
    let gateway_port = crate::ironclaw::gateway_port();

    let content = format!(
        r#"# Nyx IronClaw Configuration
# Generated by Nyx setup wizard

onboard_completed = true
selected_model = "{model}"

[embeddings]
enabled = false

[tunnel]
ts_funnel = false

[channels]
http_enabled = true
http_host = "127.0.0.1"
http_port = {port}
signal_enabled = false
wasm_channels = []
wasm_channels_enabled = false

[heartbeat]
enabled = true
interval_secs = 1800

[agent]
name = "{name}"
max_parallel_jobs = 3
job_timeout_secs = 3600
stuck_threshold_secs = 300
use_planning = true
repair_check_interval_secs = 60
max_repair_attempts = 3
session_idle_timeout_secs = 604800
max_tool_iterations = 50
auto_approve_tools = true

[safety]
max_output_length = 100000
injection_check_enabled = true
"#,
        model = default_model,
        port = gateway_port,
        name = config.agent_name,
    );

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write config.toml: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Guardrails
// ---------------------------------------------------------------------------

/// Write guardrails config from the provided GuardrailsConfig.
pub fn write_guardrails(guardrails: &GuardrailsConfig) -> Result<(), String> {
    let home = home_dir();
    let path = home.join(".nyx/secrets/defi_guardrails.env");

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
    let path = home.join(".nyx/cron/jobs.json");

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

    let jobs = json!({
        "version": 2,
        "jobs": [
            {
                "id": "nyx-heartbeat",
                "agentId": "default",
                "name": format!("{} Heartbeat", &config.agent_name),
                "schedule": { "kind": "every", "everyMs": 14400000 },
                "sessionTarget": "isolated",
                "payload": {
                    "kind": "agentTurn",
                    "message": format!(
                        "Run the {} DeFi heartbeat check. Use the near-intents skill to execute: \
                        /opt/near-intents-helper/run_near_intents.sh heartbeat --risk medium\n\n\
                        If any actions were taken or errors occurred, send me a brief summary on {}. \
                        If everything is stable and no actions were needed, stay silent (don't message me).",
                        &config.agent_name, delivery_channel
                    )
                },
                "state": { "nextRunAtMs": null },
                "enabled": defi_enabled,
                "delivery": { "mode": "none" }
            },
            {
                "id": "daily-defi-report",
                "agentId": "default",
                "name": "Daily DeFi Report",
                "schedule": { "kind": "cron", "expr": "0 9 * * *", "tz": tz },
                "sessionTarget": "isolated",
                "payload": {
                    "kind": "agentTurn",
                    "message": format!(
                        "Generate and send the daily DeFi report. Use the near-intents skill to execute: \
                        /opt/near-intents-helper/run_near_intents.sh daily-report\n\n\
                        Format the results into a clear, concise message with:\n\
                        - Portfolio value and daily P&L\n\
                        - Any active positions (staking, lending)\n\
                        - Top yield opportunities\n\
                        - Trading status (active or halted)\n\
                        - Any alerts or warnings\n\n\
                        Keep it brief and readable on a phone screen. Send via {}.",
                        delivery_channel
                    )
                },
                "state": { "nextRunAtMs": null },
                "enabled": defi_enabled,
                "delivery": { "mode": "none" }
            },
            {
                "id": "hourly-email-triage",
                "agentId": "default",
                "name": "Hourly Email Triage",
                "schedule": { "kind": "cron", "expr": triage_cron, "tz": tz },
                "sessionTarget": "isolated",
                "payload": {
                    "kind": "agentTurn",
                    "message": "Quick email triage \u{2014} scan for high-priority emails across all configured accounts.\n\n\
                        Use `gog accounts list` to discover all configured accounts, then for EACH account run:\n  \
                        gog gmail search 'newer_than:1h is:unread' --max 20 --account <account>\n\n\
                        Classify each unread email as:\n\
                        - URGENT \u{2014} needs immediate attention (time-sensitive, from key contacts, financial, legal, security)\n\
                        - IMPORTANT \u{2014} should be addressed today (client comms, scheduled items, action required)\n\
                        - NORMAL \u{2014} can wait (newsletters, notifications, FYI)\n\n\
                        ONLY message me if there are URGENT emails. Include:\n\
                        - Sender, subject, one-line summary\n\
                        - Why it's urgent\n\
                        - Suggested action\n\n\
                        If nothing urgent, stay silent. Do NOT send a message saying 'nothing urgent'."
                },
                "state": { "nextRunAtMs": null },
                "enabled": email_enabled,
                "delivery": { "mode": "none" }
            },
            {
                "id": "daily-email-digest",
                "agentId": "default",
                "name": "Daily Email Digest",
                "schedule": { "kind": "cron", "expr": digest_cron, "tz": tz },
                "sessionTarget": "isolated",
                "payload": {
                    "kind": "agentTurn",
                    "message": "Generate the daily email digest \u{2014} a comprehensive inbox summary across all accounts.\n\n\
                        Use `gog accounts list` to discover all configured accounts, then for EACH account run:\n  \
                        gog gmail search 'newer_than:24h' --max 50 --account <account>\n\n\
                        Produce a concise digest grouped by priority:\n\n\
                        URGENT (needs action NOW)\n\
                        - [sender] subject \u{2014} one-line summary + suggested action\n\n\
                        IMPORTANT (action today)\n\
                        - [sender] subject \u{2014} one-line summary\n\n\
                        STATS\n\
                        - Total new emails (per account)\n\
                        - Unread count\n\
                        - Threads awaiting your reply\n\n\
                        Skip newsletters, automated notifications, and low-priority items unless there's an unusual spike. \
                        Keep it scannable on a phone screen.\n\n\
                        Always send this digest even if nothing urgent \u{2014} it's the morning briefing."
                },
                "state": { "nextRunAtMs": null },
                "enabled": email_enabled,
                "delivery": { "mode": "none" }
            }
        ]
    });

    let content = serde_json::to_string_pretty(&jobs)
        .map_err(|e| format!("Failed to serialize cron jobs: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write cron jobs: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Cron job types and CRUD
// ---------------------------------------------------------------------------

/// Represents the on-disk cron jobs file (version 2 format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobsFile {
    pub version: u32,
    pub jobs: Vec<CronJob>,
}

/// A single cron job entry. Uses `flatten` to preserve any extra fields
/// written by the gateway (updatedAtMs, lastRunAtMs, etc.) during round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJob {
    pub id: String,
    pub agent_id: String,
    pub name: String,
    pub schedule: CronSchedule,
    pub session_target: String,
    pub payload: CronPayload,
    #[serde(default = "default_cron_state")]
    pub state: serde_json::Value,
    pub enabled: bool,
    #[serde(default = "default_cron_delivery")]
    pub delivery: serde_json::Value,
    /// Preserve extra fields written by the gateway (updatedAtMs, etc.)
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_cron_state() -> serde_json::Value {
    json!({ "nextRunAtMs": null })
}

fn default_cron_delivery() -> serde_json::Value {
    json!({ "mode": "none" })
}

/// Cron schedule — either a fixed interval or a cron expression.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CronSchedule {
    #[serde(rename = "every")]
    Every {
        #[serde(rename = "everyMs")]
        every_ms: u64,
        /// Anchor timestamp (set by gateway, preserved on round-trip)
        #[serde(skip_serializing_if = "Option::is_none", rename = "anchorMs")]
        anchor_ms: Option<u64>,
    },
    #[serde(rename = "cron")]
    Cron {
        expr: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tz: Option<String>,
    },
}

/// Cron payload — the action to execute when the job fires.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum CronPayload {
    #[serde(rename = "agentTurn")]
    AgentTurn { message: String },
}

/// Partial update for a cron job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronJobUpdate {
    pub name: Option<String>,
    pub schedule: Option<CronSchedule>,
    pub message: Option<String>,
    pub enabled: Option<bool>,
}

/// Built-in job IDs that cannot be deleted (only disabled).
/// Covers both Nyx and Atlas naming conventions.
const BUILTIN_JOB_IDS: &[&str] = &[
    "nyx-heartbeat",
    "atlas-defi-heartbeat",
    "daily-defi-report",
    "hourly-email-triage",
    "daily-email-digest",
    "proactive-intelligence",
];

fn cron_jobs_path() -> PathBuf {
    home_dir().join(".nyx/cron/jobs.json")
}

/// Read all cron jobs from disk.
pub fn read_cron_jobs() -> Result<CronJobsFile, String> {
    let path = cron_jobs_path();
    if !path.exists() {
        return Ok(CronJobsFile {
            version: 2,
            jobs: vec![],
        });
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read cron jobs: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse cron jobs: {}", e))
}

/// Write the full cron jobs file back to disk.
fn write_cron_jobs_file(file: &CronJobsFile) -> Result<(), String> {
    let path = cron_jobs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create cron directory: {}", e))?;
    }
    let content = serde_json::to_string_pretty(file)
        .map_err(|e| format!("Failed to serialize cron jobs: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("Failed to write cron jobs: {}", e))
}

/// Create a new cron job and persist to disk.
pub fn create_cron_job(
    name: String,
    schedule: CronSchedule,
    message: String,
    enabled: bool,
) -> Result<CronJob, String> {
    let mut file = read_cron_jobs()?;

    // Generate a unique ID from the name
    let base_id = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>();
    let id = if file.jobs.iter().any(|j| j.id == base_id) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() % 100000)
            .unwrap_or(0);
        format!("{}-{}", base_id, ts)
    } else {
        base_id
    };

    // Reject duplicate IDs
    if file.jobs.iter().any(|j| j.id == id) {
        return Err(format!("A job with ID '{}' already exists", id));
    }

    let job = CronJob {
        id,
        agent_id: "default".to_string(),
        name,
        schedule,
        session_target: "isolated".to_string(),
        payload: CronPayload::AgentTurn { message },
        state: default_cron_state(),
        enabled,
        delivery: default_cron_delivery(),
        extra: serde_json::Map::new(),
    };

    file.jobs.push(job.clone());
    write_cron_jobs_file(&file)?;
    Ok(job)
}

/// Update an existing cron job by ID. Returns the updated job.
pub fn update_cron_job(id: &str, updates: CronJobUpdate) -> Result<CronJob, String> {
    let mut file = read_cron_jobs()?;

    let job = file
        .jobs
        .iter_mut()
        .find(|j| j.id == id)
        .ok_or_else(|| format!("Job '{}' not found", id))?;

    if let Some(name) = updates.name {
        job.name = name;
    }
    if let Some(schedule) = updates.schedule {
        job.schedule = schedule;
    }
    if let Some(message) = updates.message {
        job.payload = CronPayload::AgentTurn { message };
    }
    if let Some(enabled) = updates.enabled {
        job.enabled = enabled;
    }

    let updated = job.clone();
    write_cron_jobs_file(&file)?;
    Ok(updated)
}

/// Delete a cron job by ID. Built-in jobs cannot be deleted.
pub fn delete_cron_job(id: &str) -> Result<(), String> {
    if BUILTIN_JOB_IDS.contains(&id) {
        return Err(format!(
            "Cannot delete built-in job '{}'. Use update to disable it instead.",
            id
        ));
    }

    let mut file = read_cron_jobs()?;
    let before = file.jobs.len();
    file.jobs.retain(|j| j.id != id);

    if file.jobs.len() == before {
        return Err(format!("Job '{}' not found", id));
    }

    write_cron_jobs_file(&file)
}

// ---------------------------------------------------------------------------
// Resource copying
// ---------------------------------------------------------------------------

/// Copy bundled resources to user directories.
pub fn copy_resources(resources_dir: &Path) -> Result<(), String> {
    let home = home_dir();

    // Copy workspace files (SOUL.md, IDENTITY.md, etc.)
    copy_dir_contents(
        &resources_dir.join("workspace"),
        &home.join(".nyx/workspace"),
    )?;

    // Copy skills
    copy_dir_contents(
        &resources_dir.join("local-skills"),
        &home.join(".nyx/local-skills"),
    )?;

    // Copy Python modules (NEAR intents helper)
    copy_dir_contents(
        &resources_dir.join("near-intents-helper"),
        &home.join(".nyx/near-intents-helper"),
    )?;

    // Copy gog binary (macOS native — IronClaw runs natively, no Docker)
    let gog_src = resources_dir.join("bin/gog");
    let gog_dst = home.join(".nyx/bin/gog");
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

    // Copy jq binary
    let jq_src = resources_dir.join("bin/jq");
    let jq_dst = home.join(".nyx/bin/jq");
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
