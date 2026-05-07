use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::db::Database;
use crate::models::JobRecord;

const JOB_HISTORY_RETENTION_DAYS: i64 = 7;
const JOB_HISTORY_CLEANUP_INTERVAL_MS: i64 = 60 * 60 * 1000;

pub struct SqliteJobHistoryStore {
    db: Database,
    last_cleanup_ms: Option<i64>,
}

impl SqliteJobHistoryStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let db = Database::open(path)?;
        ensure_job_history_columns(db.connection())?;
        let mut store = Self {
            db,
            last_cleanup_ms: None,
        };
        store.purge_expired_jobs()?;
        store.mark_incomplete_jobs_as_interrupted()?;
        Ok(store)
    }

    pub fn list_recent_jobs(&self, limit: usize) -> anyhow::Result<Vec<JobRecord>> {
        let mut statement = self.db.connection().prepare(
            "SELECT id, kind, status, message, started_at, updated_at, ended_at,
                    duration_ms, input_params_json, result_summary, error_details
             FROM jobs
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], map_job_row)?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn next_job_id(&self) -> anyhow::Result<i64> {
        Ok(self.db.connection().query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM jobs",
            [],
            |row| row.get(0),
        )?)
    }

    pub fn upsert_job(&mut self, job: &JobRecord) -> anyhow::Result<()> {
        self.purge_expired_jobs_if_due()?;
        self.db.connection().execute(
            "INSERT INTO jobs (
              id, kind, status, message, started_at, updated_at, ended_at,
              duration_ms, input_params_json, result_summary, error_details
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
              kind = excluded.kind,
              status = excluded.status,
              message = excluded.message,
              started_at = excluded.started_at,
              updated_at = excluded.updated_at,
              ended_at = excluded.ended_at,
              duration_ms = excluded.duration_ms,
              input_params_json = excluded.input_params_json,
              result_summary = excluded.result_summary,
              error_details = excluded.error_details",
            rusqlite::params![
                job.id,
                job.kind,
                job.status,
                job.message,
                job.started_at,
                job.updated_at,
                job.ended_at,
                job.duration_ms,
                job.input_params_json,
                job.result_summary,
                job.error_details,
            ],
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub fn insert_job_for_tests(&mut self, job: &JobRecord) -> anyhow::Result<()> {
        self.db.connection().execute(
            "INSERT INTO jobs (
              id, kind, status, message, started_at, updated_at, ended_at,
              duration_ms, input_params_json, result_summary, error_details
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                job.id,
                job.kind,
                job.status,
                job.message,
                job.started_at,
                job.updated_at,
                job.ended_at,
                job.duration_ms,
                job.input_params_json,
                job.result_summary,
                job.error_details,
            ],
        )?;
        Ok(())
    }

    fn purge_expired_jobs_if_due(&mut self) -> anyhow::Result<()> {
        let now_ms = current_utc_millis();
        if self.last_cleanup_ms.is_some_and(|last_cleanup_ms| {
            now_ms.saturating_sub(last_cleanup_ms) < JOB_HISTORY_CLEANUP_INTERVAL_MS
        }) {
            return Ok(());
        }

        self.purge_expired_jobs()?;
        Ok(())
    }

    fn purge_expired_jobs(&mut self) -> anyhow::Result<usize> {
        let cutoff_ms =
            current_utc_millis().saturating_sub(JOB_HISTORY_RETENTION_DAYS * 24 * 60 * 60 * 1000);
        let mut statement = self
            .db
            .connection()
            .prepare("SELECT id, started_at FROM jobs")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut expired_ids = Vec::new();
        for row in rows {
            let (id, started_at) = row?;
            if parse_job_timestamp_millis(&started_at)
                .is_some_and(|started_at_ms| started_at_ms < cutoff_ms)
            {
                expired_ids.push(id);
            }
        }

        for id in &expired_ids {
            self.db
                .connection()
                .execute("DELETE FROM jobs WHERE id = ?1", [id])?;
        }

        self.last_cleanup_ms = Some(current_utc_millis());
        Ok(expired_ids.len())
    }

    fn mark_incomplete_jobs_as_interrupted(&mut self) -> anyhow::Result<()> {
        let now = current_job_timestamp();
        let mut statement = self.db.connection().prepare(
            "SELECT id, kind, started_at
             FROM jobs
             WHERE status IN ('running', 'queued')",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        for row in rows {
            let (id, kind, started_at) = row?;
            if kind == crate::jobs::kinds::PAPER_ORDER {
                continue;
            }
            self.db.connection().execute(
                "UPDATE jobs
                 SET status = 'cancelled',
                     updated_at = ?2,
                     ended_at = ?2,
                     duration_ms = ?3,
                     error_details = ?4
                 WHERE id = ?1",
                rusqlite::params![
                    id,
                    now,
                    duration_between(&started_at, &now),
                    "Interrupted by app restart",
                ],
            )?;
        }

        Ok(())
    }
}

fn map_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    Ok(JobRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        status: row.get(2)?,
        message: row.get(3)?,
        started_at: row.get(4)?,
        updated_at: row.get(5)?,
        ended_at: row.get(6)?,
        duration_ms: row.get(7)?,
        input_params_json: row.get(8)?,
        result_summary: row.get(9)?,
        error_details: row.get(10)?,
    })
}

fn ensure_job_history_columns(connection: &rusqlite::Connection) -> anyhow::Result<()> {
    let columns = table_columns(connection, "jobs")?;

    if !columns.contains("ended_at") {
        connection.execute("ALTER TABLE jobs ADD COLUMN ended_at TEXT", [])?;
    }
    if !columns.contains("duration_ms") {
        connection.execute("ALTER TABLE jobs ADD COLUMN duration_ms INTEGER", [])?;
    }
    if !columns.contains("input_params_json") {
        connection.execute("ALTER TABLE jobs ADD COLUMN input_params_json TEXT", [])?;
    }
    if !columns.contains("result_summary") {
        connection.execute("ALTER TABLE jobs ADD COLUMN result_summary TEXT", [])?;
    }
    if !columns.contains("error_details") {
        connection.execute("ALTER TABLE jobs ADD COLUMN error_details TEXT", [])?;
    }

    Ok(())
}

fn table_columns(
    connection: &rusqlite::Connection,
    table_name: &str,
) -> anyhow::Result<HashSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;

    Ok(rows.collect::<Result<HashSet<_>, _>>()?)
}

pub(crate) fn current_job_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| fallback_epoch_timestamp())
}

pub(crate) fn duration_between(started_at: &str, ended_at: &str) -> Option<i64> {
    let started_at_ms = parse_job_timestamp_millis(started_at)?;
    let ended_at_ms = parse_job_timestamp_millis(ended_at)?;
    Some(ended_at_ms.saturating_sub(started_at_ms))
}

pub(crate) fn parse_job_timestamp_millis(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if let Some(stripped) = trimmed.strip_prefix("epoch:") {
        return stripped.parse::<i64>().ok().map(normalize_epoch_millis);
    }

    if let Ok(number) = trimmed.parse::<i64>() {
        return Some(normalize_epoch_millis(number));
    }

    OffsetDateTime::parse(trimmed, &Rfc3339)
        .ok()
        .map(|timestamp| (timestamp.unix_timestamp_nanos() / 1_000_000) as i64)
}

fn normalize_epoch_millis(value: i64) -> i64 {
    if value < 1_000_000_000_000 {
        value.saturating_mul(1000)
    } else {
        value
    }
}

fn current_utc_millis() -> i64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

fn fallback_epoch_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("epoch:{seconds}")
}
