#!/bin/bash
# Start Nyx agent via IronClaw daemon
# Invoked by the Nyx app at launch
set -euo pipefail

if [ -f "$HOME/.nyx/.env" ]; then
    set -a; source "$HOME/.nyx/.env"; set +a
fi

exec "$HOME/.cargo/bin/ironclaw" run --config "$HOME/.nyx/config.toml"
