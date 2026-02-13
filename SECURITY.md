# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 1.x | Yes |

## Reporting a Vulnerability

If you discover a security vulnerability in Nyx, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Email your findings to the repository maintainer (see GitHub profile)
3. Include a description of the vulnerability, steps to reproduce, and potential impact
4. Allow reasonable time for a fix before public disclosure

## Security Architecture

### Local-First Design

Nyx runs entirely on your machine. The AI agent operates inside a Docker container with no inbound network access — it can only make outbound API calls that you have explicitly configured.

### Credential Injection at Boundary (IronClaw Pattern)

Credentials are injected into the Docker container as **environment variables** at startup. They are never mounted as files.

```
Host: ~/openclaw/docker.env (chmod 600)
  |
  v
Docker Compose: env_file directive
  |
  v
Container: process environment only — no file mounts for secrets
```

- The `~/.openclaw/secrets/` directory is NOT mounted into the container
- Private keys exist only as env vars inside the running process
- If the container filesystem is compromised, no secrets are on disk

### Leak Detection

All DeFi tool output is piped through a leak detector that scans for secret patterns before any text reaches the agent:

- `ed25519:` (NEAR private keys)
- `sk-ant-` (Anthropic API keys)
- `sk-proj-` (OpenAI API keys)
- `AKIA` (AWS access keys)
- `ghp_` (GitHub tokens)
- 64-character hex strings (private keys)
- Phone number patterns

If a match is found, the output is redacted and a warning is logged.

### Hardened Executable Whitelist

The agent can only execute a specific set of binaries:

```
gog, ls, find, wc, date, openclaw, curl
```

Common secret exfiltration vectors (`cat`, `grep`, `head`, `tail`, `echo`, `python3`, `sh`, `bash`) are explicitly excluded.

### DeFi Security Guardrails

All DeFi operations pass through enforced guardrails that cannot be overridden by the agent:

| Guardrail | Conservative | Balanced | Autonomous |
|-----------|-------------|----------|------------|
| Max transaction | $100 | $500 | $10,000 |
| Daily loss limit | 2% | 5% | 50% |
| Weekly loss limit | 5% | 15% | 100% |
| Max slippage | 1% | 2% | 5% |
| Max daily transactions | 5 | 20 | 100 |
| Max concentration | 30% | 60% | 95% |
| Min health factor | 2.0 | 1.5 | 1.3 |

### Messaging Autonomy

Each messaging channel has a configurable autonomy level:

- **Draft Only** (default): Agent writes drafts. User must manually send.
- **Send with Confirmation**: Agent composes, shows to user, waits for approval.
- **Autonomous**: Agent sends directly. All messages logged.

### Update Security

App updates are signed with a minisign keypair. The Tauri updater verifies the signature before installing any update.

### Privacy Defaults

- Financial operations default to ZEC shielded transactions where feasible
- Local AI option (Ollama) available for fully offline inference
- NEAR.ai uses TEE (Trusted Execution Environment) for confidential compute
- No telemetry, no analytics, no phone-home

## Configuration Files

| File | Purpose | Permissions |
|------|---------|-------------|
| `~/.openclaw/openclaw.json` | Agent configuration | 644 |
| `~/openclaw/docker.env` | API keys and credentials | 600 |
| `~/.openclaw/secrets/near_account.json` | NEAR wallet (host only) | 600 |
| `~/.openclaw/secrets/function_call_keys.json` | DeFi access keys | 600 |
| `~/.openclaw/secrets/defi_guardrails.env` | Guardrail values | 600 |
