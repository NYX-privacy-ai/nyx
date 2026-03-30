#!/usr/bin/env python3
"""
Autonomous DeFi strategy engine for Nyx.

Commands:
  balance   - Show all token balances
  positions - Show active DeFi positions
  report    - Full portfolio + yield report
  rebalance - Execute autonomous rebalancing
  emergency-exit - Unwind all positions back to NEAR

All write operations go through the guardrails module.
"""

import argparse
import json
import sys
import time
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from typing import Optional

from near_rpc import NearRpcClient, MAINNET_RPC
from tx_builder import create_builder, load_signing_key
from portfolio import Portfolio, TOKEN_DECIMALS, _raw_to_human, _human_to_raw
from oracle import PriceOracle, get_all_prices_usd
from staking import MetaPool, WNear
from burrow import Burrow
from yield_engine import YieldEngine
from guardrails import Guardrails, GuardrailViolation

SECRETS_DIR = Path.home() / ".openclaw" / "secrets"
STATE_DIR = Path.home() / ".openclaw" / "defi-state"


def _die(msg: str):
    print(json.dumps({"status": "error", "message": msg}, indent=2), file=sys.stderr)
    sys.exit(1)


def _output(data: dict):
    print(json.dumps(data, indent=2, default=str))


# ------------------------------------------------------------------
# Command: balance
# ------------------------------------------------------------------

def cmd_balance(args):
    """Show all token balances."""
    account_id, _ = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)
    oracle = PriceOracle(rpc)

    prices = oracle.get_all_prices_usd()
    data = portfolio.update_portfolio_from_chain(prices)

    _output({
        "status": "ok",
        "command": "balance",
        "account_id": account_id,
        "holdings": data.get("holdings", {}),
        "total_usd_value": data.get("total_usd_value"),
        "last_update": data.get("last_update"),
    })


# ------------------------------------------------------------------
# Command: positions
# ------------------------------------------------------------------

def cmd_positions(args):
    """Show active DeFi positions."""
    account_id, signing_key = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    builder = create_builder(rpc=rpc)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)

    positions = portfolio.load_positions()

    # Refresh staking positions from Meta Pool
    try:
        mp = MetaPool(builder)
        mp_info = mp.get_account_info()
        if mp_info:
            positions["staking"]["meta_pool"] = mp_info
    except Exception as e:
        positions["staking"]["meta_pool_error"] = str(e)

    # Refresh lending positions from Burrow
    try:
        burrow = Burrow(builder)
        burrow_info = burrow.get_account()
        if burrow_info:
            positions["lending"]["burrow"] = burrow_info
            hf = burrow.compute_health_factor(burrow_info)
            positions["lending"]["burrow_health_factor"] = hf
    except Exception as e:
        positions["lending"]["burrow_error"] = str(e)

    portfolio.save_positions(positions)

    _output({
        "status": "ok",
        "command": "positions",
        "account_id": account_id,
        "positions": positions,
    })


# ------------------------------------------------------------------
# Command: report
# ------------------------------------------------------------------

def cmd_report(args):
    """Full portfolio + yield comparison report."""
    account_id, _ = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)
    oracle = PriceOracle(rpc)
    yield_eng = YieldEngine(rpc, oracle)

    prices = oracle.get_all_prices_usd()
    port_data = portfolio.update_portfolio_from_chain(prices)
    yield_report = yield_eng.to_report()

    # Recommendations
    near_balance = port_data.get("holdings", {}).get("NEAR", {}).get("human", 0)
    positions = portfolio.load_positions()
    recommendations = yield_eng.recommend_allocation(
        available_near=near_balance,
        current_positions=positions,
        risk_tolerance=args.risk if hasattr(args, "risk") and args.risk else "medium",
    )

    _output({
        "status": "ok",
        "command": "report",
        "account_id": account_id,
        "portfolio": port_data,
        "yield_opportunities": yield_report["opportunities"],
        "recommendations": [
            {
                "action": r.action,
                "protocol": r.protocol,
                "token": r.token,
                "amount": r.amount,
                "reason": r.reason,
                "expected_apy": r.expected_apy,
                "risk": r.risk,
            }
            for r in recommendations
        ],
    })


# ------------------------------------------------------------------
# Command: rebalance
# ------------------------------------------------------------------

def cmd_rebalance(args):
    """Execute autonomous portfolio rebalancing."""
    confirm = getattr(args, "confirm", "")
    if confirm != "AUTONOMOUS":
        _die("Rebalance requires --confirm AUTONOMOUS")

    account_id, signing_key = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    builder = create_builder(rpc=rpc)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)
    oracle = PriceOracle(rpc)
    yield_eng = YieldEngine(rpc, oracle)
    guardrails = Guardrails(STATE_DIR)

    prices = oracle.get_all_prices_usd()
    port_data = portfolio.update_portfolio_from_chain(prices)

    # Check loss limits
    total_usd = port_data.get("total_usd_value", 0)
    try:
        guardrails.check_loss_limits(total_usd)
    except GuardrailViolation as e:
        _output({"status": "halted", "reason": str(e)})
        portfolio.log_action("rebalance_halted", {"reason": str(e)})
        return

    near_balance = port_data.get("holdings", {}).get("NEAR", {}).get("human", 0)
    positions = portfolio.load_positions()
    recommendations = yield_eng.recommend_allocation(
        available_near=near_balance,
        current_positions=positions,
        risk_tolerance=getattr(args, "risk", "medium") or "medium",
    )

    actions_taken = []
    errors = []

    for rec in recommendations:
        if rec.action == "hold":
            continue

        # Get USD value of the action
        near_price = prices.get("NEAR", 0)
        action_usd = rec.amount * near_price if rec.token == "NEAR" else rec.amount

        # Holdings for concentration check
        holdings_usd = {}
        for sym, h in port_data.get("holdings", {}).items():
            holdings_usd[sym] = h.get("usd_value", 0)

        try:
            guardrails.pre_tx_check(
                amount_usd=action_usd,
                near_balance=near_balance,
                spend_near=rec.amount if rec.token == "NEAR" else 0,
                holdings_usd=holdings_usd,
                target_token=rec.token,
                action=rec.action,
            )
        except GuardrailViolation as e:
            errors.append({"action": rec.action, "error": str(e)})
            portfolio.log_action("guardrail_blocked", {
                "action": rec.action, "amount": rec.amount, "reason": str(e),
            })
            continue

        try:
            if rec.action == "stake" and rec.protocol == "meta_pool":
                mp = MetaPool(builder)
                result = mp.deposit_and_stake(rec.amount)
                actions_taken.append({
                    "action": "stake",
                    "protocol": "meta_pool",
                    "amount_near": rec.amount,
                    "tx_result": "success" if result else "unknown",
                })
                guardrails.record_transaction()

            elif rec.action == "supply" and rec.protocol == "burrow_supply":
                # For Burrow supply, we need wNEAR
                wnear = WNear(builder)
                wnear.ensure_storage()
                wnear.wrap(rec.amount)
                guardrails.record_transaction()

                burrow = Burrow(builder)
                burrow.ensure_storage()
                amount_raw = str(_human_to_raw(rec.amount, TOKEN_DECIMALS.get(rec.token, 24)))
                result = burrow.supply(rec.token if rec.token != "NEAR" else "WNEAR", amount_raw)
                actions_taken.append({
                    "action": "supply",
                    "protocol": "burrow",
                    "token": rec.token,
                    "amount": rec.amount,
                    "tx_result": "success" if result else "unknown",
                })
                guardrails.record_transaction()

        except Exception as e:
            errors.append({"action": rec.action, "error": str(e)})
            portfolio.log_action("tx_error", {
                "action": rec.action, "amount": rec.amount, "error": str(e),
            })

    # Record P&L
    guardrails.record_portfolio_value(total_usd)

    # Log all actions
    portfolio.log_action("rebalance", {
        "actions_taken": actions_taken,
        "errors": errors,
        "recommendations_count": len(recommendations),
    })

    _output({
        "status": "ok",
        "command": "rebalance",
        "actions_taken": actions_taken,
        "errors": errors,
        "total_usd_before": total_usd,
    })


# ------------------------------------------------------------------
# Command: emergency-exit
# ------------------------------------------------------------------

def cmd_emergency_exit(args):
    """Unwind ALL DeFi positions back to native NEAR.

    Order:
    1. Liquid unstake all stNEAR (Meta Pool)
    2. Repay all Burrow loans
    3. Withdraw all Burrow collateral
    4. Unwrap all wNEAR
    """
    confirm = getattr(args, "confirm", "")
    if confirm != "YES":
        _die("Emergency exit requires --confirm YES")

    account_id, signing_key = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    builder = create_builder(rpc=rpc)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)

    actions = []
    errors = []

    # 1. Unstake stNEAR from Meta Pool
    try:
        mp = MetaPool(builder)
        mp_info = mp.get_account_info()
        stnear_raw = mp.stnear_balance()
        if stnear_raw > 0:
            stnear_human = _raw_to_human(stnear_raw, 24)
            result = mp.liquid_unstake(stnear_human, min_expected_near=0)
            actions.append({"step": "unstake_stnear", "amount": stnear_human, "status": "done"})
    except Exception as e:
        errors.append({"step": "unstake_stnear", "error": str(e)})

    # 2. Handle Burrow positions
    try:
        burrow = Burrow(builder)
        burrow_account = burrow.get_account()

        # 2a. Repay all borrows
        borrowed = burrow_account.get("borrowed", [])
        for b in borrowed:
            token_id = b.get("token_id", "")
            balance = b.get("balance", "0")
            if int(balance) > 0:
                # Find symbol for this token
                symbol = None
                from burrow import TOKEN_CONTRACTS
                for sym, contract in TOKEN_CONTRACTS.items():
                    if contract == token_id:
                        symbol = sym
                        break
                if symbol:
                    try:
                        burrow.repay(symbol, balance)
                        actions.append({"step": f"repay_{symbol}", "amount_raw": balance, "status": "done"})
                    except Exception as e:
                        errors.append({"step": f"repay_{symbol}", "error": str(e)})

        # 2b. Withdraw all collateral
        collateral = burrow_account.get("collateral", [])
        for c in collateral:
            token_id = c.get("token_id", "")
            balance = c.get("balance", "0")
            if int(balance) > 0:
                symbol = None
                from burrow import TOKEN_CONTRACTS
                for sym, contract in TOKEN_CONTRACTS.items():
                    if contract == token_id:
                        symbol = sym
                        break
                if symbol:
                    try:
                        burrow.withdraw(symbol, balance)
                        actions.append({"step": f"withdraw_{symbol}", "amount_raw": balance, "status": "done"})
                    except Exception as e:
                        errors.append({"step": f"withdraw_{symbol}", "error": str(e)})

    except Exception as e:
        errors.append({"step": "burrow_unwind", "error": str(e)})

    # 3. Unwrap all wNEAR
    try:
        wnear = WNear(builder)
        wnear_raw = wnear.balance()
        if wnear_raw > 0:
            wnear_human = _raw_to_human(wnear_raw, 24)
            wnear.unwrap(wnear_human)
            actions.append({"step": "unwrap_wnear", "amount": wnear_human, "status": "done"})
    except Exception as e:
        errors.append({"step": "unwrap_wnear", "error": str(e)})

    # Log everything
    portfolio.log_action("emergency_exit", {"actions": actions, "errors": errors})

    # Halt further trading
    guardrails = Guardrails(STATE_DIR)
    guardrails.halt_trading("Emergency exit executed")

    _output({
        "status": "ok",
        "command": "emergency-exit",
        "actions": actions,
        "errors": errors,
        "trading_halted": True,
    })


# ------------------------------------------------------------------
# Command: heartbeat (for cron — every 4 hours)
# ------------------------------------------------------------------

def cmd_heartbeat(args):
    """Periodic strategy heartbeat for autonomous cron execution.

    Checks:
    1. Portfolio health (loss limits, halt status)
    2. Burrow health factor (emergency deleverage if needed)
    3. Yield opportunities vs current allocation
    4. Executes rebalance if conditions are met

    Designed to be triggered by cron every 4 hours.
    """
    account_id, signing_key = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    builder = create_builder(rpc=rpc)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)
    oracle = PriceOracle(rpc)
    yield_eng = YieldEngine(rpc, oracle)
    guardrails = Guardrails(STATE_DIR)

    result = {
        "status": "ok",
        "command": "heartbeat",
        "account_id": account_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "checks": {},
        "actions_taken": [],
        "errors": [],
    }

    # 1. Check halt status
    try:
        guardrails.check_halt()
        result["checks"]["halt"] = "clear"
    except GuardrailViolation as e:
        result["checks"]["halt"] = str(e)
        result["status"] = "halted"
        _output(result)
        return

    # 2. Fetch balances and prices
    try:
        prices = oracle.get_all_prices_usd()
        port_data = portfolio.update_portfolio_from_chain(prices)
        total_usd = port_data.get("total_usd_value", 0)
        near_balance = port_data.get("holdings", {}).get("NEAR", {}).get("human", 0)
        result["checks"]["portfolio_usd"] = round(total_usd, 2)
        result["checks"]["near_balance"] = round(near_balance, 4)
    except Exception as e:
        result["errors"].append({"step": "fetch_balances", "error": str(e)})
        _output(result)
        return

    # 3. Check loss limits
    try:
        guardrails.check_loss_limits(total_usd)
        result["checks"]["loss_limits"] = "ok"
    except GuardrailViolation as e:
        result["checks"]["loss_limits"] = str(e)
        result["status"] = "halted"
        portfolio.log_action("heartbeat_halted", {"reason": str(e)})
        _output(result)
        return

    # 4. Check Burrow health factor
    try:
        burrow = Burrow(builder)
        burrow_account = burrow.get_account()
        if burrow_account and burrow_account.get("borrowed"):
            hf = burrow.compute_health_factor(burrow_account)
            result["checks"]["burrow_health_factor"] = round(hf, 2)
            hf_status = guardrails.check_health_factor(hf)
            result["checks"]["burrow_hf_status"] = hf_status

            # Emergency deleverage if critical
            if hf_status == "emergency":
                try:
                    # Repay as much as possible
                    for b in burrow_account.get("borrowed", []):
                        token_id = b.get("token_id", "")
                        balance = b.get("balance", "0")
                        if int(balance) > 0:
                            from burrow import TOKEN_CONTRACTS
                            for sym, contract in TOKEN_CONTRACTS.items():
                                if contract == token_id:
                                    burrow.repay(sym, balance)
                                    result["actions_taken"].append({
                                        "action": "emergency_repay",
                                        "token": sym,
                                        "amount_raw": balance,
                                    })
                                    break
                except Exception as e:
                    result["errors"].append({"step": "emergency_deleverage", "error": str(e)})
        else:
            result["checks"]["burrow_health_factor"] = "no_borrows"
    except Exception as e:
        result["checks"]["burrow_health_factor"] = f"error: {str(e)}"

    # 5. Check yield and rebalance if beneficial
    try:
        positions = portfolio.load_positions()
        recommendations = yield_eng.recommend_allocation(
            available_near=near_balance,
            current_positions=positions,
            risk_tolerance=getattr(args, "risk", "medium") or "medium",
        )

        actionable = [r for r in recommendations if r.action != "hold"]
        result["checks"]["recommendations"] = len(actionable)

        # Only auto-rebalance if there are actionable recommendations
        # and the portfolio is above a minimum threshold
        if actionable and total_usd >= 5.0:
            for rec in actionable:
                near_price = prices.get("NEAR", 0)
                action_usd = rec.amount * near_price if rec.token == "NEAR" else rec.amount

                holdings_usd = {}
                for sym, h in port_data.get("holdings", {}).items():
                    holdings_usd[sym] = h.get("usd_value", 0)

                try:
                    guardrails.pre_tx_check(
                        amount_usd=action_usd,
                        near_balance=near_balance,
                        spend_near=rec.amount if rec.token == "NEAR" else 0,
                        holdings_usd=holdings_usd,
                        target_token=rec.token,
                        action=rec.action,
                    )
                except GuardrailViolation as e:
                    result["errors"].append({"action": rec.action, "error": str(e)})
                    continue

                try:
                    if rec.action == "stake" and rec.protocol == "meta_pool":
                        mp = MetaPool(builder)
                        mp.deposit_and_stake(rec.amount)
                        result["actions_taken"].append({
                            "action": "stake", "protocol": "meta_pool",
                            "amount_near": rec.amount,
                        })
                        guardrails.record_transaction()
                    elif rec.action == "supply" and rec.protocol == "burrow_supply":
                        wnear = WNear(builder)
                        wnear.ensure_storage()
                        wnear.wrap(rec.amount)
                        guardrails.record_transaction()
                        burrow_inst = Burrow(builder)
                        burrow_inst.ensure_storage()
                        amount_raw = str(_human_to_raw(rec.amount, TOKEN_DECIMALS.get(rec.token, 24)))
                        burrow_inst.supply(rec.token if rec.token != "NEAR" else "WNEAR", amount_raw)
                        result["actions_taken"].append({
                            "action": "supply", "protocol": "burrow",
                            "token": rec.token, "amount": rec.amount,
                        })
                        guardrails.record_transaction()
                except Exception as e:
                    result["errors"].append({"action": rec.action, "error": str(e)})

    except Exception as e:
        result["errors"].append({"step": "yield_check", "error": str(e)})

    # Record P&L
    guardrails.record_portfolio_value(total_usd)
    portfolio.log_action("heartbeat", {
        "checks": result["checks"],
        "actions": result["actions_taken"],
        "errors": result["errors"],
    })

    _output(result)


# ------------------------------------------------------------------
# Command: daily-report (for cron — 9am UK time)
# ------------------------------------------------------------------

def cmd_daily_report(args):
    """Generate a daily P&L and portfolio summary for WhatsApp delivery.

    Designed to be triggered by cron at 9am UK time.
    Returns a structured report suitable for the agent to format
    and send via WhatsApp.
    """
    account_id, _ = load_signing_key()
    rpc = NearRpcClient(MAINNET_RPC)
    portfolio = Portfolio(account_id, STATE_DIR, rpc)
    oracle = PriceOracle(rpc)
    yield_eng = YieldEngine(rpc, oracle)
    guardrails = Guardrails(STATE_DIR)

    # Fetch current state
    prices = oracle.get_all_prices_usd()
    port_data = portfolio.update_portfolio_from_chain(prices)
    total_usd = float(port_data.get("total_usd_value", 0) or 0)
    near_balance = float(port_data.get("holdings", {}).get("NEAR", {}).get("human", 0) or 0)

    # Compute P&L
    pnl_data = guardrails._load_pnl()
    start_value = float(pnl_data.get("start_value_usd", 0) or 0)
    daily_entries = pnl_data.get("daily", {})

    # Yesterday's value for daily P&L
    today_str = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    sorted_dates = sorted([k for k in daily_entries.keys() if k < today_str])
    yesterday_value = float(daily_entries[sorted_dates[-1]].get("value_usd", 0)) if sorted_dates else start_value

    daily_pnl_usd = total_usd - yesterday_value if yesterday_value else 0
    daily_pnl_pct = (daily_pnl_usd / yesterday_value * 100) if yesterday_value and yesterday_value > 0 else 0
    total_pnl_usd = total_usd - start_value if start_value else 0
    total_pnl_pct = (total_pnl_usd / start_value * 100) if start_value and start_value > 0 else 0

    # Positions summary
    positions = portfolio.load_positions()

    # Yield opportunities
    yield_report = yield_eng.to_report()

    # Halt status
    halt_path = guardrails.state_dir / guardrails.config.halt_file
    is_halted = halt_path.exists()

    # Recent activity (last 24h from strategy log)
    log_file = guardrails.state_dir / "strategy_log.jsonl"
    recent_actions = []
    if log_file.exists():
        from datetime import timedelta
        cutoff_dt = datetime.now(timezone.utc) - timedelta(hours=24)
        cutoff_str = cutoff_dt.isoformat()
        with open(log_file, "r") as f:
            for line in f:
                try:
                    entry = json.loads(line.strip())
                    ts = entry.get("timestamp", "")
                    # Compare ISO 8601 strings lexicographically (works for UTC)
                    if isinstance(ts, str) and ts >= cutoff_str:
                        recent_actions.append(entry)
                except (json.JSONDecodeError, TypeError):
                    continue

    # Record today's value
    guardrails.record_portfolio_value(total_usd)

    _output({
        "status": "ok",
        "command": "daily-report",
        "account_id": account_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "portfolio": {
            "total_usd": round(total_usd, 2),
            "holdings": port_data.get("holdings", {}),
        },
        "pnl": {
            "daily_usd": round(daily_pnl_usd, 2),
            "daily_pct": round(daily_pnl_pct, 2),
            "total_usd": round(total_pnl_usd, 2),
            "total_pct": round(total_pnl_pct, 2),
            "start_value_usd": round(start_value, 2),
        },
        "positions": positions,
        "yield_opportunities": yield_report.get("opportunities", []),
        "trading_halted": is_halted,
        "recent_actions_24h": len(recent_actions),
    })


# ------------------------------------------------------------------
# CLI
# ------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Nyx DeFi Strategy Engine")
    sub = parser.add_subparsers(dest="command", required=True)

    # balance
    sub.add_parser("balance", help="Show all token balances")

    # positions
    sub.add_parser("positions", help="Show active DeFi positions")

    # report
    p_report = sub.add_parser("report", help="Full portfolio + yield report")
    p_report.add_argument("--risk", choices=["low", "medium", "high"], default="medium")

    # rebalance
    p_rebalance = sub.add_parser("rebalance", help="Execute autonomous rebalancing")
    p_rebalance.add_argument("--confirm", required=True, help="Must be 'AUTONOMOUS'")
    p_rebalance.add_argument("--risk", choices=["low", "medium", "high"], default="medium")

    # emergency-exit
    p_exit = sub.add_parser("emergency-exit", help="Unwind all positions to NEAR")
    p_exit.add_argument("--confirm", required=True, help="Must be 'YES'")

    # heartbeat (cron — every 4h)
    p_heartbeat = sub.add_parser("heartbeat", help="Periodic strategy heartbeat (cron)")
    p_heartbeat.add_argument("--risk", choices=["low", "medium", "high"], default="medium")

    # daily-report (cron — 9am UK)
    sub.add_parser("daily-report", help="Daily P&L report (cron)")

    args = parser.parse_args()

    try:
        if args.command == "balance":
            cmd_balance(args)
        elif args.command == "positions":
            cmd_positions(args)
        elif args.command == "report":
            cmd_report(args)
        elif args.command == "rebalance":
            cmd_rebalance(args)
        elif args.command == "emergency-exit":
            cmd_emergency_exit(args)
        elif args.command == "heartbeat":
            cmd_heartbeat(args)
        elif args.command == "daily-report":
            cmd_daily_report(args)
        else:
            _die(f"Unknown command: {args.command}")
    except GuardrailViolation as e:
        _output({"status": "guardrail_violation", "rule": e.rule, "message": e.message})
        sys.exit(1)
    except Exception as e:
        _die(str(e))


if __name__ == "__main__":
    main()
