#!/usr/bin/env python3
"""
NEAR AI Auth Token Generator — NEP-413 off-chain message signing.

Generates a wallet-signed bearer token for market.near.ai authentication.
Protocol: NEP-413 (off-chain message signing with Ed25519).

Usage:
    python3 nearai_auth.py              # prints JSON bearer token to stdout
    python3 nearai_auth.py --bearer     # prints just the base64-encoded token string
"""

import argparse
import base64
import hashlib
import json
import struct
import sys
import time
from typing import Optional

import base58
import nacl.signing


# ---------------------------------------------------------------------------
# NEP-413 constants
# ---------------------------------------------------------------------------
# Tag = 2^31 + 413 (NEP-413 specification)
NEP413_TAG = (1 << 31) + 413  # 2147484061

# NEAR AI specific
NEARAI_MESSAGE = "Welcome to NEAR AI"
NEARAI_RECIPIENT = "ai.near"


# ---------------------------------------------------------------------------
# Borsh serialization helpers (subset needed for NEP-413)
# ---------------------------------------------------------------------------

def _borsh_u32(value: int) -> bytes:
    return struct.pack("<I", value)


def _borsh_string(s: str) -> bytes:
    encoded = s.encode("utf-8")
    return _borsh_u32(len(encoded)) + encoded


def _borsh_option_string(s: Optional[str]) -> bytes:
    """Borsh Option<String>: 0x00 for None, 0x01 + string for Some."""
    if s is None:
        return b"\x00"
    return b"\x01" + _borsh_string(s)


# ---------------------------------------------------------------------------
# NEP-413 payload construction
# ---------------------------------------------------------------------------

def build_nep413_payload(
    message: str,
    nonce: str,
    recipient: str,
    callback_url: Optional[str] = None,
) -> bytes:
    """Borsh-serialize the NEP-413 payload.

    Format (Borsh):
        tag: u32 (little-endian) = 2147484061
        message: String
        nonce: [u8; 32] (fixed 32-byte buffer, UTF-8 padded with zeros)
        recipient: String
        callbackUrl: Option<String>
    """
    result = _borsh_u32(NEP413_TAG)
    result += _borsh_string(message)

    # Nonce must be exactly 32 bytes (zero-padded)
    nonce_bytes = nonce.encode("utf-8")
    if len(nonce_bytes) > 32:
        nonce_bytes = nonce_bytes[:32]
    nonce_bytes = nonce_bytes.ljust(32, b"\x00")
    result += nonce_bytes

    result += _borsh_string(recipient)
    result += _borsh_option_string(callback_url)

    return result


def sign_nep413(
    signing_key: nacl.signing.SigningKey,
    message: str,
    nonce: str,
    recipient: str,
    callback_url: Optional[str] = None,
) -> bytes:
    """Build NEP-413 payload, SHA-256 hash it, and sign with Ed25519.

    Returns the 64-byte signature.
    """
    payload = build_nep413_payload(message, nonce, recipient, callback_url)
    payload_hash = hashlib.sha256(payload).digest()
    signed = signing_key.sign(payload_hash)
    return signed.signature  # 64 bytes


# ---------------------------------------------------------------------------
# Token generation
# ---------------------------------------------------------------------------

def generate_auth_token(
    account_id: str,
    signing_key: nacl.signing.SigningKey,
    message: str = NEARAI_MESSAGE,
    recipient: str = NEARAI_RECIPIENT,
    callback_url: Optional[str] = None,
) -> dict:
    """Generate a NEAR AI auth token using NEP-413 signing.

    Returns a dict suitable for use as a Bearer token.
    """
    # Nonce: current timestamp in milliseconds, zero-padded to 32 chars
    nonce = str(int(time.time() * 1000)).zfill(32)

    # Sign
    signature = sign_nep413(
        signing_key, message, nonce, recipient, callback_url
    )

    # Public key in NEAR format
    public_key_bytes = signing_key.verify_key.encode()
    public_key_str = "ed25519:" + base58.b58encode(public_key_bytes).decode("utf-8")

    # Build token
    token = {
        "account_id": account_id,
        "public_key": public_key_str,
        "signature": base64.b64encode(signature).decode("utf-8"),
        "callback_url": callback_url,
        "message": message,
        "recipient": recipient,
        "nonce": nonce,
    }

    return token


def token_to_bearer(token: dict) -> str:
    """Encode the token dict as a base64 Bearer string."""
    token_json = json.dumps(token, separators=(",", ":"))
    return base64.b64encode(token_json.encode("utf-8")).decode("utf-8")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _load_account():
    """Load NEAR account credentials (same logic as near_intents.py)."""
    import os
    from pathlib import Path

    env_key = os.environ.get("NEAR_PRIVATE_KEY")
    env_account = os.environ.get("NEAR_ACCOUNT_ID")
    if env_key and env_account:
        return env_account, _parse_near_key(env_key)

    secrets_dir = Path.home() / ".openclaw" / "secrets"
    account_file = secrets_dir / "near_account.json"
    if not account_file.exists():
        print(json.dumps({"status": "error", "message": f"Account file not found: {account_file}"}), file=sys.stderr)
        sys.exit(1)
    with open(account_file, "r") as f:
        data = json.load(f)
    return data["account_id"], _parse_near_key(data["private_key"])


def _parse_near_key(raw_key: str) -> nacl.signing.SigningKey:
    """Parse a NEAR ed25519 key string into a nacl SigningKey."""
    if raw_key.startswith("ed25519:"):
        raw_key = raw_key[len("ed25519:"):]
    key_bytes = base58.b58decode(raw_key)
    if len(key_bytes) == 64:
        return nacl.signing.SigningKey(key_bytes[:32])
    elif len(key_bytes) == 32:
        return nacl.signing.SigningKey(key_bytes)
    else:
        print(json.dumps({"status": "error", "message": f"Unexpected key length: {len(key_bytes)}"}), file=sys.stderr)
        sys.exit(1)


def main():
    parser = argparse.ArgumentParser(
        description="Generate NEAR AI auth token (NEP-413 wallet signing)"
    )
    parser.add_argument(
        "--bearer", action="store_true",
        help="Output only the base64-encoded bearer token string"
    )
    parser.add_argument(
        "--message", default=NEARAI_MESSAGE,
        help=f"Message to sign (default: '{NEARAI_MESSAGE}')"
    )
    parser.add_argument(
        "--recipient", default=NEARAI_RECIPIENT,
        help=f"Recipient (default: '{NEARAI_RECIPIENT}')"
    )
    args = parser.parse_args()

    account_id, signing_key = _load_account()
    token = generate_auth_token(
        account_id=account_id,
        signing_key=signing_key,
        message=args.message,
        recipient=args.recipient,
    )

    if args.bearer:
        print(token_to_bearer(token))
    else:
        output = {
            "status": "ok",
            "account_id": account_id,
            "token": token,
            "bearer": token_to_bearer(token),
            "usage": f"Authorization: Bearer {token_to_bearer(token)[:40]}...",
        }
        print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
