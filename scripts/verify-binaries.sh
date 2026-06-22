#!/usr/bin/env bash
# Verify the vendored third-party binaries against their recorded SHA-256
# checksums and report each one's architecture. Run from anywhere.
set -euo pipefail

cd "$(dirname "$0")/.."
manifest="scripts/binary-checksums.sha256"

echo "==> Verifying vendored binary checksums"
# Strip comments/blank lines so the checker doesn't warn on them.
checkfile="$(mktemp)"
trap 'rm -f "$checkfile"' EXIT
grep -vE '^[[:space:]]*(#|$)' "$manifest" > "$checkfile"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c "$checkfile"
else
  shasum -a 256 -c "$checkfile"
fi

echo
echo "==> Architectures"
while read -r _hash path; do
  case "$_hash" in ''|\#*) continue ;; esac
  if [ -f "$path" ]; then
    printf '  %-40s %s\n' "$path" "$(file -b "$path" | cut -d',' -f1-2)"
  fi
done < "$manifest"
