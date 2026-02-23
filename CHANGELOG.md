# Changelog

All notable changes to Nyx will be documented in this file.
Format based on [Keep a Changelog](https://keepachangelog.com/); versions follow [Semantic Versioning](https://semver.org/).

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
