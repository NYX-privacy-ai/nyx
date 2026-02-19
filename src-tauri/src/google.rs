use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct GogStatus {
    pub installed: bool,
    pub authenticated: bool,
    pub version: Option<String>,
}

/// Check if the `gog` CLI binary is available.
pub async fn check_gog_available() -> Result<GogStatus, String> {
    // Try ~/openclaw/bin/gog first, then PATH
    let gog_path = gog_binary_path();

    let output = Command::new(&gog_path)
        .args(["--version"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // Check if authenticated by trying to list calendars
            let auth_check = Command::new(&gog_path)
                .args(["calendar", "list", "--limit", "1"])
                .output();

            let authenticated = auth_check
                .map(|o| o.status.success())
                .unwrap_or(false);

            Ok(GogStatus {
                installed: true,
                authenticated,
                version: if version.is_empty() { None } else { Some(version) },
            })
        }
        _ => Ok(GogStatus {
            installed: false,
            authenticated: false,
            version: None,
        }),
    }
}

/// Run `gog auth` to initiate OAuth browser flow.
/// Returns when the auth process completes (user finishes in browser).
pub async fn run_gog_auth() -> Result<bool, String> {
    let gog_path = gog_binary_path();

    let output = Command::new(&gog_path)
        .args(["auth"])
        .output()
        .map_err(|e| format!("Failed to run gog auth: {}", e))?;

    if output.status.success() {
        Ok(true)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("gog auth failed: {}", stderr))
    }
}

/// Check if gog is authenticated (quick check).
pub async fn check_gog_authenticated() -> Result<bool, String> {
    let gog_path = gog_binary_path();

    let output = Command::new(&gog_path)
        .args(["calendar", "list", "--limit", "1"])
        .output();

    Ok(output.map(|o| o.status.success()).unwrap_or(false))
}

/// Install the gog CLI binary from bundled app resources.
pub async fn install_gog(app_handle: &tauri::AppHandle) -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let bin_dir = format!("{}/openclaw/bin", home);
    let gog_path = format!("{}/gog", bin_dir);

    // Create bin directory if it doesn't exist
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create bin directory: {}", e))?;

    // Resolve bundled resources directory (works in both dev and production)
    let resources_dir = crate::setup::resolve_resources_dir(app_handle)?;
    let bundled_gog = resources_dir.join("bin/gog");

    if !bundled_gog.exists() {
        return Err("Bundled gog binary not found in app resources".to_string());
    }

    // Copy bundled binary to install location
    std::fs::copy(&bundled_gog, &gog_path)
        .map_err(|e| format!("Failed to copy gog binary: {}", e))?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&gog_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
    }

    // Also copy the Linux ARM64 binary (used inside the Docker container)
    let bundled_gog_linux = resources_dir.join("bin/gog-linux-arm64");
    let gog_linux_path = format!("{}/gog-linux-arm64", bin_dir);
    if bundled_gog_linux.exists() {
        std::fs::copy(&bundled_gog_linux, &gog_linux_path)
            .map_err(|e| format!("Failed to copy gog-linux-arm64 binary: {}", e))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&gog_linux_path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to set linux gog permissions: {}", e))?;
        }
    }

    // Verify the macOS binary runs on the host
    let verify = Command::new(&gog_path)
        .args(["--version"])
        .output();

    match verify {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(version)
        }
        _ => Err("gog binary copied but failed to execute".to_string()),
    }
}

fn gog_binary_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_path = format!("{}/openclaw/bin/gog", home);

    // Prefer the bundled gog binary if it exists
    if std::path::Path::new(&local_path).exists() {
        local_path
    } else {
        "gog".to_string() // Fall back to PATH
    }
}
