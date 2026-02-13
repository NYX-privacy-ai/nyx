use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

use crate::config::{Chain, WalletConfig};

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletInfo {
    pub account_id: String,
    pub public_key: String,
    pub secret_key: String,
}

// ---------------------------------------------------------------------------
// NEAR wallet generation
// ---------------------------------------------------------------------------

/// Generate a new NEAR ed25519 keypair.
/// Returns a `WalletInfo` (contains the private key for secret storage) and a
/// `WalletConfig` (safe to persist in the main config file).
pub async fn generate_near_wallet() -> Result<(WalletInfo, WalletConfig), String> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let public_bytes = verifying_key.as_bytes();
    let secret_bytes = signing_key.to_bytes();

    // NEAR implicit account = hex(sha256(public_key))
    let mut hasher = Sha256::new();
    hasher.update(public_bytes);
    let account_id = hex::encode(hasher.finalize());

    // NEAR key format: ed25519:<base58_encoded_key>
    let public_key = format!("ed25519:{}", bs58::encode(public_bytes).into_string());

    // Secret key includes both secret + public (64 bytes total)
    let mut full_secret = Vec::with_capacity(64);
    full_secret.extend_from_slice(&secret_bytes);
    full_secret.extend_from_slice(public_bytes);
    let secret_key = format!("ed25519:{}", bs58::encode(&full_secret).into_string());

    // Generate a wallet id (UUID-like 128-bit hex string, no uuid crate needed)
    let wallet_id = format!("{:032x}", rand::thread_rng().gen::<u128>());

    let wallet_info = WalletInfo {
        account_id: account_id.clone(),
        public_key,
        secret_key,
    };

    let wallet_config = WalletConfig {
        id: wallet_id,
        chain: Chain::NEAR,
        address: account_id,
        label: "NEAR wallet".to_string(),
        has_private_key: true,
        is_active: true,
    };

    Ok((wallet_info, wallet_config))
}

// ---------------------------------------------------------------------------
// Address validation
// ---------------------------------------------------------------------------

/// Validate a wallet address for the given chain.
pub fn validate_address(chain: &Chain, address: &str) -> Result<(), String> {
    match chain {
        Chain::NEAR => validate_near_address(address),
        Chain::ETH => validate_eth_address(address),
        Chain::SOL => validate_sol_address(address),
        Chain::BTC => validate_btc_address(address),
        Chain::ZEC => validate_zec_address(address),
    }
}

fn validate_near_address(address: &str) -> Result<(), String> {
    // Named accounts: *.near or *.testnet
    if address.ends_with(".near") || address.ends_with(".testnet") {
        let prefix = if address.ends_with(".near") {
            &address[..address.len() - 5]
        } else {
            &address[..address.len() - 8]
        };
        if prefix.is_empty() {
            return Err("NEAR named account has empty prefix".to_string());
        }
        return Ok(());
    }

    // Implicit account: 64-char hex
    if address.len() == 64 && address.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }

    Err("Invalid NEAR address: must be a .near/.testnet name or 64-char hex implicit account".to_string())
}

fn validate_eth_address(address: &str) -> Result<(), String> {
    if !address.starts_with("0x") {
        return Err("ETH address must start with 0x".to_string());
    }
    if address.len() != 42 {
        return Err(format!("ETH address must be 42 characters, got {}", address.len()));
    }
    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("ETH address contains non-hex characters after 0x".to_string());
    }
    Ok(())
}

fn validate_sol_address(address: &str) -> Result<(), String> {
    if address.len() < 32 || address.len() > 44 {
        return Err(format!(
            "SOL address must be 32-44 characters, got {}",
            address.len()
        ));
    }
    // Must be valid base58
    if bs58::decode(address).into_vec().is_err() {
        return Err("SOL address is not valid base58".to_string());
    }
    Ok(())
}

fn validate_btc_address(address: &str) -> Result<(), String> {
    let len = address.len();
    if len < 25 || len > 62 {
        return Err(format!(
            "BTC address must be 25-62 characters, got {}",
            len
        ));
    }
    if !(address.starts_with('1')
        || address.starts_with('3')
        || address.starts_with("bc1")
        || address.starts_with("tb1"))
    {
        return Err("BTC address must start with 1, 3, bc1, or tb1".to_string());
    }
    Ok(())
}

fn validate_zec_address(address: &str) -> Result<(), String> {
    let len = address.len();

    // Transparent t-addresses: t1... (t1 = mainnet P2PKH), t3... (t3 = mainnet P2SH)
    if address.starts_with("t1") || address.starts_with("t3") {
        if len != 35 {
            return Err(format!(
                "ZEC transparent address must be 35 characters, got {}",
                len
            ));
        }
        // Base58check encoded
        if bs58::decode(address).into_vec().is_err() {
            return Err("ZEC transparent address is not valid base58".to_string());
        }
        return Ok(());
    }

    // Shielded Sapling addresses: zs1...
    if address.starts_with("zs1") {
        // Sapling addresses are Bech32-encoded, typically 78 chars
        if len < 70 || len > 90 {
            return Err(format!(
                "ZEC shielded (Sapling) address should be ~78 characters, got {}",
                len
            ));
        }
        return Ok(());
    }

    // Unified addresses: u1...
    if address.starts_with("u1") {
        // Unified addresses vary in length but are typically 200+ chars
        if len < 50 {
            return Err(format!(
                "ZEC unified address seems too short, got {}",
                len
            ));
        }
        return Ok(());
    }

    Err("ZEC address must start with t1/t3 (transparent), zs1 (shielded), or u1 (unified)".to_string())
}

// ---------------------------------------------------------------------------
// Import an external (watch-only) wallet
// ---------------------------------------------------------------------------

/// Import an external wallet by address. The resulting `WalletConfig` has
/// `has_private_key = false` and `is_active = false` so the caller can decide
/// when to activate it.
pub fn import_wallet(chain: Chain, address: String, label: String) -> Result<WalletConfig, String> {
    validate_address(&chain, &address)?;

    let wallet_id = format!("{:032x}", rand::thread_rng().gen::<u128>());

    Ok(WalletConfig {
        id: wallet_id,
        chain,
        address,
        label,
        has_private_key: false,
        is_active: false,
    })
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

/// Return the base secrets directory: `~/.openclaw/secrets`
fn secrets_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    Ok(PathBuf::from(home).join(".openclaw/secrets"))
}

/// Save wallet credentials to the legacy `near_account.json` location.
pub fn save_wallet(wallet: &WalletInfo, secrets_dir: &std::path::Path) -> Result<(), String> {
    let wallet_json = serde_json::json!({
        "account_id": wallet.account_id,
        "public_key": wallet.public_key,
        "private_key": wallet.secret_key
    });

    let path = secrets_dir.join("near_account.json");
    std::fs::write(&path, serde_json::to_string_pretty(&wallet_json).unwrap())
        .map_err(|e| format!("Failed to write wallet: {}", e))?;

    // chmod 600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set wallet permissions: {}", e))?;
    }

    Ok(())
}

/// Save a wallet's private-key material to
/// `~/.openclaw/secrets/wallets/{wallet_id}.json`.
pub fn save_wallet_key(wallet_id: &str, wallet_info: &WalletInfo) -> Result<(), String> {
    let wallets_dir = secrets_dir()?.join("wallets");

    fs::create_dir_all(&wallets_dir)
        .map_err(|e| format!("Failed to create wallets dir: {}", e))?;

    let path = wallets_dir.join(format!("{}.json", wallet_id));
    let content = serde_json::to_string_pretty(wallet_info)
        .map_err(|e| format!("Failed to serialize wallet key: {}", e))?;

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write wallet key: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set wallet key permissions: {}", e))?;
    }

    Ok(())
}

/// Load a wallet's private-key material from
/// `~/.openclaw/secrets/wallets/{wallet_id}.json`.
/// Returns `Ok(None)` when the file does not exist.
pub fn load_wallet_key(wallet_id: &str) -> Result<Option<WalletInfo>, String> {
    let path = secrets_dir()?.join("wallets").join(format!("{}.json", wallet_id));

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read wallet key: {}", e))?;

    let info: WalletInfo = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse wallet key: {}", e))?;

    Ok(Some(info))
}
