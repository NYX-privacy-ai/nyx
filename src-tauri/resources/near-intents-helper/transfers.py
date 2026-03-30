#!/usr/bin/env python3
"""
NEAR transfer helper — send NEAR between accounts.

Used by Finance agent to fund ring-fenced agent wallets.
All transfers are logged to audit.jsonl.

Guardrails:
  - Max single transfer: 50 NEAR (configurable via MAX_TRANSFER_NEAR)
  - Minimum gas reserve: 2 NEAR always kept in sender account
  - All transfers logged to audit trail
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path

import base58
import nacl.signing

from near_rpc import NearRpcClient, MAINNET_RPC
from tx_builder import TransactionBuilder

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
LOG_DIR = Path.home() / ".openclaw" / "defi-state" / "logs"
AUDIT_LOG = LOG_DIR / "audit.jsonl"

YOCTO = 10**24
GAS_RESERVE_NEAR = Decimal("2.0")  # Never let sender drop below this
MAX_TRANSFER_NEAR = Decimal(os.environ.get("MAX_TRANSFER_NEAR", "50.0"))


def _die(msg: str):
    print(json.dumps({"status": "error", "message": msg}))
    sys.exit(1)


def _log_audit(entry: dict):
    """Append to audit log."""
    LOG_DIR.mkdir(parents=True, exist_ok=True)
    entry["timestamp"] = datetime.now(timezone.utc).isoformat()
    with open(AUDIT_LOG, "a") as f:
        f.write(json.dumps(entry) + "\n")


def _load_account():
    """Load NEAR account from env vars (injected at container boundary)."""
    env_key = os.environ.get("NEAR_PRIVATE_KEY")
    env_account = os.environ.get("NEAR_ACCOUNT_ID")
    if not env_key or not env_account:
        _die("NEAR_ACCOUNT_ID and NEAR_PRIVATE_KEY must be set as env vars.")

    if env_key.startswith("ed25519:"):
        env_key = env_key[len("ed25519:"):]
    key_bytes = base58.b58decode(env_key)
    if len(key_bytes) == 64:
        signing_key = nacl.signing.SigningKey(key_bytes[:32])
    elif len(key_bytes) == 32:
        signing_key = nacl.signing.SigningKey(key_bytes)
    else:
        _die(f"Unexpected key length: {len(key_bytes)}")

    return env_account, signing_key


def _get_balance_near(rpc: NearRpcClient, account_id: str) -> Decimal:
    """Get account balance in NEAR."""
    try:
        info = rpc.view_account(account_id)
        return Decimal(info["amount"]) / YOCTO
    except Exception as e:
        _die(f"Failed to check balance for {account_id}: {e}")


def cmd_transfer(args):
    """Execute a NEAR transfer."""
    receiver = args.to
    amount = Decimal(str(args.amount))

    # --- Guardrails ---
    if amount <= 0:
        _die("Transfer amount must be positive.")
    if amount > MAX_TRANSFER_NEAR:
        _die(f"Transfer exceeds max allowed ({MAX_TRANSFER_NEAR} NEAR). "
             f"Set MAX_TRANSFER_NEAR env var to increase.")

    # Load sender account
    account_id, signing_key = _load_account()
    rpc = NearRpcClient(MAINNET_RPC)

    # Check sender balance
    sender_balance = _get_balance_near(rpc, account_id)
    required = amount + GAS_RESERVE_NEAR
    if sender_balance < required:
        _die(f"Insufficient balance. Have {sender_balance:.4f} NEAR, "
             f"need {amount} + {GAS_RESERVE_NEAR} gas reserve = {required} NEAR.")

    # Convert to yoctoNEAR
    amount_yocto = int(amount * YOCTO)

    # Execute transfer
    tx_builder = TransactionBuilder(account_id, signing_key, rpc)

    _log_audit({
        "action": "transfer",
        "from": account_id,
        "to": receiver,
        "amount_near": str(amount),
        "amount_yocto": str(amount_yocto),
        "status": "submitting",
    })

    try:
        result = tx_builder.transfer(receiver, amount_yocto, wait=True)
    except Exception as e:
        _log_audit({
            "action": "transfer",
            "from": account_id,
            "to": receiver,
            "amount_near": str(amount),
            "status": "failed",
            "error": str(e),
        })
        _die(f"Transfer failed: {e}")

    # Extract tx hash
    tx_hash = result.get("transaction", {}).get("hash", "unknown")

    _log_audit({
        "action": "transfer",
        "from": account_id,
        "to": receiver,
        "amount_near": str(amount),
        "tx_hash": tx_hash,
        "status": "success",
    })

    # Return result
    output = {
        "status": "ok",
        "command": "transfer",
        "from": account_id,
        "to": receiver,
        "amount_near": str(amount),
        "tx_hash": tx_hash,
    }
    print(json.dumps(output, indent=2))


def main():
    parser = argparse.ArgumentParser(description="NEAR transfer helper")
    parser.add_argument("--to", required=True, help="Receiver NEAR account ID")
    parser.add_argument("--amount", required=True, type=float,
                        help="Amount in NEAR (human-readable, e.g. 5.0)")
    args = parser.parse_args()
    cmd_transfer(args)


if __name__ == "__main__":
    main()
