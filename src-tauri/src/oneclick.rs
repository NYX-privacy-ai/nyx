use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ONECLICK_BASE_URL: &str = "https://1click.chaindefuser.com/v0";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenInfo {
    pub defuse_asset_id: String,
    pub decimals: u32,
    pub symbol: String,
    pub blockchain: String,
    pub chain_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteRequest {
    pub defuse_asset_identifier_in: String,
    pub defuse_asset_identifier_out: String,
    pub exact_amount_in: Option<String>,
    pub exact_amount_out: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub quote_hash: String,
    pub defuse_asset_identifier_in: String,
    pub defuse_asset_identifier_out: String,
    pub amount_in: String,
    pub amount_out: String,
    pub expiration_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositAddress {
    pub chain: String,
    pub address: String,
    pub memo: Option<String>,
    pub min_deposit: Option<String>,
    pub max_deposit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SwapStatus {
    pub id: String,
    pub status: String, // "pending", "processing", "complete", "failed"
    pub tx_hash_in: Option<String>,
    pub tx_hash_out: Option<String>,
    pub amount_in: Option<String>,
    pub amount_out: Option<String>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Fetch the list of supported tokens from the 1Click API.
pub async fn get_tokens() -> Result<Vec<TokenInfo>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/tokens", ONECLICK_BASE_URL);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("1Click tokens request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("1Click tokens error ({}): {}", status, body));
    }

    let tokens: Vec<TokenInfo> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse tokens response: {}", e))?;

    Ok(tokens)
}

/// Request a swap quote from the 1Click API.
pub async fn get_quote(
    asset_in: &str,
    asset_out: &str,
    amount_in: &str,
) -> Result<QuoteResponse, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/quote", ONECLICK_BASE_URL);

    let body = QuoteRequest {
        defuse_asset_identifier_in: asset_in.to_string(),
        defuse_asset_identifier_out: asset_out.to_string(),
        exact_amount_in: Some(amount_in.to_string()),
        exact_amount_out: None,
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("1Click quote request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("1Click quote error ({}): {}", status, err_body));
    }

    let quote: QuoteResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse quote response: {}", e))?;

    Ok(quote)
}

/// Get the status of a swap.
pub async fn get_status(swap_id: &str) -> Result<SwapStatus, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/status/{}", ONECLICK_BASE_URL, swap_id);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("1Click status request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        return Err(format!("1Click status error ({}): {}", status, err_body));
    }

    let swap_status: SwapStatus = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse status response: {}", e))?;

    Ok(swap_status)
}

// ---------------------------------------------------------------------------
// Asset ID mapping helpers
// ---------------------------------------------------------------------------

/// Map a chain+symbol pair to a defuse asset identifier.
/// Examples: ("eth", "ETH") -> "nep141:eth.omft.near"
///           ("near", "NEAR") -> "near:native"
///           ("sol", "SOL") -> "nep141:sol.omft.near"
pub fn resolve_asset_id(chain: &str, symbol: &str) -> Result<String, String> {
    // Common known mappings
    let key = format!("{}:{}", chain.to_lowercase(), symbol.to_uppercase());

    let known: HashMap<&str, &str> = HashMap::from([
        ("near:NEAR", "near:native"),
        ("near:WNEAR", "nep141:wrap.near"),
        ("near:USDC", "nep141:a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near"),
        ("near:USDT", "nep141:dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near"),
        ("eth:ETH", "nep141:eth.omft.near"),
        ("eth:USDC", "nep141:usdc.eth.omft.near"),
        ("eth:USDT", "nep141:usdt.eth.omft.near"),
        ("sol:SOL", "nep141:sol.omft.near"),
        ("btc:BTC", "btc:btc"),
        ("zec:ZEC", "nep141:zec.omft.near"),
        ("base:ETH", "nep141:eth.base.omft.near"),
        ("arbitrum:ETH", "nep141:eth.arb.omft.near"),
    ]);

    known
        .get(key.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Unknown asset: {}:{}", chain, symbol))
}
