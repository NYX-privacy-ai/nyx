#!/usr/bin/env python3
"""
NEAR JSON-RPC client for mainnet.

Portable module — no OpenClaw-specific dependencies.
Used by the Nyx DeFi engine for on-chain queries and transaction submission.
"""

import base64
import json
from typing import Any, Optional

import requests

MAINNET_RPC = "https://rpc.mainnet.near.org"
ARCHIVAL_RPC = "https://archival-rpc.mainnet.near.org"


class NearRpcError(Exception):
    """Raised when a NEAR RPC call returns an error."""
    def __init__(self, message: str, cause: Optional[dict] = None):
        super().__init__(message)
        self.cause = cause


class NearRpcClient:
    """Minimal NEAR JSON-RPC client."""

    def __init__(self, rpc_url: str = MAINNET_RPC, timeout: int = 15):
        self.rpc_url = rpc_url
        self.timeout = timeout
        self._request_id = 0

    def _call(self, method: str, params: Any) -> dict:
        """Execute a JSON-RPC call."""
        self._request_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
            "params": params,
        }
        resp = requests.post(self.rpc_url, json=payload, timeout=self.timeout)
        resp.raise_for_status()
        data = resp.json()
        if "error" in data:
            raise NearRpcError(
                f"RPC error: {json.dumps(data['error'])}",
                cause=data["error"],
            )
        return data.get("result", {})

    # ------------------------------------------------------------------
    # Account queries
    # ------------------------------------------------------------------

    def view_account(self, account_id: str) -> dict:
        """Get account info (balance, storage, code_hash)."""
        return self._call("query", {
            "request_type": "view_account",
            "finality": "final",
            "account_id": account_id,
        })

    def get_balance(self, account_id: str) -> int:
        """Get native NEAR balance in yoctoNEAR."""
        result = self.view_account(account_id)
        return int(result.get("amount", "0"))

    # ------------------------------------------------------------------
    # Contract view calls
    # ------------------------------------------------------------------

    def view_function(
        self,
        contract_id: str,
        method_name: str,
        args: Optional[dict] = None,
    ) -> Any:
        """Call a view function on a contract. Returns decoded JSON result."""
        args_base64 = ""
        if args is not None:
            args_base64 = base64.b64encode(
                json.dumps(args).encode("utf-8")
            ).decode("utf-8")

        result = self._call("query", {
            "request_type": "call_function",
            "finality": "final",
            "account_id": contract_id,
            "method_name": method_name,
            "args_base64": args_base64,
        })

        # Result bytes are in result["result"] as a list of ints
        result_bytes = bytes(result.get("result", []))
        if not result_bytes:
            return None
        return json.loads(result_bytes.decode("utf-8"))

    # ------------------------------------------------------------------
    # FT (NEP-141) token queries
    # ------------------------------------------------------------------

    def ft_balance_of(self, token_contract: str, account_id: str) -> int:
        """Get fungible token balance (raw, in smallest unit)."""
        result = self.view_function(
            token_contract,
            "ft_balance_of",
            {"account_id": account_id},
        )
        return int(result) if result else 0

    def ft_metadata(self, token_contract: str) -> dict:
        """Get fungible token metadata (name, symbol, decimals, icon)."""
        return self.view_function(token_contract, "ft_metadata") or {}

    # ------------------------------------------------------------------
    # Access keys
    # ------------------------------------------------------------------

    def view_access_key(self, account_id: str, public_key: str) -> dict:
        """Get access key info for an account."""
        return self._call("query", {
            "request_type": "view_access_key",
            "finality": "final",
            "account_id": account_id,
            "public_key": public_key,
        })

    def view_access_key_list(self, account_id: str) -> list:
        """Get all access keys for an account."""
        result = self._call("query", {
            "request_type": "view_access_key_list",
            "finality": "final",
            "account_id": account_id,
        })
        return result.get("keys", [])

    # ------------------------------------------------------------------
    # Block / network info
    # ------------------------------------------------------------------

    def get_status(self) -> dict:
        """Get node status (chain_id, latest_block, sync_info)."""
        return self._call("status", [])

    def get_block(self, finality: str = "final") -> dict:
        """Get latest block."""
        return self._call("block", {"finality": finality})

    def get_gas_price(self, block_hash: Optional[str] = None) -> int:
        """Get gas price in yoctoNEAR. Returns gas_price as int."""
        params = [block_hash] if block_hash else [None]
        result = self._call("gas_price", params)
        return int(result.get("gas_price", "0"))

    # ------------------------------------------------------------------
    # Transaction submission
    # ------------------------------------------------------------------

    def send_tx_commit(self, signed_tx_base64: str) -> dict:
        """Submit a signed transaction and wait for it to complete.

        Args:
            signed_tx_base64: Base64-encoded Borsh-serialized signed transaction.

        Returns:
            Transaction result including status, receipts, etc.
        """
        return self._call("broadcast_tx_commit", [signed_tx_base64])

    def send_tx_async(self, signed_tx_base64: str) -> str:
        """Submit a signed transaction asynchronously.

        Returns:
            Transaction hash string.
        """
        return self._call("broadcast_tx_async", [signed_tx_base64])

    def tx_status(self, tx_hash: str, sender_id: str) -> dict:
        """Check transaction status."""
        return self._call("tx", [tx_hash, sender_id])


# ------------------------------------------------------------------
# Convenience functions (module-level)
# ------------------------------------------------------------------
_default_client: Optional[NearRpcClient] = None


def get_client(rpc_url: str = MAINNET_RPC) -> NearRpcClient:
    """Get or create a default RPC client."""
    global _default_client
    if _default_client is None or _default_client.rpc_url != rpc_url:
        _default_client = NearRpcClient(rpc_url)
    return _default_client


def view_account(account_id: str) -> dict:
    return get_client().view_account(account_id)


def get_balance(account_id: str) -> int:
    return get_client().get_balance(account_id)


def ft_balance_of(token_contract: str, account_id: str) -> int:
    return get_client().ft_balance_of(token_contract, account_id)


def view_function(contract_id: str, method_name: str, args: Optional[dict] = None) -> Any:
    return get_client().view_function(contract_id, method_name, args)
