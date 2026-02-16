// ---------------------------------------------------------------------------
// Activity Intelligence — Observe → Learn → Suggest → Act
// ---------------------------------------------------------------------------
// Local SQLite-backed intelligence engine that observes calendar events and
// email metadata (never bodies) to build a behavioural model of the user's
// communication patterns. All data stays on-device in ~/.nyx/intelligence.db.
// ---------------------------------------------------------------------------

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContactSummary {
    pub email: String,
    pub name: Option<String>,
    pub interaction_count: i64,
    pub last_seen: String,
    pub preferred_channel: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    pub id: i64,
    #[serde(rename = "type")]
    pub suggestion_type: String,
    pub title: String,
    pub description: String,
    pub contact_email: Option<String>,
    pub confidence: f64,
    pub context: Option<String>,
    pub status: String,
    pub created_at: String,
    pub acted_at: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContactInsight {
    pub email: String,
    pub name: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub interaction_count: i64,
    pub avg_response_time_mins: Option<f64>,
    pub preferred_channel: Option<String>,
    pub tags: Vec<String>,
    pub recent_emails: i64,
    pub recent_meetings: i64,
    pub unanswered_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivityStats {
    pub contacts_tracked: u32,
    pub emails_observed_24h: u32,
    pub calendar_events_7d: u32,
    pub suggestions_pending: u32,
    pub top_contacts: Vec<ContactSummary>,
    pub last_observation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AutonomySetting {
    pub activity_type: String,
    pub level: String,         // observe | suggest | draft | act
    pub promoted_at: Option<String>,
    pub total_accepted: i64,
    pub total_dismissed: i64,
}

// ---------------------------------------------------------------------------
// gog JSON output structures (Google Calendar API + Gmail API pass-through)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GogCalendarEvent {
    id: Option<String>,
    summary: Option<String>,
    start: Option<GogDateTime>,
    end: Option<GogDateTime>,
    attendees: Option<Vec<GogAttendee>>,
    location: Option<String>,
    recurrence: Option<Vec<String>>,
    organizer: Option<GogEmailRef>,
}

#[derive(Debug, Deserialize)]
struct GogDateTime {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GogAttendee {
    email: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GogEmailRef {
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GogCalendarResponse {
    items: Option<Vec<GogCalendarEvent>>,
}

#[derive(Debug, Deserialize)]
struct GogGmailThread {
    id: Option<String>,
    messages: Option<Vec<GogGmailMessage>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GogGmailMessage {
    id: Option<String>,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    #[serde(rename = "labelIds")]
    label_ids: Option<Vec<String>>,
    payload: Option<GogMessagePayload>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GogMessagePayload {
    headers: Option<Vec<GogHeader>>,
}

#[derive(Debug, Deserialize)]
struct GogHeader {
    name: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GogGmailSearchResponse {
    threads: Option<Vec<GogGmailThread>>,
}

// ---------------------------------------------------------------------------
// Database path + connection
// ---------------------------------------------------------------------------

fn db_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".nyx").join("intelligence.db")
}

fn open_db() -> Result<Connection, String> {
    let path = db_path();
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create ~/.nyx/ directory: {}", e))?;
    }
    Connection::open(&path).map_err(|e| format!("Failed to open intelligence.db: {}", e))
}

// ---------------------------------------------------------------------------
// Schema initialisation + migrations
// ---------------------------------------------------------------------------

pub fn init_db() -> Result<(), String> {
    let conn = open_db()?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS contacts (
            id INTEGER PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            name TEXT,
            first_seen TEXT NOT NULL,
            last_seen TEXT NOT NULL,
            interaction_count INTEGER DEFAULT 0,
            avg_response_time_mins REAL,
            preferred_channel TEXT,
            tags TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS calendar_events (
            id INTEGER PRIMARY KEY,
            event_id TEXT UNIQUE NOT NULL,
            summary TEXT,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            attendees TEXT,
            location TEXT,
            is_recurring INTEGER DEFAULT 0,
            organizer_email TEXT,
            observed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS email_observations (
            id INTEGER PRIMARY KEY,
            thread_id TEXT NOT NULL,
            message_id TEXT UNIQUE,
            from_email TEXT NOT NULL,
            to_emails TEXT,
            subject TEXT,
            timestamp TEXT NOT NULL,
            is_inbound INTEGER,
            replied INTEGER DEFAULT 0,
            reply_time_mins REAL,
            labels TEXT,
            observed_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS suggestions (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            contact_email TEXT,
            confidence REAL,
            context TEXT,
            status TEXT DEFAULT 'pending',
            created_at TEXT NOT NULL,
            acted_at TEXT,
            expires_at TEXT
        );

        CREATE TABLE IF NOT EXISTS autonomy_settings (
            activity_type TEXT PRIMARY KEY,
            level TEXT DEFAULT 'suggest',
            promoted_at TEXT,
            total_accepted INTEGER DEFAULT 0,
            total_dismissed INTEGER DEFAULT 0
        );

        -- Seed default autonomy settings if empty
        INSERT OR IGNORE INTO autonomy_settings (activity_type, level)
        VALUES
            ('scheduling', 'suggest'),
            ('email_reply', 'observe'),
            ('follow_up', 'suggest'),
            ('outreach', 'observe');

        -- Indices for common queries
        CREATE INDEX IF NOT EXISTS idx_contacts_last_seen ON contacts(last_seen);
        CREATE INDEX IF NOT EXISTS idx_contacts_interaction ON contacts(interaction_count DESC);
        CREATE INDEX IF NOT EXISTS idx_email_timestamp ON email_observations(timestamp);
        CREATE INDEX IF NOT EXISTS idx_email_from ON email_observations(from_email);
        CREATE INDEX IF NOT EXISTS idx_email_replied ON email_observations(is_inbound, replied);
        CREATE INDEX IF NOT EXISTS idx_calendar_start ON calendar_events(start_time);
        CREATE INDEX IF NOT EXISTS idx_suggestions_status ON suggestions(status);
        ",
    )
    .map_err(|e| format!("Failed to initialise intelligence schema: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// gog CLI helper
// ---------------------------------------------------------------------------

fn gog_binary_path() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let local_path = format!("{}/openclaw/bin/gog", home);
    if std::path::Path::new(&local_path).exists() {
        local_path
    } else {
        "gog".to_string()
    }
}

fn now_iso() -> String {
    chrono_now()
}

/// Simple ISO 8601 timestamp using std (avoids chrono dependency).
fn chrono_now() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();

    // Convert epoch seconds to ISO 8601 (UTC)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since 1970-01-01 → year/month/day (simplified calendar)
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

fn days_to_ymd(mut total_days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;

    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if total_days < days_in_year {
            break;
        }
        total_days -= days_in_year;
        year += 1;
    }

    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &md in &month_days {
        if total_days < md {
            break;
        }
        total_days -= md;
        month += 1;
    }

    (year, month, total_days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Returns ISO date string for N days ago (YYYY-MM-DD).
fn days_ago(n: u64) -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let target = secs.saturating_sub(n * 86400);
    let (y, m, d) = days_to_ymd(target / 86400);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Returns ISO date string for N days ahead (YYYY-MM-DD).
fn days_ahead(n: u64) -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + n * 86400;
    let (y, m, d) = days_to_ymd(secs / 86400);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

// ---------------------------------------------------------------------------
// Calendar observation
// ---------------------------------------------------------------------------

pub fn observe_calendar() -> Result<u32, String> {
    let gog = gog_binary_path();
    let from = days_ago(7);
    let to = days_ahead(14);

    let output = Command::new(&gog)
        .args([
            "calendar", "events", "primary",
            "--from", &from,
            "--to", &to,
            "--max", "200",
            "--json",
            "--no-input",
        ])
        .output()
        .map_err(|e| format!("Failed to run gog calendar: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gog calendar failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // gog --json may return either a wrapper object with `items` or a direct array
    let events: Vec<GogCalendarEvent> = if let Ok(resp) =
        serde_json::from_str::<GogCalendarResponse>(&stdout)
    {
        resp.items.unwrap_or_default()
    } else if let Ok(arr) = serde_json::from_str::<Vec<GogCalendarEvent>>(&stdout) {
        arr
    } else {
        return Err("Failed to parse calendar JSON".to_string());
    };

    let conn = open_db()?;
    let now = now_iso();
    let mut count = 0u32;

    for event in &events {
        let event_id = match &event.id {
            Some(id) => id,
            None => continue,
        };

        let start = event
            .start
            .as_ref()
            .and_then(|s| s.date_time.as_ref().or(s.date.as_ref()))
            .cloned()
            .unwrap_or_default();

        let end = event
            .end
            .as_ref()
            .and_then(|e| e.date_time.as_ref().or(e.date.as_ref()))
            .cloned()
            .unwrap_or_default();

        let attendees_json = event
            .attendees
            .as_ref()
            .map(|a| {
                let emails: Vec<&str> = a
                    .iter()
                    .filter_map(|att| att.email.as_deref())
                    .collect();
                serde_json::to_string(&emails).unwrap_or_else(|_| "[]".to_string())
            })
            .unwrap_or_else(|| "[]".to_string());

        let is_recurring = event.recurrence.as_ref().map_or(0, |r| {
            if r.is_empty() { 0 } else { 1 }
        });

        let organizer_email = event
            .organizer
            .as_ref()
            .and_then(|o| o.email.as_deref());

        // Upsert calendar event
        conn.execute(
            "INSERT INTO calendar_events (event_id, summary, start_time, end_time, attendees, location, is_recurring, organizer_email, observed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(event_id) DO UPDATE SET
                summary = excluded.summary,
                start_time = excluded.start_time,
                end_time = excluded.end_time,
                attendees = excluded.attendees,
                location = excluded.location,
                is_recurring = excluded.is_recurring,
                organizer_email = excluded.organizer_email,
                observed_at = excluded.observed_at",
            params![
                event_id,
                event.summary,
                start,
                end,
                attendees_json,
                event.location,
                is_recurring,
                organizer_email,
                now,
            ],
        )
        .map_err(|e| format!("Failed to upsert calendar event: {}", e))?;

        count += 1;

        // Extract attendee contacts
        if let Some(attendees) = &event.attendees {
            for att in attendees {
                if let Some(email) = &att.email {
                    upsert_contact(&conn, email, att.display_name.as_deref(), "calendar", &now)?;
                }
            }
        }

        // Extract organizer contact
        if let Some(email) = organizer_email {
            upsert_contact(&conn, email, None, "calendar", &now)?;
        }
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Email observation
// ---------------------------------------------------------------------------

pub fn observe_email() -> Result<u32, String> {
    let gog = gog_binary_path();

    let output = Command::new(&gog)
        .args([
            "gmail", "search",
            "newer_than:24h",
            "--max", "100",
            "--json",
            "--no-input",
        ])
        .output()
        .map_err(|e| format!("Failed to run gog gmail: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gog gmail failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // gog --json may return a wrapper object or direct array of threads
    let threads: Vec<GogGmailThread> = if let Ok(resp) =
        serde_json::from_str::<GogGmailSearchResponse>(&stdout)
    {
        resp.threads.unwrap_or_default()
    } else if let Ok(arr) = serde_json::from_str::<Vec<GogGmailThread>>(&stdout) {
        arr
    } else {
        // Might be empty result
        Vec::new()
    };

    let conn = open_db()?;
    let now = now_iso();
    let mut count = 0u32;

    // Try to determine the user's own email for is_inbound classification
    let user_email = detect_user_email(&conn);

    for thread in &threads {
        let thread_id = match &thread.id {
            Some(id) => id,
            None => continue,
        };

        if let Some(messages) = &thread.messages {
            for msg in messages {
                let message_id = match &msg.id {
                    Some(id) => id,
                    None => continue,
                };

                let headers = msg
                    .payload
                    .as_ref()
                    .and_then(|p| p.headers.as_ref());

                let from_email = headers
                    .and_then(|h| find_header(h, "From"))
                    .map(|s| extract_email_from_header(&s))
                    .unwrap_or_default();

                let to_raw = headers
                    .and_then(|h| find_header(h, "To"))
                    .unwrap_or_default();
                let to_emails: Vec<String> = to_raw
                    .split(',')
                    .map(|s| extract_email_from_header(s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect();

                let subject = headers.and_then(|h| find_header(h, "Subject"));

                // Parse internal date (milliseconds since epoch)
                let timestamp = msg
                    .internal_date
                    .as_ref()
                    .and_then(|d| d.parse::<u64>().ok())
                    .map(|ms| {
                        let secs = ms / 1000;
                        let (y, m, d) = days_to_ymd(secs / 86400);
                        let tod = secs % 86400;
                        format!(
                            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                            y, m, d, tod / 3600, (tod % 3600) / 60, tod % 60
                        )
                    })
                    .unwrap_or_else(|| now.clone());

                let is_inbound = if from_email.is_empty() {
                    None
                } else if let Some(ref user) = user_email {
                    Some(if from_email.to_lowercase() != user.to_lowercase() {
                        1
                    } else {
                        0
                    })
                } else {
                    None
                };

                let labels_json = msg
                    .label_ids
                    .as_ref()
                    .map(|l| serde_json::to_string(l).unwrap_or_else(|_| "[]".to_string()));

                let to_json = serde_json::to_string(&to_emails)
                    .unwrap_or_else(|_| "[]".to_string());

                // Insert email observation (skip if already seen)
                let result = conn.execute(
                    "INSERT OR IGNORE INTO email_observations
                     (thread_id, message_id, from_email, to_emails, subject, timestamp, is_inbound, labels, observed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        thread_id,
                        message_id,
                        from_email,
                        to_json,
                        subject,
                        timestamp,
                        is_inbound,
                        labels_json,
                        now,
                    ],
                );

                if let Ok(rows) = result {
                    if rows > 0 {
                        count += 1;
                    }
                }

                // Upsert contacts from email participants
                if !from_email.is_empty() {
                    upsert_contact(&conn, &from_email, None, "email", &now)?;
                }
                for to in &to_emails {
                    if !to.is_empty() {
                        upsert_contact(&conn, to, None, "email", &now)?;
                    }
                }
            }
        }
    }

    // After processing all messages, detect reply patterns
    detect_reply_patterns(&conn)?;

    Ok(count)
}

// ---------------------------------------------------------------------------
// Contact upsert
// ---------------------------------------------------------------------------

fn upsert_contact(
    conn: &Connection,
    email: &str,
    name: Option<&str>,
    channel: &str,
    now: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO contacts (email, name, first_seen, last_seen, interaction_count, preferred_channel, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3, 1, ?4, ?3, ?3)
         ON CONFLICT(email) DO UPDATE SET
            name = COALESCE(excluded.name, contacts.name),
            last_seen = excluded.last_seen,
            interaction_count = contacts.interaction_count + 1,
            preferred_channel = CASE
                WHEN contacts.interaction_count > 5 THEN contacts.preferred_channel
                ELSE excluded.preferred_channel
            END,
            updated_at = excluded.updated_at",
        params![email, name, now, channel],
    )
    .map_err(|e| format!("Failed to upsert contact: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Reply pattern detection
// ---------------------------------------------------------------------------

fn detect_reply_patterns(conn: &Connection) -> Result<(), String> {
    // Find inbound emails that haven't been checked for replies yet
    // A reply exists if the same thread has a subsequent outbound message
    conn.execute_batch(
        "UPDATE email_observations SET replied = 1
         WHERE is_inbound = 1 AND replied = 0
         AND EXISTS (
             SELECT 1 FROM email_observations e2
             WHERE e2.thread_id = email_observations.thread_id
             AND e2.is_inbound = 0
             AND e2.timestamp > email_observations.timestamp
         );

         -- Calculate reply time for newly detected replies
         UPDATE email_observations SET reply_time_mins = (
             SELECT MIN(
                 (JULIANDAY(e2.timestamp) - JULIANDAY(email_observations.timestamp)) * 1440
             )
             FROM email_observations e2
             WHERE e2.thread_id = email_observations.thread_id
             AND e2.is_inbound = 0
             AND e2.timestamp > email_observations.timestamp
         )
         WHERE is_inbound = 1 AND replied = 1 AND reply_time_mins IS NULL;

         -- Update contact avg response times
         UPDATE contacts SET avg_response_time_mins = (
             SELECT AVG(reply_time_mins)
             FROM email_observations
             WHERE from_email = contacts.email
             AND is_inbound = 1
             AND replied = 1
             AND reply_time_mins IS NOT NULL
             AND reply_time_mins > 0
         )
         WHERE email IN (
             SELECT DISTINCT from_email FROM email_observations
             WHERE is_inbound = 1 AND replied = 1 AND reply_time_mins IS NOT NULL
         );",
    )
    .map_err(|e| format!("Failed to detect reply patterns: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// User email detection (for inbound/outbound classification)
// ---------------------------------------------------------------------------

fn detect_user_email(conn: &Connection) -> Option<String> {
    // First, check if we have a gog account
    let gog = gog_binary_path();
    if let Ok(output) = Command::new(&gog)
        .args(["auth", "whoami", "--json", "--no-input"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if let Some(email) = val.get("email").and_then(|v| v.as_str()) {
                    return Some(email.to_string());
                }
            }
        }
    }

    // Fallback: the email that appears most frequently in "from" field for outbound-looking messages
    // (messages with SENT label)
    conn.query_row(
        "SELECT from_email FROM email_observations
         WHERE labels LIKE '%SENT%'
         GROUP BY from_email
         ORDER BY COUNT(*) DESC
         LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
}

// ---------------------------------------------------------------------------
// Email header helpers
// ---------------------------------------------------------------------------

fn find_header(headers: &[GogHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|h| h.name.as_deref() == Some(name))
        .and_then(|h| h.value.clone())
}

/// Extract raw email from "Name <email@example.com>" format.
fn extract_email_from_header(raw: &str) -> String {
    if let Some(start) = raw.rfind('<') {
        if let Some(end) = raw.rfind('>') {
            if end > start {
                return raw[start + 1..end].trim().to_lowercase();
            }
        }
    }
    // If no angle brackets, assume the whole thing is an email
    raw.trim().to_lowercase()
}

// ---------------------------------------------------------------------------
// Query functions
// ---------------------------------------------------------------------------

#[allow(dead_code)] // Used by MCP tools and future dashboard
pub fn get_contact_summary(email: &str) -> Result<Option<ContactSummary>, String> {
    let conn = open_db()?;

    conn.query_row(
        "SELECT email, name, interaction_count, last_seen, preferred_channel, tags
         FROM contacts WHERE email = ?1",
        params![email],
        |row| {
            let tags_raw: Option<String> = row.get(5)?;
            let tags: Vec<String> = tags_raw
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            Ok(ContactSummary {
                email: row.get(0)?,
                name: row.get(1)?,
                interaction_count: row.get(2)?,
                last_seen: row.get(3)?,
                preferred_channel: row.get(4)?,
                tags,
            })
        },
    )
    .optional()
    .map_err(|e| format!("Failed to query contact: {}", e))
}

pub fn get_recent_contacts(limit: u32) -> Result<Vec<ContactSummary>, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            "SELECT email, name, interaction_count, last_seen, preferred_channel, tags
             FROM contacts
             ORDER BY interaction_count DESC, last_seen DESC
             LIMIT ?1",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map(params![limit], |row| {
            let tags_raw: Option<String> = row.get(5)?;
            let tags: Vec<String> = tags_raw
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();

            Ok(ContactSummary {
                email: row.get(0)?,
                name: row.get(1)?,
                interaction_count: row.get(2)?,
                last_seen: row.get(3)?,
                preferred_channel: row.get(4)?,
                tags,
            })
        })
        .map_err(|e| format!("Failed to query contacts: {}", e))?;

    let mut contacts = Vec::new();
    for row in rows {
        contacts.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(contacts)
}

#[allow(dead_code)] // Used by MCP tools and future dashboard
pub fn get_unanswered_emails(hours: u32) -> Result<Vec<serde_json::Value>, String> {
    let conn = open_db()?;

    // Calculate cutoff time
    let cutoff_secs = hours as u64 * 3600;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let cutoff = now_secs.saturating_sub(cutoff_secs);
    let (y, m, d) = days_to_ymd(cutoff / 86400);
    let tod = cutoff % 86400;
    let cutoff_iso = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, tod / 3600, (tod % 3600) / 60, tod % 60
    );

    let mut stmt = conn
        .prepare(
            "SELECT e.from_email, e.subject, e.timestamp, e.thread_id,
                    c.name, c.interaction_count
             FROM email_observations e
             LEFT JOIN contacts c ON c.email = e.from_email
             WHERE e.is_inbound = 1
             AND e.replied = 0
             AND e.timestamp >= ?1
             ORDER BY e.timestamp ASC",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map(params![cutoff_iso], |row| {
            Ok(serde_json::json!({
                "from_email": row.get::<_, String>(0)?,
                "subject": row.get::<_, Option<String>>(1)?,
                "timestamp": row.get::<_, String>(2)?,
                "thread_id": row.get::<_, String>(3)?,
                "contact_name": row.get::<_, Option<String>>(4)?,
                "interaction_count": row.get::<_, Option<i64>>(5)?,
            }))
        })
        .map_err(|e| format!("Failed to query unanswered: {}", e))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(results)
}

pub fn get_contact_insights(email: &str) -> Result<ContactInsight, String> {
    let conn = open_db()?;

    let contact = conn
        .query_row(
            "SELECT email, name, first_seen, last_seen, interaction_count,
                    avg_response_time_mins, preferred_channel, tags
             FROM contacts WHERE email = ?1",
            params![email],
            |row| {
                let tags_raw: Option<String> = row.get(7)?;
                let tags: Vec<String> = tags_raw
                    .and_then(|t| serde_json::from_str(&t).ok())
                    .unwrap_or_default();

                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<f64>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    tags,
                ))
            },
        )
        .map_err(|e| format!("Contact not found: {}", e))?;

    let seven_days_ago = days_ago(7);

    let recent_emails: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM email_observations
             WHERE (from_email = ?1 OR to_emails LIKE ?2)
             AND timestamp >= ?3",
            params![email, format!("%{}%", email), seven_days_ago],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let recent_meetings: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM calendar_events
             WHERE attendees LIKE ?1
             AND start_time >= ?2",
            params![format!("%{}%", email), seven_days_ago],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let unanswered_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM email_observations
             WHERE from_email = ?1 AND is_inbound = 1 AND replied = 0",
            params![email],
            |row| row.get(0),
        )
        .unwrap_or(0);

    Ok(ContactInsight {
        email: contact.0,
        name: contact.1,
        first_seen: contact.2,
        last_seen: contact.3,
        interaction_count: contact.4,
        avg_response_time_mins: contact.5,
        preferred_channel: contact.6,
        tags: contact.7,
        recent_emails,
        recent_meetings,
        unanswered_count,
    })
}

pub fn get_activity_stats() -> Result<ActivityStats, String> {
    let conn = open_db()?;

    let contacts_tracked: u32 = conn
        .query_row("SELECT COUNT(*) FROM contacts", [], |row| row.get(0))
        .unwrap_or(0);

    let one_day_ago = days_ago(1);
    let emails_observed_24h: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM email_observations WHERE observed_at >= ?1",
            params![one_day_ago],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let seven_days_ago = days_ago(7);
    let calendar_events_7d: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM calendar_events WHERE start_time >= ?1",
            params![seven_days_ago],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let suggestions_pending: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM suggestions WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let last_observation: Option<String> = conn
        .query_row(
            "SELECT MAX(observed_at) FROM (
                SELECT observed_at FROM calendar_events
                UNION ALL
                SELECT observed_at FROM email_observations
            )",
            [],
            |row| row.get(0),
        )
        .unwrap_or(None);

    let top_contacts = get_recent_contacts(5)?;

    Ok(ActivityStats {
        contacts_tracked,
        emails_observed_24h,
        calendar_events_7d,
        suggestions_pending,
        top_contacts,
        last_observation,
    })
}

// ---------------------------------------------------------------------------
// Suggestion management
// ---------------------------------------------------------------------------

pub fn get_suggestions() -> Result<Vec<Suggestion>, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            "SELECT id, type, title, description, contact_email, confidence,
                    context, status, created_at, acted_at, expires_at
             FROM suggestions
             WHERE status = 'pending'
             AND (expires_at IS NULL OR expires_at > ?1)
             ORDER BY confidence DESC, created_at DESC
             LIMIT 20",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let now = now_iso();
    let rows = stmt
        .query_map(params![now], |row| {
            Ok(Suggestion {
                id: row.get(0)?,
                suggestion_type: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                contact_email: row.get(4)?,
                confidence: row.get(5)?,
                context: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                acted_at: row.get(9)?,
                expires_at: row.get(10)?,
            })
        })
        .map_err(|e| format!("Failed to query suggestions: {}", e))?;

    let mut suggestions = Vec::new();
    for row in rows {
        suggestions.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(suggestions)
}

pub fn dismiss_suggestion(id: i64) -> Result<(), String> {
    let conn = open_db()?;
    let now = now_iso();

    // Update suggestion status
    conn.execute(
        "UPDATE suggestions SET status = 'dismissed', acted_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| format!("Failed to dismiss suggestion: {}", e))?;

    // Increment dismissed count for the activity type
    conn.execute(
        "UPDATE autonomy_settings SET total_dismissed = total_dismissed + 1
         WHERE activity_type = (SELECT type FROM suggestions WHERE id = ?1)",
        params![id],
    )
    .ok(); // Non-critical

    Ok(())
}

pub fn accept_suggestion(id: i64) -> Result<Suggestion, String> {
    let conn = open_db()?;
    let now = now_iso();

    conn.execute(
        "UPDATE suggestions SET status = 'accepted', acted_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| format!("Failed to accept suggestion: {}", e))?;

    // Increment accepted count for the activity type
    conn.execute(
        "UPDATE autonomy_settings SET total_accepted = total_accepted + 1
         WHERE activity_type = (SELECT type FROM suggestions WHERE id = ?1)",
        params![id],
    )
    .ok(); // Non-critical

    // Return the suggestion
    conn.query_row(
        "SELECT id, type, title, description, contact_email, confidence,
                context, status, created_at, acted_at, expires_at
         FROM suggestions WHERE id = ?1",
        params![id],
        |row| {
            Ok(Suggestion {
                id: row.get(0)?,
                suggestion_type: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                contact_email: row.get(4)?,
                confidence: row.get(5)?,
                context: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                acted_at: row.get(9)?,
                expires_at: row.get(10)?,
            })
        },
    )
    .map_err(|e| format!("Failed to read accepted suggestion: {}", e))
}

// ---------------------------------------------------------------------------
// Autonomy settings
// ---------------------------------------------------------------------------

pub fn get_autonomy_settings() -> Result<Vec<AutonomySetting>, String> {
    let conn = open_db()?;

    let mut stmt = conn
        .prepare(
            "SELECT activity_type, level, promoted_at, total_accepted, total_dismissed
             FROM autonomy_settings
             ORDER BY activity_type",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AutonomySetting {
                activity_type: row.get(0)?,
                level: row.get(1)?,
                promoted_at: row.get(2)?,
                total_accepted: row.get(3)?,
                total_dismissed: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query autonomy settings: {}", e))?;

    let mut settings = Vec::new();
    for row in rows {
        settings.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(settings)
}

pub fn set_autonomy_level(activity_type: &str, level: &str) -> Result<(), String> {
    let valid_levels = ["observe", "suggest", "draft", "act"];
    if !valid_levels.contains(&level) {
        return Err(format!(
            "Invalid autonomy level '{}'. Use: observe, suggest, draft, or act.",
            level
        ));
    }

    let conn = open_db()?;
    let now = now_iso();

    conn.execute(
        "UPDATE autonomy_settings SET level = ?1, promoted_at = ?2 WHERE activity_type = ?3",
        params![level, now, activity_type],
    )
    .map_err(|e| format!("Failed to update autonomy level: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Pattern detection (Phase 2)
// ---------------------------------------------------------------------------

/// Detect contacts with 3+ interactions in 14 days but no contact in 5+ days.
pub fn detect_frequent_contacts() -> Result<Vec<Suggestion>, String> {
    let conn = open_db()?;
    let fourteen_days = days_ago(14);
    let five_days = days_ago(5);
    let now = now_iso();

    let mut stmt = conn
        .prepare(
            "SELECT c.email, c.name, c.interaction_count, c.last_seen
             FROM contacts c
             WHERE c.last_seen >= ?1
             AND c.last_seen < ?2
             AND c.interaction_count >= 3
             AND c.email NOT IN (
                 SELECT COALESCE(contact_email, '') FROM suggestions
                 WHERE type = 'catch_up' AND status = 'pending'
             )
             ORDER BY c.interaction_count DESC
             LIMIT 5",
        )
        .map_err(|e| format!("Query failed: {}", e))?;

    let rows = stmt
        .query_map(params![fourteen_days, five_days], |row| {
            let email: String = row.get(0)?;
            let name: Option<String> = row.get(1)?;
            let count: i64 = row.get(2)?;
            let last: String = row.get(3)?;

            let display = name.as_deref().unwrap_or(&email);
            let confidence = (count as f64 / 20.0).min(0.9);

            Ok(Suggestion {
                id: 0,
                suggestion_type: "catch_up".to_string(),
                title: format!("Catch up with {}", display),
                description: format!(
                    "{} has had {} interactions recently but hasn't been in touch since {}.",
                    display, count, &last[..10.min(last.len())]
                ),
                contact_email: Some(email),
                confidence,
                context: Some(serde_json::json!({
                    "interaction_count": count,
                    "last_seen": last,
                })
                .to_string()),
                status: "pending".to_string(),
                created_at: now.clone(),
                acted_at: None,
                expires_at: Some(days_ahead(7)),
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut suggestions = Vec::new();
    for row in rows {
        suggestions.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(suggestions)
}

/// Detect inbound emails from known contacts with no reply in 24+ hours.
pub fn detect_unanswered_threads() -> Result<Vec<Suggestion>, String> {
    let conn = open_db()?;
    let one_day = days_ago(1);
    let seven_days = days_ago(7);
    let now = now_iso();

    let mut stmt = conn
        .prepare(
            "SELECT e.from_email, e.subject, e.timestamp, e.thread_id, c.name
             FROM email_observations e
             LEFT JOIN contacts c ON c.email = e.from_email
             WHERE e.is_inbound = 1
             AND e.replied = 0
             AND e.timestamp < ?1
             AND e.timestamp >= ?2
             AND e.from_email NOT IN (
                 SELECT COALESCE(contact_email, '') FROM suggestions
                 WHERE type = 'respond' AND status = 'pending'
             )
             ORDER BY c.interaction_count DESC, e.timestamp ASC
             LIMIT 5",
        )
        .map_err(|e| format!("Query failed: {}", e))?;

    let rows = stmt
        .query_map(params![one_day, seven_days], |row| {
            let email: String = row.get(0)?;
            let subject: Option<String> = row.get(1)?;
            let ts: String = row.get(2)?;
            let thread_id: String = row.get(3)?;
            let name: Option<String> = row.get(4)?;

            let display = name.as_deref().unwrap_or(&email);
            let subj = subject.as_deref().unwrap_or("(no subject)");

            Ok(Suggestion {
                id: 0,
                suggestion_type: "respond".to_string(),
                title: format!("Reply to {} about \"{}\"", display, truncate(subj, 40)),
                description: format!(
                    "You haven't replied to {} about \"{}\". Sent {}.",
                    display, subj, &ts[..10.min(ts.len())]
                ),
                contact_email: Some(email),
                confidence: 0.7,
                context: Some(serde_json::json!({
                    "thread_id": thread_id,
                    "subject": subj,
                    "timestamp": ts,
                })
                .to_string()),
                status: "pending".to_string(),
                created_at: now.clone(),
                acted_at: None,
                expires_at: Some(days_ahead(3)),
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut suggestions = Vec::new();
    for row in rows {
        suggestions.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(suggestions)
}

/// Detect contacts who sent 2+ emails in 7 days with no response.
pub fn detect_reachout_attempts() -> Result<Vec<Suggestion>, String> {
    let conn = open_db()?;
    let seven_days = days_ago(7);
    let now = now_iso();

    let mut stmt = conn
        .prepare(
            "SELECT e.from_email, COUNT(*) as cnt, c.name, MAX(e.timestamp) as latest
             FROM email_observations e
             LEFT JOIN contacts c ON c.email = e.from_email
             WHERE e.is_inbound = 1
             AND e.replied = 0
             AND e.timestamp >= ?1
             GROUP BY e.from_email
             HAVING cnt >= 2
             AND e.from_email NOT IN (
                 SELECT COALESCE(contact_email, '') FROM suggestions
                 WHERE type = 'reachout' AND status = 'pending'
             )
             ORDER BY cnt DESC
             LIMIT 5",
        )
        .map_err(|e| format!("Query failed: {}", e))?;

    let rows = stmt
        .query_map(params![seven_days], |row| {
            let email: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let name: Option<String> = row.get(2)?;
            let latest: String = row.get(3)?;

            let display = name.as_deref().unwrap_or(&email);

            Ok(Suggestion {
                id: 0,
                suggestion_type: "reachout".to_string(),
                title: format!("{} has been trying to reach you", display),
                description: format!(
                    "{} has sent {} unanswered emails in the last 7 days. Latest: {}.",
                    display, count, &latest[..10.min(latest.len())]
                ),
                contact_email: Some(email),
                confidence: 0.85,
                context: Some(serde_json::json!({
                    "email_count": count,
                    "latest": latest,
                })
                .to_string()),
                status: "pending".to_string(),
                created_at: now.clone(),
                acted_at: None,
                expires_at: Some(days_ahead(3)),
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut suggestions = Vec::new();
    for row in rows {
        suggestions.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(suggestions)
}

/// Detect recurring meeting attendees with no recent event.
pub fn detect_meeting_patterns() -> Result<Vec<Suggestion>, String> {
    let conn = open_db()?;
    let thirty_days = days_ago(30);
    let fourteen_days = days_ago(14);
    let now = now_iso();

    // Find attendees who appeared in 3+ meetings in the last 30 days
    // but have no meetings scheduled in the next 14 days
    let mut stmt = conn
        .prepare(
            "SELECT attendee_email, COUNT(*) as meeting_count, name
             FROM (
                 -- Expand attendees JSON array into rows
                 SELECT je.value as attendee_email, c.name
                 FROM calendar_events ce,
                      json_each(ce.attendees) je
                 LEFT JOIN contacts c ON c.email = je.value
                 WHERE ce.start_time >= ?1
                 AND ce.start_time < ?2
             )
             GROUP BY attendee_email
             HAVING meeting_count >= 3
             AND attendee_email NOT IN (
                 -- Exclude if they have an upcoming meeting
                 SELECT je2.value
                 FROM calendar_events ce2,
                      json_each(ce2.attendees) je2
                 WHERE ce2.start_time >= ?2
             )
             AND attendee_email NOT IN (
                 SELECT COALESCE(contact_email, '') FROM suggestions
                 WHERE type = 'schedule_meeting' AND status = 'pending'
             )
             ORDER BY meeting_count DESC
             LIMIT 5",
        )
        .map_err(|e| format!("Query failed: {}", e))?;

    let rows = stmt
        .query_map(params![thirty_days, fourteen_days], |row| {
            let email: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            let name: Option<String> = row.get(2)?;

            let display = name.as_deref().unwrap_or(&email);

            Ok(Suggestion {
                id: 0,
                suggestion_type: "schedule_meeting".to_string(),
                title: format!("Schedule meeting with {}", display),
                description: format!(
                    "You've had {} meetings with {} in the last month but none upcoming. Time to schedule one?",
                    count, display
                ),
                contact_email: Some(email),
                confidence: 0.6,
                context: Some(serde_json::json!({
                    "meeting_count_30d": count,
                })
                .to_string()),
                status: "pending".to_string(),
                created_at: now.clone(),
                acted_at: None,
                expires_at: Some(days_ahead(7)),
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?;

    let mut suggestions = Vec::new();
    for row in rows {
        suggestions.push(row.map_err(|e| format!("Row error: {}", e))?);
    }

    Ok(suggestions)
}

/// Master function: run all detectors, deduplicate, insert into suggestions table.
pub fn generate_suggestions() -> Result<u32, String> {
    let mut all: Vec<Suggestion> = Vec::new();

    // Run each detector, ignoring errors (best-effort)
    if let Ok(mut s) = detect_reachout_attempts() {
        all.append(&mut s);
    }
    if let Ok(mut s) = detect_unanswered_threads() {
        all.append(&mut s);
    }
    if let Ok(mut s) = detect_frequent_contacts() {
        all.append(&mut s);
    }
    if let Ok(mut s) = detect_meeting_patterns() {
        all.append(&mut s);
    }

    if all.is_empty() {
        return Ok(0);
    }

    let conn = open_db()?;
    let mut count = 0u32;

    // Check autonomy level before inserting
    for suggestion in &all {
        let autonomy_level: String = conn
            .query_row(
                "SELECT level FROM autonomy_settings WHERE activity_type = ?1",
                params![suggestion.suggestion_type],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "suggest".to_string());

        // If level is "observe", don't create suggestions
        if autonomy_level == "observe" {
            continue;
        }

        // Deduplicate: skip if a similar pending suggestion already exists
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM suggestions
                 WHERE type = ?1 AND contact_email = ?2 AND status = 'pending'",
                params![suggestion.suggestion_type, suggestion.contact_email],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0;

        if exists {
            continue;
        }

        conn.execute(
            "INSERT INTO suggestions (type, title, description, contact_email, confidence, context, status, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                suggestion.suggestion_type,
                suggestion.title,
                suggestion.description,
                suggestion.contact_email,
                suggestion.confidence,
                suggestion.context,
                "pending",
                suggestion.created_at,
                suggestion.expires_at,
            ],
        )
        .map_err(|e| format!("Failed to insert suggestion: {}", e))?;

        count += 1;
    }

    // Clean up expired suggestions
    let now = now_iso();
    conn.execute(
        "UPDATE suggestions SET status = 'expired'
         WHERE status = 'pending' AND expires_at IS NOT NULL AND expires_at < ?1",
        params![now],
    )
    .ok();

    // Clean up old dismissed/expired suggestions (30+ days)
    let thirty_days = days_ago(30);
    conn.execute(
        "DELETE FROM suggestions
         WHERE status IN ('dismissed', 'expired', 'executed')
         AND acted_at < ?1",
        params![thirty_days],
    )
    .ok();

    Ok(count)
}

// ---------------------------------------------------------------------------
// Trust building — check if any activity type qualifies for promotion
// ---------------------------------------------------------------------------

#[allow(dead_code)] // Used by Phase 4 autonomy escalation
pub fn check_promotion_eligibility() -> Result<Option<AutonomySetting>, String> {
    let conn = open_db()?;

    // Find activity types with 10+ consecutive accepts and 0 dismissals
    let result = conn
        .query_row(
            "SELECT activity_type, level, promoted_at, total_accepted, total_dismissed
             FROM autonomy_settings
             WHERE total_accepted >= 10
             AND total_dismissed = 0
             AND level != 'act'
             ORDER BY total_accepted DESC
             LIMIT 1",
            [],
            |row| {
                Ok(AutonomySetting {
                    activity_type: row.get(0)?,
                    level: row.get(1)?,
                    promoted_at: row.get(2)?,
                    total_accepted: row.get(3)?,
                    total_dismissed: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Failed to check promotions: {}", e))?;

    Ok(result)
}

/// Get the next autonomy level after the current one.
#[allow(dead_code)] // Used by Phase 4 autonomy escalation
pub fn next_autonomy_level(current: &str) -> Option<&'static str> {
    match current {
        "observe" => Some("suggest"),
        "suggest" => Some("draft"),
        "draft" => Some("act"),
        "act" => None,
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Data management
// ---------------------------------------------------------------------------

/// Delete all intelligence data (for when user disables the feature).
pub fn clear_all_data() -> Result<(), String> {
    let conn = open_db()?;

    conn.execute_batch(
        "DELETE FROM email_observations;
         DELETE FROM calendar_events;
         DELETE FROM contacts;
         DELETE FROM suggestions;
         -- Reset autonomy counters but keep level settings
         UPDATE autonomy_settings SET total_accepted = 0, total_dismissed = 0;",
    )
    .map_err(|e| format!("Failed to clear intelligence data: {}", e))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Background observer
// ---------------------------------------------------------------------------

pub fn start_observer(app: AppHandle) {
    tokio::spawn(async move {
        // Wait a few seconds for app to finish initialising
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Initialise database
        if let Err(e) = init_db() {
            eprintln!("[intelligence] Failed to initialise database: {}", e);
            return;
        }

        let mut calendar_interval =
            tokio::time::interval(tokio::time::Duration::from_secs(15 * 60));
        let mut email_interval =
            tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        let mut suggestion_interval =
            tokio::time::interval(tokio::time::Duration::from_secs(60 * 60));

        // Tick once immediately to skip the first instant tick
        calendar_interval.tick().await;
        email_interval.tick().await;
        suggestion_interval.tick().await;

        loop {
            tokio::select! {
                _ = calendar_interval.tick() => {
                    // Check if capability is still enabled
                    if !is_intelligence_enabled() {
                        continue;
                    }
                    match observe_calendar() {
                        Ok(count) => {
                            if count > 0 {
                                let _ = app.emit("intelligence:update", serde_json::json!({
                                    "source": "calendar",
                                    "count": count,
                                }));
                            }
                        }
                        Err(e) => eprintln!("[intelligence] Calendar observation failed: {}", e),
                    }
                }
                _ = email_interval.tick() => {
                    if !is_intelligence_enabled() {
                        continue;
                    }
                    match observe_email() {
                        Ok(count) => {
                            if count > 0 {
                                let _ = app.emit("intelligence:update", serde_json::json!({
                                    "source": "email",
                                    "count": count,
                                }));
                            }
                        }
                        Err(e) => eprintln!("[intelligence] Email observation failed: {}", e),
                    }
                }
                _ = suggestion_interval.tick() => {
                    if !is_intelligence_enabled() {
                        continue;
                    }
                    match generate_suggestions() {
                        Ok(count) => {
                            if count > 0 {
                                let _ = app.emit("intelligence:suggestions", serde_json::json!({
                                    "new_count": count,
                                }));
                            }
                        }
                        Err(e) => eprintln!("[intelligence] Suggestion generation failed: {}", e),
                    }
                }
            }
        }
    });
}

/// Check if the activity_intelligence capability is enabled.
fn is_intelligence_enabled() -> bool {
    // Read from the settings config (config is in nyx_lib)
    match nyx_lib::config::read_current_config() {
        Ok(settings) => settings.capabilities.activity_intelligence,
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}
