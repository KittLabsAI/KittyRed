use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use rusqlite::{params, OptionalExtension};
use time::OffsetDateTime;

use crate::db::Database;
use crate::models::{RecommendationAuditDto, RecommendationRunDto};

#[derive(Debug, Clone)]
pub struct RecommendationRecord {
    pub run: RecommendationRunDto,
    pub trigger_type: String,
    pub ai_raw_output: String,
    pub ai_structured_output: String,
    pub risk_result: String,
    pub market_snapshot: String,
    pub account_snapshot: String,
}

#[derive(Debug, Clone)]
pub struct PersistedRecommendation {
    pub run: RecommendationRunDto,
    pub market_snapshot: String,
    pub executed: bool,
    pub modified: bool,
}

#[derive(Debug, Clone)]
pub struct RecommendationEvaluation {
    pub evaluation_id: String,
    pub recommendation_id: String,
    pub horizon: String,
    pub price_at_horizon: f64,
    pub max_favorable_price: f64,
    pub max_adverse_price: f64,
    pub take_profit_hit: bool,
    pub stop_loss_hit: bool,
    pub estimated_fee: f64,
    pub estimated_slippage: f64,
    pub funding_fee: f64,
    pub estimated_pnl: f64,
    pub estimated_pnl_percent: f64,
    pub result: String,
    pub evaluated_at: String,
}

#[derive(Clone)]
pub struct RecommendationLedger {
    path: PathBuf,
}

impl RecommendationLedger {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let db = Database::open(&path)?;
        ensure_recommendation_run_columns(db.connection())?;
        Ok(Self { path })
    }

    pub fn insert_record(&self, record: &RecommendationRecord) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        let final_output = serde_json::to_string(&record.run)?;

        db.connection().execute(
            "INSERT INTO recommendation_runs (
                recommendation_id,
                trigger_type,
                status,
                ai_raw_output,
                ai_structured_output,
                risk_result,
                final_output,
                model_provider,
                model_name,
                prompt_version,
                user_preference_version,
                market_snapshot,
                account_snapshot,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                &record.run.recommendation_id,
                &record.trigger_type,
                &record.run.status,
                &record.ai_raw_output,
                &record.ai_structured_output,
                &record.risk_result,
                final_output,
                &record.run.model_provider,
                &record.run.model_name,
                &record.run.prompt_version,
                &record.run.user_preference_version,
                &record.market_snapshot,
                &record.account_snapshot,
                &record.run.generated_at,
            ],
        )?;

        Ok(())
    }

    pub fn load_latest_run(&self) -> anyhow::Result<Option<RecommendationRunDto>> {
        Ok(self.load_latest_runs()?.into_iter().next())
    }

    pub fn load_latest_runs(&self) -> anyhow::Result<Vec<RecommendationRunDto>> {
        let db = Database::open(&self.path)?;
        let created_at = db
            .connection()
            .query_row(
                "SELECT created_at
                 FROM recommendation_runs
                 ORDER BY created_at DESC
                 LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(created_at) = created_at else {
            return Ok(Vec::new());
        };

        let mut statement = db.connection().prepare(
            "SELECT final_output
             FROM recommendation_runs
             WHERE created_at = ?1
             ORDER BY rowid DESC",
        )?;
        let rows = statement.query_map(params![created_at], |row| row.get::<_, String>(0))?;
        let mut runs = Vec::new();
        for row in rows {
            let payload = row?;
            runs.push(
                serde_json::from_str(&payload)
                    .with_context(|| "failed to parse persisted recommendation payload")?,
            );
        }
        Ok(runs)
    }

    pub fn load_run(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Option<RecommendationRunDto>> {
        let db = Database::open(&self.path)?;
        let final_output = db
            .connection()
            .query_row(
                "SELECT final_output
                 FROM recommendation_runs
                 WHERE recommendation_id = ?1",
                params![recommendation_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        final_output
            .map(|payload| {
                serde_json::from_str(&payload)
                    .with_context(|| "failed to parse persisted recommendation payload")
            })
            .transpose()
    }

    pub fn load_audit_record(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Option<RecommendationAuditDto>> {
        let db = Database::open(&self.path)?;
        let payload = db
            .connection()
            .query_row(
                "SELECT
                    trigger_type,
                    final_output,
                    created_at,
                    model_provider,
                    model_name,
                    prompt_version,
                    user_preference_version,
                    ai_raw_output,
                    ai_structured_output,
                    risk_result,
                    market_snapshot,
                    account_snapshot
                 FROM recommendation_runs
                 WHERE recommendation_id = ?1",
                params![recommendation_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                    ))
                },
            )
            .optional()?;

        payload
            .map(
                |(
                    trigger_type,
                    final_output,
                    created_at,
                    model_provider,
                    model_name,
                    prompt_version,
                    user_preference_version,
                    ai_raw_output,
                    ai_structured_output,
                    risk_result,
                    market_snapshot,
                    account_snapshot,
                )| {
                    let run = serde_json::from_str::<RecommendationRunDto>(&final_output)
                        .with_context(|| {
                            format!(
                                "failed to parse persisted recommendation payload for {recommendation_id}"
                            )
                        })?;
                    Ok(RecommendationAuditDto {
                        recommendation_id: run.recommendation_id.clone(),
                        trigger_type,
                        symbol: run.symbol.unwrap_or_else(|| "No Recommendation".into()),
                        exchange: run
                            .exchanges
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "N/A".into()),
                        market_type: run.market_type,
                        created_at,
                        model_provider,
                        model_name,
                        prompt_version,
                        user_preference_version,
                        ai_raw_output,
                        ai_structured_output,
                        risk_result,
                        market_snapshot,
                        account_snapshot,
                    })
                },
            )
            .transpose()
    }

    pub fn list_records(&self, limit: usize) -> anyhow::Result<Vec<PersistedRecommendation>> {
        let db = Database::open(&self.path)?;
        let action_flags = self.load_action_flags(db.connection())?;
        let mut statement = db.connection().prepare(
            "SELECT recommendation_id, final_output, market_snapshot
             FROM recommendation_runs
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut records = Vec::new();
        for row in rows {
            let (recommendation_id, payload, market_snapshot) = row?;
            let run =
                serde_json::from_str::<RecommendationRunDto>(&payload).with_context(|| {
                    format!(
                        "failed to parse persisted recommendation payload for {recommendation_id}"
                    )
                })?;
            let (executed, modified) = action_flags
                .get(&recommendation_id)
                .copied()
                .unwrap_or((false, false));
            records.push(PersistedRecommendation {
                run,
                market_snapshot,
                executed,
                modified,
            });
        }

        Ok(records)
    }

    pub fn list_evaluations(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Vec<RecommendationEvaluation>> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT
                evaluation_id,
                recommendation_id,
                horizon,
                price_at_horizon,
                max_favorable_price,
                max_adverse_price,
                take_profit_hit,
                stop_loss_hit,
                estimated_fee,
                estimated_slippage,
                funding_fee,
                estimated_pnl,
                estimated_pnl_percent,
                result,
                evaluated_at
             FROM recommendation_evaluations
             WHERE recommendation_id = ?1
             ORDER BY evaluated_at ASC",
        )?;
        let rows = statement.query_map(params![recommendation_id], |row| {
            Ok(RecommendationEvaluation {
                evaluation_id: row.get(0)?,
                recommendation_id: row.get(1)?,
                horizon: row.get(2)?,
                price_at_horizon: row.get(3)?,
                max_favorable_price: row.get(4)?,
                max_adverse_price: row.get(5)?,
                take_profit_hit: row.get::<_, i64>(6)? != 0,
                stop_loss_hit: row.get::<_, i64>(7)? != 0,
                estimated_fee: row.get(8)?,
                estimated_slippage: row.get(9)?,
                funding_fee: row.get(10)?,
                estimated_pnl: row.get(11)?,
                estimated_pnl_percent: row.get(12)?,
                result: row.get(13)?,
                evaluated_at: row.get(14)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn insert_evaluations(
        &self,
        evaluations: &[RecommendationEvaluation],
    ) -> anyhow::Result<()> {
        if evaluations.is_empty() {
            return Ok(());
        }

        let db = Database::open(&self.path)?;
        for evaluation in evaluations {
            db.connection().execute(
                "INSERT OR IGNORE INTO recommendation_evaluations (
                    evaluation_id,
                    recommendation_id,
                    horizon,
                    price_at_horizon,
                    max_favorable_price,
                    max_adverse_price,
                    take_profit_hit,
                    stop_loss_hit,
                    estimated_fee,
                    estimated_slippage,
                    funding_fee,
                    estimated_pnl,
                    estimated_pnl_percent,
                    result,
                    evaluated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    evaluation.evaluation_id,
                    evaluation.recommendation_id,
                    evaluation.horizon,
                    evaluation.price_at_horizon,
                    evaluation.max_favorable_price,
                    evaluation.max_adverse_price,
                    if evaluation.take_profit_hit { 1 } else { 0 },
                    if evaluation.stop_loss_hit { 1 } else { 0 },
                    evaluation.estimated_fee,
                    evaluation.estimated_slippage,
                    evaluation.funding_fee,
                    evaluation.estimated_pnl,
                    evaluation.estimated_pnl_percent,
                    evaluation.result,
                    evaluation.evaluated_at,
                ],
            )?;
        }

        Ok(())
    }

    pub fn append_user_action(
        &self,
        recommendation_id: &str,
        action_type: &str,
        payload: &str,
    ) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        let created_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| OffsetDateTime::now_utc().to_string());
        let action_id = format!(
            "action-{}-{}",
            recommendation_id,
            (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i128
        );

        db.connection().execute(
            "INSERT INTO recommendation_user_actions (
                action_id,
                recommendation_id,
                action_type,
                payload,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                action_id,
                recommendation_id,
                action_type,
                payload,
                created_at
            ],
        )?;

        Ok(())
    }

    pub fn delete_record(&self, recommendation_id: &str) -> anyhow::Result<()> {
        let db = Database::open(&self.path)?;
        db.connection().execute(
            "DELETE FROM recommendation_user_actions
             WHERE recommendation_id = ?1",
            params![recommendation_id],
        )?;
        db.connection().execute(
            "DELETE FROM recommendation_evaluations
             WHERE recommendation_id = ?1",
            params![recommendation_id],
        )?;
        db.connection().execute(
            "DELETE FROM recommendation_runs
             WHERE recommendation_id = ?1",
            params![recommendation_id],
        )?;

        Ok(())
    }

    pub fn count_runs_since(
        &self,
        trigger_type: &str,
        start_unix_seconds: i64,
    ) -> anyhow::Result<u32> {
        let db = Database::open(&self.path)?;
        let count = db.connection().query_row(
            "SELECT COUNT(*)
             FROM recommendation_runs
             WHERE trigger_type = ?1
               AND CAST(strftime('%s', created_at) AS INTEGER) >= ?2",
            params![trigger_type, start_unix_seconds],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(count.max(0) as u32)
    }

    pub fn count_consecutive_losing_evaluations(
        &self,
        trigger_type: &str,
        horizon: &str,
    ) -> anyhow::Result<u32> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT evaluation.estimated_pnl_percent
             FROM recommendation_evaluations evaluation
             INNER JOIN recommendation_runs run
               ON run.recommendation_id = evaluation.recommendation_id
             WHERE run.trigger_type = ?1
               AND evaluation.horizon = ?2
             ORDER BY CAST(strftime('%s', evaluation.evaluated_at) AS INTEGER) DESC,
                      evaluation.evaluated_at DESC",
        )?;
        let rows =
            statement.query_map(params![trigger_type, horizon], |row| row.get::<_, f64>(0))?;

        let mut streak = 0_u32;
        for row in rows {
            if row? < 0.0 {
                streak += 1;
            } else {
                break;
            }
        }

        Ok(streak)
    }

    pub fn sum_negative_pnl_percent_since(
        &self,
        trigger_type: &str,
        horizon: &str,
        start_unix_seconds: i64,
    ) -> anyhow::Result<f64> {
        let db = Database::open(&self.path)?;
        let mut statement = db.connection().prepare(
            "SELECT evaluation.estimated_pnl_percent
             FROM recommendation_evaluations evaluation
             INNER JOIN recommendation_runs run
               ON run.recommendation_id = evaluation.recommendation_id
             WHERE run.trigger_type = ?1
               AND evaluation.horizon = ?2
               AND CAST(strftime('%s', evaluation.evaluated_at) AS INTEGER) >= ?3",
        )?;
        let rows = statement
            .query_map(params![trigger_type, horizon, start_unix_seconds], |row| {
                row.get::<_, f64>(0)
            })?;

        let mut total_loss = 0.0;
        for row in rows {
            let pnl_percent = row?;
            if pnl_percent < 0.0 {
                total_loss += pnl_percent.abs();
            }
        }

        Ok(total_loss)
    }

    fn load_action_flags(
        &self,
        connection: &rusqlite::Connection,
    ) -> anyhow::Result<HashMap<String, (bool, bool)>> {
        let mut statement = connection.prepare(
            "SELECT recommendation_id, action_type
             FROM recommendation_user_actions",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut flags = HashMap::new();
        for row in rows {
            let (recommendation_id, action_type) = row?;
            let entry = flags.entry(recommendation_id).or_insert((false, false));
            match action_type.as_str() {
                "executed" => entry.0 = true,
                "modified" => entry.1 = true,
                _ => {}
            }
        }

        Ok(flags)
    }
}

fn ensure_recommendation_run_columns(connection: &rusqlite::Connection) -> anyhow::Result<()> {
    let mut statement = connection.prepare("PRAGMA table_info(recommendation_runs)")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let columns = rows.collect::<Result<Vec<_>, _>>()?;

    if !columns.iter().any(|column| column == "market_snapshot") {
        connection.execute(
            "ALTER TABLE recommendation_runs ADD COLUMN market_snapshot TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }

    if !columns.iter().any(|column| column == "account_snapshot") {
        connection.execute(
            "ALTER TABLE recommendation_runs ADD COLUMN account_snapshot TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }

    Ok(())
}
