use serde::{Deserialize, Serialize};
use std::process::Command;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OllamaStatus {
    pub available: bool,
    pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub parameter_size: String,
    pub quantization: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Health Check
// ---------------------------------------------------------------------------

/// Check if Ollama is running on localhost:11434.
pub async fn check_ollama() -> Result<OllamaStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    match client.get(OLLAMA_BASE_URL).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            let available = text.contains("Ollama");
            // Try to get version from /api/version
            let version = match client
                .get(format!("{}/api/version", OLLAMA_BASE_URL))
                .send()
                .await
            {
                Ok(v) => {
                    let json: serde_json::Value =
                        serde_json::from_str(&v.text().await.unwrap_or_default())
                            .unwrap_or_default();
                    json.get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }
                Err(_) => None,
            };
            Ok(OllamaStatus { available, version })
        }
        Err(_) => Ok(OllamaStatus {
            available: false,
            version: None,
        }),
    }
}

// ---------------------------------------------------------------------------
// Installation
// ---------------------------------------------------------------------------

/// Download and install Ollama from the official macOS zip.
pub async fn install_ollama() -> Result<String, String> {
    let url = "https://ollama.com/download/Ollama-darwin.zip";
    let tmp_zip = "/tmp/Ollama-darwin.zip";
    let tmp_unzip_dir = "/tmp/Ollama-unzipped";

    // 1. Download
    let output = Command::new("curl")
        .args(["-fSL", "--progress-bar", "-o", tmp_zip, url])
        .output()
        .map_err(|e| format!("Failed to download Ollama: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Download failed: {}", stderr));
    }

    // 2. Unzip — clean up any previous extraction first
    let _ = std::fs::remove_dir_all(tmp_unzip_dir);
    let output = Command::new("unzip")
        .args(["-o", "-q", tmp_zip, "-d", tmp_unzip_dir])
        .output()
        .map_err(|e| format!("Failed to unzip Ollama: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(tmp_zip);
        let _ = std::fs::remove_dir_all(tmp_unzip_dir);
        return Err(format!("Unzip failed: {}", stderr));
    }

    // 3. Move Ollama.app to /Applications (overwrite if exists)
    let _ = std::fs::remove_dir_all("/Applications/Ollama.app");
    let output = Command::new("cp")
        .args([
            "-R",
            &format!("{}/Ollama.app", tmp_unzip_dir),
            "/Applications/Ollama.app",
        ])
        .output()
        .map_err(|e| format!("Failed to copy Ollama.app: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(tmp_zip);
        let _ = std::fs::remove_dir_all(tmp_unzip_dir);
        return Err(format!("Install failed: {}", stderr));
    }

    // 4. Clean up temp files
    let _ = std::fs::remove_file(tmp_zip);
    let _ = std::fs::remove_dir_all(tmp_unzip_dir);

    // 5. Launch Ollama.app (starts the HTTP server on :11434)
    let _ = Command::new("open")
        .args(["/Applications/Ollama.app"])
        .output();

    // 6. Brief wait then verify via HTTP
    std::thread::sleep(std::time::Duration::from_secs(3));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    match client.get(OLLAMA_BASE_URL).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.contains("Ollama") {
                Ok("Ollama installed and running".to_string())
            } else {
                Ok("Ollama installed — starting up...".to_string())
            }
        }
        Err(_) => Ok("Ollama installed — please wait for it to finish starting.".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Model Management
// ---------------------------------------------------------------------------

/// List locally installed Ollama models.
pub async fn list_models() -> Result<Vec<OllamaModel>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .await
        .map_err(|e| format!("Failed to list models: {}", e))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse model list: {}", e))?;

    let models = json
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .map(|m| {
                    let details = m.get("details").unwrap_or(&serde_json::Value::Null);
                    OllamaModel {
                        name: m
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        size: m.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                        parameter_size: details
                            .get("parameter_size")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        quantization: details
                            .get("quantization_level")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(models)
}

/// Pull (download) a model from the Ollama library.
/// This blocks until the download is complete — models can be 2-8GB.
pub async fn pull_model(model: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1800)) // 30 min max
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let body = serde_json::json!({
        "name": model,
        "stream": false
    });

    let resp = client
        .post(format!("{}/api/pull", OLLAMA_BASE_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to pull model: {}", e))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read pull response: {}", e))?;

    if status.is_success() {
        Ok("ok".to_string())
    } else {
        Err(format!("Pull failed ({}): {}", status, text))
    }
}

/// Delete a locally installed model.
pub async fn delete_model(model: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let body = serde_json::json!({ "name": model });

    let resp = client
        .delete(format!("{}/api/delete", OLLAMA_BASE_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to delete model: {}", e))?;

    if resp.status().is_success() {
        Ok("ok".to_string())
    } else {
        let text = resp.text().await.unwrap_or_default();
        Err(format!("Delete failed: {}", text))
    }
}

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

/// Send a chat message to a local Ollama model.
/// Ollama doesn't maintain sessions — the full message history must be provided.
pub async fn chat_ollama(
    model: String,
    message: String,
    history: Vec<ChatMessage>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // 5 min max for generation
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // Build messages array: history + the new user message
    let mut messages: Vec<serde_json::Value> = history
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content
            })
        })
        .collect();

    messages.push(serde_json::json!({
        "role": "user",
        "content": message
    }));

    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let resp = client
        .post(format!("{}/api/chat", OLLAMA_BASE_URL))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama chat failed: {}", e))?;

    let status = resp.status();
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    if status.is_success() {
        let content = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("No response from model")
            .to_string();
        Ok(content)
    } else {
        let error = json
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error")
            .to_string();
        Err(format!("Ollama error: {}", error))
    }
}

// ---------------------------------------------------------------------------
// System Info
// ---------------------------------------------------------------------------

/// Get total system RAM in GB (macOS via sysctl).
pub async fn get_system_ram() -> Result<u64, String> {
    let output = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .map_err(|e| format!("Failed to run sysctl: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let bytes: u64 = stdout
        .parse()
        .map_err(|e| format!("Failed to parse RAM size: {}", e))?;

    Ok(bytes / (1024 * 1024 * 1024)) // Convert bytes to GB
}
