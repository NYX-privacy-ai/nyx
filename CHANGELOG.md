# Changelog

All notable changes to Nyx will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/); versions follow [Semantic Versioning](https://semver.org/).

## [2.0.4] — 2026-04-09

### Fixed

- **Conversation history unbounded** — chat sessions were sending the full message history on every request with no limit, causing context poisoning in long sessions. Now capped at 40 messages (20 turns).
- **DeFi tools path mismatch** — HEARTBEAT.md, SKILL.md, and cron job prompts referenced `/opt/near-intents-helper/` which was never created during setup. All references corrected to `~/.nyx/near-intents-helper/`.
- **Dual heartbeat** — IronClaw's native heartbeat (every 30 min) overlapped with the dedicated cron heartbeat job (every 4h). Native heartbeat now disabled; cron jobs own all periodic work with proper session isolation.
- **Python venv bootstrap** — run_near_intents.sh hardcoded a Docker-era container venv path that never exists on native macOS. Now auto-creates the local venv on first invocation if missing.
- **Email config always defaulting** — `read_email_config` called `.as_array()` on the top-level JSON object instead of the nested `jobs` array; also used wrong key names (`"cron"` / `"timezone"` vs `"expr"` / `"tz"`). Settings page now correctly reads back configured email schedule.
- **Stale default model** — Anthropic default updated from `claude-sonnet-4-20250514` to `claude-sonnet-4-6`.

### Changed

- **Release workflow** — GitHub Actions now uploads a stable `Nyx_aarch64.dmg` asset (no version suffix) alongside the versioned artifact, so the website download link never needs updating.

## [2.0.0] — 2026-03-04

### Changed

- **Backend migrated from OpenClaw (Docker) to IronClaw (native daemon)** — Nyx no longer requires Docker Desktop. The AI agent runs as a native IronClaw daemon managed via macOS LaunchAgent (`com.nyx.daemon`). All config stored under `~/.nyx/`.
- **Gateway repointed** — HTTP gateway now communicates with IronClaw on port 3000 (auto-fallback to 3001 if port occupied).
- **Session management** — Conversation history now managed client-side in local SQLite (`~/.nyx/sessions.db`) since IronClaw gateway is stateless.
- **Config format** — OpenClaw JSON config (`openclaw.json`) replaced with IronClaw TOML (`config.toml`).
- **Environment** — `OPENCLAW_GATEWAY_TOKEN` → `GATEWAY_AUTH_TOKEN`, credentials stored in `~/.nyx/.env`.
- **Setup wizard** — No longer requires Docker Desktop prerequisite. Checks for IronClaw binary, auto-installs via `cargo install`, starts daemon.
- **Settings page** — Docker container controls replaced with IronClaw daemon controls (start/stop/restart).
- **MCP tools** — `nyx_docker_status` → `nyx_ironclaw_status`.
- **Website** — Updated to reflect IronClaw architecture.

### Fixed

- **Port collision detection** — Auto-detects if another IronClaw instance occupies port 3000, falls back to 3001.
- **Gateway port persistence** — `GATEWAY_PORT` now written to `.env` and preserved across settings updates.
- **Daemon script portability** — Uses PATH resolution for ironclaw binary instead of hardcoded path.
- **ClawdTalk setup order** — Voice calling now configured after directory creation (was silently failing).

### Removed

- Docker Desktop dependency (Docker rollback code kept in `docker.rs` for transition period).
- Unused `gog-linux-arm64` binary copy (was only needed inside Docker container).

## [1.4.1] — 2026-02-23

### Fixed

- **Cron jobs failing silently** — OpenClaw's cron engine requires `agentId` and `id` fields on every job, but Nyx was using non-standard field names (`prompt` instead of `payload`, `schedule.cron` instead of `schedule.expr`, `delivery.channel` instead of `delivery.mode`). All cron jobs silently fell back to the default agent on a single shared session lane, causing serialization bottlenecks and permanently stuck jobs.
- **Bundled `jobs.json` corrected** — added `id` and `agentId` fields to all 4 jobs; changed `delivery.mode` from `"announce"` (triggers gateway pairing hang) to `"none"`.
- **`config.rs` `write_cron_jobs()` rewritten** — now generates correct OpenClaw v2 job structure: `payload` with `kind: "agentTurn"`, proper `schedule` format (`kind`/`expr`/`tz`), `sessionTarget: "isolated"`, `state` with `nextRunAtMs`, and `delivery.mode: "none"`. Jobs generated from settings updates now match the bundled defaults.

## [1.4.0] — 2026-02-20

### Security

- **Gateway bind hardened** — `OPENCLAW_GATEWAY_BIND` changed from `0.0.0.0` to `localhost`, preventing unintended network exposure.
- **safeBins hardened** — removed `cat`, `grep`, `head`, `tail` (secret exfiltration vectors); added `touch`.
- **Egress proxy** — all container web traffic now routes through a Squid proxy (`egress-proxy` service) for network-layer control.

### Added

- **Container browser automation** — Playwright Chromium support with tmpfs mounts for Chrome NSS keystore and fontconfig, volume mounts for Playwright binaries and shared libraries, and `LD_LIBRARY_PATH` for Chromium deps.
- **Browser config in `openclaw.json`** — headless Chromium with `noSandbox` and explicit executable path for the containerised environment.
- **`squid.conf` bundled** — egress proxy configuration deployed during setup.
- **Browser-libs and Playwright directories** created during initial setup.

### Added

- **Perplexity web search** — setup wizard and settings page support for Perplexity API key; web search provider auto-configured when key is present.
- **Privacy Shield execution** — "Shield Now" and "Convert Now" buttons are now fully functional, executing live ZEC shield/unshield swaps via NEAR Intents.
- **Signal messaging** — Signal channel config now persists through setup, settings, and docker.env (previously silently dropped).
- **Messaging autonomy persistence** — channel autonomy levels (DraftOnly, SendWithConfirm, Autonomous) now saved to and read from docker.env.
- **NEAR credentials in docker.env** — `NEAR_ACCOUNT_ID`, `NEAR_NETWORK_ID`, and `SOLVER_RELAY_URL` written automatically for the active NEAR wallet.
- **Min Health Factor** input added to custom guardrails editor in setup wizard.
- **Auto-paste** now detects Perplexity (`pplx-`) API keys.
- **Default LLM model** — `agents.defaults.model` set in openclaw.json based on the selected provider.
- **Slack skill** added to `skills.allowBundled` when Slack token is configured.

### Changed

- **OpenClaw image upgraded** from `2026.2.9` to `2026.2.17` across all references (Docker Compose, config template, setup, prepull).
- **Docker Compose rewritten** — added `egress-proxy` service, Chrome tmpfs mounts, Playwright + browser-libs volumes, proxy environment variables.
- **ClawdTalk ordering** — voice calling configured before main setup so safeBins and skill entries are included in initial openclaw.json.
- **LLM provider validation** — setup wizard now requires the selected default provider to have a valid API key before proceeding.
- **BigInt precision** — privacy shield amount conversion uses `BigInt(10) ** BigInt(decimals)` to avoid float precision loss for high-decimal assets.
- **Dashboard navigation** uses SvelteKit `goto()` instead of `window.location.href`.
- **SaveBar reactivity** — `restartRequired` uses `$derived.by()` for correct reactive computation.
- **Dashboard portfolio** — `get_portfolio` called on mount to populate positions, allocation, and health data.

### Fixed

- **Billing patches updated** — four `pi-embedded-helpers-*.js` patch files replaced with hashes matching the 2026.2.17 image.

## [1.3.1] — 2026-02-19

### Fixed

- **Google Workspace broken in Docker** — macOS ARM64 `gog` binary was mounted into the Linux container since v1.0.0, causing `exec format error` on every Google command. Now bundles a separate Linux ARM64 `gog` binary (`gog-linux-arm64`) and mounts that into the container instead. Google Calendar, Gmail, Drive, Contacts, Sheets, and Docs all work correctly now.
- **`gog` upgraded to v0.11.0** (from v0.9.0) as part of the Linux binary addition.
- Setup (`config.rs`, `google.rs`) now copies both macOS and Linux gog binaries during initial install.
- Docker Compose template updated to mount `gog-linux-arm64` for both gateway and CLI services.

## [1.3.0] — 2026-02-18

### Added

- **Web Browsing** — agent-controlled browser for navigating websites on the user's behalf (booking travel, ordering groceries, filling forms). Opens a secondary WebView window with real-time activity feed. 25-iteration safety limit. Never enters passwords or payment details.
- **Activity Intelligence** — background observer that watches calendar and email patterns, learns from user behaviour, and offers proactive suggestions. Includes privacy controls and autonomy levels (Observe, Suggest, Draft, Autonomous).
- **Claude Code Integration** — bidirectional MCP server for Claude Code to access Nyx capabilities, plus embedded terminal for direct Claude Code sessions.
- **Browse Page** — new `/browse` route with URL bar, back/forward navigation, command input for natural language instructions, and real-time activity feed showing each action the agent takes.
- **Web Browsing capability toggle** in Settings and Setup (default: enabled).
- **Activity Intelligence restart notice** — amber banner prompts app restart when the feature is newly enabled.
- **Browse nav item** in sidebar (globe icon).

### Changed

- Bumped internal version to 1.3.0.
- Added `url` crate dependency for browser URL parsing.
- Added `web_browsing` field to `CapabilitiesConfig` (default: true).

### Fixed

- Cleaned up all build warnings: zero Rust warnings, zero Svelte warnings.

## [1.2.0] — 2026-02-16

### Added

- **Claude Code integration** — bidirectional MCP server + embedded terminal.
- **Privacy Shield** — shielded ZEC via NEAR Intents cross-chain swaps.

## [1.0.1] — 2026-02-14

### Fixed

- Minor bug fixes and stability improvements.

## [1.0.0] — 2026-02-13

### Added

- Initial release of Nyx — a private AI chief of staff for macOS
- **Setup Wizard** — guided configuration for API keys, LLM provider selection, NEAR wallet creation, DeFi security guardrails, messaging channels, email notifications, and capabilities
- **Settings Page** — full post-setup configuration with collapsible sections for Agent Identity, LLM Providers, DeFi Security, Messaging Channels, Email Notifications, Capabilities, App Updates, and System Status
- **Chat Interface** — markdown-rendered conversations with streaming responses, folder management, and message history
- **Source Verification** — credibility analysis with confidence scoring for claims and URLs
- **Portfolio Dashboard** — DeFi positions, allocation breakdown, health indicators, and transaction history
- **Local LLM Support** — one-click Ollama model downloads for recommended models, plus custom model pull by tag
- **Multi-Provider LLM** — Anthropic, OpenAI, Venice AI, NEAR.ai, and Ollama
- **DeFi Security Guardrails** — Conservative, Balanced, Autonomous, and Custom presets
- **NEAR Wallet** — Ed25519 keypair generation with deterministic implicit account ID
- **Auto-Updates** — checks for new versions on startup with in-app download and install
- **Messaging Channels** — Gmail, WhatsApp, Telegram, Slack with per-channel autonomy controls
- **Email Intelligence** — configurable daily digest and hourly triage schedule
- **Capabilities** — toggleable domains: DeFi, Travel, Google Workspace, Email Intelligence, Communications, Source Verification
- **Google Workspace** — Gmail, Calendar, Drive, Docs integration with OAuth
- **Docker Management** — start, stop, restart the agent container from the UI
- **System Tray** — menu bar icon for quick access
