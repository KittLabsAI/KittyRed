#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        build_market_prompt, build_system_prompt, endpoint_url, extract_anthropic_content,
        extract_openai_content, new_recommendation_id, normalize_model_recommendation,
        parse_model_recommendation, stock_agent_fetch_plan_labels, PositionContext,
        PromptKlineContext, StockAgentData, FinancialAnalysisPromptContext,
        build_stock_agent_market_prompt,
    };
    use crate::models::{
        default_assistant_system_prompt, MarketListRow, RuntimeNotificationSettingsDto,
        RuntimeSettingsDto,
    };

    #[test]
    fn extracts_openai_compatible_json_content() {
        let body = r#"{
          "choices": [
            {
              "message": {
                "content": "{\"has_trade\":true,\"stock_code\":\"SHSE.600000\",\"direction\":\"买入\",\"confidence_score\":72,\"rationale\":\"量价结构改善。\",\"entry_low\":8.61,\"entry_high\":8.66,\"stop_loss\":8.42,\"take_profit\":\"8.90 / 9.12\",\"amount_cny\":12000,\"invalidation\":\"跌破 8.42。\",\"max_loss_cny\":330}"
              }
            }
          ]
        }"#;

        let content = extract_openai_content(body).expect("response extraction should succeed");
        assert!(content.contains("\"has_trade\":true"));
    }

    #[test]
    fn falls_back_to_openai_message_content_when_anthropic_blocks_have_no_text() {
        let body = r#"{
          "content": [
            {
              "type": "tool_use",
              "id": "toolu_123",
              "name": "noop",
              "input": {}
            }
          ],
          "choices": [
            {
              "message": {
                "content": "{\"has_trade\":false,\"rationale\":\"No setup.\"}"
              }
            }
          ]
        }"#;

        let content = extract_anthropic_content(body).expect("response extraction should succeed");
        assert!(content.contains("\"has_trade\":false"));
    }

    #[test]
    fn extracts_anthropic_legacy_completion_content() {
        let body = r#"{
          "content": [],
          "completion": "{\"has_trade\":false,\"rationale\":\"No setup.\"}"
        }"#;

        let content = extract_anthropic_content(body).expect("response extraction should succeed");
        assert!(content.contains("\"rationale\":\"No setup.\""));
    }

    #[test]
    fn normalizes_trade_payload_into_executable_run() {
        let runtime = runtime_settings();
        let payload = parse_model_recommendation(
            r#"{"has_trade":true,"direction":"买入","confidence_score":72,"rationale":"量价结构改善。","entry_low":8.61,"entry_high":8.66,"stop_loss":8.42,"take_profit":"8.90 / 9.12","amount_cny":12000,"invalidation":"跌破 8.42。","max_loss_cny":330}"#,
        )
        .expect("payload parse should succeed");
        let row = sample_rows()
            .into_iter()
            .find(|row| row.symbol == "SHSE.600000")
            .expect("sample row should exist");

        let run = normalize_model_recommendation(
            &payload,
            &row,
            &sample_rows(),
            &runtime,
            100_000.0,
            None,
        )
        .expect("trade payload should normalize");

        assert!(run.has_trade);
        assert_eq!(run.symbol.as_deref(), Some("SHSE.600000"));
        assert_eq!(run.stock_name.as_deref(), Some("浦发银行"));
        assert_eq!(run.market_type, "ashare");
        assert_eq!(run.direction.as_deref(), Some("买入"));
        assert_eq!(run.leverage, None);
        assert_eq!(run.amount_cny, Some(12_000.0));
        assert_eq!(run.max_loss_cny, Some(330.0));
        let serialized = serde_json::to_string(&run).expect("run should serialize");
        assert!(serialized.contains("amount_cny"));
        assert!(serialized.contains("max_loss_cny"));
        assert!(!serialized.contains("amount_usdt"));
        assert!(!serialized.contains("max_loss_usdt"));
        assert_eq!(run.risk_status, "approved");
        assert_eq!(run.risk_details.status, "approved");
        assert!(run.risk_details.block_reasons.is_empty());
    }

    #[test]
    fn builds_a_share_prompt_with_watchlist_shortlist_and_kline_context() {
        let mut runtime = runtime_settings();
        runtime.watchlist_symbols = vec!["SZSE.000001".into()];
        let mut candles = HashMap::new();
        candles.insert(
            "SHSE.600000".to_string(),
            PromptKlineContext {
                bars: HashMap::from([
                    (
                        "5m".to_string(),
                        vec![["8.61".into(), "8.66".into(), "8.58".into(), "8.63".into()]],
                    ),
                    (
                        "1h".to_string(),
                        vec![["8.50".into(), "8.70".into(), "8.48".into(), "8.63".into()]],
                    ),
                    (
                        "1d".to_string(),
                        vec![["8.20".into(), "8.70".into(), "8.15".into(), "8.63".into()]],
                    ),
                    (
                        "1w".to_string(),
                        vec![["8.00".into(), "8.90".into(), "7.95".into(), "8.63".into()]],
                    ),
                ]),
                messages: HashMap::from([(
                    "1w".to_string(),
                    "网络原因导致 1w K 线读取超时，已返回空值。".to_string(),
                )]),
            },
        );
        let prompt = build_market_prompt(
            &sample_prompt_rows(),
            &runtime,
            Some("SHSE.600000"),
            &candles,
        );
        let value: serde_json::Value =
            serde_json::from_str(&prompt).expect("prompt should be structured JSON");

        assert!(value.get("request_meta").is_some());
        assert!(value.get("market_universe").is_none());
        assert!(value.get("perpetual_shortlist").is_none());
        assert!(value.get("spot_shortlist").is_none());
        assert_eq!(
            value
                .get("request_meta")
                .and_then(|meta| meta.get("market_scope"))
                .and_then(serde_json::Value::as_str),
            Some("a_share_watchlist")
        );
        assert!(value
            .get("a_share_shortlist")
            .and_then(serde_json::Value::as_array)
            .expect("A-share shortlist should be present")
            .iter()
            .any(|row| row.get("ticker_bias").and_then(serde_json::Value::as_str) == Some("偏多")));
        assert!(value
            .get("a_share_shortlist")
            .and_then(serde_json::Value::as_array)
            .expect("A-share shortlist should be present")
            .iter()
            .any(
                |row| row.get("stock_code").and_then(serde_json::Value::as_str)
                    == Some("SZSE.000001")
            ));
        let first_stock = value
            .get("a_share_shortlist")
            .and_then(serde_json::Value::as_array)
            .and_then(|items| items.first())
            .expect("A-share shortlist should include rows");
        assert!(first_stock.get("coin_info").is_none());
        assert!(first_stock.get("orderbook_summary").is_none());
        assert!(first_stock.get("recent_trade_flow_summary").is_none());
        assert!(first_stock.get("kline_data").is_some());
        let kline_data = first_stock
            .get("kline_data")
            .expect("K-line data should exist");
        assert!(kline_data.get("5m").is_some());
        assert!(kline_data.get("1h").is_some());
        assert!(kline_data.get("1d").is_some());
        assert!(kline_data.get("1w").is_some());
        assert!(kline_data.get("messages").is_some());
        assert!(value.get("account_risk_summary").is_some());
        let required_fields = value
            .get("output_contract")
            .and_then(|contract| contract.get("required_fields"))
            .and_then(serde_json::Value::as_array)
            .expect("output contract should be present");
        let required_field_names = required_fields
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            required_field_names,
            vec![
                "has_trade",
                "direction",
                "confidence_score",
                "rationale",
                "entry_low",
                "entry_high",
                "stop_loss",
                "take_profit",
                "amount_cny",
                "invalidation",
                "max_loss_cny"
            ]
        );
        assert!(!prompt.contains("amount_usdt"));
        assert!(!prompt.contains("market_type"));
        assert!(!prompt.contains("perpetual"));
        assert!(!prompt.contains("USDT"));
    }

    #[test]
    fn stock_agent_fetch_plan_requests_profile_bid_ask_and_each_kline_period() {
        let labels = stock_agent_fetch_plan_labels();

        assert_eq!(
            labels,
            vec![
                "stock_info",
                "bid_ask",
                "kline_5m",
                "kline_1h",
                "kline_1d",
                "kline_1w"
            ]
        );
    }

    #[test]
    fn stock_agent_prompt_includes_financial_report_analysis_when_enabled() {
        let mut runtime = runtime_settings();
        runtime.use_financial_report_data = true;
        let mut agent_data = StockAgentData::default();
        agent_data.financial_report_analysis = Some(FinancialAnalysisPromptContext {
            key_summary: "收入稳定增长".into(),
            positive_factors: "经营现金流改善".into(),
            negative_factors: "费用率上升".into(),
            fraud_risk_points: "暂无明显异常".into(),
        });
        let mut rows = sample_rows();
        let row = rows.remove(0);

        let prompt = build_stock_agent_market_prompt(&row, &runtime, &agent_data, None);
        let value: serde_json::Value = serde_json::from_str(&prompt).unwrap();
        let financial = value
            .pointer("/stock_context/financial_report_analysis")
            .expect("financial context should be included");

        assert_eq!(
            financial.get("key_summary").and_then(serde_json::Value::as_str),
            Some("收入稳定增长")
        );
        assert!(prompt.contains("fraud_risk_points"));
        assert!(!prompt.contains("raw_sections"));
    }

    #[test]
    fn stock_agent_prompt_excludes_financial_report_analysis_when_disabled() {
        let mut runtime = runtime_settings();
        runtime.use_financial_report_data = false;
        let mut agent_data = StockAgentData::default();
        agent_data.financial_report_analysis = Some(FinancialAnalysisPromptContext {
            key_summary: "收入稳定增长".into(),
            positive_factors: "经营现金流改善".into(),
            negative_factors: "费用率上升".into(),
            fraud_risk_points: "暂无明显异常".into(),
        });
        let mut rows = sample_rows();
        let row = rows.remove(0);

        let prompt = build_stock_agent_market_prompt(&row, &runtime, &agent_data, None);
        let value: serde_json::Value = serde_json::from_str(&prompt).unwrap();

        assert!(value
            .pointer("/stock_context/financial_report_analysis")
            .is_none());
    }

    #[test]
    fn turns_risk_block_into_no_trade_result() {
        let mut runtime = runtime_settings();
        runtime.min_confidence_score = 80.0;
        let payload = parse_model_recommendation(
            r#"{"has_trade":true,"direction":"买入","confidence_score":72,"rationale":"量价结构改善。","entry_low":8.61,"entry_high":8.66,"stop_loss":8.42,"take_profit":"8.90 / 9.12","amount_cny":12000,"invalidation":"跌破 8.42。","max_loss_cny":330}"#,
        )
        .expect("payload parse should succeed");
        let row = sample_rows()
            .into_iter()
            .find(|row| row.symbol == "SHSE.600000")
            .expect("sample row should exist");

        let run = normalize_model_recommendation(
            &payload,
            &row,
            &sample_rows(),
            &runtime,
            100_000.0,
            None,
        )
        .expect("blocked payload should still normalize");

        assert!(!run.has_trade);
        assert_eq!(run.risk_status, "blocked");
        assert_eq!(run.risk_details.status, "blocked");
        assert!(run
            .no_trade_reason
            .unwrap_or_default()
            .contains("confidence_below_threshold"));
    }

    #[test]
    fn keeps_sell_direction_for_held_stock() {
        let runtime = runtime_settings();
        let payload = parse_model_recommendation(
            r#"{"has_trade":true,"direction":"卖出","confidence_score":72,"rationale":"持仓跌破 1h 支撑，先退出模拟仓位。","entry_low":8.61,"entry_high":8.66,"stop_loss":null,"take_profit":null,"amount_cny":null,"invalidation":"重新站回 8.80。","max_loss_cny":null}"#,
        )
        .expect("payload parse should succeed");
        let row = sample_rows()
            .into_iter()
            .find(|row| row.symbol == "SHSE.600000")
            .expect("sample row should exist");

        let run = normalize_model_recommendation(
            &payload,
            &row,
            &sample_rows(),
            &runtime,
            100_000.0,
            Some(&PositionContext {
                symbol: "SHSE.600000".into(),
                side: "long".into(),
                size: "100股".into(),
                entry_price: 8.9,
                mark_price: 8.63,
                pnl_percent: -3.0,
            }),
        )
        .expect("held stock sell payload should normalize");

        assert!(run.has_trade);
        assert_eq!(run.direction.as_deref(), Some("卖出"));
        assert_eq!(run.amount_cny, None);
        assert_eq!(run.max_loss_cny, None);
        assert_eq!(run.risk_status, "approved");
    }

    #[test]
    fn blocks_sell_direction_without_position() {
        let runtime = runtime_settings();
        let payload = parse_model_recommendation(
            r#"{"has_trade":true,"direction":"卖出","confidence_score":72,"rationale":"没有持仓时不应卖出。","entry_low":8.61,"entry_high":8.66,"stop_loss":null,"take_profit":null,"amount_cny":null,"invalidation":null,"max_loss_cny":null}"#,
        )
        .expect("payload parse should succeed");
        let row = sample_rows()
            .into_iter()
            .find(|row| row.symbol == "SHSE.600000")
            .expect("sample row should exist");

        let run = normalize_model_recommendation(
            &payload,
            &row,
            &sample_rows(),
            &runtime,
            100_000.0,
            None,
        )
        .expect("unheld sell payload should normalize into block");

        assert!(!run.has_trade);
        assert_eq!(run.risk_status, "blocked");
        assert!(run
            .no_trade_reason
            .unwrap_or_default()
            .contains("无持仓股票不能给出卖出建议"));
    }

    #[test]
    fn parses_rationale_only_no_trade_payload() {
        let payload = parse_model_recommendation(
            r#"{"has_trade":false,"rationale":"候选股票成交额不足且 1h 走势未确认。"}"#,
        )
        .expect("minimal no-trade payload should parse");

        assert!(!payload.has_trade);
        assert_eq!(payload.confidence_score, None);
        assert_eq!(payload.rationale, "候选股票成交额不足且 1h 走势未确认。");
    }

    #[test]
    fn parses_no_trade_payload_with_numeric_invalidation() {
        let payload = parse_model_recommendation(
            r#"{"has_trade":false,"direction":"观望","confidence_score":40,"rationale":"价格趋势向下。","entry_low":0,"entry_high":0,"stop_loss":0,"take_profit":0,"amount_cny":0,"invalidation":0,"max_loss_cny":0}"#,
        )
        .expect("numeric invalidation should parse");

        assert!(!payload.has_trade);
        assert_eq!(payload.invalidation.as_deref(), Some("0"));
        assert_eq!(payload.take_profit.as_deref(), Some("0"));
    }

    #[test]
    fn parses_numeric_plan_fields_from_numbers_or_strings() {
        let payload = parse_model_recommendation(
            r#"{"has_trade":true,"direction":"买入","confidence_score":"72","rationale":"量价结构改善。","entry_low":"8.61","entry_high":8.66,"stop_loss":"8.42","take_profit":8.90,"amount_cny":"12000","invalidation":8.42,"max_loss_cny":"330"}"#,
        )
        .expect("numeric plan fields should parse from numbers or strings");

        assert_eq!(payload.confidence_score, Some(72.0));
        assert_eq!(payload.entry_low, Some(8.61));
        assert_eq!(payload.entry_high, Some(8.66));
        assert_eq!(payload.stop_loss, Some(8.42));
        assert_eq!(payload.take_profit.as_deref(), Some("8.9"));
        assert_eq!(payload.amount_cny, Some(12000.0));
        assert_eq!(payload.invalidation.as_deref(), Some("8.42"));
        assert_eq!(payload.max_loss_cny, Some(330.0));
    }

    #[test]
    fn rejects_no_trade_payload_without_rationale() {
        let error =
            parse_model_recommendation(r#"{"has_trade":false,"no_trade_reason":"No setup."}"#)
                .expect_err("rationale is required for no-trade payloads");

        assert!(error
            .to_string()
            .contains("failed to parse model recommendation payload"));
    }

    #[test]
    fn system_prompt_requires_specific_no_trade_rationale() {
        let prompt = build_system_prompt(&runtime_settings());

        assert!(prompt.contains("你是 KittyRed 的沪深 A 股模拟投资助手"));
        assert!(prompt.contains("has_trade=false"));
        assert!(prompt.contains("说明最重要的 2 到 3 个未满足条件"));
        assert!(!prompt.contains("必须包含 stock_code"));
        assert!(!prompt.contains(r#""stock_code":"#));
        assert!(prompt.contains("amount_cny"));
        assert!(!prompt.contains("perpetual"));
        assert!(!prompt.contains("USDT"));
        assert!(!prompt.contains("market_type"));
        assert!(!prompt.contains("no_trade_reason"));
    }

    #[test]
    fn injects_input_symbol_for_no_trade_payload_without_model_symbol() {
        let runtime = runtime_settings();
        let payload = parse_model_recommendation(
            r#"{"has_trade":false,"direction":null,"confidence_score":null,"rationale":"暂无符合风险收益比的 A 股机会。","entry_low":null,"entry_high":null,"stop_loss":null,"take_profit":null,"amount_cny":null,"invalidation":null,"max_loss_cny":null}"#,
        )
        .expect("payload parse should succeed");
        let row = sample_rows()
            .into_iter()
            .find(|row| row.symbol == "SHSE.600000")
            .expect("sample row should exist");

        let run = normalize_model_recommendation(
            &payload,
            &row,
            &sample_rows(),
            &runtime,
            10_000.0,
            None,
        )
        .expect("no-trade payload should normalize");

        assert_eq!(run.symbol.as_deref(), Some("SHSE.600000"));
        assert_eq!(run.stock_name.as_deref(), Some("浦发银行"));
        assert_eq!(run.rationale, "暂无符合风险收益比的 A 股机会。");
        assert_eq!(run.no_trade_reason, None);
    }

    #[test]
    fn recommendation_ids_are_unique_for_same_millisecond_subagents() {
        let first = new_recommendation_id("watch", "SHSE.600000");
        let second = new_recommendation_id("watch", "SZSE.000001");

        assert_ne!(first, second);
        assert!(first.contains("shse-600000"));
        assert!(second.contains("szse-000001"));
    }

    #[test]
    fn adds_v1_prefix_for_anthropic_root_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://api.anthropic.com",
                "https://api.anthropic.com/v1",
                "messages"
            ),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn adds_v1_prefix_for_openai_root_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://api.openai.com",
                "https://api.openai.com/v1",
                "chat/completions"
            ),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn adds_v1_prefix_for_anthropic_gateway_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://token-plan-cn.xiaomimimo.com/anthropic",
                "https://api.anthropic.com/v1",
                "messages"
            ),
            "https://token-plan-cn.xiaomimimo.com/anthropic/v1/messages"
        );
    }

    #[test]
    fn adds_v1_prefix_for_openai_gateway_base_urls() {
        assert_eq!(
            endpoint_url(
                "https://token-plan-cn.xiaomimimo.com/openai",
                "https://api.openai.com/v1",
                "chat/completions"
            ),
            "https://token-plan-cn.xiaomimimo.com/openai/v1/chat/completions"
        );
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
            has_stored_model_api_key: true,
            auto_analyze_enabled: true,
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
            recommendation_system_prompt: String::new(),
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

    fn sample_rows() -> Vec<MarketListRow> {
        vec![MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "浦发银行".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 8.63,
            change_24h: 2.8,
            volume_24h: 180_000_000.0,
            funding_rate: None,
            spread_bps: 3.4,
            exchanges: vec!["akshare:xueqiu".into()],
            updated_at: "2026-05-03T20:20:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]
    }

    fn sample_prompt_rows() -> Vec<MarketListRow> {
        let mut rows = (0..11)
            .map(|index| MarketListRow {
                symbol: format!("SHSE.6000{index:02}"),
                base_asset: format!("样例股票{index}"),
                market_type: "ashare".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: "mid".into(),
                last_price: 8.0 + index as f64 * 0.1,
                change_24h: 10.0 - index as f64 * 0.5,
                volume_24h: 100_000_000.0 - index as f64 * 1_000_000.0,
                funding_rate: None,
                spread_bps: 2.0 + index as f64 * 0.1,
                exchanges: vec!["akshare:xueqiu".into()],
                updated_at: "2026-05-03T20:20:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            })
            .collect::<Vec<_>>();

        rows.push(MarketListRow {
            symbol: "SZSE.000001".into(),
            base_asset: "平安银行".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 1.23,
            change_24h: -0.2,
            volume_24h: 10_000.0,
            funding_rate: None,
            spread_bps: 35.0,
            exchanges: vec!["akshare:xueqiu".into()],
            updated_at: "2026-05-03T20:20:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        });

        rows
    }
}

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use futures::future::join_all;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::market::MarketDataService;
use crate::models::{
    default_assistant_system_prompt, default_recommendation_system_prompt, ConnectionTestResultDto,
    MarketListRow, ModelConnectionTestPayloadDto, RecommendationRunDto, RiskDecisionDto,
    RuntimeNotificationSettingsDto, RuntimeSettingsDto, SymbolRecommendationDto,
};
use crate::recommendations::risk_engine::{
    evaluate_plan, risk_settings_from_runtime, CandidatePlan,
};
use crate::settings::SettingsService;

type PromptCandles = HashMap<String, PromptKlineContext>;

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct PromptKlineContext {
    bars: HashMap<String, Vec<[String; 4]>>,
    messages: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct StockAgentData {
    stock_info: Value,
    bid_ask: Value,
    kline_data: PromptKlineContext,
    financial_report_analysis: Option<FinancialAnalysisPromptContext>,
    messages: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FinancialAnalysisPromptContext {
    pub key_summary: String,
    pub positive_factors: String,
    pub negative_factors: String,
    pub fraud_risk_points: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PositionContext {
    pub symbol: String,
    pub side: String,
    pub size: String,
    pub entry_price: f64,
    pub mark_price: f64,
    pub pnl_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
struct StockAgentAudit {
    stock_code: String,
    stock_name: String,
    ok: bool,
    raw_output: Option<String>,
    structured_output: Option<ModelRecommendation>,
    normalized_run: Option<RecommendationRunDto>,
    error: Option<String>,
}

pub const RECOMMENDATION_PROMPT_VERSION: &str = "recommendation-system-v2";

#[derive(Debug, Clone)]
pub struct GeneratedTradePlan {
    pub run: RecommendationRunDto,
    pub ai_raw_output: String,
    pub ai_structured_output: String,
    pub system_prompt: String,
    pub user_prompt: String,
}

fn deserialize_number_or_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Number(n) => Ok(Some(n.to_string())),
        _ => Err(D::Error::custom("expected null, string, or number")),
    }
}

fn deserialize_f64_flexible<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(s) => s
            .trim()
            .parse::<f64>()
            .map(Some)
            .map_err(|_| D::Error::custom("expected a numeric string")),
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(Some)
            .ok_or_else(|| D::Error::custom("expected a finite number")),
        _ => Err(D::Error::custom("expected null, number, or numeric string")),
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ModelRecommendation {
    has_trade: bool,
    direction: Option<String>,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    confidence_score: Option<f64>,
    rationale: String,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    entry_low: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    entry_high: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    stop_loss: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_number_or_string")]
    take_profit: Option<String>,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    amount_cny: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_number_or_string")]
    invalidation: Option<String>,
    #[serde(default, deserialize_with = "deserialize_f64_flexible")]
    max_loss_cny: Option<f64>,
}

pub async fn generate_trade_plan(
    settings_service: &SettingsService,
    _market_data_service: &MarketDataService,
    market_rows: &[MarketListRow],
    account_equity_usdt: f64,
    symbol: Option<&str>,
    enabled_exchanges: &[String],
    positions: &[PositionContext],
    financial_analyses: &HashMap<String, FinancialAnalysisPromptContext>,
) -> anyhow::Result<Vec<GeneratedTradePlan>> {
    if market_rows.is_empty() {
        bail!("No live market data was available for the enabled exchanges.");
    }
    let decision_rows = market_rows
        .iter()
        .filter(|row| row.market_type.eq_ignore_ascii_case("ashare"))
        .cloned()
        .collect::<Vec<_>>();
    if decision_rows.is_empty() {
        bail!("自选股暂无可用于 AI 推荐的 A 股行情。");
    }

    let api_key = match settings_service.model_api_key()? {
        Some(value) if !value.trim().is_empty() => value,
        _ => bail!("Model API key is not configured."),
    };

    let runtime = settings_service.get_runtime_settings();
    let system_prompt = build_system_prompt(&runtime);
    let targets = shortlist_candle_targets(market_rows, &runtime, enabled_exchanges);
    let target_symbols = targets
        .iter()
        .map(|target| target.symbol.as_str())
        .collect::<HashSet<_>>();
    let agent_rows = decision_rows
        .iter()
        .filter(|row| {
            symbol
                .map(|focus| row.symbol == focus)
                .unwrap_or_else(|| target_symbols.contains(row.symbol.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let agent_rows = if agent_rows.is_empty() {
        decision_rows.iter().take(1).cloned().collect::<Vec<_>>()
    } else {
        agent_rows
    };

    let agent_tasks = agent_rows.iter().cloned().map(|row| {
        let runtime = runtime.clone();
        let api_key = api_key.clone();
        let system_prompt = system_prompt.clone();
        let decision_rows = decision_rows.clone();
        let financial_analysis = financial_analyses.get(&row.symbol).cloned();
        let position_context = positions
            .iter()
            .find(|position| position.symbol == row.symbol)
            .cloned();
        async move {
            let mut agent_data = fetch_stock_agent_data(&row.symbol, &runtime).await;
            agent_data.financial_report_analysis = financial_analysis;
            let user_prompt = build_stock_agent_market_prompt(
                &row,
                &runtime,
                &agent_data,
                position_context.as_ref(),
            );
            let content = call_text_model(&runtime, &api_key, &system_prompt, &user_prompt)
                .await
                .map(|value| strip_think_blocks(&value));
            let content = match content {
                Ok(value) => value,
                Err(error) => {
                    return StockAgentAudit {
                        stock_code: row.symbol,
                        stock_name: row.base_asset,
                        ok: false,
                        raw_output: None,
                        structured_output: None,
                        normalized_run: None,
                        error: Some(format!("LLM 子任务失败：{error}")),
                    };
                }
            };
            let parsed = match parse_model_recommendation(&content) {
                Ok(value) => value,
                Err(error) => {
                    return StockAgentAudit {
                        stock_code: row.symbol,
                        stock_name: row.base_asset,
                        ok: false,
                        raw_output: Some(content),
                        structured_output: None,
                        normalized_run: None,
                        error: Some(format!("AI 输出解析失败：{error}")),
                    };
                }
            };
            let run = normalize_model_recommendation(
                &parsed,
                &row,
                &decision_rows,
                &runtime,
                account_equity_usdt,
                position_context.as_ref(),
            );
            match run {
                Ok(run) => StockAgentAudit {
                    stock_code: row.symbol,
                    stock_name: row.base_asset,
                    ok: true,
                    raw_output: Some(content),
                    structured_output: Some(parsed),
                    normalized_run: Some(run),
                    error: None,
                },
                Err(error) => StockAgentAudit {
                    stock_code: row.symbol,
                    stock_name: row.base_asset,
                    ok: false,
                    raw_output: Some(content),
                    structured_output: Some(parsed),
                    normalized_run: None,
                    error: Some(format!("AI 建议归一化失败：{error}")),
                },
            }
        }
    });
    let agent_results = join_all(agent_tasks).await;
    let successful_agents = agent_results
        .iter()
        .filter(|agent| agent.normalized_run.is_some())
        .collect::<Vec<_>>();
    if successful_agents.is_empty() {
        let errors = agent_results
            .iter()
            .filter_map(|agent| agent.error.as_deref())
            .collect::<Vec<_>>()
            .join("；");
        bail!("AI 建议子任务全部失败：{errors}");
    }

    let batch_generated_at = current_rfc3339_timestamp()?;
    let symbol_recommendations = agent_results
        .iter()
        .filter_map(|agent| agent.normalized_run.as_ref())
        .filter_map(symbol_recommendation_from_run)
        .collect::<Vec<_>>();

    successful_agents
        .into_iter()
        .map(|agent| {
            let mut run = agent
                .normalized_run
                .clone()
                .ok_or_else(|| anyhow!("AI 子任务缺少归一化结果"))?;
            run.generated_at = batch_generated_at.clone();
            run.symbol_recommendations = symbol_recommendations.clone();
            Ok(GeneratedTradePlan {
                run,
                ai_raw_output: serde_json::to_string(&agent.raw_output)?,
                ai_structured_output: serde_json::to_string(&json!({
                    "mode": "stock_subagent",
                    "input_stock_code": agent.stock_code,
                    "input_stock_name": agent.stock_name,
                    "structured_output": agent.structured_output,
                    "normalized_run": agent.normalized_run,
                    "error": agent.error,
                }))?,
                system_prompt: system_prompt.clone(),
                user_prompt: format!("单股 subagent：{} {}", agent.stock_code, agent.stock_name),
            })
        })
        .collect()
}

pub async fn generate_trade_plan_with_historical_context(
    settings_service: &SettingsService,
    row: &MarketListRow,
    decision_rows: &[MarketListRow],
    account_equity_usdt: f64,
    position_context: Option<&PositionContext>,
    stock_info: Value,
    bid_ask: Option<Value>,
    kline_bars: HashMap<String, Vec<[f64; 4]>>,
) -> anyhow::Result<GeneratedTradePlan> {
    let api_key = match settings_service.model_api_key()? {
        Some(value) if !value.trim().is_empty() => value,
        _ => bail!("Model API key is not configured."),
    };
    let runtime = settings_service.get_runtime_settings();
    let system_prompt = build_system_prompt(&runtime);
    let mut agent_data = StockAgentData::default();
    agent_data.stock_info = stock_info;
    agent_data.bid_ask = bid_ask.unwrap_or_else(|| json!({}));
    for (interval, bars) in kline_bars {
        agent_data.kline_data.bars.insert(
            interval,
            bars.into_iter()
                .map(|bar| {
                    [
                        compress_ohlc(bar[0]),
                        compress_ohlc(bar[1]),
                        compress_ohlc(bar[2]),
                        compress_ohlc(bar[3]),
                    ]
                })
                .collect(),
        );
    }
    agent_data
        .messages
        .insert("source".into(), "历史回测快照，不读取当前行情。".into());

    let user_prompt = build_stock_agent_market_prompt(row, &runtime, &agent_data, position_context);
    let content = call_text_model(&runtime, &api_key, &system_prompt, &user_prompt)
        .await
        .map(|value| strip_think_blocks(&value))?;
    let parsed = parse_model_recommendation(&content)?;
    let run = normalize_model_recommendation(
        &parsed,
        row,
        decision_rows,
        &runtime,
        account_equity_usdt,
        position_context,
    )?;

    Ok(GeneratedTradePlan {
        run,
        ai_raw_output: serde_json::to_string(&content)?,
        ai_structured_output: serde_json::to_string(&json!({
            "mode": "historical_backtest",
            "input_stock_code": row.symbol,
            "input_stock_name": row.base_asset,
            "structured_output": parsed,
        }))?,
        system_prompt,
        user_prompt,
    })
}

fn symbol_recommendation_from_run(run: &RecommendationRunDto) -> Option<SymbolRecommendationDto> {
    Some(SymbolRecommendationDto {
        symbol: run.symbol.clone()?,
        stock_name: run.stock_name.clone(),
        direction: run.direction.clone(),
        rationale: run.rationale.clone(),
        risk_status: run.risk_status.clone(),
        has_trade: run.has_trade,
    })
}

fn stock_agent_fetch_plan_labels() -> Vec<&'static str> {
    vec![
        "stock_info",
        "bid_ask",
        "kline_5m",
        "kline_1h",
        "kline_1d",
        "kline_1w",
    ]
}

async fn fetch_stock_agent_data(symbol: &str, runtime: &RuntimeSettingsDto) -> StockAgentData {
    let stock_info_symbol = symbol.to_string();
    let bid_ask_symbol = symbol.to_string();
    let count = runtime.ai_kline_bar_count.max(1) as usize;
    let use_bid_ask_data = runtime.use_bid_ask_data;
    let kline_symbols = runtime
        .ai_kline_frequencies
        .iter()
        .map(|frequency| (frequency.to_string(), symbol.to_string()))
        .collect::<Vec<_>>();

    let stock_info_task = fetch_agent_json("stock_info", move || {
        crate::market::akshare::fetch_stock_info(&stock_info_symbol)
    });
    let bid_ask_task = async move {
        if use_bid_ask_data {
            fetch_agent_json("bid_ask", move || {
                crate::market::akshare::fetch_bid_ask(&bid_ask_symbol)
            })
            .await
        } else {
            AgentFetchResult {
                value: Some(json!({})),
                message: Some("bid_ask 已按 AI 数据设置关闭。".into()),
            }
        }
    };
    let kline_tasks = kline_symbols.into_iter().map(move |(frequency, symbol)| {
        let label = format!("kline_{frequency}");
        async move {
            let frequency_for_fetch = frequency.clone();
            let result = fetch_agent_bars(&label, move || {
                crate::market::akshare::fetch_history_bars(&symbol, &frequency_for_fetch, count)
            })
            .await;
            (frequency, result)
        }
    });

    let (stock_info, bid_ask, kline_results) =
        tokio::join!(stock_info_task, bid_ask_task, join_all(kline_tasks));
    let mut data = StockAgentData::default();
    data.stock_info = stock_info.value.unwrap_or_else(|| json!({}));
    data.bid_ask = bid_ask.value.unwrap_or_else(|| json!({}));
    if let Some(message) = stock_info.message {
        data.messages.insert("stock_info".into(), message);
    }
    if let Some(message) = bid_ask.message {
        data.messages.insert("bid_ask".into(), message);
    }
    for (frequency, result) in kline_results {
        if let Some(bars) = result.value {
            data.kline_data
                .bars
                .insert(frequency.clone(), prompt_ohlc_from_bars(&bars, count));
        } else {
            data.kline_data.bars.insert(frequency.clone(), Vec::new());
        }
        if let Some(message) = result.message {
            data.kline_data.messages.insert(frequency, message);
        }
    }
    data
}

struct AgentFetchResult<T> {
    value: Option<T>,
    message: Option<String>,
}

async fn fetch_agent_json<F>(label: &'static str, fetch: F) -> AgentFetchResult<Value>
where
    F: FnOnce() -> anyhow::Result<Value> + Send + 'static,
{
    fetch_agent_value(label, fetch).await
}

async fn fetch_agent_bars<F>(
    label: &str,
    fetch: F,
) -> AgentFetchResult<Vec<crate::models::OhlcvBar>>
where
    F: FnOnce() -> anyhow::Result<Vec<crate::models::OhlcvBar>> + Send + 'static,
{
    fetch_agent_value(label, fetch).await
}

async fn fetch_agent_value<T, F>(label: &str, fetch: F) -> AgentFetchResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    match tokio::time::timeout(Duration::from_secs(15), tokio::task::spawn_blocking(fetch)).await {
        Ok(Ok(Ok(value))) => AgentFetchResult {
            value: Some(value),
            message: None,
        },
        Ok(Ok(Err(error))) => AgentFetchResult {
            value: None,
            message: Some(format!("{label} 读取失败：{error}")),
        },
        Ok(Err(error)) => AgentFetchResult {
            value: None,
            message: Some(format!("{label} 任务失败：{error}")),
        },
        Err(_) => AgentFetchResult {
            value: None,
            message: Some(format!("{label} 因网络原因 15 秒内未返回，已返回空值。")),
        },
    }
}

fn prompt_ohlc_from_bars(bars: &[crate::models::OhlcvBar], count: usize) -> Vec<[String; 4]> {
    bars.iter()
        .rev()
        .take(count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|bar| {
            [
                compress_ohlc(bar.open),
                compress_ohlc(bar.high),
                compress_ohlc(bar.low),
                compress_ohlc(bar.close),
            ]
        })
        .collect()
}

fn select_aggregate_recommendation(
    mut runs: Vec<RecommendationRunDto>,
    runtime: &RuntimeSettingsDto,
) -> anyhow::Result<RecommendationRunDto> {
    runs.sort_by(|left, right| {
        right
            .has_trade
            .cmp(&left.has_trade)
            .then(right.risk_status.cmp(&left.risk_status))
            .then(right.confidence_score.total_cmp(&left.confidence_score))
            .then(right.generated_at.cmp(&left.generated_at))
    });
    let mut selected = runs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("没有可汇总的 AI 建议结果"))?;
    if !selected.has_trade && selected.rationale.trim().is_empty() {
        selected.rationale = "各股票 subagent 均未发现满足 A 股模拟风控的买入机会。".into();
    }
    selected.user_preference_version = recommendation_user_preference_version(runtime);
    Ok(selected)
}

pub async fn complete_text(
    settings_service: &SettingsService,
    system_prompt: &str,
    user_prompt: &str,
) -> anyhow::Result<Option<String>> {
    let api_key = match settings_service.model_api_key()? {
        Some(value) if !value.trim().is_empty() => value,
        _ => return Ok(None),
    };

    let runtime = settings_service.get_runtime_settings();
    Ok(Some(
        call_text_model(&runtime, &api_key, system_prompt, user_prompt).await?,
    ))
}

pub async fn test_model_connection(
    payload: &ModelConnectionTestPayloadDto,
) -> ConnectionTestResultDto {
    if payload.model_name.trim().is_empty() {
        return ConnectionTestResultDto {
            ok: false,
            message: "Model name is required.".into(),
        };
    }
    if payload.model_api_key.trim().is_empty() {
        return ConnectionTestResultDto {
            ok: false,
            message: "Provider API key is required.".into(),
        };
    }

    let runtime = runtime_from_model_test_payload(payload);
    match call_text_model(
        &runtime,
        &payload.model_api_key,
        "You are a connection test. Reply with the single word OK.",
        "Return OK.",
    )
    .await
    {
        Ok(answer) => ConnectionTestResultDto {
            ok: true,
            message: format!("Model connection ok: {}", answer.trim()),
        },
        Err(error) => ConnectionTestResultDto {
            ok: false,
            message: error.to_string(),
        },
    }
}

pub(crate) fn build_system_prompt(runtime: &RuntimeSettingsDto) -> String {
    if !runtime.recommendation_system_prompt.trim().is_empty() {
        return runtime.recommendation_system_prompt.trim().to_string();
    }

    let mut prompt = format!(
        "你是 KittyRed 的沪深 A 股模拟投资助手。只输出 JSON，不要输出 Markdown 或解释性前后缀。\
必须始终提供 rationale。没有清晰机会时返回 has_trade=false，并在 rationale 里说明最重要的 2 到 3 个未满足条件。\
如果 has_trade=true，只能给本地模拟买入或已有持仓卖出计划，必须包含 direction、confidence_score、rationale、entry_low、entry_high、stop_loss、take_profit、amount_cny、invalidation、max_loss_cny。\
卖出只适用于 position_context 存在的股票，代表退出或减仓本地模拟持仓，不代表开空仓；无持仓股票只能返回买入或观望。\
不要输出杠杆、真实交易、券商账户、其他市场或套利建议。允许方向模式：{}。最低置信度：{}。\
最低 24h 成交额：{:.0}。最大价差：{:.1} bps。",
        runtime.allowed_direction,
        runtime.min_confidence_score,
        runtime.min_volume_24h,
        runtime.max_spread_bps
    );

    if !runtime.whitelist_symbols.is_empty() {
        prompt.push_str(" 优先关注股票：");
        prompt.push_str(&runtime.whitelist_symbols.join(", "));
        prompt.push('。');
    }

    if !runtime.blacklist_symbols.is_empty() {
        prompt.push_str(" 禁止推荐股票：");
        prompt.push_str(&runtime.blacklist_symbols.join(", "));
        prompt.push('。');
    }

    if !runtime.prompt_extension.trim().is_empty() {
        prompt.push_str(" 用户补充偏好：");
        prompt.push_str(runtime.prompt_extension.trim());
    }

    prompt.push_str(
        r#" has_trade=false 时不要只写“暂无机会”，要结合输入中的价格、成交额、价差、K 线或风控阈值说明原因。必需字段示例：无交易 -> {"has_trade":false,"direction":null,"confidence_score":null,"rationale":"候选股票成交额低于阈值，1h K 线没有确认突破，止损空间无法满足风险收益比。","entry_low":null,"entry_high":null,"stop_loss":null,"take_profit":null,"amount_cny":null,"invalidation":null,"max_loss_cny":null}; 有交易 -> {"has_trade":true,"direction":"买入","confidence_score":72,"rationale":"日线趋势向上，1h 回踩后重新放量，5m 结构确认。","entry_low":8.61,"entry_high":8.66,"stop_loss":8.42,"take_profit":"8.90 / 9.12","amount_cny":12000,"invalidation":"跌破 8.42。","max_loss_cny":330}。必须使用这些 snake_case 字段名。"#,
    );

    prompt
}

pub fn build_market_prompt(
    market_rows: &[MarketListRow],
    runtime: &RuntimeSettingsDto,
    symbol: Option<&str>,
    candles: &PromptCandles,
) -> String {
    let decision_rows = market_rows
        .iter()
        .filter(|row| row.market_type.eq_ignore_ascii_case("ashare"))
        .cloned()
        .collect::<Vec<_>>();
    let scored = score_market_rows(&decision_rows);
    let a_share_shortlist =
        shortlist_for_market(&scored, "ashare", &runtime.watchlist_symbols, candles);

    serde_json::to_string(&json!({
        "request_meta": {
            "trigger_type": "manual_or_auto",
            "focus_stock_code": symbol,
            "generated_at": current_rfc3339_timestamp().unwrap_or_default(),
            "market_scope": "a_share_watchlist",
            "data_source": "akshare",
            "top_n_per_market": 10
        },
        "a_share_shortlist": a_share_shortlist,
        "account_risk_summary": {
            "account_equity_source": "runtime",
            "notes": "仅用于本地 A 股模拟交易计划，遵守运行时风控设置并避免重复暴露。"
        },
        "output_contract": {
            "format": "json",
            "required_fields": [
                "has_trade",
                "direction",
                "confidence_score",
                "rationale",
                "entry_low",
                "entry_high",
                "stop_loss",
                "take_profit",
                "amount_cny",
                "invalidation",
                "max_loss_cny"
            ]
        }
    }))
    .expect("prompt JSON should serialize")
}

fn build_stock_agent_market_prompt(
    row: &MarketListRow,
    runtime: &RuntimeSettingsDto,
    agent_data: &StockAgentData,
    position_context: Option<&PositionContext>,
) -> String {
    let allowed_directions = if position_context.is_some() {
        vec!["买入", "卖出", "持有", "观望"]
    } else {
        vec!["买入", "观望"]
    };
    let kline_data = kline_prompt_value(agent_data);
    let mut stock_context = json!({
        "stock_code": row.symbol,
        "stock_name": row.base_asset,
        "last_price": row.last_price,
        "change_24h": row.change_24h,
        "volume_24h": row.volume_24h,
        "spread_bps": row.spread_bps,
        "ticker_bias": ticker_bias(row),
        "snapshot_age_sec": snapshot_age_seconds(&row.updated_at),
        "stock_info": agent_data.stock_info,
        "bid_ask": agent_data.bid_ask,
        "kline_data": kline_data,
        "messages": agent_data.messages
    });
    if runtime.use_financial_report_data {
        if let Some(financial_report_analysis) = &agent_data.financial_report_analysis {
            stock_context
                .as_object_mut()
                .expect("stock context should be an object")
                .insert("financial_report_analysis".into(), json!(financial_report_analysis));
        }
    }
    serde_json::to_string(&json!({
        "request_meta": {
            "trigger_type": "manual_or_auto",
            "stock_code": row.symbol,
            "stock_name": row.base_asset,
            "market_scope": "single_a_share",
            "data_source": "akshare",
            "generated_at": current_rfc3339_timestamp().unwrap_or_default()
        },
        "stock_context": stock_context,
        "position_context": position_context,
        "allowed_directions_for_this_stock": allowed_directions,
        "risk_settings": {
            "min_confidence_score": runtime.min_confidence_score,
            "max_loss_per_trade_percent": runtime.max_loss_per_trade_percent,
            "min_risk_reward_ratio": runtime.min_risk_reward_ratio,
            "min_volume_24h": runtime.min_volume_24h,
            "max_spread_bps": runtime.max_spread_bps
        },
        "output_contract": {
            "format": "json",
            "required_fields": [
                "has_trade",
                "direction",
                "confidence_score",
                "rationale",
                "entry_low",
                "entry_high",
                "stop_loss",
                "take_profit",
                "amount_cny",
                "invalidation",
                "max_loss_cny"
            ]
        }
    }))
    .expect("stock agent prompt JSON should serialize")
}

fn kline_prompt_value(agent_data: &StockAgentData) -> Value {
    let mut value = serde_json::Map::new();
    for (frequency, bars) in &agent_data.kline_data.bars {
        value.insert(frequency.clone(), json!(bars));
    }
    value.insert("messages".into(), json!(agent_data.kline_data.messages));
    Value::Object(value)
}

struct ScoredMarketRow<'a> {
    row: &'a MarketListRow,
    score: f64,
}

fn shortlist_for_market(
    scored: &[ScoredMarketRow<'_>],
    market_type: &str,
    watchlist_symbols: &[String],
    candles: &PromptCandles,
) -> Vec<Value> {
    let base_items = scored
        .iter()
        .filter(|item| item.row.market_type.eq_ignore_ascii_case(market_type))
        .collect::<Vec<_>>();
    let mut selected = base_items.iter().take(10).copied().collect::<Vec<_>>();
    let mut included_symbols = selected
        .iter()
        .map(|item| item.row.symbol.to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();

    for item in base_items {
        if !symbol_is_watchlisted(&item.row.symbol, watchlist_symbols) {
            continue;
        }
        let normalized_symbol = item.row.symbol.to_ascii_lowercase();
        if included_symbols.insert(normalized_symbol) {
            selected.push(item);
        }
    }

    selected
        .into_iter()
        .map(|item| shortlist_json(item, candles))
        .collect()
}

async fn fetch_shortlist_candles(
    _market_data_service: &MarketDataService,
    market_rows: &[MarketListRow],
    runtime: &RuntimeSettingsDto,
    _enabled_exchanges: &[String],
) -> PromptCandleData {
    let targets = shortlist_candle_targets(market_rows, runtime, &[]);
    let futures_list = targets.into_iter().map({
        move |target| async move {
            let symbol = target.symbol.clone();
            let payload = tokio::task::spawn_blocking(move || {
                crate::market::akshare::fetch_multi_frequency_bars(&symbol, 60)
            })
            .await
            .ok()
            .and_then(Result::ok);

            let Some(payload) = payload else {
                return (target.symbol, None, PromptKlineContext::default());
            };

            (
                target.symbol,
                Some("akshare".into()),
                parse_prompt_kline_context(&payload),
            )
        }
    });

    let results = join_all(futures_list).await;
    let mut candles: PromptCandles = HashMap::new();
    let mut resolved_exchanges = HashMap::new();

    for (symbol, resolved_exchange, context) in results {
        if let Some(exchange) = resolved_exchange {
            resolved_exchanges.insert(symbol.clone(), exchange);
        }
        if !context.bars.is_empty() || !context.messages.is_empty() {
            candles.insert(symbol, context);
        }
    }

    PromptCandleData {
        candles,
        resolved_exchanges,
    }
}

struct CandleTarget {
    symbol: String,
}

struct PromptCandleData {
    candles: PromptCandles,
    resolved_exchanges: HashMap<String, String>,
}

fn shortlist_candle_targets(
    market_rows: &[MarketListRow],
    runtime: &RuntimeSettingsDto,
    _enabled_exchanges: &[String],
) -> Vec<CandleTarget> {
    let decision_rows = market_rows
        .iter()
        .filter(|row| row.market_type.eq_ignore_ascii_case("ashare"))
        .cloned()
        .collect::<Vec<_>>();
    let scored = score_market_rows(&decision_rows);
    let mut targets = Vec::new();
    let mut seen = HashSet::new();

    let base_items = scored.iter().collect::<Vec<_>>();
    let mut selected = base_items.iter().take(10).copied().collect::<Vec<_>>();
    let mut included_symbols: HashSet<_> = selected
        .iter()
        .map(|item| item.row.symbol.to_ascii_lowercase())
        .collect();

    for item in base_items {
        if !symbol_is_watchlisted(&item.row.symbol, &runtime.watchlist_symbols) {
            continue;
        }
        let normalized = item.row.symbol.to_ascii_lowercase();
        if included_symbols.insert(normalized) {
            selected.push(item);
        }
    }

    for item in selected {
        let key = (item.row.symbol.clone(), item.row.market_type.clone());
        if seen.insert(key.clone()) {
            targets.push(CandleTarget { symbol: key.0 });
        }
    }

    targets
}

fn align_rows_to_resolved_candle_venues(
    decision_rows: &[MarketListRow],
    _source_rows: &[MarketListRow],
    _resolved_exchanges: &HashMap<String, String>,
) -> Vec<MarketListRow> {
    decision_rows.to_vec()
}

fn score_market_rows(rows: &[MarketListRow]) -> Vec<ScoredMarketRow<'_>> {
    let max_volume = rows
        .iter()
        .map(|row| row.volume_24h)
        .fold(0.0, f64::max)
        .max(1.0);
    let max_movement = rows
        .iter()
        .map(|row| row.change_24h.abs())
        .fold(0.0, f64::max)
        .max(1.0);
    let venue_count = rows
        .iter()
        .flat_map(|row| row.exchanges.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .len()
        .max(1);
    let mut scored = rows
        .iter()
        .filter(|row| !row.stale)
        .map(|row| {
            let liquidity_score = (row.volume_24h / max_volume * 100.0).clamp(0.0, 100.0);
            let movement = row.change_24h.max(0.0);
            let momentum_score = (movement / max_movement * 100.0).clamp(0.0, 100.0);
            let spread_score = (100.0 - row.spread_bps.min(100.0)).clamp(0.0, 100.0);
            let venue_score =
                (row.exchanges.len() as f64 / venue_count as f64 * 100.0).clamp(0.0, 100.0);
            let score = 0.45 * liquidity_score
                + 0.30 * momentum_score
                + 0.15 * spread_score
                + 0.10 * venue_score;
            ScoredMarketRow { row, score }
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then(right.row.volume_24h.total_cmp(&left.row.volume_24h))
            .then(left.row.spread_bps.total_cmp(&right.row.spread_bps))
            .then(right.row.updated_at.cmp(&left.row.updated_at))
            .then(right.row.exchanges.len().cmp(&left.row.exchanges.len()))
    });
    scored
}

fn market_universe_json(item: &ScoredMarketRow<'_>) -> Value {
    json!({
        "stock_code": item.row.symbol,
        "stock_name": item.row.base_asset,
        "last_price": item.row.last_price,
        "change_24h": item.row.change_24h,
        "volume_24h": item.row.volume_24h,
        "spread_bps": item.row.spread_bps,
        "snapshot_age_sec": snapshot_age_seconds(&item.row.updated_at),
        "data_sources": item.row.exchanges,
        "selection_score": (item.score * 100.0).round() / 100.0,
        "ticker_bias": ticker_bias(item.row)
    })
}

fn shortlist_json(item: &ScoredMarketRow<'_>, candles: &PromptCandles) -> Value {
    let mut value = market_universe_json(item);
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "kline_data".into(),
            candles_prompt_json(&item.row.symbol, candles),
        );
    }
    value
}

fn candles_prompt_json(symbol: &str, candles: &PromptCandles) -> Value {
    let context = candles.get(symbol);
    let bars = |interval: &str| -> Vec<[String; 4]> {
        context
            .and_then(|value| value.bars.get(interval))
            .cloned()
            .unwrap_or_default()
    };

    json!({
        "5m": bars("5m"),
        "1h": bars("1h"),
        "1d": bars("1d"),
        "1w": bars("1w"),
        "messages": context.map(|value| value.messages.clone()).unwrap_or_default(),
    })
}

fn parse_prompt_kline_context(payload: &Value) -> PromptKlineContext {
    let mut context = PromptKlineContext::default();
    if let Some(messages) = payload.get("messages").and_then(Value::as_object) {
        for (interval, message) in messages {
            if let Some(message) = message.as_str() {
                context
                    .messages
                    .insert(interval.to_string(), message.to_string());
            }
        }
    }

    let Some(bars_by_interval) = payload.get("bars").and_then(Value::as_object) else {
        return context;
    };
    for interval in ["5m", "1h", "1d", "1w"] {
        let bars = bars_by_interval
            .get(interval)
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .rev()
                    .take(5)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .filter_map(prompt_ohlc_from_value)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        context.bars.insert(interval.to_string(), bars);
    }
    context
}

fn prompt_ohlc_from_value(value: &Value) -> Option<[String; 4]> {
    let open = value.get("open").and_then(Value::as_f64)?;
    let high = value.get("high").and_then(Value::as_f64)?;
    let low = value.get("low").and_then(Value::as_f64)?;
    let close = value.get("close").and_then(Value::as_f64)?;
    Some([
        compress_ohlc(open),
        compress_ohlc(high),
        compress_ohlc(low),
        compress_ohlc(close),
    ])
}

fn compress_ohlc(value: f64) -> String {
    let abs = value.abs();
    let decimals = if abs >= 1000.0 {
        2
    } else if abs >= 1.0 {
        4
    } else if abs >= 0.01 {
        6
    } else if abs >= 0.0001 {
        8
    } else {
        12
    };
    let formatted = format!("{:.precision$}", value, precision = decimals);
    let trimmed = formatted.trim_end_matches('0');
    if trimmed.ends_with('.') {
        trimmed.trim_end_matches('.').to_string()
    } else {
        trimmed.to_string()
    }
}

fn ticker_bias(row: &MarketListRow) -> &'static str {
    if row.change_24h >= 0.0 {
        "偏多"
    } else {
        "偏弱"
    }
}

fn symbol_is_watchlisted(symbol: &str, watchlist_symbols: &[String]) -> bool {
    watchlist_symbols
        .iter()
        .any(|watchlist_symbol| watchlist_symbol.eq_ignore_ascii_case(symbol))
}

fn snapshot_age_seconds(updated_at: &str) -> Option<i64> {
    parse_timestamp_millis(updated_at).map(|millis| {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        now.saturating_sub(millis / 1_000)
    })
}

fn parse_timestamp_millis(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if let Some(raw) = trimmed.strip_prefix("epoch:") {
        return raw.parse::<i64>().ok().map(|value| {
            if value < 1_000_000_000_000 {
                value * 1_000
            } else {
                value
            }
        });
    }
    if let Ok(raw) = trimmed.parse::<i64>() {
        return Some(if raw < 1_000_000_000_000 {
            raw * 1_000
        } else {
            raw
        });
    }
    OffsetDateTime::parse(trimmed, &Rfc3339)
        .ok()
        .map(|time| time.unix_timestamp() * 1_000)
}

async fn call_openai_compatible(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> anyhow::Result<String> {
    let endpoint = endpoint_url(
        &runtime.model_base_url,
        "https://api.openai.com/v1",
        "chat/completions",
    );
    let response = Client::new()
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&json!({
            "model": runtime.model_name,
            "temperature": runtime.model_temperature,
            "max_tokens": runtime.model_max_tokens,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ]
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("openai-compatible provider returned {status} for {endpoint}: {body}");
    }

    extract_openai_content(&body)
}

async fn call_text_model(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> anyhow::Result<String> {
    if runtime
        .model_provider
        .eq_ignore_ascii_case("anthropic-compatible")
    {
        call_anthropic_compatible(runtime, api_key, system_prompt, user_prompt).await
    } else {
        call_openai_compatible(runtime, api_key, system_prompt, user_prompt).await
    }
}

async fn call_anthropic_compatible(
    runtime: &RuntimeSettingsDto,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> anyhow::Result<String> {
    let endpoint = endpoint_url(
        &runtime.model_base_url,
        "https://api.anthropic.com/v1",
        "messages",
    );
    let response = Client::new()
        .post(&endpoint)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&json!({
            "model": runtime.model_name,
            "max_tokens": runtime.model_max_tokens,
            "temperature": runtime.model_temperature,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": user_prompt }
            ]
        }))
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("anthropic-compatible provider returned {status} for {endpoint}: {body}");
    }

    extract_anthropic_content(&body)
}

fn runtime_from_model_test_payload(payload: &ModelConnectionTestPayloadDto) -> RuntimeSettingsDto {
    RuntimeSettingsDto {
        exchanges: Vec::new(),
        model_provider: payload.model_provider.clone(),
        model_name: payload.model_name.clone(),
        model_base_url: payload.model_base_url.clone(),
        model_temperature: payload.model_temperature,
        model_max_tokens: payload.model_max_tokens,
        model_max_context: payload.model_max_context,
        has_stored_model_api_key: !payload.model_api_key.trim().is_empty(),
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

fn endpoint_url(base_url: &str, default_base: &str, endpoint: &str) -> String {
    let base = if base_url.trim().is_empty() {
        default_base.trim_end_matches('/').to_string()
    } else {
        base_url.trim().trim_end_matches('/').to_string()
    };
    let default_path = reqwest::Url::parse(default_base)
        .ok()
        .and_then(|url| {
            url.path_segments().map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default();

    if base.ends_with(endpoint) {
        base
    } else if base_url.trim().is_empty() {
        format!("{base}/{endpoint}")
    } else if let Ok(parsed_base) = reqwest::Url::parse(&base) {
        let base_path = parsed_base
            .path_segments()
            .map(|segments| {
                segments
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let has_default_path = !default_path.is_empty()
            && base_path.len() >= default_path.len()
            && base_path[base_path.len() - default_path.len()..] == default_path;

        if default_path.is_empty() || has_default_path {
            format!("{base}/{endpoint}")
        } else {
            format!("{base}/{}/{endpoint}", default_path.join("/"))
        }
    } else {
        format!("{base}/{endpoint}")
    }
}

fn extract_anthropic_content(body: &str) -> anyhow::Result<String> {
    let value = serde_json::from_str::<Value>(body)?;

    if let Some(content) = value.get("content") {
        if let Some(text) = content.as_str().filter(|text| !text.trim().is_empty()) {
            return Ok(text.to_string());
        }

        if let Some(blocks) = content.as_array() {
            let text = blocks
                .iter()
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");

            if !text.trim().is_empty() {
                return Ok(text);
            }
        }
    }

    if let Some(text) = value
        .get("completion")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
    {
        return Ok(text.to_string());
    }

    if let Ok(text) = extract_openai_content(body) {
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    bail!("anthropic-compatible response did not contain text");
}

fn extract_openai_content(body: &str) -> anyhow::Result<String> {
    let value = serde_json::from_str::<Value>(body)?;
    let message = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("message"))
        .ok_or_else(|| anyhow!("openai-compatible response missing first choice message"))?;

    if let Some(content) = message.get("content").and_then(Value::as_str) {
        return Ok(content.to_string());
    }

    if let Some(parts) = message.get("content").and_then(Value::as_array) {
        let text = parts
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    bail!("openai-compatible response did not contain textual message content");
}

fn strip_think_blocks(source: &str) -> String {
    let mut result = String::new();
    let mut rest = source;
    loop {
        let Some(start) = rest.find("<think>") else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..start]);
        let after_start = &rest[start + "<think>".len()..];
        let Some(end) = after_start.find("</think>") else {
            break;
        };
        rest = &after_start[end + "</think>".len()..];
    }
    result
}

fn parse_model_recommendation(content: &str) -> anyhow::Result<ModelRecommendation> {
    let normalized = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let payload: ModelRecommendation = serde_json::from_str(normalized)
        .with_context(|| format!("failed to parse model recommendation payload: {normalized}"))?;
    if payload.rationale.trim().is_empty() {
        bail!("model recommendation payload must include a non-empty rationale");
    }
    Ok(payload)
}

fn normalize_model_recommendation(
    payload: &ModelRecommendation,
    input_row: &MarketListRow,
    market_rows: &[MarketListRow],
    runtime: &RuntimeSettingsDto,
    account_equity_usdt: f64,
    position_context: Option<&PositionContext>,
) -> anyhow::Result<RecommendationRunDto> {
    let generated_at = current_rfc3339_timestamp()?;
    let symbol = Some(input_row.symbol.clone());
    let stock_name = Some(input_row.base_asset.clone());
    let market_type = "ashare".to_string();
    let rationale = payload.rationale.clone();
    let market_context = market_rows.iter().find(|row| {
        row.symbol == input_row.symbol && row.market_type.eq_ignore_ascii_case("ashare")
    });
    let exchanges = market_context
        .map(|row| row.exchanges.clone())
        .unwrap_or_default();
    let data_snapshot_at = market_context
        .map(|row| row.updated_at.clone())
        .unwrap_or_else(|| generated_at.clone());
    let user_preference_version = recommendation_user_preference_version(runtime);

    if !payload.has_trade {
        return Ok(RecommendationRunDto {
            recommendation_id: new_recommendation_id("watch", &input_row.symbol),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: false,
            symbol,
            stock_name,
            direction: None,
            market_type,
            exchanges,
            confidence_score: payload.confidence_score.unwrap_or(40.0).clamp(0.0, 100.0),
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
            no_trade_reason: None,
            risk_details: RiskDecisionDto {
                status: "watch".into(),
                risk_score: 0,
                max_loss_estimate: None,
                checks: Vec::new(),
                modifications: Vec::new(),
                block_reasons: Vec::new(),
            },
            data_snapshot_at,
            model_provider: runtime.model_provider.clone(),
            model_name: runtime.model_name.clone(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version,
            generated_at,
        });
    }

    let requested_sell = is_sell_direction(payload.direction.as_deref());
    let direction =
        normalize_a_share_direction(payload.direction.as_deref(), position_context.is_some());
    if requested_sell && position_context.is_none() {
        return Ok(RecommendationRunDto {
            recommendation_id: new_recommendation_id("blocked", &input_row.symbol),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: false,
            symbol,
            stock_name,
            direction: None,
            market_type,
            exchanges,
            confidence_score: payload.confidence_score.unwrap_or(45.0).clamp(0.0, 100.0),
            rationale,
            symbol_recommendations: Vec::new(),
            risk_status: "blocked".into(),
            entry_low: None,
            entry_high: None,
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: None,
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: Some("无持仓股票不能给出卖出建议。".into()),
            risk_details: RiskDecisionDto {
                status: "blocked".into(),
                risk_score: 0,
                max_loss_estimate: None,
                checks: Vec::new(),
                modifications: Vec::new(),
                block_reasons: vec!["无持仓股票不能给出卖出建议。".into()],
            },
            data_snapshot_at,
            model_provider: runtime.model_provider.clone(),
            model_name: runtime.model_name.clone(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version,
            generated_at,
        });
    }
    let leverage = 1.0;
    let is_sell = direction == "卖出";
    let amount_cny = if is_sell {
        None
    } else {
        Some(
            payload
                .amount_cny
                .unwrap_or_else(|| default_amount_cny(symbol.as_deref())),
        )
    };
    let entry_low = payload.entry_low;
    let entry_high = payload.entry_high;
    let stop_loss = payload.stop_loss;
    let max_loss_cny = payload.max_loss_cny.or_else(|| {
        average_entry_price(entry_low, entry_high).and_then(|entry| {
            stop_loss.and_then(|stop| {
                amount_cny.map(|amount| amount * ((entry - stop).abs() / entry.max(1.0)))
            })
        })
    });
    let take_profit_targets = parse_take_profit_targets(payload.take_profit.as_deref());

    let risk = evaluate_plan(
        &CandidatePlan {
            symbol: symbol.clone().unwrap_or_else(|| "A股观察池".into()),
            market_type: market_type.clone(),
            direction: if is_sell {
                "sell".into()
            } else {
                "long".into()
            },
            leverage,
            stop_loss,
            entry_low,
            entry_high,
            take_profit_targets,
            amount_cny,
            volume_24h: market_context.map(|row| row.volume_24h).unwrap_or_default(),
            spread_bps: market_context.map(|row| row.spread_bps).unwrap_or_default(),
            confidence_score: payload.confidence_score.unwrap_or(65.0).clamp(0.0, 100.0),
            risk_tags: Vec::new(),
        },
        &risk_settings_from_runtime(runtime, account_equity_usdt),
    );

    if risk.status != "approved" {
        return Ok(RecommendationRunDto {
            recommendation_id: new_recommendation_id("blocked", &input_row.symbol),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: false,
            symbol,
            stock_name,
            direction: None,
            market_type,
            exchanges,
            confidence_score: payload.confidence_score.unwrap_or(45.0).clamp(0.0, 100.0),
            rationale,
            symbol_recommendations: Vec::new(),
            risk_status: "blocked".into(),
            entry_low: None,
            entry_high: None,
            stop_loss: None,
            take_profit: None,
            leverage: None,
            amount_cny: None,
            invalidation: None,
            max_loss_cny: None,
            no_trade_reason: Some(format!(
                "AI 给出的 A 股计划未通过风控：{}",
                risk.primary_reason()
                    .unwrap_or_else(|| "unknown_risk_reason".into())
            )),
            risk_details: risk.to_decision_dto(),
            data_snapshot_at,
            model_provider: runtime.model_provider.clone(),
            model_name: runtime.model_name.clone(),
            prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
            user_preference_version,
            generated_at,
        });
    }

    Ok(RecommendationRunDto {
        recommendation_id: new_recommendation_id("llm", &input_row.symbol),
        status: "completed".into(),
        trigger_type: "manual".into(),
        has_trade: true,
        symbol,
        stock_name,
        direction: Some(direction),
        market_type,
        exchanges,
        confidence_score: payload.confidence_score.unwrap_or(65.0).clamp(0.0, 100.0),
        rationale,
        symbol_recommendations: Vec::new(),
        risk_status: "approved".into(),
        entry_low,
        entry_high,
        stop_loss,
        take_profit: payload.take_profit.clone(),
        leverage: None,
        amount_cny,
        invalidation: Some(
            payload
                .invalidation
                .clone()
                .unwrap_or_else(|| "跌破止损位时原交易假设失效。".into()),
        ),
        max_loss_cny: max_loss_cny.map(|value| (value * 100.0).round() / 100.0),
        no_trade_reason: None,
        risk_details: risk.to_decision_dto(),
        data_snapshot_at,
        model_provider: runtime.model_provider.clone(),
        model_name: runtime.model_name.clone(),
        prompt_version: RECOMMENDATION_PROMPT_VERSION.into(),
        user_preference_version,
        generated_at,
    })
}

pub fn recommendation_user_preference_version(runtime: &RuntimeSettingsDto) -> String {
    let payload = json!({
        "allowed_markets": runtime.allowed_markets,
        "allowed_direction": runtime.allowed_direction,
        "max_leverage": runtime.max_leverage,
        "max_loss_per_trade_percent": runtime.max_loss_per_trade_percent,
        "max_daily_loss_percent": runtime.max_daily_loss_percent,
        "min_risk_reward_ratio": runtime.min_risk_reward_ratio,
        "min_volume_24h": runtime.min_volume_24h,
        "max_spread_bps": runtime.max_spread_bps,
        "min_confidence_score": runtime.min_confidence_score,
        "allow_meme_coins": runtime.allow_meme_coins,
        "whitelist_symbols": runtime.whitelist_symbols,
        "blacklist_symbols": runtime.blacklist_symbols,
        "prompt_extension": runtime.prompt_extension,
        "account_mode": runtime.account_mode,
        "auto_paper_execution": runtime.auto_paper_execution,
    });
    let mut hasher = Sha256::new();
    hasher.update(payload.to_string().as_bytes());
    let digest = hasher.finalize();
    format!("prefs-{}", hex::encode(&digest[..6]))
}

fn normalize_a_share_direction(direction: Option<&str>, has_position: bool) -> String {
    match direction.unwrap_or_default().trim().to_lowercase().as_str() {
        "sell" | "卖出" | "减仓" | "退出" if has_position => "卖出".into(),
        "hold" | "watch" | "observe" | "持有" | "观望" => "观望".into(),
        _ => "买入".into(),
    }
}

fn is_sell_direction(direction: Option<&str>) -> bool {
    matches!(
        direction.unwrap_or_default().trim().to_lowercase().as_str(),
        "sell" | "卖出" | "减仓" | "退出"
    )
}

fn average_entry_price(entry_low: Option<f64>, entry_high: Option<f64>) -> Option<f64> {
    match (entry_low, entry_high) {
        (Some(low), Some(high)) => Some((low + high) / 2.0),
        (Some(low), None) => Some(low),
        (None, Some(high)) => Some(high),
        (None, None) => None,
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

fn default_amount_cny(symbol: Option<&str>) -> f64 {
    match symbol {
        Some(value) if value.starts_with("SHSE.600") => 12_000.0,
        Some(value) if value.starts_with("SZSE.000") => 10_000.0,
        _ => 8_000.0,
    }
}

fn new_recommendation_id(prefix: &str, symbol: &str) -> String {
    let normalized_symbol = symbol
        .to_ascii_lowercase()
        .replace('.', "-")
        .replace('/', "-")
        .replace('_', "-");
    format!(
        "rec-{prefix}-{normalized_symbol}-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    )
}

fn current_rfc3339_timestamp() -> anyhow::Result<String> {
    Ok(OffsetDateTime::now_utc().format(&Rfc3339)?)
}
