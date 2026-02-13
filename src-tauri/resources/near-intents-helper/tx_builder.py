#!/usr/bin/env python3
"""
NEAR transaction builder with Borsh serialization.

Constructs, signs, and submits NEAR protocol transactions.
Supports Transfer and FunctionCall actions.

Portable module — no OpenClaw-specific dependencies.
"""

import base64
import hashlib
import json
import struct
from typing import List, Optional, Union

import base58
import nacl.signing

from near_rpc import NearRpcClient, MAINNET_RPC


# ------------------------------------------------------------------
# Borsh serialization helpers
# ------------------------------------------------------------------

def _borsh_u8(value: int) -> bytes:
    return struct.pack("<B", value)

def _borsh_u32(value: int) -> bytes:
    return struct.pack("<I", value)

def _borsh_u64(value: int) -> bytes:
    return struct.pack("<Q", value)

def _borsh_u128(value: int) -> bytes:
    return value.to_bytes(16, byteorder="little")

def _borsh_string(s: str) -> bytes:
    encoded = s.encode("utf-8")
    return _borsh_u32(len(encoded)) + encoded

def _borsh_bytes(b: bytes) -> bytes:
    return _borsh_u32(len(b)) + b

def _borsh_vec(items: list, serializer) -> bytes:
    result = _borsh_u32(len(items))
    for item in items:
        result += serializer(item)
    return result

def _borsh_public_key(public_key_bytes: bytes) -> bytes:
    """Serialize a public key (KeyType::ED25519 = 0, then 32 bytes)."""
    return _borsh_u8(0) + public_key_bytes

def _borsh_block_hash(block_hash_bytes: bytes) -> bytes:
    """Block hash is 32 raw bytes (no length prefix)."""
    return block_hash_bytes


# ------------------------------------------------------------------
# Action types (NEAR protocol)
# ------------------------------------------------------------------

# Action enum variants
ACTION_CREATE_ACCOUNT = 0
ACTION_DEPLOY_CONTRACT = 1
ACTION_FUNCTION_CALL = 2
ACTION_TRANSFER = 3
ACTION_STAKE = 4
ACTION_ADD_KEY = 5
ACTION_DELETE_KEY = 6
ACTION_DELETE_ACCOUNT = 7


def serialize_transfer(amount_yocto: int) -> bytes:
    """Serialize a Transfer action."""
    return _borsh_u8(ACTION_TRANSFER) + _borsh_u128(amount_yocto)


def serialize_function_call(
    method_name: str,
    args: Union[bytes, str, dict],
    gas: int = 30_000_000_000_000,  # 30 TGas default
    deposit: int = 0,
) -> bytes:
    """Serialize a FunctionCall action."""
    if isinstance(args, dict):
        args_bytes = json.dumps(args).encode("utf-8")
    elif isinstance(args, str):
        args_bytes = args.encode("utf-8")
    else:
        args_bytes = args

    return (
        _borsh_u8(ACTION_FUNCTION_CALL)
        + _borsh_string(method_name)
        + _borsh_bytes(args_bytes)
        + _borsh_u64(gas)
        + _borsh_u128(deposit)
    )


def serialize_add_key(
    public_key_bytes: bytes,
    nonce: int = 0,
    permission: Optional[dict] = None,
) -> bytes:
    """Serialize an AddKey action.

    Args:
        public_key_bytes: 32-byte ED25519 public key to add.
        nonce: Starting nonce for the new key (usually 0).
        permission: None for FullAccess, or dict with:
            - "allowance": int (yoctoNEAR allowance, None for unlimited)
            - "receiver_id": str (contract the key can call)
            - "method_names": list[str] (allowed methods, empty = all)
    """
    result = _borsh_u8(ACTION_ADD_KEY)
    # Public key
    result += _borsh_public_key(public_key_bytes)
    # Access key: nonce + permission
    result += _borsh_u64(nonce)

    if permission is None:
        # FullAccess: enum variant 1
        result += _borsh_u8(1)
    else:
        # FunctionCall: enum variant 0
        result += _borsh_u8(0)
        # Allowance: Option<u128>
        allowance = permission.get("allowance")
        if allowance is not None:
            result += _borsh_u8(1)  # Some
            result += _borsh_u128(allowance)
        else:
            result += _borsh_u8(0)  # None (unlimited)
        # Receiver ID
        result += _borsh_string(permission["receiver_id"])
        # Method names: Vec<String>
        method_names = permission.get("method_names", [])
        result += _borsh_u32(len(method_names))
        for name in method_names:
            result += _borsh_string(name)

    return result


def serialize_delete_key(public_key_bytes: bytes) -> bytes:
    """Serialize a DeleteKey action."""
    return _borsh_u8(ACTION_DELETE_KEY) + _borsh_public_key(public_key_bytes)


def serialize_action(action: dict) -> bytes:
    """Serialize an action from a dict spec.

    Supported action types:
        {"type": "transfer", "amount": int}
        {"type": "function_call", "method": str, "args": ..., "gas": int, "deposit": int}
        {"type": "add_key", "public_key": bytes, "nonce": int, "permission": dict|None}
        {"type": "delete_key", "public_key": bytes}
    """
    action_type = action["type"]
    if action_type == "transfer":
        return serialize_transfer(action["amount"])
    elif action_type == "function_call":
        return serialize_function_call(
            method_name=action["method"],
            args=action.get("args", {}),
            gas=action.get("gas", 30_000_000_000_000),
            deposit=action.get("deposit", 0),
        )
    elif action_type == "add_key":
        return serialize_add_key(
            public_key_bytes=action["public_key"],
            nonce=action.get("nonce", 0),
            permission=action.get("permission"),
        )
    elif action_type == "delete_key":
        return serialize_delete_key(action["public_key"])
    else:
        raise ValueError(f"Unsupported action type: {action_type}")


# ------------------------------------------------------------------
# Transaction serialization
# ------------------------------------------------------------------

def serialize_transaction(
    signer_id: str,
    public_key_bytes: bytes,
    nonce: int,
    receiver_id: str,
    block_hash_bytes: bytes,
    actions: List[bytes],
) -> bytes:
    """Serialize a NEAR transaction (unsigned) in Borsh format."""
    result = _borsh_string(signer_id)
    result += _borsh_public_key(public_key_bytes)
    result += _borsh_u64(nonce)
    result += _borsh_string(receiver_id)
    result += _borsh_block_hash(block_hash_bytes)
    result += _borsh_u32(len(actions))
    for action in actions:
        result += action
    return result


def serialize_signed_transaction(
    tx_bytes: bytes,
    signature_bytes: bytes,
    public_key_bytes: bytes,
) -> bytes:
    """Wrap a serialized transaction with its signature."""
    result = tx_bytes
    # Signature: enum variant 0 (ED25519) + 64 bytes
    result += _borsh_u8(0) + signature_bytes
    return result


# ------------------------------------------------------------------
# Transaction builder class
# ------------------------------------------------------------------

class TransactionBuilder:
    """Build, sign, and submit NEAR transactions."""

    def __init__(
        self,
        account_id: str,
        signing_key: nacl.signing.SigningKey,
        rpc: Optional[NearRpcClient] = None,
    ):
        self.account_id = account_id
        self.signing_key = signing_key
        self.public_key_bytes = signing_key.verify_key.encode()
        self.public_key_str = "ed25519:" + base58.b58encode(self.public_key_bytes).decode("utf-8")
        self.rpc = rpc or NearRpcClient(MAINNET_RPC)

    def _get_nonce(self) -> int:
        """Get the next nonce for this account's access key."""
        key_info = self.rpc.view_access_key(self.account_id, self.public_key_str)
        return key_info["nonce"] + 1

    def _get_block_hash(self) -> bytes:
        """Get the latest block hash as raw bytes."""
        block = self.rpc.get_block()
        block_hash_b58 = block["header"]["hash"]
        return base58.b58decode(block_hash_b58)

    def build_and_sign(
        self,
        receiver_id: str,
        actions: List[dict],
    ) -> str:
        """Build, serialize, sign a transaction. Returns base64-encoded signed tx.

        Args:
            receiver_id: The contract or account to interact with.
            actions: List of action dicts (see serialize_action).

        Returns:
            Base64-encoded signed transaction ready for broadcast.
        """
        nonce = self._get_nonce()
        block_hash = self._get_block_hash()

        serialized_actions = [serialize_action(a) for a in actions]

        tx_bytes = serialize_transaction(
            signer_id=self.account_id,
            public_key_bytes=self.public_key_bytes,
            nonce=nonce,
            receiver_id=receiver_id,
            block_hash_bytes=block_hash,
            actions=serialized_actions,
        )

        # Sign the SHA-256 hash of the serialized transaction
        tx_hash = hashlib.sha256(tx_bytes).digest()
        signed = self.signing_key.sign(tx_hash)
        signature_bytes = signed.signature  # 64 bytes

        # Serialize the signed transaction
        signed_tx = serialize_signed_transaction(tx_bytes, signature_bytes, self.public_key_bytes)

        return base64.b64encode(signed_tx).decode("utf-8")

    def sign_and_submit(
        self,
        receiver_id: str,
        actions: List[dict],
        wait: bool = True,
    ) -> dict:
        """Build, sign, and submit a transaction.

        Args:
            receiver_id: The contract or account to interact with.
            actions: List of action dicts.
            wait: If True, use broadcast_tx_commit (wait for execution).
                  If False, use broadcast_tx_async (return tx hash immediately).

        Returns:
            Transaction result dict (if wait=True) or {"tx_hash": str} (if wait=False).
        """
        signed_tx_b64 = self.build_and_sign(receiver_id, actions)

        if wait:
            return self.rpc.send_tx_commit(signed_tx_b64)
        else:
            tx_hash = self.rpc.send_tx_async(signed_tx_b64)
            return {"tx_hash": tx_hash}

    # ------------------------------------------------------------------
    # Convenience methods
    # ------------------------------------------------------------------

    def transfer(self, receiver_id: str, amount_yocto: int, wait: bool = True) -> dict:
        """Send NEAR to another account."""
        return self.sign_and_submit(
            receiver_id=receiver_id,
            actions=[{"type": "transfer", "amount": amount_yocto}],
            wait=wait,
        )

    def function_call(
        self,
        contract_id: str,
        method: str,
        args: Union[dict, bytes, str] = None,
        gas: int = 30_000_000_000_000,
        deposit: int = 0,
        wait: bool = True,
    ) -> dict:
        """Call a function on a contract."""
        return self.sign_and_submit(
            receiver_id=contract_id,
            actions=[{
                "type": "function_call",
                "method": method,
                "args": args or {},
                "gas": gas,
                "deposit": deposit,
            }],
            wait=wait,
        )


# ------------------------------------------------------------------
# Helper: load signing key from near_account.json
# ------------------------------------------------------------------

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
        raise ValueError(f"Unexpected key length: {len(key_bytes)}")


def load_signing_key(account_file: Optional[str] = None):
    """Load account_id and SigningKey.

    Checks environment variables first (NEAR_ACCOUNT_ID + NEAR_PRIVATE_KEY),
    falling back to the account JSON file only if env vars are not set.

    Returns:
        Tuple of (account_id, nacl.signing.SigningKey)
    """
    import os
    env_key = os.environ.get("NEAR_PRIVATE_KEY")
    env_account = os.environ.get("NEAR_ACCOUNT_ID")
    if env_key and env_account:
        return env_account, _parse_near_key(env_key)

    from pathlib import Path
    path = Path(account_file) if account_file else (Path.home() / ".openclaw" / "secrets" / "near_account.json")
    with open(path, "r") as f:
        data = json.load(f)

    return data["account_id"], _parse_near_key(data["private_key"])


def create_builder(account_file: Optional[str] = None, rpc: Optional[NearRpcClient] = None) -> TransactionBuilder:
    """Create a TransactionBuilder from the account JSON file."""
    account_id, signing_key = load_signing_key(account_file)
    return TransactionBuilder(account_id, signing_key, rpc)
