#!/usr/bin/env python3
"""
NEAR Intents helper — quote and publish via Solver Relay JSON-RPC.

Guardrails enforced:
  - Asset allowlists (configurable via env)
  - Max swap amount per asset (configurable via env)
  - Publish requires explicit --confirm YES flag
  - All actions logged to audit.jsonl
"""

import argparse
import base64
from decimal import Decimal
import json
import os
import random
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

import base58
import nacl.signing
import requests
from dotenv import load_dotenv

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
SECRETS_DIR = Path.home() / ".openclaw" / "secrets"
ENV_FILE = SECRETS_DIR / "near_intents.env"
ACCOUNT_FILE = SECRETS_DIR / "near_account.json"
LOG_DIR = Path.home() / ".openclaw" / "defi-state" / "logs"
AUDIT_LOG = LOG_DIR / "audit.jsonl"

# ---------------------------------------------------------------------------
# Asset registry
# ---------------------------------------------------------------------------
ASSET_MAP = {
    "NEAR": {
        "defuse_id": "nep141:wrap.near",
        "contract": "wrap.near",
        "decimals": 24,
    },
    "WNEAR": {
        "defuse_id": "nep141:wrap.near",
        "contract": "wrap.near",
        "decimals": 24,
    },
    "USDC": {
        "defuse_id": "nep141:a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near",
        "contract": "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near",
        "decimals": 6,
    },
    "USDT": {
        "defuse_id": "nep141:dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
        "contract": "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
        "decimals": 6,
    },
    "STNEAR": {
        "defuse_id": "nep141:meta-pool.near",
        "contract": "meta-pool.near",
        "decimals": 24,
    },
    "AURORA": {
        "defuse_id": "nep141:aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near",
        "contract": "aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near",
        "decimals": 18,
    },
    "WBTC": {
        "defuse_id": "nep141:2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near",
        "contract": "2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near",
        "decimals": 8,
    },
    "WETH": {
        "defuse_id": "nep141:c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2.factory.bridge.near",
        "contract": "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2.factory.bridge.near",
        "decimals": 18,
    },
}


def _load_config():
    """Load env config.

    Checks environment variables first (injected at container boundary).
    Falls back to .env file if SOLVER_RELAY_URL is not already set.
    """
    if not os.environ.get("SOLVER_RELAY_URL") and ENV_FILE.exists():
        load_dotenv(ENV_FILE)
    if not os.environ.get("SOLVER_RELAY_URL"):
        _die("SOLVER_RELAY_URL not set. Configure via env vars or near_intents.env")
    return {
        "solver_relay_url": os.environ["SOLVER_RELAY_URL"],
        "max_near": float(os.environ.get("MAX_SWAP_AMOUNT_NEAR", "5.0")),
        "max_usdc": float(os.environ.get("MAX_SWAP_AMOUNT_USDC", "50.0")),
        "allowlist_in": {s.strip() for s in os.environ.get("ALLOWLIST_IN", "NEAR,USDC").split(",")},
        "allowlist_out": {s.strip() for s in os.environ.get("ALLOWLIST_OUT", "NEAR,USDC").split(",")},
        "deadline_ms": int(os.environ.get("DEFAULT_DEADLINE_MS", "120000")),
    }


def _parse_near_key(raw_key: str):
    """Parse a NEAR ed25519 key string into a nacl SigningKey."""
    if raw_key.startswith("ed25519:"):
        raw_key = raw_key[len("ed25519:"):]
    key_bytes = base58.b58decode(raw_key)
    if len(key_bytes) == 64:
        return nacl.signing.SigningKey(key_bytes[:32])
    elif len(key_bytes) == 32:
        return nacl.signing.SigningKey(key_bytes)
    else:
        _die(f"Unexpected key length: {len(key_bytes)}")


def _load_account():
    """Load NEAR account credentials.

    Checks environment variables first (NEAR_ACCOUNT_ID + NEAR_PRIVATE_KEY),
    falling back to the account JSON file only if env vars are not set.
    """
    env_key = os.environ.get("NEAR_PRIVATE_KEY")
    env_account = os.environ.get("NEAR_ACCOUNT_ID")
    if env_key and env_account:
        return env_account, _parse_near_key(env_key)

    if not ACCOUNT_FILE.exists():
        _die(f"Account file not found: {ACCOUNT_FILE}")
    with open(ACCOUNT_FILE, "r") as f:
        data = json.load(f)
    return data["account_id"], _parse_near_key(data["private_key"])


# ---------------------------------------------------------------------------
# Guardrails
# ---------------------------------------------------------------------------
def _check_asset(symbol: str, direction: str, cfg: dict):
    allowed = cfg["allowlist_in"] if direction == "in" else cfg["allowlist_out"]
    if symbol not in allowed:
        _die(f"Asset '{symbol}' not in {direction} allowlist: {allowed}")
    if symbol not in ASSET_MAP:
        _die(f"Unknown asset symbol: {symbol}")


def _check_amount(symbol: str, human_amount: float, cfg: dict):
    limits = {"NEAR": cfg["max_near"], "USDC": cfg["max_usdc"]}
    limit = limits.get(symbol)
    if limit is not None and human_amount > limit:
        _die(f"Amount {human_amount} {symbol} exceeds max {limit}")
    if human_amount <= 0:
        _die("Amount must be positive")


def _to_raw(symbol: str, human_amount: float) -> str:
    decimals = ASSET_MAP[symbol]["decimals"]
    return str(int(Decimal(str(human_amount)) * Decimal(10 ** decimals)))


# ---------------------------------------------------------------------------
# Audit logging
# ---------------------------------------------------------------------------
def _audit(entry: dict):
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    entry["timestamp"] = datetime.now(timezone.utc).isoformat()
    with open(AUDIT_LOG, "a") as f:
        f.write(json.dumps(entry, separators=(",", ":")) + "\n")


# ---------------------------------------------------------------------------
# JSON-RPC helper
# ---------------------------------------------------------------------------
def _rpc(url: str, method: str, params: list, timeout: int = 15) -> dict:
    payload = {
        "id": "openclaw-near-intents",
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    }
    _audit({"action": "rpc_request", "method": method, "params": params})
    resp = requests.post(url, json=payload, timeout=timeout)
    resp.raise_for_status()
    result = resp.json()
    if "error" in result:
        _audit({"action": "rpc_error", "method": method, "error": result["error"]})
        _die(f"RPC error: {json.dumps(result['error'])}")
    _audit({"action": "rpc_response", "method": method, "result_keys": list(result.get("result", {}).keys()) if isinstance(result.get("result"), dict) else "array"})
    return result


# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------
def cmd_quote(args, cfg):
    """Request quotes from solvers."""
    symbol_in = args.asset_in.upper()
    symbol_out = args.asset_out.upper()
    human_amount = float(args.amount)

    _check_asset(symbol_in, "in", cfg)
    _check_asset(symbol_out, "out", cfg)
    _check_amount(symbol_in, human_amount, cfg)

    raw_amount = _to_raw(symbol_in, human_amount)
    defuse_in = ASSET_MAP[symbol_in]["defuse_id"]
    defuse_out = ASSET_MAP[symbol_out]["defuse_id"]

    params = [{
        "defuse_asset_identifier_in": defuse_in,
        "defuse_asset_identifier_out": defuse_out,
        "exact_amount_in": raw_amount,
        "min_deadline_ms": cfg["deadline_ms"],
    }]

    result = _rpc(cfg["solver_relay_url"], "quote", params)
    quotes = result.get("result", [])

    output = {
        "status": "ok",
        "asset_in": symbol_in,
        "asset_out": symbol_out,
        "amount_in_human": human_amount,
        "amount_in_raw": raw_amount,
        "quotes_count": len(quotes) if isinstance(quotes, list) else 0,
        "quotes": [],
    }

    if isinstance(quotes, list):
        for q in quotes:
            amount_out_raw = q.get("amount_out", "0")
            decimals_out = ASSET_MAP[symbol_out]["decimals"]
            amount_out_human = int(amount_out_raw) / (10 ** decimals_out)
            output["quotes"].append({
                "quote_hash": q.get("quote_hash"),
                "amount_out_raw": amount_out_raw,
                "amount_out_human": amount_out_human,
                "expiration_time": q.get("expiration_time"),
            })
        # Sort best first
        output["quotes"].sort(key=lambda x: float(x["amount_out_human"]), reverse=True)

    _audit({"action": "quote_result", "asset_in": symbol_in, "asset_out": symbol_out, "amount": human_amount, "quotes_count": output["quotes_count"]})
    print(json.dumps(output, indent=2))


def cmd_publish(args, cfg):
    """Publish a signed intent to the solver relay."""
    if args.confirm != "YES":
        _die("Publish requires --confirm YES (explicit confirmation)")

    account_id, signing_key = _load_account()

    quote_hash = args.quote_hash
    symbol_in = args.asset_in.upper()
    symbol_out = args.asset_out.upper()
    amount_in = float(args.amount_in)
    amount_out = float(args.amount_out)

    _check_asset(symbol_in, "in", cfg)
    _check_asset(symbol_out, "out", cfg)
    _check_amount(symbol_in, amount_in, cfg)

    raw_in = _to_raw(symbol_in, amount_in)
    raw_out = _to_raw(symbol_out, amount_out)
    defuse_in = ASSET_MAP[symbol_in]["defuse_id"]
    defuse_out = ASSET_MAP[symbol_out]["defuse_id"]

    deadline_ms = str(int(time.time() * 1000) + cfg["deadline_ms"])
    nonce = base64.b64encode(random.getrandbits(256).to_bytes(32, byteorder="big")).decode("utf-8")

    intent_message = json.dumps({
        "deadline": deadline_ms,
        "signer_id": account_id,
        "nonce": nonce,
        "verifying_contract": "intents.near",
        "intents": [{
            "intent": "token_diff",
            "diff": {
                defuse_in: f"-{raw_in}",
                defuse_out: raw_out,
            },
        }],
    }, separators=(",", ":"))

    # raw_ed25519 signing: sign the UTF-8 bytes of the JSON message directly
    message_bytes = intent_message.encode("utf-8")
    signed = signing_key.sign(message_bytes)
    signature_bytes = signed.signature  # 64 bytes
    public_key_bytes = signing_key.verify_key.encode()

    signature_str = "ed25519:" + base58.b58encode(signature_bytes).decode("utf-8")
    public_key_str = "ed25519:" + base58.b58encode(public_key_bytes).decode("utf-8")

    signed_data = {
        "standard": "raw_ed25519",
        "payload": intent_message,
        "signature": signature_str,
        "public_key": public_key_str,
    }

    params = [{
        "quote_hashes": [quote_hash],
        "signed_data": signed_data,
    }]

    _audit({
        "action": "publish_intent",
        "account_id": account_id,
        "asset_in": symbol_in,
        "asset_out": symbol_out,
        "amount_in": amount_in,
        "amount_out": amount_out,
        "quote_hash": quote_hash,
    })

    result = _rpc(cfg["solver_relay_url"], "publish_intent", params)
    publish_result = result.get("result", {})

    output = {
        "status": publish_result.get("status", "UNKNOWN"),
        "intent_hash": publish_result.get("intent_hash"),
        "account_id": account_id,
        "asset_in": symbol_in,
        "asset_out": symbol_out,
        "amount_in_human": amount_in,
        "amount_out_human": amount_out,
    }

    if publish_result.get("reason"):
        output["reason"] = publish_result["reason"]

    _audit({"action": "publish_result", "status": output["status"], "intent_hash": output.get("intent_hash")})
    print(json.dumps(output, indent=2))


def cmd_status(args, cfg):
    """Check intent status."""
    params = [{"intent_hash": args.intent_hash}]
    result = _rpc(cfg["solver_relay_url"], "get_status", params)
    print(json.dumps(result.get("result", {}), indent=2))


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
def _die(msg: str):
    output = {"status": "error", "message": msg}
    _audit({"action": "error", "message": msg})
    print(json.dumps(output, indent=2), file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------
def main():
    parser = argparse.ArgumentParser(description="NEAR Intents helper (guardrailed)")
    sub = parser.add_subparsers(dest="command", required=True)

    # quote
    p_quote = sub.add_parser("quote", help="Request swap quotes")
    p_quote.add_argument("--in", dest="asset_in", required=True, help="Input asset symbol")
    p_quote.add_argument("--out", dest="asset_out", required=True, help="Output asset symbol")
    p_quote.add_argument("--amount", required=True, help="Amount to swap (human-readable)")

    # publish
    p_pub = sub.add_parser("publish", help="Publish a signed intent")
    p_pub.add_argument("--quote-hash", required=True, help="Quote hash from solver")
    p_pub.add_argument("--in", dest="asset_in", required=True, help="Input asset symbol")
    p_pub.add_argument("--out", dest="asset_out", required=True, help="Output asset symbol")
    p_pub.add_argument("--amount-in", required=True, help="Amount selling (human-readable)")
    p_pub.add_argument("--amount-out", required=True, help="Amount receiving (human-readable)")
    p_pub.add_argument("--confirm", required=True, help="Must be 'YES' to proceed")

    # status
    p_status = sub.add_parser("status", help="Check intent status")
    p_status.add_argument("--intent-hash", required=True, help="Intent hash to check")

    args = parser.parse_args()
    cfg = _load_config()

    if args.command == "quote":
        cmd_quote(args, cfg)
    elif args.command == "publish":
        cmd_publish(args, cfg)
    elif args.command == "status":
        cmd_status(args, cfg)
    else:
        _die(f"Unknown command: {args.command}")


if __name__ == "__main__":
    main()
