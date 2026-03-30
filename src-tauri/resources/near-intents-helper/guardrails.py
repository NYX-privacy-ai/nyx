#!/usr/bin/env python3
"""
Python-enforced guardrails for Nyx DeFi engine.

These guardrails are enforced in code — they CANNOT be overridden by the LLM.
All checks happen BEFORE any transaction is signed or submitted.

Configurable via ~/.openclaw/secrets/defi_guardrails.env

Portable module — no OpenClaw-specific dependencies.
"""

import json
import os
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from typing import Optional

from dotenv import load_dotenv


# ------------------------------------------------------------------
# Default guardrail values
# ------------------------------------------------------------------

DEFAULTS = {
    "MAX_SINGLE_TX_USD": 500.0,
    "DAILY_LOSS_LIMIT_PCT": 5.0,
    "WEEKLY_LOSS_LIMIT_PCT": 10.0,
    "MIN_BALANCE_FLOOR_NEAR": 2.0,
    "MAX_CONCENTRATION_PCT": 60.0,
    "BURROW_MIN_HEALTH_FACTOR": 1.5,
    "BURROW_EMERGENCY_HEALTH_FACTOR": 1.2,
    "MAX_SLIPPAGE_PCT": 2.0,
    "MAX_DAILY_TXS": 20,
    "HALT_FILE": "halt.flag",  # If this file exists, all trading stops
}


@dataclass
class GuardrailConfig:
    """Guardrail configuration loaded from env."""
    max_single_tx_usd: float
    daily_loss_limit_pct: float
    weekly_loss_limit_pct: float
    min_balance_floor_near: float
    max_concentration_pct: float
    burrow_min_health_factor: float
    burrow_emergency_health_factor: float
    max_slippage_pct: float
    max_daily_txs: int
    halt_file: str


class GuardrailViolation(Exception):
    """Raised when a guardrail check fails. Transaction MUST NOT proceed."""
    def __init__(self, rule: str, message: str):
        self.rule = rule
        self.message = message
        super().__init__(f"GUARDRAIL [{rule}]: {message}")


class Guardrails:
    """Enforce DeFi trading guardrails."""

    def __init__(self, state_dir: Optional[Path] = None, env_file: Optional[str] = None):
        self.state_dir = state_dir or (Path.home() / ".openclaw" / "defi-state")
        self.state_dir.mkdir(parents=True, exist_ok=True)

        # Load config from env file (if available), then overlay with env vars.
        # In container mode, guardrails are injected as env vars directly —
        # no secrets file mount needed.
        env_path = env_file or str(Path.home() / ".openclaw" / "secrets" / "defi_guardrails.env")
        if Path(env_path).exists():
            load_dotenv(env_path)

        self.config = GuardrailConfig(
            max_single_tx_usd=float(os.environ.get("MAX_SINGLE_TX_USD", DEFAULTS["MAX_SINGLE_TX_USD"])),
            daily_loss_limit_pct=float(os.environ.get("DAILY_LOSS_LIMIT_PCT", DEFAULTS["DAILY_LOSS_LIMIT_PCT"])),
            weekly_loss_limit_pct=float(os.environ.get("WEEKLY_LOSS_LIMIT_PCT", DEFAULTS["WEEKLY_LOSS_LIMIT_PCT"])),
            min_balance_floor_near=float(os.environ.get("MIN_BALANCE_FLOOR_NEAR", DEFAULTS["MIN_BALANCE_FLOOR_NEAR"])),
            max_concentration_pct=float(os.environ.get("MAX_CONCENTRATION_PCT", DEFAULTS["MAX_CONCENTRATION_PCT"])),
            burrow_min_health_factor=float(os.environ.get("BURROW_MIN_HEALTH_FACTOR", DEFAULTS["BURROW_MIN_HEALTH_FACTOR"])),
            burrow_emergency_health_factor=float(os.environ.get("BURROW_EMERGENCY_HEALTH_FACTOR", DEFAULTS["BURROW_EMERGENCY_HEALTH_FACTOR"])),
            max_slippage_pct=float(os.environ.get("MAX_SLIPPAGE_PCT", DEFAULTS["MAX_SLIPPAGE_PCT"])),
            max_daily_txs=int(os.environ.get("MAX_DAILY_TXS", DEFAULTS["MAX_DAILY_TXS"])),
            halt_file=os.environ.get("HALT_FILE", DEFAULTS["HALT_FILE"]),
        )

        self._tx_log_file = self.state_dir / "tx_count.json"
        self._pnl_file = self.state_dir / "daily_pnl.json"

    # ------------------------------------------------------------------
    # Halt check
    # ------------------------------------------------------------------

    def check_halt(self):
        """Check if trading is halted (halt file exists)."""
        halt_path = self.state_dir / self.config.halt_file
        if halt_path.exists():
            raise GuardrailViolation("HALT", f"Trading halted. Remove {halt_path} to resume.")

    def halt_trading(self, reason: str):
        """Halt all trading by creating the halt file."""
        halt_path = self.state_dir / self.config.halt_file
        halt_path.write_text(json.dumps({
            "halted_at": datetime.now(timezone.utc).isoformat(),
            "reason": reason,
        }))

    def resume_trading(self):
        """Resume trading by removing the halt file."""
        halt_path = self.state_dir / self.config.halt_file
        if halt_path.exists():
            halt_path.unlink()

    # ------------------------------------------------------------------
    # Transaction size limit
    # ------------------------------------------------------------------

    def check_tx_size(self, amount_usd: float):
        """Check that transaction doesn't exceed max single tx size."""
        if self.config.max_single_tx_usd >= 1_000_000:
            return  # Effectively no limit (Autonomous preset)
        if amount_usd > self.config.max_single_tx_usd:
            raise GuardrailViolation(
                "MAX_TX_SIZE",
                f"Transaction ${amount_usd:.2f} exceeds max ${self.config.max_single_tx_usd:.2f}",
            )

    # ------------------------------------------------------------------
    # Minimum balance floor
    # ------------------------------------------------------------------

    def check_min_balance(self, current_near_balance: float, spend_amount_near: float):
        """Check that NEAR balance stays above floor after transaction."""
        remaining = current_near_balance - spend_amount_near
        if remaining < self.config.min_balance_floor_near:
            raise GuardrailViolation(
                "MIN_BALANCE",
                f"Would leave {remaining:.4f} NEAR, below floor of {self.config.min_balance_floor_near} NEAR",
            )

    # ------------------------------------------------------------------
    # Concentration limit
    # ------------------------------------------------------------------

    def check_concentration(self, holdings_usd: dict, token: str, add_amount_usd: float):
        """Check that no single asset exceeds concentration limit.

        Args:
            holdings_usd: Dict of {symbol: usd_value}.
            token: Token being added to.
            add_amount_usd: USD value being added.
        """
        if self.config.max_concentration_pct >= 100:
            return  # No concentration limit (Autonomous preset)
        total = sum(holdings_usd.values()) + add_amount_usd
        if total <= 0:
            return
        current = holdings_usd.get(token, 0.0) + add_amount_usd
        pct = (current / total) * 100
        if pct > self.config.max_concentration_pct:
            raise GuardrailViolation(
                "CONCENTRATION",
                f"{token} would be {pct:.1f}% of portfolio, exceeds max {self.config.max_concentration_pct}%",
            )

    # ------------------------------------------------------------------
    # Slippage protection
    # ------------------------------------------------------------------

    def check_slippage(self, expected_amount: float, actual_amount: float):
        """Check that slippage doesn't exceed limit."""
        if expected_amount <= 0:
            return
        slippage_pct = ((expected_amount - actual_amount) / expected_amount) * 100
        if slippage_pct > self.config.max_slippage_pct:
            raise GuardrailViolation(
                "SLIPPAGE",
                f"Slippage {slippage_pct:.2f}% exceeds max {self.config.max_slippage_pct}%",
            )

    # ------------------------------------------------------------------
    # Burrow health factor
    # ------------------------------------------------------------------

    def check_health_factor(self, health_factor: float) -> str:
        """Check Burrow health factor.

        Returns:
            "ok" if healthy, "warning" if below min, "emergency" if critical.

        Raises:
            GuardrailViolation if health factor is at emergency level and
            the action would reduce it further.
        """
        if health_factor < self.config.burrow_emergency_health_factor:
            return "emergency"
        elif health_factor < self.config.burrow_min_health_factor:
            return "warning"
        return "ok"

    def enforce_health_factor(self, health_factor: float, action: str = "borrow"):
        """Enforce health factor limits for borrow actions."""
        status = self.check_health_factor(health_factor)
        if status == "emergency":
            raise GuardrailViolation(
                "HEALTH_FACTOR_EMERGENCY",
                f"Health factor {health_factor:.2f} is critical (< {self.config.burrow_emergency_health_factor}). "
                f"Cannot {action}. Must deleverage.",
            )
        if status == "warning" and action in ("borrow",):
            raise GuardrailViolation(
                "HEALTH_FACTOR_WARNING",
                f"Health factor {health_factor:.2f} is below safe threshold ({self.config.burrow_min_health_factor}). "
                f"Cannot borrow more.",
            )

    # ------------------------------------------------------------------
    # Daily transaction count
    # ------------------------------------------------------------------

    def _load_tx_count(self) -> dict:
        if self._tx_log_file.exists():
            with open(self._tx_log_file, "r") as f:
                return json.load(f)
        return {"date": "", "count": 0}

    def _save_tx_count(self, data: dict):
        with open(self._tx_log_file, "w") as f:
            json.dump(data, f)

    def check_daily_tx_count(self):
        """Check that daily transaction count hasn't been exceeded."""
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        data = self._load_tx_count()
        if data["date"] != today:
            data = {"date": today, "count": 0}
        if data["count"] >= self.config.max_daily_txs:
            raise GuardrailViolation(
                "DAILY_TX_LIMIT",
                f"Daily tx limit reached ({self.config.max_daily_txs}). Halting for today.",
            )

    def record_transaction(self):
        """Record a transaction for daily counting."""
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        data = self._load_tx_count()
        if data["date"] != today:
            data = {"date": today, "count": 0}
        data["count"] += 1
        self._save_tx_count(data)

    # ------------------------------------------------------------------
    # P&L tracking and loss limits
    # ------------------------------------------------------------------

    def _load_pnl(self) -> dict:
        if self._pnl_file.exists():
            with open(self._pnl_file, "r") as f:
                return json.load(f)
        return {"daily": {}, "start_value_usd": 0.0}

    def _save_pnl(self, data: dict):
        with open(self._pnl_file, "w") as f:
            json.dump(data, f, indent=2)

    def record_portfolio_value(self, total_usd: float):
        """Record current portfolio value for P&L tracking."""
        data = self._load_pnl()
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        if "start_value_usd" not in data or data["start_value_usd"] == 0:
            data["start_value_usd"] = total_usd
        data["daily"][today] = {"value_usd": total_usd, "timestamp": time.time()}
        self._save_pnl(data)

    def check_loss_limits(self, current_value_usd: float):
        """Check daily and weekly loss limits.

        Raises GuardrailViolation if losses exceed limits.
        Also halts trading if weekly limit is breached.
        """
        if self.config.daily_loss_limit_pct >= 100 and self.config.weekly_loss_limit_pct >= 100:
            return  # No loss limits (Autonomous preset)
        data = self._load_pnl()
        start_value = data.get("start_value_usd", 0)
        if start_value <= 0:
            return

        # Daily loss check
        today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
        daily = data.get("daily", {})

        # Find today's opening value (or start value)
        yesterday_keys = sorted([k for k in daily.keys() if k < today])
        if yesterday_keys:
            day_start = daily[yesterday_keys[-1]].get("value_usd", start_value)
        else:
            day_start = start_value

        if day_start > 0:
            daily_loss_pct = ((day_start - current_value_usd) / day_start) * 100
            if daily_loss_pct > self.config.daily_loss_limit_pct:
                self.halt_trading(f"Daily loss {daily_loss_pct:.1f}% exceeds {self.config.daily_loss_limit_pct}%")
                raise GuardrailViolation(
                    "DAILY_LOSS",
                    f"Daily loss {daily_loss_pct:.1f}% exceeds limit of {self.config.daily_loss_limit_pct}%. Trading halted.",
                )

        # Weekly loss check
        week_keys = sorted([k for k in daily.keys()])[-7:]
        if week_keys:
            week_start = daily[week_keys[0]].get("value_usd", start_value)
            if week_start > 0:
                weekly_loss_pct = ((week_start - current_value_usd) / week_start) * 100
                if weekly_loss_pct > self.config.weekly_loss_limit_pct:
                    self.halt_trading(f"Weekly loss {weekly_loss_pct:.1f}% exceeds {self.config.weekly_loss_limit_pct}%")
                    raise GuardrailViolation(
                        "WEEKLY_LOSS",
                        f"Weekly loss {weekly_loss_pct:.1f}% exceeds limit of {self.config.weekly_loss_limit_pct}%. Trading halted.",
                    )

    # ------------------------------------------------------------------
    # Combined pre-transaction check
    # ------------------------------------------------------------------

    def pre_tx_check(
        self,
        amount_usd: float = 0,
        near_balance: float = 0,
        spend_near: float = 0,
        holdings_usd: Optional[dict] = None,
        target_token: Optional[str] = None,
        health_factor: Optional[float] = None,
        action: str = "trade",
    ):
        """Run all applicable guardrail checks before a transaction.

        Call this BEFORE signing any transaction. If it raises, do NOT proceed.
        """
        self.check_halt()
        self.check_daily_tx_count()

        if amount_usd > 0:
            self.check_tx_size(amount_usd)

        if spend_near > 0 and near_balance > 0:
            self.check_min_balance(near_balance, spend_near)

        if holdings_usd and target_token and amount_usd > 0:
            self.check_concentration(holdings_usd, target_token, amount_usd)

        if health_factor is not None:
            self.enforce_health_factor(health_factor, action)
