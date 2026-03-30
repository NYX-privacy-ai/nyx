#!/usr/bin/env python3
"""
Portfolio state tracking for Nyx DeFi engine.

Maintains on-disk JSON state for:
  - Current token holdings (with cost basis)
  - Active DeFi positions (stakes, loans, LP)
  - Append-only strategy action log

Portable module — no OpenClaw-specific dependencies.
"""

import json
import time
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from typing import Any, Optional

from near_rpc import NearRpcClient, MAINNET_RPC

# Default state directory (overridable)
DEFAULT_STATE_DIR = Path.home() / ".openclaw" / "defi-state"

# Asset registry (must match near_intents.py)
TOKEN_CONTRACTS = {
    "NEAR": None,  # Native token, no contract
    "WNEAR": "wrap.near",
    "USDC": "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near",
    "USDT": "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
    "STNEAR": "meta-pool.near",
    "AURORA": "aaaaaa20d9e0e2461697782ef11675f668207961.factory.bridge.near",
    "WBTC": "2260fac5e5542a773aa44fbcfedf7c193bc2c599.factory.bridge.near",
    "WETH": "c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2.factory.bridge.near",
}

TOKEN_DECIMALS = {
    "NEAR": 24,
    "WNEAR": 24,
    "USDC": 6,
    "USDT": 6,
    "STNEAR": 24,
    "AURORA": 18,
    "WBTC": 8,
    "WETH": 18,
}


def _raw_to_human(raw: int, decimals: int) -> float:
    """Convert raw token amount to human-readable float."""
    return float(Decimal(str(raw)) / Decimal(10 ** decimals))


def _human_to_raw(human: float, decimals: int) -> int:
    """Convert human-readable amount to raw token amount."""
    return int(Decimal(str(human)) * Decimal(10 ** decimals))


class Portfolio:
    """Manages portfolio state on disk."""

    def __init__(self, account_id: str, state_dir: Optional[Path] = None, rpc: Optional[NearRpcClient] = None):
        self.account_id = account_id
        self.state_dir = state_dir or DEFAULT_STATE_DIR
        self.rpc = rpc or NearRpcClient(MAINNET_RPC)

        self.state_dir.mkdir(parents=True, exist_ok=True)

        self.portfolio_file = self.state_dir / "portfolio.json"
        self.positions_file = self.state_dir / "positions.json"
        self.strategy_log_file = self.state_dir / "strategy_log.jsonl"

    # ------------------------------------------------------------------
    # On-chain balance queries
    # ------------------------------------------------------------------

    def fetch_native_balance(self) -> int:
        """Fetch native NEAR balance in yoctoNEAR."""
        return self.rpc.get_balance(self.account_id)

    def fetch_token_balance(self, symbol: str) -> int:
        """Fetch NEP-141 token balance in raw units."""
        contract = TOKEN_CONTRACTS.get(symbol)
        if contract is None:
            if symbol == "NEAR":
                return self.fetch_native_balance()
            return 0
        return self.rpc.ft_balance_of(contract, self.account_id)

    def fetch_all_balances(self) -> dict:
        """Fetch all known token balances. Returns {symbol: {"raw": int, "human": float}}."""
        balances = {}
        for symbol in TOKEN_CONTRACTS:
            try:
                raw = self.fetch_token_balance(symbol)
                human = _raw_to_human(raw, TOKEN_DECIMALS[symbol])
                if raw > 0:
                    balances[symbol] = {"raw": raw, "human": human}
            except Exception:
                # Token might not exist for this account yet
                pass
        return balances

    # ------------------------------------------------------------------
    # Portfolio state persistence
    # ------------------------------------------------------------------

    def load_portfolio(self) -> dict:
        """Load portfolio state from disk."""
        defaults = {
            "account_id": self.account_id,
            "holdings": {},
            "total_deposited_usd": 0.0,
            "last_update": None,
        }
        if self.portfolio_file.exists():
            with open(self.portfolio_file, "r") as f:
                data = json.load(f)
            for key, val in defaults.items():
                data.setdefault(key, val)
            return data
        return defaults

    def save_portfolio(self, data: dict):
        """Save portfolio state to disk."""
        data["last_update"] = datetime.now(timezone.utc).isoformat()
        with open(self.portfolio_file, "w") as f:
            json.dump(data, f, indent=2)

    def update_portfolio_from_chain(self, prices_usd: Optional[dict] = None) -> dict:
        """Refresh portfolio from on-chain data.

        Args:
            prices_usd: Optional dict of {symbol: usd_price} for valuation.

        Returns:
            Updated portfolio dict.
        """
        portfolio = self.load_portfolio()
        balances = self.fetch_all_balances()

        holdings = {}
        total_usd = 0.0
        for symbol, bal in balances.items():
            entry = {
                "raw": bal["raw"],
                "human": bal["human"],
            }
            if prices_usd and symbol in prices_usd:
                usd_value = bal["human"] * prices_usd[symbol]
                entry["usd_value"] = round(usd_value, 2)
                total_usd += usd_value
            holdings[symbol] = entry

        portfolio["holdings"] = holdings
        if prices_usd:
            portfolio["total_usd_value"] = round(total_usd, 2)
        self.save_portfolio(portfolio)
        return portfolio

    # ------------------------------------------------------------------
    # Positions state (stakes, loans, LP)
    # ------------------------------------------------------------------

    def load_positions(self) -> dict:
        """Load DeFi positions from disk."""
        defaults = {
            "account_id": self.account_id,
            "staking": {},
            "lending": {},
            "lp": {},
            "last_update": None,
        }
        if self.positions_file.exists():
            with open(self.positions_file, "r") as f:
                data = json.load(f)
            # Merge with defaults to ensure all keys exist
            for key, val in defaults.items():
                data.setdefault(key, val)
            return data
        return defaults

    def save_positions(self, data: dict):
        """Save DeFi positions to disk."""
        data["last_update"] = datetime.now(timezone.utc).isoformat()
        with open(self.positions_file, "w") as f:
            json.dump(data, f, indent=2)

    # ------------------------------------------------------------------
    # Strategy action log
    # ------------------------------------------------------------------

    def log_action(self, action: str, details: dict):
        """Append an action to the strategy log."""
        entry = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "account_id": self.account_id,
            "action": action,
            **details,
        }
        with open(self.strategy_log_file, "a") as f:
            f.write(json.dumps(entry, separators=(",", ":")) + "\n")

    # ------------------------------------------------------------------
    # Summary / reporting
    # ------------------------------------------------------------------

    def generate_report(self, prices_usd: Optional[dict] = None) -> dict:
        """Generate a portfolio summary report."""
        portfolio = self.update_portfolio_from_chain(prices_usd)
        positions = self.load_positions()

        return {
            "account_id": self.account_id,
            "holdings": portfolio.get("holdings", {}),
            "total_usd_value": portfolio.get("total_usd_value"),
            "staking": positions.get("staking", {}),
            "lending": positions.get("lending", {}),
            "lp": positions.get("lp", {}),
            "last_update": portfolio.get("last_update"),
        }
