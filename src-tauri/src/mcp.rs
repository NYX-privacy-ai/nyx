// ---------------------------------------------------------------------------
// MCP Server — Nyx tools exposed via Model Context Protocol
// ---------------------------------------------------------------------------
// This module defines the MCP tools that Claude Code (and other MCP clients)
// can discover and call. Tools wrap the shared nyx_lib functions.
// ---------------------------------------------------------------------------

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};

use crate::config;
use crate::gateway;
use crate::ironclaw;
use crate::oneclick;
use crate::portfolio_data;

// ---------------------------------------------------------------------------
// Tool parameter types (must impl Deserialize + JsonSchema)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatParams {
    /// The message to send to the Nyx agent
    pub message: String,
    /// Optional session key (default: "agent:default:main")
    pub session_key: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VerifySourceParams {
    /// URL or claim to analyse for credibility
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SessionsParams {
    /// Action to perform: "list" or "create"
    pub action: String,
    /// Title for a new session (only used with action "create")
    pub title: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ZecQuoteParams {
    /// Direction: "shield" (any to ZEC) or "unshield" (ZEC to any)
    pub direction: String,
    /// Asset identifier (e.g. "eth:ETH", "near:NEAR", "sol:USDC")
    pub asset: String,
    /// Amount to swap
    pub amount: String,
    /// Recipient address (required for unshield direction)
    pub recipient: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConfidentialQuoteParams {
    /// Origin asset identifier (e.g. "nep141:wrap.near", "nep141:eth.omft.near")
    pub origin_asset: String,
    /// Destination asset identifier (e.g. "nep141:usdc.eth.omft.near")
    pub destination_asset: String,
    /// Amount to swap (in smallest unit of the origin asset)
    pub amount: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScheduleParams {
    /// Action to perform: "list", "create", "update", "delete", "enable", "disable"
    pub action: String,
    /// Job ID (required for update, delete, enable, disable)
    pub id: Option<String>,
    /// Job name (required for create)
    pub name: Option<String>,
    /// Schedule string. For interval: "every:3600000" (ms). For cron: "cron:0 9 * * *" or "cron:0 9 * * *:Europe/London"
    pub schedule: Option<String>,
    /// The message/instruction the agent executes when the job fires (required for create)
    pub message: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SessionHandoffParams {
    /// Session key to load history from (e.g. "agent:default:main")
    pub session_key: String,
    /// Maximum number of recent messages to retrieve (default: 20)
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// MCP Server handler
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NyxMcpServer {
    tool_router: ToolRouter<Self>,
}

impl NyxMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for NyxMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl NyxMcpServer {
    /// Send a message to the Nyx agent and get a response.
    #[tool(
        description = "Send a message to the Nyx agent and get a response. The agent can search the web, manage calendars, execute DeFi operations, and more."
    )]
    async fn nyx_chat(&self, Parameters(params): Parameters<ChatParams>) -> String {
        let session = params
            .session_key
            .unwrap_or_else(|| "agent:default:main".to_string());

        match gateway::send_message_to_session(params.message, session).await {
            Ok(reply) => reply,
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Get the current DeFi portfolio data.
    #[tool(
        description = "Get the current DeFi portfolio data including positions, allocation, health status, and recent activity."
    )]
    async fn nyx_portfolio(&self) -> String {
        match portfolio_data::read_portfolio().await {
            Ok(data) => serde_json::to_string_pretty(&data)
                .unwrap_or_else(|_| "Failed to serialize portfolio".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Analyse a URL or claim for credibility.
    #[tool(
        description = "Analyse a URL or claim for credibility. Returns a detailed credibility score across 6 dimensions including source reputation, corroboration, and evidence quality."
    )]
    async fn nyx_verify_source(
        &self,
        Parameters(params): Parameters<VerifySourceParams>,
    ) -> String {
        match gateway::verify_source(params.query).await {
            Ok(analysis) => analysis,
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Check the Nyx IronClaw daemon status.
    #[tool(
        description = "Check the Nyx IronClaw daemon status including whether it's running, the version, and gateway port."
    )]
    async fn nyx_ironclaw_status(&self) -> String {
        match ironclaw::check_ironclaw_detailed().await {
            Ok(status) => serde_json::to_string_pretty(&status)
                .unwrap_or_else(|_| "Failed to serialize status".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// List or create chat sessions.
    #[tool(
        description = "List or create chat sessions. Use action 'list' to get all sessions, or 'create' with an optional title to start a new session."
    )]
    async fn nyx_sessions(&self, Parameters(params): Parameters<SessionsParams>) -> String {
        match params.action.as_str() {
            "list" => match gateway::list_sessions() {
                Ok(sessions) => serde_json::to_string_pretty(&sessions)
                    .unwrap_or_else(|_| "Failed to serialize sessions".to_string()),
                Err(e) => format!("Error: {}", e),
            },
            "create" => match gateway::create_session(params.title, None) {
                Ok(key) => format!("Created session: {}", key),
                Err(e) => format!("Error: {}", e),
            },
            other => format!("Unknown action '{}'. Use 'list' or 'create'.", other),
        }
    }

    /// Get a cross-chain swap quote for shielding or unshielding ZEC.
    #[tool(
        description = "Get a cross-chain swap quote for shielding assets into Zcash (ZEC) or unshielding from ZEC to any supported crypto. Uses NEAR Intents for cross-chain routing."
    )]
    async fn nyx_zec_quote(&self, Parameters(params): Parameters<ZecQuoteParams>) -> String {
        let result = match params.direction.as_str() {
            "shield" => {
                let zec_address = match config::get_zec_address() {
                    Some(addr) => addr,
                    None => {
                        return "Error: No ZEC address configured. Add a ZEC wallet in Settings."
                            .to_string()
                    }
                };
                let refund_to =
                    config::get_near_account().unwrap_or_else(|| "nyx.near".to_string());
                oneclick::get_zec_quote(&params.asset, &params.amount, &zec_address, &refund_to)
                    .await
            }
            "unshield" => {
                let zec_refund = match config::get_zec_address() {
                    Some(addr) => addr,
                    None => {
                        return "Error: No ZEC address configured. Add a ZEC wallet in Settings."
                            .to_string()
                    }
                };
                let recipient = match params.recipient {
                    Some(r) => r,
                    None => return "Error: recipient address required for unshield".to_string(),
                };
                oneclick::get_quote_from_zec(&params.asset, &params.amount, &recipient, &zec_refund)
                    .await
            }
            other => {
                return format!("Unknown direction '{}'. Use 'shield' or 'unshield'.", other);
            }
        };

        match result {
            Ok(quote) => serde_json::to_string_pretty(&quote)
                .unwrap_or_else(|_| "Failed to serialize quote".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Get a confidential cross-chain swap quote via NEAR Confidential Intents.
    #[tool(
        description = "Get a confidential cross-chain swap quote via NEAR Confidential Intents. Executes in a TEE-secured private shard — prevents MEV, frontrunning, and strategy leakage. Same speed and cost as public swaps, but transaction details remain confidential during execution."
    )]
    async fn nyx_confidential_quote(
        &self,
        Parameters(params): Parameters<ConfidentialQuoteParams>,
    ) -> String {
        let near_account = config::get_near_account().unwrap_or_else(|| "nyx.near".to_string());

        let result = oneclick::get_confidential_quote(
            &params.origin_asset,
            &params.destination_asset,
            &params.amount,
            &near_account,
            &near_account,
        )
        .await;

        match result {
            Ok(quote) => serde_json::to_string_pretty(&quote)
                .unwrap_or_else(|_| "Failed to serialize quote".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Manage scheduled tasks (cron jobs). Create, list, update, delete, enable or disable recurring agent tasks.
    #[tool(
        description = "Manage scheduled tasks (cron jobs). Actions: 'list' all tasks, 'create' a new recurring task, 'update' an existing task, 'delete' a task, 'enable'/'disable' a task. Schedule format: 'every:3600000' (interval in ms) or 'cron:0 9 * * *:Europe/London' (cron expression with optional timezone)."
    )]
    async fn nyx_schedule(&self, Parameters(params): Parameters<ScheduleParams>) -> String {
        match params.action.as_str() {
            "list" => match config::read_cron_jobs() {
                Ok(file) => serde_json::to_string_pretty(&file.jobs)
                    .unwrap_or_else(|_| "Failed to serialize jobs".to_string()),
                Err(e) => format!("Error: {}", e),
            },
            "create" => {
                let name = match params.name {
                    Some(n) => n,
                    None => return "Error: 'name' is required for create".to_string(),
                };
                let schedule_str = match params.schedule {
                    Some(s) => s,
                    None => return "Error: 'schedule' is required for create".to_string(),
                };
                let message = match params.message {
                    Some(m) => m,
                    None => return "Error: 'message' is required for create".to_string(),
                };
                let schedule = match parse_schedule_string(&schedule_str) {
                    Ok(s) => s,
                    Err(e) => return format!("Error: {}", e),
                };
                match config::create_cron_job(name, schedule, message, true) {
                    Ok(job) => {
                        serde_json::to_string_pretty(&job).unwrap_or_else(|_| "Created".to_string())
                    }
                    Err(e) => format!("Error: {}", e),
                }
            }
            "update" => {
                let id = match params.id {
                    Some(i) => i,
                    None => return "Error: 'id' is required for update".to_string(),
                };
                let schedule = match params.schedule {
                    Some(s) => match parse_schedule_string(&s) {
                        Ok(sched) => Some(sched),
                        Err(e) => return format!("Error: {}", e),
                    },
                    None => None,
                };
                match config::update_cron_job(
                    &id,
                    config::CronJobUpdate {
                        name: params.name,
                        schedule,
                        message: params.message,
                        enabled: None,
                    },
                ) {
                    Ok(job) => {
                        serde_json::to_string_pretty(&job).unwrap_or_else(|_| "Updated".to_string())
                    }
                    Err(e) => format!("Error: {}", e),
                }
            }
            "delete" => {
                let id = match params.id {
                    Some(i) => i,
                    None => return "Error: 'id' is required for delete".to_string(),
                };
                match config::delete_cron_job(&id) {
                    Ok(()) => format!("Deleted job '{}'", id),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "enable" | "disable" => {
                let id = match params.id {
                    Some(i) => i,
                    None => return format!("Error: 'id' is required for {}", params.action),
                };
                let enabled = params.action == "enable";
                match config::update_cron_job(
                    &id,
                    config::CronJobUpdate {
                        name: None,
                        schedule: None,
                        message: None,
                        enabled: Some(enabled),
                    },
                ) {
                    Ok(job) => {
                        let state = if enabled { "enabled" } else { "disabled" };
                        format!("Job '{}' {}: {}", job.id, state, job.name)
                    }
                    Err(e) => format!("Error: {}", e),
                }
            }
            other => format!(
                "Unknown action '{}'. Use: list, create, update, delete, enable, disable.",
                other
            ),
        }
    }

    /// Load recent message history from a session transcript. Useful for context handoff between sessions or platforms.
    #[tool(
        description = "Load recent message history from a session transcript. Returns the last N messages from a given session. Useful for cross-platform session continuation or context handoff between different conversations."
    )]
    async fn nyx_session_handoff(
        &self,
        Parameters(params): Parameters<SessionHandoffParams>,
    ) -> String {
        let limit = params.limit.unwrap_or(20);
        match gateway::load_session_history(&params.session_key, limit) {
            Ok(messages) => serde_json::to_string_pretty(&messages)
                .unwrap_or_else(|_| "Failed to serialize history".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }
}

/// Parse a schedule string like "every:3600000" or "cron:0 9 * * *:Europe/London"
fn parse_schedule_string(s: &str) -> Result<config::CronSchedule, String> {
    if let Some(rest) = s.strip_prefix("every:") {
        let ms: u64 = rest
            .parse()
            .map_err(|_| format!("Invalid interval '{}' — must be milliseconds", rest))?;
        if ms < 60000 {
            return Err("Minimum interval is 60000ms (1 minute)".to_string());
        }
        Ok(config::CronSchedule::Every {
            every_ms: ms,
            anchor_ms: None,
        })
    } else if let Some(rest) = s.strip_prefix("cron:") {
        // Format: "cron:0 9 * * *" or "cron:0 9 * * *:Europe/London"
        // Cron expression has 5 fields; timezone is after the 5th field separator
        let parts: Vec<&str> = rest.splitn(6, ' ').collect();
        if parts.len() < 5 {
            return Err(format!(
                "Invalid cron expression '{}' — need 5 fields (min hour dom mon dow)",
                rest
            ));
        }
        let (expr, tz) = if parts.len() == 6 {
            // Last part may be "dow:TZ" if timezone was appended with ':'
            let last = parts[5];
            if last.contains(':') {
                let split: Vec<&str> = last.splitn(2, ':').collect();
                (
                    format!("{} {}", parts[..5].join(" "), split[0]),
                    Some(split[1].to_string()),
                )
            } else {
                (parts.join(" "), None)
            }
        } else {
            (rest.to_string(), None)
        };
        Ok(config::CronSchedule::Cron { expr, tz })
    } else {
        Err(format!(
            "Invalid schedule '{}'. Use 'every:<ms>' or 'cron:<expr>'",
            s
        ))
    }
}

#[tool_handler]
impl ServerHandler for NyxMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Nyx is an AI-powered productivity app. Tools include chatting with the agent, \
                 DeFi portfolio data, source credibility analysis, IronClaw daemon status, \
                 session management, schedule management, session history handoff, \
                 ZEC privacy shield quotes, and confidential cross-chain \
                 swaps via NEAR Confidential Intents (TEE-protected)."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
