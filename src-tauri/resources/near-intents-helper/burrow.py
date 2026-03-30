#!/usr/bin/env python3
"""
Burrow Finance lending/borrowing integration.

Contract: contract.main.burrow.near

Supports:
  - Supply tokens as collateral (earn supply APY)
  - Borrow tokens against collateral
  - Repay borrowed tokens
  - Withdraw supplied tokens
  - Health factor monitoring

Portable module — no OpenClaw-specific dependencies.
"""

import json
from decimal import Decimal
from typing import Dict, List, Optional

from near_rpc import NearRpcClient, MAINNET_RPC
from tx_builder import TransactionBuilder, create_builder

# Burrow contract
BURROW = "contract.main.burrow.near"

# Gas
GAS_100T = 100_000_000_000_000
GAS_200T = 200_000_000_000_000
GAS_300T = 300_000_000_000_000

# 1 yoctoNEAR
ONE_YOCTO = 1

# Token contracts needed for ft_transfer_call to Burrow
TOKEN_CONTRACTS = {
    "WNEAR": "wrap.near",
    "USDC": "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near",
    "USDT": "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
    "STNEAR": "meta-pool.near",
    "WBTC": "2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near",
    "WETH": "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2.factory.bridge.near",
}

TOKEN_DECIMALS = {
    "WNEAR": 24,
    "USDC": 6,
    "USDT": 6,
    "STNEAR": 24,
    "WBTC": 8,
    "WETH": 18,
}


class Burrow:
    """Burrow Finance lending protocol operations."""

    def __init__(self, builder: TransactionBuilder):
        self.builder = builder

    # ------------------------------------------------------------------
    # Supply (deposit collateral)
    # ------------------------------------------------------------------

    def supply(self, token_symbol: str, amount_raw: str, wait: bool = True) -> dict:
        """Supply tokens to Burrow as collateral via ft_transfer_call.

        Burrow uses ft_transfer_call on the token contract, with a JSON msg
        telling Burrow to treat it as a supply action.

        Args:
            token_symbol: Token symbol (e.g., "WNEAR", "USDC").
            amount_raw: Amount in raw/smallest units as string.
        """
        token_contract = TOKEN_CONTRACTS.get(token_symbol)
        if not token_contract:
            raise ValueError(f"Unknown token: {token_symbol}")

        msg = json.dumps({"Execute": {"actions": [{"IncreaseCollateral": {"token_id": token_contract, "max_amount": amount_raw}}]}})

        return self.builder.function_call(
            contract_id=token_contract,
            method="ft_transfer_call",
            args={
                "receiver_id": BURROW,
                "amount": amount_raw,
                "msg": msg,
            },
            gas=GAS_300T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    # ------------------------------------------------------------------
    # Borrow
    # ------------------------------------------------------------------

    def borrow(self, token_symbol: str, amount_raw: str, wait: bool = True) -> dict:
        """Borrow tokens from Burrow.

        Args:
            token_symbol: Token to borrow (e.g., "USDC").
            amount_raw: Amount to borrow in raw units as string.
        """
        token_contract = TOKEN_CONTRACTS.get(token_symbol)
        if not token_contract:
            raise ValueError(f"Unknown token: {token_symbol}")

        return self.builder.function_call(
            contract_id=BURROW,
            method="execute",
            args={
                "actions": [{"Borrow": {"token_id": token_contract, "amount": amount_raw}}],
            },
            gas=GAS_300T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    # ------------------------------------------------------------------
    # Repay
    # ------------------------------------------------------------------

    def repay(self, token_symbol: str, amount_raw: str, wait: bool = True) -> dict:
        """Repay borrowed tokens via ft_transfer_call.

        Args:
            token_symbol: Token to repay.
            amount_raw: Amount to repay in raw units as string.
        """
        token_contract = TOKEN_CONTRACTS.get(token_symbol)
        if not token_contract:
            raise ValueError(f"Unknown token: {token_symbol}")

        msg = json.dumps({"Execute": {"actions": [{"Repay": {"token_id": token_contract, "max_amount": amount_raw}}]}})

        return self.builder.function_call(
            contract_id=token_contract,
            method="ft_transfer_call",
            args={
                "receiver_id": BURROW,
                "amount": amount_raw,
                "msg": msg,
            },
            gas=GAS_300T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    # ------------------------------------------------------------------
    # Withdraw
    # ------------------------------------------------------------------

    def withdraw(self, token_symbol: str, amount_raw: str, wait: bool = True) -> dict:
        """Withdraw supplied collateral from Burrow.

        Args:
            token_symbol: Token to withdraw.
            amount_raw: Amount to withdraw in raw units as string.
        """
        token_contract = TOKEN_CONTRACTS.get(token_symbol)
        if not token_contract:
            raise ValueError(f"Unknown token: {token_symbol}")

        return self.builder.function_call(
            contract_id=BURROW,
            method="execute",
            args={
                "actions": [{"DecreaseCollateral": {"token_id": token_contract, "amount": amount_raw}}],
            },
            gas=GAS_300T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    # ------------------------------------------------------------------
    # Account / position queries
    # ------------------------------------------------------------------

    def get_account(self, rpc: Optional[NearRpcClient] = None) -> dict:
        """Get Burrow account info (positions, health factor).

        Returns dict with:
          - account_id
          - supplied: list of {token_id, balance, shares}
          - collateral: list of {token_id, balance, shares}
          - borrowed: list of {token_id, balance, shares}
        """
        client = rpc or self.builder.rpc
        result = client.view_function(
            BURROW,
            "get_account",
            {"account_id": self.builder.account_id},
        )
        return result or {}

    def get_asset(self, token_symbol: str, rpc: Optional[NearRpcClient] = None) -> dict:
        """Get asset info from Burrow (supply APR, borrow APR, etc.)."""
        token_contract = TOKEN_CONTRACTS.get(token_symbol)
        if not token_contract:
            raise ValueError(f"Unknown token: {token_symbol}")
        client = rpc or self.builder.rpc
        result = client.view_function(
            BURROW,
            "get_asset",
            {"token_id": token_contract},
        )
        return result or {}

    def get_assets_paged(self, from_index: int = 0, limit: int = 20, rpc: Optional[NearRpcClient] = None) -> list:
        """Get paginated list of all Burrow assets."""
        client = rpc or self.builder.rpc
        result = client.view_function(
            BURROW,
            "get_assets_paged",
            {"from_index": from_index, "limit": limit},
        )
        return result or []

    # ------------------------------------------------------------------
    # Health factor calculation
    # ------------------------------------------------------------------

    def compute_health_factor(self, account_data: Optional[dict] = None, rpc: Optional[NearRpcClient] = None) -> float:
        """Compute health factor from Burrow account data.

        Health factor = total collateral value / total borrowed value
        If > 1.0, account is healthy. If <= 1.0, liquidation risk.

        Returns float health factor, or float('inf') if no borrows.
        """
        if account_data is None:
            account_data = self.get_account(rpc)

        collateral = account_data.get("collateral", [])
        borrowed = account_data.get("borrowed", [])

        if not borrowed:
            return float("inf")

        # Sum up collateral and borrow values
        # Burrow returns adjusted values that account for collateral factors
        total_collateral = Decimal("0")
        total_borrowed = Decimal("0")

        for c in collateral:
            balance = Decimal(str(c.get("balance", "0")))
            total_collateral += balance

        for b in borrowed:
            balance = Decimal(str(b.get("balance", "0")))
            total_borrowed += balance

        if total_borrowed == 0:
            return float("inf")

        return float(total_collateral / total_borrowed)

    def ensure_storage(self, wait: bool = True) -> dict:
        """Register storage deposit on Burrow if not already registered."""
        return self.builder.function_call(
            contract_id=BURROW,
            method="storage_deposit",
            args={},
            gas=GAS_100T,
            deposit=int(Decimal("0.25") * Decimal(10 ** 24)),  # 0.25 NEAR
            wait=wait,
        )


# ------------------------------------------------------------------
# Convenience
# ------------------------------------------------------------------

def create_burrow(account_file: str = None, rpc: NearRpcClient = None) -> Burrow:
    """Create a Burrow instance from the default account file."""
    return Burrow(create_builder(account_file, rpc))
