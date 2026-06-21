// ---------------------------------------------------------------------------
// Tasks — Local task/todo management
// ---------------------------------------------------------------------------
// SQLite-backed task system stored in ~/.nyx/intelligence.db alongside the
// intelligence engine. Tasks can be created manually or generated from
// proactive suggestions. All data stays on-device.
// ---------------------------------------------------------------------------

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::intelligence;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,   // pending | in_progress | completed | cancelled
    pub priority: String, // urgent | high | normal | low
    pub category: Option<String>,
    pub due_date: Option<String>,
    pub source: Option<String>, // manual | suggestion | email | calendar | chat
    pub source_ref: Option<String>, // suggestion_id, email thread, etc.
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskStats {
    pub total: u32,
    pub pending: u32,
    pub in_progress: u32,
    pub completed_today: u32,
    pub overdue: u32,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub due_date: Option<String>,
    pub source: Option<String>,
    pub source_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub due_date: Option<String>,
}

// ---------------------------------------------------------------------------
// Schema (called during intelligence::init_db)
// ---------------------------------------------------------------------------

pub fn init_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS tasks (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT DEFAULT 'pending',
            priority TEXT DEFAULT 'normal',
            category TEXT,
            due_date TEXT,
            source TEXT DEFAULT 'manual',
            source_ref TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
        CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);
        ",
    )
    .map_err(|e| format!("Failed to init tasks tables: {}", e))
}

// ---------------------------------------------------------------------------
// CRUD operations
// ---------------------------------------------------------------------------

pub fn list_tasks(
    status_filter: Option<&str>,
    category_filter: Option<&str>,
) -> Result<Vec<Task>, String> {
    let conn = intelligence::open_db_pub()?;
    let mut query = "SELECT id, title, description, status, priority, category, due_date, source, source_ref, created_at, updated_at, completed_at FROM tasks WHERE 1=1".to_string();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(s) = status_filter {
        query.push_str(" AND status = ?");
        params_vec.push(Box::new(s.to_string()));
    }
    if let Some(c) = category_filter {
        query.push_str(" AND category = ?");
        params_vec.push(Box::new(c.to_string()));
    }
    query.push_str(" ORDER BY CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END, due_date ASC NULLS LAST, created_at DESC");

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(Task {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                category: row.get(5)?,
                due_date: row.get(6)?,
                source: row.get(7)?,
                source_ref: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                completed_at: row.get(11)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut tasks = vec![];
    for row in rows {
        tasks.push(row.map_err(|e| e.to_string())?);
    }
    Ok(tasks)
}

pub fn create_task(input: CreateTaskInput) -> Result<Task, String> {
    let conn = intelligence::open_db_pub()?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let priority = input.priority.unwrap_or_else(|| "normal".to_string());
    let source = input.source.unwrap_or_else(|| "manual".to_string());

    conn.execute(
        "INSERT INTO tasks (title, description, priority, category, due_date, source, source_ref, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![input.title, input.description, priority, input.category, input.due_date, source, input.source_ref, now, now],
    ).map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();
    Ok(Task {
        id,
        title: input.title,
        description: input.description,
        status: "pending".to_string(),
        priority,
        category: input.category,
        due_date: input.due_date,
        source: Some(source),
        source_ref: input.source_ref,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
    })
}

pub fn update_task(id: i64, input: UpdateTaskInput) -> Result<Task, String> {
    let conn = intelligence::open_db_pub()?;
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Build dynamic update
    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut param_idx = 2u32;
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

    macro_rules! maybe_set {
        ($field:ident, $col:literal) => {
            if let Some(v) = &input.$field {
                sets.push(format!("{} = ?{}", $col, param_idx));
                params_vec.push(Box::new(v.clone()));
                param_idx += 1;
            }
        };
    }

    maybe_set!(title, "title");
    maybe_set!(description, "description");
    maybe_set!(priority, "priority");
    maybe_set!(category, "category");
    maybe_set!(due_date, "due_date");

    if let Some(s) = &input.status {
        sets.push(format!("status = ?{}", param_idx));
        params_vec.push(Box::new(s.clone()));
        param_idx += 1;
        if s == "completed" || s == "cancelled" {
            sets.push(format!("completed_at = ?{}", param_idx));
            params_vec.push(Box::new(now.clone()));
            param_idx += 1;
        }
    }
    let _ = param_idx; // suppress unused warning

    // Add the id as the last param
    params_vec.push(Box::new(id));
    let query = format!(
        "UPDATE tasks SET {} WHERE id = ?{}",
        sets.join(", "),
        params_vec.len()
    );

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| e.to_string())?;

    // Return updated task
    let tasks = list_tasks(None, None)?;
    tasks
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| "Task not found".to_string())
}

pub fn delete_task(id: i64) -> Result<(), String> {
    let conn = intelligence::open_db_pub()?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_task_stats() -> Result<TaskStats, String> {
    let conn = intelligence::open_db_pub()?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let total: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status != 'cancelled'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'pending'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let in_progress: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let completed_today: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status = 'completed' AND completed_at LIKE ?1",
            params![format!("{}%", today)],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let overdue: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE status IN ('pending','in_progress') AND due_date IS NOT NULL AND due_date < ?1",
            params![today],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(TaskStats {
        total,
        pending,
        in_progress,
        completed_today,
        overdue,
    })
}
