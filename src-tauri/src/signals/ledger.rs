use super::UnifiedSignal;
use crate::db::Database;
use crate::signals::config::{StrategyConfig, ACTIVE_STRATEGY_IDS};
use crate::signals::stats::{ScanRunRecord, StrategyStats};
use rusqlite::{params, OptionalExtension};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SignalRecord {
    pub signal: UnifiedSignal,
    pub risk_result: Option<String>,
    pub executed: bool,
    pub modified: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignalEvaluation {
    pub evaluation_id: String,
    pub signal_id: String,
    pub horizon: String,
    pub price_at_horizon: f64,
    pub pnl_percent: f64,
    pub result: String,
    pub evaluated_at: String,
}

#[derive(Clone)]
pub struct SignalLedger {
    path: PathBuf,
}

impl SignalLedger {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let db = Database::open(&path)?;
        db.connection().execute_batch(
            "CREATE TABLE IF NOT EXISTS signals (
                signal_id TEXT PRIMARY KEY,
                symbol TEXT NOT NULL,
                market_type TEXT NOT NULL,
                direction TEXT NOT NULL,
                score REAL NOT NULL,
                strength REAL NOT NULL,
                contributors TEXT NOT NULL,
                category_breakdown TEXT NOT NULL,
                entry_zone_low REAL,
                entry_zone_high REAL,
                stop_loss REAL,
                take_profit REAL,
                reason_summary TEXT NOT NULL,
                risk_status TEXT NOT NULL,
                risk_result TEXT NOT NULL,
                executed INTEGER NOT NULL DEFAULT 0,
                modified INTEGER NOT NULL DEFAULT 0,
                generated_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS signal_evaluations (
                evaluation_id TEXT PRIMARY KEY,
                signal_id TEXT NOT NULL,
                horizon TEXT NOT NULL,
                price_at_horizon REAL,
                pnl_percent REAL,
                result TEXT,
                evaluated_at TEXT NOT NULL,
                FOREIGN KEY (signal_id) REFERENCES signals(signal_id)
            );

            CREATE TABLE IF NOT EXISTS signal_user_actions (
                action_id TEXT PRIMARY KEY,
                signal_id TEXT NOT NULL,
                action_type TEXT NOT NULL,
                payload TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS strategy_config (
                strategy_id TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1,
                params_json TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS signal_strategies (
                signal_id TEXT NOT NULL,
                strategy_id TEXT NOT NULL,
                PRIMARY KEY (signal_id, strategy_id)
            );

            CREATE TABLE IF NOT EXISTS scan_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                symbols_scanned INTEGER NOT NULL DEFAULT 0,
                signals_found INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER,
                status TEXT NOT NULL DEFAULT 'running',
                error TEXT
            );",
        )?;

        for id in &ACTIVE_STRATEGY_IDS {
            db.connection().execute(
                "INSERT OR IGNORE INTO strategy_config (strategy_id, enabled, params_json) VALUES (?1, 1, '{}')",
                [id],
            )?;
        }

        Ok(Self { path })
    }

    pub fn insert_signal(&self, signal: &UnifiedSignal, risk_result: &str) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "INSERT OR REPLACE INTO signals (
                signal_id, symbol, market_type, direction, score, strength,
                contributors, category_breakdown,
                entry_zone_low, entry_zone_high, stop_loss, take_profit,
                reason_summary, risk_status, risk_result, generated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                signal.signal_id,
                signal.symbol,
                signal.market_type,
                serde_json::to_string(&signal.direction)?,
                signal.score,
                signal.strength,
                serde_json::to_string(&signal.contributors)?,
                serde_json::to_string(&signal.category_breakdown)?,
                signal.entry_zone_low,
                signal.entry_zone_high,
                signal.stop_loss,
                signal.take_profit,
                signal.reason_summary,
                signal.risk_status,
                risk_result,
                signal.generated_at,
            ],
        )?;

        // Insert strategy contributors into join table
        for contributor in &signal.contributors {
            db.connection().execute(
                "INSERT OR IGNORE INTO signal_strategies (signal_id, strategy_id) VALUES (?1, ?2)",
                rusqlite::params![signal.signal_id, contributor],
            )?;
        }

        Ok(())
    }

    pub fn mark_executed(&self, signal_id: &str) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "UPDATE signals SET executed = 1 WHERE signal_id = ?1",
            params![signal_id],
        )?;
        Ok(())
    }

    pub fn list_signals(&self, limit: usize) -> anyhow::Result<Vec<SignalRecord>> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT signal_id, symbol, market_type, direction, score, strength,
                    contributors, category_breakdown,
                    entry_zone_low, entry_zone_high, stop_loss, take_profit,
                    reason_summary, risk_status, risk_result, generated_at,
                    executed, modified
             FROM signals ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            let signal_id: String = row.get(0)?;
            let direction_str: String = row.get(3)?;
            let direction: super::strategies::SignalDirection =
                serde_json::from_str(&direction_str)
                    .or_else(|_| {
                        serde_json::from_str(&format!("\"{}\"", direction_str.trim_matches('"')))
                    })
                    .unwrap_or(super::strategies::SignalDirection::Neutral);
            let contributors_str: String = row.get(6)?;
            let breakdown_str: String = row.get(7)?;
            Ok(SignalRecord {
                signal: UnifiedSignal {
                    signal_id: signal_id.clone(),
                    symbol: row.get(1)?,
                    market_type: row.get(2)?,
                    direction,
                    score: row.get(4)?,
                    strength: row.get(5)?,
                    contributors: serde_json::from_str(&contributors_str).unwrap_or_default(),
                    category_breakdown: serde_json::from_str(&breakdown_str).unwrap_or_default(),
                    entry_zone_low: row.get(8)?,
                    entry_zone_high: row.get(9)?,
                    stop_loss: row.get(10)?,
                    take_profit: row.get(11)?,
                    reason_summary: row.get(12)?,
                    risk_status: row.get(13)?,
                    generated_at: row.get(15)?,
                },
                risk_result: row.get(14)?,
                executed: row.get::<_, i64>(16)? != 0,
                modified: row.get::<_, i64>(17)? != 0,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_evaluations(&self, signal_id: &str) -> anyhow::Result<Vec<SignalEvaluation>> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT evaluation_id, signal_id, horizon, price_at_horizon, pnl_percent, result, evaluated_at
             FROM signal_evaluations WHERE signal_id = ?1 ORDER BY evaluated_at ASC"
        )?;
        let rows = statement.query_map(params![signal_id], |row| {
            Ok(SignalEvaluation {
                evaluation_id: row.get(0)?,
                signal_id: row.get(1)?,
                horizon: row.get(2)?,
                price_at_horizon: row.get(3)?,
                pnl_percent: row.get(4)?,
                result: row.get(5)?,
                evaluated_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn insert_evaluations(&self, evaluations: &[SignalEvaluation]) -> anyhow::Result<()> {
        if evaluations.is_empty() {
            return Ok(());
        }
        let db = Database::open(&self.path)?;
        for e in evaluations {
            db.connection().execute(
                "INSERT OR IGNORE INTO signal_evaluations (
                    evaluation_id, signal_id, horizon, price_at_horizon, pnl_percent, result, evaluated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![e.evaluation_id, e.signal_id, e.horizon, e.price_at_horizon, e.pnl_percent, e.result, e.evaluated_at],
            )?;
        }
        Ok(())
    }

    pub fn append_user_action(
        &self,
        signal_id: &str,
        action_type: &str,
        payload: &str,
    ) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        let created_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| String::new());
        let action_id = format!(
            "sigaction-{}-{}",
            signal_id,
            time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
        );
        db.connection().execute(
            "INSERT INTO signal_user_actions (action_id, signal_id, action_type, payload, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![action_id, signal_id, action_type, payload, created_at],
        )?;
        Ok(())
    }

    pub fn count_signals_today(&self) -> anyhow::Result<u32> {
        let db = Database::open(&self.path)?;
        let count: i64 = db.connection().query_row(
            "SELECT COUNT(*) FROM signals WHERE date(created_at) = date('now', 'localtime') AND risk_status = 'approved' AND executed = 1",
            [], |row| row.get(0))?;
        Ok(count.max(0) as u32)
    }

    pub fn last_signal_direction_timestamp(
        &self,
        symbol: &str,
        direction: &str,
    ) -> anyhow::Result<Option<i64>> {
        let db = Database::open(&self.path)?;
        let ts: Option<i64> = db
            .connection()
            .query_row(
                "SELECT MAX(CAST(strftime('%s', generated_at) AS INTEGER))
             FROM signals WHERE symbol = ?1 AND direction = ?2",
                params![symbol, direction],
                |row| row.get(0),
            )
            .optional()?;
        Ok(ts)
    }

    pub fn delete_signal(&self, signal_id: &str) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "DELETE FROM signal_user_actions WHERE signal_id = ?1",
            params![signal_id],
        )?;
        db.connection().execute(
            "DELETE FROM signal_evaluations WHERE signal_id = ?1",
            params![signal_id],
        )?;
        db.connection().execute(
            "DELETE FROM signals WHERE signal_id = ?1",
            params![signal_id],
        )?;
        db.connection().execute(
            "DELETE FROM signal_strategies WHERE signal_id = ?1",
            params![signal_id],
        )?;
        Ok(())
    }

    pub fn strategy_stats(&self) -> anyhow::Result<Vec<StrategyStats>> {
        let db = Database::open(&self.path)?;
        let mut stmt = db.connection().prepare(
            "SELECT ss.strategy_id,
                    COUNT(*) as total_signals,
                    SUM(CASE WHEN s.direction LIKE '%Buy%' THEN 1 ELSE 0 END) as buy_count,
                    SUM(CASE WHEN s.direction LIKE '%Sell%' THEN 1 ELSE 0 END) as sell_count,
                    SUM(CASE WHEN s.direction NOT LIKE '%Buy%' AND s.direction NOT LIKE '%Sell%' THEN 1 ELSE 0 END) as neutral_count,
                    AVG(s.score) as avg_score,
                    MAX(s.generated_at) as last_generated_at
             FROM signal_strategies ss
             JOIN signals s ON ss.signal_id = s.signal_id
             GROUP BY ss.strategy_id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(StrategyStats {
                strategy_id: row.get(0)?,
                total_signals: row.get(1)?,
                buy_count: row.get(2)?,
                sell_count: row.get(3)?,
                neutral_count: row.get(4)?,
                avg_score: row.get::<_, f64>(5).unwrap_or(0.0),
                last_generated_at: row.get(6)?,
            })
        })?;
        let mut stats: Vec<StrategyStats> = Vec::new();
        for r in rows {
            stats.push(r?);
        }
        // Fill in zeros for strategies with no signals yet
        for id in &ACTIVE_STRATEGY_IDS {
            if !stats.iter().any(|s| s.strategy_id == *id) {
                stats.push(StrategyStats {
                    strategy_id: id.to_string(),
                    total_signals: 0,
                    buy_count: 0,
                    sell_count: 0,
                    neutral_count: 0,
                    avg_score: 0.0,
                    last_generated_at: None,
                });
            }
        }
        Ok(stats)
    }

    pub fn insert_scan_run(&self, started_at: &str) -> anyhow::Result<i64> {
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "INSERT INTO scan_runs (started_at, status) VALUES (?1, 'running')",
            [started_at],
        )?;
        Ok(db.connection().last_insert_rowid())
    }

    pub fn complete_scan_run(
        &self,
        id: i64,
        symbols_scanned: u32,
        signals_found: u32,
        duration_ms: u32,
        error: Option<&str>,
    ) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        let status = if error.is_some() { "failed" } else { "done" };
        db.connection().execute(
            "UPDATE scan_runs SET ended_at = datetime('now'), symbols_scanned = ?1, signals_found = ?2, duration_ms = ?3, status = ?4, error = ?5 WHERE id = ?6",
            rusqlite::params![symbols_scanned, signals_found, duration_ms, status, error, id],
        )?;
        Ok(())
    }

    pub fn list_scan_runs(
        &self,
        page: usize,
        page_size: usize,
    ) -> anyhow::Result<(Vec<ScanRunRecord>, usize)> {
        let db = Database::open(&self.path)?;
        let total: usize =
            db.connection()
                .query_row("SELECT COUNT(*) FROM scan_runs", [], |r| r.get(0))?;
        let offset = page.saturating_sub(1).saturating_mul(page_size);
        let mut stmt = db.connection().prepare(
            "SELECT id, started_at, ended_at, symbols_scanned, signals_found, duration_ms, status
             FROM scan_runs ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![page_size as i64, offset as i64], |row| {
            Ok(ScanRunRecord {
                id: row.get::<_, i64>(0)? as u32,
                started_at: row.get(1)?,
                ended_at: row.get(2)?,
                symbols_scanned: row.get::<_, i64>(3).unwrap_or(0) as u32,
                signals_found: row.get::<_, i64>(4).unwrap_or(0) as u32,
                duration_ms: row.get::<_, i64>(5).map(|v| v as u32).ok(),
                status: row.get(6)?,
            })
        })?;
        let mut records = Vec::new();
        for r in rows {
            records.push(r?);
        }
        Ok((records, total))
    }

    pub fn get_strategy_configs(&self) -> anyhow::Result<Vec<StrategyConfig>> {
        let db = Database::open(&self.path)?;
        let mut stmt = db
            .connection()
            .prepare("SELECT strategy_id, enabled, params_json FROM strategy_config")?;
        let rows = stmt.query_map([], |row| {
            let params_json: String = row.get(2)?;
            let params: HashMap<String, f64> =
                serde_json::from_str(&params_json).unwrap_or_default();
            Ok(StrategyConfig {
                strategy_id: row.get(0)?,
                enabled: row.get::<_, i64>(1).unwrap_or(1) != 0,
                params,
            })
        })?;
        let mut configs = Vec::new();
        for r in rows {
            let config = r?;
            if ACTIVE_STRATEGY_IDS.contains(&config.strategy_id.as_str()) {
                configs.push(config);
            }
        }
        Ok(configs)
    }

    pub fn update_strategy_config(
        &self,
        strategy_id: &str,
        enabled: Option<bool>,
        params_json: Option<&str>,
    ) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        if let Some(en) = enabled {
            db.connection().execute(
                "UPDATE strategy_config SET enabled = ?1 WHERE strategy_id = ?2",
                rusqlite::params![en as i64, strategy_id],
            )?;
        }
        if let Some(pj) = params_json {
            db.connection().execute(
                "UPDATE strategy_config SET params_json = ?1 WHERE strategy_id = ?2",
                rusqlite::params![pj, strategy_id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::strategies::SignalDirection;

    fn temp_db_path() -> PathBuf {
        std::env::temp_dir().join(format!(
            "kittyalpha-signal-ledger-{}.sqlite3",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn get_strategy_configs_excludes_retired_arbitrage_rows() {
        let path = temp_db_path();
        let ledger = SignalLedger::new(path.clone()).expect("ledger should initialize");
        {
            let db = Database::open(&path).expect("db should open");
            db.connection()
                .execute(
                    "INSERT OR IGNORE INTO strategy_config (strategy_id, enabled, params_json) VALUES (?1, 1, '{}')",
                    ["spread_arbitrage"],
                )
                .expect("retired strategy row should insert");
        }

        let configs = ledger.get_strategy_configs().expect("configs should load");
        let ids = configs
            .iter()
            .map(|config| config.strategy_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids.len(), ACTIVE_STRATEGY_IDS.len());
        assert!(!ids.contains(&"spread_arbitrage"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn preserves_signal_direction_when_listing_history() {
        let path = temp_db_path();
        let ledger = SignalLedger::new(path.clone()).expect("ledger should initialize");
        ledger
            .insert_signal(
                &UnifiedSignal {
                    signal_id: "sig-direction-1".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "perpetual".into(),
                    direction: SignalDirection::Buy,
                    score: 80.0,
                    strength: 0.8,
                    category_breakdown: HashMap::new(),
                    contributors: vec!["ma_cross".into()],
                    entry_zone_low: 50_000.0,
                    entry_zone_high: 50_100.0,
                    stop_loss: 49_000.0,
                    take_profit: 52_000.0,
                    reason_summary: "direction regression".into(),
                    risk_status: "approved".into(),
                    generated_at: "2026-05-03T20:11:00+08:00".into(),
                },
                "{}",
            )
            .expect("signal should persist");

        let rows = ledger.list_signals(10).expect("signals should list");

        assert_eq!(rows[0].signal.direction, SignalDirection::Buy);
        let _ = std::fs::remove_file(path);
    }
}
