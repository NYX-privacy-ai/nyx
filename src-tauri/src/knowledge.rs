// ---------------------------------------------------------------------------
// Knowledge — Local wiki/knowledge base
// ---------------------------------------------------------------------------
// SQLite-backed knowledge store in ~/.nyx/intelligence.db. The agent creates
// entries from conversations, document ingestion, and proactive learning.
// Users can browse, search, and edit entries via the Wiki page.
// All data stays on-device.
// ---------------------------------------------------------------------------

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::intelligence;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeEntry {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub category: String, // entity | concept | document | note | meeting | project
    pub source: Option<String>, // chat | email | calendar | manual | document | web
    pub source_ref: Option<String>, // e.g. session_id, email thread, URL
    pub related_ids: Vec<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeStats {
    pub total_entries: u32,
    pub categories: Vec<CategoryCount>,
    pub recent_count_7d: u32,
    pub top_tags: Vec<TagCount>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryCount {
    pub category: String,
    pub count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TagCount {
    pub tag: String,
    pub count: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateKnowledgeInput {
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub source: Option<String>,
    pub source_ref: Option<String>,
    pub related_ids: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKnowledgeInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

pub fn init_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS knowledge (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '[]',
            category TEXT DEFAULT 'note',
            source TEXT,
            source_ref TEXT,
            related_ids TEXT DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_knowledge_category ON knowledge(category);
        CREATE INDEX IF NOT EXISTS idx_knowledge_title ON knowledge(title);

        -- Full-text search support
        CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts USING fts5(
            title, content, tags, category,
            content=knowledge, content_rowid=id
        );

        -- Keep FTS in sync via triggers
        CREATE TRIGGER IF NOT EXISTS knowledge_ai AFTER INSERT ON knowledge BEGIN
            INSERT INTO knowledge_fts(rowid, title, content, tags, category)
            VALUES (new.id, new.title, new.content, new.tags, new.category);
        END;

        CREATE TRIGGER IF NOT EXISTS knowledge_ad AFTER DELETE ON knowledge BEGIN
            INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, tags, category)
            VALUES ('delete', old.id, old.title, old.content, old.tags, old.category);
        END;

        CREATE TRIGGER IF NOT EXISTS knowledge_au AFTER UPDATE ON knowledge BEGIN
            INSERT INTO knowledge_fts(knowledge_fts, rowid, title, content, tags, category)
            VALUES ('delete', old.id, old.title, old.content, old.tags, old.category);
            INSERT INTO knowledge_fts(rowid, title, content, tags, category)
            VALUES (new.id, new.title, new.content, new.tags, new.category);
        END;
        ",
    )
    .map_err(|e| format!("Failed to init knowledge tables: {}", e))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const VALID_CATEGORIES: &[&str] = &[
    "entity", "concept", "document", "note", "meeting", "project",
];

fn validate_category(value: &str) -> Result<(), String> {
    if VALID_CATEGORIES.contains(&value) {
        Ok(())
    } else {
        Err(format!(
            "invalid category: '{}' (allowed: {})",
            value,
            VALID_CATEGORIES.join(", ")
        ))
    }
}

/// Turn arbitrary user input into a safe FTS5 MATCH expression: each
/// whitespace-separated token becomes a quoted phrase (embedded quotes
/// doubled), so special characters and operators (`"`, `*`, `AND`, `:`, …)
/// can't cause a query-syntax error. Returns an empty string when there are no
/// usable tokens.
fn fts_sanitize(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_json_array(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

fn parse_id_array(s: &str) -> Vec<i64> {
    serde_json::from_str(s).unwrap_or_default()
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeEntry> {
    let tags_str: String = row.get(3)?;
    let related_str: String = row.get(7)?;
    Ok(KnowledgeEntry {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        tags: parse_json_array(&tags_str),
        category: row.get(4)?,
        source: row.get(5)?,
        source_ref: row.get(6)?,
        related_ids: parse_id_array(&related_str),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

pub fn list_entries(
    category_filter: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<KnowledgeEntry>, String> {
    let conn = intelligence::open_db_pub()?;
    let lim = limit.unwrap_or(100);

    let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(cat) =
        category_filter
    {
        (
            "SELECT id, title, content, tags, category, source, source_ref, related_ids, created_at, updated_at FROM knowledge WHERE category = ?1 ORDER BY updated_at DESC LIMIT ?2".to_string(),
            vec![Box::new(cat.to_string()), Box::new(lim)],
        )
    } else {
        (
            "SELECT id, title, content, tags, category, source, source_ref, related_ids, created_at, updated_at FROM knowledge ORDER BY updated_at DESC LIMIT ?1".to_string(),
            vec![Box::new(lim)],
        )
    };

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_refs.as_slice(), row_to_entry)
        .map_err(|e| e.to_string())?;

    let mut entries = vec![];
    for row in rows {
        entries.push(row.map_err(|e| e.to_string())?);
    }
    Ok(entries)
}

pub fn search_entries(query: &str) -> Result<Vec<KnowledgeEntry>, String> {
    // Sanitize into a safe FTS5 expression; blank input → no results.
    let match_expr = fts_sanitize(query);
    if match_expr.is_empty() {
        return Ok(vec![]);
    }

    let conn = intelligence::open_db_pub()?;
    let mut stmt = conn
        .prepare(
            "SELECT k.id, k.title, k.content, k.tags, k.category, k.source, k.source_ref, k.related_ids, k.created_at, k.updated_at
             FROM knowledge k
             JOIN knowledge_fts fts ON k.id = fts.rowid
             WHERE knowledge_fts MATCH ?1
             ORDER BY rank
             LIMIT 50",
        )
        .map_err(|e| e.to_string())?;

    // Degrade gracefully: a malformed FTS query yields no results rather than
    // surfacing a raw SQLite syntax error to the user.
    let rows = match stmt.query_map(params![match_expr], row_to_entry) {
        Ok(rows) => rows,
        Err(_) => return Ok(vec![]),
    };
    let mut entries = vec![];
    for row in rows.flatten() {
        entries.push(row);
    }
    Ok(entries)
}

pub fn get_entry(id: i64) -> Result<KnowledgeEntry, String> {
    let conn = intelligence::open_db_pub()?;
    conn.query_row(
        "SELECT id, title, content, tags, category, source, source_ref, related_ids, created_at, updated_at FROM knowledge WHERE id = ?1",
        params![id],
        row_to_entry,
    )
    .map_err(|e| format!("Knowledge entry not found: {}", e))
}

pub fn create_entry(input: CreateKnowledgeInput) -> Result<KnowledgeEntry, String> {
    let conn = intelligence::open_db_pub()?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let tags =
        serde_json::to_string(&input.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".to_string());
    let category = input.category.unwrap_or_else(|| "note".to_string());
    validate_category(&category)?;
    let related = serde_json::to_string(&input.related_ids.unwrap_or_default())
        .unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT INTO knowledge (title, content, tags, category, source, source_ref, related_ids, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![input.title, input.content, tags, category, input.source, input.source_ref, related, now, now],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    get_entry(id)
}

pub fn update_entry(id: i64, input: UpdateKnowledgeInput) -> Result<KnowledgeEntry, String> {
    let conn = intelligence::open_db_pub()?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut param_idx = 2u32;
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];

    if let Some(v) = &input.title {
        sets.push(format!("title = ?{}", param_idx));
        params_vec.push(Box::new(v.clone()));
        param_idx += 1;
    }
    if let Some(v) = &input.content {
        sets.push(format!("content = ?{}", param_idx));
        params_vec.push(Box::new(v.clone()));
        param_idx += 1;
    }
    if let Some(v) = &input.tags {
        let json = serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string());
        sets.push(format!("tags = ?{}", param_idx));
        params_vec.push(Box::new(json));
        param_idx += 1;
    }
    if let Some(v) = &input.category {
        validate_category(v)?;
        sets.push(format!("category = ?{}", param_idx));
        params_vec.push(Box::new(v.clone()));
        param_idx += 1;
    }
    let _ = param_idx;

    params_vec.push(Box::new(id));
    let query = format!(
        "UPDATE knowledge SET {} WHERE id = ?{}",
        sets.join(", "),
        params_vec.len()
    );

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| e.to_string())?;

    get_entry(id)
}

pub fn delete_entry(id: i64) -> Result<(), String> {
    let conn = intelligence::open_db_pub()?;
    let affected = conn
        .execute("DELETE FROM knowledge WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    if affected == 0 {
        return Err(format!("Knowledge entry {} not found", id));
    }
    Ok(())
}

pub fn get_knowledge_stats() -> Result<KnowledgeStats, String> {
    let conn = intelligence::open_db_pub()?;

    let total: u32 = conn
        .query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))
        .unwrap_or(0);

    let recent_7d: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM knowledge WHERE created_at > datetime('now', '-7 days')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Categories
    let mut stmt = conn
        .prepare(
            "SELECT category, COUNT(*) FROM knowledge GROUP BY category ORDER BY COUNT(*) DESC",
        )
        .map_err(|e| e.to_string())?;
    let categories: Vec<CategoryCount> = stmt
        .query_map([], |r| {
            Ok(CategoryCount {
                category: r.get(0)?,
                count: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Top tags (parse JSON arrays, count occurrences)
    let mut tag_counts = std::collections::HashMap::new();
    let mut tag_stmt = conn
        .prepare("SELECT tags FROM knowledge")
        .map_err(|e| e.to_string())?;
    let tag_rows = tag_stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for row in tag_rows {
        if let Ok(tags_str) = row {
            for tag in parse_json_array(&tags_str) {
                *tag_counts.entry(tag).or_insert(0u32) += 1;
            }
        }
    }
    let mut top_tags: Vec<TagCount> = tag_counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect();
    top_tags.sort_by(|a, b| b.count.cmp(&a.count));
    top_tags.truncate(10);

    Ok(KnowledgeStats {
        total_entries: total,
        categories,
        recent_count_7d: recent_7d,
        top_tags,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_sanitize_quotes_tokens() {
        assert_eq!(fts_sanitize("hello world"), "\"hello\" \"world\"");
        // Embedded double-quotes are doubled, not left to break the expression.
        assert_eq!(fts_sanitize("a\"b"), "\"a\"\"b\"");
        // FTS operators/special chars become literal quoted tokens.
        assert_eq!(fts_sanitize("foo OR *"), "\"foo\" \"OR\" \"*\"");
    }

    #[test]
    fn fts_sanitize_blank_is_empty() {
        assert_eq!(fts_sanitize(""), "");
        assert_eq!(fts_sanitize("   \t  "), "");
    }

    #[test]
    fn validates_category() {
        assert!(validate_category("note").is_ok());
        assert!(validate_category("project").is_ok());
        assert!(validate_category("random").is_err());
        assert!(validate_category("").is_err());
    }
}
