#!/bin/bash
# Start Nyx agent via Docker Compose
# Invoked by the macOS LaunchAgent at login
set -euo pipefail

cd "$HOME/openclaw" && /usr/local/bin/docker compose up -d
