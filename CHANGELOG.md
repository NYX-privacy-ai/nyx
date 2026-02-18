# Changelog

All notable changes to Nyx will be documented in this file.

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
