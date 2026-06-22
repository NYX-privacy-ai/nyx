//! End-to-end-ish setup test: point HOME at a throwaway directory and exercise
//! the real config-writing path, asserting the generated files, their contents,
//! and their permissions. Runs as its own test binary so the HOME override is
//! isolated from the in-crate unit tests. Single test on purpose (HOME is a
//! process-global, so we don't want parallel tests fighting over it).

use nyx_lib::config;
use std::fs;

#[test]
fn setup_writes_expected_files_and_permissions_in_temp_home() {
    let tmp = std::env::temp_dir().join(format!("nyx-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    std::env::set_var("HOME", &tmp);

    let nyx = tmp.join(".nyx");

    // 1) Directory scaffolding, with a 0700 secrets dir.
    config::create_directories().expect("create_directories");
    assert!(nyx.join("secrets").is_dir(), "secrets dir should exist");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(nyx.join("secrets"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o700, "secrets dir should be chmod 700");
    }

    // 2) Guardrails file: written chmod 600 with the long env-var names the
    //    Python helper actually reads (regression guard for the name mismatch).
    let guardrails = config::GuardrailsConfig::default();
    config::write_guardrails(&guardrails).expect("write_guardrails");
    let gpath = nyx.join("secrets/defi_guardrails.env");
    let content = fs::read_to_string(&gpath).expect("read guardrails");
    assert!(content.contains("MAX_SINGLE_TX_USD="), "uses long tx name");
    assert!(
        content.contains("DAILY_LOSS_LIMIT_PCT="),
        "uses long loss name"
    );
    assert!(
        content.contains("BURROW_MIN_HEALTH_FACTOR="),
        "uses long HF name"
    );
    assert!(
        !content.contains("MAX_TX_USD="),
        "must not use the old short name"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&gpath).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "guardrails file should be chmod 600");
    }

    // 3) Invalid guardrails must be rejected before they can be persisted.
    let mut bad = config::GuardrailsConfig::from_preset(config::SecurityPreset::Balanced);
    bad.daily_loss_percent = 150.0; // > 100%
    assert!(
        config::write_guardrails(&bad).is_err(),
        "invalid guardrails rejected"
    );

    let _ = fs::remove_dir_all(&tmp);
}
