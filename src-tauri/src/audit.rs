//! Lightweight append-only security audit log.
//!
//! Sensitive operations (wallet generation, config/secret rewrites, live DeFi
//! execution, arbitrary browser JS, data purge, installs) append a one-line
//! record to `~/.nyx/logs/security-audit.log` so there is a forensic trail of
//! security-relevant actions. This is deliberately best-effort: auditing must
//! never block or fail the operation it records.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn audit_log_path() -> PathBuf {
    crate::ironclaw::config_dir().join("logs/security-audit.log")
}

/// Format a single audit line. `details` must NOT contain secret values; any
/// newlines are flattened so each event stays on exactly one line.
fn format_entry(timestamp: &str, event: &str, details: &str) -> String {
    let safe = details.replace(['\n', '\r'], " ");
    if safe.is_empty() {
        format!("{timestamp} {event}\n")
    } else {
        format!("{timestamp} {event} {safe}\n")
    }
}

/// Append a security-relevant event to the audit log (chmod 600). Best-effort
/// and non-fatal. Never pass secret values in `details`.
pub fn record(event: &str, details: &str) {
    let ts = chrono::Utc::now().to_rfc3339();
    let line = format_entry(&ts, event, details);
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_entry;

    #[test]
    fn entry_is_single_line_and_flattens_injection() {
        let e = format_entry(
            "2026-06-22T00:00:00Z",
            "wallet.generate",
            "acct=abc\ninjected",
        );
        assert!(e.ends_with('\n'));
        assert_eq!(e.matches('\n').count(), 1, "exactly one newline");
        assert!(e.contains("wallet.generate"));
        assert!(e.contains("acct=abc injected"));
    }

    #[test]
    fn entry_without_details() {
        assert_eq!(
            format_entry("T", "purge_local_data", ""),
            "T purge_local_data\n"
        );
    }
}
