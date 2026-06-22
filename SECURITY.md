# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.x     | Yes       |
| 1.x     | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in Nyx, please report it responsibly:

1. **Do not** open a public GitHub issue.
2. Email your findings to the repository maintainer (see the GitHub org profile)
   or use GitHub's private "Report a vulnerability" advisory flow.
3. Include a description, steps to reproduce, and potential impact.
4. Allow reasonable time for a fix before public disclosure.

## Architecture (v2)

Nyx is a local-first macOS desktop app (Tauri v2 + SvelteKit). The agent runtime
is a **native IronClaw daemon** running under `~/.nyx` — the older
OpenClaw-in-Docker model from v1 has been removed. The app talks to whichever AI
/ DeFi providers you configure; nothing is sent off-device except those calls
(see [docs/DATA_AND_PRIVACY.md](docs/DATA_AND_PRIVACY.md)).

### Where data lives

All local state is under `~/.nyx`:

| Path | Contents | Permissions |
|------|----------|-------------|
| `~/.nyx/.env` | Provider/bot/gateway tokens | 600 |
| `~/.nyx/config.toml` | Agent configuration | 644 |
| `~/.nyx/secrets/` (dir) | Wallet keys, function-call keys, guardrails | 700 |
| `~/.nyx/secrets/near_account.json` | NEAR wallet (account id + private key) | 600 |
| `~/.nyx/secrets/wallets/<id>.json` | Per-wallet key material | 600 |
| `~/.nyx/secrets/defi_guardrails.env` | Guardrail values | 600 |
| `~/.nyx/ironclaw.db`, `sessions.db`, `intelligence.db` | Agent/session/intelligence data | 600/644 |

> **Secret-at-rest status:** secrets are currently protected by Unix file
> permissions (0600) only — they are **not** yet encrypted or stored in the
> macOS Keychain. Keychain-backed storage is planned. Treat your home directory
> accordingly.

A one-shot **purge** command (`purge_local_data`) deletes the entire `~/.nyx`
tree (wallet keys, tokens, databases, caches) after stopping the daemon.

### Webview / IPC hardening

- A real **Content-Security-Policy** is set (no longer `null`): `script-src
  'self'`, `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`,
  with an explicit `connect-src` allowlist for the local gateway, provider APIs,
  and the GitHub updater.
- The embedded terminal (`pty_spawn`) is restricted to an **allowlist** (only
  `claude`), not an arbitrary command.
- The browser agent cannot run arbitrary JavaScript (`execute_js` is not exposed
  as an agent action), and form reads **redact** password/payment/secret-looking
  field values before they reach the model.

### DeFi guardrails

DeFi limits are enforced in the Python helper (`guardrails.py`) and validated in
Rust before being written to disk (NaN/negative/out-of-range values are
rejected). Presets:

| Guardrail | Conservative | Balanced | Autonomous |
|-----------|-------------|----------|------------|
| Max transaction (USD) | 100 | 500 | 10,000 |
| Daily loss limit | 2% | 5% | 25% |
| Weekly loss limit | 5% | 15% | 50% |
| Max slippage | 1% | 2% | 5% |
| Max daily transactions | 10 | 20 | 100 |
| Max concentration | 25% | 40% | 75% |
| Min health factor | 2.0 | 1.5 | 1.3 |

> **Known limitation:** the configured limits govern the strategy/heartbeat
> paths. The direct `execute_*` swap/transaction commands do not yet enforce a
> Rust-side confirmation gate — treat autonomous live-funds use with care until
> that lands.

### Update security

Updates are signed with a minisign keypair; the Tauri updater verifies the
signature before installing.

### Supply chain

Vendored binaries (`jq`, `gog`) are checksum-pinned — see
[BINARIES.md](BINARIES.md) and `scripts/verify-binaries.sh` (run in CI).

## Privacy defaults

- No telemetry, no analytics, no phone-home.
- A fully local inference option (Ollama) is available.
- DeFi operations prefer ZEC shielded transactions where feasible.
