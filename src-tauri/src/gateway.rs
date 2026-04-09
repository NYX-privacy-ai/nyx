use crate::ironclaw;
use rusqlite::Connection;
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

/// A message in a conversation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Local sessions DB — Nyx manages conversation history client-side
// (IronClaw gateway is stateless, standard OpenAI pattern)
// ---------------------------------------------------------------------------

fn sessions_db_path() -> PathBuf {
    ironclaw::config_dir().join("sessions.db")
}

fn open_sessions_db() -> Result<Connection, String> {
    let path = sessions_db_path();
    let conn = Connection::open(&path)
        .map_err(|e| format!("Failed to open sessions DB: {}", e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL DEFAULT 'New Chat',
            folder TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
        CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);",
    )
    .map_err(|e| format!("Failed to init sessions DB: {}", e))?;
    Ok(conn)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read gateway token from Nyx's .env file.
fn read_gateway_token() -> Result<String, String> {
    let env_path = ironclaw::config_dir().join(".env");

    let content = fs::read_to_string(&env_path)
        .map_err(|e| format!("Failed to read .env: {}", e))?;

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("GATEWAY_AUTH_TOKEN=") {
            return Ok(val.to_string());
        }
    }

    Err("GATEWAY_AUTH_TOKEN not found in .env".to_string())
}

/// Extract the assistant reply from an OpenAI chat completion response.
fn extract_openai_reply(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(content) = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
        {
            return content.to_string();
        }
    }
    text.to_string()
}

fn folders_path() -> PathBuf {
    ironclaw::config_dir().join("chat_folders.json")
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
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(folders)
        .map_err(|e| format!("Failed to serialize folders: {}", e))?;
    fs::write(&path, content)
        .map_err(|e| format!("Failed to write folders: {}", e))
}

// ---------------------------------------------------------------------------
// Chat API — IronClaw gateway (stateless, OpenAI-compatible)
// ---------------------------------------------------------------------------

/// Send a message to the default session.
pub async fn send_message(message: String) -> Result<String, String> {
    send_message_to_session(message, "main".to_string()).await
}

/// Send a message to a specific session. Includes conversation history
/// in the request (IronClaw gateway is stateless).
pub async fn send_message_to_session(message: String, session_key: String) -> Result<String, String> {
    let token = read_gateway_token()?;
    let port = ironclaw::gateway_port();
    let url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

    // --- Sync DB block: load history + save user message ---
    // (rusqlite::Connection is not Send, so all DB work must happen
    // before the async HTTP call)
    let body = {
        let db = open_sessions_db()?;

        // Ensure session exists
        db.execute(
            "INSERT OR IGNORE INTO sessions (id, title) VALUES (?1, ?1)",
            rusqlite::params![&session_key],
        ).map_err(|e| format!("DB error: {}", e))?;

        // Load history — cap at 40 messages (20 pairs) to prevent context poisoning
        let mut stmt = db.prepare(
            "SELECT role, content FROM (
                SELECT role, content, created_at FROM messages WHERE session_id = ?1
                ORDER BY created_at DESC LIMIT 40
            ) ORDER BY created_at ASC"
        ).map_err(|e| format!("DB error: {}", e))?;

        let history: Vec<serde_json::Value> = stmt
            .query_map(rusqlite::params![&session_key], |row| {
                let role: String = row.get(0)?;
                let content: String = row.get(1)?;
                Ok(serde_json::json!({"role": role, "content": content}))
            })
            .map_err(|e| format!("DB error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        // Build messages array: history + new message
        let mut messages = history;
        messages.push(serde_json::json!({"role": "user", "content": &message}));

        // Save the user message
        db.execute(
            "INSERT INTO messages (session_id, role, content) VALUES (?1, 'user', ?2)",
            rusqlite::params![&session_key, &message],
        ).map_err(|e| format!("DB error: {}", e))?;

        serde_json::json!({
            "model": "default",
            "messages": messages
        })
        // db is dropped here — Connection no longer held across await
    };

    // --- Async HTTP call ---
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
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
        let reply = extract_openai_reply(&text);

        // --- Sync DB block: save assistant reply ---
        if let Ok(db) = open_sessions_db() {
            let _ = db.execute(
                "INSERT INTO messages (session_id, role, content) VALUES (?1, 'assistant', ?2)",
                rusqlite::params![&session_key, &reply],
            );
            let _ = db.execute(
                "UPDATE sessions SET updated_at = strftime('%s','now') WHERE id = ?1",
                rusqlite::params![&session_key],
            );
        }

        Ok(reply)
    } else {
        Err(format!("Gateway error ({}): {}", status, text))
    }
}

// ---------------------------------------------------------------------------
// Session management — local DB
// ---------------------------------------------------------------------------

/// List all chat sessions.
pub fn list_sessions() -> Result<Vec<SessionInfo>, String> {
    let db = open_sessions_db()?;
    let folders_data = load_folders();

    let mut stmt = db.prepare(
        "SELECT s.id, s.title, s.updated_at,
                (SELECT COUNT(*) FROM messages WHERE session_id = s.id AND role = 'user') as msg_count
         FROM sessions s
         ORDER BY s.updated_at DESC"
    ).map_err(|e| format!("DB error: {}", e))?;

    let sessions: Vec<SessionInfo> = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let title: String = row.get(1)?;
            let updated_at: Option<u64> = row.get(2)?;
            let _msg_count: i64 = row.get(3)?;
            Ok((id, title, updated_at))
        })
        .map_err(|e| format!("DB error: {}", e))?
        .filter_map(|r| r.ok())
        .map(|(id, title, updated_at)| {
            SessionInfo {
                session_key: id.clone(),
                session_id: Some(id.clone()),
                updated_at,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                model: None,
                title: folders_data.session_titles.get(&id).cloned()
                    .or(Some(title)),
                folder: folders_data.session_folders.get(&id).cloned(),
            }
        })
        .collect();

    Ok(sessions)
}

/// Create a new chat session.
pub fn create_session(title: Option<String>, folder: Option<String>) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let session_key = format!("chat_{}", id);
    let display_title = title.clone().unwrap_or_else(|| "New Chat".to_string());

    let db = open_sessions_db()?;
    db.execute(
        "INSERT INTO sessions (id, title) VALUES (?1, ?2)",
        rusqlite::params![&session_key, &display_title],
    ).map_err(|e| format!("DB error: {}", e))?;

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
    let db = open_sessions_db()?;
    db.execute(
        "UPDATE sessions SET title = ?2 WHERE id = ?1",
        rusqlite::params![&session_key, &title],
    ).map_err(|e| format!("DB error: {}", e))?;

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
// Verify source
// ---------------------------------------------------------------------------

/// Verify a source URL for credibility via the Veritas analysis prompt.
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

    // Veritas analysis runs as a standalone request (no session history needed)
    let token = read_gateway_token()?;
    let port = ironclaw::gateway_port();
    let api_url = format!("http://127.0.0.1:{}/v1/chat/completions", port);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let body = serde_json::json!({
        "model": "default",
        "messages": [{"role": "user", "content": prompt}]
    });

    let response = client
        .post(&api_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
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
// Session history — from local DB
// ---------------------------------------------------------------------------

/// Load message history for a session.
pub fn load_session_history(
    session_key: &str,
    limit: usize,
) -> Result<Vec<HistoryMessage>, String> {
    let db = open_sessions_db()?;

    let mut stmt = db.prepare(
        "SELECT role, content FROM messages
         WHERE session_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2"
    ).map_err(|e| format!("DB error: {}", e))?;

    let mut messages: Vec<HistoryMessage> = stmt
        .query_map(rusqlite::params![session_key, limit as i64], |row| {
            Ok(HistoryMessage {
                role: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .map_err(|e| format!("DB error: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // Reverse since we selected DESC order
    messages.reverse();
    Ok(messages)
}
