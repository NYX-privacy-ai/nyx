#!/usr/bin/env python3
"""Leak detector — scans text for secret patterns and redacts them.

Inspired by IronClaw's dual-layer leak detection approach.
Used as an import by wrapper scripts to sanitize tool output before
it reaches the LLM.

Usage:
    from leak_detector import scan_and_redact
    safe_output = scan_and_redact(raw_output)
"""

import os
import re
import sys
from typing import List, Tuple

# Secret patterns: (name, regex, severity)
# severity: "critical" = block entirely, "high" = redact, "low" = warn
SECRET_PATTERNS: List[Tuple[str, re.Pattern, str]] = [
    ("near_private_key", re.compile(r"ed25519:[A-Za-z0-9+/]{40,}"), "critical"),
    ("anthropic_key", re.compile(r"sk-ant-[A-Za-z0-9_\-]{20,}"), "critical"),
    ("openai_key", re.compile(r"sk-proj-[A-Za-z0-9_\-]{20,}"), "critical"),
    ("openai_key_old", re.compile(r"sk-[A-Za-z0-9]{48}"), "critical"),
    ("aws_key", re.compile(r"AKIA[0-9A-Z]{16}"), "critical"),
    ("github_token", re.compile(r"ghp_[A-Za-z0-9]{36}"), "critical"),
    ("github_token_fine", re.compile(r"github_pat_[A-Za-z0-9_]{22,}"), "critical"),
    ("stripe_key", re.compile(r"sk_live_[A-Za-z0-9]{24,}"), "critical"),
    ("telegram_token", re.compile(r"\d{8,10}:[A-Za-z0-9_\-]{35}"), "high"),
    ("jwt_token", re.compile(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+"), "high"),
    ("pem_key", re.compile(r"-----BEGIN[A-Z ]*PRIVATE KEY-----"), "critical"),
    ("hex_secret_64", re.compile(r"(?<![A-Za-z0-9])[0-9a-f]{64}(?![A-Za-z0-9])"), "high"),
    ("base64_long", re.compile(r"(?<![A-Za-z0-9+/])[A-Za-z0-9+/]{80,}={0,2}(?![A-Za-z0-9+/])"), "low"),
]

# Also check for known env var values if they're set
_KNOWN_SECRETS: List[str] = []


def _load_known_secrets():
    """Load actual secret values from env vars for exact-match detection."""
    global _KNOWN_SECRETS
    secret_env_vars = [
        "NEAR_PRIVATE_KEY", "ANTHROPIC_API_KEY", "OPENAI_API_KEY",
        "TELEGRAM_BOT_TOKEN", "GOG_KEYRING_PASSWORD",
        "OPENCLAW_GATEWAY_TOKEN",
    ]
    for var in secret_env_vars:
        val = os.environ.get(var, "")
        if val and len(val) > 8:
            _KNOWN_SECRETS.append(val)


def scan(text: str) -> List[dict]:
    """Scan text for secret patterns. Returns list of matches."""
    if not _KNOWN_SECRETS:
        _load_known_secrets()

    matches = []

    # Check for exact known secret values
    for secret in _KNOWN_SECRETS:
        if secret in text:
            matches.append({
                "type": "known_secret",
                "severity": "critical",
                "preview": secret[:4] + "..." + secret[-4:],
            })

    # Check regex patterns
    for name, pattern, severity in SECRET_PATTERNS:
        for m in pattern.finditer(text):
            val = m.group()
            matches.append({
                "type": name,
                "severity": severity,
                "preview": val[:6] + "..." + val[-4:] if len(val) > 14 else "[MATCH]",
                "start": m.start(),
                "end": m.end(),
            })

    return matches


def redact(text: str) -> str:
    """Redact all detected secrets from text."""
    if not _KNOWN_SECRETS:
        _load_known_secrets()

    result = text

    # Redact exact known values first
    for secret in _KNOWN_SECRETS:
        if secret in result:
            result = result.replace(secret, "[REDACTED]")

    # Redact pattern matches (process longest matches first to avoid partial redaction)
    all_matches = []
    for name, pattern, severity in SECRET_PATTERNS:
        if severity in ("critical", "high"):
            for m in pattern.finditer(result):
                all_matches.append((m.start(), m.end(), name))

    # Sort by position descending so replacements don't shift indices
    all_matches.sort(key=lambda x: x[0], reverse=True)
    for start, end, name in all_matches:
        result = result[:start] + f"[REDACTED:{name}]" + result[end:]

    return result


def scan_and_redact(text: str) -> str:
    """Scan text and return redacted version. Main entry point."""
    matches = scan(text)
    if not matches:
        return text

    critical = [m for m in matches if m["severity"] == "critical"]
    if critical:
        # Log the detection (to stderr, which goes to container logs, not to LLM)
        for m in critical:
            print(f"[LEAK_DETECTOR] CRITICAL: {m['type']} detected ({m['preview']})",
                  file=sys.stderr)

    return redact(text)


if __name__ == "__main__":
    # CLI mode: pipe text through for redaction
    import sys
    text = sys.stdin.read()
    print(scan_and_redact(text))
