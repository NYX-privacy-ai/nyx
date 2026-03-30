#!/bin/bash
# Poll Apple notarization status every 30 minutes.
# When complete: staple the DMG and log result.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [[ -f "${SCRIPT_DIR}/.env" ]]; then
    set -a; source "${SCRIPT_DIR}/.env"; set +a
fi

SUBMISSION_ID="${1:?Usage: $0 <submission-id> <dmg-path>}"
DMG_PATH="${2:?Usage: $0 <submission-id> <dmg-path>}"
TEAM_ID="${APPLE_TEAM_ID:?Set APPLE_TEAM_ID in .env}"
LOG="/tmp/notarize-poll.log"

echo "[$(date)] Notarization poll started for submission ${SUBMISSION_ID}" >> "$LOG"

while true; do
    STATUS=$(xcrun notarytool info "$SUBMISSION_ID" \
        --apple-id "$APPLE_ID" \
        --password "$APPLE_PASSWORD" \
        --team-id "$TEAM_ID" 2>&1 | grep "status:" | tail -1 | sed 's/.*status: //')

    echo "[$(date)] Status: ${STATUS}" >> "$LOG"

    if [[ "$STATUS" == "Accepted" ]]; then
        echo "[$(date)] Notarization ACCEPTED! Stapling..." >> "$LOG"
        xcrun stapler staple "$DMG_PATH" >> "$LOG" 2>&1 || true
        echo "[$(date)] Done. Stapled DMG at ${DMG_PATH}" >> "$LOG"
        exit 0

    elif [[ "$STATUS" == "Invalid" ]]; then
        echo "[$(date)] Notarization REJECTED." >> "$LOG"
        xcrun notarytool log "$SUBMISSION_ID" \
            --apple-id "$APPLE_ID" \
            --password "$APPLE_PASSWORD" \
            --team-id "$TEAM_ID" >> "$LOG" 2>&1 || true
        exit 1
    fi

    # Still in progress — wait 30 minutes
    sleep 1800
done
