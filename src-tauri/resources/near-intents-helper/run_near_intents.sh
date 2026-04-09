#!/usr/bin/env bash
# Wrapper script for NEAR Intents + Nyx DeFi helper.
# This is the ONLY command IronClaw is allowed to invoke.
# Strict argument parsing — no arbitrary shell injection.
# All stdout is piped through leak_detector to redact secrets.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INTENTS_HELPER="${SCRIPT_DIR}/near_intents.py"
STRATEGY_HELPER="${SCRIPT_DIR}/strategy.py"
LEAK_DETECTOR="${SCRIPT_DIR}/leak_detector.py"
REQUIREMENTS="${SCRIPT_DIR}/requirements.txt"

# Local venv — created on first run if missing
LOCAL_VENV="${SCRIPT_DIR}/.venv"

if [[ ! -f "${LOCAL_VENV}/bin/python3" ]]; then
    echo '{"status":"info","message":"Bootstrap: creating Python venv..."}' >&2
    python3 -m venv "${LOCAL_VENV}"
    if [[ -f "${REQUIREMENTS}" ]]; then
        "${LOCAL_VENV}/bin/pip" install --quiet -r "${REQUIREMENTS}"
    fi
fi

PYTHON="${LOCAL_VENV}/bin/python3"

# Set PYTHONPATH so modules can find each other
export PYTHONPATH="${SCRIPT_DIR}:${PYTHONPATH:-}"

# Leak detection wrapper: pipe stdout through leak_detector
# This ensures no secret patterns reach the LLM in tool output
run_with_leak_scan() {
    if [[ -f "${LEAK_DETECTOR}" ]]; then
        "$@" 2>&1 | "${PYTHON}" "${LEAK_DETECTOR}"
    else
        "$@"
    fi
}

# Only allow known subcommands
COMMAND="${1:-}"
case "${COMMAND}" in
    # --- Original NEAR Intents commands ---
    quote|publish|status)
        if [[ ! -f "${INTENTS_HELPER}" ]]; then
            echo '{"status":"error","message":"near_intents.py not found."}' >&2
            exit 1
        fi
        shift
        run_with_leak_scan "${PYTHON}" "${INTENTS_HELPER}" "${COMMAND}" "$@"
        ;;
    # --- Nyx DeFi commands ---
    balance|positions|report|rebalance|burrow-loop|emergency-exit|heartbeat|daily-report)
        if [[ ! -f "${STRATEGY_HELPER}" ]]; then
            echo '{"status":"error","message":"strategy.py not found."}' >&2
            exit 1
        fi
        shift
        run_with_leak_scan "${PYTHON}" "${STRATEGY_HELPER}" "${COMMAND}" "$@"
        ;;
    # --- Access key management ---
    deploy-keys|list-keys)
        KEYS_HELPER="${SCRIPT_DIR}/access_keys.py"
        if [[ ! -f "${KEYS_HELPER}" ]]; then
            echo '{"status":"error","message":"access_keys.py not found."}' >&2
            exit 1
        fi
        shift
        run_with_leak_scan "${PYTHON}" "${KEYS_HELPER}" "${COMMAND}" "$@"
        ;;
    # --- NEAR AI auth token ---
    auth-token)
        AUTH_HELPER="${SCRIPT_DIR}/nearai_auth.py"
        if [[ ! -f "${AUTH_HELPER}" ]]; then
            echo '{"status":"error","message":"nearai_auth.py not found."}' >&2
            exit 1
        fi
        shift
        run_with_leak_scan "${PYTHON}" "${AUTH_HELPER}" "$@"
        ;;
    # --- NEAR transfers ---
    transfer)
        TRANSFER_HELPER="${SCRIPT_DIR}/transfers.py"
        if [[ ! -f "${TRANSFER_HELPER}" ]]; then
            echo '{"status":"error","message":"transfers.py not found."}' >&2
            exit 1
        fi
        shift
        run_with_leak_scan "${PYTHON}" "${TRANSFER_HELPER}" "$@"
        ;;
    *)
        echo '{"status":"error","message":"Unknown command. Allowed: quote, publish, status, balance, positions, report, rebalance, burrow-loop, emergency-exit, heartbeat, daily-report, deploy-keys, list-keys, auth-token, transfer"}' >&2
        exit 1
        ;;
esac
