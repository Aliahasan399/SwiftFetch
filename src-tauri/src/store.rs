//! SQLite persistence for the download queue, settings and event log.
//!
//! Schema follows specification §8.2. All timestamps are stored as RFC3339
//! text in UTC. The database is opened in WAL mode so a reader never blocks
//! the writer and a crash leaves a consistent log (spec §12.1).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{
    parse_rfc3339, to_rfc3339, DownloadProjection, DownloadStatus, OverwritePolicy, StartPolicy,
};

pub struct Store {
    conn: Mutex<Connection>,
    path: PathBuf,
}

pub(crate) enum BrowserIntake {
    Accepted,
    SettingsChanged,
    Replayed,
    Expired,
}

#[derive(Debug, Clone)]
pub struct SegmentRow {
    pub id: String,
    pub download_id: String,
    pub segment_index: i64,
    pub start_offset: i64,
    pub end_offset: i64,
    pub received_bytes: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub destination_template: String,
    pub extension_rules_json: String,
}

#[derive(Debug, Clone)]
pub struct Queue {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub max_active_tasks: i64,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub id: String,
    pub queue_id: String,
    pub days_mask: i64,
    pub start_local_time: String,
    pub end_local_time: String,
    pub action_after: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: String,
    pub queue_id: String,
    pub category_id: Option<String>,
    pub original_url: String,
    pub final_url: Option<String>,
    pub headers_json: String,
    pub display_name: String,
    pub destination_path: String,
    pub temporary_path: Option<String>,
    pub status: String,
    pub total_bytes: Option<i64>,
    pub received_bytes: i64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub accept_ranges: bool,
    pub content_type: Option<String>,
    pub checksum_algorithm: Option<String>,
    pub checksum_expected: Option<String>,
    pub checksum_actual: Option<String>,
    pub checksum_verified: bool,
    pub retry_count: i64,
    pub max_retries: i64,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
    pub priority: i64,
    pub start_policy: String,
    pub connections: i64,
    pub speed_limit: i64,
    pub overwrite_policy: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// Outcome of an `encrypt_saved_secrets` migration. Safe to return to the UI:
/// it contains counts and a boolean only — never header values or secrets.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SecretMigrationResult {
    pub enabled: bool,
    pub rows_rewritten: usize,
}

impl Store {
    /// Open (and migrate) the database at `path`.
    pub fn open(path: &Path) -> rusqlite::Result<Store> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", "10000")?;
        let store = Store {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Absolute path of the SQLite database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open an initialized queue for a short-lived browser handoff.
    pub(crate) fn open_existing(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE)?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Self { conn: Mutex::new(conn), path: path.to_path_buf() })
    }

    pub fn count_tasks(&self) -> rusqlite::Result<i64> {
        let conn = self.conn();
        conn.query_row("SELECT COUNT(*) FROM downloads", [], |r| r.get(0))
    }

    pub fn list_recent_events(&self, limit: i64) -> rusqlite::Result<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT event_type, created_at FROM event_log ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Delete every task in a terminal state. Files on disk are never touched
    /// (spec §8.4: "Clear history without deleting files").
    pub fn clear_finished_tasks(&self) -> rusqlite::Result<usize> {
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM download_segments WHERE download_id IN
                (SELECT id FROM downloads WHERE status IN ('completed','failed','cancelled'))",
            [],
        )?;
        tx.execute(
            "DELETE FROM event_log WHERE event_type != 'browser_intent' AND download_id IN
                (SELECT id FROM downloads WHERE status IN ('completed','failed','cancelled'))",
            [],
        )?;
        let count = tx.execute(
            "DELETE FROM downloads WHERE status IN ('completed','failed','cancelled')",
            [],
        )?;
        tx.commit()?;
        Ok(count)
    }

    /// `(total segments, completed segments)` for the UI projection.
    pub fn segment_counts(&self, download_id: &str) -> (u32, u32) {
        match self.list_segments(download_id) {
            Ok(segs) => {
                let total = segs.len() as u32;
                let done = segs.iter().filter(|s| s.status == "completed").count() as u32;
                (total, done)
            }
            Err(_) => (0, 0),
        }
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS queues (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              max_active_tasks INTEGER NOT NULL DEFAULT 3,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS categories (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              destination_template TEXT NOT NULL,
              extension_rules_json TEXT NOT NULL DEFAULT '[]',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS downloads (
              id TEXT PRIMARY KEY,
              queue_id TEXT NOT NULL REFERENCES queues(id),
              category_id TEXT REFERENCES categories(id),
              original_url TEXT NOT NULL,
              final_url TEXT,
              headers_json TEXT NOT NULL DEFAULT '[]',
              display_name TEXT NOT NULL,
              destination_path TEXT NOT NULL,
              temporary_path TEXT,
              status TEXT NOT NULL,
              total_bytes INTEGER,
              received_bytes INTEGER NOT NULL DEFAULT 0,
              etag TEXT,
              last_modified TEXT,
              accept_ranges INTEGER NOT NULL DEFAULT 0,
              content_type TEXT,
              checksum_algorithm TEXT,
              checksum_expected TEXT,
              checksum_actual TEXT,
              checksum_verified INTEGER NOT NULL DEFAULT 0,
              retry_count INTEGER NOT NULL DEFAULT 0,
              max_retries INTEGER NOT NULL DEFAULT 3,
              last_error_code TEXT,
              last_error_message TEXT,
              priority INTEGER NOT NULL DEFAULT 50,
              start_policy TEXT NOT NULL DEFAULT 'immediate',
              connections INTEGER NOT NULL DEFAULT 4,
              speed_limit INTEGER NOT NULL DEFAULT 0,
              overwrite_policy TEXT NOT NULL DEFAULT 'rename',
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              completed_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
            CREATE INDEX IF NOT EXISTS idx_downloads_queue ON downloads(queue_id);

            CREATE TABLE IF NOT EXISTS download_segments (
              id TEXT PRIMARY KEY,
              download_id TEXT NOT NULL REFERENCES downloads(id),
              segment_index INTEGER NOT NULL,
              start_offset INTEGER NOT NULL,
              end_offset INTEGER NOT NULL,
              received_bytes INTEGER NOT NULL DEFAULT 0,
              status TEXT NOT NULL,
              UNIQUE(download_id, segment_index)
            );

            CREATE TABLE IF NOT EXISTS schedules (
              id TEXT PRIMARY KEY,
              queue_id TEXT NOT NULL REFERENCES queues(id),
              days_mask INTEGER NOT NULL DEFAULT 127,
              start_local_time TEXT NOT NULL DEFAULT '00:00',
              end_local_time TEXT NOT NULL DEFAULT '23:59',
              action_after TEXT NOT NULL DEFAULT 'nothing',
              enabled INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS settings (
              key TEXT PRIMARY KEY,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS event_log (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              download_id TEXT,
              event_type TEXT NOT NULL,
              safe_payload_json TEXT NOT NULL DEFAULT '{}',
              created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_event_log_download ON event_log(download_id);
            "#,
        )?;
        Ok(())
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("store mutex poisoned")
    }

    // ---------------------------------------------------------------- queues

    pub fn ensure_default_queue(&self) -> rusqlite::Result<String> {
        let conn = self.conn();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM queues WHERE id = 'default'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "INSERT INTO queues (id, name, enabled, max_active_tasks, created_at, updated_at)
             VALUES ('default', 'Default', 1, 3, ?1, ?1)",
            params![now],
        )?;
        Ok("default".to_string())
    }

    pub fn list_queues(&self) -> rusqlite::Result<Vec<Queue>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, enabled, max_active_tasks FROM queues ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Queue {
                id: r.get(0)?,
                name: r.get(1)?,
                enabled: r.get::<_, i64>(2)? != 0,
                max_active_tasks: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ------------------------------------------------------------ categories

    pub fn ensure_default_category(&self) -> rusqlite::Result<String> {
        let conn = self.conn();
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM categories WHERE id = 'default'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "INSERT INTO categories (id, name, destination_template, extension_rules_json, created_at, updated_at)
             VALUES ('default', 'Downloads', '%USERPROFILE%/Downloads', '[]', ?1, ?1)",
            params![now],
        )?;
        Ok("default".to_string())
    }

    pub fn list_categories(&self) -> rusqlite::Result<Vec<Category>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, destination_template, extension_rules_json FROM categories ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                destination_template: r.get(2)?,
                extension_rules_json: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_category(
        &self,
        id: &str,
        name: &str,
        destination_template: &str,
        extension_rules_json: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "INSERT INTO categories (id, name, destination_template, extension_rules_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET name=?2, destination_template=?3,
               extension_rules_json=?4, updated_at=?5",
            params![id, name, destination_template, extension_rules_json, now],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------- settings

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT value_json FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten()
    }

    pub fn set_setting(&self, key: &str, value_json: &str) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value_json=?2, updated_at=?3",
            params![key, value_json, now],
        )?;
        Ok(())
    }

    pub fn all_settings(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value_json FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // -------------------------------------------------------------- schedules

    pub fn list_schedules(&self) -> rusqlite::Result<Vec<Schedule>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, queue_id, days_mask, start_local_time, end_local_time, action_after, enabled
             FROM schedules ORDER BY rowid",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Schedule {
                id: r.get(0)?,
                queue_id: r.get(1)?,
                days_mask: r.get(2)?,
                start_local_time: r.get(3)?,
                end_local_time: r.get(4)?,
                action_after: r.get(5)?,
                enabled: r.get::<_, i64>(6)? != 0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_schedule(
        &self,
        id: &str,
        queue_id: &str,
        days_mask: i64,
        start_local_time: &str,
        end_local_time: &str,
        action_after: &str,
        enabled: bool,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO schedules (id, queue_id, days_mask, start_local_time, end_local_time, action_after, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET queue_id=?2, days_mask=?3, start_local_time=?4,
               end_local_time=?5, action_after=?6, enabled=?7",
            params![
                id,
                queue_id,
                days_mask,
                start_local_time,
                end_local_time,
                action_after,
                if enabled { 1 } else { 0 }
            ],
        )?;
        Ok(())
    }

    pub fn delete_schedule(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --------------------------------------------------------------- segments

    /// Register a segment in the persisted plan.
    ///
    /// This is deliberately *not* a replace: the engine re-persists the plan on
    /// every attempt, and replacing a row would reset `received_bytes` to zero
    /// and `status` to `pending`, destroying durable checkpoints on resume
    /// (spec §12.2). A fresh plan is created after `clear_segments`, so the
    /// conflict branch only occurs when we are resuming an existing plan.
    pub fn insert_segment(
        &self,
        id: &str,
        download_id: &str,
        index: i64,
        start: i64,
        end: i64,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO download_segments
               (id, download_id, segment_index, start_offset, end_offset, received_bytes, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 'pending')
             ON CONFLICT(download_id, segment_index) DO NOTHING",
            params![id, download_id, index, start, end],
        )?;
        Ok(())
    }

    pub fn list_segments(&self, download_id: &str) -> rusqlite::Result<Vec<SegmentRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, download_id, segment_index, start_offset, end_offset, received_bytes, status
             FROM download_segments WHERE download_id = ?1 ORDER BY segment_index",
        )?;
        let rows = stmt.query_map(params![download_id], |r| {
            Ok(SegmentRow {
                id: r.get(0)?,
                download_id: r.get(1)?,
                segment_index: r.get(2)?,
                start_offset: r.get(3)?,
                end_offset: r.get(4)?,
                received_bytes: r.get(5)?,
                status: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_segment_progress(
        &self,
        download_id: &str,
        index: i64,
        received: i64,
        status: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute(
            "UPDATE download_segments SET received_bytes = ?3, status = ?4
             WHERE download_id = ?1 AND segment_index = ?2",
            params![download_id, index, received, status],
        )?;
        Ok(())
    }

    pub fn clear_segments(&self, download_id: &str) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM download_segments WHERE download_id = ?1",
            params![download_id],
        )?;
        Ok(())
    }

    // ---------------------------------------------------------------- events

    pub fn log_event(
        &self,
        download_id: Option<&str>,
        event_type: &str,
        payload_json: &str,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "INSERT INTO event_log (download_id, event_type, safe_payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![download_id, event_type, payload_json, now],
        )?;
        Ok(())
    }

    // --------------------------------------------------------------- helpers

    /// Mark every task that was mid-flight when the process died as paused.
    ///
    /// Called once at startup (spec §19, "App crash → startup reconciliation").
    pub fn reconcile_after_restart(&self) -> rusqlite::Result<usize> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        let n = conn.execute(
            "UPDATE downloads SET status = 'paused', updated_at = ?1
             WHERE status IN ('starting', 'downloading', 'verifying', 'scanning', 'retry_wait')",
            params![now],
        )?;
        Ok(n)
    }

    pub fn next_queued_tasks(&self, limit: i64) -> rusqlite::Result<Vec<TaskRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT * FROM downloads WHERE status = 'queued' ORDER BY priority DESC, created_at ASC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], task_row_from)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn next_queued_in_queue(&self, queue_id: &str, limit: i64, allow_scheduled: bool) -> rusqlite::Result<Vec<TaskRow>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT * FROM downloads WHERE queue_id = ?1 AND status = 'queued' AND start_policy != 'manual'
             AND (start_policy != 'scheduled' OR ?3)
             ORDER BY priority DESC, created_at ASC, id ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![queue_id, limit, allow_scheduled], task_row_from)?;
        rows.collect()
    }

    pub fn make_task_automatic(&self, id: &str) -> rusqlite::Result<()> {
        self.conn().execute(
            "UPDATE downloads SET start_policy = 'immediate' WHERE id = ?1 AND start_policy = 'manual'",
            params![id],
        )?;
        Ok(())
    }

    pub fn complete_task(&self, id: &str, received: u64, destination: &str) -> rusqlite::Result<()> {
        let name = std::path::Path::new(destination).file_name().and_then(|name| name.to_str()).unwrap_or("download");
        let mut conn = self.conn();
        let tx = conn.transaction()?;
        tx.execute("UPDATE download_segments SET status = 'completed' WHERE download_id = ?1", params![id])?;
        tx.execute(
            "UPDATE downloads SET status = 'completed', received_bytes = ?2, destination_path = ?3,
             display_name = ?4, temporary_path = NULL, completed_at = ?5, updated_at = ?5,
             last_error_code = NULL, last_error_message = NULL WHERE id = ?1",
            params![id, received.min(i64::MAX as u64) as i64, destination, name, to_rfc3339(chrono::Utc::now())],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_active(&self) -> rusqlite::Result<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM downloads WHERE status IN ('starting','downloading','verifying','scanning')",
            [],
            |r| r.get(0),
        )
    }

    pub fn count_active_in_queue(&self, queue_id: &str) -> rusqlite::Result<i64> {
        let conn = self.conn();
        conn.query_row(
            "SELECT COUNT(*) FROM downloads WHERE queue_id = ?1
               AND status IN ('starting','downloading','verifying','scanning')",
            params![queue_id],
            |r| r.get(0),
        )
    }

    pub fn get_task(&self, id: &str) -> rusqlite::Result<Option<TaskRow>> {
        let conn = self.conn();
        conn.query_row("SELECT * FROM downloads WHERE id = ?1", params![id], task_row_from)
            .optional()
    }

    pub fn list_tasks(&self, limit: i64) -> rusqlite::Result<Vec<TaskRow>> {
        let conn = self.conn();
        let mut stmt =
            conn.prepare("SELECT * FROM downloads ORDER BY datetime(updated_at) DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], task_row_from)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Paginated task query. Status filtering and the keyset cursor are applied
    /// in SQL *before* the limit, so a filtered page cannot be starved by newer
    /// non-matching rows. The cursor is the `"{updated_at}|{id}"` key of the
    /// last row of the previous page.
    pub fn list_tasks_page(
        &self,
        limit: i64,
        cursor: Option<(&str, &str)>,
        status: Option<&str>,
    ) -> rusqlite::Result<Vec<TaskRow>> {
        let mut sql = String::from("SELECT * FROM downloads WHERE 1 = 1");
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(status) = status {
            binds.push(rusqlite::types::Value::Text(status.to_string()));
            sql.push_str(&format!(" AND status = ?{}", binds.len()));
        }
        if let Some((updated_at, id)) = cursor {
            binds.push(rusqlite::types::Value::Text(updated_at.to_string()));
            let p1 = binds.len();
            binds.push(rusqlite::types::Value::Text(updated_at.to_string()));
            let p2 = binds.len();
            binds.push(rusqlite::types::Value::Text(id.to_string()));
            let p3 = binds.len();
            sql.push_str(&format!(
                " AND (updated_at < ?{p1} OR (updated_at = ?{p2} AND id < ?{p3}))"
            ));
        }

        sql.push_str(" ORDER BY updated_at DESC, id DESC");
        let p = binds.len() + 1;
        binds.push(rusqlite::types::Value::Integer(limit));
        sql.push_str(&format!(" LIMIT ?{p}"));

        let conn = self.conn();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(binds), task_row_from)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn insert_task(&self, t: &TaskRow) -> rusqlite::Result<()> {
        let conn = self.conn();
        Self::insert_task_on(&conn, t)
    }

    pub(crate) fn insert_browser_task(
        &self,
        task: &TaskRow,
        expected_settings: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> rusqlite::Result<BrowserIntake> {
        let mut conn = self.conn();
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let now = chrono::Utc::now();
        if expires_at <= now {
            return Ok(BrowserIntake::Expired);
        }
        let app_settings: Option<String> = tx
            .query_row("SELECT value_json FROM settings WHERE key = 'app_settings'", [], |row| row.get(0))
            .optional()?;
        if app_settings.as_deref() != Some(expected_settings) {
            return Ok(BrowserIntake::SettingsChanged);
        }

        let oldest = to_rfc3339(now - chrono::TimeDelta::minutes(5));
        let replayed: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM event_log WHERE download_id = ?1 AND event_type = 'browser_intent' AND created_at >= ?2)
                 OR EXISTS(SELECT 1 FROM downloads WHERE id = ?1)",
            params![task.id, oldest],
            |row| row.get(0),
        )?;
        if replayed {
            return Ok(BrowserIntake::Replayed);
        }
        Self::insert_task_on(&tx, task)?;
        tx.execute(
            "DELETE FROM event_log WHERE event_type = 'browser_intent' AND created_at < ?1",
            params![oldest],
        )?;
        tx.execute(
            "INSERT INTO event_log (download_id, event_type, created_at) VALUES (?1, 'browser_intent', ?2)",
            params![task.id, to_rfc3339(now)],
        )?;
        tx.commit()?;
        Ok(BrowserIntake::Accepted)
    }

    /// Read the persisted opt-in from the settings table. The database is the
    /// source of truth, so both the direct insert and the browser-intake path
    /// encrypt consistently without needing a settings argument threaded
    /// through every call site. Malformed/missing settings default to off.
    fn headers_encryption_enabled(conn: &Connection) -> bool {
        conn.query_row(
            "SELECT value_json FROM settings WHERE key = 'app_settings'",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| serde_json::from_str::<crate::settings::Settings>(&raw).ok())
        .map(|settings| settings.encrypt_saved_headers)
        .unwrap_or(false)
    }

    fn insert_task_on(conn: &Connection, t: &TaskRow) -> rusqlite::Result<()> {
        // Encrypt headers at the storage boundary. If encryption is requested
        // but unavailable, fail closed rather than persist readable secrets.
        let stored_headers = if crate::domain::secrets::is_envelope(&t.headers_json) {
            t.headers_json.clone()
        } else if Self::headers_encryption_enabled(conn) {
            crate::domain::secrets::encrypt(t.headers_json.as_bytes())
                .map_err(|_| rusqlite::Error::InvalidQuery)?
        } else {
            t.headers_json.clone()
        };
        conn.execute(
            r#"INSERT INTO downloads (
                 id, queue_id, category_id, original_url, final_url, headers_json,
                 display_name, destination_path, temporary_path, status, total_bytes,
                 received_bytes, etag, last_modified, accept_ranges, content_type,
                 checksum_algorithm, checksum_expected, checksum_actual, checksum_verified,
                 retry_count, max_retries, last_error_code, last_error_message,
                 priority, start_policy, connections, speed_limit, overwrite_policy,
                 created_at, updated_at, completed_at
               ) VALUES (
                 ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                 ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32
               )"#,
            params![
                t.id,
                t.queue_id,
                t.category_id,
                t.original_url,
                t.final_url,
                stored_headers,
                t.display_name,
                t.destination_path,
                t.temporary_path,
                t.status,
                t.total_bytes,
                t.received_bytes,
                t.etag,
                t.last_modified,
                t.accept_ranges,
                t.content_type,
                t.checksum_algorithm,
                t.checksum_expected,
                t.checksum_actual,
                t.checksum_verified,
                t.retry_count,
                t.max_retries,
                t.last_error_code,
                t.last_error_message,
                t.priority,
                t.start_policy,
                t.connections,
                t.speed_limit,
                t.overwrite_policy,
                t.created_at,
                t.updated_at,
                t.completed_at
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_task(
        &self,
        id: &str,
        status: Option<DownloadStatus>,
        received_bytes: Option<i64>,
        total_bytes: Option<Option<i64>>,
        error: Option<(&str, &str)>,
        checksum_actual: Option<Option<String>>,
        checksum_verified: Option<bool>,
        retry_count: Option<i64>,
        final_url: Option<&str>,
        etag: Option<&str>,
        last_modified: Option<&str>,
        accept_ranges: Option<bool>,
        content_type: Option<&str>,
        temporary_path: Option<&str>,
        completed_at: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        let mut sets: Vec<String> = Vec::new();
        let mut binds: Vec<rusqlite::types::Value> = Vec::new();

        if let Some(s) = status {
            sets.push(format!("status = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(s.as_str().to_string()));
        }
        if let Some(v) = received_bytes {
            sets.push(format!("received_bytes = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Integer(v));
        }
        if let Some(v) = total_bytes {
            sets.push(format!("total_bytes = ?{}", binds.len() + 1));
            binds.push(match v {
                Some(n) => rusqlite::types::Value::Integer(n),
                None => rusqlite::types::Value::Null,
            });
        }
        if let Some((code, msg)) = error {
            sets.push(format!("last_error_code = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(code.to_string()));
            sets.push(format!("last_error_message = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(msg.to_string()));
        }
        if let Some(v) = checksum_actual {
            sets.push(format!("checksum_actual = ?{}", binds.len() + 1));
            binds.push(match v {
                Some(s) => rusqlite::types::Value::Text(s),
                None => rusqlite::types::Value::Null,
            });
        }
        if let Some(v) = checksum_verified {
            sets.push(format!("checksum_verified = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Integer(if v { 1 } else { 0 }));
        }
        if let Some(v) = retry_count {
            sets.push(format!("retry_count = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Integer(v));
        }
        if let Some(v) = final_url {
            sets.push(format!("final_url = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }
        if let Some(v) = etag {
            sets.push(format!("etag = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }
        if let Some(v) = last_modified {
            sets.push(format!("last_modified = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }
        if let Some(v) = accept_ranges {
            sets.push(format!("accept_ranges = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Integer(if v { 1 } else { 0 }));
        }
        if let Some(v) = content_type {
            sets.push(format!("content_type = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }
        if let Some(v) = temporary_path {
            sets.push(format!("temporary_path = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }
        if let Some(v) = completed_at {
            sets.push(format!("completed_at = ?{}", binds.len() + 1));
            binds.push(rusqlite::types::Value::Text(v.to_string()));
        }

        if sets.is_empty() {
            return Ok(());
        }
        sets.push(format!("updated_at = ?{}", binds.len() + 1));
        binds.push(rusqlite::types::Value::Text(now));
        binds.push(rusqlite::types::Value::Text(id.to_string()));

        let sql = format!("UPDATE downloads SET {} WHERE id = ?", sets.join(", "));
        rusqlite::Statement::execute(&mut conn.prepare(&sql)?, rusqlite::params_from_iter(binds))?;
        Ok(())
    }

    pub fn set_priority(&self, id: &str, priority: i64) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "UPDATE downloads SET priority = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, priority, now],
        )?;
        Ok(())
    }

    pub fn set_task_status(&self, id: &str, status: DownloadStatus) -> rusqlite::Result<()> {
        self.update_task(
            id,
            Some(status),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn delete_task(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM download_segments WHERE download_id = ?1", params![id])?;
        conn.execute("DELETE FROM event_log WHERE download_id = ?1 AND event_type != 'browser_intent'", params![id])?;
        conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ------------------------------------------------- convenience mutators

    pub fn set_error(&self, id: &str, code: &str, message: &str) -> rusqlite::Result<()> {
        self.update_task(
            id,
            None,
            None,
            None,
            Some((code, message)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn set_retry_count(&self, id: &str, n: i64) -> rusqlite::Result<()> {
        self.update_task(
            id,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(n),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn set_checksum_result(
        &self,
        id: &str,
        actual: Option<String>,
        verified: bool,
    ) -> rusqlite::Result<()> {
        self.update_task(
            id,
            None,
            None,
            None,
            None,
            Some(actual),
            Some(verified),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_transfer_metadata(
        &self,
        id: &str,
        total_bytes: Option<u64>,
        etag: Option<&str>,
        last_modified: Option<&str>,
        accept_ranges: bool,
        content_type: Option<&str>,
        final_url: Option<&str>,
        temporary_path: Option<&str>,
    ) -> rusqlite::Result<()> {
        self.update_task(
            id,
            None,
            None,
            Some(total_bytes.map(|v| v as i64)),
            None,
            None,
            None,
            None,
            final_url,
            etag,
            last_modified,
            Some(accept_ranges),
            content_type,
            temporary_path,
            None,
        )
    }

    pub fn set_received(&self, id: &str, received: u64) -> rusqlite::Result<()> {
        self.update_task(
            id,
            None,
            Some(received as i64),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    // --------------------------------------------------- retention & backup

    /// Delete terminal task *metadata* older than `retention_days`. Files on
    /// disk are never touched. A non-positive value disables cleanup.
    pub fn cleanup_retention(&self, retention_days: i64) -> rusqlite::Result<usize> {
        if retention_days <= 0 {
            return Ok(0);
        }
        let cutoff = to_rfc3339(
            chrono::Utc::now() - chrono::Duration::days(retention_days),
        );
        let conn = self.conn();
        let ids: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT id FROM downloads WHERE status IN ('completed','failed','cancelled')
                   AND datetime(COALESCE(completed_at, updated_at)) < datetime(?1)",
            )?;
            let rows = stmt.query_map(params![cutoff], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut removed = 0;
        for id in ids {
            conn.execute(
                "DELETE FROM download_segments WHERE download_id = ?1",
                params![id],
            )?;
            conn.execute("DELETE FROM event_log WHERE download_id = ?1 AND event_type != 'browser_intent'", params![id])?;
            removed += conn.execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        }
        Ok(removed)
    }

    /// Keep the event log bounded (it is local diagnostics, not surveillance).
    pub fn trim_event_log(&self, max_rows: i64) -> rusqlite::Result<usize> {
        let keep = max_rows.max(1);
        let conn = self.conn();
        Ok(conn.execute(
            "DELETE FROM event_log WHERE id NOT IN
               (SELECT id FROM event_log ORDER BY id DESC LIMIT ?1)
               AND NOT (event_type = 'browser_intent' AND datetime(created_at) >= datetime('now','-5 minutes'))",
            params![keep],
        )?)
    }

    /// Full local database backup. This file stays on the user's disk; unlike
    /// `export_metadata_json` it is not a portable, shareable artifact.
    pub fn backup_database(&self, path: &Path) -> rusqlite::Result<()> {
        let conn = self.conn();
        conn.execute("VACUUM INTO ?1", params![path.to_string_lossy().to_string()])?;
        Ok(())
    }

    /// Validate and sanitize a full backup in a separate database before it is
    /// allowed to replace any live queue. The staging file must be new.
    pub(crate) fn stage_recovery_backup(source: &Path, stage: &Path, startup: bool) -> rusqlite::Result<usize> {
        if !source.is_file() || source.metadata().map_err(|_| rusqlite::Error::InvalidQuery)?.len() > 128 * 1024 * 1024
            || stage.exists()
        { return Err(rusqlite::Error::InvalidQuery); }
        let from = Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let integrity: String = from.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if integrity != "ok" { return Err(rusqlite::Error::InvalidQuery); }
        // Compare with the canonical migrated schema so every runtime field is
        // required, including fields which are absent from an empty task list.
        let reference = Store { conn: Mutex::new(Connection::open_in_memory()?), path: PathBuf::new() };
        reference.migrate()?;
        let schema = reference.conn();
        for table in ["downloads", "queues", "categories", "download_segments", "schedules", "settings", "event_log"] {
            let kind: String = from.query_row("SELECT type FROM sqlite_master WHERE name=?1", [table], |row| row.get(0))?;
            if kind != "table" { return Err(rusqlite::Error::InvalidQuery); }
            let query = format!("SELECT * FROM {table} LIMIT 0");
            let required = schema.prepare(&query)?;
            let actual = from.prepare(&query)?;
            if required.column_names().iter().any(|name| !actual.column_names().contains(name)) {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }
        if from.prepare("PRAGMA foreign_key_check")?.exists([])?
            || from.prepare("SELECT 1 FROM sqlite_master WHERE type='trigger'")?.exists([])?
        { return Err(rusqlite::Error::InvalidQuery); }
        let mut tasks = from.prepare("SELECT * FROM downloads")?;
        for task in tasks.query_map([], task_row_from)? {
            let task = task?;
            if DownloadStatus::parse(&task.status).is_none() { return Err(rusqlite::Error::InvalidQuery); }
        }
        let mut destination = Connection::open(stage)?;
        rusqlite::backup::Backup::new(&from, &mut destination)?.run_to_completion(64, std::time::Duration::from_millis(5), None)?;
        let raw: Option<String> = destination.query_row("SELECT value_json FROM settings WHERE key='app_settings'", [], |row| row.get(0)).optional()?;
        let mut settings = match raw {
            Some(raw) => serde_json::from_str::<crate::settings::Settings>(&raw).map_err(|_| rusqlite::Error::InvalidQuery)?,
            None => crate::settings::Settings::default(),
        }.normalized();
        // Restoring data does not grant clipboard, browser, power or startup
        // consent, and cannot silently alter Windows registration.
        settings.shutdown_after_complete = false;
        settings.clipboard_capture = false;
        settings.browser_integration = false;
        settings.start_on_login = startup;
        settings.scan_with_antivirus = false;
        settings.scanner_command.clear();
        settings.overwrite_policy = OverwritePolicy::Rename;
        settings.keep_partial_on_failure = true;
        if crate::build_http_client(&settings.proxy).is_err() { settings.proxy.clear(); }
        destination.execute("INSERT OR REPLACE INTO settings(key,value_json,updated_at)VALUES('app_settings',?1,?2)",
            params![settings.to_json(), to_rfc3339(chrono::Utc::now())])?;
        destination.execute("UPDATE downloads SET status='paused',start_policy='manual' WHERE status NOT IN ('completed','failed','cancelled')", [])?;
        destination.execute("UPDATE downloads SET overwrite_policy='rename'", [])?;
        destination.execute("UPDATE schedules SET enabled=0", [])?;
        destination.pragma_update(None, "journal_mode", "DELETE")?;
        let count = destination.query_row("SELECT count(*) FROM downloads", [], |row| row.get::<_, i64>(0))?;
        Ok(count.max(0) as usize)
    }

    pub(crate) fn restore_database(&self, source: &Path, startup: bool) -> rusqlite::Result<(usize, PathBuf)> {
        let folder = self.path.parent().unwrap_or_else(|| Path::new(".")).join("backups");
        std::fs::create_dir_all(&folder).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let stage = folder.join(format!("validated-{}.sqlite", uuid::Uuid::new_v4()));
        let count = Self::stage_recovery_backup(source, &stage, startup)?;
        let safety = folder.join(format!("before-restore-{}.sqlite", uuid::Uuid::new_v4()));
        let staged = Connection::open_with_flags(&stage, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut connection = self.conn();
        // Hold the queue mutex across the safety snapshot and activation: no
        // concurrent command can add data between the two operations.
        connection.execute("VACUUM INTO ?1", params![safety.to_string_lossy().as_ref()])?;
        let activated = (|| -> rusqlite::Result<()> {
            let backup = rusqlite::backup::Backup::new(&staged, &mut connection)?;
            backup.run_to_completion(64, std::time::Duration::from_millis(5), None)
        })();
        if let Err(error) = activated {
            // The safety snapshot exists: make a best-effort restore from it so
            // a failed mid-copy cannot leave the live queue half-replaced.
            if let Ok(snapshot) = Connection::open_with_flags(&safety, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) {
                if let Ok(restore) = rusqlite::backup::Backup::new(&snapshot, &mut connection) {
                    let _ = restore.run_to_completion(64, std::time::Duration::from_millis(5), None);
                }
            }
            return Err(error);
        }
        connection.pragma_update(None, "foreign_keys", "ON")?;
        Ok((count, safety))
    }

    /// Export queue metadata without request headers, cookies, credentials,
    /// full URLs or error bodies. Hostnames are kept (scheme + host only) so a
    /// queue can be recognised after import.
    pub fn export_metadata_json(&self) -> rusqlite::Result<String> {
        use serde_json::json;
        let conn = self.conn();

        let queues = {
            let mut stmt = conn.prepare(
                "SELECT id, name, enabled, max_active_tasks FROM queues ORDER BY created_at",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "enabled": r.get::<_, i64>(2)? != 0,
                    "max_active_tasks": r.get::<_, i64>(3)?,
                }))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let categories = {
            let mut stmt = conn.prepare(
                "SELECT id, name, destination_template, extension_rules_json
                   FROM categories ORDER BY created_at",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "destination_template": r.get::<_, String>(2)?,
                    "extension_rules_json": r.get::<_, String>(3)?,
                }))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let schedules = {
            let mut stmt = conn.prepare(
                "SELECT id, queue_id, days_mask, start_local_time, end_local_time,
                        action_after, enabled FROM schedules ORDER BY rowid",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "queue_id": r.get::<_, String>(1)?,
                    "days_mask": r.get::<_, i64>(2)?,
                    "start_local_time": r.get::<_, String>(3)?,
                    "end_local_time": r.get::<_, String>(4)?,
                    "action_after": r.get::<_, String>(5)?,
                    "enabled": r.get::<_, i64>(6)? != 0,
                }))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let tasks = {
            let mut stmt = conn.prepare("SELECT * FROM downloads ORDER BY created_at")?;
            let rows = stmt.query_map([], |r| {
                let row = task_row_from(r)?;
                let host = url::Url::parse(&row.original_url).ok().and_then(|u| {
                    u.host_str().map(|h| format!("{}://{}", u.scheme(), h))
                });
                Ok(json!({
                    "id": row.id,
                    "queue_id": row.queue_id,
                    "category_id": row.category_id,
                    "display_name": row.display_name,
                    "destination_path": row.destination_path,
                    "status": row.status,
                    "total_bytes": row.total_bytes,
                    "received_bytes": row.received_bytes,
                    "checksum_algorithm": row.checksum_algorithm,
                    "checksum_expected": row.checksum_expected,
                    "checksum_actual": row.checksum_actual,
                    "checksum_verified": row.checksum_verified,
                    "retry_count": row.retry_count,
                    "max_retries": row.max_retries,
                    "priority": row.priority,
                    "start_policy": row.start_policy,
                    "connections": row.connections,
                    "overwrite_policy": row.overwrite_policy,
                    "created_at": row.created_at,
                    "updated_at": row.updated_at,
                    "completed_at": row.completed_at,
                    "host": host,
                }))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let segments = {
            let mut stmt = conn.prepare(
                "SELECT download_id, segment_index, start_offset, end_offset,
                        received_bytes, status FROM download_segments ORDER BY download_id, segment_index",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(json!({
                    "download_id": r.get::<_, String>(0)?,
                    "segment_index": r.get::<_, i64>(1)?,
                    "start_offset": r.get::<_, i64>(2)?,
                    "end_offset": r.get::<_, i64>(3)?,
                    "received_bytes": r.get::<_, i64>(4)?,
                    "status": r.get::<_, String>(5)?,
                }))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        Ok(json!({
            "protocol": crate::domain::PROTOCOL,
            "exported_at": to_rfc3339(chrono::Utc::now()),
            "queues": queues,
            "categories": categories,
            "schedules": schedules,
            "tasks": tasks,
            "segments": segments,
        })
        .to_string())
    }

    /// Restore a metadata export. Inserts are idempotent (`INSERT OR IGNORE`),
    /// secrets cannot be reintroduced because the export never contains them.
    pub fn import_metadata_json(&self, raw: &str) -> rusqlite::Result<usize> {
        let doc: serde_json::Value =
            serde_json::from_str(raw).map_err(|_| rusqlite::Error::InvalidQuery)?;
        if raw.len() > 8 * 1024 * 1024 || doc["protocol"].as_u64() != Some(crate::domain::PROTOCOL as u64) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        for key in ["queues", "categories", "tasks", "schedules", "segments"] {
            let Some(rows) = doc[key].as_array() else { return Err(rusqlite::Error::InvalidQuery) };
            if rows.len() > 50_000 { return Err(rusqlite::Error::InvalidQuery); }
        }
        let mut guard = self.conn();
        let transaction = guard.transaction()?;
        let conn = &transaction;
        let now = to_rfc3339(chrono::Utc::now());
        let mut restored = 0;

        if let Some(queues) = doc.get("queues").and_then(|v| v.as_array()) {
            for q in queues {
                restored += conn.execute(
                    "INSERT OR IGNORE INTO queues
                       (id, name, enabled, max_active_tasks, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![
                        q["id"].as_str().unwrap_or(""),
                        q["name"].as_str().unwrap_or("Imported"),
                        0, // Imported queues require explicit review before use.
                        q["max_active_tasks"].as_i64().unwrap_or(3).clamp(1, 16),
                        now,
                    ],
                )?;
            }
        }
        if let Some(cats) = doc.get("categories").and_then(|v| v.as_array()) {
            for c in cats {
                restored += conn.execute(
                    "INSERT OR IGNORE INTO categories
                       (id, name, destination_template, extension_rules_json, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![
                        c["id"].as_str().unwrap_or(""),
                        c["name"].as_str().unwrap_or("Imported"),
                        c["destination_template"].as_str().unwrap_or("."),
                        c["extension_rules_json"].as_str().unwrap_or("[]"),
                        now,
                    ],
                )?;
            }
        }
        if let Some(tasks) = doc.get("tasks").and_then(|v| v.as_array()) {
            for t in tasks {
                let id = t["id"].as_str().unwrap_or("");
                if id.is_empty() {
                    continue;
                }
                restored += conn.execute(
                    r#"INSERT OR IGNORE INTO downloads (
                         id, queue_id, category_id, original_url, headers_json,
                         display_name, destination_path, status, total_bytes,
                         received_bytes, checksum_algorithm, checksum_expected,
                         checksum_actual, checksum_verified, retry_count, max_retries,
                         priority, start_policy, connections, overwrite_policy,
                         created_at, updated_at, completed_at
                       ) VALUES (?1, ?2, ?3, ?4, '[]', ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                                 ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"#,
                    params![
                        id,
                        t["queue_id"].as_str().unwrap_or("default"),
                        t["category_id"].as_str(),
                        // Deliberately synthetic: the real URL may carry tokens.
                        "https://import.invalid/metadata",
                        t["display_name"].as_str().unwrap_or("download"),
                        t["destination_path"].as_str().unwrap_or("."),
                        if t["status"].as_str() == Some("completed") { "completed" } else { "paused" },
                        t["total_bytes"].as_i64(),
                        t["received_bytes"].as_i64().unwrap_or(0),
                        t["checksum_algorithm"].as_str(),
                        t["checksum_expected"].as_str(),
                        t["checksum_actual"].as_str(),
                        0, // Imported assertions are not integrity evidence.
                        t["retry_count"].as_i64().unwrap_or(0),
                        t["max_retries"].as_i64().unwrap_or(3),
                        t["priority"].as_i64().unwrap_or(50).clamp(0, 100),
                        "manual",
                        t["connections"].as_i64().unwrap_or(1).clamp(1, 16),
                        "rename",
                        t["created_at"].as_str().unwrap_or(&now),
                        t["updated_at"].as_str().unwrap_or(&now),
                        t["completed_at"].as_str(),
                    ],
                )?;
            }
        }
        if let Some(schedules) = doc.get("schedules").and_then(|v| v.as_array()) {
            for s in schedules {
                restored += conn.execute(
                    "INSERT OR IGNORE INTO schedules
                       (id, queue_id, days_mask, start_local_time, end_local_time, action_after, enabled)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        s["id"].as_str().unwrap_or(""),
                        s["queue_id"].as_str().unwrap_or("default"),
                        s["days_mask"].as_i64().unwrap_or(127),
                        s["start_local_time"].as_str().unwrap_or("00:00"),
                        s["end_local_time"].as_str().unwrap_or("23:59"),
                        "nothing",
                        0,
                    ],
                )?;
            }
        }
        if let Some(segments) = doc.get("segments").and_then(|v| v.as_array()) {
            for s in segments {
                let download_id = s["download_id"].as_str().unwrap_or("");
                let index = s["segment_index"].as_i64().unwrap_or(-1);
                restored += conn.execute(
                    "INSERT OR IGNORE INTO download_segments
                       (id, download_id, segment_index, start_offset, end_offset, received_bytes, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        format!("{download_id}-{index}"),
                        download_id,
                        index,
                        s["start_offset"].as_i64().unwrap_or(0),
                        s["end_offset"].as_i64().unwrap_or(0),
                        s["received_bytes"].as_i64().unwrap_or(0),
                        s["status"].as_str().unwrap_or("pending"),
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(restored)
    }

    /// Flip the `encrypt_saved_headers` setting and rewrite every
    /// `headers_json` cell to match, atomically. Any failure (unavailable OS
    /// store, undecryptable source row, DB error) rolls the transaction back,
    /// leaving BOTH the setting and all rows exactly as they were.
    ///
    /// `enable = true`  : plaintext rows -> envelopes; already-encrypted rows
    ///                    are left alone (idempotent).
    /// `enable = false` : envelopes -> plaintext; already-plaintext rows are
    ///                    left alone. An undecryptable envelope aborts rather
    ///                    than dropping data.
    pub fn encrypt_saved_secrets(&self, enable: bool) -> rusqlite::Result<SecretMigrationResult> {
        use crate::domain::secrets;

        if enable && !secrets::is_available() {
            // Never pretend encryption happened on a platform without DPAPI.
            return Err(rusqlite::Error::InvalidQuery);
        }

        let mut conn = self.conn();
        let tx = conn.transaction()?;

        let raw: Option<String> = tx
            .query_row(
                "SELECT value_json FROM settings WHERE key = 'app_settings'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let mut settings = match raw {
            Some(raw) => crate::settings::Settings::from_json(&raw),
            None => crate::settings::Settings::default(),
        };

        let rows: Vec<(String, String)> = {
            let mut stmt = tx.prepare("SELECT id, headers_json FROM downloads")?;
            let mapped = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?
                        .unwrap_or_else(|| "[]".to_string()),
                ))
            })?;
            mapped.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let mut rewritten = 0usize;
        for (id, value) in &rows {
            let next = if enable {
                if secrets::is_envelope(value) {
                    continue;
                }
                // Fail closed: refuse to migrate if protection cannot be made.
                secrets::encrypt(value.as_bytes()).map_err(|_| rusqlite::Error::InvalidQuery)?
            } else {
                if !secrets::is_envelope(value) {
                    continue;
                }
                // Never silently discard a row we cannot decrypt.
                secrets::decrypt(value).map_err(|_| rusqlite::Error::InvalidQuery)?
            };
            tx.execute(
                "UPDATE downloads SET headers_json = ?2 WHERE id = ?1",
                params![id, next],
            )?;
            rewritten += 1;
        }

        settings.encrypt_saved_headers = enable;
        let now = to_rfc3339(chrono::Utc::now());
        tx.execute(
            "INSERT INTO settings (key, value_json, updated_at) VALUES ('app_settings', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_json = ?1, updated_at = ?2",
            params![settings.to_json(), now],
        )?;
        tx.commit()?;

        Ok(SecretMigrationResult { enabled: enable, rows_rewritten: rewritten })
    }

    pub fn set_segments_summary(
        &self,
        id: &str,
        segment_count: u32,
        completed: u32,
    ) -> rusqlite::Result<()> {
        let conn = self.conn();
        let now = to_rfc3339(chrono::Utc::now());
        conn.execute(
            "UPDATE downloads SET updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )?;
        // Stored on the segment rows; the projection derives the counts.
        let _ = (segment_count, completed);
        Ok(())
    }
}

/// Transparently reverse `insert_task_on` for readers. Legacy plaintext is
/// returned unchanged. A recognizable but undecryptable envelope fails closed:
/// the task loads with no headers and never with ciphertext or a panic. We
/// deliberately do not log the value (it may be a secret).
fn decrypt_headers_for_read(raw: &str) -> String {
    if !crate::domain::secrets::is_envelope(raw) {
        return raw.to_string();
    }
    match crate::domain::secrets::decrypt(raw) {
        Ok(plaintext) => plaintext,
        Err(_) => "[]".to_string(),
    }
}

fn task_row_from(r: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: r.get("id")?,
        queue_id: r.get("queue_id")?,
        category_id: r.get("category_id")?,
        original_url: r.get("original_url")?,
        final_url: r.get("final_url")?,
        headers_json: decrypt_headers_for_read(
            &r.get::<_, Option<String>>("headers_json")?
                .unwrap_or_else(|| "[]".to_string()),
        ),
        display_name: r.get("display_name")?,
        destination_path: r.get("destination_path")?,
        temporary_path: r.get("temporary_path")?,
        status: r.get("status")?,
        total_bytes: r.get("total_bytes")?,
        received_bytes: r.get("received_bytes")?,
        etag: r.get("etag")?,
        last_modified: r.get("last_modified")?,
        accept_ranges: r.get("accept_ranges")?,
        content_type: r.get("content_type")?,
        checksum_algorithm: r.get("checksum_algorithm")?,
        checksum_expected: r.get("checksum_expected")?,
        checksum_actual: r.get("checksum_actual")?,
        checksum_verified: r.get("checksum_verified")?,
        retry_count: r.get("retry_count")?,
        max_retries: r.get("max_retries")?,
        last_error_code: r.get("last_error_code")?,
        last_error_message: r.get("last_error_message")?,
        priority: r.get("priority")?,
        start_policy: r.get("start_policy")?,
        connections: r.get("connections")?,
        speed_limit: r.get("speed_limit")?,
        overwrite_policy: r.get("overwrite_policy")?,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
        completed_at: r.get("completed_at")?,
    })
}

impl TaskRow {
    pub fn status_enum(&self) -> DownloadStatus {
        DownloadStatus::parse(&self.status).unwrap_or(DownloadStatus::Failed)
    }

    pub fn start_policy_enum(&self) -> StartPolicy {
        StartPolicy::parse(&self.start_policy).unwrap_or(StartPolicy::Immediate)
    }

    pub fn overwrite_enum(&self) -> OverwritePolicy {
        OverwritePolicy::parse(&self.overwrite_policy).unwrap_or_default()
    }

    pub fn to_projection(&self) -> DownloadProjection {
        let status = self.status_enum();
        DownloadProjection {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            status,
            bytes_received: self.received_bytes.max(0) as u64,
            total_bytes: self.total_bytes.map(|v| v.max(0) as u64),
            bytes_per_second: 0,
            eta_seconds: None,
            queue_id: self.queue_id.clone(),
            category_id: self.category_id.clone(),
            priority: self.priority,
            can_resume: matches!(
                status,
                DownloadStatus::Paused | DownloadStatus::Failed | DownloadStatus::RetryWait
            ) && self.temporary_path.is_some(),
            error_code: self.last_error_code.clone(),
            error_message: self.last_error_message.clone(),
            original_url: self.original_url.clone(),
            final_url: self.final_url.clone(),
            destination_path: self.destination_path.clone(),
            temporary_path: self.temporary_path.clone(),
            content_type: self.content_type.clone(),
            checksum_algorithm: self.checksum_algorithm.clone(),
            checksum_expected: self.checksum_expected.clone(),
            checksum_actual: self.checksum_actual.clone(),
            checksum_verified: self.checksum_verified,
            retry_count: self.retry_count,
            max_retries: self.max_retries,
            connections: self.connections.max(0) as u32,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            completed_at: self.completed_at.clone(),
            segment_count: 0,
            completed_segments: 0,
        }
    }

    pub fn created_at_dt(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        parse_rfc3339(&self.created_at)
    }
}
