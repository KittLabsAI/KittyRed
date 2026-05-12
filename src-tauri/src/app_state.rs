use crate::assistant::AssistantService;
use crate::backtest::BacktestService;
use crate::financial_reports::FinancialReportService;
use crate::jobs::JobService;
use crate::market::MarketDataService;
use crate::notifications::NotificationService;
use crate::paper::PaperService;
use crate::portfolio::PortfolioService;
use crate::recommendations::automation::spawn_auto_analyze_worker;
use crate::recommendations::RecommendationService;
use crate::settings::SettingsService;
use crate::signals::SignalService;
use std::path::PathBuf;
use std::time::Duration;

const MARKET_TICKER_REFRESH_INTERVAL_SECS: u64 = 60;

#[cfg(test)]
mod tests {
    use super::{
        run_market_ticker_refresh_cycle, spawn_market_ticker_refresh_worker,
        MARKET_TICKER_REFRESH_INTERVAL_SECS,
    };
    use crate::jobs::JobService;
    use crate::market::MarketDataService;
    use crate::models::MarketListRow;
    use crate::settings::SettingsService;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn market_ticker_refresh_worker_stays_idle_without_watchlist() {
        let settings_path = unique_temp_settings_path("ticker-refresh-worker");
        let settings = SettingsService::new(settings_path.clone());
        let jobs = JobService::empty();
        let market_data = MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "BTC/USDT".into(),
            base_asset: "BTC".into(),
            market_type: "perpetual".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 68_420.0,
            change_24h: 2.8,
            volume_24h: 180_000_000.0,
            funding_rate: Some(0.011),
            spread_bps: 3.4,
            exchanges: vec!["akshare".into(), "人民币现金".into()],
            updated_at: "2026-05-04T10:00:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]);

        spawn_market_ticker_refresh_worker(settings, jobs.clone(), market_data);

        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(jobs.list_jobs().len(), 0);

        tokio::time::sleep(Duration::from_secs(18)).await;
        assert_eq!(jobs.list_jobs().len(), 0);

        let _ = std::fs::remove_file(settings_path);
    }

    #[tokio::test]
    async fn market_ticker_refresh_skips_when_watchlist_is_missing() {
        let settings_path = unique_temp_settings_path("ticker-refresh-no-token");
        let settings = SettingsService::new(settings_path.clone());
        let jobs = JobService::empty();
        let market_data = MarketDataService::default();

        run_market_ticker_refresh_cycle(&settings, &jobs, &market_data).await;

        assert!(jobs.list_jobs().is_empty());
        let _ = std::fs::remove_file(settings_path);
    }

    #[test]
    fn market_ticker_refresh_interval_is_sixty_seconds() {
        assert_eq!(MARKET_TICKER_REFRESH_INTERVAL_SECS, 60);
    }

    fn unique_temp_settings_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.json"))
    }
}

pub struct AppState {
    pub app_handle: tauri::AppHandle,
    pub settings_service: SettingsService,
    pub job_service: JobService,
    pub market_data_service: MarketDataService,
    pub notification_service: NotificationService,
    pub portfolio_service: PortfolioService,
    pub paper_service: PaperService,
    pub recommendation_service: RecommendationService,
    pub assistant_service: AssistantService,
    pub signal_service: SignalService,
    pub backtest_service: BacktestService,
    pub financial_report_service: FinancialReportService,
}

impl AppState {
    pub fn new(settings_path: PathBuf, app_handle: tauri::AppHandle) -> Self {
        let recommendation_path = recommendation_path_for(&settings_path);
        let backtest_path = backtest_path_for(&settings_path);
        let financial_report_path = financial_report_path_for(&settings_path);
        let market_cache_path = market_cache_path_for(&settings_path);
        let paper_path = paper_path_for(&settings_path);
        let job_history_path = recommendation_path.clone();
        let notification_service = NotificationService::new(recommendation_path.clone());
        let signal_path = settings_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("kittyalpha.signals.sqlite3");
        let signal_service =
            SignalService::new(signal_path).expect("signal service should initialize");
        let settings_service = SettingsService::new(settings_path);
        let recommendation_service = RecommendationService::new(recommendation_path);
        let paper_service =
            PaperService::new(paper_path).unwrap_or_else(|_| PaperService::default());
        let portfolio_service = PortfolioService::new(paper_service.clone());
        let job_service =
            JobService::new(job_history_path).unwrap_or_else(|_| JobService::default());

        let state = Self {
            app_handle: app_handle.clone(),
            settings_service,
            job_service,
            market_data_service: MarketDataService::new(market_cache_path)
                .unwrap_or_else(|_| MarketDataService::default()),
            portfolio_service,
            paper_service,
            assistant_service: AssistantService::new(app_handle.clone()),
            notification_service,
            recommendation_service,
            signal_service,
            backtest_service: BacktestService::new(backtest_path)
                .expect("backtest service should initialize"),
            financial_report_service: FinancialReportService::new(financial_report_path)
                .expect("financial report service should initialize"),
        };

        spawn_auto_analyze_worker(
            state.settings_service.clone(),
            state.job_service.clone(),
            state.market_data_service.clone(),
            state.recommendation_service.clone(),
            state.notification_service.clone(),
            state.paper_service.clone(),
            state.financial_report_service.clone(),
            app_handle,
        );
        spawn_market_ticker_refresh_worker(
            state.settings_service.clone(),
            state.job_service.clone(),
            state.market_data_service.clone(),
        );
        crate::commands::paper::resume_pending_paper_order_jobs(
            state.job_service.clone(),
            state.paper_service.clone(),
            state.settings_service.clone(),
        );
        crate::commands::backtest::resume_pending_backtest_jobs(
            state.backtest_service.clone(),
            state.market_data_service.clone(),
            state.settings_service.clone(),
            state.financial_report_service.clone(),
        );
        spawn_stock_universe_cache_worker(
            state.settings_service.clone(),
            state.market_data_service.clone(),
        );
        state
    }
}

fn recommendation_path_for(settings_path: &PathBuf) -> PathBuf {
    let parent = settings_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("kittyalpha.recommendations.sqlite3")
}

fn paper_path_for(settings_path: &PathBuf) -> PathBuf {
    settings_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("kittyalpha.paper.sqlite3")
}

fn market_cache_path_for(settings_path: &PathBuf) -> PathBuf {
    let parent = settings_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("kittyalpha.market.sqlite3")
}

fn backtest_path_for(settings_path: &PathBuf) -> PathBuf {
    settings_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("kittyred.backtest.sqlite3")
}

fn financial_report_path_for(settings_path: &PathBuf) -> PathBuf {
    settings_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("kittyred.financial_reports.sqlite3")
}

fn spawn_market_ticker_refresh_worker(
    settings_service: SettingsService,
    job_service: JobService,
    market_data_service: MarketDataService,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_market_ticker_refresh_cycle(&settings_service, &job_service, &market_data_service)
                .await;
            tokio::time::sleep(Duration::from_secs(MARKET_TICKER_REFRESH_INTERVAL_SECS)).await;
        }
    });
}

fn spawn_stock_universe_cache_worker(
    settings_service: SettingsService,
    market_data_service: MarketDataService,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_stock_universe_cache_cycle(&settings_service, &market_data_service).await;
            tokio::time::sleep(Duration::from_secs(60 * 60)).await;
        }
    });
}

async fn run_market_ticker_refresh_cycle(
    settings_service: &SettingsService,
    job_service: &JobService,
    market_data_service: &MarketDataService,
) {
    let runtime = settings_service.get_runtime_settings();
    let watchlist = runtime.watchlist_symbols.clone();
    if watchlist.is_empty() {
        return;
    }

    let Some(job_id) = job_service.start_non_reentrant_job(
        crate::jobs::kinds::MARKET_REFRESH_TICKERS,
        "刷新自选股行情",
        Some(serde_json::json!({ "source": "akshare", "symbols": watchlist }).to_string()),
    ) else {
        return;
    };

    match market_data_service
        .refresh_ticker_cache_from_akshare(settings_service, &runtime.watchlist_symbols)
        .await
    {
        Ok(rows) => job_service.finish_job(
            job_id,
            "done",
            &format!("已刷新 {} 条自选股行情", rows.len()),
            Some(format!("已刷新 {} 条自选股行情", rows.len())),
            None,
        ),
        Err(error) => job_service.finish_job(
            job_id,
            "failed",
            &format!("Ticker cache refresh failed: {error}"),
            None,
            Some(error.to_string()),
        ),
    }
}

async fn run_stock_universe_cache_cycle(
    settings_service: &SettingsService,
    market_data_service: &MarketDataService,
) {
    if let Err(error) =
        market_data_service.refresh_a_share_symbol_cache_from_akshare(settings_service)
    {
        eprintln!("Failed to warm A-share stock universe cache: {error}");
    }
}
