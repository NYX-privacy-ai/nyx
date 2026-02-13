<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="128" alt="Nyx" />
</p>

<h1 align="center">Nyx</h1>

<p align="center">
  <strong>Your private AI chief of staff.</strong><br>
  A native macOS desktop app that connects your communications, calendars, wallets and documents through a single secure interface — running entirely on your machine.
</p>

<p align="center">
  <a href="https://github.com/macsco80/nyx/releases/latest">
    <img src="https://img.shields.io/github/v/release/macsco80/nyx?style=flat-square&color=6366F1" alt="Latest Release" />
  </a>
  <img src="https://img.shields.io/badge/platform-macOS%2012%2B-000?style=flat-square" alt="Platform" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="License" />
</p>

---

## What Nyx Does

Nyx is a native macOS app built with [Tauri v2](https://v2.tauri.app). It acts as a control surface for an AI agent that can manage your:

| Domain | Capabilities |
|--------|-------------|
| **DeFi & Crypto** | Cross-chain swaps (25+ chains via NEAR Intents), staking, lending, leveraged looping, portfolio management. ZEC shielded by default. |
| **Communications** | Telegram, WhatsApp, Slack messaging with configurable autonomy levels (draft, confirm, autonomous). |
| **Google Workspace** | Gmail, Calendar, Drive, Docs integration. |
| **Email Intelligence** | Hourly triage, daily digest, priority classification. |
| **Travel** | Flight, hotel, and transport research. |
| **Source Verification** | Credibility analysis and fact-checking for URLs and claims. |
| **Local AI** | Private on-device inference via Ollama. No data leaves your machine. |

Everything runs locally. Your data never leaves your machine unless you explicitly send it.

## Features

- **Guided Setup Wizard** — Walk through API key entry, model selection, security guardrails, messaging channels, and wallet creation in a single flow
- **Full Settings Page** — Change any configuration post-setup: API keys, default LLM provider, DeFi security presets, messaging autonomy, email schedules, capabilities, and more
- **Local LLM Support** — Run models locally via [Ollama](https://ollama.com) with one-click downloads for recommended models, or pull any model by tag
- **Multi-Provider LLM** — Anthropic, OpenAI, Venice AI, NEAR.ai, or local Ollama as your default provider
- **DeFi Security Guardrails** — Conservative, Balanced, Autonomous, or Custom presets controlling transaction limits, loss caps, slippage, concentration, and health factor thresholds
- **NEAR Wallet** — Generates an Ed25519 keypair during setup with a deterministic NEAR implicit account ID
- **Auto-Updates** — Checks for new versions on startup with in-app install, or check manually from Settings
- **Chat Interface** — Converse with your agent in a markdown-rendered chat with streaming responses
- **Source Verification** — Submit claims or URLs for credibility analysis with confidence scoring
- **Portfolio Dashboard** — View DeFi positions, allocation breakdown, health indicators, and transaction history

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| **macOS** | 12.0+ (Monterey) | Apple Silicon or Intel |
| **Docker Desktop** | 4.x+ | Required for the agent container |
| **Node.js** | 22+ | For frontend development |
| **Rust** | 1.80+ | For Tauri backend development |
| **Ollama** | Latest | Optional — for local LLM inference |

## Quick Start

### Download the App

Grab the latest `.dmg` from the [Releases](https://github.com/macsco80/nyx/releases/latest) page, mount it, and drag Nyx to your Applications folder.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/macsco80/nyx.git
cd nyx

# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production (.dmg + .app)
npm run tauri build
```

The bundles will be in `src-tauri/target/release/bundle/`.

## Architecture

```
nyx/
├── src/                    # SvelteKit frontend
│   ├── routes/
│   │   ├── +layout.svelte  # Sidebar nav, update banner
│   │   ├── +page.svelte    # Portfolio dashboard
│   │   ├── chat/           # Chat interface
│   │   ├── verify/         # Source verification
│   │   ├── setup/          # Setup wizard
│   │   └── settings/       # Settings page
│   └── lib/
│       └── components/     # Reusable UI components
├── src-tauri/              # Rust backend (Tauri v2)
│   ├── src/
│   │   ├── main.rs         # App entry, command registration
│   │   ├── config.rs       # Config read/write, settings
│   │   ├── docker.rs       # Docker container management
│   │   ├── gateway.rs      # Agent gateway API
│   │   ├── setup.rs        # Setup wizard backend
│   │   ├── wallet.rs       # NEAR wallet generation
│   │   ├── portfolio.rs    # DeFi portfolio data
│   │   └── google.rs       # Google Workspace auth
│   ├── capabilities/       # Tauri permission policies
│   └── resources/          # Bundled runtime files
├── .github/workflows/      # CI/CD release pipeline
└── tailwind.config.js      # Design tokens
```

**Frontend:** SvelteKit 2 with Svelte 5 runes (`$state`, `$derived`, `$effect`, `$props`), Tailwind CSS, static adapter.

**Backend:** Rust via Tauri v2 IPC commands. Manages Docker containers, file I/O, config parsing, wallet operations, and HTTP communication with the agent gateway.

**Agent Engine:** [OpenClaw](https://openclaw.com) runs in a Docker container and handles all AI reasoning, tool use, and external API calls. Nyx is the desktop interface.

## AI Provider Options

| Provider | Type | Key Feature |
|----------|------|-------------|
| Anthropic Claude | Cloud | Recommended default, best reasoning |
| OpenAI | Cloud | GPT-4o, voice features |
| Venice AI | Cloud | Privacy-first, no data retention |
| NEAR.ai | Cloud (TEE) | Confidential compute, data encrypted in transit and at rest |
| Ollama | Local | Fully private, no network required |

## Auto-Updates

Nyx uses the Tauri updater plugin. When a new version is available:

1. A notification banner appears at the bottom of the app window
2. Click **Update Now** to download and install
3. You can also check manually from **Settings > App Updates**

Updates are signed and served from GitHub Releases.

## Security

See [SECURITY.md](SECURITY.md) for the full security model.

Key points:
- Credentials injected at the Docker boundary as environment variables
- No secrets mounted as files inside the container
- Leak detection scans all DeFi tool output for secret patterns
- Hardened executable whitelist — no shell, no Python exposed to the agent
- Configurable DeFi guardrails (transaction limits, loss limits, health factors)
- All messaging channels default to Draft Only autonomy

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

[MIT](LICENSE)
