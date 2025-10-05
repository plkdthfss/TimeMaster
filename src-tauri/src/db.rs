use std::path::PathBuf;

use chrono::Utc;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("failed to access application data directory: {0}")]
    AppDir(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Clone)]
pub struct Db {
    pool: Pool<SqliteConnectionManager>,
}

impl Db {
    pub fn init(handle: &AppHandle) -> Result<Self> {
        let db_path = Self::database_path(handle)?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder().max_size(8).build(manager)?;
        let db = Self { pool };
        db.run_migrations()?;
        Ok(db)
    }

    fn database_path(handle: &AppHandle) -> Result<PathBuf> {
        let base = handle
            .path()
            .app_data_dir()
            .map_err(|err| DbError::AppDir(err.to_string()))?;
        Ok(base.join("storage").join("timemaster.db"))
    }

    fn conn(&self) -> Result<PooledConnection<SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }

    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                task_type TEXT NOT NULL,
                progress INTEGER NOT NULL DEFAULT 0,
                target INTEGER NOT NULL DEFAULT 1,
                repeat_rule TEXT,
                start_date TEXT,
                end_date TEXT,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
            "#,
        )?;
        Ok(())
    }

    pub fn list_tasks(&self, status: Option<String>) -> Result<Vec<Task>> {
        let conn = self.conn()?;
        if let Some(status) = status {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, task_type, progress, target, repeat_rule, start_date, end_date, status, created_at, updated_at FROM tasks WHERE status = ? ORDER BY updated_at DESC",
            )?;
            let rows = stmt
                .query_map([status], Task::from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, task_type, progress, target, repeat_rule, start_date, end_date, status, created_at, updated_at FROM tasks ORDER BY updated_at DESC",
            )?;
            let rows = stmt
                .query_map([], Task::from_row)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        }
    }

    pub fn create_task(&self, payload: NewTask) -> Result<Task> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();
        let id = payload.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let target = payload.target.unwrap_or(1).max(1);
        let (repeat_rule, start_date, end_date) =
            normalize_schedule(payload.task_type.as_str(), payload.repeat.clone(), payload.date_range.clone())?;

        conn.execute(
            "INSERT INTO tasks (id, name, description, task_type, progress, target, repeat_rule, start_date, end_date, status, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                payload.name.trim(),
                payload.description.unwrap_or_default().trim(),
                payload.task_type,
                target,
                repeat_rule,
                start_date,
                end_date,
                "active",
                now,
                now
            ],
        )?;

        self.fetch_task(&conn, &id)
    }

    pub fn update_task(&self, payload: UpdateTask) -> Result<Task> {
        let conn = self.conn()?;
        let mut existing = self.fetch_task(&conn, &payload.id)?;

        let (repeat_rule, start_date, end_date) =
            normalize_schedule(payload.task_type.as_str(), payload.repeat.clone(), payload.date_range.clone())?;
        existing.name = payload.name.trim().to_string();
        existing.description = payload
            .description
            .unwrap_or_default()
            .trim()
            .to_string();
        existing.task_type = payload.task_type;
        existing.target = payload.target.unwrap_or(existing.target).max(1);
        existing.repeat_rule = repeat_rule;
        existing.start_date = start_date;
        existing.end_date = end_date;
        existing.updated_at = Utc::now().to_rfc3339();

        if existing.progress > existing.target {
            existing.progress = existing.target;
        }

        if existing.status != "archived" {
            existing.status = if existing.progress >= existing.target {
                "completed".into()
            } else {
                "active".into()
            };
        }

        conn.execute(
            "UPDATE tasks SET name = ?1, description = ?2, task_type = ?3, target = ?4, repeat_rule = ?5, start_date = ?6, end_date = ?7, progress = ?8, status = ?9, updated_at = ?10 WHERE id = ?11",
            params![
                existing.name,
                existing.description,
                existing.task_type,
                existing.target,
                existing.repeat_rule,
                existing.start_date,
                existing.end_date,
                existing.progress,
                existing.status,
                existing.updated_at,
                existing.id
            ],
        )?;

        self.fetch_task(&conn, &existing.id)
    }

    pub fn delete_task(&self, id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn increment_progress(&self, id: &str) -> Result<Task> {
        let conn = self.conn()?;
        let mut task = self.fetch_task(&conn, id)?;

        if task.status == "archived" {
            return Err(DbError::InvalidInput("cannot update archived task".into()));
        }

        if task.progress < task.target {
            task.progress += 1;
        }
        if task.progress >= task.target {
            task.status = "completed".into();
        }
        task.updated_at = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE tasks SET progress = ?1, status = ?2, updated_at = ?3 WHERE id = ?4",
            params![task.progress, task.status, task.updated_at, task.id],
        )?;

        Ok(task)
    }

    pub fn archive_task(&self, id: &str) -> Result<Task> {
        self.update_status(id, "archived")
    }

    pub fn reopen_task(&self, id: &str) -> Result<Task> {
        let task = self.update_status(id, "active")?;
        if task.task_type == "cycle" {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE tasks SET progress = 0, updated_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), task.id],
            )?;
            return self.fetch_task(&conn, &task.id);
        }
        Ok(task)
    }

    fn update_status(&self, id: &str, status: &str) -> Result<Task> {
        let conn = self.conn()?;
        let updated_at = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, updated_at, id],
        )?;
        self.fetch_task(&conn, id)
    }

    fn fetch_task(&self, conn: &rusqlite::Connection, id: &str) -> Result<Task> {
        conn
            .query_row(
                "SELECT id, name, description, task_type, progress, target, repeat_rule, start_date, end_date, status, created_at, updated_at FROM tasks WHERE id = ?1",
                [id],
                Task::from_row,
            )
            .optional()
            .map_err(DbError::from)?
            .ok_or_else(|| DbError::InvalidInput(format!("task {id} not found")))
    }
}

fn normalize_schedule(
    task_type: &str,
    repeat: Option<String>,
    date_range: Option<Vec<String>>,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    match task_type {
        "cycle" => Ok((repeat.filter(|r| !r.is_empty()), None, None)),
        "long_term" => {
            let range = date_range.unwrap_or_default();
            if range.len() == 2 {
                Ok((None, Some(range[0].clone()), Some(range[1].clone())))
            } else {
                Err(DbError::InvalidInput(
                    "long term task requires start and end date".into(),
                ))
            }
        }
        _ => Ok((None, None, None)),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub progress: i64,
    pub target: i64,
    #[serde(rename = "repeatRule")]
    pub repeat_rule: Option<String>,
    #[serde(rename = "startDate")]
    pub start_date: Option<String>,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

impl Task {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            task_type: row.get(3)?,
            progress: row.get(4)?,
            target: row.get(5)?,
            repeat_rule: row.get(6)?,
            start_date: row.get(7)?,
            end_date: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub task_type: String,
    pub target: Option<i64>,
    pub repeat: Option<String>,
    pub date_range: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTask {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub task_type: String,
    pub target: Option<i64>,
    pub repeat: Option<String>,
    pub date_range: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdPayload {
    pub id: String,
}


