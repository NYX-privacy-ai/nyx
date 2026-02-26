#!/bin/bash
# Poll Apple notarization status every 30 minutes.
# When complete: staple the DMG and notify via Atlas gateway → Telegram.
set -euo pipefail

source /Users/chrisdonovan/Desktop/M80-OCv1/consigliere/.env

SUBMISSION_ID="7817f3da-e50e-4b25-85ef-cf022d158afe"
TEAM_ID="QYN3D88RR3"
DMG_PATH="/tmp/nyx-sign/Nyx_1.4.1_aarch64.dmg"
FINAL_DMG="/Users/chrisdonovan/Desktop/M80-OCv1/consigliere/src-tauri/target/release/bundle/dmg/Nyx_1.4.1_aarch64.dmg"
GATEWAY="http://localhost:18790"
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

        # Staple the ticket
        xcrun stapler staple "$DMG_PATH" >> "$LOG" 2>&1 || true

        # Copy to project
        mkdir -p "$(dirname "$FINAL_DMG")"
        cp "$DMG_PATH" "$FINAL_DMG"
        echo "[$(date)] Stapled DMG copied to ${FINAL_DMG}" >> "$LOG"

        # Notify via Atlas gateway → Telegram
        curl -s -X POST "${GATEWAY}/api/message" \
            -H "Content-Type: application/json" \
            -d '{"message":"Nyx DMG notarization COMPLETE ✅ — Apple accepted the build. Signed + stapled DMG ready at src-tauri/target/release/bundle/dmg/. Run ./build-signed.sh for future builds.","sessionKey":"agent:default:main"}' \
            >> "$LOG" 2>&1 || true

        echo "[$(date)] Done. Exiting poll." >> "$LOG"
        exit 0

    elif [[ "$STATUS" == "Invalid" ]]; then
        echo "[$(date)] Notarization REJECTED." >> "$LOG"

        # Get the log for details
        xcrun notarytool log "$SUBMISSION_ID" \
            --apple-id "$APPLE_ID" \
            --password "$APPLE_PASSWORD" \
            --team-id "$TEAM_ID" >> "$LOG" 2>&1 || true

        # Notify via Atlas
        curl -s -X POST "${GATEWAY}/api/message" \
            -H "Content-Type: application/json" \
            -d '{"message":"Nyx DMG notarization REJECTED ❌ — Apple returned Invalid. Check /tmp/notarize-poll.log for details.","sessionKey":"agent:default:main"}' \
            >> "$LOG" 2>&1 || true

        exit 1
    fi

    # Still in progress — wait 30 minutes
    sleep 1800
done
