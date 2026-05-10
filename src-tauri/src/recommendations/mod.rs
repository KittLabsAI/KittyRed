pub mod automation;
pub mod evaluator;
mod ledger;
pub mod llm;
pub mod risk_engine;

#[cfg(test)]
mod tests {
    use super::{
        append_pending_history_row, build_missing_evaluations_from_bar_sets, build_trade_plan,
        missing_evaluation_horizons, parse_rfc3339_millis, refresh_history_row_from_bars,
        round_percent, EvaluationBarSets,
    };
    use crate::market::MarketDataService;
    use crate::models::{
        default_assistant_system_prompt, default_recommendation_system_prompt, MarketListRow,
        OhlcvBar, RecommendationRunDto, RuntimeNotificationSettingsDto, RuntimeSettingsDto,
    };
    use crate::recommendations::ledger::RecommendationEvaluation;
    use crate::recommendations::risk_engine;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_explicit_execution_fields_for_tradeable_market() {
        let run = build_trade_plan(
            &MarketListRow {
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
            },
            &risk_engine::risk_settings_from_runtime(&runtime_settings(), 10_000.0),
        );

        assert!(run.has_trade);
        assert_eq!(run.risk_status, "approved");
        assert!(run.entry_low.is_some());
        assert!(run.entry_high.is_some());
        assert!(run.stop_loss.is_some());
        assert!(run.amount_cny.is_some());
        assert!(run.leverage.is_some());
        assert!(run.invalidation.is_some());
        assert!(run.max_loss_cny.is_some());
        assert_eq!(run.risk_details.status, "approved");
        assert!(run.risk_details.block_reasons.is_empty());
        assert!(run
            .risk_details
            .checks
            .iter()
            .any(|check| check.name == "max_leverage"));
    }

    #[test]
    fn watchlist_only_scan_prefers_candidates_inside_the_watchlist() {
        let service = super::RecommendationService::default();
        let mut runtime = runtime_settings();
        runtime.scan_scope = "watchlist_only".into();
        runtime.watchlist_symbols = vec!["ETH/USDT".into()];
        let rows = vec![
            MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "perpetual".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: "small".into(),
                last_price: 68_420.0,
                change_24h: 5.8,
                volume_24h: 280_000_000.0,
                funding_rate: Some(0.018),
                spread_bps: 2.1,
                exchanges: vec!["akshare".into(), "人民币现金".into()],
                updated_at: "2026-05-03T18:20:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            },
            MarketListRow {
                symbol: "ETH/USDT".into(),
                base_asset: "ETH".into(),
                market_type: "perpetual".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: "small".into(),
                last_price: 3_420.0,
                change_24h: 2.8,
                volume_24h: 190_000_000.0,
                funding_rate: Some(0.011),
                spread_bps: 3.4,
                exchanges: vec!["akshare".into(), "深圳证券交易所".into()],
                updated_at: "2026-05-03T18:20:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            },
        ];

        let run = service
            .plan_manual(&rows, &runtime, 10_000.0, None)
            .expect("watchlist-only plan should resolve");

        assert_eq!(run.symbol.as_deref(), Some("ETH/USDT"));
    }

    #[test]
    fn keeps_blocked_risk_status_when_a_raw_setup_fails_the_risk_gate() {
        let mut runtime = runtime_settings();
        runtime.max_leverage = 1.0;

        let run = build_trade_plan(
            &MarketListRow {
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
            },
            &risk_engine::risk_settings_from_runtime(&runtime, 10_000.0),
        );

        assert!(!run.has_trade);
        assert_eq!(run.risk_status, "blocked");
        assert_eq!(run.risk_details.status, "blocked");
        assert!(run
            .risk_details
            .block_reasons
            .contains(&"max_leverage_exceeded".to_string()));
    }

    fn runtime_settings() -> RuntimeSettingsDto {
        RuntimeSettingsDto {
            exchanges: Vec::new(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-5.5".into(),
            model_base_url: String::new(),
            model_temperature: 0.2,
            model_max_tokens: 900,
            model_max_context: 16_000,
            has_stored_model_api_key: false,
            auto_analyze_enabled: false,
            auto_analyze_frequency: "10m".into(),
            scan_scope: "all_markets".into(),
            watchlist_symbols: Vec::new(),
            daily_max_ai_calls: 24,
            use_bid_ask_data: true,
            use_financial_report_data: false,
            ai_kline_bar_count: 60,
            ai_kline_frequencies: crate::models::default_ai_kline_frequencies(),
            pause_after_consecutive_losses: 3,
            min_confidence_score: 60.0,
            allowed_markets: "all".into(),
            allowed_direction: "long_short".into(),
            max_leverage: 3.0,
            max_loss_per_trade_percent: 1.0,
            max_daily_loss_percent: 3.0,
            min_risk_reward_ratio: 1.5,
            min_volume_24h: 20_000_000.0,
            max_spread_bps: 12.0,
            allow_meme_coins: true,
            whitelist_symbols: Vec::new(),
            blacklist_symbols: Vec::new(),
            prompt_extension: String::new(),
            assistant_system_prompt: default_assistant_system_prompt(),
            recommendation_system_prompt: default_recommendation_system_prompt(),
            account_mode: "paper".into(),
            auto_paper_execution: false,
            notifications: RuntimeNotificationSettingsDto {
                recommendations: true,
                spreads: true,
                paper_orders: true,
            },
            signals_enabled: false,
            signal_scan_frequency: "15m".into(),
            signal_min_score: 30.0,
            signal_cooldown_minutes: 15,
            signal_daily_max: 50,
            signal_auto_execute: false,
            signal_notifications: false,
            signal_watchlist_symbols: vec![],
        }
    }

    #[test]
    fn appends_pending_history_row_for_new_recommendation() {
        let run = RecommendationRunDto {
            recommendation_id: "rec-test-1".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("SOL/USDT".into()),
            stock_name: Some("浦发银行".into()),
            direction: Some("Long".into()),
            market_type: "perpetual".into(),
            exchanges: vec!["akshare".into(), "深圳证券交易所".into()],
            confidence_score: 74.0,
            rationale: "Momentum and contained basis.".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(176.8),
            entry_high: Some(178.2),
            stop_loss: Some(171.5),
            take_profit: Some("183.4 / 187.9".into()),
            leverage: Some(3.0),
            amount_cny: Some(1_500.0),
            invalidation: Some("Lose 171.5 on expanding sell pressure.".into()),
            max_loss_cny: Some(44.7),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-03T18:18:00+08:00".into(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-5.5".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-test".into(),
            generated_at: "2026-05-03T18:18:00+08:00".into(),
        };

        let row = append_pending_history_row(&run);
        assert_eq!(row.symbol, "SOL/USDT");
        assert_eq!(row.stock_name, "浦发银行");
        assert_eq!(row.exchange, "akshare");
        assert_eq!(row.result, "Pending");
        assert_eq!(row.direction, "Long");
        assert_eq!(row.pnl_10m, 0.0);
        assert!(row.outcome.contains("等待下一交易K线"));
    }

    #[test]
    fn skips_refresh_when_all_evaluation_horizons_are_cached() {
        let evaluations = ["5m", "10m", "30m", "60m", "24h", "7d"]
            .into_iter()
            .map(|horizon| RecommendationEvaluation {
                evaluation_id: format!("eval-{horizon}"),
                recommendation_id: "rec-cached".into(),
                horizon: horizon.into(),
                price_at_horizon: 100.0,
                max_favorable_price: 101.0,
                max_adverse_price: 99.0,
                take_profit_hit: false,
                stop_loss_hit: false,
                estimated_fee: 0.0,
                estimated_slippage: 0.0,
                funding_fee: 0.0,
                estimated_pnl: 0.0,
                estimated_pnl_percent: 0.0,
                result: "Flat".into(),
                evaluated_at: "2026-05-03T20:00:00Z".into(),
            })
            .collect::<Vec<_>>();

        assert!(missing_evaluation_horizons(&evaluations).is_empty());
    }

    #[test]
    fn backfills_elapsed_recommendation_windows_from_candles() {
        let run = RecommendationRunDto {
            recommendation_id: "rec-eval-1".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some("ETH/USDT".into()),
            stock_name: Some("ETH".into()),
            direction: Some("Long".into()),
            market_type: "perpetual".into(),
            exchanges: vec!["akshare".into(), "人民币现金".into()],
            confidence_score: 70.0,
            rationale: "Evaluation backfill regression".into(),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(99.0),
            entry_high: Some(101.0),
            stop_loss: Some(96.0),
            take_profit: Some("105 / 110".into()),
            leverage: Some(1.0),
            amount_cny: Some(1_000.0),
            invalidation: Some("Lose 96".into()),
            max_loss_cny: Some(30.0),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "1970-01-01T00:00:00Z".into(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-5.5".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-test".into(),
            generated_at: "1970-01-01T00:00:00Z".into(),
        };
        let pending = append_pending_history_row(&run);
        let bars = vec![
            OhlcvBar {
                open_time: "300000".into(),
                open: 100.0,
                high: 106.0,
                low: 99.0,
                close: 105.0,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "600000".into(),
                open: 105.0,
                high: 105.0,
                low: 101.0,
                close: 102.0,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "1800000".into(),
                open: 102.0,
                high: 103.0,
                low: 97.0,
                close: 98.0,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "3600000".into(),
                open: 98.0,
                high: 111.0,
                low: 98.0,
                close: 110.0,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "86400000".into(),
                open: 110.0,
                high: 121.0,
                low: 109.0,
                close: 120.0,
                volume: 100.0,
                turnover: None,
            },
        ];

        let updated = refresh_history_row_from_bars(&run, &pending, &bars, 90_000_000)
            .expect("elapsed horizons should be evaluated");

        assert!((updated.pnl_5m - 4.9).abs() < 0.0001);
        assert!((updated.pnl_10m - 1.9).abs() < 0.0001);
        assert!((updated.pnl_30m - (-2.1)).abs() < 0.0001);
        assert!((updated.pnl_60m - 9.9).abs() < 0.0001);
        assert!((updated.pnl_24h - 19.9).abs() < 0.0001);
        assert!(updated.outcome.contains("24小时"));
    }

    #[test]
    fn backfills_recommendation_windows_from_akshare_datetime_bars() {
        let mut run = sample_persisted_no_trade_run();
        run.recommendation_id = "rec-eval-akshare-time".into();
        run.has_trade = true;
        run.symbol = Some("SHSE.600000".into());
        run.stock_name = Some("浦发银行".into());
        run.direction = Some("买入".into());
        run.market_type = "A 股".into();
        run.entry_low = Some(9.9);
        run.entry_high = Some(10.1);
        run.generated_at = "2026-05-06T09:30:00+08:00".into();

        let pending = append_pending_history_row(&run);
        let bars = vec![
            OhlcvBar {
                open_time: "2026-05-06 09:35:00".into(),
                open: 10.0,
                high: 10.1,
                low: 9.9,
                close: 10.1,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "2026-05-06 09:40:00".into(),
                open: 10.1,
                high: 10.25,
                low: 10.0,
                close: 10.2,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "2026-05-06 10:30:00".into(),
                open: 10.2,
                high: 10.35,
                low: 10.1,
                close: 10.3,
                volume: 100.0,
                turnover: None,
            },
            OhlcvBar {
                open_time: "2026-05-07 09:30:00".into(),
                open: 10.3,
                high: 10.55,
                low: 10.2,
                close: 10.5,
                volume: 100.0,
                turnover: None,
            },
        ];
        let now_ms =
            parse_rfc3339_millis("2026-05-07T10:00:00+08:00").expect("timestamp should parse");

        let updated = refresh_history_row_from_bars(&run, &pending, &bars, now_ms)
            .expect("formatted AKShare bars should be evaluated");

        assert!((updated.pnl_10m - 1.9).abs() < 0.0001);
        assert!((updated.pnl_60m - 2.9).abs() < 0.0001);
        assert!((updated.pnl_24h - 4.9).abs() < 0.0001);
    }

    #[test]
    fn short_recommendation_windows_do_not_use_daily_bars() {
        let mut run = sample_persisted_no_trade_run();
        run.recommendation_id = "rec-eval-window-source".into();
        run.has_trade = true;
        run.symbol = Some("SHSE.600000".into());
        run.stock_name = Some("浦发银行".into());
        run.direction = Some("买入".into());
        run.market_type = "A 股".into();
        run.entry_low = Some(9.9);
        run.entry_high = Some(10.1);
        run.generated_at = "2026-05-06T09:30:00+08:00".into();

        let bar_sets = EvaluationBarSets {
            intraday: vec![OhlcvBar {
                open_time: "2026-05-06 09:40:00".into(),
                open: 10.1,
                high: 10.25,
                low: 10.0,
                close: 10.2,
                volume: 100.0,
                turnover: None,
            }],
            hourly: vec![OhlcvBar {
                open_time: "2026-05-06 10:30:00".into(),
                open: 10.2,
                high: 10.35,
                low: 10.1,
                close: 10.3,
                volume: 100.0,
                turnover: None,
            }],
            daily: vec![OhlcvBar {
                open_time: "2026-05-07".into(),
                open: 14.8,
                high: 15.2,
                low: 14.5,
                close: 15.0,
                volume: 100.0,
                turnover: None,
            }],
        };
        let now_ms =
            parse_rfc3339_millis("2026-05-07T10:00:00+08:00").expect("timestamp should parse");

        let evaluations =
            build_missing_evaluations_from_bar_sets(&run, &bar_sets, now_ms, &HashSet::new())
                .expect("evaluations should build");
        let pnl = evaluations
            .iter()
            .map(|evaluation| {
                (
                    evaluation.horizon.as_str(),
                    round_percent(evaluation.estimated_pnl_percent),
                )
            })
            .collect::<Vec<_>>();

        assert!(pnl.contains(&("10m", 1.9)));
        assert!(pnl.contains(&("60m", 2.9)));
        assert!(pnl.contains(&("24h", 49.9)));
    }

    #[tokio::test]
    async fn persists_recommendation_runs_across_service_restart() {
        let path = unique_temp_recommendation_db_path("recommendation-ledger");
        let service = super::RecommendationService::new(path.clone());
        let run = sample_persisted_no_trade_run();

        service
            .store_record(super::RecommendationRecord {
                run: run.clone(),
                trigger_type: run.trigger_type.clone(),
                ai_raw_output: "{\"has_trade\":false}".into(),
                ai_structured_output: "{\"reason\":\"manual_audit_test\"}".into(),
                risk_result: "{\"status\":\"watch\",\"reason\":\"manual_audit_test\"}".into(),
                market_snapshot: sample_market_snapshot_json(),
                account_snapshot: "{\"account_mode\":\"paper\"}".into(),
            })
            .await
            .expect("recommendation record should persist");

        let restored = super::RecommendationService::new(path.clone());
        let latest = restored
            .get_latest()
            .await
            .expect("latest lookup should succeed")
            .into_iter()
            .next()
            .expect("persisted latest recommendation should exist");
        assert_eq!(latest.recommendation_id, run.recommendation_id);
        assert_eq!(latest.model_name, "gpt-5.5");
        assert_eq!(latest.prompt_version, "recommendation-system-v2");

        let history = restored
            .list_history(&MarketDataService::with_static_rows(Vec::new()), &[])
            .await
            .expect("history lookup should succeed");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].symbol, "No Recommendation");
        assert_eq!(history[0].model_name, "gpt-5.5");
        assert_eq!(history[0].trigger_type, "manual");
        assert_eq!(
            history[0].shortlist,
            vec![
                "BTC/USDT".to_string(),
                "ETH/USDT".to_string(),
                "SOL/USDT".to_string()
            ]
        );
        assert!(!history[0].executed);
        assert!(!history[0].modified);

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn appends_user_execution_actions_without_mutating_the_recommendation_record() {
        let path = unique_temp_recommendation_db_path("recommendation-actions");
        let service = super::RecommendationService::new(path.clone());
        let run = sample_persisted_no_trade_run();

        service
            .store_record(super::RecommendationRecord {
                run: run.clone(),
                trigger_type: run.trigger_type.clone(),
                ai_raw_output: "{\"has_trade\":false}".into(),
                ai_structured_output: "{\"reason\":\"manual_audit_test\"}".into(),
                risk_result: "{\"status\":\"watch\",\"reason\":\"manual_audit_test\"}".into(),
                market_snapshot: sample_market_snapshot_json(),
                account_snapshot: "{\"account_mode\":\"paper\"}".into(),
            })
            .await
            .expect("recommendation record should persist");

        service
            .append_user_action(
                &run.recommendation_id,
                "executed",
                "{\"account_id\":\"paper-cash\"}",
            )
            .await
            .expect("execution audit event should append");

        let history = service
            .list_history(&MarketDataService::with_static_rows(Vec::new()), &[])
            .await
            .expect("history lookup should succeed");
        assert_eq!(history.len(), 1);
        assert!(history[0].executed);
        assert!(!history[0].modified);

        let resolved = service
            .resolve_recommendation(&run.recommendation_id)
            .await
            .expect("resolve should succeed")
            .expect("stored recommendation should resolve");
        assert_eq!(resolved.recommendation_id, run.recommendation_id);
        assert_eq!(resolved.no_trade_reason.as_deref(), Some("No clean setup."));

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn loads_persisted_audit_payload_for_history_drilldown() {
        let path = unique_temp_recommendation_db_path("recommendation-audit");
        let service = super::RecommendationService::new(path.clone());
        let run = sample_persisted_no_trade_run();

        service
            .store_record(super::RecommendationRecord {
                run: run.clone(),
                trigger_type: run.trigger_type.clone(),
                ai_raw_output: "{\"has_trade\":false}".into(),
                ai_structured_output: "{\"reason\":\"manual_audit_test\"}".into(),
                risk_result: "{\"status\":\"watch\",\"reason\":\"manual_audit_test\"}".into(),
                market_snapshot: sample_market_snapshot_json(),
                account_snapshot: "{\"account_mode\":\"paper\"}".into(),
            })
            .await
            .expect("recommendation record should persist");

        let audit = service
            .load_audit(&run.recommendation_id)
            .await
            .expect("audit lookup should succeed")
            .expect("stored audit payload should exist");
        assert_eq!(audit.recommendation_id, run.recommendation_id);
        assert_eq!(audit.symbol, "No Recommendation");
        assert_eq!(audit.exchange, "akshare");
        assert_eq!(audit.prompt_version, "recommendation-system-v2");
        assert_eq!(audit.account_snapshot, "{\"account_mode\":\"paper\"}");
        assert!(audit.risk_result.contains("\"status\":\"watch\""));
        assert!(audit.market_snapshot.contains("\"shortlist\""));

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn deletes_persisted_recommendation_records_from_history_and_audit() {
        let path = unique_temp_recommendation_db_path("recommendation-delete");
        let service = super::RecommendationService::new(path.clone());
        let run = sample_persisted_no_trade_run();

        service
            .store_record(super::RecommendationRecord {
                run: run.clone(),
                trigger_type: run.trigger_type.clone(),
                ai_raw_output: "{\"has_trade\":false}".into(),
                ai_structured_output: "{\"reason\":\"manual_audit_test\"}".into(),
                risk_result: "{\"status\":\"watch\",\"reason\":\"manual_audit_test\"}".into(),
                market_snapshot: sample_market_snapshot_json(),
                account_snapshot: "{\"account_mode\":\"paper\"}".into(),
            })
            .await
            .expect("recommendation record should persist");

        service
            .append_user_action(
                &run.recommendation_id,
                "executed",
                "{\"account_id\":\"paper-cash\"}",
            )
            .await
            .expect("execution audit event should append");

        service
            .delete_recommendation(&run.recommendation_id)
            .await
            .expect("recommendation should delete");

        assert!(service
            .resolve_recommendation(&run.recommendation_id)
            .await
            .expect("resolve should succeed")
            .is_none());
        assert!(service
            .load_audit(&run.recommendation_id)
            .await
            .expect("audit lookup should succeed")
            .is_none());
        assert!(service
            .list_history(&MarketDataService::with_static_rows(Vec::new()), &[])
            .await
            .expect("history lookup should succeed")
            .is_empty());

        let _ = std::fs::remove_file(path);
    }

    fn sample_persisted_no_trade_run() -> RecommendationRunDto {
        RecommendationRunDto {
            recommendation_id: "rec-persisted-1".into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: false,
            symbol: None,
            stock_name: None,
            direction: None,
            market_type: "spot".into(),
            exchanges: vec!["akshare".into(), "人民币现金".into()],
            confidence_score: 44.0,
            rationale: "Manual persistence regression".into(),
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
            no_trade_reason: Some("No clean setup.".into()),
            risk_details: crate::models::RiskDecisionDto::default(),
            data_snapshot_at: "2026-05-04T00:30:00Z".into(),
            model_provider: "OpenAI-compatible".into(),
            model_name: "gpt-5.5".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-regression".into(),
            generated_at: "2026-05-04T00:30:00Z".into(),
        }
    }

    fn sample_market_snapshot_json() -> String {
        serde_json::json!({
            "shortlist": {
                "spot_shortlist": [
                    { "symbol": "BTC/USDT" },
                    { "symbol": "ETH/USDT" }
                ],
                "perpetual_shortlist": [
                    { "symbol": "SOL/USDT" }
                ]
            }
        })
        .to_string()
    }

    fn unique_temp_recommendation_db_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyalpha-{label}-{nanos}.sqlite3"))
    }
}

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{anyhow, Context};
use futures::future::join_all;
use ledger::{PersistedRecommendation, RecommendationEvaluation, RecommendationLedger};
use time::format_description::{parse as parse_time_format, well_known::Rfc3339};
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use crate::market::{coin_info, MarketDataService};
use crate::models::{
    MarketListRow, OhlcvBar, RecommendationAuditDto, RecommendationGenerationProgressDto,
    RecommendationGenerationProgressItemDto, RecommendationHistoryRowDto, RecommendationRunDto,
    RiskDecisionDto, RuntimeSettingsDto,
};
use evaluator::estimate_pnl_percent;
use risk_engine::{evaluate_plan, risk_settings_from_runtime, CandidatePlan, RiskSettings};

pub(crate) use ledger::RecommendationRecord;

#[derive(Clone)]
pub struct RecommendationService {
    ledger: Arc<RecommendationLedger>,
    latest: Arc<RwLock<Vec<RecommendationRunDto>>>,
    generation_progress: Arc<Mutex<RecommendationGenerationProgressDto>>,
}

impl Default for RecommendationService {
    fn default() -> Self {
        let service = Self::new(unique_temp_default_recommendation_db_path(
            "recommendation-default",
        ));
        let _ = service.store_record_sync(RecommendationRecord {
            trigger_type: "seed".into(),
            ai_raw_output: "{\"source\":\"seed\"}".into(),
            ai_structured_output: "{\"source\":\"seed\"}".into(),
            risk_result: default_risk_result_json(&seed_latest_recommendation()),
            market_snapshot: "{\"source\":\"seed\"}".into(),
            account_snapshot: "{\"source\":\"seed\"}".into(),
            run: seed_latest_recommendation(),
        });
        service
    }
}

impl RecommendationService {
    pub fn new(path: PathBuf) -> Self {
        let ledger =
            RecommendationLedger::new(path).expect("recommendation ledger should initialize");
        let latest = ledger
            .load_latest_runs()
            .expect("latest recommendation should load");

        Self {
            ledger: Arc::new(ledger),
            latest: Arc::new(RwLock::new(latest)),
            generation_progress: Arc::new(Mutex::new(idle_generation_progress())),
        }
    }

    pub async fn get_latest(&self) -> anyhow::Result<Vec<RecommendationRunDto>> {
        let latest = self
            .latest
            .read()
            .map_err(|_| anyhow!("failed to read latest recommendation state"))?;
        Ok(latest.clone())
    }

    pub async fn resolve_recommendation(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Option<RecommendationRunDto>> {
        let latest = self
            .latest
            .read()
            .map_err(|_| anyhow!("failed to read latest recommendation state"))?;
        if let Some(run) = latest
            .iter()
            .find(|item| item.recommendation_id == recommendation_id)
            .cloned()
        {
            return Ok(Some(run));
        }
        drop(latest);

        self.ledger.load_run(recommendation_id)
    }

    pub async fn load_audit(
        &self,
        recommendation_id: &str,
    ) -> anyhow::Result<Option<RecommendationAuditDto>> {
        self.ledger.load_audit_record(recommendation_id)
    }

    pub fn generation_progress(&self) -> anyhow::Result<RecommendationGenerationProgressDto> {
        Ok(self
            .generation_progress
            .lock()
            .expect("recommendation generation progress lock poisoned")
            .clone())
    }

    pub fn initialize_generation_progress(
        &self,
        watchlist_symbols: &[String],
        rows: &[MarketListRow],
    ) {
        let items = watchlist_symbols
            .iter()
            .map(|symbol| RecommendationGenerationProgressItemDto {
                stock_code: symbol.clone(),
                short_name: rows
                    .iter()
                    .find(|row| row.symbol == *symbol)
                    .map(|row| row.base_asset.clone())
                    .unwrap_or_else(|| symbol.rsplit('.').next().unwrap_or(symbol).to_string()),
                status: "idle".into(),
                attempt: 0,
                error_message: None,
            })
            .collect::<Vec<_>>();
        let mut progress = self
            .generation_progress
            .lock()
            .expect("recommendation generation progress lock poisoned");
        *progress = RecommendationGenerationProgressDto {
            status: "running".into(),
            completed_count: 0,
            total_count: items.len() as u32,
            message: if items.is_empty() {
                "自选股票池为空，无法生成 AI 建议".into()
            } else {
                "正在生成 AI 建议".into()
            },
            items,
        };
    }

    pub fn update_generation_item(
        &self,
        stock_code: &str,
        status: &str,
        attempt: u32,
        error_message: Option<String>,
    ) {
        let mut progress = self
            .generation_progress
            .lock()
            .expect("recommendation generation progress lock poisoned");
        if let Some(item) = progress.items.iter_mut().find(|item| item.stock_code == stock_code) {
            item.status = status.into();
            item.attempt = attempt;
            item.error_message = error_message;
        } else {
            return;
        }
        progress.completed_count = progress
            .items
            .iter()
            .filter(|item| matches!(item.status.as_str(), "succeeded" | "failed"))
            .count() as u32;
        progress.total_count = progress.items.len() as u32;
        progress.message = if let Some(item) = progress.items.iter().find(|item| item.stock_code == stock_code) {
            if status == "failed" {
                format!("{} 建议生成失败", item.short_name)
            } else if status == "succeeded" {
                format!("{} 建议生成完成", item.short_name)
            } else {
                format!("正在生成 {} 建议", item.short_name)
            }
        } else {
            "正在生成 AI 建议".into()
        };
    }

    pub fn complete_generation_progress(&self, message: String) {
        let mut progress = self
            .generation_progress
            .lock()
            .expect("recommendation generation progress lock poisoned");
        progress.completed_count = progress
            .items
            .iter()
            .filter(|item| matches!(item.status.as_str(), "succeeded" | "failed"))
            .count() as u32;
        progress.total_count = progress.items.len() as u32;
        progress.status = "completed".into();
        progress.message = message;
    }

    pub fn fail_generation_progress(&self, message: String) {
        let mut progress = self
            .generation_progress
            .lock()
            .expect("recommendation generation progress lock poisoned");
        progress.completed_count = progress
            .items
            .iter()
            .filter(|item| matches!(item.status.as_str(), "succeeded" | "failed"))
            .count() as u32;
        progress.total_count = progress.items.len() as u32;
        progress.status = "failed".into();
        progress.message = message;
    }

    pub fn plan_manual(
        &self,
        rows: &[MarketListRow],
        runtime: &RuntimeSettingsDto,
        account_equity_usdt: f64,
        symbol: Option<String>,
    ) -> anyhow::Result<RecommendationRunDto> {
        let decision_rows = crate::market::perpetual_primary_venue_rows(rows, &[]);
        let risk_settings = risk_settings_from_runtime(runtime, account_equity_usdt);
        if let Some(focus) =
            select_focus_market(&decision_rows, &risk_settings, runtime, symbol.as_deref())
        {
            Ok(build_trade_plan(focus, &risk_settings))
        } else if let Some(fallback) = decision_rows
            .iter()
            .find(|row| {
                symbol
                    .as_deref()
                    .map(|target| row.symbol == target)
                    .unwrap_or(true)
            })
            .or_else(|| rows.first())
        {
            Ok(build_no_trade(
                fallback,
                &no_trade_reason_for_scan_scope(runtime, symbol.as_deref()),
            ))
        } else {
            Err(anyhow!(
                "no live perpetual markets available for recommendation scan"
            ))?
        }
    }

    pub async fn trigger_manual(
        &self,
        rows: &[MarketListRow],
        runtime: &RuntimeSettingsDto,
        account_equity_usdt: f64,
        symbol: Option<String>,
    ) -> anyhow::Result<RecommendationRunDto> {
        let run = self.plan_manual(rows, runtime, account_equity_usdt, symbol)?;
        self.store_run(run).await
    }

    pub async fn store_record(
        &self,
        record: RecommendationRecord,
    ) -> anyhow::Result<RecommendationRunDto> {
        self.store_record_sync(record)
    }

    pub async fn append_user_action(
        &self,
        recommendation_id: &str,
        action_type: &str,
        payload: &str,
    ) -> anyhow::Result<()> {
        self.ledger
            .append_user_action(recommendation_id, action_type, payload)
    }

    pub async fn delete_recommendation(&self, recommendation_id: &str) -> anyhow::Result<()> {
        self.ledger.delete_record(recommendation_id)?;

        let mut latest = self
            .latest
            .write()
            .map_err(|_| anyhow!("failed to update latest recommendation state"))?;
        *latest = self.ledger.load_latest_runs()?;
        Ok(())
    }

    pub async fn count_runs_since(
        &self,
        trigger_type: &str,
        start_unix_seconds: i64,
    ) -> anyhow::Result<u32> {
        self.ledger
            .count_runs_since(trigger_type, start_unix_seconds)
    }

    pub async fn count_consecutive_losing_evaluations(
        &self,
        trigger_type: &str,
        horizon: &str,
    ) -> anyhow::Result<u32> {
        self.ledger
            .count_consecutive_losing_evaluations(trigger_type, horizon)
    }

    pub async fn sum_negative_pnl_percent_since(
        &self,
        trigger_type: &str,
        horizon: &str,
        start_unix_seconds: i64,
    ) -> anyhow::Result<f64> {
        self.ledger
            .sum_negative_pnl_percent_since(trigger_type, horizon, start_unix_seconds)
    }

    pub async fn store_run(
        &self,
        run: RecommendationRunDto,
    ) -> anyhow::Result<RecommendationRunDto> {
        let ai_structured_output = serde_json::json!({
            "source": "heuristic_fallback",
            "final_output": &run,
        });
        self.store_record(RecommendationRecord {
            trigger_type: run.trigger_type.clone(),
            ai_raw_output: "{\"source\":\"heuristic_fallback\"}".into(),
            ai_structured_output: serde_json::to_string(&ai_structured_output)?,
            risk_result: default_risk_result_json(&run),
            market_snapshot: "{\"rows\":[]}".into(),
            account_snapshot: "{\"account_mode\":\"unknown\"}".into(),
            run,
        })
        .await
    }

    pub async fn list_history(
        &self,
        market_data_service: &MarketDataService,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<Vec<RecommendationHistoryRowDto>> {
        let records = self.ledger.list_records(50)?;
        let mut evaluations_by_id = HashMap::new();

        for record in &records {
            let evaluations = self
                .ledger
                .list_evaluations(&record.run.recommendation_id)?;
            evaluations_by_id.insert(record.run.recommendation_id.clone(), evaluations);
        }

        let refreshes = records
            .iter()
            .filter(|record| record.run.has_trade)
            .map(|record| {
                let existing = evaluations_by_id
                    .get(&record.run.recommendation_id)
                    .cloned()
                    .unwrap_or_default();
                async move {
                    let refreshed = refresh_missing_evaluations(
                        &record.run,
                        &existing,
                        market_data_service,
                        enabled_exchanges,
                    )
                    .await?;
                    Ok::<_, anyhow::Error>((record.run.recommendation_id.clone(), refreshed))
                }
            });

        for result in join_all(refreshes).await {
            let Ok((recommendation_id, new_evaluations)) = result else {
                continue;
            };
            self.ledger.insert_evaluations(&new_evaluations)?;
            let evaluations = evaluations_by_id.entry(recommendation_id).or_default();
            evaluations.extend(new_evaluations);
            evaluations.sort_by(|left, right| left.horizon.cmp(&right.horizon));
        }

        Ok(records
            .into_iter()
            .map(|record| {
                let evaluations = evaluations_by_id
                    .remove(&record.run.recommendation_id)
                    .unwrap_or_default();
                history_row_from_persisted(&record, &evaluations)
            })
            .collect())
    }

    pub async fn list_history_snapshot(
        &self,
        limit: usize,
    ) -> anyhow::Result<Vec<RecommendationHistoryRowDto>> {
        let records = self.ledger.list_records(limit)?;
        let mut evaluations_by_id = HashMap::new();

        for record in &records {
            let evaluations = self
                .ledger
                .list_evaluations(&record.run.recommendation_id)?;
            evaluations_by_id.insert(record.run.recommendation_id.clone(), evaluations);
        }

        Ok(records
            .into_iter()
            .map(|record| {
                let evaluations = evaluations_by_id
                    .remove(&record.run.recommendation_id)
                    .unwrap_or_default();
                history_row_from_persisted(&record, &evaluations)
            })
            .collect())
    }

    fn store_record_sync(
        &self,
        record: RecommendationRecord,
    ) -> anyhow::Result<RecommendationRunDto> {
        self.ledger.insert_record(&record)?;

        let mut latest = self
            .latest
            .write()
            .map_err(|_| anyhow!("failed to update latest recommendation state"))?;
        *latest = vec![record.run.clone()];
        Ok(record.run)
    }

    pub async fn store_records(
        &self,
        records: Vec<RecommendationRecord>,
    ) -> anyhow::Result<Vec<RecommendationRunDto>> {
        for record in &records {
            self.ledger.insert_record(record)?;
        }
        let runs = records
            .into_iter()
            .map(|record| record.run)
            .collect::<Vec<_>>();
        let mut latest = self
            .latest
            .write()
            .map_err(|_| anyhow!("failed to update latest recommendation state"))?;
        *latest = runs.clone();
        Ok(runs)
    }
}

fn seed_latest_recommendation() -> RecommendationRunDto {
    RecommendationRunDto {
        recommendation_id: "rec-20260503-1758".into(),
        status: "completed".into(),
        trigger_type: "seed".into(),
        has_trade: true,
        symbol: Some("BTC/USDT".into()),
        stock_name: Some("BTC".into()),
        direction: Some("Long".into()),
        market_type: "perpetual".into(),
        exchanges: vec!["人民币现金".into(), "akshare".into(), "A股缓存".into()],
        confidence_score: 78.0,
        rationale:
            "Spot-led breakout with contained basis and positive breadth across three venues."
                .into(),
        symbol_recommendations: Vec::new(),
        risk_status: "approved".into(),
        entry_low: Some(68_280.0),
        entry_high: Some(68_420.0),
        stop_loss: Some(67_680.0),
        take_profit: Some("69,550 / 70,120".into()),
        leverage: Some(3.0),
        amount_cny: Some(1_800.0),
        invalidation: Some(
            "If basis widens while spot loses 67,680, the breakout thesis is invalid.".into(),
        ),
        max_loss_cny: Some(47.4),
        no_trade_reason: None,
        risk_details: RiskDecisionDto {
            status: "approved".into(),
            risk_score: 42,
            max_loss_estimate: Some("0.47%".into()),
            checks: Vec::new(),
            modifications: Vec::new(),
            block_reasons: Vec::new(),
        },
        data_snapshot_at: "2026-05-03T17:58:00+08:00".into(),
        model_provider: "OpenAI-compatible".into(),
        model_name: "gpt-5.5".into(),
        prompt_version: "recommendation-system-v2".into(),
        user_preference_version: "prefs-seed".into(),
        generated_at: "2026-05-03T17:58:00+08:00".into(),
    }
}

fn idle_generation_progress() -> RecommendationGenerationProgressDto {
    RecommendationGenerationProgressDto {
        status: "idle".into(),
        completed_count: 0,
        total_count: 0,
        message: "尚未开始 AI 建议生成".into(),
        items: Vec::new(),
    }
}

const EVALUATION_HORIZONS: [(&str, i64); 6] = [
    ("5m", 5 * 60_000),
    ("10m", 10 * 60_000),
    ("30m", 30 * 60_000),
    ("60m", 60 * 60_000),
    ("24h", 24 * 60 * 60_000),
    ("7d", 7 * 24 * 60 * 60_000),
];

fn unique_temp_default_recommendation_db_path(label: &str) -> PathBuf {
    let millis = (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i128;
    std::env::temp_dir().join(format!("kittyalpha-{label}-{millis}.sqlite3"))
}

pub(crate) fn default_risk_result_json(run: &RecommendationRunDto) -> String {
    serde_json::to_string(&run.risk_details).unwrap_or_else(|_| {
        serde_json::json!({
            "status": run.risk_status,
            "risk_score": 0,
            "max_loss_estimate": run.max_loss_cny.map(|value| format!("{value:.2} CNY")),
            "checks": [],
            "modifications": [],
            "block_reasons": if run.rationale.trim().is_empty() {
                Vec::<String>::new()
            } else {
                vec![run.rationale.clone()]
            },
        })
        .to_string()
    })
}

fn history_row_from_persisted(
    record: &PersistedRecommendation,
    evaluations: &[RecommendationEvaluation],
) -> RecommendationHistoryRowDto {
    let mut row = append_pending_history_row(&record.run);
    row.shortlist = shortlist_symbols_from_market_snapshot(&record.market_snapshot);
    row.executed = record.executed;
    row.modified = record.modified;

    if !record.run.has_trade {
        return row;
    }

    let mut completed = Vec::new();
    for evaluation in evaluations {
        match evaluation.horizon.as_str() {
            "5m" => {
                row.pnl_5m = round_percent(evaluation.estimated_pnl_percent);
                completed.push("5m");
            }
            "10m" => {
                row.pnl_10m = round_percent(evaluation.estimated_pnl_percent);
                completed.push("10m");
            }
            "30m" => {
                row.pnl_30m = round_percent(evaluation.estimated_pnl_percent);
                completed.push("30m");
            }
            "60m" => {
                row.pnl_60m = round_percent(evaluation.estimated_pnl_percent);
                completed.push("60m");
            }
            "24h" => {
                row.pnl_24h = round_percent(evaluation.estimated_pnl_percent);
                completed.push("24h");
            }
            "7d" => {
                row.pnl_7d = round_percent(evaluation.estimated_pnl_percent);
                completed.push("7d");
            }
            _ => {}
        }
    }

    row.result = derive_history_result(&record.run, evaluations);

    let completed_set = completed.into_iter().collect::<HashSet<_>>();
    let pending = EVALUATION_HORIZONS
        .iter()
        .filter_map(|(label, _)| (!completed_set.contains(label)).then_some(*label))
        .collect::<Vec<_>>();

    row.outcome = if completed_set.is_empty() {
        "等待下一交易K线：10分钟、60分钟、24小时、7天。".into()
    } else if pending.is_empty() {
        "已使用持久化记录完成 7 天评估。".into()
    } else {
        let last_completed = EVALUATION_HORIZONS
            .iter()
            .map(|(label, _)| *label)
            .filter(|label| completed_set.contains(label))
            .last()
            .unwrap_or("5m");
        format!(
            "已评估至{}，等待下一交易K线：{}。",
            evaluation_horizon_label(last_completed),
            pending
                .iter()
                .map(|label| evaluation_horizon_label(label))
                .collect::<Vec<_>>()
                .join("、")
        )
    };

    row
}

async fn refresh_missing_evaluations(
    run: &RecommendationRunDto,
    existing: &[RecommendationEvaluation],
    market_data_service: &MarketDataService,
    _enabled_exchanges: &[String],
) -> anyhow::Result<Vec<RecommendationEvaluation>> {
    if !run.has_trade {
        return Ok(Vec::new());
    }
    if missing_evaluation_horizons(existing).is_empty() {
        return Ok(Vec::new());
    }

    let symbol = run
        .symbol
        .as_deref()
        .ok_or_else(|| anyhow!("recommendation symbol is missing for evaluation"))?;
    let fetches = ["5m", "1h", "1d"].into_iter().map(|interval| {
        let symbol = symbol.to_string();
        async move {
            let interval_for_fetch = interval.to_string();
            let result = tokio::task::spawn_blocking(move || {
                crate::market::akshare::fetch_history_bars(&symbol, &interval_for_fetch, 240)
            })
            .await
            .ok()
            .and_then(Result::ok);
            (interval.to_string(), result)
        }
    });
    let mut bar_sets = EvaluationBarSets::default();
    for (interval, fetched) in join_all(fetches).await {
        let bars = if let Some(fetched) = fetched {
            market_data_service.cache_candle_bars(symbol, &interval, &fetched)?;
            fetched
        } else {
            market_data_service.cached_candle_bars(symbol, &interval, 240)
        };
        match interval.as_str() {
            "5m" => bar_sets.intraday = normalize_evaluation_bars(bars),
            "1h" => bar_sets.hourly = normalize_evaluation_bars(bars),
            "1d" => bar_sets.daily = normalize_evaluation_bars(bars),
            _ => {}
        }
    }

    let existing_horizons = existing
        .iter()
        .map(|evaluation| evaluation.horizon.as_str())
        .collect::<HashSet<_>>();

    build_missing_evaluations_from_bar_sets(
        run,
        &bar_sets,
        current_utc_millis(),
        &existing_horizons,
    )
}

fn evaluation_horizon_label(label: &str) -> &'static str {
    match label {
        "5m" => "5分钟",
        "10m" => "10分钟",
        "30m" => "30分钟",
        "60m" => "60分钟",
        "24h" => "24小时",
        "7d" => "7天",
        _ => "未知周期",
    }
}

fn missing_evaluation_horizons(existing: &[RecommendationEvaluation]) -> Vec<&'static str> {
    let existing_horizons = existing
        .iter()
        .map(|evaluation| evaluation.horizon.as_str())
        .collect::<HashSet<_>>();
    EVALUATION_HORIZONS
        .iter()
        .filter_map(|(label, _)| (!existing_horizons.contains(label)).then_some(*label))
        .collect()
}

fn build_missing_evaluations(
    run: &RecommendationRunDto,
    bars: &[OhlcvBar],
    now_ms: i64,
    existing_horizons: &HashSet<&str>,
) -> anyhow::Result<Vec<RecommendationEvaluation>> {
    let bar_sets = EvaluationBarSets {
        intraday: bars.to_vec(),
        hourly: bars.to_vec(),
        daily: bars.to_vec(),
    };
    build_missing_evaluations_from_bar_sets(run, &bar_sets, now_ms, existing_horizons)
}

#[derive(Default)]
struct EvaluationBarSets {
    intraday: Vec<OhlcvBar>,
    hourly: Vec<OhlcvBar>,
    daily: Vec<OhlcvBar>,
}

fn build_missing_evaluations_from_bar_sets(
    run: &RecommendationRunDto,
    bar_sets: &EvaluationBarSets,
    now_ms: i64,
    existing_horizons: &HashSet<&str>,
) -> anyhow::Result<Vec<RecommendationEvaluation>> {
    let mut evaluations = Vec::new();

    for (label, horizon_ms) in EVALUATION_HORIZONS {
        if existing_horizons.contains(label) {
            continue;
        }

        for bars in evaluation_bar_candidates(label, bar_sets) {
            if let Some(evaluation) = build_evaluation(run, bars, now_ms, label, horizon_ms)? {
                evaluations.push(evaluation);
                break;
            }
        }
    }

    Ok(evaluations)
}

fn evaluation_bar_candidates<'a>(
    label: &str,
    bar_sets: &'a EvaluationBarSets,
) -> Vec<&'a [OhlcvBar]> {
    match label {
        "5m" | "10m" | "30m" => vec![bar_sets.intraday.as_slice()],
        "60m" => vec![bar_sets.intraday.as_slice(), bar_sets.hourly.as_slice()],
        "24h" => vec![bar_sets.hourly.as_slice(), bar_sets.daily.as_slice()],
        "7d" => vec![bar_sets.daily.as_slice()],
        _ => Vec::new(),
    }
}

fn normalize_evaluation_bars(mut bars: Vec<OhlcvBar>) -> Vec<OhlcvBar> {
    bars.sort_by_key(|bar| parse_bar_open_millis(&bar.open_time).unwrap_or(i64::MAX));
    bars.dedup_by(|left, right| left.open_time == right.open_time);
    bars
}

fn build_evaluation(
    run: &RecommendationRunDto,
    bars: &[OhlcvBar],
    now_ms: i64,
    horizon_label: &str,
    horizon_ms: i64,
) -> anyhow::Result<Option<RecommendationEvaluation>> {
    let generated_at_ms = parse_rfc3339_millis(&run.generated_at)?;
    let target_ms = match generated_at_ms.checked_add(horizon_ms) {
        Some(value) => value,
        None => return Ok(None),
    };
    if now_ms < target_ms {
        return Ok(None);
    }

    let entry_price = average_entry_price(run.entry_low, run.entry_high)
        .ok_or_else(|| anyhow!("recommendation entry range is missing for evaluation"))?;
    let direction = normalized_trade_direction(run);
    let exit_bar = match bars.iter().find(|bar| {
        parse_bar_open_millis(&bar.open_time).is_some_and(|open_time| open_time >= target_ms)
    }) {
        Some(bar) => bar,
        None => return Ok(None),
    };
    let window = bars
        .iter()
        .filter(|bar| {
            parse_bar_open_millis(&bar.open_time)
                .is_some_and(|open_time| open_time >= generated_at_ms && open_time <= target_ms)
        })
        .collect::<Vec<_>>();
    let max_favorable_price = favorable_price(direction, &window).unwrap_or(exit_bar.close);
    let max_adverse_price = adverse_price(direction, &window).unwrap_or(exit_bar.close);
    let take_profit_targets = parse_take_profit_targets(run.take_profit.as_deref());
    let take_profit_hit = take_profit_targets
        .iter()
        .any(|target| target_hit(direction, &window, *target));
    let stop_loss_hit = run
        .stop_loss
        .map(|stop| target_hit(direction, &window, stop))
        .unwrap_or(false);
    let estimated_pnl_percent =
        estimate_pnl_percent(entry_price, exit_bar.close, direction, 0.1) * 100.0;
    let notional = run.amount_cny.unwrap_or_default() * run.leverage.unwrap_or(1.0).max(1.0);
    let estimated_fee = (notional * 0.001 * 100.0).round() / 100.0;
    let estimated_pnl = (notional * (estimated_pnl_percent / 100.0) * 100.0).round() / 100.0;
    let result = if stop_loss_hit {
        "stop_loss_hit"
    } else if take_profit_hit {
        "take_profit_hit"
    } else if estimated_pnl_percent > 0.0 {
        "profit"
    } else if estimated_pnl_percent < 0.0 {
        "loss"
    } else {
        "flat"
    };

    Ok(Some(RecommendationEvaluation {
        evaluation_id: format!("eval-{}-{horizon_label}", run.recommendation_id),
        recommendation_id: run.recommendation_id.clone(),
        horizon: horizon_label.into(),
        price_at_horizon: exit_bar.close,
        max_favorable_price,
        max_adverse_price,
        take_profit_hit,
        stop_loss_hit,
        estimated_fee,
        estimated_slippage: 0.0,
        funding_fee: 0.0,
        estimated_pnl,
        estimated_pnl_percent,
        result: result.into(),
        evaluated_at: current_rfc3339_timestamp().unwrap_or_else(|| run.generated_at.clone()),
    }))
}

fn favorable_price(direction: &str, bars: &[&OhlcvBar]) -> Option<f64> {
    match direction {
        "short" => bars
            .iter()
            .map(|bar| bar.low)
            .min_by(|left, right| left.total_cmp(right)),
        _ => bars
            .iter()
            .map(|bar| bar.high)
            .max_by(|left, right| left.total_cmp(right)),
    }
}

fn adverse_price(direction: &str, bars: &[&OhlcvBar]) -> Option<f64> {
    match direction {
        "short" => bars
            .iter()
            .map(|bar| bar.high)
            .max_by(|left, right| left.total_cmp(right)),
        _ => bars
            .iter()
            .map(|bar| bar.low)
            .min_by(|left, right| left.total_cmp(right)),
    }
}

fn target_hit(direction: &str, bars: &[&OhlcvBar], target: f64) -> bool {
    match direction {
        "short" => bars.iter().any(|bar| bar.low <= target),
        _ => bars.iter().any(|bar| bar.high >= target),
    }
}

fn parse_take_profit_targets(value: Option<&str>) -> Vec<f64> {
    value
        .unwrap_or_default()
        .split('/')
        .flat_map(|segment| segment.split(','))
        .filter_map(|segment| segment.trim().replace(',', "").parse::<f64>().ok())
        .collect()
}

async fn refresh_history_entry(
    run: &RecommendationRunDto,
    row: &RecommendationHistoryRowDto,
    market_data_service: &MarketDataService,
    enabled_exchanges: &[String],
) -> anyhow::Result<RecommendationHistoryRowDto> {
    if !run.has_trade {
        return Ok(row.clone());
    }

    let symbol = run
        .symbol
        .as_deref()
        .ok_or_else(|| anyhow!("recommendation symbol is missing for evaluation"))?;
    let intraday = market_data_service
        .get_pair_candles_for_exchanges(symbol, &run.market_type, "5m", None, enabled_exchanges)
        .await?;
    let hourly = market_data_service
        .get_pair_candles_for_exchanges(symbol, &run.market_type, "1H", None, enabled_exchanges)
        .await?;
    let mut bars = intraday.bars;
    bars.extend(hourly.bars);
    bars.sort_by(|left, right| left.open_time.cmp(&right.open_time));
    bars.dedup_by(|left, right| left.open_time == right.open_time);

    refresh_history_row_from_bars(run, row, &bars, current_utc_millis())
}

fn refresh_history_row_from_bars(
    run: &RecommendationRunDto,
    row: &RecommendationHistoryRowDto,
    bars: &[OhlcvBar],
    now_ms: i64,
) -> anyhow::Result<RecommendationHistoryRowDto> {
    if !run.has_trade {
        return Ok(row.clone());
    }

    let generated_at_ms = parse_rfc3339_millis(&run.generated_at)?;
    let entry_price = average_entry_price(run.entry_low, run.entry_high)
        .ok_or_else(|| anyhow!("recommendation entry range is missing for evaluation"))?;
    let direction = normalized_trade_direction(run);
    let mut updated = row.clone();

    let pnl_5m = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        5 * 60_000,
        entry_price,
        direction,
    );
    let pnl_10m = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        10 * 60_000,
        entry_price,
        direction,
    );
    let pnl_30m = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        30 * 60_000,
        entry_price,
        direction,
    );
    let pnl_60m = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        60 * 60_000,
        entry_price,
        direction,
    );
    let pnl_24h = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        24 * 60 * 60_000,
        entry_price,
        direction,
    );
    let pnl_7d = evaluate_window(
        bars,
        generated_at_ms,
        now_ms,
        7 * 24 * 60 * 60_000,
        entry_price,
        direction,
    );

    if let Some(value) = pnl_5m {
        updated.pnl_5m = round_percent(value);
    }
    if let Some(value) = pnl_10m {
        updated.pnl_10m = round_percent(value);
    }
    if let Some(value) = pnl_30m {
        updated.pnl_30m = round_percent(value);
    }
    if let Some(value) = pnl_60m {
        updated.pnl_60m = round_percent(value);
    }
    if let Some(value) = pnl_24h {
        updated.pnl_24h = round_percent(value);
    }
    if let Some(value) = pnl_7d {
        updated.pnl_7d = round_percent(value);
    }

    let completed = [
        ("10m", pnl_10m.is_some()),
        ("60m", pnl_60m.is_some()),
        ("24h", pnl_24h.is_some()),
        ("7d", pnl_7d.is_some()),
    ]
    .into_iter()
    .filter_map(|(label, done)| done.then_some(label))
    .collect::<Vec<_>>();
    let pending = [
        ("10m", pnl_10m.is_none()),
        ("60m", pnl_60m.is_none()),
        ("24h", pnl_24h.is_none()),
        ("7d", pnl_7d.is_none()),
    ]
    .into_iter()
    .filter_map(|(label, todo)| todo.then_some(label))
    .collect::<Vec<_>>();

    updated.outcome = if completed.is_empty() {
        "等待下一交易K线：10分钟、60分钟、24小时、7天。".into()
    } else if pending.is_empty() {
        "已使用行情 K 线完成 7 天评估。".into()
    } else {
        format!(
            "已评估至{}，等待下一交易K线：{}。",
            evaluation_horizon_label(completed.last().copied().unwrap_or("5m")),
            pending
                .iter()
                .map(|label| evaluation_horizon_label(label))
                .collect::<Vec<_>>()
                .join("、")
        )
    };

    Ok(updated)
}

fn evaluate_window(
    bars: &[OhlcvBar],
    generated_at_ms: i64,
    now_ms: i64,
    horizon_ms: i64,
    entry_price: f64,
    direction: &str,
) -> Option<f64> {
    let target_ms = generated_at_ms.checked_add(horizon_ms)?;
    if now_ms < target_ms {
        return None;
    }

    let exit_price = bars.iter().find_map(|bar| {
        let open_time = parse_bar_open_millis(&bar.open_time)?;
        (open_time >= target_ms).then_some(bar.close)
    })?;

    Some(estimate_pnl_percent(entry_price, exit_price, direction, 0.1) * 100.0)
}

fn normalized_trade_direction(run: &RecommendationRunDto) -> &'static str {
    match run.direction.as_deref() {
        Some(value) if value.eq_ignore_ascii_case("short") => "short",
        _ if run.market_type == "spot" => "spot_buy",
        _ => "long",
    }
}

fn parse_rfc3339_millis(value: &str) -> anyhow::Result<i64> {
    let parsed = OffsetDateTime::parse(value, &Rfc3339)
        .with_context(|| format!("failed to parse generated_at timestamp: {value}"))?;
    Ok((parsed.unix_timestamp_nanos() / 1_000_000) as i64)
}

fn parse_bar_open_millis(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if let Ok(millis) = trimmed.parse::<i64>() {
        return Some(millis);
    }
    if let Ok(parsed) = OffsetDateTime::parse(trimmed, &Rfc3339) {
        return Some((parsed.unix_timestamp_nanos() / 1_000_000) as i64);
    }

    let normalized = trimmed.replace('T', " ");
    let offset = UtcOffset::from_hms(8, 0, 0).ok()?;
    for pattern in [
        "[year]-[month]-[day] [hour]:[minute]:[second]",
        "[year]-[month]-[day] [hour]:[minute]",
    ] {
        let format = parse_time_format(pattern).ok()?;
        if let Ok(parsed) = PrimitiveDateTime::parse(&normalized, &format) {
            return Some((parsed.assume_offset(offset).unix_timestamp_nanos() / 1_000_000) as i64);
        }
    }

    let date_format = parse_time_format("[year]-[month]-[day]").ok()?;
    Date::parse(&normalized, &date_format).ok().map(|date| {
        let close_time = Time::from_hms(15, 0, 0).unwrap_or(Time::MIDNIGHT);
        (PrimitiveDateTime::new(date, close_time)
            .assume_offset(offset)
            .unix_timestamp_nanos()
            / 1_000_000) as i64
    })
}

fn current_utc_millis() -> i64 {
    (OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

fn select_focus_market<'a>(
    rows: &'a [MarketListRow],
    settings: &RiskSettings,
    runtime: &RuntimeSettingsDto,
    symbol: Option<&str>,
) -> Option<&'a MarketListRow> {
    let mut filtered = rows
        .iter()
        .filter(|row| symbol.map(|target| row.symbol == target).unwrap_or(true))
        .filter(|row| scan_scope_allows(row, runtime, symbol))
        .filter(|row| scan_candidate_allowed(row, settings))
        .collect::<Vec<_>>();

    filtered
        .sort_by(|left, right| recommendation_score(right).total_cmp(&recommendation_score(left)));

    filtered
        .iter()
        .copied()
        .find(|row| row.market_type == "perpetual")
        .or_else(|| filtered.first().copied())
}

fn scan_scope_allows(
    row: &MarketListRow,
    runtime: &RuntimeSettingsDto,
    symbol: Option<&str>,
) -> bool {
    if symbol.is_some() {
        return true;
    }

    if runtime.scan_scope != "watchlist_only" {
        return true;
    }

    symbol_is_listed(&row.symbol, &runtime.watchlist_symbols)
}

fn no_trade_reason_for_scan_scope(runtime: &RuntimeSettingsDto, symbol: Option<&str>) -> String {
    if symbol.is_some() || runtime.scan_scope != "watchlist_only" {
        return "Current risk profile does not allow any executable candidates in the scan universe."
            .into();
    }

    if runtime.watchlist_symbols.is_empty() {
        "Watchlist-only scan is enabled, but the watchlist is empty.".into()
    } else {
        "Watchlist-only scan is enabled, but none of the watchlist symbols produced an executable candidate.".into()
    }
}

fn recommendation_score(row: &MarketListRow) -> f64 {
    let market_bonus = if row.market_type == "perpetual" {
        0.75
    } else {
        0.25
    };
    let funding_signal = row.funding_rate.unwrap_or_default().abs() * 100.0;
    let freshness_penalty = if row.stale { -2.0 } else { 0.0 };

    row.change_24h.abs() * 1.4
        + row.spread_bps * 0.05
        + funding_signal
        + market_bonus
        + freshness_penalty
}

fn build_trade_plan(row: &MarketListRow, settings: &RiskSettings) -> RecommendationRunDto {
    let generated_at = current_rfc3339_timestamp().unwrap_or_else(|| row.updated_at.clone());
    if row.stale {
        return build_no_trade(row, "Live venue coverage is partial right now, so the system is withholding an executable plan.");
    }

    if row.volume_24h < settings.min_volume_24h {
        return build_no_trade(
            row,
            "Liquidity is too thin across the responding venues for a clean executable setup.",
        );
    }

    if row.change_24h.abs() < 1.0 && row.spread_bps < 1.5 {
        return build_no_trade(
            row,
            "Momentum and single-venue spread are both too small to justify a trade.",
        );
    }

    let direction = recommended_direction(row);
    let entry_buffer = (row.spread_bps.max(1.0) / 10_000.0).clamp(0.0010, 0.0035);
    let leverage = if row.market_type == "perpetual" {
        if row.change_24h.abs() >= 4.0 {
            2.0
        } else {
            3.0
        }
    } else {
        1.0
    };
    let base_amount_cny = match row.symbol.as_str() {
        "BTC/USDT" => 1_800.0,
        "ETH/USDT" => 1_500.0,
        _ => 1_200.0,
    };
    let (entry_low, entry_high) = if direction == "Long" {
        (
            row.last_price * (1.0 - entry_buffer),
            row.last_price * (1.0 - entry_buffer * 0.25),
        )
    } else {
        (
            row.last_price * (1.0 + entry_buffer * 0.25),
            row.last_price * (1.0 + entry_buffer),
        )
    };
    let risk_buffer = (row.change_24h.abs() / 100.0).clamp(0.010, 0.024);
    let stop_loss = if direction == "Long" {
        entry_low * (1.0 - risk_buffer)
    } else {
        entry_high * (1.0 + risk_buffer)
    };
    let entry_mid = (entry_low + entry_high) / 2.0;
    let stop_distance = (entry_mid - stop_loss).abs() / entry_mid.max(1.0);
    let risk_budget_usdt =
        settings.account_equity_usdt * (settings.max_loss_per_trade_percent / 100.0);
    let risk_capped_amount_cny = if leverage > 0.0 && stop_distance > 0.0 {
        risk_budget_usdt / (leverage * stop_distance)
    } else {
        base_amount_cny
    };
    let amount_cny = base_amount_cny.min(risk_capped_amount_cny.max(0.0));
    if amount_cny <= 0.0 {
        return build_no_trade(
            row,
            "Risk budget for this account mode is too small for the current stop distance.",
        );
    }
    let take_profit_1 = if direction == "Long" {
        entry_high * (1.0 + risk_buffer * 1.8)
    } else {
        entry_low * (1.0 - risk_buffer * 1.8)
    };
    let take_profit_2 = if direction == "Long" {
        entry_high * (1.0 + risk_buffer * 3.0)
    } else {
        entry_low * (1.0 - risk_buffer * 3.0)
    };
    let confidence_score = recommended_confidence_score(row);
    let risk_tags = coin_info::get_coin_info(&row.symbol, &row.exchanges).risk_tags;

    let plan = CandidatePlan {
        symbol: row.symbol.clone(),
        market_type: row.market_type.clone(),
        direction: direction.to_lowercase(),
        leverage,
        stop_loss: Some(stop_loss),
        entry_low: Some(entry_low),
        entry_high: Some(entry_high),
        take_profit_targets: vec![take_profit_1, take_profit_2],
        amount_cny: Some(amount_cny),
        volume_24h: row.volume_24h,
        spread_bps: row.spread_bps,
        confidence_score,
        risk_tags,
    };
    let risk = evaluate_plan(&plan, settings);
    if risk.status != "approved" {
        return build_blocked_no_trade(
            row,
            &format!(
                "A raw setup was detected for {} but the risk gate blocked it: {}.",
                row.symbol,
                risk.primary_reason()
                    .unwrap_or_else(|| "unknown_risk_reason".into())
            ),
            &risk,
        );
    }

    let max_loss_cny = amount_cny * leverage * stop_distance;
    let venue_text = row.exchanges.join(", ");
    let funding_text = row
        .funding_rate
        .map(|value| format!("{value:.3}%"))
        .unwrap_or_else(|| "N/A".into());

    RecommendationRunDto {
        recommendation_id: format!("rec-{}-{}", row.symbol.replace('/', "-").to_lowercase(), row.updated_at),
        status: "completed".into(),
        trigger_type: "manual".into(),
        has_trade: true,
        symbol: Some(row.symbol.clone()),
        stock_name: Some(row.base_asset.clone()),
        direction: Some(direction.into()),
        market_type: row.market_type.clone(),
        exchanges: row.exchanges.clone(),
        confidence_score,
        rationale: format!(
            "{} {} is showing {:.2}% 24h movement, {:.1} bps of single-venue spread, and funding {} on {}.",
            row.symbol, direction, row.change_24h, row.spread_bps, funding_text, venue_text
        ),
        symbol_recommendations: Vec::new(),
        risk_status: "approved".into(),
        entry_low: Some(round_price(entry_low)),
        entry_high: Some(round_price(entry_high)),
        stop_loss: Some(round_price(stop_loss)),
        take_profit: Some(format!(
            "{} / {}",
            pretty_price(take_profit_1),
            pretty_price(take_profit_2)
        )),
        leverage: Some(leverage),
        amount_cny: Some(amount_cny),
        invalidation: Some(if direction == "Long" {
            format!(
                "If the perpetual contract loses {}, the long setup is invalid.",
                pretty_price(stop_loss)
            )
        } else {
            format!(
                "If price reclaims {} while funding cools, the short setup is invalid.",
                pretty_price(stop_loss)
            )
        }),
        max_loss_cny: Some((max_loss_cny * 100.0).round() / 100.0),
        no_trade_reason: None,
        risk_details: risk.to_decision_dto(),
        data_snapshot_at: row.updated_at.clone(),
        model_provider: "System".into(),
        model_name: "heuristic-fallback".into(),
        prompt_version: "recommendation-system-v2".into(),
        user_preference_version: "prefs-manual".into(),
        generated_at,
    }
}

fn scan_candidate_allowed(row: &MarketListRow, settings: &RiskSettings) -> bool {
    if settings.allowed_markets == "spot" && row.market_type != "spot" {
        return false;
    }

    if settings.allowed_markets == "perpetual" && row.market_type != "perpetual" {
        return false;
    }

    if symbol_is_listed(&row.symbol, &settings.blacklist_symbols) {
        return false;
    }

    if !settings.whitelist_symbols.is_empty()
        && !symbol_is_listed(&row.symbol, &settings.whitelist_symbols)
    {
        return false;
    }

    if settings.allowed_direction == "observe_only" {
        return false;
    }

    if settings.allowed_direction == "long_only" && recommended_direction(row) == "Short" {
        return false;
    }

    if recommended_confidence_score(row) < settings.min_confidence_score {
        return false;
    }

    if !settings.allow_meme_coins {
        let coin = coin_info::get_coin_info(&row.symbol, &row.exchanges);
        if coin
            .risk_tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("meme"))
        {
            return false;
        }
    }

    true
}

fn recommended_direction(row: &MarketListRow) -> &'static str {
    if row.change_24h >= 0.0 {
        "Long"
    } else {
        "Short"
    }
}

fn recommended_confidence_score(row: &MarketListRow) -> f64 {
    (62.0 + row.change_24h.abs() * 4.0 + row.spread_bps * 0.35).clamp(55.0, 89.0)
}

fn symbol_is_listed(symbol: &str, list: &[String]) -> bool {
    list.iter().any(|item| item.eq_ignore_ascii_case(symbol))
}

fn build_no_trade(row: &MarketListRow, reason: &str) -> RecommendationRunDto {
    build_no_trade_with_risk(
        row,
        reason,
        "watch",
        RiskDecisionDto {
            status: "watch".into(),
            risk_score: 0,
            max_loss_estimate: None,
            checks: Vec::new(),
            modifications: Vec::new(),
            block_reasons: Vec::new(),
        },
    )
}

fn build_blocked_no_trade(
    row: &MarketListRow,
    reason: &str,
    risk: &risk_engine::RiskResult,
) -> RecommendationRunDto {
    build_no_trade_with_risk(row, reason, "blocked", risk.to_decision_dto())
}

fn build_no_trade_with_risk(
    row: &MarketListRow,
    reason: &str,
    risk_status: &str,
    risk_details: RiskDecisionDto,
) -> RecommendationRunDto {
    let generated_at = current_rfc3339_timestamp().unwrap_or_else(|| row.updated_at.clone());
    RecommendationRunDto {
        recommendation_id: format!(
            "rec-watch-{}-{}",
            row.symbol.replace('/', "-").to_lowercase(),
            row.updated_at
        ),
        status: "completed".into(),
        trigger_type: "manual".into(),
        has_trade: false,
        symbol: Some(row.symbol.clone()),
        stock_name: Some(row.base_asset.clone()),
        direction: None,
        market_type: row.market_type.clone(),
        exchanges: row.exchanges.clone(),
        confidence_score: 42.0,
        rationale: format!(
            "LLM scan is watching {} {} but did not publish an executable plan.",
            row.symbol, row.market_type
        ),
        symbol_recommendations: Vec::new(),
        risk_status: risk_status.into(),
        entry_low: None,
        entry_high: None,
        stop_loss: None,
        take_profit: None,
        leverage: None,
        amount_cny: None,
        invalidation: None,
        max_loss_cny: None,
        no_trade_reason: Some(reason.into()),
        risk_details,
        data_snapshot_at: row.updated_at.clone(),
        model_provider: "System".into(),
        model_name: "heuristic-fallback".into(),
        prompt_version: "recommendation-system-v2".into(),
        user_preference_version: "prefs-manual".into(),
        generated_at,
    }
}

fn append_pending_history_row(run: &RecommendationRunDto) -> RecommendationHistoryRowDto {
    RecommendationHistoryRowDto {
        recommendation_id: run.recommendation_id.clone(),
        created_at: run.generated_at.clone(),
        trigger_type: run.trigger_type.clone(),
        symbol: run
            .symbol
            .clone()
            .unwrap_or_else(|| "No Recommendation".into()),
        stock_name: run.stock_name.clone().unwrap_or_else(|| "未知".into()),
        shortlist: Vec::new(),
        exchange: run
            .exchanges
            .first()
            .cloned()
            .unwrap_or_else(|| "N/A".into()),
        market_type: run.market_type.clone(),
        direction: run.direction.clone().unwrap_or_else(|| "No Trade".into()),
        rationale: run.rationale.clone(),
        risk_status: run.risk_status.clone(),
        result: derive_history_result(run, &[]),
        entry_low: run.entry_low,
        entry_high: run.entry_high,
        stop_loss: run.stop_loss,
        take_profit: run.take_profit.clone(),
        leverage: run.leverage,
        amount_cny: run.amount_cny,
        confidence_score: run.confidence_score,
        model_name: run.model_name.clone(),
        prompt_version: run.prompt_version.clone(),
        executed: false,
        modified: false,
        pnl_5m: 0.0,
        pnl_10m: 0.0,
        pnl_30m: 0.0,
        pnl_60m: 0.0,
        pnl_24h: 0.0,
        pnl_7d: 0.0,
        outcome: if run.has_trade {
            "等待下一交易K线：10分钟、60分钟、24小时、7天。".into()
        } else {
            run.rationale.clone()
        },
    }
}

fn derive_history_result(
    run: &RecommendationRunDto,
    evaluations: &[RecommendationEvaluation],
) -> String {
    if !run.has_trade {
        return match run.risk_status.as_str() {
            "blocked" | "failed" => "Blocked".into(),
            _ => "No Trade".into(),
        };
    }

    let Some(evaluation_24h) = evaluations
        .iter()
        .find(|evaluation| evaluation.horizon == "24h")
    else {
        return "Pending".into();
    };

    if evaluation_24h.estimated_pnl_percent > 0.0 {
        "Win".into()
    } else if evaluation_24h.estimated_pnl_percent < 0.0 {
        "Loss".into()
    } else {
        "Flat".into()
    }
}

fn shortlist_symbols_from_market_snapshot(snapshot: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(snapshot) else {
        return Vec::new();
    };

    shortlist_symbols_from_snapshot_value(&value)
}

fn shortlist_symbols_from_snapshot_value(value: &serde_json::Value) -> Vec<String> {
    if let Some(shortlist) = value.get("shortlist") {
        let symbols = shortlist_symbols_from_prompt_value(shortlist);
        if !symbols.is_empty() {
            return symbols;
        }
    }

    value
        .get("user_prompt")
        .and_then(serde_json::Value::as_str)
        .and_then(|prompt| serde_json::from_str::<serde_json::Value>(prompt).ok())
        .map(|prompt| shortlist_symbols_from_prompt_value(&prompt))
        .unwrap_or_default()
}

fn shortlist_symbols_from_prompt_value(value: &serde_json::Value) -> Vec<String> {
    let mut seen = HashSet::new();
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
            let normalized = symbol.to_ascii_lowercase();
            if seen.insert(normalized) {
                symbols.push(symbol);
            }
        }
    }

    symbols
}

fn average_entry_price(low: Option<f64>, high: Option<f64>) -> Option<f64> {
    match (low, high) {
        (Some(low), Some(high)) => Some((low + high) / 2.0),
        (Some(low), None) => Some(low),
        (None, Some(high)) => Some(high),
        (None, None) => None,
    }
}

fn round_percent(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn round_price(value: f64) -> f64 {
    if value >= 1_000.0 {
        (value * 100.0).round() / 100.0
    } else if value >= 1.0 {
        (value * 1_000.0).round() / 1_000.0
    } else {
        (value * 10_000.0).round() / 10_000.0
    }
}

fn pretty_price(value: f64) -> String {
    let rounded = round_price(value);
    if rounded >= 1_000.0 {
        format!("{rounded:.2}")
    } else if rounded >= 1.0 {
        format!("{rounded:.3}")
    } else {
        format!("{rounded:.4}")
    }
}

fn current_rfc3339_timestamp() -> Option<String> {
    OffsetDateTime::now_utc().format(&Rfc3339).ok()
}
