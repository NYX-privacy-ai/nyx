use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerCheck {
    pub installed: bool,
    pub running: bool,
    pub version: Option<String>,
    pub download_url: Option<String>,
}

/// Detailed Docker status: installed, running, version, download link.
pub async fn check_docker_detailed() -> Result<DockerCheck, String> {
    // Check if docker binary exists
    let version_output = Command::new("docker")
        .args(["--version"])
        .output();

    let (installed, version) = match version_output {
        Ok(out) if out.status.success() => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            (true, if v.is_empty() { None } else { Some(v) })
        }
        _ => (false, None),
    };

    // Check if Docker daemon is running
    let running = if installed {
        Command::new("docker")
            .args(["info"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    // Architecture-aware download URL
    let download_url = if !installed {
        Some(get_docker_download_url())
    } else {
        None
    };

    Ok(DockerCheck {
        installed,
        running,
        version,
        download_url,
    })
}

/// Get the Docker Desktop download URL based on the current macOS architecture.
pub fn get_docker_download_url() -> String {
    let arch = std::env::consts::ARCH;
    if arch == "aarch64" {
        "https://desktop.docker.com/mac/main/arm64/Docker.dmg".to_string()
    } else {
        "https://desktop.docker.com/mac/main/amd64/Docker.dmg".to_string()
    }
}

/// Check if Docker Desktop is running.
pub async fn is_docker_running() -> Result<bool, String> {
    let output = Command::new("docker")
        .args(["info"])
        .output()
        .map_err(|e| format!("Docker not found: {}", e))?;

    Ok(output.status.success())
}

/// Start the openclaw-gateway container.
pub async fn start_container() -> Result<(), String> {
    let home = dirs_next().ok_or("Cannot determine home directory")?;
    let compose_file = format!("{}/openclaw/docker-compose.yml", home);

    let output = Command::new("docker")
        .args(["compose", "-f", &compose_file, "up", "-d", "openclaw-gateway"])
        .output()
        .map_err(|e| format!("Failed to start container: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Container start failed: {}", stderr))
    }
}

/// Stop the openclaw-gateway container.
pub async fn stop_container() -> Result<(), String> {
    let home = dirs_next().ok_or("Cannot determine home directory")?;
    let compose_file = format!("{}/openclaw/docker-compose.yml", home);

    let output = Command::new("docker")
        .args(["compose", "-f", &compose_file, "stop", "openclaw-gateway"])
        .output()
        .map_err(|e| format!("Failed to stop container: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Container stop failed: {}", stderr))
    }
}

/// Get container status.
pub async fn container_status() -> Result<String, String> {
    let output = Command::new("docker")
        .args(["ps", "--filter", "name=openclaw-gateway", "--format", "{{.Status}}"])
        .output()
        .map_err(|e| format!("Failed to check status: {}", e))?;

    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if status.is_empty() {
        Ok("stopped".to_string())
    } else {
        Ok(status)
    }
}

/// Restart the openclaw-gateway container (stop + start).
pub async fn restart_container() -> Result<(), String> {
    stop_container().await?;
    // Brief pause for clean shutdown
    std::thread::sleep(std::time::Duration::from_secs(2));
    start_container().await
}

/// Download and install Docker Desktop from the official DMG.
///
/// Steps:
///   1. Download Docker.dmg (architecture-aware) to a temp file
///   2. Mount the DMG via `hdiutil attach`
///   3. Copy Docker.app to /Applications
///   4. Unmount the DMG
///   5. Launch Docker.app so the daemon starts
///   6. Wait briefly and verify `docker --version`
pub async fn install_docker() -> Result<String, String> {
    let url = get_docker_download_url();
    let tmp_dmg = "/tmp/DockerDesktop.dmg";

    // 1. Download
    let output = Command::new("curl")
        .args(["-fSL", "--progress-bar", "-o", tmp_dmg, &url])
        .output()
        .map_err(|e| format!("Failed to download Docker Desktop: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Download failed: {}", stderr));
    }

    // 2. Mount
    let output = Command::new("hdiutil")
        .args(["attach", tmp_dmg, "-nobrowse", "-quiet"])
        .output()
        .map_err(|e| format!("Failed to mount DMG: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(tmp_dmg);
        return Err(format!("Mount failed: {}", stderr));
    }

    // Find the mount point — Docker DMGs mount at /Volumes/Docker
    let mount_point = "/Volumes/Docker";

    // 3. Copy Docker.app to /Applications
    let output = Command::new("cp")
        .args(["-R", &format!("{}/Docker.app", mount_point), "/Applications/Docker.app"])
        .output()
        .map_err(|e| format!("Failed to copy Docker.app: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Unmount before returning error
        let _ = Command::new("hdiutil")
            .args(["detach", mount_point, "-quiet"])
            .output();
        let _ = std::fs::remove_file(tmp_dmg);
        return Err(format!("Install failed: {}", stderr));
    }

    // 4. Unmount
    let _ = Command::new("hdiutil")
        .args(["detach", mount_point, "-quiet"])
        .output();

    // Clean up temp file
    let _ = std::fs::remove_file(tmp_dmg);

    // 5. Launch Docker.app (starts the daemon)
    let _ = Command::new("open")
        .args(["/Applications/Docker.app"])
        .output();

    // 6. Brief wait then verify
    std::thread::sleep(std::time::Duration::from_secs(3));

    let verify = Command::new("/Applications/Docker.app/Contents/Resources/bin/docker")
        .args(["--version"])
        .output();

    // Also try the standard docker path in case symlinks are already set up
    let verify = match verify {
        Ok(ref out) if out.status.success() => verify,
        _ => Command::new("docker").args(["--version"]).output(),
    };

    match verify {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Ok(version)
        }
        Ok(_) => {
            // Installed but binary not on PATH yet — Docker Desktop is still starting up
            Ok("Docker Desktop installed — starting up...".to_string())
        }
        Err(_) => {
            Ok("Docker Desktop installed — please wait for it to finish starting.".to_string())
        }
    }
}

/// Pull the Docker image with progress.
pub async fn pull_image(image: &str) -> Result<(), String> {
    let output = Command::new("docker")
        .args(["pull", image])
        .output()
        .map_err(|e| format!("Failed to pull image: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Image pull failed: {}", stderr))
    }
}

fn dirs_next() -> Option<String> {
    std::env::var("HOME").ok()
}
