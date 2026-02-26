#!/bin/bash
set -euo pipefail

# Nyx — signed + notarized build script
# Builds unsigned via Tauri, copies to /tmp to escape iCloud xattr interference,
# signs all binaries with hardened runtime + timestamp, creates DMG, notarizes.

IDENTITY="Developer ID Application: Robin Janaway (QYN3D88RR3)"
TEAM_ID="QYN3D88RR3"
BUNDLE_DIR="src-tauri/target/release/bundle/macos"
APP_PATH="${BUNDLE_DIR}/Nyx.app"
SIGN_DIR="/tmp/nyx-sign"
VERSION=$(grep '"version"' src-tauri/tauri.conf.json | head -1 | sed 's/.*: "//;s/".*//')
DMG_NAME="Nyx_${VERSION}_aarch64.dmg"
FINAL_DMG="src-tauri/target/release/bundle/dmg/${DMG_NAME}"

# Load Apple credentials
if [[ -f .env ]]; then
    set -a; source .env; set +a
fi

echo "==> Step 1: Building unsigned app via Tauri..."
rm -rf "${BUNDLE_DIR}" "src-tauri/target/release/bundle/dmg"
npm run tauri build 2>&1 || true
# Tauri may exit 1 due to missing TAURI_SIGNING_PRIVATE_KEY (updater),
# but the .app bundle is still produced. Check it exists:
if [[ ! -d "${APP_PATH}" ]]; then
    echo "ERROR: ${APP_PATH} was not created. Build failed."
    exit 1
fi
echo "    App bundle created at ${APP_PATH}"

echo ""
echo "==> Step 2: Copying to /tmp to escape iCloud xattr interference..."
rm -rf "${SIGN_DIR}"
mkdir -p "${SIGN_DIR}"
cp -R "${APP_PATH}" "${SIGN_DIR}/"
xattr -cr "${SIGN_DIR}/Nyx.app"
echo "    Copied and cleaned: ${SIGN_DIR}/Nyx.app"

echo ""
echo "==> Step 3: Signing all embedded binaries (hardened runtime + timestamp)..."
# Sign all Mach-O binaries inside the bundle individually
find "${SIGN_DIR}/Nyx.app" -type f | while read -r f; do
    if file "$f" | grep -q "Mach-O"; then
        echo "    Signing: $(basename "$f")"
        codesign --force --options runtime --timestamp --sign "${IDENTITY}" "$f"
    fi
done

echo ""
echo "==> Step 4: Signing the app bundle..."
codesign --force --options runtime --timestamp --sign "${IDENTITY}" "${SIGN_DIR}/Nyx.app"
echo "    App signed"

# Verify
codesign --verify --deep --strict --verbose=2 "${SIGN_DIR}/Nyx.app" 2>&1
echo "    Verification passed"

echo ""
echo "==> Step 5: Creating DMG..."
mkdir -p "src-tauri/target/release/bundle/dmg"
hdiutil create -volname "Nyx" -srcfolder "${SIGN_DIR}/Nyx.app" \
    -ov -format UDZO "${SIGN_DIR}/${DMG_NAME}"
codesign --force --sign "${IDENTITY}" "${SIGN_DIR}/${DMG_NAME}"
echo "    DMG created and signed"

echo ""
echo "==> Step 6: Notarizing with Apple..."
if [[ -z "${APPLE_ID:-}" || -z "${APPLE_PASSWORD:-}" ]]; then
    echo "    WARNING: APPLE_ID or APPLE_PASSWORD not set — skipping notarization"
    echo "    Set them in .env and re-run, or notarize manually:"
    echo "    xcrun notarytool submit '${SIGN_DIR}/${DMG_NAME}' --apple-id \$APPLE_ID --password \$APPLE_PASSWORD --team-id ${TEAM_ID} --wait"
    cp "${SIGN_DIR}/${DMG_NAME}" "${FINAL_DMG}"
    echo "    Unsigned DMG copied to ${FINAL_DMG}"
    exit 0
fi

xcrun notarytool submit "${SIGN_DIR}/${DMG_NAME}" \
    --apple-id "${APPLE_ID}" \
    --password "${APPLE_PASSWORD}" \
    --team-id "${TEAM_ID}" \
    --wait

echo ""
echo "==> Step 7: Stapling notarization ticket..."
xcrun stapler staple "${SIGN_DIR}/${DMG_NAME}"

# Copy final DMG back to project
cp "${SIGN_DIR}/${DMG_NAME}" "${FINAL_DMG}"

echo ""
echo "==> Done! Signed + notarized DMG at:"
echo "    ${FINAL_DMG}"

# Cleanup
rm -rf "${SIGN_DIR}"
