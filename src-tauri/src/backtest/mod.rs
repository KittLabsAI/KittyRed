use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use anyhow::{anyhow, bail};
use futures::{stream, StreamExt};
use rusqlite::{params, OptionalExtension};
use time::format_description::well_known::Rfc3339;
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

use crate::db::Database;
use crate::financial_reports::FinancialReportService;
use crate::market::{akshare, MarketDataService};
use crate::models::{
    BacktestDatasetDto, BacktestEquityPointDto, BacktestFetchFailureDto, BacktestFetchProgressDto,
    BacktestOpenPositionDto, BacktestRunDto, BacktestSignalDto, BacktestSummaryDto,
    BacktestTradeDto, CreateBacktestDatasetRequestDto, CreateBacktestRequestDto, MarketListRow,
    OhlcvBar, RiskDecisionDto, RuntimeSettingsDto,
};
use crate::recommendations::llm::{self, PositionContext, RECOMMENDATION_PROMPT_VERSION};
use crate::sentiment::SentimentAnalysisService;
use crate::settings::SettingsService;
use crate::watchlist_selection::normalize_selected_symbols;

const DEFAULT_SPREAD_BPS: f64 = 3.0;
const COST_RATE: f64 = 0.001;
const BACKTEST_SIGNAL_TIMEOUT_SECS: u64 = 60;
const BACKTEST_SIGNAL_RETRY_LIMIT: u8 = 3;
const BACKTEST_INITIAL_CAPITAL_CNY: f64 = 1_000_000.0;

#[derive(Clone)]
pub struct BacktestService {
    db: Arc<Mutex<Database>>,
    cancellations: Arc<dashmap::DashMap<String, Arc<AtomicBool>>>,
}

#[derive(Debug, Clone)]
struct BacktestSnapshot {
    snapshot_id: String,
    dataset_id: String,
    symbol: String,
    stock_name: Option<String>,
    captured_at: String,
    last_price: f64,
    high_price: f64,
    low_price: f64,
    change_24h: f64,
    volume_24h: f64,
    spread_bps: f64,
    kline_5m: String,
    kline_1h: String,
    kline_1d: String,
    kline_1w: String,
    kline_data_json: String,
    stock_info: String,
}

#[derive(Debug, Clone)]
struct VirtualPosition {
    signal_id: String,
    symbol: String,
    stock_name: Option<String>,
    entry_price: f64,
    entry_at: String,
    stop_loss: Option<f64>,
    take_profit: Option<f64>,
    amount_cny: f64,
    holding_periods: u32,
    max_favorable_price: f64,
    max_adverse_price: f64,
}

#[derive(Debug, Clone, Copy)]
struct FetchOutcome {
    inserted: u32,
    total: u32,
    failures: u32,
}

#[derive(Clone)]
struct SampledBar<'a> {
    captured_at: String,
    bar: &'a OhlcvBar,
}

impl BacktestService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            db: Arc::new(Mutex::new(Database::open(&path)?)),
            cancellations: Arc::new(dashmap::DashMap::new()),
        })
    }

    pub fn create_dataset(
        &self,
        request: CreateBacktestDatasetRequestDto,
    ) -> anyhow::Result<BacktestDatasetDto> {
        if request.name.trim().is_empty() {
            bail!("数据集名称不能为空");
        }
        if request.symbols.is_empty() {
            bail!("回测数据集至少需要一个自选股标的");
        }
        let interval = request.interval_minutes.max(5);
        let estimated = planned_snapshot_count(
            &request.start_date,
            &request.end_date,
            interval,
            request.symbols.len(),
        )?;
        let dataset_id = format!("dataset-{}", Uuid::new_v4());
        let created_at = now_rfc3339();
        let symbols_json = serde_json::to_string(&request.symbols)?;

        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_datasets (
              dataset_id, name, status, symbols_json, start_date, end_date,
              interval_minutes, estimated_llm_calls, created_at
            ) VALUES (?1, ?2, 'pending', ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    dataset_id,
                    request.name.trim(),
                    symbols_json,
                    request.start_date,
                    request.end_date,
                    interval,
                    estimated,
                    created_at,
                ],
            )?;

        self.load_dataset(&dataset_id)?
            .ok_or_else(|| anyhow!("新建回测数据集失败"))
    }

    pub fn fetch_snapshots(
        &self,
        dataset_id: &str,
        market_data_service: &MarketDataService,
        settings_service: &SettingsService,
        runtime: &RuntimeSettingsDto,
    ) -> anyhow::Result<()> {
        let cancel = self.cancel_flag(dataset_id);
        cancel.store(false, Ordering::SeqCst);
        let Some(dataset) = self.load_dataset(dataset_id)? else {
            bail!("回测数据集不存在");
        };
        let estimated_total = planned_snapshot_count(
            &dataset.start_date,
            &dataset.end_date,
            dataset.interval_minutes,
            dataset.symbols.len(),
        )?;

        self.clear_dataset_fetch_state(dataset_id)?;
        self.set_dataset_estimated_total(dataset_id, estimated_total)?;
        self.update_dataset_status(dataset_id, "fetching", None, None)?;
        let result = self.fetch_snapshots_inner(
            &dataset,
            market_data_service,
            settings_service,
            runtime,
            &cancel,
        );
        match result {
            Ok(outcome) => {
                let message = (outcome.failures > 0).then(|| {
                    format!(
                        "拉取完成，{} 个快照可用，{} 个股票-时间点失败，可在失败记录中查看。",
                        outcome.inserted, outcome.failures
                    )
                });
                self.update_dataset_status(dataset_id, "ready", Some(outcome.total), message)
            }
            Err(error) => {
                let status = if cancel.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "failed"
                };
                self.update_dataset_status(dataset_id, status, None, Some(error.to_string()))
            }
        }
    }

    pub fn cancel(&self, id: &str) {
        self.cancel_flag(id).store(true, Ordering::SeqCst);
    }

    pub fn list_datasets(&self) -> anyhow::Result<Vec<BacktestDatasetDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT dataset_id, name, status, symbols_json, start_date, end_date,
                    interval_minutes, total_snapshots, fetched_count, estimated_llm_calls,
                    error_message, created_at, completed_at
             FROM backtest_datasets
             ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], dataset_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn active_fetch_dataset_ids(&self) -> anyhow::Result<Vec<String>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db
            .connection()
            .prepare("SELECT dataset_id FROM backtest_datasets WHERE status = 'fetching'")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn fetch_progress(&self, dataset_id: &str) -> anyhow::Result<BacktestFetchProgressDto> {
        let dataset = self
            .load_dataset(dataset_id)?
            .ok_or_else(|| anyhow!("回测数据集不存在"))?;
        let failure_count = self.fetch_failure_count(dataset_id)?;
        Ok(BacktestFetchProgressDto {
            dataset_id: dataset.dataset_id,
            status: dataset.status,
            total_snapshots: dataset.total_snapshots,
            fetched_count: dataset.fetched_count,
            failure_count,
            error_message: dataset.error_message,
            recent_failures: self.list_fetch_failures(dataset_id, 5)?,
        })
    }

    pub fn list_fetch_failures(
        &self,
        dataset_id: &str,
        limit: u32,
    ) -> anyhow::Result<Vec<BacktestFetchFailureDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT failure_id, dataset_id, symbol, captured_at, timeframe, stage,
                    reason, error_detail, created_at
             FROM backtest_fetch_failures
             WHERE dataset_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![dataset_id, limit], fetch_failure_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn delete_dataset(&self, dataset_id: &str) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "DELETE FROM backtest_datasets WHERE dataset_id = ?1",
                params![dataset_id],
            )?;
        Ok(())
    }

    pub fn create_run(
        &self,
        request: CreateBacktestRequestDto,
        runtime: &RuntimeSettingsDto,
    ) -> anyhow::Result<BacktestRunDto> {
        if request.name.trim().is_empty() {
            bail!("回测名称不能为空");
        }
        let dataset = self
            .load_dataset(&request.dataset_id)?
            .ok_or_else(|| anyhow!("回测数据集不存在"))?;
        if dataset.status != "ready" {
            bail!("只能基于已就绪的数据集执行回测");
        }
        let backtest_id = format!("bt-{}", Uuid::new_v4());
        let created_at = now_rfc3339();
        let risk_settings_json = serde_json::to_string(runtime)?;
        let total_ai_calls = self.snapshot_count(&request.dataset_id)?;
        let total_timepoints = self.snapshot_timepoint_count(&request.dataset_id)?;
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
              backtest_id, dataset_id, name, status, model_provider, model_name,
              prompt_version, risk_settings_json, max_holding_days, total_ai_calls,
              total_timepoints, created_at
            ) VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    backtest_id,
                    request.dataset_id,
                    request.name.trim(),
                    runtime.model_provider,
                    runtime.model_name,
                    RECOMMENDATION_PROMPT_VERSION,
                    risk_settings_json,
                    request.max_holding_days.unwrap_or(7).max(1),
                    total_ai_calls,
                    total_timepoints,
                    created_at,
                ],
            )?;
        self.load_run(&backtest_id)?
            .ok_or_else(|| anyhow!("新建回测失败"))
    }

    pub async fn run_backtest(
        &self,
        backtest_id: &str,
        settings_service: &SettingsService,
        market_data_service: &MarketDataService,
        financial_report_service: &FinancialReportService,
        sentiment_analysis_service: &SentimentAnalysisService,
    ) -> anyhow::Result<()> {
        self.generate_signals(
            backtest_id,
            settings_service,
            market_data_service,
            financial_report_service,
            sentiment_analysis_service,
            None,
        )
        .await?;
        self.replay_trades(backtest_id).await
    }

    pub async fn generate_signals(
        &self,
        backtest_id: &str,
        settings_service: &SettingsService,
        market_data_service: &MarketDataService,
        financial_report_service: &FinancialReportService,
        sentiment_analysis_service: &SentimentAnalysisService,
        selected_symbols: Option<&[String]>,
    ) -> anyhow::Result<()> {
        let cancel = self.cancel_flag(backtest_id);
        cancel.store(false, Ordering::SeqCst);
        let run = self
            .load_run(backtest_id)?
            .ok_or_else(|| anyhow!("回测不存在"))?;
        let runtime = settings_service.get_runtime_settings();
        self.update_run_status(backtest_id, "generating_signals", None)?;

        let result = self
            .generate_signals_inner(
                &run,
                &runtime,
                settings_service,
                market_data_service,
                financial_report_service,
                sentiment_analysis_service,
                cancel.clone(),
                selected_symbols,
            )
            .await;
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let status = if cancel.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "failed"
                };
                self.update_run_status(backtest_id, status, Some(error.to_string()))?;
                Err(error)
            }
        }
    }

    pub async fn replay_trades(&self, backtest_id: &str) -> anyhow::Result<()> {
        let cancel = self.cancel_flag(backtest_id);
        cancel.store(false, Ordering::SeqCst);
        let run = self
            .load_run(backtest_id)?
            .ok_or_else(|| anyhow!("回测不存在"))?;
        self.update_run_status(backtest_id, "replaying", None)?;

        let result = self.replay_trades_inner(&run, &cancel).await;
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let status = if cancel.load(Ordering::SeqCst) {
                    "cancelled"
                } else {
                    "failed"
                };
                self.update_run_status(backtest_id, status, Some(error.to_string()))?;
                Err(error)
            }
        }
    }

    pub fn list_runs(&self) -> anyhow::Result<Vec<BacktestRunDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT backtest_id, dataset_id, name, status, model_provider, model_name,
                    prompt_version, max_holding_days, total_ai_calls, processed_ai_calls,
                    total_timepoints, processed_timepoints, total_signals, trade_signals,
                    open_trades, win_count, loss_count, flat_count, total_pnl_cny,
                    total_pnl_percent, max_drawdown_percent, profit_factor, error_message,
                    created_at, completed_at
             FROM backtest_runs
             ORDER BY created_at DESC",
        )?;
        let rows = statement.query_map([], run_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn active_run_ids(&self, statuses: &[&str]) -> anyhow::Result<Vec<(String, String)>> {
        let wanted = statuses.iter().copied().collect::<HashSet<_>>();
        Ok(self
            .list_runs()?
            .into_iter()
            .filter(|run| wanted.contains(run.status.as_str()))
            .map(|run| (run.backtest_id, run.status))
            .collect())
    }

    pub fn list_signals(&self, backtest_id: &str) -> anyhow::Result<Vec<BacktestSignalDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT s.signal_id, s.backtest_id, s.symbol,
                    COALESCE(s.stock_name, snap.stock_name, s.symbol),
                    s.captured_at, s.has_trade, s.direction, s.confidence_score,
                    s.risk_status, s.entry_low, s.entry_high, s.stop_loss,
                    s.take_profit, s.amount_cny, s.max_loss_cny, s.rationale, s.result
             FROM backtest_signals s
             LEFT JOIN backtest_runs r ON r.backtest_id = s.backtest_id
             LEFT JOIN backtest_snapshots snap
               ON snap.dataset_id = r.dataset_id
              AND snap.symbol = s.symbol
              AND snap.captured_at = s.captured_at
             WHERE s.backtest_id = ?1
             ORDER BY s.captured_at ASC, s.symbol ASC",
        )?;
        let rows = statement.query_map(params![backtest_id], signal_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_trades(&self, backtest_id: &str) -> anyhow::Result<Vec<BacktestTradeDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT trade_id, backtest_id, signal_id, symbol, stock_name, direction,
                    entry_price, entry_at, exit_price, exit_at, exit_reason, stop_loss,
                    take_profit, amount_cny, holding_periods, pnl_cny, pnl_percent
             FROM backtest_trades
             WHERE backtest_id = ?1
             ORDER BY exit_at ASC, symbol ASC",
        )?;
        let rows = statement.query_map(params![backtest_id], trade_from_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn summary(&self, backtest_id: &str) -> anyhow::Result<BacktestSummaryDto> {
        let run = self
            .load_run(backtest_id)?
            .ok_or_else(|| anyhow!("回测不存在"))?;
        let trades = self.list_trades(backtest_id)?;
        let equity_curve = match self.load_equity_curve(backtest_id)? {
            curve if !curve.is_empty() => curve,
            _ => {
                let timepoints = self.snapshot_timepoints(&run.dataset_id)?;
                if timepoints.is_empty() {
                    equity_curve_from_trade_exits(&trades)
                } else {
                    equity_curve_from_timepoints(&timepoints, &trades)
                }
            }
        };
        let open_positions = self.load_open_positions(backtest_id, &run.dataset_id)?;
        let trade_count = trades.len() as u32 * 2 + open_positions.len() as u32;
        let winning_trade_actions =
            trades.iter().filter(|trade| trade.pnl_cny > 0.0).count() as u32 * 2
                + open_positions
                    .iter()
                    .filter(|position| position.unrealized_pnl_cny > 0.0)
                    .count() as u32;
        let win_rate = if trade_count == 0 {
            0.0
        } else {
            winning_trade_actions as f64 / trade_count as f64 * 100.0
        };

        Ok(BacktestSummaryDto {
            backtest_id: backtest_id.to_string(),
            total_signals: run.total_signals,
            trade_count,
            win_rate: round2(win_rate),
            total_pnl_cny: run.total_pnl_cny,
            total_pnl_percent: run.total_pnl_percent,
            max_drawdown_percent: run.max_drawdown_percent,
            profit_factor: run.profit_factor,
            equity_curve,
            open_positions,
        })
    }

    pub fn delete_run(&self, backtest_id: &str) -> anyhow::Result<()> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        db.connection().execute(
            "DELETE FROM backtest_runs WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        db.connection().execute(
            "DELETE FROM backtest_equity_curve WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        Ok(())
    }

    fn fetch_snapshots_inner(
        &self,
        dataset: &BacktestDatasetDto,
        market_data_service: &MarketDataService,
        settings_service: &SettingsService,
        runtime: &RuntimeSettingsDto,
        cancel: &AtomicBool,
    ) -> anyhow::Result<FetchOutcome> {
        self.fetch_snapshots_inner_with_loaders(
            dataset,
            runtime,
            cancel,
            |symbol, interval, count, start_date, end_date| {
                load_or_fetch_bars(
                    market_data_service,
                    settings_service,
                    symbol,
                    interval,
                    count,
                    start_date,
                    end_date,
                )
            },
            |symbol| {
                akshare::fetch_stock_info_with_settings(settings_service, symbol)
                    .unwrap_or_else(|_| serde_json::json!({}))
            },
        )
    }

    fn fetch_snapshots_inner_with_loaders<F, G>(
        &self,
        dataset: &BacktestDatasetDto,
        runtime: &RuntimeSettingsDto,
        cancel: &AtomicBool,
        mut load_bars: F,
        mut load_stock_info: G,
    ) -> anyhow::Result<FetchOutcome>
    where
        F: FnMut(&str, &str, usize, Option<&str>, Option<&str>) -> anyhow::Result<Vec<OhlcvBar>>,
        G: FnMut(&str) -> serde_json::Value,
    {
        let mut inserted = 0;
        let mut total = 0;
        self.set_dataset_total(&dataset.dataset_id, total)?;
        let mut failures = 0;
        for symbol in &dataset.symbols {
            if cancel.load(Ordering::SeqCst) {
                bail!("数据拉取已取消");
            }
            let bars_5m = match load_bars(
                symbol,
                "5m",
                900,
                Some(dataset.start_date.as_str()),
                Some(dataset.end_date.as_str()),
            ) {
                Ok(bars) => bars,
                Err(error) => {
                    let timepoints = expected_timepoints(
                        &dataset.start_date,
                        &dataset.end_date,
                        dataset.interval_minutes,
                    )?;
                    total += timepoints.len() as u32;
                    self.set_dataset_total(&dataset.dataset_id, total)?;
                    let recorded = self.record_failures_for_timepoints(
                        &dataset.dataset_id,
                        symbol,
                        "5m",
                        "history_bars",
                        &timepoints,
                        &format!("{} 5m K 线拉取失败", symbol),
                        &error.to_string(),
                    )?;
                    failures += recorded;
                    self.set_dataset_progress(&dataset.dataset_id, inserted)?;
                    continue;
                }
            };
            if bars_5m.is_empty() {
                let timepoints = expected_timepoints(
                    &dataset.start_date,
                    &dataset.end_date,
                    dataset.interval_minutes,
                )?;
                total += timepoints.len() as u32;
                self.set_dataset_total(&dataset.dataset_id, total)?;
                let recorded = self.record_failures_for_timepoints(
                    &dataset.dataset_id,
                    symbol,
                    "5m",
                    "history_bars",
                    &timepoints,
                    &format!("{} 5m K 线为空", symbol),
                    "AKShare 未返回 5m K 线",
                )?;
                failures += recorded;
                continue;
            }
            let selected = sampled_bars(
                &bars_5m,
                &dataset.start_date,
                &dataset.end_date,
                dataset.interval_minutes,
            );
            total += selected.len() as u32;
            self.set_dataset_total(&dataset.dataset_id, total)?;
            let mut context_bars = HashMap::new();
            for frequency in configured_ai_kline_frequencies(runtime) {
                let bars = if frequency == "5m" {
                    bars_5m.clone()
                } else {
                    self.load_optional_context_bars(
                        &dataset.dataset_id,
                        symbol,
                        &frequency,
                        runtime.ai_kline_bar_count.max(1) as usize,
                        Some(dataset.start_date.as_str()),
                        Some(dataset.end_date.as_str()),
                        &selected,
                        &mut load_bars,
                        &mut failures,
                    )?
                };
                context_bars.insert(frequency, bars);
            }
            let stock_info = load_stock_info(symbol);
            let stock_name = stock_name_from_info(&stock_info).unwrap_or_else(|| symbol.clone());
            for (index, sampled) in selected.iter().enumerate() {
                if cancel.load(Ordering::SeqCst) {
                    bail!("数据拉取已取消");
                }
                let bar = sampled.bar;
                let captured_at = sampled.captured_at.as_str();
                if bar.close <= 0.0 || bar.high <= 0.0 || bar.low <= 0.0 {
                    failures += self.insert_fetch_failure(
                        &dataset.dataset_id,
                        symbol,
                        Some(captured_at),
                        "snapshot",
                        "validate_bar",
                        &format!("{} {} 价格数据无效", symbol, captured_at),
                        Some("K 线价格必须大于 0"),
                    )?;
                    self.set_dataset_progress(&dataset.dataset_id, inserted)?;
                    continue;
                }
                let previous = selected
                    .get(index.saturating_sub(48))
                    .filter(|_| index >= 48)
                    .map(|item| item.bar.close)
                    .unwrap_or(bar.open);
                let change_24h = if previous.abs() > f64::EPSILON {
                    (bar.close - previous) / previous * 100.0
                } else {
                    0.0
                };
                let volume_24h = selected
                    .iter()
                    .skip(index.saturating_sub(48))
                    .take(index.saturating_sub(index.saturating_sub(48)) + 1)
                    .map(|item| item.bar.volume)
                    .sum::<f64>();
                let kline_data = context_bars
                    .iter()
                    .map(|(frequency, bars)| {
                        (
                            frequency.clone(),
                            recent_ohlc(
                                bars,
                                &bar.open_time,
                                runtime.ai_kline_bar_count.max(1) as usize,
                            ),
                        )
                    })
                    .collect::<HashMap<_, _>>();
                let legacy_5m = kline_data.get("5m").cloned().unwrap_or_default();
                let legacy_1h = kline_data.get("1h").cloned().unwrap_or_default();
                let legacy_1d = kline_data.get("1d").cloned().unwrap_or_default();
                let legacy_1w = kline_data.get("1w").cloned().unwrap_or_default();
                let insert_result = self.insert_snapshot(
                    dataset,
                    symbol,
                    Some(stock_name.as_str()),
                    captured_at,
                    bar,
                    change_24h,
                    volume_24h,
                    serde_json::to_string(&legacy_5m)?,
                    serde_json::to_string(&legacy_1h)?,
                    serde_json::to_string(&legacy_1d)?,
                    serde_json::to_string(&legacy_1w)?,
                    serde_json::to_string(&kline_data)?,
                    serde_json::to_string(&stock_info)?,
                );
                match insert_result {
                    Ok(()) => {
                        inserted += 1;
                    }
                    Err(error) => {
                        failures += self.insert_fetch_failure(
                            &dataset.dataset_id,
                            symbol,
                            Some(captured_at),
                            "snapshot",
                            "insert_snapshot",
                            &format!("{} {} 快照写入失败", symbol, captured_at),
                            Some(&error.to_string()),
                        )?;
                    }
                }
                self.set_dataset_progress(&dataset.dataset_id, inserted)?;
            }
        }
        if inserted == 0 {
            bail!(
                "没有拉取到可用历史快照，已记录 {} 条失败股票-时间点。",
                failures
            );
        }
        Ok(FetchOutcome {
            inserted,
            total,
            failures,
        })
    }

    async fn generate_signals_inner(
        &self,
        run: &BacktestRunDto,
        runtime: &RuntimeSettingsDto,
        settings_service: &SettingsService,
        _market_data_service: &MarketDataService,
        financial_report_service: &FinancialReportService,
        sentiment_analysis_service: &SentimentAnalysisService,
        cancel: Arc<AtomicBool>,
        selected_symbols: Option<&[String]>,
    ) -> anyhow::Result<()> {
        let mut snapshots = self.load_snapshots(&run.dataset_id)?;
        if snapshots.is_empty() {
            bail!("回测数据集没有可用快照");
        }
        if let Some(selected_symbols) = selected_symbols {
            let selected = normalize_selected_symbols(selected_symbols);
            snapshots.retain(|snapshot| selected.contains(&snapshot.symbol));
        }
        if snapshots.is_empty() {
            bail!("回测数据集没有匹配所选股票的快照");
        }
        let existing = self.existing_signal_keys(&run.backtest_id)?;
        let pending = snapshots
            .into_iter()
            .filter(|snapshot| {
                !existing.contains(&(snapshot.symbol.clone(), snapshot.captured_at.clone()))
            })
            .collect::<Vec<_>>();
        let total_ai_calls = existing.len().saturating_add(pending.len()) as u32;
        self.reset_signal_generation(&run.backtest_id, total_ai_calls, existing.len() as u32)?;

        let stream = stream::iter(pending.into_iter().map(|snapshot| {
            let settings_service = settings_service.clone();
            let cancel = cancel.clone();
            async move {
                if cancel.load(Ordering::SeqCst) {
                    bail!("回测已取消");
                }
                let row = market_row_from_snapshot(&snapshot);
                let financial_report_analysis =
                    backtest_financial_context(financial_report_service, runtime, &snapshot.symbol);
                let sentiment_analysis =
                    backtest_sentiment_context(sentiment_analysis_service, &snapshot.symbol);
                let stock_info = serde_json::from_str(&snapshot.stock_info)
                    .unwrap_or_else(|_| serde_json::json!({}));
                let kline_map = snapshot_kline_map(&snapshot);
                let plan = complete_backtest_signal_with_retry(
                    Duration::from_secs(BACKTEST_SIGNAL_TIMEOUT_SECS),
                    || async {
                        llm::generate_trade_plan_with_historical_context(
                            &settings_service,
                            &row,
                            &[row.clone()],
                            1_000_000.0,
                            None::<&PositionContext>,
                            financial_report_analysis.clone(),
                            sentiment_analysis.clone(),
                            stock_info.clone(),
                            kline_map.clone(),
                        )
                        .await
                    },
                )
                .await
                .map_err(|error| error.to_string());
                Ok::<_, anyhow::Error>((snapshot, plan))
            }
        }))
        .buffer_unordered(20);
        futures::pin_mut!(stream);

        while let Some(result) = stream.next().await {
            if cancel.load(Ordering::SeqCst) {
                bail!("回测已取消");
            }
            let (snapshot, plan) = result?;
            let signal_id = format!("sig-{}", Uuid::new_v4());
            match plan {
                Ok(plan) => {
                    self.insert_signal(
                        &run.backtest_id,
                        &signal_id,
                        &snapshot,
                        &plan.run,
                        &plan.ai_raw_output,
                        &plan.ai_structured_output,
                        if plan.run.has_trade {
                            "generated"
                        } else {
                            "no_trade"
                        },
                    )?;
                }
                Err(error) => {
                    let fallback = fallback_signal_run(&snapshot, runtime, &error);
                    self.insert_signal(
                        &run.backtest_id,
                        &signal_id,
                        &snapshot,
                        &fallback,
                        &serde_json::to_string(&serde_json::json!({
                            "source": "backtest_snapshot_fallback",
                            "error": error,
                        }))?,
                        &serde_json::to_string(&serde_json::json!({
                            "source": "backtest_snapshot_fallback",
                            "has_trade": false,
                            "symbol": snapshot.symbol,
                            "captured_at": snapshot.captured_at,
                        }))?,
                        "fallback",
                    )?;
                }
            }
            self.increment_processed_ai_calls(&run.backtest_id)?;
        }

        self.update_run_status(&run.backtest_id, "signals_ready", None)
    }

    async fn replay_trades_inner(
        &self,
        run: &BacktestRunDto,
        cancel: &AtomicBool,
    ) -> anyhow::Result<()> {
        let snapshots = self.load_snapshots(&run.dataset_id)?;
        if snapshots.is_empty() {
            bail!("回测数据集没有可用快照");
        }
        let signals = self.list_signals(&run.backtest_id)?;
        if signals.is_empty() {
            bail!("请先生成 AI 信号，再回放交易");
        }
        self.reset_trade_replay(&run.backtest_id)?;

        let grouped = snapshots
            .into_iter()
            .fold(BTreeMap::new(), |mut map, snapshot| {
                map.entry(snapshot.captured_at.clone())
                    .or_insert_with(Vec::new)
                    .push(snapshot);
                map
            });
        self.set_total_timepoints(&run.backtest_id, grouped.len() as u32)?;
        let signals_by_time = signals.into_iter().fold(HashMap::new(), |mut map, signal| {
            map.entry(signal.captured_at.clone())
                .or_insert_with(Vec::new)
                .push(signal);
            map
        });

        let mut positions: Vec<VirtualPosition> = Vec::new();
        let mut trades: Vec<BacktestTradeDto> = Vec::new();
        let mut equity_curve: Vec<BacktestEquityPointDto> = Vec::new();
        let mut latest_snapshots_by_symbol: HashMap<String, BacktestSnapshot> = HashMap::new();
        let mut cash_cny = BACKTEST_INITIAL_CAPITAL_CNY;
        let mut latest_total_pnl_cny = 0.0;
        let mut latest_total_pnl_percent = 0.0;
        for (index, (captured_at, items)) in grouped.iter().enumerate() {
            if cancel.load(Ordering::SeqCst) {
                bail!("回测已取消");
            }

            let current_by_symbol = items
                .iter()
                .map(|snapshot| (snapshot.symbol.clone(), snapshot))
                .collect::<HashMap<_, _>>();
            latest_snapshots_by_symbol.extend(
                items
                    .iter()
                    .cloned()
                    .map(|snapshot| (snapshot.symbol.clone(), snapshot)),
            );
            let mut remaining = Vec::new();
            for mut position in positions {
                position.holding_periods += 1;
                if let Some(snapshot) = current_by_symbol.get(&position.symbol) {
                    position.max_favorable_price =
                        position.max_favorable_price.max(snapshot.high_price);
                    position.max_adverse_price = position.max_adverse_price.min(snapshot.low_price);
                    if let Some(mut trade) =
                        maybe_exit_position(&position, snapshot, run.max_holding_days, captured_at)
                    {
                        trade.backtest_id = run.backtest_id.clone();
                        self.insert_trade(&trade)?;
                        cash_cny += position.amount_cny + trade.pnl_cny;
                        trades.push(trade);
                        continue;
                    }
                }
                remaining.push(position);
            }
            positions = remaining;

            for signal in signals_by_time.get(captured_at).into_iter().flatten() {
                if !signal.has_trade || signal.direction.as_deref() != Some("买入") {
                    continue;
                }
                if positions
                    .iter()
                    .any(|position| position.symbol == signal.symbol)
                {
                    continue;
                }
                let Some(snapshot) = current_by_symbol.get(&signal.symbol) else {
                    continue;
                };
                if let Some(position) = open_position_from_signal(signal, snapshot) {
                    if position.amount_cny > cash_cny {
                        self.update_signal_result(
                            &run.backtest_id,
                            &signal.signal_id,
                            "insufficient_funds",
                        )?;
                        continue;
                    }
                    cash_cny -= position.amount_cny;
                    self.update_signal_result(&run.backtest_id, &signal.signal_id, "opened")?;
                    positions.push(position);
                }
            }

            let market_value_cny: f64 = positions
                .iter()
                .filter_map(|p| {
                    let snapshot = current_by_symbol
                        .get(&p.symbol)
                        .copied()
                        .or_else(|| latest_snapshots_by_symbol.get(&p.symbol))?;
                    Some(position_market_value_cny(p, snapshot.last_price))
                })
                .sum();
            let total_equity_cny: f64 = cash_cny + market_value_cny;
            latest_total_pnl_cny = total_equity_cny - BACKTEST_INITIAL_CAPITAL_CNY;
            latest_total_pnl_percent = latest_total_pnl_cny / BACKTEST_INITIAL_CAPITAL_CNY * 100.0;
            equity_curve.push(BacktestEquityPointDto {
                captured_at: captured_at.clone(),
                cumulative_pnl_percent: round2(latest_total_pnl_percent),
            });

            self.set_run_progress(
                &run.backtest_id,
                index as u32 + 1,
                positions.len() as u32,
                &trades,
                latest_total_pnl_cny,
                latest_total_pnl_percent,
            )?;
        }

        self.insert_equity_curve(&run.backtest_id, &equity_curve)?;
        self.complete_run(
            &run.backtest_id,
            &trades,
            &equity_curve,
            positions.len() as u32,
            latest_total_pnl_cny,
            latest_total_pnl_percent,
        )
    }

    fn cancel_flag(&self, id: &str) -> Arc<AtomicBool> {
        self.cancellations
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    fn load_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<BacktestDatasetDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        db.connection()
            .query_row(
                "SELECT dataset_id, name, status, symbols_json, start_date, end_date,
                        interval_minutes, total_snapshots, fetched_count, estimated_llm_calls,
                        error_message, created_at, completed_at
                 FROM backtest_datasets WHERE dataset_id = ?1",
                params![dataset_id],
                dataset_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    fn load_run(&self, backtest_id: &str) -> anyhow::Result<Option<BacktestRunDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        db.connection()
            .query_row(
                "SELECT backtest_id, dataset_id, name, status, model_provider, model_name,
                        prompt_version, max_holding_days, total_ai_calls, processed_ai_calls,
                        total_timepoints, processed_timepoints, total_signals, trade_signals,
                        open_trades, win_count, loss_count, flat_count, total_pnl_cny,
                        total_pnl_percent, max_drawdown_percent, profit_factor, error_message,
                        created_at, completed_at
                 FROM backtest_runs WHERE backtest_id = ?1",
                params![backtest_id],
                run_from_row,
            )
            .optional()
            .map_err(Into::into)
    }

    fn load_snapshots(&self, dataset_id: &str) -> anyhow::Result<Vec<BacktestSnapshot>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT snapshot_id, dataset_id, symbol, stock_name, captured_at, last_price,
                    high_price, low_price, change_24h, volume_24h, spread_bps,
                    kline_5m, kline_1h, kline_1d, kline_1w, kline_data_json,
                    stock_info
             FROM backtest_snapshots
             WHERE dataset_id = ?1
             ORDER BY captured_at ASC, symbol ASC",
        )?;
        let rows = statement.query_map(params![dataset_id], |row| {
            Ok(BacktestSnapshot {
                snapshot_id: row.get(0)?,
                dataset_id: row.get(1)?,
                symbol: row.get(2)?,
                stock_name: row.get(3)?,
                captured_at: row.get(4)?,
                last_price: row.get(5)?,
                high_price: row.get(6)?,
                low_price: row.get(7)?,
                change_24h: row.get(8)?,
                volume_24h: row.get(9)?,
                spread_bps: row.get(10)?,
                kline_5m: row.get(11)?,
                kline_1h: row.get(12)?,
                kline_1d: row.get(13)?,
                kline_1w: row.get(14)?,
                kline_data_json: row.get(15)?,
                stock_info: row.get(16)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn snapshot_count(&self, dataset_id: &str) -> anyhow::Result<u32> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM backtest_snapshots WHERE dataset_id = ?1",
                params![dataset_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u32)
            .map_err(Into::into)
    }

    fn snapshot_timepoint_count(&self, dataset_id: &str) -> anyhow::Result<u32> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .query_row(
                "SELECT COUNT(DISTINCT captured_at) FROM backtest_snapshots WHERE dataset_id = ?1",
                params![dataset_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u32)
            .map_err(Into::into)
    }

    fn snapshot_timepoints(&self, dataset_id: &str) -> anyhow::Result<Vec<String>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT DISTINCT captured_at
             FROM backtest_snapshots
             WHERE dataset_id = ?1
             ORDER BY captured_at ASC",
        )?;
        let rows = statement.query_map(params![dataset_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn existing_signal_keys(&self, backtest_id: &str) -> anyhow::Result<HashSet<(String, String)>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db
            .connection()
            .prepare("SELECT symbol, captured_at FROM backtest_signals WHERE backtest_id = ?1")?;
        let rows = statement.query_map(params![backtest_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<HashSet<_>, _>>().map_err(Into::into)
    }

    fn update_dataset_status(
        &self,
        dataset_id: &str,
        status: &str,
        total: Option<u32>,
        error: Option<String>,
    ) -> anyhow::Result<()> {
        let completed_at = matches!(status, "ready" | "failed" | "cancelled").then(now_rfc3339);
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_datasets
             SET status = ?2,
                 total_snapshots = COALESCE(?3, total_snapshots),
                 error_message = ?4,
                 completed_at = COALESCE(?5, completed_at)
             WHERE dataset_id = ?1",
                params![dataset_id, status, total, error, completed_at],
            )?;
        Ok(())
    }

    fn set_dataset_progress(&self, dataset_id: &str, count: u32) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_datasets SET fetched_count = ?2 WHERE dataset_id = ?1",
                params![dataset_id, count],
            )?;
        Ok(())
    }

    fn set_dataset_total(&self, dataset_id: &str, total: u32) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_datasets
                 SET total_snapshots = ?2
                 WHERE dataset_id = ?1",
                params![dataset_id, total],
            )?;
        Ok(())
    }

    fn set_dataset_estimated_total(&self, dataset_id: &str, total: u32) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_datasets
                 SET estimated_llm_calls = ?2
                 WHERE dataset_id = ?1",
                params![dataset_id, total],
            )?;
        Ok(())
    }

    fn clear_dataset_fetch_state(&self, dataset_id: &str) -> anyhow::Result<()> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        db.connection().execute(
            "DELETE FROM backtest_snapshots WHERE dataset_id = ?1",
            params![dataset_id],
        )?;
        db.connection().execute(
            "DELETE FROM backtest_fetch_failures WHERE dataset_id = ?1",
            params![dataset_id],
        )?;
        db.connection().execute(
            "UPDATE backtest_datasets
             SET total_snapshots = 0,
                 fetched_count = 0,
                 error_message = NULL,
                 completed_at = NULL
             WHERE dataset_id = ?1",
            params![dataset_id],
        )?;
        Ok(())
    }

    fn load_optional_context_bars<F>(
        &self,
        dataset_id: &str,
        symbol: &str,
        interval: &str,
        count: usize,
        start_date: Option<&str>,
        end_date: Option<&str>,
        selected: &[SampledBar<'_>],
        load_bars: &mut F,
        failures: &mut u32,
    ) -> anyhow::Result<Vec<OhlcvBar>>
    where
        F: FnMut(&str, &str, usize, Option<&str>, Option<&str>) -> anyhow::Result<Vec<OhlcvBar>>,
    {
        match load_bars(symbol, interval, count, start_date, end_date) {
            Ok(bars) => Ok(bars),
            Err(error) => {
                let timepoints = selected
                    .iter()
                    .map(|item| item.captured_at.clone())
                    .collect::<Vec<_>>();
                *failures += self.record_failures_for_timepoints(
                    dataset_id,
                    symbol,
                    interval,
                    "history_bars",
                    &timepoints,
                    &format!("{} {} K 线拉取失败", symbol, interval),
                    &error.to_string(),
                )?;
                Ok(Vec::new())
            }
        }
    }

    fn record_failures_for_timepoints(
        &self,
        dataset_id: &str,
        symbol: &str,
        timeframe: &str,
        stage: &str,
        timepoints: &[String],
        reason: &str,
        error_detail: &str,
    ) -> anyhow::Result<u32> {
        if timepoints.is_empty() {
            return self.insert_fetch_failure(
                dataset_id,
                symbol,
                None,
                timeframe,
                stage,
                reason,
                Some(error_detail),
            );
        }

        let mut count = 0;
        for captured_at in timepoints {
            count += self.insert_fetch_failure(
                dataset_id,
                symbol,
                Some(captured_at),
                timeframe,
                stage,
                reason,
                Some(error_detail),
            )?;
        }
        Ok(count)
    }

    fn insert_fetch_failure(
        &self,
        dataset_id: &str,
        symbol: &str,
        captured_at: Option<&str>,
        timeframe: &str,
        stage: &str,
        reason: &str,
        error_detail: Option<&str>,
    ) -> anyhow::Result<u32> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_fetch_failures (
              failure_id, dataset_id, symbol, captured_at, timeframe, stage,
              reason, error_detail, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    format!("btf-{}", Uuid::new_v4()),
                    dataset_id,
                    symbol,
                    captured_at,
                    timeframe,
                    stage,
                    reason,
                    error_detail,
                    now_rfc3339(),
                ],
            )?;
        Ok(1)
    }

    fn fetch_failure_count(&self, dataset_id: &str) -> anyhow::Result<u32> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM backtest_fetch_failures WHERE dataset_id = ?1",
                params![dataset_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|count| count as u32)
            .map_err(Into::into)
    }

    fn insert_snapshot(
        &self,
        dataset: &BacktestDatasetDto,
        symbol: &str,
        stock_name: Option<&str>,
        captured_at: &str,
        bar: &OhlcvBar,
        change_24h: f64,
        volume_24h: f64,
        kline_5m: String,
        kline_1h: String,
        kline_1d: String,
        kline_1w: String,
        kline_data_json: String,
        stock_info: String,
    ) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT OR REPLACE INTO backtest_snapshots (
              snapshot_id, dataset_id, symbol, stock_name, captured_at, last_price,
              high_price, low_price, change_24h, volume_24h, spread_bps,
              kline_5m, kline_1h, kline_1d, kline_1w, kline_data_json,
              stock_info
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    format!("snap-{}", Uuid::new_v4()),
                    dataset.dataset_id,
                    symbol,
                    stock_name,
                    captured_at,
                    bar.close,
                    bar.high,
                    bar.low,
                    round2(change_24h),
                    volume_24h,
                    DEFAULT_SPREAD_BPS,
                    kline_5m,
                    kline_1h,
                    kline_1d,
                    kline_1w,
                    kline_data_json,
                    stock_info,
                ],
            )?;
        Ok(())
    }

    fn update_run_status(
        &self,
        backtest_id: &str,
        status: &str,
        error: Option<String>,
    ) -> anyhow::Result<()> {
        let completed_at = matches!(status, "completed" | "failed" | "cancelled").then(now_rfc3339);
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET status = ?2, error_message = ?3, completed_at = COALESCE(?4, completed_at)
             WHERE backtest_id = ?1",
                params![backtest_id, status, error, completed_at],
            )?;
        Ok(())
    }

    fn set_total_timepoints(&self, backtest_id: &str, total: u32) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs SET total_timepoints = ?2 WHERE backtest_id = ?1",
                params![backtest_id, total],
            )?;
        Ok(())
    }

    fn set_run_progress(
        &self,
        backtest_id: &str,
        processed: u32,
        open_trades: u32,
        trades: &[BacktestTradeDto],
        total_pnl_cny: f64,
        total_pnl_percent: f64,
    ) -> anyhow::Result<()> {
        let win = trades.iter().filter(|trade| trade.pnl_cny > 0.0).count() as u32;
        let loss = trades.iter().filter(|trade| trade.pnl_cny < 0.0).count() as u32;
        let flat = trades.len() as u32 - win - loss;
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET processed_timepoints = ?2,
                 open_trades = ?3,
                 win_count = ?4,
                 loss_count = ?5,
                 flat_count = ?6,
                 total_pnl_cny = ?7,
                 total_pnl_percent = ?8
             WHERE backtest_id = ?1",
                params![
                    backtest_id,
                    processed,
                    open_trades,
                    win,
                    loss,
                    flat,
                    round2(total_pnl_cny),
                    round2(total_pnl_percent),
                ],
            )?;
        Ok(())
    }

    fn reset_signal_generation(
        &self,
        backtest_id: &str,
        total_ai_calls: u32,
        processed_ai_calls: u32,
    ) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET total_ai_calls = ?2,
                 processed_ai_calls = ?3,
                 total_signals = ?3,
                 trade_signals = (
                   SELECT COUNT(*) FROM backtest_signals
                   WHERE backtest_id = ?1 AND has_trade = 1
                 ),
                 processed_timepoints = 0,
                 open_trades = 0,
                 win_count = 0,
                 loss_count = 0,
                 flat_count = 0,
                 total_pnl_cny = 0,
                 total_pnl_percent = 0,
                 max_drawdown_percent = 0,
                 profit_factor = NULL,
                 error_message = NULL,
                 completed_at = NULL
             WHERE backtest_id = ?1",
                params![backtest_id, total_ai_calls, processed_ai_calls],
            )?;
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "DELETE FROM backtest_trades WHERE backtest_id = ?1",
                params![backtest_id],
            )?;
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "DELETE FROM backtest_equity_curve WHERE backtest_id = ?1",
                params![backtest_id],
            )?;
        Ok(())
    }

    fn reset_trade_replay(&self, backtest_id: &str) -> anyhow::Result<()> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        db.connection().execute(
            "DELETE FROM backtest_trades WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        db.connection().execute(
            "DELETE FROM backtest_equity_curve WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        db.connection().execute(
            "UPDATE backtest_signals
             SET result = CASE
               WHEN has_trade = 1 THEN 'generated'
               ELSE result
             END
             WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        db.connection().execute(
            "UPDATE backtest_runs
             SET processed_timepoints = 0,
                 open_trades = 0,
                 win_count = 0,
                 loss_count = 0,
                 flat_count = 0,
                 total_pnl_cny = 0,
                 total_pnl_percent = 0,
                 max_drawdown_percent = 0,
                 profit_factor = NULL,
                 error_message = NULL,
                 completed_at = NULL
             WHERE backtest_id = ?1",
            params![backtest_id],
        )?;
        Ok(())
    }

    fn update_signal_result(
        &self,
        backtest_id: &str,
        signal_id: &str,
        result: &str,
    ) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_signals SET result = ?3 WHERE backtest_id = ?1 AND signal_id = ?2",
                params![backtest_id, signal_id, result],
            )?;
        Ok(())
    }

    fn increment_processed_ai_calls(&self, backtest_id: &str) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET processed_ai_calls = processed_ai_calls + 1
             WHERE backtest_id = ?1",
                params![backtest_id],
            )?;
        Ok(())
    }

    fn insert_signal(
        &self,
        backtest_id: &str,
        signal_id: &str,
        snapshot: &BacktestSnapshot,
        run: &crate::models::RecommendationRunDto,
        ai_raw_output: &str,
        ai_structured_output: &str,
        result: &str,
    ) -> anyhow::Result<()> {
        self.db.lock().expect("backtest db lock poisoned").connection().execute(
            "INSERT INTO backtest_signals (
              signal_id, backtest_id, symbol, stock_name, captured_at, has_trade,
              direction, confidence_score, risk_status, entry_low, entry_high, stop_loss,
              take_profit, amount_cny, max_loss_cny, rationale, ai_raw_output,
              ai_structured_output, result
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                signal_id,
                backtest_id,
                snapshot.symbol,
                snapshot.stock_name,
                snapshot.captured_at,
                if run.has_trade { 1 } else { 0 },
                run.direction,
                run.confidence_score,
                run.risk_status,
                run.entry_low,
                run.entry_high,
                run.stop_loss,
                run.take_profit,
                run.amount_cny,
                run.max_loss_cny,
                run.rationale,
                ai_raw_output,
                ai_structured_output,
                result,
            ],
        )?;
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET total_signals = total_signals + 1,
                 trade_signals = trade_signals + ?2
             WHERE backtest_id = ?1",
                params![backtest_id, if run.has_trade { 1 } else { 0 }],
            )?;
        Ok(())
    }

    fn insert_trade(&self, trade: &BacktestTradeDto) -> anyhow::Result<()> {
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_trades (
              trade_id, backtest_id, signal_id, symbol, stock_name, direction,
              entry_price, entry_at, exit_price, exit_at, exit_reason, stop_loss,
              take_profit, amount_cny, holding_periods, pnl_cny, pnl_percent
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    trade.trade_id,
                    trade.backtest_id,
                    trade.signal_id,
                    trade.symbol,
                    trade.stock_name,
                    trade.direction,
                    trade.entry_price,
                    trade.entry_at,
                    trade.exit_price,
                    trade.exit_at,
                    trade.exit_reason,
                    trade.stop_loss,
                    trade.take_profit,
                    trade.amount_cny,
                    trade.holding_periods,
                    trade.pnl_cny,
                    trade.pnl_percent,
                ],
            )?;
        Ok(())
    }

    fn insert_equity_curve(
        &self,
        backtest_id: &str,
        curve: &[BacktestEquityPointDto],
    ) -> anyhow::Result<()> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let conn = db.connection();
        for point in curve {
            conn.execute(
                "INSERT OR REPLACE INTO backtest_equity_curve (backtest_id, captured_at, cumulative_pnl_percent) VALUES (?1, ?2, ?3)",
                params![backtest_id, point.captured_at, point.cumulative_pnl_percent],
            )?;
        }
        Ok(())
    }

    fn load_equity_curve(&self, backtest_id: &str) -> anyhow::Result<Vec<BacktestEquityPointDto>> {
        let db = self.db.lock().expect("backtest db lock poisoned");
        let mut statement = db.connection().prepare(
            "SELECT captured_at, cumulative_pnl_percent FROM backtest_equity_curve WHERE backtest_id = ?1 ORDER BY captured_at ASC",
        )?;
        let rows = statement.query_map(params![backtest_id], |row| {
            Ok(BacktestEquityPointDto {
                captured_at: row.get(0)?,
                cumulative_pnl_percent: row.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn load_open_positions(
        &self,
        backtest_id: &str,
        dataset_id: &str,
    ) -> anyhow::Result<Vec<BacktestOpenPositionDto>> {
        let signals = self.list_signals(backtest_id)?;
        let trades = self.list_trades(backtest_id)?;
        let closed_signal_ids = trades
            .iter()
            .filter_map(|trade| trade.signal_id.clone())
            .collect::<HashSet<_>>();
        let snapshots = self.load_snapshots(dataset_id)?;
        let snapshots_by_symbol = snapshots.into_iter().fold(
            HashMap::<String, Vec<BacktestSnapshot>>::new(),
            |mut map, snapshot| {
                map.entry(snapshot.symbol.clone())
                    .or_default()
                    .push(snapshot);
                map
            },
        );

        Ok(signals
            .into_iter()
            .filter(|signal| {
                signal.has_trade
                    && signal.direction.as_deref() == Some("买入")
                    && signal.result == "opened"
            })
            .filter(|signal| !closed_signal_ids.contains(&signal.signal_id))
            .filter_map(|signal| {
                let symbol_snapshots = snapshots_by_symbol.get(&signal.symbol)?;
                let entry_at = signal.captured_at.clone();
                let latest = symbol_snapshots
                    .iter()
                    .rev()
                    .find(|snapshot| snapshot.captured_at >= entry_at)?;
                let entry_price = match (signal.entry_low, signal.entry_high) {
                    (Some(low), Some(high)) => (low + high) / 2.0,
                    (Some(value), None) | (None, Some(value)) => value,
                    _ => latest.last_price,
                };
                let amount_cny = signal.amount_cny.unwrap_or(10_000.0);
                let quantity = amount_cny / entry_price.max(0.01);
                let unrealized_pnl_cny = (latest.last_price - entry_price) * quantity;
                let unrealized_pnl_percent =
                    ((latest.last_price - entry_price) / entry_price.max(0.01)) * 100.0;

                Some(BacktestOpenPositionDto {
                    signal_id: signal.signal_id,
                    symbol: signal.symbol,
                    stock_name: signal.stock_name.or_else(|| latest.stock_name.clone()),
                    entry_price: round4(entry_price),
                    entry_at: entry_at.clone(),
                    mark_price: round4(latest.last_price),
                    amount_cny: round2(amount_cny),
                    holding_periods: symbol_snapshots
                        .iter()
                        .filter(|snapshot| snapshot.captured_at > entry_at)
                        .count() as u32,
                    unrealized_pnl_cny: round2(unrealized_pnl_cny),
                    unrealized_pnl_percent: round2(unrealized_pnl_percent),
                })
            })
            .collect())
    }

    fn complete_run(
        &self,
        backtest_id: &str,
        trades: &[BacktestTradeDto],
        equity_curve: &[BacktestEquityPointDto],
        open_trades: u32,
        total_pnl_cny: f64,
        total_pnl_percent: f64,
    ) -> anyhow::Result<()> {
        let win = trades.iter().filter(|trade| trade.pnl_cny > 0.0).count() as u32;
        let loss = trades.iter().filter(|trade| trade.pnl_cny < 0.0).count() as u32;
        let flat = trades.len() as u32 - win - loss;
        let gross_win = trades
            .iter()
            .filter(|trade| trade.pnl_cny > 0.0)
            .map(|trade| trade.pnl_cny)
            .sum::<f64>();
        let gross_loss = trades
            .iter()
            .filter(|trade| trade.pnl_cny < 0.0)
            .map(|trade| trade.pnl_cny.abs())
            .sum::<f64>();
        let profit_factor = (gross_loss > 0.0).then_some(round2(gross_win / gross_loss));
        self.db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "UPDATE backtest_runs
             SET status = 'completed',
                 open_trades = ?2,
                 win_count = ?3,
                 loss_count = ?4,
                 flat_count = ?5,
                 total_pnl_cny = ?6,
                 total_pnl_percent = ?7,
                 max_drawdown_percent = ?8,
                 profit_factor = ?9,
                 completed_at = ?10
             WHERE backtest_id = ?1",
                params![
                    backtest_id,
                    open_trades,
                    win,
                    loss,
                    flat,
                    round2(total_pnl_cny),
                    round2(total_pnl_percent),
                    max_drawdown_from_curve(equity_curve),
                    profit_factor,
                    now_rfc3339(),
                ],
            )?;
        Ok(())
    }
}

fn dataset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BacktestDatasetDto> {
    let symbols_json: String = row.get(3)?;
    let symbols = serde_json::from_str(&symbols_json).unwrap_or_default();
    Ok(BacktestDatasetDto {
        dataset_id: row.get(0)?,
        name: row.get(1)?,
        status: row.get(2)?,
        symbols,
        start_date: row.get(4)?,
        end_date: row.get(5)?,
        interval_minutes: row.get::<_, i64>(6)? as u32,
        total_snapshots: row.get::<_, i64>(7)? as u32,
        fetched_count: row.get::<_, i64>(8)? as u32,
        estimated_llm_calls: row.get::<_, i64>(9)? as u32,
        error_message: row.get(10)?,
        created_at: row.get(11)?,
        completed_at: row.get(12)?,
    })
}

fn fetch_failure_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BacktestFetchFailureDto> {
    Ok(BacktestFetchFailureDto {
        failure_id: row.get(0)?,
        dataset_id: row.get(1)?,
        symbol: row.get(2)?,
        captured_at: row.get(3)?,
        timeframe: row.get(4)?,
        stage: row.get(5)?,
        reason: row.get(6)?,
        error_detail: row.get(7)?,
        created_at: row.get(8)?,
    })
}

fn run_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BacktestRunDto> {
    Ok(BacktestRunDto {
        backtest_id: row.get(0)?,
        dataset_id: row.get(1)?,
        name: row.get(2)?,
        status: row.get(3)?,
        model_provider: row.get(4)?,
        model_name: row.get(5)?,
        prompt_version: row.get(6)?,
        max_holding_days: row.get::<_, i64>(7)? as u32,
        total_ai_calls: row.get::<_, i64>(8)? as u32,
        processed_ai_calls: row.get::<_, i64>(9)? as u32,
        total_timepoints: row.get::<_, i64>(10)? as u32,
        processed_timepoints: row.get::<_, i64>(11)? as u32,
        total_signals: row.get::<_, i64>(12)? as u32,
        trade_signals: row.get::<_, i64>(13)? as u32,
        open_trades: row.get::<_, i64>(14)? as u32,
        win_count: row.get::<_, i64>(15)? as u32,
        loss_count: row.get::<_, i64>(16)? as u32,
        flat_count: row.get::<_, i64>(17)? as u32,
        total_pnl_cny: row.get(18)?,
        total_pnl_percent: row.get(19)?,
        max_drawdown_percent: row.get(20)?,
        profit_factor: row.get(21)?,
        error_message: row.get(22)?,
        created_at: row.get(23)?,
        completed_at: row.get(24)?,
    })
}

fn signal_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BacktestSignalDto> {
    Ok(BacktestSignalDto {
        signal_id: row.get(0)?,
        backtest_id: row.get(1)?,
        symbol: row.get(2)?,
        stock_name: row.get(3)?,
        captured_at: row.get(4)?,
        has_trade: row.get::<_, i64>(5)? != 0,
        direction: row.get(6)?,
        confidence_score: row.get(7)?,
        risk_status: row.get(8)?,
        entry_low: row.get(9)?,
        entry_high: row.get(10)?,
        stop_loss: row.get(11)?,
        take_profit: row.get(12)?,
        amount_cny: row.get(13)?,
        max_loss_cny: row.get(14)?,
        rationale: row.get(15)?,
        result: row.get(16)?,
    })
}

fn trade_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BacktestTradeDto> {
    Ok(BacktestTradeDto {
        trade_id: row.get(0)?,
        backtest_id: row.get(1)?,
        signal_id: row.get(2)?,
        symbol: row.get(3)?,
        stock_name: row.get(4)?,
        direction: row.get(5)?,
        entry_price: row.get(6)?,
        entry_at: row.get(7)?,
        exit_price: row.get(8)?,
        exit_at: row.get(9)?,
        exit_reason: row.get(10)?,
        stop_loss: row.get(11)?,
        take_profit: row.get(12)?,
        amount_cny: row.get(13)?,
        holding_periods: row.get::<_, i64>(14)? as u32,
        pnl_cny: row.get(15)?,
        pnl_percent: row.get(16)?,
    })
}

fn load_or_fetch_bars(
    market_data_service: &MarketDataService,
    settings_service: &SettingsService,
    symbol: &str,
    interval: &str,
    count: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> anyhow::Result<Vec<OhlcvBar>> {
    if start_date.is_some() || end_date.is_some() {
        let bars = akshare::fetch_history_bars_in_range_with_settings(
            settings_service,
            symbol,
            interval,
            count,
            start_date,
            end_date,
        )?;
        let _ = market_data_service.cache_candle_bars(symbol, interval, &bars);
        return Ok(bars);
    }
    let cached = market_data_service.cached_candle_bars(symbol, interval, count);
    if cached.len() >= count.min(20) {
        return Ok(cached);
    }
    let bars =
        akshare::fetch_history_bars_with_settings(settings_service, symbol, interval, count)?;
    let _ = market_data_service.cache_candle_bars(symbol, interval, &bars);
    Ok(bars)
}

fn sampled_bars<'a>(
    bars: &'a [OhlcvBar],
    start_date: &str,
    end_date: &str,
    interval_minutes: u32,
) -> Vec<SampledBar<'a>> {
    let targets = expected_timepoints(start_date, end_date, interval_minutes).unwrap_or_default();
    sampled_bars_with_targets(bars, start_date, end_date, &targets)
}

fn sampled_bars_with_targets<'a>(
    bars: &'a [OhlcvBar],
    start_date: &str,
    end_date: &str,
    targets: &[String],
) -> Vec<SampledBar<'a>> {
    let filtered = bars
        .iter()
        .filter(|bar| {
            let normalized = normalize_bar_time(&bar.open_time);
            let date = normalized.get(0..10).unwrap_or("");
            date >= start_date && date <= end_date && is_a_share_session(&normalized)
        })
        .collect::<Vec<_>>();

    let mut selected = Vec::new();
    let mut last_selected_time = String::new();
    for target in targets {
        let target_date = target.get(0..10).unwrap_or("");
        let matched = filtered
            .iter()
            .rev()
            .find(|bar| {
                let normalized = normalize_bar_time(&bar.open_time);
                let date = normalized.get(0..10).unwrap_or("");
                date == target_date
                    && normalized.as_str() <= target.as_str()
                    && normalized > last_selected_time
            })
            .copied()
            .or_else(|| {
                filtered
                    .iter()
                    .find(|bar| {
                        let normalized = normalize_bar_time(&bar.open_time);
                        let date = normalized.get(0..10).unwrap_or("");
                        date == target_date
                            && normalized.as_str() >= target.as_str()
                            && normalized > last_selected_time
                    })
                    .copied()
            });
        if let Some(bar) = matched {
            last_selected_time = normalize_bar_time(&bar.open_time);
            selected.push(SampledBar {
                captured_at: target.clone(),
                bar,
            });
        }
    }
    selected
}

fn append_session_timepoints(
    output: &mut Vec<String>,
    date: &str,
    start_time: &str,
    end_time: &str,
    step_minutes: i64,
) {
    push_session_timepoints(output, date, start_time, end_time, step_minutes);
    let session_end = format!("{date}T{end_time}:00+08:00");
    if output.last() != Some(&session_end) {
        output.push(session_end);
    }
}

fn expected_timepoints(
    start_date: &str,
    end_date: &str,
    interval_minutes: u32,
) -> anyhow::Result<Vec<String>> {
    expected_timepoints_with_trade_day_filter(start_date, end_date, interval_minutes, |date| {
        is_trade_day(date)
    })
}

fn expected_timepoints_with_trade_day_filter<F>(
    start_date: &str,
    end_date: &str,
    interval_minutes: u32,
    mut is_trade_day_fn: F,
) -> anyhow::Result<Vec<String>>
where
    F: FnMut(Date) -> bool,
{
    let mut date = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let mut timepoints = Vec::new();
    let step = interval_minutes.max(5) as i64;
    while date <= end {
        if is_trade_day_fn(date) {
            append_session_timepoints(&mut timepoints, &date.to_string(), "09:30", "11:30", step);
            append_session_timepoints(&mut timepoints, &date.to_string(), "13:00", "15:00", step);
        }
        date = date.next_day().ok_or_else(|| anyhow!("日期范围过大"))?;
    }
    Ok(timepoints)
}

fn push_session_timepoints(
    output: &mut Vec<String>,
    date: &str,
    start_time: &str,
    end_time: &str,
    step_minutes: i64,
) {
    let Some(mut minutes) = minutes_of_day(start_time) else {
        return;
    };
    let Some(end) = minutes_of_day(end_time) else {
        return;
    };
    while minutes <= end {
        output.push(format!(
            "{}T{:02}:{:02}:00+08:00",
            date,
            minutes / 60,
            minutes % 60
        ));
        minutes += step_minutes;
    }
}

fn minutes_of_day(value: &str) -> Option<i64> {
    let (hour, minute) = value.split_once(':')?;
    Some(hour.parse::<i64>().ok()? * 60 + minute.parse::<i64>().ok()?)
}

fn recent_ohlc(bars: &[OhlcvBar], open_time: &str, limit: usize) -> Vec<[f64; 4]> {
    let end = bars
        .iter()
        .rposition(|bar| normalize_bar_time(&bar.open_time) <= normalize_bar_time(open_time))
        .unwrap_or_else(|| bars.len().saturating_sub(1));
    bars.iter()
        .take(end + 1)
        .rev()
        .take(limit)
        .map(|bar| [bar.open, bar.high, bar.low, bar.close])
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn is_a_share_session(open_time: &str) -> bool {
    let time = open_time.get(11..16).unwrap_or("");
    (time >= "09:30" && time <= "11:30") || (time >= "13:00" && time <= "15:00")
}

fn normalize_bar_time(value: &str) -> String {
    if value.contains('T') {
        return value.to_string();
    }
    format!("{}+08:00", value.replace(' ', "T"))
}

fn market_row_from_snapshot(snapshot: &BacktestSnapshot) -> MarketListRow {
    MarketListRow {
        symbol: snapshot.symbol.clone(),
        base_asset: snapshot
            .stock_name
            .clone()
            .unwrap_or_else(|| snapshot.symbol.clone()),
        market_type: "ashare".into(),
        market_cap_usd: None,
        market_cap_rank: None,
        market_size_tier: "small".into(),
        last_price: snapshot.last_price,
        change_24h: snapshot.change_24h,
        volume_24h: snapshot.volume_24h,
        funding_rate: None,
        spread_bps: snapshot.spread_bps,
        exchanges: vec!["akshare:xueqiu".into(), "模拟账户".into()],
        updated_at: snapshot.captured_at.clone(),
        stale: false,
        venue_snapshots: Vec::new(),
        best_bid_exchange: None,
        best_ask_exchange: None,
        best_bid_price: None,
        best_ask_price: None,
        responded_exchange_count: 1,
        fdv_usd: None,
    }
}

fn snapshot_kline_map(snapshot: &BacktestSnapshot) -> HashMap<String, Vec<[f64; 4]>> {
    let dynamic = serde_json::from_str::<HashMap<String, Vec<[f64; 4]>>>(&snapshot.kline_data_json)
        .unwrap_or_default();
    if !dynamic.is_empty() {
        return dynamic;
    }
    [
        ("5m", &snapshot.kline_5m),
        ("1h", &snapshot.kline_1h),
        ("1d", &snapshot.kline_1d),
        ("1w", &snapshot.kline_1w),
    ]
    .into_iter()
    .map(|(interval, payload)| {
        let bars = serde_json::from_str::<Vec<[f64; 4]>>(payload).unwrap_or_default();
        (interval.to_string(), bars)
    })
    .collect()
}

fn configured_ai_kline_frequencies(runtime: &RuntimeSettingsDto) -> Vec<String> {
    let allowed = ["1m", "5m", "30m", "1h", "1d", "1w", "1M"];
    let mut frequencies = Vec::new();
    for frequency in &runtime.ai_kline_frequencies {
        if allowed.contains(&frequency.as_str()) && !frequencies.contains(frequency) {
            frequencies.push(frequency.clone());
        }
    }
    if frequencies.is_empty() {
        crate::models::default_ai_kline_frequencies()
    } else {
        frequencies
    }
}

fn stock_name_from_info(stock_info: &serde_json::Value) -> Option<String> {
    let items = stock_info.get("items").unwrap_or(stock_info);
    ["股票简称", "名称", "name", "org_name_cn", "short_name"]
        .iter()
        .find_map(|key| {
            items
                .get(*key)
                .and_then(|value| value.as_str())
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
}

fn position_context(position: &VirtualPosition) -> PositionContext {
    PositionContext {
        symbol: position.symbol.clone(),
        side: "long".into(),
        size: format!(
            "{:.2}",
            position.amount_cny / position.entry_price.max(0.01)
        ),
        entry_price: position.entry_price,
        mark_price: position.entry_price,
        pnl_percent: 0.0,
    }
}

fn open_position(
    signal_id: &str,
    snapshot: &BacktestSnapshot,
    run: &crate::models::RecommendationRunDto,
) -> Option<VirtualPosition> {
    let entry = match (run.entry_low, run.entry_high) {
        (Some(low), Some(high)) => (low + high) / 2.0,
        (Some(value), None) | (None, Some(value)) => value,
        _ => snapshot.last_price,
    };
    Some(VirtualPosition {
        signal_id: signal_id.to_string(),
        symbol: snapshot.symbol.clone(),
        stock_name: snapshot.stock_name.clone(),
        entry_price: entry,
        entry_at: snapshot.captured_at.clone(),
        stop_loss: run.stop_loss,
        take_profit: run.take_profit.as_deref().and_then(first_price),
        amount_cny: run.amount_cny.unwrap_or(10_000.0),
        holding_periods: 0,
        max_favorable_price: entry,
        max_adverse_price: entry,
    })
}

fn open_position_from_signal(
    signal: &BacktestSignalDto,
    snapshot: &BacktestSnapshot,
) -> Option<VirtualPosition> {
    let entry = match (signal.entry_low, signal.entry_high) {
        (Some(low), Some(high)) => (low + high) / 2.0,
        (Some(value), None) | (None, Some(value)) => value,
        _ => snapshot.last_price,
    };
    Some(VirtualPosition {
        signal_id: signal.signal_id.clone(),
        symbol: signal.symbol.clone(),
        stock_name: signal
            .stock_name
            .clone()
            .or_else(|| snapshot.stock_name.clone()),
        entry_price: entry,
        entry_at: signal.captured_at.clone(),
        stop_loss: signal.stop_loss,
        take_profit: signal.take_profit.as_deref().and_then(first_price),
        amount_cny: signal.amount_cny.unwrap_or(10_000.0),
        holding_periods: 0,
        max_favorable_price: entry,
        max_adverse_price: entry,
    })
}

fn fallback_signal_run(
    snapshot: &BacktestSnapshot,
    runtime: &RuntimeSettingsDto,
    error: &str,
) -> crate::models::RecommendationRunDto {
    let rationale = format!(
        "该历史快照 AI 信号生成失败，已使用观望 fallback；失败原因：{}",
        error
    );
    crate::models::RecommendationRunDto {
        recommendation_id: format!("bt-fallback-{}", Uuid::new_v4()),
        status: "completed".into(),
        trigger_type: "backtest".into(),
        has_trade: false,
        symbol: Some(snapshot.symbol.clone()),
        stock_name: Some(
            snapshot
                .stock_name
                .clone()
                .unwrap_or_else(|| snapshot.symbol.clone()),
        ),
        direction: None,
        market_type: "ashare".into(),
        exchanges: vec!["akshare:xueqiu".into(), "模拟账户".into()],
        confidence_score: 0.0,
        rationale,
        symbol_recommendations: Vec::new(),
        risk_status: "watch".into(),
        entry_low: None,
        entry_high: None,
        stop_loss: None,
        take_profit: None,
        leverage: None,
        amount_cny: None,
        invalidation: None,
        max_loss_cny: None,
        no_trade_reason: Some("AI 信号生成失败，fallback 为观望。".into()),
        risk_details: RiskDecisionDto {
            status: "watch".into(),
            risk_score: 0,
            max_loss_estimate: None,
            checks: Vec::new(),
            modifications: Vec::new(),
            block_reasons: vec![error.to_string()],
        },
        data_snapshot_at: snapshot.captured_at.clone(),
        model_provider: runtime.model_provider.clone(),
        model_name: runtime.model_name.clone(),
        prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
        user_preference_version: "backtest-fallback".into(),
        generated_at: now_rfc3339(),
    }
}

fn maybe_exit_position(
    position: &VirtualPosition,
    snapshot: &BacktestSnapshot,
    max_holding_days: u32,
    captured_at: &str,
) -> Option<BacktestTradeDto> {
    let entry_date = position.entry_at.get(0..10).unwrap_or("");
    let exit_date = captured_at.get(0..10).unwrap_or("");
    if entry_date == exit_date {
        return None;
    }
    if let Some(stop_loss) = position.stop_loss {
        if snapshot.low_price <= stop_loss {
            return Some(close_position(
                position,
                stop_loss,
                captured_at,
                "stop_loss",
            ));
        }
    }
    if let Some(take_profit) = position.take_profit {
        if snapshot.high_price >= take_profit {
            return Some(close_position(
                position,
                take_profit,
                captured_at,
                "take_profit",
            ));
        }
    }
    if position.holding_periods >= max_holding_days.saturating_mul(8) {
        return Some(close_position(
            position,
            snapshot.last_price,
            captured_at,
            "timeout",
        ));
    }
    None
}

fn close_position(
    position: &VirtualPosition,
    exit_price: f64,
    exit_at: &str,
    reason: &str,
) -> BacktestTradeDto {
    let pnl_percent =
        ((exit_price - position.entry_price) / position.entry_price - COST_RATE) * 100.0;
    let pnl_cny = position.amount_cny * pnl_percent / 100.0;
    BacktestTradeDto {
        trade_id: format!("trade-{}", Uuid::new_v4()),
        backtest_id: String::new(),
        signal_id: Some(position.signal_id.clone()),
        symbol: position.symbol.clone(),
        stock_name: position.stock_name.clone(),
        direction: "long".into(),
        entry_price: round2(position.entry_price),
        entry_at: position.entry_at.clone(),
        exit_price: round2(exit_price),
        exit_at: exit_at.to_string(),
        exit_reason: reason.into(),
        stop_loss: position.stop_loss,
        take_profit: position.take_profit,
        amount_cny: Some(position.amount_cny),
        holding_periods: position.holding_periods,
        pnl_cny: round2(pnl_cny),
        pnl_percent: round2(pnl_percent),
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn position_market_value_cny(position: &VirtualPosition, mark_price: f64) -> f64 {
    let quantity = position.amount_cny / position.entry_price.max(0.01);
    quantity * mark_price
}

async fn complete_backtest_signal_with_retry<F, Fut>(
    timeout_duration: Duration,
    mut runner: F,
) -> anyhow::Result<llm::GeneratedTradePlan>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<llm::GeneratedTradePlan>>,
{
    let mut last_error = String::new();
    for attempt in 1..=BACKTEST_SIGNAL_RETRY_LIMIT {
        match tokio::time::timeout(timeout_duration, runner()).await {
            Ok(Ok(plan)) => return Ok(plan),
            Ok(Err(error)) => {
                last_error = format!("第 {attempt} 次尝试失败：{error}");
            }
            Err(_) => {
                last_error = format!(
                    "第 {attempt} 次尝试失败：调用模型超时（>{}秒）",
                    timeout_duration.as_secs()
                );
            }
        }
        if attempt < BACKTEST_SIGNAL_RETRY_LIMIT {
            continue;
        }
    }
    bail!("AI 信号生成失败（已重试 {BACKTEST_SIGNAL_RETRY_LIMIT} 次）：{last_error}")
}

fn first_price(value: &str) -> Option<f64> {
    value
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
        .find_map(|part| part.parse::<f64>().ok())
}

fn estimate_llm_calls(start_date: &str, end_date: &str, interval_minutes: u32) -> u32 {
    let days = trading_day_count(start_date, end_date).unwrap_or(1);
    let slots_per_day = (240 / interval_minutes.max(5)).max(1);
    days.saturating_mul(slots_per_day)
}

fn planned_snapshot_count(
    start_date: &str,
    end_date: &str,
    interval_minutes: u32,
    symbol_count: usize,
) -> anyhow::Result<u32> {
    Ok((expected_timepoints(start_date, end_date, interval_minutes)?.len() * symbol_count) as u32)
}

fn trading_day_count(start_date: &str, end_date: &str) -> anyhow::Result<u32> {
    trading_day_count_with_filter(start_date, end_date, |date| is_trade_day(date))
}

fn trading_day_count_with_filter<F>(
    start_date: &str,
    end_date: &str,
    mut is_trade_day_fn: F,
) -> anyhow::Result<u32>
where
    F: FnMut(Date) -> bool,
{
    let mut date = parse_date(start_date)?;
    let end = parse_date(end_date)?;
    let mut count = 0;
    while date <= end {
        if is_trade_day_fn(date) {
            count += 1;
        }
        date = date.next_day().ok_or_else(|| anyhow!("日期范围过大"))?;
    }
    Ok(count)
}

fn is_trade_day(date: Date) -> bool {
    let date_string = date.to_string();
    akshare::is_trade_date(&date_string).unwrap_or_else(|_| {
        let weekday = date.weekday().number_from_monday();
        weekday <= 5
    })
}

fn parse_date(value: &str) -> anyhow::Result<Date> {
    let parts = value
        .split('-')
        .map(|part| part.parse::<u32>())
        .collect::<Result<Vec<_>, _>>()?;
    if parts.len() != 3 {
        bail!("日期格式必须是 YYYY-MM-DD");
    }
    Ok(Date::from_calendar_date(
        parts[0] as i32,
        Month::try_from(parts[1] as u8)?,
        parts[2] as u8,
    )?)
}

fn max_drawdown(trades: &[BacktestTradeDto]) -> f64 {
    let mut equity = 0.0;
    let mut peak: f64 = 0.0;
    let mut drawdown: f64 = 0.0;
    for trade in trades {
        equity += trade.pnl_percent;
        peak = peak.max(equity);
        drawdown = drawdown.max(peak - equity);
    }
    round2(drawdown)
}

fn max_drawdown_from_curve(curve: &[BacktestEquityPointDto]) -> f64 {
    let mut peak: f64 = 0.0;
    let mut drawdown: f64 = 0.0;
    for point in curve {
        peak = peak.max(point.cumulative_pnl_percent);
        drawdown = drawdown.max(peak - point.cumulative_pnl_percent);
    }
    round2(drawdown)
}

fn equity_curve_from_timepoints(
    timepoints: &[String],
    trades: &[BacktestTradeDto],
) -> Vec<BacktestEquityPointDto> {
    let mut pnl_by_exit = BTreeMap::<String, f64>::new();
    for trade in trades {
        *pnl_by_exit.entry(trade.exit_at.clone()).or_default() += trade.pnl_percent;
    }

    let mut cumulative = 0.0;
    let mut exits = pnl_by_exit.into_iter().peekable();
    timepoints
        .iter()
        .map(|captured_at| {
            while matches!(
                exits.peek(),
                Some((exit_at, _)) if exit_at.as_str() <= captured_at.as_str()
            ) {
                if let Some((_, pnl_percent)) = exits.next() {
                    cumulative += pnl_percent;
                }
            }
            BacktestEquityPointDto {
                captured_at: captured_at.clone(),
                cumulative_pnl_percent: round2(cumulative),
            }
        })
        .collect()
}

fn equity_curve_from_trade_exits(trades: &[BacktestTradeDto]) -> Vec<BacktestEquityPointDto> {
    let mut cumulative = 0.0;
    let mut pnl_by_exit = BTreeMap::<String, f64>::new();
    for trade in trades {
        *pnl_by_exit.entry(trade.exit_at.clone()).or_default() += trade.pnl_percent;
    }
    pnl_by_exit
        .into_iter()
        .map(|(captured_at, pnl_percent)| {
            cumulative += pnl_percent;
            BacktestEquityPointDto {
                captured_at,
                cumulative_pnl_percent: round2(cumulative),
            }
        })
        .collect()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn backtest_financial_context(
    financial_report_service: &FinancialReportService,
    runtime: &RuntimeSettingsDto,
    stock_code: &str,
) -> Option<crate::models::AiFinancialReportContextDto> {
    if !runtime.use_financial_report_data {
        return None;
    }
    financial_report_service
        .shared_ai_financial_context(stock_code)
        .ok()
        .flatten()
}

fn backtest_sentiment_context(
    sentiment_analysis_service: &SentimentAnalysisService,
    stock_code: &str,
) -> Option<crate::models::AiSentimentAnalysisContextDto> {
    sentiment_analysis_service
        .shared_ai_sentiment_context(stock_code)
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn test_service() -> BacktestService {
        let path =
            std::env::temp_dir().join(format!("kittyred-backtest-{}.sqlite", Uuid::new_v4()));
        BacktestService::new(path).expect("backtest service should open")
    }

    fn insert_dataset(service: &BacktestService, symbols: &[&str]) -> BacktestDatasetDto {
        let dataset_id = format!("dataset-{}", Uuid::new_v4());
        let symbols_json = serde_json::to_string(&symbols).expect("symbols should encode");
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_datasets (
                  dataset_id, name, status, symbols_json, start_date, end_date,
                  interval_minutes, estimated_llm_calls, created_at
                ) VALUES (?1, '测试数据集', 'pending', ?2, '2026-05-06', '2026-05-06', 5, 8, ?3)",
                params![dataset_id, symbols_json, now_rfc3339()],
            )
            .expect("dataset should insert");
        service
            .load_dataset(&dataset_id)
            .expect("dataset should load")
            .expect("dataset should exist")
    }

    fn bar(open_time: &str, close: f64) -> OhlcvBar {
        OhlcvBar {
            open_time: open_time.into(),
            open: close,
            high: close + 0.1,
            low: (close - 0.1).max(0.0),
            close,
            volume: 1000.0,
            turnover: Some(10_000.0),
        }
    }

    fn runtime_settings() -> RuntimeSettingsDto {
        let path = std::env::temp_dir().join(format!("kittyred-settings-{}.json", Uuid::new_v4()));
        crate::settings::SettingsService::new(path).get_runtime_settings()
    }

    fn financial_report_service_with_analysis() -> FinancialReportService {
        let path =
            std::env::temp_dir().join(format!("kittyred-financial-{}.sqlite", Uuid::new_v4()));
        let service =
            FinancialReportService::new(path).expect("financial report service should open");
        service
            .seed_test_analysis("SHSE.600000")
            .expect("analysis should seed");
        service
    }

    fn generated_trade_plan() -> llm::GeneratedTradePlan {
        llm::GeneratedTradePlan {
            run: crate::models::RecommendationRunDto {
                recommendation_id: format!("rec-{}", Uuid::new_v4()),
                status: "completed".into(),
                trigger_type: "backtest".into(),
                has_trade: false,
                symbol: Some("SHSE.600000".into()),
                stock_name: Some("浦发银行".into()),
                direction: None,
                market_type: "ashare".into(),
                exchanges: vec!["akshare:xueqiu".into()],
                confidence_score: 0.0,
                rationale: "测试".into(),
                symbol_recommendations: Vec::new(),
                risk_status: "watch".into(),
                entry_low: None,
                entry_high: None,
                stop_loss: None,
                take_profit: None,
                leverage: None,
                amount_cny: None,
                invalidation: None,
                max_loss_cny: None,
                no_trade_reason: Some("测试".into()),
                risk_details: RiskDecisionDto::default(),
                data_snapshot_at: "2026-05-06T09:30:00+08:00".into(),
                model_provider: "OpenAI-compatible".into(),
                model_name: "gpt-test".into(),
                prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
                user_preference_version: "test".into(),
                generated_at: now_rfc3339(),
            },
            ai_raw_output: "{}".into(),
            ai_structured_output: "{}".into(),
            system_prompt: "system".into(),
            user_prompt: "user".into(),
        }
    }

    #[test]
    fn fetch_records_failed_timepoint_and_continues_same_symbol() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        let cancel = AtomicBool::new(false);
        let runtime = runtime_settings();

        let outcome = service
            .fetch_snapshots_inner_with_loaders(
                &dataset,
                &runtime,
                &cancel,
                |_, interval, _, _, _| {
                    if interval == "5m" {
                        Ok(vec![
                            bar("2026-05-06 09:30:00", 8.7),
                            bar("2026-05-06 10:00:00", 0.0),
                            bar("2026-05-06 10:30:00", 8.9),
                        ])
                    } else {
                        Ok(vec![bar("2026-05-06 09:30:00", 8.7)])
                    }
                },
                |_| serde_json::json!({"items": {"名称": "浦发银行"}}),
            )
            .expect("partial timepoint failure should not fail dataset");

        assert_eq!(outcome.inserted, 2);
        assert_eq!(outcome.total, 3);
        assert_eq!(outcome.failures, 1);
        let failures = service
            .list_fetch_failures(&dataset.dataset_id, 10)
            .expect("failures should list");
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].symbol, "SHSE.600000");
        assert_eq!(
            failures[0].captured_at.as_deref(),
            Some("2026-05-06T09:35:00+08:00")
        );
    }

    #[test]
    fn fetch_requests_history_with_dataset_date_range() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        let cancel = AtomicBool::new(false);
        let runtime = runtime_settings();
        let requests = Arc::new(Mutex::new(Vec::<(
            String,
            String,
            Option<String>,
            Option<String>,
        )>::new()));
        let requests_for_loader = requests.clone();

        service
            .fetch_snapshots_inner_with_loaders(
                &dataset,
                &runtime,
                &cancel,
                move |symbol, interval, _, start_date, end_date| {
                    requests_for_loader
                        .lock()
                        .expect("requests lock poisoned")
                        .push((
                            symbol.to_string(),
                            interval.to_string(),
                            start_date.map(ToString::to_string),
                            end_date.map(ToString::to_string),
                        ));
                    Ok(vec![bar("2026-05-06 09:30:00", 8.7)])
                },
                |_| serde_json::json!({"items": {"名称": "浦发银行"}}),
            )
            .expect("fetch should succeed");

        let requests = requests.lock().expect("requests lock poisoned");
        assert!(requests.iter().any(|(_, interval, start_date, end_date)| {
            interval == "5m"
                && start_date.as_deref() == Some(dataset.start_date.as_str())
                && end_date.as_deref() == Some(dataset.end_date.as_str())
        }));
    }

    #[test]
    fn fetch_fails_when_no_snapshots_are_produced() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        let cancel = AtomicBool::new(false);
        let runtime = runtime_settings();

        let error = service
            .fetch_snapshots_inner_with_loaders(
                &dataset,
                &runtime,
                &cancel,
                |_, _, _, _, _| Err(anyhow!("sina ssl disconnected")),
                |_| serde_json::json!({}),
            )
            .expect_err("zero inserted snapshots should fail");

        assert!(error.to_string().contains("没有拉取到可用历史快照"));
        assert!(
            service
                .list_fetch_failures(&dataset.dataset_id, 500)
                .expect("failures should list")
                .len()
                > 0
        );
    }

    #[tokio::test]
    async fn backtest_signal_retries_three_times_before_failure() {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempts_for_runner = attempts.clone();

        let error = complete_backtest_signal_with_retry(Duration::from_millis(20), move || {
            let attempts_for_runner = attempts_for_runner.clone();
            async move {
                attempts_for_runner.fetch_add(1, Ordering::SeqCst);
                bail!("模型繁忙");
            }
        })
        .await
        .expect_err("runner should fail after retries");

        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert!(error.to_string().contains("已重试 3 次"));
    }

    #[tokio::test]
    async fn backtest_signal_timeout_retries_three_times_before_failure() {
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let attempts_for_runner = attempts.clone();

        let error = complete_backtest_signal_with_retry(Duration::from_millis(1), move || {
            let attempts_for_runner = attempts_for_runner.clone();
            async move {
                attempts_for_runner.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(generated_trade_plan())
            }
        })
        .await
        .expect_err("runner should timeout after retries");

        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert!(error.to_string().contains("调用模型超时"));
    }

    #[test]
    fn summary_equity_curve_uses_snapshot_timepoints() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        for (captured_at, price) in [
            ("2026-05-06T09:30:00+08:00", 8.7),
            ("2026-05-06T10:00:00+08:00", 8.8),
            ("2026-05-06T10:30:00+08:00", 8.9),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    "SHSE.600000",
                    Some("浦发银行"),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'completed', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![
                    backtest_id,
                    dataset.dataset_id,
                    RECOMMENDATION_PROMPT_VERSION,
                    now_rfc3339()
                ],
            )
            .expect("run should insert");

        let trades = vec![
            BacktestTradeDto {
                trade_id: format!("trade-{}", Uuid::new_v4()),
                backtest_id: backtest_id.clone(),
                signal_id: None,
                symbol: "SHSE.600000".into(),
                stock_name: Some("浦发银行".into()),
                direction: "long".into(),
                entry_price: 8.7,
                entry_at: "2026-05-06T09:30:00+08:00".into(),
                exit_price: 8.9,
                exit_at: "2026-05-06T10:00:00+08:00".into(),
                exit_reason: "take_profit".into(),
                stop_loss: None,
                take_profit: None,
                amount_cny: Some(10000.0),
                holding_periods: 1,
                pnl_cny: 200.0,
                pnl_percent: 2.0,
            },
            BacktestTradeDto {
                trade_id: format!("trade-{}", Uuid::new_v4()),
                backtest_id: backtest_id.clone(),
                signal_id: None,
                symbol: "SHSE.600000".into(),
                stock_name: Some("浦发银行".into()),
                direction: "long".into(),
                entry_price: 8.8,
                entry_at: "2026-05-06T09:30:00+08:00".into(),
                exit_price: 8.75,
                exit_at: "2026-05-06T10:00:00+08:00".into(),
                exit_reason: "stop_loss".into(),
                stop_loss: None,
                take_profit: None,
                amount_cny: Some(10000.0),
                holding_periods: 1,
                pnl_cny: -50.0,
                pnl_percent: -0.5,
            },
        ];
        for trade in &trades {
            service.insert_trade(trade).expect("trade should insert");
        }
        let equity_curve = vec![];
        service
            .complete_run(&backtest_id, &trades, &equity_curve, 0, 150.0, 1.5)
            .expect("run should complete");

        let summary = service.summary(&backtest_id).expect("summary should load");

        assert_eq!(summary.equity_curve.len(), 3);
        assert_eq!(
            summary
                .equity_curve
                .iter()
                .map(|point| point.captured_at.as_str())
                .collect::<Vec<_>>(),
            vec![
                "2026-05-06T09:30:00+08:00",
                "2026-05-06T10:00:00+08:00",
                "2026-05-06T10:30:00+08:00"
            ]
        );
        assert_eq!(
            summary
                .equity_curve
                .iter()
                .map(|point| point.cumulative_pnl_percent)
                .collect::<Vec<_>>(),
            vec![0.0, 1.5, 1.5]
        );
    }

    #[tokio::test]
    async fn replay_keeps_open_positions_without_forced_backtest_end_close() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        for (captured_at, price) in [
            ("2026-05-06T09:30:00+08:00", 8.7),
            ("2026-05-07T10:00:00+08:00", 8.9),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    "SHSE.600000",
                    Some("浦发银行"),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'signals_ready', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![
                    backtest_id,
                    dataset.dataset_id,
                    RECOMMENDATION_PROMPT_VERSION,
                    now_rfc3339()
                ],
            )
            .expect("run should insert");

        let snapshot = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load")
            .into_iter()
            .find(|item| item.captured_at == "2026-05-06T09:30:00+08:00")
            .expect("first snapshot should exist");
        let signal_run = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 70.0,
            rationale: "测试开仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(8.7),
            entry_high: Some(8.7),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(10_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-1",
                &snapshot,
                &signal_run,
                "{}",
                "{}",
                "opened",
            )
            .expect("signal should insert");

        let run = service
            .load_run(&backtest_id)
            .expect("run should load")
            .expect("run should exist");
        let cancel = AtomicBool::new(false);
        service
            .replay_trades_inner(&run, &cancel)
            .await
            .expect("replay should succeed");

        let trades = service
            .list_trades(&backtest_id)
            .expect("trades should load");
        assert!(trades.is_empty());

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert_eq!(summary.trade_count, 1);
        assert_eq!(summary.open_positions.len(), 1);
        assert_eq!(summary.open_positions[0].symbol, "SHSE.600000");
        assert!(summary.open_positions[0].unrealized_pnl_percent > 0.0);
    }

    #[test]
    fn summary_ignores_generated_signals_before_replay() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        for (captured_at, price) in [
            ("2026-05-06T09:30:00+08:00", 10.0),
            ("2026-05-07T09:30:00+08:00", 11.0),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    "SHSE.600000",
                    Some("浦发银行"),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'signals_ready', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![backtest_id, dataset.dataset_id, RECOMMENDATION_PROMPT_VERSION, now_rfc3339()],
            )
            .expect("run should insert");

        let snapshot = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load")
            .into_iter()
            .next()
            .expect("snapshot should exist");
        let signal_run = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 70.0,
            rationale: "测试开仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(10.0),
            entry_high: Some(10.0),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(10_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-1",
                &snapshot,
                &signal_run,
                "{}",
                "{}",
                "generated",
            )
            .expect("signal should insert");

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert!(summary.open_positions.is_empty());
    }

    #[tokio::test]
    async fn replay_respects_initial_capital_and_skips_orders_when_cash_is_insufficient() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000", "SZSE.000001"]);
        for (symbol, name, price) in [
            ("SHSE.600000", "浦发银行", 10.0),
            ("SZSE.000001", "平安银行", 20.0),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    symbol,
                    Some(name),
                    "2026-05-06T09:30:00+08:00",
                    &bar("2026-05-06 09:30:00", price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'signals_ready', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![backtest_id, dataset.dataset_id, RECOMMENDATION_PROMPT_VERSION, now_rfc3339()],
            )
            .expect("run should insert");

        let snapshots = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load");
        for (signal_id, symbol, stock_name) in [
            ("sig-1", "SHSE.600000", "浦发银行"),
            ("sig-2", "SZSE.000001", "平安银行"),
        ] {
            let snapshot = snapshots
                .iter()
                .find(|item| item.symbol == symbol)
                .expect("snapshot should exist");
            let signal_run = crate::models::RecommendationRunDto {
                recommendation_id: format!("rec-{}", Uuid::new_v4()),
                status: "completed".into(),
                trigger_type: "backtest".into(),
                has_trade: true,
                symbol: Some(symbol.into()),
                stock_name: Some(stock_name.into()),
                direction: Some("买入".into()),
                market_type: "ashare".into(),
                exchanges: vec!["akshare:xueqiu".into()],
                confidence_score: 70.0,
                rationale: "测试开仓".into(),
                symbol_recommendations: Vec::new(),
                risk_status: "approved".into(),
                entry_low: Some(snapshot.last_price),
                entry_high: Some(snapshot.last_price),
                stop_loss: None,
                take_profit: None,
                leverage: None,
                amount_cny: Some(600_000.0),
                invalidation: None,
                max_loss_cny: None,
                no_trade_reason: None,
                risk_details: RiskDecisionDto::default(),
                data_snapshot_at: snapshot.captured_at.clone(),
                model_provider: "OpenAI-compatible".into(),
                model_name: "gpt-test".into(),
                prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
                user_preference_version: "test".into(),
                generated_at: now_rfc3339(),
            };
            service
                .insert_signal(
                    &backtest_id,
                    signal_id,
                    snapshot,
                    &signal_run,
                    "{}",
                    "{}",
                    "opened",
                )
                .expect("signal should insert");
        }

        let run = service
            .load_run(&backtest_id)
            .expect("run should load")
            .expect("run should exist");
        service
            .replay_trades_inner(&run, &AtomicBool::new(false))
            .await
            .expect("replay should succeed");

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert_eq!(summary.open_positions.len(), 1);
        assert_eq!(summary.open_positions[0].signal_id, "sig-1");
    }

    #[tokio::test]
    async fn replay_equity_curve_uses_total_equity_instead_of_trade_percent_sum() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000"]);
        for (captured_at, price) in [
            ("2026-05-06T09:30:00+08:00", 10.0),
            ("2026-05-07T10:00:00+08:00", 11.0),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    "SHSE.600000",
                    Some("浦发银行"),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'signals_ready', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![backtest_id, dataset.dataset_id, RECOMMENDATION_PROMPT_VERSION, now_rfc3339()],
            )
            .expect("run should insert");

        let snapshot = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load")
            .into_iter()
            .find(|item| item.captured_at == "2026-05-06T09:30:00+08:00")
            .expect("snapshot should exist");
        let signal_run = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 70.0,
            rationale: "测试开仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(10.0),
            entry_high: Some(10.0),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(500_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-1",
                &snapshot,
                &signal_run,
                "{}",
                "{}",
                "opened",
            )
            .expect("signal should insert");

        let run = service
            .load_run(&backtest_id)
            .expect("run should load")
            .expect("run should exist");
        service
            .replay_trades_inner(&run, &AtomicBool::new(false))
            .await
            .expect("replay should succeed");

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert_eq!(
            summary
                .equity_curve
                .iter()
                .map(|point| point.cumulative_pnl_percent)
                .collect::<Vec<_>>(),
            vec![0.0, 5.0]
        );
        assert_eq!(summary.total_pnl_cny, 50_000.0);
        assert_eq!(summary.total_pnl_percent, 5.0);
    }

    #[tokio::test]
    async fn replay_equity_curve_carries_forward_latest_price_when_symbol_missing_current_timepoint(
    ) {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000", "SZSE.000001"]);
        service
            .insert_snapshot(
                &dataset,
                "SHSE.600000",
                Some("浦发银行"),
                "2026-05-06T09:35:00+08:00",
                &bar("2026-05-06 09:35:00", 10.0),
                0.0,
                1000.0,
                "[]".into(),
                "[]".into(),
                "[]".into(),
                "[]".into(),
                "{}".into(),
                "{}".into(),
            )
            .expect("sparse snapshot should insert");
        for captured_at in ["2026-05-06T09:35:00+08:00", "2026-05-06T09:40:00+08:00"] {
            service
                .insert_snapshot(
                    &dataset,
                    "SZSE.000001",
                    Some("平安银行"),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), 20.0),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("reference snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'signals_ready', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![backtest_id, dataset.dataset_id, RECOMMENDATION_PROMPT_VERSION, now_rfc3339()],
            )
            .expect("run should insert");

        let snapshot = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load")
            .into_iter()
            .find(|item| item.symbol == "SHSE.600000")
            .expect("opening snapshot should exist");
        let signal_run = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SHSE.600000".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 70.0,
            rationale: "测试开仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(10.0),
            entry_high: Some(10.0),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(500_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-1",
                &snapshot,
                &signal_run,
                "{}",
                "{}",
                "opened",
            )
            .expect("signal should insert");

        let run = service
            .load_run(&backtest_id)
            .expect("run should load")
            .expect("run should exist");
        service
            .replay_trades_inner(&run, &AtomicBool::new(false))
            .await
            .expect("replay should succeed");

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert_eq!(
            summary
                .equity_curve
                .iter()
                .map(|point| point.cumulative_pnl_percent)
                .collect::<Vec<_>>(),
            vec![0.0, 0.0]
        );
    }

    #[test]
    fn summary_counts_open_positions_in_trade_count_and_win_rate() {
        let service = test_service();
        let dataset = insert_dataset(&service, &["SHSE.600000", "SZSE.000001", "SZSE.000002"]);
        for (symbol, name, captured_at, price) in [
            ("SHSE.600000", "浦发银行", "2026-05-06T09:30:00+08:00", 10.0),
            ("SHSE.600000", "浦发银行", "2026-05-07T09:30:00+08:00", 11.0),
            ("SZSE.000001", "平安银行", "2026-05-06T09:30:00+08:00", 20.0),
            ("SZSE.000001", "平安银行", "2026-05-07T09:30:00+08:00", 21.0),
            ("SZSE.000002", "万 科Ａ", "2026-05-06T09:30:00+08:00", 30.0),
            ("SZSE.000002", "万 科Ａ", "2026-05-07T09:30:00+08:00", 29.0),
        ] {
            service
                .insert_snapshot(
                    &dataset,
                    symbol,
                    Some(name),
                    captured_at,
                    &bar(&captured_at.replace('T', " ").replace("+08:00", ""), price),
                    0.0,
                    1000.0,
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "[]".into(),
                    "{}".into(),
                    "{}".into(),
                )
                .expect("snapshot should insert");
        }

        let backtest_id = format!("bt-{}", Uuid::new_v4());
        service
            .db
            .lock()
            .expect("backtest db lock poisoned")
            .connection()
            .execute(
                "INSERT INTO backtest_runs (
                  backtest_id, dataset_id, name, status, model_provider, model_name,
                  prompt_version, risk_settings_json, max_holding_days, created_at
                ) VALUES (?1, ?2, '测试回测', 'completed', 'OpenAI-compatible', 'gpt-test', ?3, '{}', 7, ?4)",
                params![
                    backtest_id,
                    dataset.dataset_id,
                    RECOMMENDATION_PROMPT_VERSION,
                    now_rfc3339()
                ],
            )
            .expect("run should insert");

        let closed_trade = BacktestTradeDto {
            trade_id: format!("trade-{}", Uuid::new_v4()),
            backtest_id: backtest_id.clone(),
            signal_id: None,
            symbol: "SHSE.600000".into(),
            stock_name: Some("浦发银行".into()),
            direction: "long".into(),
            entry_price: 10.0,
            entry_at: "2026-05-06T09:30:00+08:00".into(),
            exit_price: 11.0,
            exit_at: "2026-05-07T09:30:00+08:00".into(),
            exit_reason: "take_profit".into(),
            stop_loss: None,
            take_profit: None,
            amount_cny: Some(10_000.0),
            holding_periods: 8,
            pnl_cny: 1_000.0,
            pnl_percent: 10.0,
        };
        service
            .insert_trade(&closed_trade)
            .expect("closed trade should insert");

        let snapshots = service
            .load_snapshots(&dataset.dataset_id)
            .expect("snapshots should load");

        let positive_open_snapshot = snapshots
            .iter()
            .find(|item| {
                item.symbol == "SZSE.000001" && item.captured_at == "2026-05-06T09:30:00+08:00"
            })
            .expect("positive snapshot should exist")
            .clone();
        let positive_open_signal = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SZSE.000001".into()),
            stock_name: Some("平安银行".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 80.0,
            rationale: "测试持仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(20.0),
            entry_high: Some(20.0),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(10_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: positive_open_snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-open-win",
                &positive_open_snapshot,
                &positive_open_signal,
                "{}",
                "{}",
                "opened",
            )
            .expect("positive open signal should insert");

        let negative_open_snapshot = snapshots
            .iter()
            .find(|item| {
                item.symbol == "SZSE.000002" && item.captured_at == "2026-05-06T09:30:00+08:00"
            })
            .expect("negative snapshot should exist")
            .clone();
        let negative_open_signal = crate::models::RecommendationRunDto {
            recommendation_id: format!("rec-{}", Uuid::new_v4()),
            status: "completed".into(),
            trigger_type: "backtest".into(),
            has_trade: true,
            symbol: Some("SZSE.000002".into()),
            stock_name: Some("万 科Ａ".into()),
            direction: Some("买入".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare:xueqiu".into()],
            confidence_score: 80.0,
            rationale: "测试持仓".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(30.0),
            entry_high: Some(30.0),
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: Some(10_000.0),
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: None,
            risk_details: RiskDecisionDto::default(),
            data_snapshot_at: negative_open_snapshot.captured_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-test".into(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version: "test".into(),
            generated_at: now_rfc3339(),
        };
        service
            .insert_signal(
                &backtest_id,
                "sig-open-loss",
                &negative_open_snapshot,
                &negative_open_signal,
                "{}",
                "{}",
                "opened",
            )
            .expect("negative open signal should insert");

        let summary = service.summary(&backtest_id).expect("summary should load");
        assert_eq!(summary.trade_count, 4);
        assert_eq!(summary.win_rate, 75.0);
        assert_eq!(summary.open_positions.len(), 2);
    }

    #[test]
    fn expected_timepoints_always_include_session_close() {
        let timepoints =
            expected_timepoints("2026-05-06", "2026-05-06", 45).expect("timepoints should build");

        assert_eq!(
            timepoints,
            vec![
                "2026-05-06T09:30:00+08:00",
                "2026-05-06T10:15:00+08:00",
                "2026-05-06T11:00:00+08:00",
                "2026-05-06T11:30:00+08:00",
                "2026-05-06T13:00:00+08:00",
                "2026-05-06T13:45:00+08:00",
                "2026-05-06T14:30:00+08:00",
                "2026-05-06T15:00:00+08:00",
            ]
        );
    }

    #[test]
    fn expected_timepoints_skip_holidays_when_calendar_marks_them_closed() {
        let timepoints =
            expected_timepoints_with_trade_day_filter("2026-05-01", "2026-05-07", 60, |date| {
                matches!(date.to_string().as_str(), "2026-05-06" | "2026-05-07")
            })
            .expect("timepoints should build");

        assert_eq!(
            timepoints,
            vec![
                "2026-05-06T09:30:00+08:00",
                "2026-05-06T10:30:00+08:00",
                "2026-05-06T11:30:00+08:00",
                "2026-05-06T13:00:00+08:00",
                "2026-05-06T14:00:00+08:00",
                "2026-05-06T15:00:00+08:00",
                "2026-05-07T09:30:00+08:00",
                "2026-05-07T10:30:00+08:00",
                "2026-05-07T11:30:00+08:00",
                "2026-05-07T13:00:00+08:00",
                "2026-05-07T14:00:00+08:00",
                "2026-05-07T15:00:00+08:00",
            ]
        );
    }

    #[test]
    fn sampled_bars_align_to_target_timepoints_and_finish_at_market_close() {
        let mut bars = Vec::new();
        let mut price = 8.7;
        for hour in [9, 10, 11] {
            let start_minute = if hour == 9 { 35 } else { 0 };
            let end_minute = if hour == 11 { 30 } else { 55 };
            let mut minute = start_minute;
            while minute <= end_minute {
                bars.push(bar(&format!("2026-05-06 {hour:02}:{minute:02}:00"), price));
                price += 0.01;
                minute += 5;
            }
        }
        for hour in [13, 14, 15] {
            let start_minute = if hour == 13 { 5 } else { 0 };
            let end_minute = if hour == 15 { 0 } else { 55 };
            let mut minute = start_minute;
            while minute <= end_minute {
                bars.push(bar(&format!("2026-05-06 {hour:02}:{minute:02}:00"), price));
                price += 0.01;
                minute += 5;
            }
        }

        let selected = sampled_bars(&bars, "2026-05-06", "2026-05-06", 60);

        assert_eq!(
            selected
                .iter()
                .map(|item| item.captured_at.clone())
                .collect::<Vec<_>>(),
            vec![
                "2026-05-06T09:30:00+08:00",
                "2026-05-06T10:30:00+08:00",
                "2026-05-06T11:30:00+08:00",
                "2026-05-06T13:00:00+08:00",
                "2026-05-06T14:00:00+08:00",
                "2026-05-06T15:00:00+08:00",
            ]
        );
        assert_eq!(
            selected
                .iter()
                .map(|item| normalize_bar_time(&item.bar.open_time))
                .collect::<Vec<_>>(),
            vec![
                "2026-05-06T09:35:00+08:00",
                "2026-05-06T10:30:00+08:00",
                "2026-05-06T11:30:00+08:00",
                "2026-05-06T13:05:00+08:00",
                "2026-05-06T14:00:00+08:00",
                "2026-05-06T15:00:00+08:00",
            ]
        );
    }

    #[test]
    fn sampled_bars_do_not_forward_fill_from_later_trading_day() {
        let bars = vec![
            bar("2026-05-06 09:30:00", 8.7),
            bar("2026-05-06 10:30:00", 8.9),
        ];
        let targets = vec![
            "2026-05-01T09:30:00+08:00".to_string(),
            "2026-05-06T09:30:00+08:00".to_string(),
            "2026-05-06T10:30:00+08:00".to_string(),
        ];

        let selected = sampled_bars_with_targets(&bars, "2026-05-01", "2026-05-06", &targets);

        assert_eq!(
            selected
                .iter()
                .map(|item| item.captured_at.clone())
                .collect::<Vec<_>>(),
            vec!["2026-05-06T09:30:00+08:00", "2026-05-06T10:30:00+08:00",]
        );
        assert_eq!(
            selected
                .iter()
                .map(|item| normalize_bar_time(&item.bar.open_time))
                .collect::<Vec<_>>(),
            vec!["2026-05-06T09:30:00+08:00", "2026-05-06T10:30:00+08:00",]
        );
    }

    #[test]
    fn backtest_financial_context_returns_shared_context_when_enabled() {
        let service = financial_report_service_with_analysis();
        let mut runtime = runtime_settings();
        runtime.use_financial_report_data = true;

        let context = backtest_financial_context(&service, &runtime, "SHSE.600000")
            .expect("context should exist");

        assert_eq!(context.key_summary, "收入和利润稳定");
        assert_eq!(context.radar_scores.profitability, 8.2);
        assert_eq!(context.radar_scores.cash_generation, 8.7);
    }

    #[test]
    fn backtest_financial_context_returns_none_when_disabled() {
        let service = financial_report_service_with_analysis();
        let mut runtime = runtime_settings();
        runtime.use_financial_report_data = false;

        assert!(backtest_financial_context(&service, &runtime, "SHSE.600000").is_none());
    }
}
