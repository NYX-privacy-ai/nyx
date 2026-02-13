#!/usr/bin/env python3
"""
Price oracle for Nyx DeFi engine.

Fetches token prices from:
  1. Ref Finance on-chain pools (primary, decentralized)
  2. CoinGecko API (fallback, for USD prices)

Portable module — no OpenClaw-specific dependencies.
"""

import json
import time
from decimal import Decimal
from typing import Dict, Optional

import requests

from near_rpc import NearRpcClient, MAINNET_RPC

# Ref Finance V2 contract
REF_FINANCE = "v2.ref-finance.near"

# Known Ref Finance pool IDs for key pairs
# These are the most liquid pools on Ref Finance
REF_POOL_IDS = {
    ("WNEAR", "USDC"): 4179,    # wNEAR/USDC.e stable pool
    ("WNEAR", "USDT"): 4513,    # wNEAR/USDT.e
    ("STNEAR", "WNEAR"): 5,     # stNEAR/wNEAR
    ("WNEAR", "AURORA"): 21,    # wNEAR/AURORA
}

# CoinGecko IDs for fallback
COINGECKO_IDS = {
    "NEAR": "near",
    "WNEAR": "near",
    "USDC": "usd-coin",
    "USDT": "tether",
    "STNEAR": "staked-near",
    "WBTC": "wrapped-bitcoin",
    "WETH": "weth",
    "AURORA": "aurora-near",
}

# Cache TTL in seconds
CACHE_TTL = 60

# Token decimals (duplicated from portfolio.py for independence)
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


class PriceCache:
    """Simple in-memory price cache with TTL."""

    def __init__(self, ttl: int = CACHE_TTL):
        self.ttl = ttl
        self._cache: Dict[str, tuple] = {}  # {key: (value, timestamp)}

    def get(self, key: str) -> Optional[float]:
        if key in self._cache:
            value, ts = self._cache[key]
            if time.time() - ts < self.ttl:
                return value
            del self._cache[key]
        return None

    def set(self, key: str, value: float):
        self._cache[key] = (value, time.time())


_cache = PriceCache()


class PriceOracle:
    """Multi-source price oracle."""

    def __init__(self, rpc: Optional[NearRpcClient] = None):
        self.rpc = rpc or NearRpcClient(MAINNET_RPC)

    # ------------------------------------------------------------------
    # Ref Finance on-chain prices
    # ------------------------------------------------------------------

    def get_ref_pool(self, pool_id: int) -> dict:
        """Get Ref Finance pool info."""
        result = self.rpc.view_function(
            REF_FINANCE,
            "get_pool",
            {"pool_id": pool_id},
        )
        return result or {}

    def get_ref_price(self, token_a: str, token_b: str) -> Optional[float]:
        """Get price of token_a in terms of token_b from Ref Finance.

        Returns: price (how many token_b per 1 token_a), or None if unavailable.
        """
        cache_key = f"ref:{token_a}/{token_b}"
        cached = _cache.get(cache_key)
        if cached is not None:
            return cached

        pool_id = REF_POOL_IDS.get((token_a, token_b))
        reverse = False
        if pool_id is None:
            pool_id = REF_POOL_IDS.get((token_b, token_a))
            reverse = True
        if pool_id is None:
            return None

        try:
            pool = self.get_ref_pool(pool_id)
            amounts = pool.get("amounts", [])
            if len(amounts) < 2:
                return None

            token_ids = pool.get("token_account_ids", [])
            if len(token_ids) < 2:
                return None

            amount_0 = int(amounts[0])
            amount_1 = int(amounts[1])

            if amount_0 == 0 or amount_1 == 0:
                return None

            # Determine which token is which in the pool
            dec_0 = TOKEN_DECIMALS.get(token_a, 24) if not reverse else TOKEN_DECIMALS.get(token_b, 24)
            dec_1 = TOKEN_DECIMALS.get(token_b, 6) if not reverse else TOKEN_DECIMALS.get(token_a, 6)

            human_0 = Decimal(str(amount_0)) / Decimal(10 ** dec_0)
            human_1 = Decimal(str(amount_1)) / Decimal(10 ** dec_1)

            if reverse:
                price = float(human_0 / human_1)
            else:
                price = float(human_1 / human_0)

            _cache.set(cache_key, price)
            return price
        except Exception:
            return None

    # ------------------------------------------------------------------
    # CoinGecko prices (USD)
    # ------------------------------------------------------------------

    def get_coingecko_prices(self, symbols: list) -> Dict[str, float]:
        """Fetch USD prices from CoinGecko for multiple tokens.

        Args:
            symbols: List of token symbols (e.g., ["NEAR", "USDC"]).

        Returns:
            Dict of {symbol: usd_price}.
        """
        # Check cache first
        result = {}
        to_fetch = []
        for s in symbols:
            cached = _cache.get(f"cg:{s}")
            if cached is not None:
                result[s] = cached
            else:
                to_fetch.append(s)

        if not to_fetch:
            return result

        # Build CoinGecko request
        ids = []
        id_to_symbol = {}
        for s in to_fetch:
            cg_id = COINGECKO_IDS.get(s)
            if cg_id:
                ids.append(cg_id)
                id_to_symbol[cg_id] = s

        if not ids:
            return result

        try:
            resp = requests.get(
                "https://api.coingecko.com/api/v3/simple/price",
                params={"ids": ",".join(ids), "vs_currencies": "usd"},
                timeout=10,
            )
            resp.raise_for_status()
            data = resp.json()

            for cg_id, price_data in data.items():
                symbol = id_to_symbol.get(cg_id)
                if symbol and "usd" in price_data:
                    price = price_data["usd"]
                    result[symbol] = price
                    _cache.set(f"cg:{symbol}", price)
        except Exception:
            pass

        # Stablecoins fallback
        for s in ["USDC", "USDT"]:
            if s in symbols and s not in result:
                result[s] = 1.0

        return result

    # ------------------------------------------------------------------
    # Combined price lookup
    # ------------------------------------------------------------------

    def get_all_prices_usd(self, symbols: Optional[list] = None) -> Dict[str, float]:
        """Get USD prices for all tracked tokens.

        Uses CoinGecko as the primary USD source.
        Falls back to Ref Finance pool ratios + NEAR/USD from CoinGecko.
        """
        if symbols is None:
            symbols = list(TOKEN_DECIMALS.keys())

        prices = self.get_coingecko_prices(symbols)

        # NEAR and WNEAR should have the same price
        if "NEAR" in prices and "WNEAR" not in prices:
            prices["WNEAR"] = prices["NEAR"]
        elif "WNEAR" in prices and "NEAR" not in prices:
            prices["NEAR"] = prices["WNEAR"]

        # Try to derive missing prices from Ref Finance + NEAR/USD
        near_usd = prices.get("NEAR")
        if near_usd:
            for symbol in symbols:
                if symbol not in prices and symbol not in ("NEAR", "WNEAR"):
                    ref_price = self.get_ref_price(symbol, "WNEAR")
                    if ref_price is not None:
                        prices[symbol] = ref_price * near_usd

        return prices

    def get_near_usd(self) -> float:
        """Get NEAR/USD price."""
        prices = self.get_coingecko_prices(["NEAR"])
        return prices.get("NEAR", 0.0)

    def get_stnear_near_ratio(self) -> Optional[float]:
        """Get stNEAR/NEAR exchange rate from Meta Pool contract."""
        try:
            result = self.rpc.view_function("meta-pool.near", "get_st_near_price")
            if result:
                return float(Decimal(str(int(result))) / Decimal(10 ** 24))
        except Exception:
            pass
        return None


# ------------------------------------------------------------------
# Module-level convenience
# ------------------------------------------------------------------

_default_oracle: Optional[PriceOracle] = None


def get_oracle(rpc: Optional[NearRpcClient] = None) -> PriceOracle:
    global _default_oracle
    if _default_oracle is None:
        _default_oracle = PriceOracle(rpc)
    return _default_oracle


def get_all_prices_usd(symbols: Optional[list] = None) -> Dict[str, float]:
    return get_oracle().get_all_prices_usd(symbols)


def get_near_usd() -> float:
    return get_oracle().get_near_usd()
