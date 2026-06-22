# Bundled binaries — provenance

Nyx ships a few prebuilt binaries inside the app bundle. Their SHA-256 checksums
are recorded in [`scripts/binary-checksums.sha256`](scripts/binary-checksums.sha256)
and verified by [`scripts/verify-binaries.sh`](scripts/verify-binaries.sh) (run
in CI). Update the manifest whenever a binary is intentionally replaced.

| File | Arch | Purpose | Source / provenance |
|------|------|---------|---------------------|
| `src-tauri/resources/bin/jq` | macOS arm64 (Mach-O) | JSON processing for the ClawdTalk skill shell scripts | Official jq 1.7.1 release: <https://github.com/jqlang/jq/releases/download/jq-1.7.1/jq-macos-arm64> (MIT). sha256 `0bbe619e…2e8a`. |
| `src-tauri/resources/bin/gog` | macOS arm64 (Mach-O) | Google Workspace CLI used by the Google integration | Vendored `gog` CLI (v0.12.0). **TODO:** pin exact upstream release URL + add reproducible build notes. |
| `src-tauri/resources/bin/gog-linux-arm64` | Linux arm64 (ELF) | Linux variant of `gog` (Docker-era; see note) | Same upstream as `gog`. **TODO:** confirm still needed. |
| `src-tauri/binaries/nyx-mcp-aarch64-apple-darwin` | macOS arm64 (Mach-O) | Nyx MCP server (Model Context Protocol) | **Built from source in this repo** (`cargo build --bin nyx-mcp`). Rebuilt by the release workflow, so it is intentionally **not** in the checksum manifest. |

## Notes

- **`gog` provenance is incomplete.** The macOS and Linux `gog` binaries are
  vendored without a recorded upstream URL or build recipe. A maintainer should
  pin the exact upstream release and document how to reproduce them, then update
  the checksum manifest.
- **`gog-linux-arm64` and (previously) the Linux `jq`** are artifacts of the old
  OpenClaw-in-Docker architecture. With the move to a native daemon they may no
  longer be needed; confirm and remove if so.
- **SBOM / signing:** generating a full SBOM and signing the vendored binaries
  is future work; the checksum manifest is the current integrity control.
