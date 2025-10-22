//! Session history tracking using SQLite

use crate::error::{OmenError, Result};
use chrono::Datelike;  // For with_day() method
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, sqlite::{SqlitePoolOptions, SqliteConnectOptions}};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

/// Session history manager
/// Planned feature for conversation memory across requests
#[allow(dead_code)]
pub struct SessionHistory {
    pool: SqlitePool,
}

/// A session represents a period of AI interaction
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub session_uuid: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub branch: String,
    pub last_commit: String,
    pub files_open: String,  // JSON array
    pub summary: Option<String>,
}

/// A single AI query within a session
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub id: i64,
    pub session_id: i64,
    pub timestamp: String,
    pub task_type: String,
    pub prompt: String,
    pub model_used: String,
    pub provider: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub latency_ms: i64,
    pub result_summary: Option<String>,
}

#[allow(dead_code)]
impl SessionHistory {
    /// Open or create session history database
    pub async fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| OmenError::Database(format!("Failed to create parent directory: {}", e)))?;
        }

        // Create connect options with create_if_missing
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
            .map_err(|e| OmenError::Database(format!("Failed to parse database URL: {}", e)))?
            .create_if_missing(true);

        // Create pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| OmenError::Database(format!("Failed to connect to SQLite: {}", e)))?;

        // Run migrations
        Self::migrate(&pool).await?;

        Ok(Self { pool })
    }

    /// Run database migrations
    async fn migrate(pool: &SqlitePool) -> Result<()> {
        // Create sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_uuid TEXT UNIQUE NOT NULL,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                branch TEXT NOT NULL,
                last_commit TEXT NOT NULL,
                files_open TEXT NOT NULL,
                summary TEXT
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to create sessions table: {}", e)))?;

        // Create queries table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS queries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                timestamp TEXT NOT NULL,
                task_type TEXT NOT NULL,
                prompt TEXT NOT NULL,
                model_used TEXT NOT NULL,
                provider TEXT NOT NULL,
                tokens_in INTEGER NOT NULL,
                tokens_out INTEGER NOT NULL,
                cost_usd REAL NOT NULL,
                latency_ms INTEGER NOT NULL,
                result_summary TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to create queries table: {}", e)))?;

        // Create indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_session_time ON sessions(started_at)")
            .execute(pool)
            .await
            .ok();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_query_session ON queries(session_id)")
            .execute(pool)
            .await
            .ok();

        Ok(())
    }

    /// Create new session
    pub async fn create_session(&self, branch: &str, commit: &str, files: Vec<String>) -> Result<Session> {
        let uuid = Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();
        let files_json = serde_json::to_string(&files)
            .unwrap_or_else(|_| "[]".to_string());

        let result = sqlx::query(
            r#"
            INSERT INTO sessions (session_uuid, started_at, branch, last_commit, files_open)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&uuid)
        .bind(&started_at)
        .bind(branch)
        .bind(commit)
        .bind(&files_json)
        .execute(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to create session: {}", e)))?;

        Ok(Session {
            id: result.last_insert_rowid(),
            session_uuid: uuid,
            started_at,
            ended_at: None,
            branch: branch.to_string(),
            last_commit: commit.to_string(),
            files_open: files_json,
            summary: None,
        })
    }

    /// End a session
    pub async fn end_session(&self, session_id: i64, summary: Option<String>) -> Result<()> {
        let ended_at = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE sessions
            SET ended_at = ?, summary = ?
            WHERE id = ?
            "#,
        )
        .bind(&ended_at)
        .bind(&summary)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to end session: {}", e)))?;

        Ok(())
    }

    /// Record a query
    pub async fn record_query(
        &self,
        session_id: i64,
        task_type: &str,
        prompt: &str,
        model_used: &str,
        provider: &str,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: f64,
        latency_ms: i64,
        result_summary: Option<String>,
    ) -> Result<Query> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            INSERT INTO queries (
                session_id, timestamp, task_type, prompt, model_used,
                provider, tokens_in, tokens_out, cost_usd, latency_ms, result_summary
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(session_id)
        .bind(&timestamp)
        .bind(task_type)
        .bind(prompt)
        .bind(model_used)
        .bind(provider)
        .bind(tokens_in)
        .bind(tokens_out)
        .bind(cost_usd)
        .bind(latency_ms)
        .bind(&result_summary)
        .execute(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to record query: {}", e)))?;

        Ok(Query {
            id: result.last_insert_rowid(),
            session_id,
            timestamp,
            task_type: task_type.to_string(),
            prompt: prompt.to_string(),
            model_used: model_used.to_string(),
            provider: provider.to_string(),
            tokens_in,
            tokens_out,
            cost_usd,
            latency_ms,
            result_summary,
        })
    }

    /// Get recent sessions
    pub async fn get_recent_sessions(&self, limit: i64) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, session_uuid, started_at, ended_at, branch, last_commit, files_open, summary
            FROM sessions
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to get recent sessions: {}", e)))?;

        Ok(sessions)
    }

    /// Get queries for a session
    pub async fn get_session_queries(&self, session_id: i64) -> Result<Vec<Query>> {
        let queries = sqlx::query_as::<_, Query>(
            r#"
            SELECT id, session_id, timestamp, task_type, prompt, model_used,
                   provider, tokens_in, tokens_out, cost_usd, latency_ms, result_summary
            FROM queries
            WHERE session_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to get session queries: {}", e)))?;

        Ok(queries)
    }

    /// Get cost for current month
    pub async fn get_monthly_cost(&self) -> Result<f64> {
        let start_of_month = chrono::Utc::now()
            .date_naive()
            .with_day(1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .to_rfc3339();

        let row: (f64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(cost_usd), 0.0)
            FROM queries
            WHERE timestamp >= ?
            "#,
        )
        .bind(&start_of_month)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| OmenError::Database(format!("Failed to get monthly cost: {}", e)))?;

        Ok(row.0)
    }
}

// Implement FromRow for Session
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Session {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Session {
            id: row.try_get("id")?,
            session_uuid: row.try_get("session_uuid")?,
            started_at: row.try_get("started_at")?,
            ended_at: row.try_get("ended_at")?,
            branch: row.try_get("branch")?,
            last_commit: row.try_get("last_commit")?,
            files_open: row.try_get("files_open")?,
            summary: row.try_get("summary")?,
        })
    }
}

// Implement FromRow for Query
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Query {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Query {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            timestamp: row.try_get("timestamp")?,
            task_type: row.try_get("task_type")?,
            prompt: row.try_get("prompt")?,
            model_used: row.try_get("model_used")?,
            provider: row.try_get("provider")?,
            tokens_in: row.try_get("tokens_in")?,
            tokens_out: row.try_get("tokens_out")?,
            cost_usd: row.try_get("cost_usd")?,
            latency_ms: row.try_get("latency_ms")?,
            result_summary: row.try_get("result_summary")?,
        })
    }
}
