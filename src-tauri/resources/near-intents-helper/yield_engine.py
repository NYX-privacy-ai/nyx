#!/usr/bin/env python3
"""
Yield comparison engine for Nyx DeFi engine.

Compares yields across:
  - Meta Pool liquid staking (stNEAR APY)
  - Burrow supply APY
  - Burrow borrow cost

Makes allocation recommendations based on risk-adjusted returns.

Portable module — no OpenClaw-specific dependencies.
"""

import json
import time
from dataclasses import dataclass, asdict
from decimal import Decimal
from typing import Dict, List, Optional

from near_rpc import NearRpcClient, MAINNET_RPC
from oracle import PriceOracle, get_oracle
from burrow import Burrow, BURROW, TOKEN_CONTRACTS


@dataclass
class YieldOpportunity:
    """A yield-bearing opportunity."""
    protocol: str          # "meta_pool", "burrow_supply", "burrow_borrow"
    token: str             # Token symbol
    apy_pct: float         # Annual percentage yield (e.g., 5.2 for 5.2%)
    risk_level: str        # "low", "medium", "high"
    description: str       # Human-readable description
    min_amount: float      # Minimum meaningful amount
    max_capacity: Optional[float] = None  # Max amount the protocol can absorb


@dataclass
class AllocationRecommendation:
    """A recommended portfolio allocation action."""
    action: str            # "stake", "supply", "borrow", "unstake", "withdraw", "hold"
    protocol: str
    token: str
    amount: float          # Human-readable amount
    reason: str
    expected_apy: float
    risk: str


class YieldEngine:
    """Compare yields and recommend allocations."""

    def __init__(self, rpc: Optional[NearRpcClient] = None, oracle: Optional[PriceOracle] = None):
        self.rpc = rpc or NearRpcClient(MAINNET_RPC)
        self.oracle = oracle or get_oracle(self.rpc)

    # ------------------------------------------------------------------
    # Yield data collection
    # ------------------------------------------------------------------

    def get_meta_pool_apy(self) -> Optional[float]:
        """Get Meta Pool staking APY from contract state."""
        try:
            state = self.rpc.view_function("meta-pool.near", "get_contract_state")
            if state:
                # Meta Pool reports reward fee basis points
                # Typical staking APY is ~5% for NEAR PoS
                # The contract state has various metrics
                reward_fee_bp = state.get("reward_fee_bp", 0)
                # Approximate: NEAR staking yield ~5% minus Meta Pool fee
                base_staking_apy = 5.0
                fee_pct = int(reward_fee_bp) / 100.0
                net_apy = base_staking_apy * (1 - fee_pct / 100.0)
                return round(net_apy, 2)
        except Exception:
            pass
        # Fallback estimate
        return 4.5

    def get_burrow_rates(self) -> Dict[str, dict]:
        """Get Burrow supply and borrow rates for all assets.

        Returns:
            Dict of {token_symbol: {"supply_apy": float, "borrow_apy": float}}
        """
        rates = {}
        try:
            assets = self.rpc.view_function(BURROW, "get_assets_paged", {"from_index": 0, "limit": 30})
            if not assets:
                return rates

            for asset_entry in assets:
                token_id = asset_entry[0] if isinstance(asset_entry, list) else asset_entry.get("token_id", "")
                asset_data = asset_entry[1] if isinstance(asset_entry, list) else asset_entry

                # Find matching symbol
                symbol = None
                for sym, contract in TOKEN_CONTRACTS.items():
                    if contract == token_id:
                        symbol = sym
                        break
                if not symbol:
                    continue

                supply_apr = asset_data.get("supply_apr", "0")
                borrow_apr = asset_data.get("borrow_apr", "0")

                # Burrow reports APR as a decimal string (e.g., "0.05" for 5%)
                try:
                    supply_apy_pct = float(Decimal(str(supply_apr)) * 100)
                    borrow_apy_pct = float(Decimal(str(borrow_apr)) * 100)
                except (ValueError, TypeError):
                    supply_apy_pct = 0.0
                    borrow_apy_pct = 0.0

                rates[symbol] = {
                    "supply_apy": round(supply_apy_pct, 2),
                    "borrow_apy": round(borrow_apy_pct, 2),
                }
        except Exception:
            pass

        return rates

    def get_all_opportunities(self) -> List[YieldOpportunity]:
        """Collect all yield opportunities across protocols."""
        opportunities = []

        # Meta Pool staking
        meta_apy = self.get_meta_pool_apy()
        if meta_apy:
            opportunities.append(YieldOpportunity(
                protocol="meta_pool",
                token="NEAR",
                apy_pct=meta_apy,
                risk_level="low",
                description=f"Liquid staking via Meta Pool: NEAR -> stNEAR at ~{meta_apy}% APY",
                min_amount=1.0,
            ))

        # Burrow supply/borrow rates
        burrow_rates = self.get_burrow_rates()
        for symbol, rates in burrow_rates.items():
            if rates["supply_apy"] > 0:
                opportunities.append(YieldOpportunity(
                    protocol="burrow_supply",
                    token=symbol,
                    apy_pct=rates["supply_apy"],
                    risk_level="medium",
                    description=f"Supply {symbol} on Burrow: {rates['supply_apy']}% APY",
                    min_amount=0.1,
                ))
            if rates["borrow_apy"] > 0:
                opportunities.append(YieldOpportunity(
                    protocol="burrow_borrow",
                    token=symbol,
                    apy_pct=-rates["borrow_apy"],  # Negative = cost
                    risk_level="high",
                    description=f"Borrow {symbol} on Burrow: {rates['borrow_apy']}% cost",
                    min_amount=0.1,
                ))

        # Sort by APY descending
        opportunities.sort(key=lambda x: x.apy_pct, reverse=True)
        return opportunities

    # ------------------------------------------------------------------
    # Allocation recommendations
    # ------------------------------------------------------------------

    def recommend_allocation(
        self,
        available_near: float,
        current_positions: dict,
        risk_tolerance: str = "medium",  # "low", "medium", "high"
    ) -> List[AllocationRecommendation]:
        """Generate allocation recommendations.

        Args:
            available_near: Available NEAR for deployment.
            current_positions: Current DeFi positions from portfolio.
            risk_tolerance: User's risk preference.

        Returns:
            List of recommended actions, ordered by priority.
        """
        recommendations = []
        opportunities = self.get_all_opportunities()

        # Reserve 2 NEAR for gas
        deployable = max(0, available_near - 2.0)
        if deployable <= 0:
            return [AllocationRecommendation(
                action="hold",
                protocol="none",
                token="NEAR",
                amount=available_near,
                reason="Insufficient NEAR balance (need >2 NEAR for gas reserve)",
                expected_apy=0.0,
                risk="none",
            )]

        # Filter by risk tolerance
        max_risk = {"low": "low", "medium": "medium", "high": "high"}
        risk_order = {"low": 0, "medium": 1, "high": 2}
        allowed_risk = risk_order[risk_tolerance]

        eligible = [o for o in opportunities if risk_order.get(o.risk_level, 2) <= allowed_risk and o.apy_pct > 0]

        if not eligible:
            return [AllocationRecommendation(
                action="hold",
                protocol="none",
                token="NEAR",
                amount=available_near,
                reason="No yield opportunities match risk tolerance",
                expected_apy=0.0,
                risk="none",
            )]

        # Simple allocation strategy:
        # - 60% to highest APY within risk tolerance
        # - 30% to second highest (if available)
        # - 10% reserved as liquid NEAR
        allocations = [0.6, 0.3]
        reserved = deployable * 0.1

        for i, pct in enumerate(allocations):
            if i >= len(eligible):
                break
            opp = eligible[i]
            amount = round(deployable * pct, 4)

            if opp.protocol == "meta_pool":
                action = "stake"
            elif opp.protocol == "burrow_supply":
                action = "supply"
            else:
                continue

            recommendations.append(AllocationRecommendation(
                action=action,
                protocol=opp.protocol,
                token=opp.token,
                amount=amount,
                reason=opp.description,
                expected_apy=opp.apy_pct,
                risk=opp.risk_level,
            ))

        # Add liquid reserve recommendation
        recommendations.append(AllocationRecommendation(
            action="hold",
            protocol="none",
            token="NEAR",
            amount=round(reserved + 2.0, 4),  # Include gas reserve
            reason="Liquid reserve (gas + opportunistic deployment)",
            expected_apy=0.0,
            risk="none",
        ))

        return recommendations

    def to_report(self) -> dict:
        """Generate a yield comparison report as JSON."""
        opportunities = self.get_all_opportunities()
        return {
            "timestamp": time.time(),
            "opportunities": [asdict(o) for o in opportunities],
            "count": len(opportunities),
        }
