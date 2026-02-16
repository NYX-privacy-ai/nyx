use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ONECLICK_BASE_URL: &str = "https://1click.chaindefuser.com/v0";

// ---------------------------------------------------------------------------
// Types — updated for 1Click API v2 schema (2026-02)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenInfo {
    #[serde(rename = "assetId")]
    pub asset_id: String,
    pub decimals: u32,
    pub symbol: String,
    pub blockchain: String,
    #[serde(rename = "chainName")]
    pub chain_name: Option<String>,
    #[serde(rename = "contractAddress")]
    pub address: Option<String>,
    pub price: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteRequest {
    pub dry: bool,
    pub swap_type: String,
    pub slippage_tolerance: u32,
    pub origin_asset: String,
    pub deposit_type: String,
    pub destination_asset: String,
    pub amount: String,
    pub refund_to: String,
    pub refund_type: String,
    pub recipient: String,
    pub recipient_type: String,
    pub deadline: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResponse {
    pub quote: QuoteDetails,
    pub signature: Option<String>,
    pub timestamp: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteDetails {
    pub amount_in: String,
    #[serde(default)]
    pub amount_in_formatted: String,
    #[serde(default)]
    pub amount_in_usd: String,
    #[serde(default)]
    pub min_amount_in: String,
    pub amount_out: String,
    #[serde(default)]
    pub amount_out_formatted: String,
    #[serde(default)]
    pub amount_out_usd: String,
    #[serde(default)]
    pub min_amount_out: String,
    #[serde(default)]
    pub time_estimate: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositAddress {
    pub chain: String,
    pub address: String,
    pub memo: Option<String>,
    pub min_deposit: Option<String>,
    pub max_deposit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapStatus {
    pub id: String,
    pub status: String,
    pub tx_hash_in: Option<String>,
    pub tx_hash_out: Option<String>,
    pub amount_in: Option<String>,
    pub amount_out: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a deadline 10 minutes from now in ISO 8601 format.
fn deadline_10min() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let future = now + 600; // 10 minutes
    // Format as ISO 8601: YYYY-MM-DDTHH:MM:SS.000Z
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_min = 60u64;

    // Days since epoch
    let mut remaining = future;
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        let year_secs = days_in_year * secs_per_day;
        if remaining < year_secs {
            break;
        }
        remaining -= year_secs;
        year += 1;
    }
    let month_days = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &days in &month_days {
        let month_secs = days * secs_per_day;
        if remaining < month_secs {
            break;
        }
        remaining -= month_secs;
        month += 1;
    }
    let day = remaining / secs_per_day + 1;
    remaining %= secs_per_day;
    let hour = remaining / secs_per_hour;
    remaining %= secs_per_hour;
    let minute = remaining / secs_per_min;
    let second = remaining % secs_per_min;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.000Z",
        year, month, day, hour, minute, second
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
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

/// Request a swap quote from the 1Click API (v2 schema).
pub async fn get_quote(
    origin_asset: &str,
    destination_asset: &str,
    amount: &str,
    recipient: &str,
    refund_to: &str,
    dry_run: bool,
) -> Result<QuoteResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = format!("{}/quote", ONECLICK_BASE_URL);

    let body = QuoteRequest {
        dry: dry_run,
        swap_type: "EXACT_INPUT".to_string(),
        slippage_tolerance: 100, // 1%
        origin_asset: origin_asset.to_string(),
        deposit_type: "INTENTS".to_string(),
        destination_asset: destination_asset.to_string(),
        amount: amount.to_string(),
        refund_to: refund_to.to_string(),
        refund_type: "INTENTS".to_string(),
        recipient: recipient.to_string(),
        recipient_type: "DESTINATION_CHAIN".to_string(),
        deadline: deadline_10min(),
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

/// Get a quote to shield assets into ZEC (any asset → ZEC).
pub async fn get_zec_quote(
    from_asset: &str,
    amount: &str,
    zec_address: &str,
    refund_to: &str,
) -> Result<QuoteResponse, String> {
    get_quote(
        from_asset,
        "nep141:zec.omft.near",
        amount,
        zec_address,
        refund_to,
        true, // dry run — user must confirm before executing
    )
    .await
}

/// Get a quote to unshield from ZEC to any asset (ZEC → any).
pub async fn get_quote_from_zec(
    to_asset: &str,
    zec_amount: &str,
    recipient: &str,
    zec_refund: &str,
) -> Result<QuoteResponse, String> {
    get_quote(
        "nep141:zec.omft.near",
        to_asset,
        zec_amount,
        recipient,
        zec_refund,
        true,
    )
    .await
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
///           ("near", "NEAR") -> "nep141:wrap.near"
///           ("sol", "SOL") -> "nep141:sol.omft.near"
pub fn resolve_asset_id(chain: &str, symbol: &str) -> Result<String, String> {
    let key = format!("{}:{}", chain.to_lowercase(), symbol.to_uppercase());

    let known: HashMap<&str, &str> = HashMap::from([
        ("near:NEAR", "nep141:wrap.near"),
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

/// Get the list of shieldable assets (human-readable for the UI).
pub fn get_shieldable_assets() -> Vec<ShieldableAsset> {
    vec![
        ShieldableAsset { chain: "eth".into(), symbol: "ETH".into(), name: "Ethereum".into(), asset_id: "nep141:eth.omft.near".into(), decimals: 18, icon: "E".into() },
        ShieldableAsset { chain: "near".into(), symbol: "NEAR".into(), name: "NEAR".into(), asset_id: "nep141:wrap.near".into(), decimals: 24, icon: "N".into() },
        ShieldableAsset { chain: "sol".into(), symbol: "SOL".into(), name: "Solana".into(), asset_id: "nep141:sol.omft.near".into(), decimals: 9, icon: "S".into() },
        ShieldableAsset { chain: "btc".into(), symbol: "BTC".into(), name: "Bitcoin".into(), asset_id: "btc:btc".into(), decimals: 8, icon: "B".into() },
        ShieldableAsset { chain: "eth".into(), symbol: "USDC".into(), name: "USDC".into(), asset_id: "nep141:usdc.eth.omft.near".into(), decimals: 6, icon: "$".into() },
        ShieldableAsset { chain: "eth".into(), symbol: "USDT".into(), name: "USDT".into(), asset_id: "nep141:usdt.eth.omft.near".into(), decimals: 6, icon: "$".into() },
    ]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShieldableAsset {
    pub chain: String,
    pub symbol: String,
    pub name: String,
    pub asset_id: String,
    pub decimals: u32,
    pub icon: String,
}
