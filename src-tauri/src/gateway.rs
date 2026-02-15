use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_key: String,
    pub session_id: Option<String>,
    pub updated_at: Option<u64>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub model: Option<String>,
    pub title: Option<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatFolders {
    pub folders: Vec<ChatFolder>,
    pub session_folders: HashMap<String, String>,
    pub session_titles: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatFolder {
    pub id: String,
    pub name: String,
    pub order: u32,
}

impl Default for ChatFolders {
    fn default() -> Self {
        Self {
            folders: vec![
                ChatFolder { id: "general".into(), name: "General".into(), order: 0 },
                ChatFolder { id: "work".into(), name: "Work".into(), order: 1 },
                ChatFolder { id: "research".into(), name: "Research".into(), order: 2 },
            ],
            session_folders: HashMap::new(),
            session_titles: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read gateway token from docker.env.
fn read_gateway_token() -> Result<String, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let env_path = PathBuf::from(&home).join("openclaw/docker.env");

    let content = fs::read_to_string(&env_path)
        .map_err(|e| format!("Failed to read docker.env: {}", e))?;

    for line in content.lines() {
        if line.starts_with("OPENCLAW_GATEWAY_TOKEN=") {
            return Ok(line.trim_start_matches("OPENCLAW_GATEWAY_TOKEN=").to_string());
        }
    }

    Err("Gateway token not found in docker.env".to_string())
}

/// Extract the assistant reply from an OpenAI chat completion response.
fn extract_openai_reply(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        // OpenAI format: choices[0].message.content
        if let Some(content) = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
        {
            return content.to_string();
        }
    }
    // Fallback: return raw text
    text.to_string()
}

fn folders_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(&home).join(".openclaw/agents/default/chat_folders.json")
}

fn load_folders() -> ChatFolders {
    let path = folders_path();
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        ChatFolders::default()
    }
}

fn save_folders(folders: &ChatFolders) -> Result<(), String> {
    let path = folders_path();
    let content = serde_json::to_string_pretty(folders)
        .map_err(|e| format!("Failed to serialize folders: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write folders: {}", e))
}

// ---------------------------------------------------------------------------
// Chat API
// ---------------------------------------------------------------------------

/// Send a message to the OpenClaw gateway via HTTP API.
pub async fn send_message(message: String) -> Result<String, String> {
    send_message_to_session(message, "agent:default:main".to_string()).await
}

/// Send a message to a specific session via the gateway's OpenAI-compatible endpoint.
pub async fn send_message_to_session(message: String, session_key: String) -> Result<String, String> {
    let token = read_gateway_token()?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let url = "http://127.0.0.1:18789/v1/chat/completions";

    let body = serde_json::json!({
        "model": "default",
        "messages": [
            { "role": "user", "content": message }
        ]
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .header("X-OpenClaw-Session-Key", &session_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gateway request failed: {}", e))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if status.is_success() {
        Ok(extract_openai_reply(&text))
    } else {
        Err(format!("Gateway error ({}): {}", status, text))
    }
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

/// List all chat sessions from sessions.json, enriched with folder metadata.
pub fn list_sessions() -> Result<Vec<SessionInfo>, String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let sessions_path = PathBuf::from(&home)
        .join(".openclaw/agents/default/sessions/sessions.json");

    let content = fs::read_to_string(&sessions_path)
        .map_err(|e| format!("Failed to read sessions.json: {}", e))?;

    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse sessions.json: {}", e))?;

    let folders_data = load_folders();

    let mut sessions: Vec<SessionInfo> = raw
        .iter()
        .filter(|(key, _)| {
            // Only include user chat sessions, skip internal ones
            key.starts_with("agent:default:")
                && !key.contains("veritas")
                && !key.contains("cron")
        })
        .map(|(key, val)| {
            let short_key = key.strip_prefix("agent:default:").unwrap_or(key);
            SessionInfo {
                session_key: key.clone(),
                session_id: val.get("sessionId").and_then(|v| v.as_str()).map(String::from),
                updated_at: val.get("updatedAt").and_then(|v| v.as_u64()),
                input_tokens: val.get("inputTokens").and_then(|v| v.as_u64()),
                output_tokens: val.get("outputTokens").and_then(|v| v.as_u64()),
                total_tokens: val.get("totalTokens").and_then(|v| v.as_u64()),
                model: val.get("model").and_then(|v| v.as_str()).map(String::from),
                title: folders_data.session_titles.get(key).cloned()
                    .or_else(|| Some(short_key.to_string())),
                folder: folders_data.session_folders.get(key).cloned(),
            }
        })
        .collect();

    // Sort by updatedAt descending (most recent first)
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(sessions)
}

/// Create a new session key for a conversation.
pub fn create_session(title: Option<String>, folder: Option<String>) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let session_key = format!("agent:default:chat_{}", id);

    let mut folders_data = load_folders();
    if let Some(t) = title {
        folders_data.session_titles.insert(session_key.clone(), t);
    }
    if let Some(f) = folder {
        folders_data.session_folders.insert(session_key.clone(), f);
    }
    save_folders(&folders_data)?;

    Ok(session_key)
}

/// Update session title.
pub fn rename_session(session_key: String, title: String) -> Result<(), String> {
    let mut folders_data = load_folders();
    folders_data.session_titles.insert(session_key, title);
    save_folders(&folders_data)
}

/// Move a session to a different folder.
pub fn move_session_to_folder(session_key: String, folder_id: Option<String>) -> Result<(), String> {
    let mut folders_data = load_folders();
    match folder_id {
        Some(f) => { folders_data.session_folders.insert(session_key, f); }
        None => { folders_data.session_folders.remove(&session_key); }
    }
    save_folders(&folders_data)
}

/// Get chat folder configuration.
pub fn get_chat_folders() -> Result<ChatFolders, String> {
    Ok(load_folders())
}

/// Create a new folder.
pub fn create_folder(name: String) -> Result<ChatFolder, String> {
    let mut data = load_folders();
    let id = name.to_lowercase().replace(' ', "_");
    let order = data.folders.len() as u32;
    let folder = ChatFolder { id: id.clone(), name, order };
    data.folders.push(folder.clone());
    save_folders(&data)?;
    Ok(folder)
}

/// Rename a folder.
pub fn rename_folder(folder_id: String, name: String) -> Result<(), String> {
    let mut data = load_folders();
    if let Some(f) = data.folders.iter_mut().find(|f| f.id == folder_id) {
        f.name = name;
    }
    save_folders(&data)
}

/// Delete a folder (moves sessions to unfiled).
pub fn delete_folder(folder_id: String) -> Result<(), String> {
    let mut data = load_folders();
    data.folders.retain(|f| f.id != folder_id);
    data.session_folders.retain(|_, v| v != &folder_id);
    save_folders(&data)
}

// ---------------------------------------------------------------------------
// Verify source (unchanged)
// ---------------------------------------------------------------------------

/// Verify a source URL for credibility via the Veritas analysis prompt.
/// Uses a dedicated session key so analysis doesn't pollute chat history.
pub async fn verify_source(url: String) -> Result<String, String> {
    let prompt = format!(
        r#"Analyze the credibility of this source: {}

Fetch the content at the URL and evaluate it across these 6 dimensions (score 0-100 each):

1. SOURCE_REPUTATION: Domain authority, publication history, editorial standards
2. AUTHOR_CREDIBILITY: Author track record, expertise, transparency
3. CORROBORATION: Are claims confirmed by multiple independent sources?
4. EVIDENCE_QUALITY: Primary sources cited, data referenced, methodology
5. CONSISTENCY: Aligns with established facts, no internal contradictions
6. PRESENTATION: Objective tone, no clickbait, balanced perspective

Weights for overall score: Source Reputation 20%, Author Credibility 15%, Corroboration 25%, Evidence Quality 20%, Consistency 10%, Presentation 10%.

Return ONLY a JSON object with no markdown fences, no extra text — raw JSON only:
{{
  "url": "<the url>",
  "title": "<article title>",
  "author": "<author name or null>",
  "domain": "<domain name>",
  "published_date": "<ISO date or null>",
  "scores": {{
    "source_reputation": <0-100>,
    "author_credibility": <0-100>,
    "corroboration": <0-100>,
    "evidence_quality": <0-100>,
    "consistency": <0-100>,
    "presentation": <0-100>
  }},
  "overall_score": <weighted 0-100>,
  "grade": "<A|B|C|D|F>",
  "claims": [
    {{ "claim": "<key claim text>", "status": "verified|unverified|disputed|misleading" }}
  ],
  "summary": "<2-3 sentence credibility assessment>",
  "limitations": "<any caveats about this analysis>"
}}"#,
        url
    );

    // Use the shared chat completions function with a dedicated Veritas session
    send_message_to_session(prompt, "agent:default:veritas".to_string()).await
}
