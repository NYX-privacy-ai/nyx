#!/usr/bin/env python3
"""
NEAR function-call access key management for Nyx.

Creates limited-permission access keys that can ONLY call specific
methods on specific contracts. This limits blast radius if keys are
compromised — they cannot transfer NEAR or call arbitrary contracts.

Contracts covered:
  - wrap.near: near_deposit, near_withdraw, storage_deposit
  - meta-pool.near: deposit_and_stake, liquid_unstake, unstake, withdraw_unstaked, storage_deposit
  - contract.main.burrow.near: execute, storage_deposit
  - Token contracts (USDC, USDT, etc.): ft_transfer_call, storage_deposit

The full-access key is retained for administrative operations
(adding/removing keys, emergency transfers). Day-to-day DeFi operations
use the function-call keys.

Portable module — no OpenClaw-specific dependencies.
"""

import argparse
import json
import sys
from decimal import Decimal
from pathlib import Path
from typing import Optional

import base58
import nacl.signing

from near_rpc import NearRpcClient, MAINNET_RPC
from tx_builder import TransactionBuilder, create_builder, load_signing_key


# ------------------------------------------------------------------
# Contract definitions for function-call keys
# ------------------------------------------------------------------

# Each entry: (receiver_id, allowed_methods, allowance_near)
# Empty method_names list = all methods on that contract allowed
DEFI_KEY_PERMISSIONS = [
    {
        "name": "wrap.near",
        "receiver_id": "wrap.near",
        "method_names": ["near_deposit", "near_withdraw", "storage_deposit", "ft_transfer_call"],
        "allowance_near": 5.0,  # 5 NEAR for gas
    },
    {
        "name": "meta-pool.near",
        "receiver_id": "meta-pool.near",
        "method_names": [
            "deposit_and_stake", "liquid_unstake", "unstake",
            "withdraw_unstaked", "get_account_info", "storage_deposit",
        ],
        "allowance_near": 5.0,
    },
    {
        "name": "contract.main.burrow.near",
        "receiver_id": "contract.main.burrow.near",
        "method_names": ["execute", "storage_deposit"],
        "allowance_near": 5.0,
    },
    {
        "name": "usdc.bridge",
        "receiver_id": "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near",
        "method_names": ["ft_transfer_call", "storage_deposit"],
        "allowance_near": 2.0,
    },
    {
        "name": "usdt.bridge",
        "receiver_id": "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
        "method_names": ["ft_transfer_call", "storage_deposit"],
        "allowance_near": 2.0,
    },
]

SECRETS_DIR = Path.home() / ".openclaw" / "secrets"
KEYS_FILE = SECRETS_DIR / "function_call_keys.json"
NEAR_DECIMALS = 24


def _near_to_yocto(amount: float) -> int:
    return int(Decimal(str(amount)) * Decimal(10 ** NEAR_DECIMALS))


def _output(data: dict):
    print(json.dumps(data, indent=2, default=str))


def _die(msg: str):
    print(json.dumps({"status": "error", "message": msg}, indent=2), file=sys.stderr)
    sys.exit(1)


# ------------------------------------------------------------------
# Key generation and storage
# ------------------------------------------------------------------

def _generate_keypair():
    """Generate a new ED25519 keypair for a function-call key."""
    signing_key = nacl.signing.SigningKey.generate()
    verify_key = signing_key.verify_key
    public_key_bytes = verify_key.encode()
    # Store as ed25519:base58(seed + public_key) like NEAR wallet format
    seed = signing_key.encode()
    private_key_str = "ed25519:" + base58.b58encode(seed + public_key_bytes).decode("utf-8")
    public_key_str = "ed25519:" + base58.b58encode(public_key_bytes).decode("utf-8")
    return {
        "public_key_bytes": public_key_bytes,
        "public_key_str": public_key_str,
        "private_key_str": private_key_str,
        "signing_key": signing_key,
    }


def _load_keys_store() -> dict:
    """Load the function-call keys store."""
    if KEYS_FILE.exists():
        with open(KEYS_FILE, "r") as f:
            return json.load(f)
    return {"keys": {}}


def _save_keys_store(store: dict):
    """Save the function-call keys store."""
    KEYS_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(KEYS_FILE, "w") as f:
        json.dump(store, f, indent=2)
    KEYS_FILE.chmod(0o600)


# ------------------------------------------------------------------
# Command: deploy-keys
# ------------------------------------------------------------------

def cmd_deploy_keys(args):
    """Deploy function-call access keys for all DeFi contracts.

    For each contract in DEFI_KEY_PERMISSIONS:
    1. Generate a new ED25519 keypair
    2. Add the key to the NEAR account with function-call permission
    3. Store the keypair in function_call_keys.json

    Uses the full-access key to add the new keys.
    """
    confirm = getattr(args, "confirm", "")
    if confirm != "YES":
        _die("Deploy keys requires --confirm YES")

    account_id, signing_key = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    builder = TransactionBuilder(account_id, signing_key, rpc)

    store = _load_keys_store()
    deployed = []
    errors = []

    for perm in DEFI_KEY_PERMISSIONS:
        name = perm["name"]

        # Skip if already deployed
        if name in store.get("keys", {}) and not getattr(args, "force", False):
            deployed.append({
                "contract": name,
                "status": "already_exists",
                "public_key": store["keys"][name]["public_key"],
            })
            continue

        try:
            # Generate new keypair
            kp = _generate_keypair()

            # Add function-call key to NEAR account
            result = builder.sign_and_submit(
                receiver_id=account_id,
                actions=[{
                    "type": "add_key",
                    "public_key": kp["public_key_bytes"],
                    "nonce": 0,
                    "permission": {
                        "allowance": _near_to_yocto(perm["allowance_near"]),
                        "receiver_id": perm["receiver_id"],
                        "method_names": perm["method_names"],
                    },
                }],
                wait=True,
            )

            # Check for success
            status = result.get("status", {})
            if isinstance(status, dict):
                success_val = status.get("SuccessValue")
                failure = status.get("Failure")
            else:
                success_val = None
                failure = None

            if failure:
                errors.append({"contract": name, "error": str(failure)})
                continue

            # Store the key
            store.setdefault("keys", {})[name] = {
                "public_key": kp["public_key_str"],
                "private_key": kp["private_key_str"],
                "receiver_id": perm["receiver_id"],
                "method_names": perm["method_names"],
                "allowance_near": perm["allowance_near"],
                "deployed_at": __import__("datetime").datetime.now(
                    __import__("datetime").timezone.utc
                ).isoformat(),
            }
            _save_keys_store(store)

            deployed.append({
                "contract": name,
                "status": "deployed",
                "public_key": kp["public_key_str"],
                "receiver_id": perm["receiver_id"],
                "method_names": perm["method_names"],
                "allowance_near": perm["allowance_near"],
            })

        except Exception as e:
            errors.append({"contract": name, "error": str(e)})

    _output({
        "status": "ok",
        "command": "deploy-keys",
        "account_id": account_id,
        "deployed": deployed,
        "errors": errors,
        "total_keys": len(deployed),
    })


# ------------------------------------------------------------------
# Command: list-keys
# ------------------------------------------------------------------

def cmd_list_keys(args):
    """List all access keys on the NEAR account."""
    account_id, _ = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)

    # Get on-chain keys
    keys = rpc.view_access_key_list(account_id)

    # Load local key store for matching
    store = _load_keys_store()
    local_keys = {v["public_key"]: k for k, v in store.get("keys", {}).items()}

    key_list = []
    for key_info in keys:
        pub_key = key_info.get("public_key", "")
        access_key = key_info.get("access_key", {})
        nonce = access_key.get("nonce", 0)
        permission = access_key.get("permission")

        entry = {
            "public_key": pub_key,
            "nonce": nonce,
            "local_name": local_keys.get(pub_key, None),
        }

        if isinstance(permission, str) and permission == "FullAccess":
            entry["permission"] = "FullAccess"
        elif isinstance(permission, dict) and "FunctionCall" in permission:
            fc = permission["FunctionCall"]
            entry["permission"] = "FunctionCall"
            entry["receiver_id"] = fc.get("receiver_id", "")
            entry["method_names"] = fc.get("method_names", [])
            allowance = fc.get("allowance")
            if allowance:
                entry["allowance_near"] = float(Decimal(str(allowance)) / Decimal(10 ** 24))
        else:
            entry["permission"] = str(permission)

        key_list.append(entry)

    _output({
        "status": "ok",
        "command": "list-keys",
        "account_id": account_id,
        "keys": key_list,
        "total": len(key_list),
    })


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="NEAR Function-Call Access Key Manager")
    sub = parser.add_subparsers(dest="command", required=True)

    # deploy-keys
    p_deploy = sub.add_parser("deploy-keys", help="Deploy function-call access keys for DeFi contracts")
    p_deploy.add_argument("--confirm", required=True, help="Must be 'YES'")
    p_deploy.add_argument("--force", action="store_true", help="Redeploy even if keys already exist")

    # list-keys
    sub.add_parser("list-keys", help="List all access keys on the account")

    args = parser.parse_args()

    try:
        if args.command == "deploy-keys":
            cmd_deploy_keys(args)
        elif args.command == "list-keys":
            cmd_list_keys(args)
        else:
            _die(f"Unknown command: {args.command}")
    except Exception as e:
        _die(str(e))


if __name__ == "__main__":
    main()
