// ---------------------------------------------------------------------------
// MCP Server — Nyx tools exposed via Model Context Protocol
// ---------------------------------------------------------------------------
// This module defines the MCP tools that Claude Code (and other MCP clients)
// can discover and call. Tools wrap the shared nyx_lib functions.
// ---------------------------------------------------------------------------

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::config;
use crate::docker;
use crate::gateway;
use crate::oneclick;
use crate::portfolio_data;

// ---------------------------------------------------------------------------
// Tool parameter types (must impl Deserialize + JsonSchema)
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ChatParams {
    /// The message to send to the OpenClaw agent
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
    /// Send a message to the OpenClaw agent (Atlas) and get a response.
    #[tool(description = "Send a message to the OpenClaw agent (Atlas) and get a response. The agent can search the web, manage calendars, execute DeFi operations, and more.")]
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
    #[tool(description = "Get the current DeFi portfolio data including positions, allocation, health status, and recent activity.")]
    async fn nyx_portfolio(&self) -> String {
        match portfolio_data::read_portfolio().await {
            Ok(data) => serde_json::to_string_pretty(&data)
                .unwrap_or_else(|_| "Failed to serialize portfolio".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Analyse a URL or claim for credibility.
    #[tool(description = "Analyse a URL or claim for credibility. Returns a detailed credibility score across 6 dimensions including source reputation, corroboration, and evidence quality.")]
    async fn nyx_verify_source(
        &self,
        Parameters(params): Parameters<VerifySourceParams>,
    ) -> String {
        match gateway::verify_source(params.query).await {
            Ok(analysis) => analysis,
            Err(e) => format!("Error: {}", e),
        }
    }

    /// Check the OpenClaw Docker container status.
    #[tool(description = "Check the OpenClaw Docker container status including whether it's running, the image version, and system health.")]
    async fn nyx_docker_status(&self) -> String {
        match docker::check_docker_detailed().await {
            Ok(status) => serde_json::to_string_pretty(&status)
                .unwrap_or_else(|_| "Failed to serialize status".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    /// List or create chat sessions.
    #[tool(description = "List or create chat sessions. Use action 'list' to get all sessions, or 'create' with an optional title to start a new session.")]
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
    #[tool(description = "Get a cross-chain swap quote for shielding assets into Zcash (ZEC) or unshielding from ZEC to any supported crypto. Uses NEAR Intents for cross-chain routing.")]
    async fn nyx_zec_quote(&self, Parameters(params): Parameters<ZecQuoteParams>) -> String {
        let result = match params.direction.as_str() {
            "shield" => {
                let zec_address = match config::get_zec_address() {
                    Some(addr) => addr,
                    None => return "Error: No ZEC address configured. Add a ZEC wallet in Settings.".to_string(),
                };
                let refund_to = config::get_near_account()
                    .unwrap_or_else(|| "nyx.near".to_string());
                oneclick::get_zec_quote(&params.asset, &params.amount, &zec_address, &refund_to)
                    .await
            }
            "unshield" => {
                let zec_refund = match config::get_zec_address() {
                    Some(addr) => addr,
                    None => return "Error: No ZEC address configured. Add a ZEC wallet in Settings.".to_string(),
                };
                let recipient = match params.recipient {
                    Some(r) => r,
                    None => return "Error: recipient address required for unshield".to_string(),
                };
                oneclick::get_quote_from_zec(
                    &params.asset,
                    &params.amount,
                    &recipient,
                    &zec_refund,
                )
                .await
            }
            other => {
                return format!(
                    "Unknown direction '{}'. Use 'shield' or 'unshield'.",
                    other
                );
            }
        };

        match result {
            Ok(quote) => serde_json::to_string_pretty(&quote)
                .unwrap_or_else(|_| "Failed to serialize quote".to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }
}

#[tool_handler]
impl ServerHandler for NyxMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Nyx is a private AI chief of staff. Tools include chatting with the OpenClaw agent, \
                 DeFi portfolio data, source credibility analysis, Docker container status, \
                 session management, and ZEC privacy shield quotes."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
