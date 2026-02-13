#!/usr/bin/env python3
"""
Generate a NEAR implicit account (ED25519 keypair).

An implicit account's ID is the hex-encoded public key (64 chars).
The account comes alive on mainnet when it receives its first deposit.

Usage:
    python3 generate_wallet.py [--output PATH]

Output: JSON file with account_id and private_key (ed25519:base58).
"""

import json
import sys
from pathlib import Path

import nacl.signing
import base58


def generate_implicit_account():
    """Generate an ED25519 keypair and derive the implicit account ID."""
    signing_key = nacl.signing.SigningKey.generate()
    verify_key = signing_key.verify_key

    # NEAR implicit account ID = hex of public key
    public_key_bytes = verify_key.encode()
    account_id = public_key_bytes.hex()

    # Full 64-byte key: seed (32) + public (32), base58-encoded
    full_key_bytes = signing_key.encode() + public_key_bytes
    private_key_b58 = "ed25519:" + base58.b58encode(full_key_bytes).decode("utf-8")
    public_key_b58 = "ed25519:" + base58.b58encode(public_key_bytes).decode("utf-8")

    return {
        "account_id": account_id,
        "private_key": private_key_b58,
        "public_key": public_key_b58,
    }


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Generate NEAR implicit account")
    parser.add_argument(
        "--output",
        default=str(Path.home() / ".openclaw" / "secrets" / "near_account.json"),
        help="Output path for account JSON",
    )
    args = parser.parse_args()

    output_path = Path(args.output)

    if output_path.exists():
        print(f"ERROR: {output_path} already exists. Remove it first to regenerate.", file=sys.stderr)
        sys.exit(1)

    account = generate_implicit_account()

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w") as f:
        json.dump(
            {"account_id": account["account_id"], "private_key": account["private_key"]},
            f,
            indent=2,
        )
    output_path.chmod(0o600)

    # Print public info only (NEVER print private key)
    print(json.dumps({
        "status": "ok",
        "account_id": account["account_id"],
        "public_key": account["public_key"],
        "file": str(output_path),
        "next_step": f"Fund this account by sending NEAR to {account['account_id']}",
    }, indent=2))


if __name__ == "__main__":
    main()
