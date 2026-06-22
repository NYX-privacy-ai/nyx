# Data & Privacy

Nyx is local-first: it runs on your machine and stores everything under
`~/.nyx`. But it is also an AI app that talks to external providers, so some
actions **do** send data off your device. This document is the precise map of
*what leaves, when* and *what is stored, where, and how to delete it*.

## What leaves your device, and when

Nothing is sent anywhere unless an action you take requires it. Concretely:

| Feature / action | Goes off-device? | Where to | What is sent |
|------------------|------------------|----------|--------------|
| Chat with the agent (default model) | Yes | Anthropic API (`api.anthropic.com`) | Your messages + relevant context/memory |
| Chat with an alternate provider | Yes | OpenAI / Venice / NEAR.ai / Perplexity (whichever you selected) | Your messages + context |
| Local model (Ollama) | **No** | localhost only | — |
| Web browsing agent | Yes | The sites you browse **and** the model provider | Page text/links/forms read by the agent (form **secrets are redacted** first) |
| Source verification / intelligence | Yes | The configured model provider + any source URLs | The claim/URL and analysis prompt |
| Email / calendar (Google) | Yes | Google APIs (via `gog`) | Requests scoped to the operation |
| Messaging channels (Telegram/Slack/WhatsApp/Signal) | Yes | The respective platform | Messages you send/draft |
| Voice / SMS (ClawdTalk) | Yes | `clawdtalk.com` | Call/message payloads |
| DeFi (NEAR Intents, swaps, quotes) | Yes | NEAR solver relay / 1Click / on-chain RPC | Quote params, signed transactions |
| App updates | Yes | GitHub releases | Standard update check |

Cloud providers are subject to **their own** data-handling policies. Nyx does not
add telemetry, analytics, or any phone-home of its own.

## What is stored on your device, and how to delete it

All persistent state lives under `~/.nyx`:

| Path | What it stores |
|------|----------------|
| `~/.nyx/.env` | Provider/bot/gateway tokens |
| `~/.nyx/config.toml` | Agent configuration |
| `~/.nyx/secrets/` | NEAR wallet keys, function-call keys, guardrail values |
| `~/.nyx/intelligence.db` | Email/calendar/messaging observations, contacts, suggestions, tasks, wiki/knowledge entries |
| `~/.nyx/sessions.db` | Chat session history |
| `~/.nyx/ironclaw.db` | Agent runtime state/memory |
| `~/.nyx/defi-state/` | DeFi tx counters, PnL, halt flag |
| `~/.nyx/logs/` | Local logs |
| `~/.nyx/local-skills/clawdtalk/skill-config.json` | ClawdTalk config (incl. API key) |
| `~/.nyx/workspace/` | Agent workspace docs (e.g. SOUL.md) |

### Deleting data

- **Intelligence only:** the `clear_intelligence_data` command wipes the
  intelligence database (email/calendar/messaging observations, contacts,
  suggestions) while leaving wallets and config intact.
- **Everything (factory reset):** the `purge_local_data` command stops the
  daemon and removes the entire `~/.nyx` tree — wallet keys, tokens, all
  databases, caches, logs, and workspace. After purging you must re-run setup.

### Retention

Nyx does not expire or rotate local data automatically — it stays until you
delete it with one of the flows above (or remove the files yourself). There is
no server-side copy held by Nyx; any retention beyond your device is governed by
the third-party providers you chose to use.
