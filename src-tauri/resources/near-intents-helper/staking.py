#!/usr/bin/env python3
"""
NEAR liquid staking via Meta Pool and wNEAR wrap/unwrap.

Portable module — no OpenClaw-specific dependencies.

Contracts:
  - wrap.near: wNEAR (NEP-141 wrapped NEAR)
  - meta-pool.near: Meta Pool liquid staking (NEAR -> stNEAR)
"""

import json
from decimal import Decimal
from typing import Optional

from near_rpc import NearRpcClient, MAINNET_RPC
from tx_builder import TransactionBuilder, create_builder

# Contract addresses
WRAP_NEAR = "wrap.near"
META_POOL = "meta-pool.near"

# Gas amounts
GAS_30T = 30_000_000_000_000   # 30 TGas
GAS_50T = 50_000_000_000_000   # 50 TGas
GAS_100T = 100_000_000_000_000  # 100 TGas
GAS_200T = 200_000_000_000_000  # 200 TGas

# 1 yoctoNEAR deposit (required for ft_transfer and storage calls)
ONE_YOCTO = 1

# NEAR decimals
NEAR_DECIMALS = 24


def _near_to_yocto(amount: float) -> int:
    """Convert NEAR to yoctoNEAR."""
    return int(Decimal(str(amount)) * Decimal(10 ** NEAR_DECIMALS))


def _yocto_to_near(yocto: int) -> float:
    """Convert yoctoNEAR to NEAR."""
    return float(Decimal(str(yocto)) / Decimal(10 ** NEAR_DECIMALS))


class WNear:
    """wNEAR (wrap.near) operations: wrap and unwrap NEAR."""

    def __init__(self, builder: TransactionBuilder):
        self.builder = builder

    def wrap(self, amount_near: float, wait: bool = True) -> dict:
        """Wrap NEAR into wNEAR by calling near_deposit on wrap.near.

        This sends native NEAR as a deposit and receives wNEAR in return.
        """
        amount_yocto = _near_to_yocto(amount_near)
        return self.builder.function_call(
            contract_id=WRAP_NEAR,
            method="near_deposit",
            args={},
            gas=GAS_30T,
            deposit=amount_yocto,
            wait=wait,
        )

    def unwrap(self, amount_near: float, wait: bool = True) -> dict:
        """Unwrap wNEAR back to native NEAR by calling near_withdraw."""
        amount_yocto = str(_near_to_yocto(amount_near))
        return self.builder.function_call(
            contract_id=WRAP_NEAR,
            method="near_withdraw",
            args={"amount": amount_yocto},
            gas=GAS_30T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    def balance(self, rpc: Optional[NearRpcClient] = None) -> int:
        """Get wNEAR balance for the builder's account."""
        client = rpc or self.builder.rpc
        return client.ft_balance_of(WRAP_NEAR, self.builder.account_id)

    def ensure_storage(self, wait: bool = True) -> dict:
        """Register storage deposit on wrap.near if not already registered."""
        return self.builder.function_call(
            contract_id=WRAP_NEAR,
            method="storage_deposit",
            args={"account_id": self.builder.account_id},
            gas=GAS_30T,
            deposit=_near_to_yocto(0.00125),  # minimum storage deposit
            wait=wait,
        )


class MetaPool:
    """Meta Pool (meta-pool.near) liquid staking operations."""

    def __init__(self, builder: TransactionBuilder):
        self.builder = builder

    def deposit_and_stake(self, amount_near: float, wait: bool = True) -> dict:
        """Deposit NEAR and receive stNEAR.

        Sends native NEAR as deposit to meta-pool.near.
        Returns stNEAR at the current exchange rate.
        """
        amount_yocto = _near_to_yocto(amount_near)
        return self.builder.function_call(
            contract_id=META_POOL,
            method="deposit_and_stake",
            args={},
            gas=GAS_200T,
            deposit=amount_yocto,
            wait=wait,
        )

    def liquid_unstake(self, stnear_amount: float, min_expected_near: float = 0, wait: bool = True) -> dict:
        """Instantly unstake stNEAR back to NEAR (small fee ~0.3%).

        Args:
            stnear_amount: Amount of stNEAR to unstake.
            min_expected_near: Minimum NEAR to receive (slippage protection).
        """
        amount_yocto = str(_near_to_yocto(stnear_amount))
        min_yocto = str(_near_to_yocto(min_expected_near))
        return self.builder.function_call(
            contract_id=META_POOL,
            method="liquid_unstake",
            args={
                "st_near_to_burn": amount_yocto,
                "min_expected_near": min_yocto,
            },
            gas=GAS_200T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    def delayed_unstake(self, stnear_amount: float, wait: bool = True) -> dict:
        """Delayed unstake stNEAR (2-3 epochs, no fee).

        After the waiting period, call withdraw_unstaked to collect.
        """
        amount_yocto = str(_near_to_yocto(stnear_amount))
        return self.builder.function_call(
            contract_id=META_POOL,
            method="unstake",
            args={"amount": amount_yocto},
            gas=GAS_200T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    def withdraw_unstaked(self, wait: bool = True) -> dict:
        """Withdraw NEAR after delayed unstake completes."""
        return self.builder.function_call(
            contract_id=META_POOL,
            method="withdraw_unstaked",
            args={},
            gas=GAS_200T,
            deposit=ONE_YOCTO,
            wait=wait,
        )

    def get_account_info(self, rpc: Optional[NearRpcClient] = None) -> dict:
        """Get staking account info from Meta Pool.

        Returns dict with:
          - account_id
          - unstaked_balance
          - staked_balance
          - can_withdraw (bool)
          - st_near_balance
        """
        client = rpc or self.builder.rpc
        result = client.view_function(
            META_POOL,
            "get_account_info",
            {"account_id": self.builder.account_id},
        )
        return result or {}

    def get_st_near_price(self, rpc: Optional[NearRpcClient] = None) -> float:
        """Get current stNEAR/NEAR exchange rate."""
        client = rpc or self.builder.rpc
        result = client.view_function(META_POOL, "get_st_near_price")
        if result:
            return _yocto_to_near(int(result))
        return 1.0

    def get_contract_state(self, rpc: Optional[NearRpcClient] = None) -> dict:
        """Get Meta Pool contract state (total staked, APY, fees, etc.)."""
        client = rpc or self.builder.rpc
        return client.view_function(META_POOL, "get_contract_state") or {}

    def stnear_balance(self, rpc: Optional[NearRpcClient] = None) -> int:
        """Get stNEAR balance for the builder's account."""
        client = rpc or self.builder.rpc
        return client.ft_balance_of(META_POOL, self.builder.account_id)


def create_wnear(account_file: str = None, rpc: NearRpcClient = None) -> WNear:
    """Create a WNear instance from the default account file."""
    return WNear(create_builder(account_file, rpc))


def create_meta_pool(account_file: str = None, rpc: NearRpcClient = None) -> MetaPool:
    """Create a MetaPool instance from the default account file."""
    return MetaPool(create_builder(account_file, rpc))
