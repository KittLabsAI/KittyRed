#[cfg(test)]
mod tests {
    use super::{
        execute_recommendation_job, run_auto_analyze_cycle, run_paper_order_monitor_cycle,
        should_start_auto_analyze,
    };
    use crate::db::Database;
    use crate::jobs::JobService;
    use crate::market::MarketDataService;
    use crate::models::{MarketListRow, RecommendationRunDto, RuntimeSecretsSyncDto};
    use crate::notifications::NotificationService;
    use crate::paper::PaperService;
    use crate::recommendations::{RecommendationRecord, RecommendationService};
    use crate::settings::SettingsService;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use time::format_description::well_known::Rfc3339;
    use time::{Duration as TimeDuration, OffsetDateTime};

    #[test]
    fn waits_for_the_configured_auto_analyze_interval() {
        let settings = SettingsService::new(unique_temp_settings_path("auto-interval"));
        let runtime = settings.get_runtime_settings();

        assert!(should_start_auto_analyze(&runtime, 600_000, None));
        assert!(!should_start_auto_analyze(&runtime, 840_000, Some(600_000)));
        assert!(should_start_auto_analyze(
            &runtime,
            1_200_000,
            Some(600_000)
        ));
    }

    #[tokio::test]
    async fn startup_auto_cycle_primes_the_interval_without_creating_a_job() {
        let path = unique_temp_settings_path("startup-auto-prime");
        let settings = SettingsService::new(path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.auto_analyze_enabled = true;
        runtime.auto_analyze_frequency = "5m".into();
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let jobs = JobService::empty();
        let market_data = sample_market_data();
        let recommendation_path = unique_temp_recommendation_db_path("startup-auto-prime");
        let recommendations = RecommendationService::new(recommendation_path.clone());
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "startup-auto-prime-notifications",
        ));
        let paper = PaperService::default();
        let mut last_run_ms = None;

        let ran = run_auto_analyze_cycle(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            &mut last_run_ms,
            600_000,
        )
        .await
        .expect("startup cycle should only prime the timer");

        assert!(!ran);
        assert_eq!(last_run_ms, Some(600_000));
        assert!(jobs.list_jobs().is_empty());
        assert!(recommendations
            .list_history_snapshot(10)
            .await
            .expect("history should load")
            .is_empty());

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn auto_cycle_does_not_fallback_when_ai_is_unavailable() {
        let path = unique_temp_settings_path("auto-cycle");
        let settings = SettingsService::new(path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.auto_analyze_enabled = true;
        runtime.auto_analyze_frequency = "5m".into();
        runtime.account_mode = "paper".into();
        runtime.auto_paper_execution = true;
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let jobs = JobService::default();
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
            exchanges: vec!["人民币现金".into(), "akshare".into(), "A股缓存".into()],
            updated_at: "2026-05-03T18:20:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]);
        let recommendation_path = unique_temp_recommendation_db_path("auto-no-ai-fallback");
        let recommendations = RecommendationService::new(recommendation_path.clone());
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "auto-cycle-notifications",
        ));
        let paper = PaperService::default();
        let initial_jobs = jobs.list_jobs().len();

        let run = execute_recommendation_job(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            None,
            "auto",
        )
        .await
        .expect(
            "auto recommendation should finish without pulling data when AKShare is not configured",
        );

        assert_eq!(run.len(), 1);
        assert!(!run[0].has_trade);
        assert!(paper.list_positions_snapshot().is_empty());
        let updated_jobs = jobs.list_jobs();
        assert_eq!(updated_jobs.len(), initial_jobs + 1);
        assert_eq!(updated_jobs[0].status, "done");
        assert!(updated_jobs[0]
            .message
            .contains("Recommendation run completed"));
        let notification_events = notifications
            .list_events(10)
            .expect("notification events should load");
        assert!(notification_events.is_empty());
        assert_eq!(
            recommendations
                .list_history_snapshot(10)
                .await
                .expect("history should load")
                .len(),
            1
        );

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn manual_cycle_with_no_live_market_rows_finishes_as_done_no_trade() {
        let path = unique_temp_settings_path("manual-empty-market");
        let settings = SettingsService::new(path.clone());
        settings
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: false,
                model_api_key: Some("sk-test".into()),
                exchanges: Vec::new(),
            })
            .expect("model api key should be available for the scan");

        let jobs = JobService::default();
        let market_data = MarketDataService::with_static_rows(Vec::new());
        let recommendation_path = unique_temp_recommendation_db_path("manual-empty-market");
        let recommendations = RecommendationService::new(recommendation_path.clone());
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "manual-empty-market-notifications",
        ));
        let paper = PaperService::default();

        let run = execute_recommendation_job(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            None,
            "manual",
        )
        .await
        .expect("manual recommendation should finish with a no-trade result");

        assert_eq!(run.len(), 1);
        assert!(!run[0].has_trade);
        assert_eq!(run[0].risk_status, "watch");
        assert!(run[0].rationale.contains("No live market data"));

        let recorded_jobs = jobs.list_jobs();
        assert_eq!(recorded_jobs[0].status, "done");
        assert!(recorded_jobs[0]
            .message
            .contains("Recommendation run completed"));
        assert!(recorded_jobs[0]
            .result_summary
            .as_deref()
            .unwrap_or_default()
            .contains("No live market data"));

        let history = recommendations
            .list_history_snapshot(10)
            .await
            .expect("history should load");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].result, "No Trade");

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn blocks_auto_cycle_after_reaching_the_daily_ai_call_limit() {
        let settings_path = unique_temp_settings_path("auto-daily-limit");
        let recommendation_path = unique_temp_recommendation_db_path("auto-daily-limit");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.auto_analyze_enabled = true;
        runtime.auto_analyze_frequency = "5m".into();
        runtime.daily_max_ai_calls = 1;
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let recommendations = RecommendationService::new(recommendation_path.clone());
        store_auto_recommendation(
            &recommendations,
            sample_auto_run("rec-auto-daily-limit", now_rfc3339()),
        )
        .await;

        let jobs = JobService::default();
        let initial_jobs = jobs.list_jobs().len();
        let market_data = sample_market_data();
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "auto-daily-limit-notifications",
        ));
        let paper = PaperService::default();

        let error = execute_recommendation_job(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            None,
            "auto",
        )
        .await
        .expect_err("auto recommendation should pause after hitting the daily call limit");

        assert!(error.to_string().contains("daily AI call limit"));
        let recorded_jobs = jobs.list_jobs();
        assert_eq!(recorded_jobs.len(), initial_jobs + 1);
        assert_eq!(recorded_jobs[0].status, "blocked");
        assert!(recorded_jobs[0].message.contains("daily AI call limit"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn blocks_auto_cycle_after_the_configured_consecutive_loss_streak() {
        let settings_path = unique_temp_settings_path("auto-loss-pause");
        let recommendation_path = unique_temp_recommendation_db_path("auto-loss-pause");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.auto_analyze_enabled = true;
        runtime.auto_analyze_frequency = "5m".into();
        runtime.daily_max_ai_calls = 8;
        runtime.pause_after_consecutive_losses = 2;
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let recommendations = RecommendationService::new(recommendation_path.clone());
        let latest = sample_auto_run(
            "rec-auto-loss-2",
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(5)),
        );
        let previous = sample_auto_run(
            "rec-auto-loss-1",
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(10)),
        );
        store_auto_recommendation(&recommendations, previous.clone()).await;
        store_auto_recommendation(&recommendations, latest.clone()).await;
        insert_evaluation(
            &recommendation_path,
            &previous.recommendation_id,
            -1.4,
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(4)),
        );
        insert_evaluation(
            &recommendation_path,
            &latest.recommendation_id,
            -0.9,
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(2)),
        );

        let jobs = JobService::default();
        let initial_jobs = jobs.list_jobs().len();
        let market_data = sample_market_data();
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "auto-loss-pause-notifications",
        ));
        let paper = PaperService::default();

        let error = execute_recommendation_job(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            None,
            "auto",
        )
        .await
        .expect_err("auto recommendation should pause after consecutive losing evaluations");

        assert!(error
            .to_string()
            .contains("consecutive losing recommendations"));
        let recorded_jobs = jobs.list_jobs();
        assert_eq!(recorded_jobs.len(), initial_jobs + 1);
        assert_eq!(recorded_jobs[0].status, "blocked");
        assert!(recorded_jobs[0]
            .message
            .contains("consecutive losing recommendations"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn blocks_auto_cycle_after_exceeding_the_daily_loss_budget() {
        let settings_path = unique_temp_settings_path("auto-daily-loss");
        let recommendation_path = unique_temp_recommendation_db_path("auto-daily-loss");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.auto_analyze_enabled = true;
        runtime.auto_analyze_frequency = "5m".into();
        runtime.daily_max_ai_calls = 8;
        runtime.pause_after_consecutive_losses = 5;
        runtime.max_daily_loss_percent = 3.0;
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let recommendations = RecommendationService::new(recommendation_path.clone());
        let first = sample_auto_run(
            "rec-auto-loss-budget-1",
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(20)),
        );
        let second = sample_auto_run(
            "rec-auto-loss-budget-2",
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(10)),
        );
        store_auto_recommendation(&recommendations, first.clone()).await;
        store_auto_recommendation(&recommendations, second.clone()).await;
        insert_evaluation(
            &recommendation_path,
            &first.recommendation_id,
            -1.8,
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(8)),
        );
        insert_evaluation(
            &recommendation_path,
            &second.recommendation_id,
            -1.5,
            format_rfc3339(OffsetDateTime::now_utc() - TimeDuration::minutes(4)),
        );

        let jobs = JobService::default();
        let initial_jobs = jobs.list_jobs().len();
        let market_data = sample_market_data();
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "auto-daily-loss-notifications",
        ));
        let paper = PaperService::default();

        let error = execute_recommendation_job(
            &jobs,
            &market_data,
            &recommendations,
            &notifications,
            &paper,
            &settings,
            None,
            None,
            None,
            "auto",
        )
        .await
        .expect_err("auto recommendation should pause after exceeding the daily loss budget");

        assert!(error.to_string().contains("daily loss limit"));
        let recorded_jobs = jobs.list_jobs();
        assert_eq!(recorded_jobs.len(), initial_jobs + 1);
        assert_eq!(recorded_jobs[0].status, "blocked");
        assert!(recorded_jobs[0].message.contains("daily loss limit"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn paper_monitor_closes_positions_after_take_profit_is_hit() {
        let path = unique_temp_settings_path("paper-monitor");
        let settings = SettingsService::new(path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.account_mode = "paper".into();
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");

        let jobs = JobService::default();
        let market_data = MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "DOGE/USDT".into(),
            base_asset: "DOGE".into(),
            market_type: "perpetual".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 0.191,
            change_24h: 7.6,
            volume_24h: 260_000_000.0,
            funding_rate: Some(0.019),
            spread_bps: 6.2,
            exchanges: vec!["akshare".into(), "深圳证券交易所".into()],
            updated_at: "2026-05-03T21:00:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]);
        let notifications = NotificationService::new(unique_temp_recommendation_db_path(
            "paper-monitor-notifications",
        ));
        let paper = PaperService::default();
        let recommendation = RecommendationRunDto {
            recommendation_id: "rec-paper-monitor".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("DOGE/USDT".into()),
            stock_name: Some("DOGE".into()),
            direction: Some("Long".into()),
            market_type: "perpetual".into(),
            exchanges: vec!["akshare".into(), "深圳证券交易所".into()],
            confidence_score: 74.0,
            rationale: "Paper monitor regression".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(0.181),
            entry_high: Some(0.182),
            stop_loss: Some(0.176),
            take_profit: Some("0.189 / 0.193".into()),
            leverage: Some(2.0),
            amount_cny: Some(1_200.0),
            invalidation: Some("Lose 0.176".into()),
            max_loss_cny: Some(40.0),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-03T20:40:00+08:00".into(),
            model_provider: "System".into(),
            model_name: "heuristic-fallback".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-paper-monitor".into(),
            generated_at: "2026-05-03T20:40:00+08:00".into(),
        };

        paper
            .create_draft_from_recommendation(&recommendation, "paper-cash")
            .await
            .expect("paper draft should open a position");

        let closed = run_paper_order_monitor_cycle(
            &jobs,
            &market_data,
            &paper,
            &settings,
            &notifications,
            None,
        )
        .await
        .expect("paper monitor should succeed");

        assert_eq!(closed, 1);
        assert!(paper.list_positions_snapshot().is_empty());
        let paper_jobs = jobs
            .list_jobs()
            .into_iter()
            .filter(|job| job.kind == crate::jobs::kinds::PAPER_ORDER_MONITOR)
            .collect::<Vec<_>>();
        assert_eq!(paper_jobs.len(), 1);
        assert!(paper_jobs[0].message.contains("DOGE/USDT"));
        let notification_events = notifications
            .list_events(10)
            .expect("paper exit notification should be recorded");
        assert_eq!(notification_events.len(), 1);
        assert_eq!(notification_events[0].channel, "desktop.paper_orders");
        assert!(notification_events[0].body.contains("DOGE/USDT"));

        let _ = std::fs::remove_file(path);
    }

    fn unique_temp_settings_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.json"))
    }

    fn unique_temp_recommendation_db_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.sqlite3"))
    }

    fn sample_market_data() -> MarketDataService {
        MarketDataService::with_static_rows(vec![MarketListRow {
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
            exchanges: vec!["人民币现金".into(), "akshare".into(), "A股缓存".into()],
            updated_at: "2026-05-03T18:20:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }])
    }

    fn sample_auto_run(recommendation_id: &str, generated_at: String) -> RecommendationRunDto {
        RecommendationRunDto {
            recommendation_id: recommendation_id.into(),
            status: "completed".into(),
            trigger_type: "auto".into(),
            has_trade: true,
            symbol: Some("BTC/USDT".into()),
            stock_name: Some("BTC".into()),
            direction: Some("Long".into()),
            market_type: "perpetual".into(),
            exchanges: vec!["人民币现金".into(), "akshare".into()],
            confidence_score: 72.0,
            rationale: "Auto recommendation regression coverage".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(68_000.0),
            entry_high: Some(68_200.0),
            stop_loss: Some(67_500.0),
            take_profit: Some("69_000 / 69_400".into()),
            leverage: Some(2.0),
            amount_cny: Some(1_200.0),
            invalidation: Some("Lose 67_500".into()),
            max_loss_cny: Some(35.0),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: generated_at.clone(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-5.5".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-auto-regression".into(),
            generated_at,
        }
    }

    async fn store_auto_recommendation(
        recommendations: &RecommendationService,
        run: RecommendationRunDto,
    ) {
        recommendations
            .store_record(RecommendationRecord {
                trigger_type: run.trigger_type.clone(),
                ai_raw_output: "{\"has_trade\":true}".into(),
                ai_structured_output: "{\"source\":\"test\"}".into(),
                risk_result: "{\"status\":\"approved\"}".into(),
                market_snapshot: "{\"rows\":[]}".into(),
                account_snapshot: "{\"account_mode\":\"paper\"}".into(),
                run,
            })
            .await
            .expect("auto recommendation seed should persist");
    }

    fn insert_evaluation(
        path: &PathBuf,
        recommendation_id: &str,
        estimated_pnl_percent: f64,
        evaluated_at: String,
    ) {
        let db = Database::open(path).expect("recommendation db should open");
        db.connection()
            .execute(
                "INSERT INTO recommendation_evaluations (
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
                ) VALUES (?1, ?2, '24h', 68000, 69000, 67000, 0, 1, 0.1, 0.1, 0.0, ?3, ?4, 'loss', ?5)",
                rusqlite::params![
                    format!("eval-{recommendation_id}"),
                    recommendation_id,
                    estimated_pnl_percent * 10.0,
                    estimated_pnl_percent,
                    evaluated_at,
                ],
            )
            .expect("evaluation seed should persist");
    }

    fn now_rfc3339() -> String {
        format_rfc3339(OffsetDateTime::now_utc())
    }

    fn format_rfc3339(timestamp: OffsetDateTime) -> String {
        timestamp
            .format(&Rfc3339)
            .expect("timestamp should format as rfc3339")
    }
}

use std::time::Duration;

use crate::jobs::{kinds, JobService};
use crate::financial_reports::FinancialReportService;
use crate::market::MarketDataService;
use crate::models::{RecommendationRunDto, RuntimeSettingsDto};
use crate::notifications::NotificationService;
use crate::paper::{paper_account_id, PaperService};
use crate::recommendations::llm;
use crate::recommendations::{
    default_risk_result_json, RecommendationRecord, RecommendationService,
};
use crate::settings::SettingsService;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, Time};

const AUTO_ANALYZE_POLL_SECS: u64 = 15;

pub fn spawn_auto_analyze_worker(
    settings_service: SettingsService,
    job_service: JobService,
    market_data_service: MarketDataService,
    recommendation_service: RecommendationService,
    notification_service: NotificationService,
    paper_service: PaperService,
    financial_report_service: FinancialReportService,
    app_handle: tauri::AppHandle,
) {
    tauri::async_runtime::spawn(async move {
        let mut last_run_ms = None;

        loop {
            let now_ms = current_utc_millis();
            let _ = run_auto_analyze_cycle(
                &job_service,
                &market_data_service,
                &recommendation_service,
                &notification_service,
                &paper_service,
                &settings_service,
                Some(&financial_report_service),
                Some(&app_handle),
                &mut last_run_ms,
                now_ms,
            )
            .await;
            let _ = run_paper_order_monitor_cycle(
                &job_service,
                &market_data_service,
                &paper_service,
                &settings_service,
                &notification_service,
                Some(&app_handle),
            )
            .await;
            tokio::time::sleep(Duration::from_secs(AUTO_ANALYZE_POLL_SECS)).await;
        }
    });
}

pub async fn run_auto_analyze_cycle(
    job_service: &JobService,
    market_data_service: &MarketDataService,
    recommendation_service: &RecommendationService,
    notification_service: &NotificationService,
    paper_service: &PaperService,
    settings_service: &SettingsService,
    financial_report_service: Option<&FinancialReportService>,
    app_handle: Option<&tauri::AppHandle>,
    last_run_ms: &mut Option<i64>,
    now_ms: i64,
) -> anyhow::Result<bool> {
    let runtime = settings_service.get_runtime_settings();
    if !runtime.auto_analyze_enabled {
        return Ok(false);
    }

    if last_run_ms.is_none() {
        *last_run_ms = Some(now_ms);
        return Ok(false);
    }

    if !should_start_auto_analyze(&runtime, now_ms, *last_run_ms) {
        return Ok(false);
    }

    *last_run_ms = Some(now_ms);
    execute_recommendation_job(
        job_service,
        market_data_service,
        recommendation_service,
        notification_service,
        paper_service,
        settings_service,
        financial_report_service,
        None,
        app_handle,
        "auto",
    )
    .await?;
    Ok(true)
}

pub async fn execute_recommendation_job(
    job_service: &JobService,
    market_data_service: &MarketDataService,
    recommendation_service: &RecommendationService,
    notification_service: &NotificationService,
    paper_service: &PaperService,
    settings_service: &SettingsService,
    financial_report_service: Option<&FinancialReportService>,
    symbol: Option<String>,
    app_handle: Option<&tauri::AppHandle>,
    trigger_source: &str,
) -> anyhow::Result<Vec<RecommendationRunDto>> {
    let runtime = settings_service.get_runtime_settings();
    let enabled_exchanges: Vec<String> = Vec::new();
    let risk_equity_usdt = recommendation_risk_equity_usdt(&runtime, paper_service);
    let focus = symbol.clone().unwrap_or_else(|| "自选股".into());
    let source_label = if trigger_source.eq_ignore_ascii_case("auto") {
        "Auto scan"
    } else {
        "Manual scan"
    };
    let job_id = job_service.start_job(
        kinds::RECOMMENDATION_GENERATE,
        &format!("{source_label}: 正在扫描 {focus} 生成 AI 推荐"),
        Some(format!(
            r#"{{"triggerSource":"{trigger_source}","symbol":{},"scope":"watchlist"}}"#,
            symbol
                .as_ref()
                .map(|value| format!(r#""{value}""#))
                .unwrap_or_else(|| "null".into())
        )),
    );

    if trigger_source.eq_ignore_ascii_case("auto") {
        if let Some(reason) =
            auto_recommendation_block_reason(recommendation_service, &runtime).await?
        {
            job_service.finish_job(job_id, "blocked", &reason, None, Some(reason.clone()));
            return Err(anyhow::anyhow!(reason));
        }
    }

    if runtime.watchlist_symbols.is_empty() {
        let rows = Vec::new();
        let run = build_no_live_market_run(
            &runtime,
            &enabled_exchanges,
            symbol.as_deref(),
            trigger_source,
        );
        let account_snapshot =
            build_account_snapshot_json(&runtime, paper_service, risk_equity_usdt);
        let stored = recommendation_service
            .store_record(RecommendationRecord {
                trigger_type: trigger_source.to_string(),
                ai_raw_output: r#"{"source":"akshare_unavailable"}"#.into(),
                ai_structured_output: serde_json::json!({
                    "has_trade": false,
                    "rationale": run.rationale,
                })
                .to_string(),
                risk_result: default_risk_result_json(&run),
                market_snapshot: build_market_snapshot_json(&run, &rows, &enabled_exchanges, None),
                account_snapshot,
                run,
            })
            .await
            .inspect(|run| {
                job_service.finish_job(
                    job_id,
                    "done",
                    &format!("Recommendation run completed: {}", run.rationale),
                    recommendation_job_reason(run),
                    None,
                );
            })?;
        return Ok(vec![stored]);
    }

    let rows = market_data_service.cached_market_rows_for_watchlist(&runtime.watchlist_symbols);
    let decision_rows = rows
        .iter()
        .filter(|row| row.market_type.eq_ignore_ascii_case("ashare"))
        .cloned()
        .collect::<Vec<_>>();
    let account_snapshot = build_account_snapshot_json(&runtime, paper_service, risk_equity_usdt);
    let result = if decision_rows.is_empty() {
        let run = build_no_live_market_run(
            &runtime,
            &enabled_exchanges,
            symbol.as_deref(),
            trigger_source,
        );
        let stored = recommendation_service
            .store_record(RecommendationRecord {
                trigger_type: trigger_source.to_string(),
                ai_raw_output: r#"{"source":"market_data_unavailable"}"#.into(),
                ai_structured_output: serde_json::json!({
                    "has_trade": false,
                    "rationale": run.rationale,
                })
                .to_string(),
                risk_result: default_risk_result_json(&run),
                market_snapshot: build_market_snapshot_json(
                    &run,
                    &decision_rows,
                    &enabled_exchanges,
                    None,
                ),
                account_snapshot: account_snapshot.clone(),
                run,
            })
            .await?;
        Ok(vec![stored])
    } else {
        let position_contexts = paper_service
            .list_positions_snapshot()
            .into_iter()
            .map(|position| llm::PositionContext {
                symbol: position.symbol,
                side: position.side,
                size: position.size,
                entry_price: position.entry_price,
                mark_price: position.mark_price,
                pnl_percent: position.pnl_percent,
            })
            .collect::<Vec<_>>();
        let financial_analyses =
            financial_analysis_contexts(financial_report_service, &runtime, &rows);
        match llm::generate_trade_plan(
            settings_service,
            market_data_service,
            &rows,
            risk_equity_usdt,
            symbol.as_deref(),
            &enabled_exchanges,
            &position_contexts,
            &financial_analyses,
        )
        .await
        {
            Ok(plans) => {
                let records = plans
                    .into_iter()
                    .map(|mut plan| {
                        plan.run.trigger_type = trigger_source.to_string();
                        RecommendationRecord {
                            trigger_type: trigger_source.to_string(),
                            ai_raw_output: plan.ai_raw_output,
                            ai_structured_output: plan.ai_structured_output,
                            risk_result: default_risk_result_json(&plan.run),
                            market_snapshot: build_market_snapshot_json(
                                &plan.run,
                                &decision_rows,
                                &enabled_exchanges,
                                Some((&plan.system_prompt, &plan.user_prompt)),
                            ),
                            account_snapshot: account_snapshot.clone(),
                            run: plan.run,
                        }
                    })
                    .collect::<Vec<_>>();
                recommendation_service.store_records(records).await
            }
            Err(error) => Err(error),
        }
    };

    match result {
        Ok(runs) => {
            let primary_run = runs
                .first()
                .ok_or_else(|| anyhow::anyhow!("AI 建议没有生成任何结果"))?;
            let trade_count = runs.iter().filter(|run| run.has_trade).count();
            let mut message = if trade_count > 0 {
                format!(
                    "已完成 {} 条 AI 个股建议，其中 {} 条为交易建议。",
                    runs.len(),
                    trade_count
                )
            } else {
                format!("已完成 {} 条 AI 个股建议，当前均为观望或拦截。", runs.len())
            };

            if runtime.auto_paper_execution && runtime.account_mode == "paper" {
                let account_id = preferred_paper_account_id(&runtime);
                for run in runs
                    .iter()
                    .filter(|run| run.has_trade && run.direction.as_deref() == Some("买入"))
                {
                    match paper_service
                        .create_draft_from_recommendation(run, &account_id)
                        .await
                    {
                        Ok(draft) => {
                            message.push_str(&format!(" 已生成 {} 的模拟委托。", draft.symbol));
                        }
                        Err(error) => {
                            message.push_str(&format!(" 模拟委托生成失败：{error}。"));
                        }
                    }
                }
            }

            if runtime.notifications.recommendations {
                for run in &runs {
                    if let Err(error) =
                        notification_service.dispatch_recommendation(app_handle, run)
                    {
                        message.push_str(&format!(" 通知发送失败：{error}。"));
                    }
                }
            }

            job_service.finish_job(
                job_id,
                "done",
                &message,
                recommendation_job_reason(primary_run),
                None,
            );
            Ok(runs)
        }
        Err(error) => {
            job_service.finish_job(
                job_id,
                "blocked",
                &format!("Recommendation run failed: {error}"),
                None,
                Some(error.to_string()),
            );
            Err(error)
        }
    }
}

fn build_no_live_market_run(
    runtime: &RuntimeSettingsDto,
    enabled_exchanges: &[String],
    symbol: Option<&str>,
    trigger_source: &str,
) -> RecommendationRunDto {
    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| OffsetDateTime::now_utc().unix_timestamp().to_string());
    let no_trade_reason = no_live_market_reason(enabled_exchanges, symbol);
    RecommendationRunDto {
        recommendation_id: format!(
            "rec-watch-no-market-{}",
            OffsetDateTime::now_utc().unix_timestamp_nanos()
        ),
        status: "completed".into(),
        trigger_type: trigger_source.into(),
        has_trade: false,
        symbol: symbol.map(str::to_string),
        stock_name: None,
        direction: None,
        market_type: "perpetual".into(),
        exchanges: enabled_exchanges.to_vec(),
        confidence_score: 0.0,
        rationale: no_trade_reason.clone(),
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
        no_trade_reason: Some(no_trade_reason),
        risk_details: Default::default(),
        data_snapshot_at: generated_at.clone(),
        model_provider: runtime.model_provider.clone(),
        model_name: runtime.model_name.clone(),
        prompt_version: llm::RECOMMENDATION_PROMPT_VERSION.into(),
        user_preference_version: llm::recommendation_user_preference_version(runtime),
        generated_at,
    }
}

fn no_live_market_reason(enabled_exchanges: &[String], symbol: Option<&str>) -> String {
    let exchange_summary = if enabled_exchanges.is_empty() {
        "the enabled exchange set".into()
    } else {
        enabled_exchanges.join(", ")
    };

    match symbol {
        Some(symbol) => format!(
            "No live market data was available for {symbol} across {exchange_summary}, so the scan finished without an AI recommendation."
        ),
        None => format!(
            "No live market data was available across {exchange_summary}, so the scan finished without an AI recommendation."
        ),
    }
}

fn recommendation_job_reason(run: &RecommendationRunDto) -> Option<String> {
    let rationale = run.rationale.trim();
    if rationale.is_empty() {
        None
    } else {
        Some(rationale.to_string())
    }
}

pub fn should_start_auto_analyze(
    runtime: &RuntimeSettingsDto,
    now_ms: i64,
    last_run_ms: Option<i64>,
) -> bool {
    if !runtime.auto_analyze_enabled {
        return false;
    }

    match last_run_ms {
        None => true,
        Some(last_run_ms) => {
            now_ms.saturating_sub(last_run_ms) >= auto_analyze_interval_ms(runtime)
        }
    }
}

pub async fn run_paper_order_monitor_cycle(
    job_service: &JobService,
    market_data_service: &MarketDataService,
    paper_service: &PaperService,
    settings_service: &SettingsService,
    notification_service: &NotificationService,
    app_handle: Option<&tauri::AppHandle>,
) -> anyhow::Result<usize> {
    let runtime = settings_service.get_runtime_settings();
    if runtime.account_mode == "real_read_only" {
        return Ok(0);
    }

    let exits = paper_service
        .sync_with_market_data(market_data_service)
        .await?;
    if exits.is_empty() {
        return Ok(0);
    }

    let summary = exits
        .iter()
        .map(|exit| {
            format!(
                "{} {} on {} with realized PnL {:.2} CNY",
                exit.status, exit.symbol, exit.exchange, exit.realized_pnl_usdt
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let job_id = job_service.start_job(
        kinds::PAPER_ORDER_MONITOR,
        "Monitoring open paper positions for stop-loss and take-profit exits",
        Some(format!(r#"{{"closedPositions":{}}}"#, exits.len())),
    );
    let mut notification_errors = Vec::new();
    if runtime.notifications.paper_orders {
        for exit in &exits {
            if let Err(error) = notification_service.dispatch_paper_order_event(app_handle, exit) {
                notification_errors.push(error.to_string());
            }
        }
    }
    job_service.finish_job(
        job_id,
        "done",
        &format!(
            "Closed {} paper position(s): {summary}.{}",
            exits.len(),
            if notification_errors.is_empty() {
                String::new()
            } else {
                format!(
                    " Notification dispatch failed for {} exit(s): {}.",
                    notification_errors.len(),
                    notification_errors.join("; ")
                )
            }
        ),
        Some(summary),
        if notification_errors.is_empty() {
            None
        } else {
            Some(notification_errors.join("; "))
        },
    );

    Ok(exits.len())
}

fn auto_analyze_interval_ms(runtime: &RuntimeSettingsDto) -> i64 {
    match runtime.auto_analyze_frequency.as_str() {
        "5m" => 5 * 60 * 1000,
        "30m" => 30 * 60 * 1000,
        "1h" => 60 * 60 * 1000,
        _ => 10 * 60 * 1000,
    }
}

fn preferred_paper_account_id(runtime: &RuntimeSettingsDto) -> String {
    runtime
        .exchanges
        .iter()
        .find(|exchange| exchange.enabled)
        .map(|exchange| paper_account_id(&exchange.exchange))
        .unwrap_or_else(|| "paper-cash".into())
}

fn recommendation_risk_equity_usdt(
    runtime: &RuntimeSettingsDto,
    paper_service: &PaperService,
) -> f64 {
    if runtime.account_mode == "paper" {
        let account_id = preferred_paper_account_id(runtime);
        return paper_service
            .list_accounts_snapshot()
            .into_iter()
            .find(|account| account.account_id == account_id)
            .map(|account| account.available_usdt)
            .unwrap_or(10_000.0);
    }

    10_000.0
}

fn build_market_snapshot_json(
    run: &RecommendationRunDto,
    market_rows: &[crate::models::MarketListRow],
    enabled_exchanges: &[String],
    prompts: Option<(&str, &str)>,
) -> String {
    let focus_symbol = run.symbol.clone();
    let (system_prompt, user_prompt) = prompts.unwrap_or(("", ""));
    let shortlist = shortlist_snapshot_from_user_prompt(user_prompt);
    let shortlist_symbols = shortlist_symbols_from_snapshot(&shortlist);
    let rows = market_rows
        .iter()
        .filter(|row| {
            if let Some(symbol) = focus_symbol.as_deref() {
                return row.symbol == symbol && row.market_type == run.market_type;
            }

            shortlist_symbols.is_empty()
                || shortlist_symbols.contains(&row.symbol.to_ascii_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();
    serde_json::json!({
        "data_snapshot_at": run.data_snapshot_at,
        "trigger_type": run.trigger_type,
        "focus_symbol": focus_symbol,
        "market_type": run.market_type,
        "enabled_exchanges": enabled_exchanges,
        "shortlist": shortlist,
        "market_rows": rows,
        "system_prompt": if system_prompt.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(system_prompt.into()) },
        "user_prompt": if user_prompt.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(user_prompt.into()) },
    })
    .to_string()
}

fn shortlist_snapshot_from_user_prompt(user_prompt: &str) -> serde_json::Value {
    let Ok(prompt_value) = serde_json::from_str::<serde_json::Value>(user_prompt) else {
        return serde_json::Value::Null;
    };

    let spot_shortlist = prompt_value
        .get("spot_shortlist")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let perpetual_shortlist = prompt_value
        .get("perpetual_shortlist")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let all_symbols = shortlist_symbols_from_prompt_value(&prompt_value);

    serde_json::json!({
        "spot_shortlist": spot_shortlist,
        "perpetual_shortlist": perpetual_shortlist,
        "all_symbols": all_symbols,
    })
}

fn financial_analysis_contexts(
    financial_report_service: Option<&FinancialReportService>,
    runtime: &crate::models::RuntimeSettingsDto,
    rows: &[crate::models::MarketListRow],
) -> std::collections::HashMap<String, llm::FinancialAnalysisPromptContext> {
    if !runtime.use_financial_report_data {
        return std::collections::HashMap::new();
    }
    let Some(service) = financial_report_service else {
        return std::collections::HashMap::new();
    };
    rows.iter()
        .filter_map(|row| {
            let snapshot = service.snapshot(&row.symbol).ok()?;
            let analysis = snapshot.analysis?;
            Some((
                row.symbol.clone(),
                llm::FinancialAnalysisPromptContext {
                    key_summary: analysis.key_summary,
                    positive_factors: analysis.positive_factors,
                    negative_factors: analysis.negative_factors,
                    fraud_risk_points: analysis.fraud_risk_points,
                },
            ))
        })
        .collect()
}

fn shortlist_symbols_from_snapshot(value: &serde_json::Value) -> std::collections::HashSet<String> {
    shortlist_symbols_from_prompt_value(value)
        .into_iter()
        .map(|symbol| symbol.to_ascii_lowercase())
        .collect()
}

fn shortlist_symbols_from_prompt_value(value: &serde_json::Value) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut symbols = Vec::new();

    for key in ["spot_shortlist", "perpetual_shortlist", "all_symbols"] {
        let Some(items) = value.get(key).and_then(serde_json::Value::as_array) else {
            continue;
        };
        for item in items {
            let symbol = item.as_str().map(str::to_string).or_else(|| {
                item.get("symbol")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            });
            let Some(symbol) = symbol else {
                continue;
            };
            if seen.insert(symbol.to_ascii_lowercase()) {
                symbols.push(symbol);
            }
        }
    }

    symbols
}

fn build_account_snapshot_json(
    runtime: &RuntimeSettingsDto,
    paper_service: &PaperService,
    risk_equity_usdt: f64,
) -> String {
    serde_json::json!({
        "account_mode": runtime.account_mode,
        "auto_paper_execution": runtime.auto_paper_execution,
        "risk_equity_usdt": risk_equity_usdt,
        "paper_accounts": paper_service.list_accounts_snapshot(),
    })
    .to_string()
}

fn current_utc_millis() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

async fn auto_recommendation_block_reason(
    recommendation_service: &RecommendationService,
    runtime: &RuntimeSettingsDto,
) -> anyhow::Result<Option<String>> {
    let day_start_unix = OffsetDateTime::now_utc()
        .replace_time(Time::MIDNIGHT)
        .unix_timestamp();
    let runs_today = recommendation_service
        .count_runs_since("auto", day_start_unix)
        .await?;
    if runs_today >= runtime.daily_max_ai_calls {
        return Ok(Some(format!(
            "Auto recommendation paused: daily AI call limit reached ({runs_today}/{}) for today.",
            runtime.daily_max_ai_calls
        )));
    }

    if runtime.pause_after_consecutive_losses > 0 {
        let streak = recommendation_service
            .count_consecutive_losing_evaluations("auto", "24h")
            .await?;
        if streak >= runtime.pause_after_consecutive_losses {
            return Ok(Some(format!(
                "Auto recommendation paused: consecutive losing recommendations reached the configured pause threshold ({streak}/{}).",
                runtime.pause_after_consecutive_losses
            )));
        }
    }

    let accumulated_loss_percent = recommendation_service
        .sum_negative_pnl_percent_since("auto", "24h", day_start_unix)
        .await?;
    if accumulated_loss_percent >= runtime.max_daily_loss_percent {
        return Ok(Some(format!(
            "Auto recommendation paused: daily loss limit reached ({accumulated_loss_percent:.2}%/{:.2}%).",
            runtime.max_daily_loss_percent
        )));
    }

    Ok(None)
}
