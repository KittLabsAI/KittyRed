pub mod config;
pub(crate) mod indicators;
pub mod ledger;
pub mod risk_gate;
pub mod scoring;
pub mod stats;
pub mod strategies;

use crate::signals::config::{merged_params, StrategyConfig, ACTIVE_STRATEGY_IDS};
use crate::signals::stats::{ScanRunRecord, StrategyStats};
use anyhow::Context;
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use strategies::bollinger_break::BollingerBreakStrategy;
use strategies::ma_cross::MaCrossStrategy;
use strategies::macd_divergence::MacdDivergenceStrategy;
use strategies::rsi_extreme::RsiExtremeStrategy;
use strategies::volume_surge::VolumeSurgeStrategy;
use strategies::ApplicableMarket;
use strategies::{SignalContext, SignalDirection, Strategy, StrategySignal};

use crate::market::MarketDataService;
use crate::models::{MarketListRow, RuntimeSettingsDto};
use crate::recommendations::risk_engine::risk_settings_from_runtime;
use ledger::SignalLedger;
use risk_gate::evaluate_signal;
use scoring::{aggregate, ScoringConfig};
use std::path::PathBuf;

const CANDLE_FETCH_CONCURRENCY: usize = 10;
#[cfg(test)]
const PRE_FILTER_TOP_N: usize = 100;

fn score_row(row: &MarketListRow) -> f64 {
    let volume_score = (row.volume_24h.max(1.0).log10() / 12.0).clamp(0.0, 1.0) * 0.4;
    let spread_score = if row.spread_bps <= 5.0 {
        1.0
    } else if row.spread_bps <= 20.0 {
        0.7
    } else if row.spread_bps <= 50.0 {
        0.4
    } else {
        0.1
    } * 0.3;
    let change_score = (1.0 - (row.change_24h.abs() / 20.0).clamp(0.0, 1.0)) * 0.1;
    let exchange_score = ((row.exchanges.len() as f64).min(5.0) / 5.0) * 0.2;
    volume_score + spread_score + change_score + exchange_score
}

fn pre_filter_rows<'a>(
    rows: &'a [MarketListRow],
    watchlist: &HashSet<String>,
    top_n: usize,
) -> Vec<&'a MarketListRow> {
    let mut selected: Vec<&MarketListRow> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    for row in rows {
        if !row.market_type.eq_ignore_ascii_case("ashare") {
            continue;
        }
        let key = (row.symbol.clone(), row.market_type.clone());
        if watchlist.contains(&row.symbol) && seen.insert(key) {
            selected.push(row);
        }
    }

    let mut remaining: Vec<&MarketListRow> = rows
        .iter()
        .filter(|r| r.market_type.eq_ignore_ascii_case("ashare"))
        .filter(|r| !watchlist.contains(&r.symbol))
        .collect();
    remaining.sort_by(|a, b| {
        score_row(b)
            .partial_cmp(&score_row(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let needed = top_n.saturating_sub(selected.len());
    for row in remaining.into_iter().take(needed) {
        let key = (row.symbol.clone(), row.market_type.clone());
        if seen.insert(key) {
            selected.push(row);
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OhlcvBar, VenueTickerSnapshot};

    fn runtime_a_share_row(symbol: &str, volume: f64) -> MarketListRow {
        MarketListRow {
            symbol: symbol.into(),
            base_asset: symbol.into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 100.0,
            change_24h: 3.0,
            volume_24h: volume,
            funding_rate: None,
            spread_bps: 2.0,
            exchanges: vec!["akshare".into()],
            updated_at: "2026-05-03T20:20:00+08:00".into(),
            stale: false,
            venue_snapshots: vec![VenueTickerSnapshot {
                exchange: "akshare".into(),
                last_price: 100.0,
                bid_price: 99.9,
                ask_price: 100.1,
                volume_24h: volume,
                funding_rate: None,
                mark_price: None,
                index_price: None,
                updated_at: "2026-05-03T20:20:00+08:00".into(),
                stale: false,
            }],
            best_bid_exchange: Some("akshare".into()),
            best_ask_exchange: Some("akshare".into()),
            best_bid_price: Some(99.9),
            best_ask_price: Some(100.1),
            responded_exchange_count: 1,
            fdv_usd: None,
        }
    }

    fn runtime_legacy_row(symbol: &str, volume: f64) -> MarketListRow {
        MarketListRow {
            market_type: "spot".into(),
            funding_rate: None,
            ..runtime_a_share_row(symbol, volume)
        }
    }

    fn flat_bars() -> Vec<OhlcvBar> {
        (0..40)
            .map(|index| OhlcvBar {
                open_time: index.to_string(),
                open: 100.0,
                high: 100.0,
                low: 100.0,
                close: 100.0,
                volume: 100.0,
                turnover: None,
            })
            .collect()
    }

    #[test]
    fn pre_filter_rows_keeps_only_a_share_watchlist_rows() {
        let rows = vec![
            runtime_legacy_row("LEGACY.000000", 900_000_000.0),
            MarketListRow {
                market_type: "ashare".into(),
                funding_rate: None,
                exchanges: vec!["akshare".into()],
                best_bid_exchange: Some("akshare".into()),
                best_ask_exchange: Some("akshare".into()),
                ..runtime_a_share_row("SHSE.600000", 100_000_000.0)
            },
        ];
        let watchlist = HashSet::new();

        let candidates = pre_filter_rows(&rows, &watchlist, PRE_FILTER_TOP_N);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].symbol, "SHSE.600000");
        assert_eq!(candidates[0].market_type, "ashare");
    }

    #[test]
    fn signal_registry_excludes_arbitrage_strategies() {
        let registry = SignalRegistry::new();
        let ids = registry
            .strategies
            .iter()
            .map(|strategy| strategy.id())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"ma_cross"));
        assert!(!ids.contains(&"funding_rate"));
        assert!(!ids.contains(&"spread_arbitrage"));
        assert!(!ids.contains(&"cross_market_arbitrage"));
        assert!(!ids.contains(&"basis_deviation"));
    }

    #[test]
    fn registry_returns_raw_result_for_every_strategy() {
        let row = runtime_a_share_row("SHSE.600000", 100_000_000.0);
        let bars = flat_bars();
        let ctx = SignalContext {
            symbol: "SHSE.600000",
            market_type: "ashare",
            row: &row,
            candles: &bars,
            venue_snapshots: &[],
        };
        let configs = ACTIVE_STRATEGY_IDS
            .iter()
            .map(|id| StrategyConfig::default_for(id))
            .collect::<Vec<_>>();

        let signals = SignalRegistry::new().run_all(&ctx, &configs);

        assert_eq!(
            signals
                .iter()
                .map(|signal| signal.strategy_id.as_str())
                .collect::<Vec<_>>(),
            ACTIVE_STRATEGY_IDS.to_vec()
        );
        assert_eq!(signals.len(), ACTIVE_STRATEGY_IDS.len());
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSignal {
    pub signal_id: String,
    pub symbol: String,
    pub market_type: String,
    pub direction: SignalDirection,
    pub score: f64,
    pub strength: f64,
    pub category_breakdown: HashMap<String, f64>,
    pub contributors: Vec<String>,
    pub entry_zone_low: f64,
    pub entry_zone_high: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub reason_summary: String,
    pub risk_status: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategySignalSet {
    pub symbol: String,
    pub market_type: String,
    pub strategy_signals: Vec<StrategySignal>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalScanOutcome {
    pub signals: Vec<UnifiedSignal>,
    pub strategy_signal_sets: Vec<StrategySignalSet>,
}

pub struct SignalRegistry {
    strategies: Vec<Box<dyn Strategy>>,
}

impl SignalRegistry {
    pub fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(MaCrossStrategy),
                Box::new(RsiExtremeStrategy),
                Box::new(MacdDivergenceStrategy),
                Box::new(BollingerBreakStrategy),
                Box::new(VolumeSurgeStrategy),
            ],
        }
    }

    pub fn run_all(&self, ctx: &SignalContext, configs: &[StrategyConfig]) -> Vec<StrategySignal> {
        let market_type = match ctx.market_type {
            "perpetual" => ApplicableMarket::Perpetual,
            _ => ApplicableMarket::Spot,
        };

        self.strategies
            .iter()
            .filter(|strategy| strategy.applicable_to().contains(&market_type))
            .filter(|strategy| {
                configs
                    .iter()
                    .find(|config| config.strategy_id == strategy.id())
                    .map(|config| config.enabled)
                    .unwrap_or(true)
            })
            .filter_map(|strategy| {
                let config = configs
                    .iter()
                    .find(|config| config.strategy_id == strategy.id());
                let user_params = config.map(|config| &config.params);
                let mp = merged_params(strategy.id(), user_params.unwrap_or(&HashMap::new()));
                Some(
                    strategy
                        .evaluate(ctx, &mp)
                        .unwrap_or_else(|| neutral_strategy_signal(strategy.as_ref())),
                )
            })
            .collect()
    }
}

fn neutral_strategy_signal(strategy: &dyn Strategy) -> StrategySignal {
    StrategySignal {
        strategy_id: strategy.id().to_string(),
        category: strategy.category(),
        direction: SignalDirection::Neutral,
        strength: 0.0,
        confidence: 0.0,
        summary: format!("No {} signal triggered.", strategy.name()),
        metrics: HashMap::new(),
    }
}

#[derive(Clone)]
pub struct SignalService {
    ledger: Arc<SignalLedger>,
    latest: Arc<RwLock<Vec<UnifiedSignal>>>,
}

impl SignalService {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let ledger = SignalLedger::new(path)?;
        Ok(Self {
            ledger: Arc::new(ledger),
            latest: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub async fn scan_all(
        &self,
        _enabled_exchanges: &[String],
        market_data: &MarketDataService,
        settings_service: &crate::settings::SettingsService,
        runtime: &RuntimeSettingsDto,
        account_equity_usdt: f64,
    ) -> anyhow::Result<Vec<UnifiedSignal>> {
        Ok(self
            .scan_all_with_strategy_signals(
                &[],
                market_data,
                settings_service,
                runtime,
                account_equity_usdt,
            )
            .await?
            .signals)
    }

    pub async fn scan_all_with_strategy_signals(
        &self,
        _enabled_exchanges: &[String],
        market_data: &MarketDataService,
        settings_service: &crate::settings::SettingsService,
        runtime: &RuntimeSettingsDto,
        account_equity_usdt: f64,
    ) -> anyhow::Result<SignalScanOutcome> {
        let scan_instant = std::time::Instant::now();
        let scan_start_iso = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let scan_id = self.ledger.insert_scan_run(&scan_start_iso).unwrap_or(0);

        if !runtime.signals_enabled || runtime.watchlist_symbols.is_empty() {
            let _ = self.ledger.complete_scan_run(
                scan_id,
                0,
                0,
                scan_instant.elapsed().as_millis() as u32,
                None,
            );
            return Ok(SignalScanOutcome {
                signals: Vec::new(),
                strategy_signal_sets: Vec::new(),
            });
        }

        let configs = self.ledger.get_strategy_configs().unwrap_or_else(|_| {
            ACTIVE_STRATEGY_IDS
                .iter()
                .map(|id| StrategyConfig::default_for(id))
                .collect()
        });

        let cached_rows = market_data.cached_market_rows_for_watchlist(&runtime.watchlist_symbols);
        let rows = cached_rows
            .iter()
            .filter(|row| row.market_type.eq_ignore_ascii_case("ashare"))
            .cloned()
            .collect::<Vec<_>>();
        let total_symbols = rows.len() as u32;

        let watchlist: HashSet<String> = runtime.watchlist_symbols.iter().cloned().collect();
        let candidates = pre_filter_rows(&rows, &watchlist, rows.len());

        let config = ScoringConfig::default();
        let risk_settings = risk_settings_from_runtime(runtime, account_equity_usdt);
        let signals_today = self.ledger.count_signals_today().unwrap_or(0);
        let registry = SignalRegistry::new();

        eprintln!(
            "Pre-filter: {} total rows -> {} candidates (watchlist: {:?})",
            rows.len(),
            candidates.len(),
            &runtime.watchlist_symbols,
        );

        let fetch_tasks: Vec<_> = candidates
            .iter()
            .map(|row| {
                let row = (*row).clone();
                let source_row = cached_rows
                    .iter()
                    .find(|candidate| {
                        candidate.symbol == row.symbol
                            && candidate.market_type.eq_ignore_ascii_case(&row.market_type)
                    })
                    .cloned()
                    .unwrap_or_else(|| row.clone());
                let symbol = row.symbol.clone();
                let settings_service = settings_service.clone();
                async move {
                    let candles =
                        tokio::time::timeout(std::time::Duration::from_secs(15), async move {
                            crate::market::akshare::fetch_history_bars_with_settings(
                                &settings_service,
                                &symbol,
                                "1d",
                                120,
                            )
                        })
                        .await;
                    (row, source_row, candles)
                }
            })
            .collect();

        let results: Vec<_> = stream::iter(fetch_tasks)
            .buffer_unordered(CANDLE_FETCH_CONCURRENCY)
            .collect()
            .await;

        let mut all_signals = Vec::new();
        let mut strategy_signal_sets = Vec::new();
        for (_row, source_row, candles_result) in results {
            let (row, candles) = match candles_result {
                Ok(Ok(bars)) => (source_row, bars),
                _ => continue,
            };
            if candles.len() < 30 {
                continue;
            }

            let ctx = SignalContext {
                symbol: &row.symbol,
                market_type: &row.market_type,
                row: &row,
                candles: &candles,
                venue_snapshots: &row.venue_snapshots,
            };

            let strategy_signals = registry.run_all(&ctx, &configs);
            strategy_signal_sets.push(StrategySignalSet {
                symbol: row.symbol.clone(),
                market_type: row.market_type.clone(),
                strategy_signals: strategy_signals.clone(),
            });
            if let Some(mut signal) = aggregate(
                strategy_signals,
                &row.symbol,
                &row.market_type,
                row.last_price,
                row.volume_24h,
                &config,
            ) {
                let direction_str = serde_json::to_string(&signal.direction).unwrap_or_default();
                let last_ts = self
                    .ledger
                    .last_signal_direction_timestamp(&signal.symbol, &direction_str)
                    .unwrap_or(None);
                let risk = evaluate_signal(
                    &signal,
                    &risk_settings,
                    runtime.signal_min_score,
                    runtime.signal_cooldown_minutes,
                    runtime.signal_daily_max,
                    signals_today + all_signals.len() as u32,
                    last_ts,
                );
                signal.risk_status = risk.status.clone();
                let risk_json = serde_json::to_string(&risk).unwrap_or_default();
                if let Err(e) = self.ledger.insert_signal(&signal, &risk_json) {
                    eprintln!("failed to persist signal {}: {e}", signal.signal_id);
                }
                all_signals.push(signal);
            }
        }

        all_signals.sort_by(|a, b| b.score.total_cmp(&a.score));
        if let Ok(mut latest) = self.latest.write() {
            *latest = all_signals.clone();
        }

        let duration_ms = scan_instant.elapsed().as_millis() as u32;
        let _ = self.ledger.complete_scan_run(
            scan_id,
            total_symbols,
            all_signals.len() as u32,
            duration_ms,
            None,
        );

        Ok(SignalScanOutcome {
            signals: all_signals,
            strategy_signal_sets,
        })
    }

    pub async fn scan_single(
        &self,
        symbol: &str,
        _market_type: &str,
        _enabled_exchanges: &[String],
        market_data: &MarketDataService,
        settings_service: &crate::settings::SettingsService,
        runtime: &RuntimeSettingsDto,
        account_equity_usdt: f64,
    ) -> anyhow::Result<(Vec<StrategySignal>, Option<UnifiedSignal>)> {
        let row = market_data
            .cached_market_rows_for_watchlist(&runtime.watchlist_symbols)
            .into_iter()
            .find(|row| row.symbol == symbol && row.market_type.eq_ignore_ascii_case("ashare"))
            .with_context(|| format!("No cached A-share market data for {symbol}"))?;
        let candles = crate::market::akshare::fetch_history_bars_with_settings(
            settings_service,
            &row.symbol,
            "1d",
            120,
        )
        .with_context(|| format!("Failed to fetch candles for {symbol}"))?;

        if candles.len() < 30 {
            return Ok((Vec::new(), None));
        }

        let configs = self.ledger.get_strategy_configs().unwrap_or_else(|_| {
            ACTIVE_STRATEGY_IDS
                .iter()
                .map(|id| StrategyConfig::default_for(id))
                .collect()
        });

        let ctx = SignalContext {
            symbol: &row.symbol,
            market_type: &row.market_type,
            row: &row,
            candles: &candles,
            venue_snapshots: &row.venue_snapshots,
        };

        let registry = SignalRegistry::new();
        let strategy_signals = registry.run_all(&ctx, &configs);

        let config = ScoringConfig::default();
        let unified = if let Some(mut signal) = aggregate(
            strategy_signals.clone(),
            &row.symbol,
            &row.market_type,
            row.last_price,
            row.volume_24h,
            &config,
        ) {
            let risk_settings = risk_settings_from_runtime(runtime, account_equity_usdt);
            let signals_today = self.ledger.count_signals_today().unwrap_or(0);
            let direction_str = serde_json::to_string(&signal.direction).unwrap_or_default();
            let last_ts = self
                .ledger
                .last_signal_direction_timestamp(&signal.symbol, &direction_str)
                .unwrap_or(None);
            let risk = evaluate_signal(
                &signal,
                &risk_settings,
                runtime.signal_min_score,
                runtime.signal_cooldown_minutes,
                runtime.signal_daily_max,
                signals_today,
                last_ts,
            );
            signal.risk_status = risk.status.clone();
            let risk_json = serde_json::to_string(&risk).unwrap_or_default();
            let _ = self.ledger.insert_signal(&signal, &risk_json);
            Some(signal)
        } else {
            None
        };

        Ok((strategy_signals, unified))
    }

    pub fn latest_signals(&self) -> Vec<UnifiedSignal> {
        self.latest.read().map(|r| r.clone()).unwrap_or_default()
    }

    pub async fn list_history(&self, limit: usize) -> anyhow::Result<Vec<ledger::SignalRecord>> {
        self.ledger.list_signals(limit)
    }

    pub async fn mark_executed(&self, signal_id: &str, payload: &str) -> anyhow::Result<()> {
        self.ledger.mark_executed(signal_id)?;
        self.ledger
            .append_user_action(signal_id, "executed", payload)
    }

    pub async fn delete_signal(&self, signal_id: &str) -> anyhow::Result<()> {
        self.ledger.delete_signal(signal_id)
    }

    pub async fn strategy_stats(&self) -> anyhow::Result<Vec<StrategyStats>> {
        self.ledger.strategy_stats()
    }

    pub async fn scan_run_history(
        &self,
        page: usize,
        page_size: usize,
    ) -> anyhow::Result<(Vec<ScanRunRecord>, usize)> {
        self.ledger.list_scan_runs(page, page_size)
    }

    pub fn get_strategy_configs(&self) -> anyhow::Result<Vec<StrategyConfig>> {
        self.ledger.get_strategy_configs()
    }

    pub fn update_strategy_config(
        &self,
        strategy_id: &str,
        enabled: Option<bool>,
        params_json: Option<&str>,
    ) -> anyhow::Result<()> {
        self.ledger
            .update_strategy_config(strategy_id, enabled, params_json)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::path::PathBuf;

    fn data_dir() -> PathBuf {
        dirs_next().unwrap_or_else(|| PathBuf::from("."))
    }

    fn dirs_next() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME").ok()?;
            Some(PathBuf::from(home).join("Library/Application Support/com.yejiming.kittyalpha"))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    #[tokio::test]
    #[ignore = "uses local app databases and live exchange requests"]
    async fn scan_all_completes_within_timeout() {
        let base = data_dir();
        let market_cache = base.join("kittyalpha.market.sqlite3");
        let signal_db = base.join("kittyalpha.signals.sqlite3");

        // Skip if files don't exist (CI / fresh env)
        if !market_cache.exists() || !signal_db.exists() {
            eprintln!("SKIP: market cache or signal DB not found at {:?}", base);
            return;
        }

        let signal_service = SignalService::new(signal_db).expect("signal service init");
        let market_data =
            crate::market::MarketDataService::new(market_cache).expect("market data init");

        let settings_path = base.join("kittyalpha.runtime.settings.json");
        let settings_json = std::fs::read_to_string(&settings_path).expect("read settings");
        let runtime: crate::models::RuntimeSettingsDto =
            serde_json::from_str(&settings_json).expect("deserialize runtime");

        eprintln!(
            "Exchanges enabled: {:?}",
            runtime
                .exchanges
                .iter()
                .filter(|e| e.enabled)
                .map(|e| &e.exchange)
                .collect::<Vec<_>>()
        );
        eprintln!("signals_enabled: {}", runtime.signals_enabled);
        eprintln!(
            "signal_min_score: {}, signal_cooldown_minutes: {}, signal_daily_max: {}",
            runtime.signal_min_score, runtime.signal_cooldown_minutes, runtime.signal_daily_max
        );

        let enabled: Vec<String> = runtime
            .exchanges
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.exchange.clone())
            .collect();
        let settings_service = crate::settings::SettingsService::default();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            signal_service.scan_all(
                &enabled,
                &market_data,
                &settings_service,
                &runtime,
                10_000.0,
            ),
        )
        .await;

        match result {
            Ok(Ok(signals)) => {
                eprintln!(
                    "✅ scan_all completed: {} signals in {:?}",
                    signals.len(),
                    std::time::Instant::now()
                );
                for s in &signals {
                    eprintln!(
                        "  {} {} {} score={:.1} risk={} strategies={:?}",
                        s.symbol,
                        s.market_type,
                        serde_json::to_string(&s.direction).unwrap_or_default(),
                        s.score,
                        s.risk_status,
                        &s.contributors[..s.contributors.len().min(3)]
                    );
                }
            }
            Ok(Err(e)) => {
                eprintln!("❌ scan_all returned error: {e}");
                panic!("scan_all failed: {e}");
            }
            Err(_elapsed) => {
                eprintln!("❌❌❌ scan_all TIMED OUT after 120 seconds — STILL HANGING");
                panic!("scan_all timed out — the hang is NOT fixed");
            }
        }
    }
}
