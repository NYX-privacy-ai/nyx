#!/usr/bin/env python3
"""Tests for the safety-critical DeFi guardrail checks.

Uses only the standard library (unittest) so it runs without installing
python-dotenv or pytest. Run with:  python3 -m unittest test_guardrails
"""
import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from guardrails import GuardrailConfig, Guardrails, GuardrailViolation  # noqa: E402


def make_guardrails(tmp, **overrides):
    """Construct Guardrails with deterministic, overridable config values."""
    g = Guardrails(state_dir=Path(tmp), env_file=str(Path(tmp) / "nonexistent.env"))
    base = dict(
        max_single_tx_usd=500.0,
        daily_loss_limit_pct=5.0,
        weekly_loss_limit_pct=10.0,
        min_balance_floor_near=2.0,
        max_concentration_pct=40.0,
        burrow_min_health_factor=1.5,
        burrow_emergency_health_factor=1.2,
        max_slippage_pct=2.0,
        max_daily_txs=20,
        halt_file="halt.flag",
    )
    base.update(overrides)
    g.config = GuardrailConfig(**base)
    return g


class TxSizeTests(unittest.TestCase):
    def test_within_limit_ok(self):
        with tempfile.TemporaryDirectory() as t:
            make_guardrails(t).check_tx_size(400.0)  # no raise

    def test_over_limit_raises(self):
        with tempfile.TemporaryDirectory() as t:
            with self.assertRaises(GuardrailViolation):
                make_guardrails(t).check_tx_size(600.0)

    def test_high_cap_still_enforced(self):
        # Regression: a high cap must still enforce. There used to be a
        # `>= $1M` escape hatch that disabled the check entirely.
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, max_single_tx_usd=2_000_000.0)
            with self.assertRaises(GuardrailViolation):
                g.check_tx_size(3_000_000.0)
            g.check_tx_size(1_000_000.0)  # under cap -> ok


class ConcentrationTests(unittest.TestCase):
    def test_over_concentration_raises(self):
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, max_concentration_pct=40.0)
            # Adding $600 of TOKEN to a $400 book => 60% concentration.
            with self.assertRaises(GuardrailViolation):
                g.check_concentration({"OTHER": 400.0}, "TOKEN", 600.0)

    def test_under_concentration_ok(self):
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, max_concentration_pct=40.0)
            g.check_concentration({"OTHER": 900.0}, "TOKEN", 100.0)  # 10% -> ok

    def test_hundred_percent_cap_never_raises(self):
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, max_concentration_pct=100.0)
            g.check_concentration({}, "TOKEN", 1000.0)  # 100% allowed


class SlippageTests(unittest.TestCase):
    def test_acceptable_slippage_ok(self):
        with tempfile.TemporaryDirectory() as t:
            make_guardrails(t, max_slippage_pct=2.0).check_slippage(100.0, 99.0)  # 1%

    def test_excess_slippage_raises(self):
        with tempfile.TemporaryDirectory() as t:
            with self.assertRaises(GuardrailViolation):
                make_guardrails(t, max_slippage_pct=2.0).check_slippage(100.0, 95.0)  # 5%


class MinBalanceTests(unittest.TestCase):
    def test_keeps_floor_ok(self):
        with tempfile.TemporaryDirectory() as t:
            make_guardrails(t, min_balance_floor_near=2.0).check_min_balance(10.0, 5.0)

    def test_breaches_floor_raises(self):
        with tempfile.TemporaryDirectory() as t:
            with self.assertRaises(GuardrailViolation):
                make_guardrails(t, min_balance_floor_near=2.0).check_min_balance(3.0, 2.0)


class HealthFactorTests(unittest.TestCase):
    def test_status_classification(self):
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, burrow_min_health_factor=1.5, burrow_emergency_health_factor=1.2)
            self.assertEqual(g.check_health_factor(2.0), "ok")
            self.assertEqual(g.check_health_factor(1.3), "warning")
            self.assertEqual(g.check_health_factor(1.1), "emergency")

    def test_enforce_blocks_borrow_when_unsafe(self):
        with tempfile.TemporaryDirectory() as t:
            g = make_guardrails(t, burrow_min_health_factor=1.5, burrow_emergency_health_factor=1.2)
            with self.assertRaises(GuardrailViolation):
                g.enforce_health_factor(1.1, action="borrow")  # emergency
            with self.assertRaises(GuardrailViolation):
                g.enforce_health_factor(1.3, action="borrow")  # warning + borrow
            g.enforce_health_factor(2.0, action="borrow")  # healthy -> ok


if __name__ == "__main__":
    unittest.main()
