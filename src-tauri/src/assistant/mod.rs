pub mod context;
pub mod llm;
pub mod tools;

#[cfg(test)]
mod tests {
    use super::{
        execute_tool, extract_symbol, AssistantEvent, AssistantEventEmitter, AssistantService,
        AssistantStoredMessage,
    };
    use crate::market::MarketDataService;
    use crate::models::MarketListRow;
    use crate::paper::PaperService;
    use crate::portfolio::PortfolioService;
    use crate::recommendations::RecommendationService;
    use crate::settings::SettingsService;
    use crate::signals::SignalService;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[derive(Clone, Default)]
    struct VecEmitter {
        events: Arc<Mutex<Vec<AssistantEvent>>>,
    }

    impl AssistantEventEmitter for VecEmitter {
        fn emit(&self, event: &AssistantEvent) {
            self.events
                .lock()
                .expect("event lock poisoned")
                .push(event.clone());
        }
    }

    fn test_signal_service() -> SignalService {
        let tmp = std::env::temp_dir().join("test_signals_assistant.sqlite3");
        SignalService::new(tmp).unwrap()
    }

    fn test_financial_report_service() -> crate::financial_reports::FinancialReportService {
        crate::financial_reports::FinancialReportService::new(unique_temp_path(
            "assistant-financial-reports",
            "sqlite3",
        ))
        .unwrap()
    }

    #[test]
    fn stop_marks_session_cancelled() {
        let service = AssistantService::new_for_tests(VecEmitter::default());
        service.record_session_messages(
            "session-1",
            vec![AssistantStoredMessage::new("user", "hello")],
        );

        assert!(service.stop_run("session-1"));
        assert!(service.is_cancelled("session-1"));
    }

    #[test]
    fn clear_session_resets_messages_and_cancelled_state() {
        let service = AssistantService::new_for_tests(VecEmitter::default());
        service.record_session_messages(
            "session-1",
            vec![
                AssistantStoredMessage::new("user", "hello"),
                AssistantStoredMessage::new("assistant", "world"),
            ],
        );
        let _ = service.stop_run("session-1");

        service.clear_session("session-1");

        assert!(!service.is_cancelled("session-1"));
        assert!(service.session_messages("session-1").is_empty());
    }

    #[test]
    fn extracts_symbols_from_the_dynamic_market_universe() {
        let available_symbols = vec!["SHSE.600000".to_string(), "SZSE.000001".to_string()];

        assert_eq!(
            extract_symbol("看一下 600000 今天怎么样", &available_symbols),
            Some("SHSE.600000".to_string())
        );
        assert_eq!(
            extract_symbol("000001 的持仓风险呢", &available_symbols),
            Some("SZSE.000001".to_string())
        );
        assert_eq!(extract_symbol("看一下黄金", &available_symbols), None);
    }

    #[tokio::test]
    async fn market_data_tool_resolves_base_symbol_to_cached_pair() {
        let settings_path = unique_temp_path("assistant-settings", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let market_data = MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "浦发银行".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 9.16,
            change_24h: -0.22,
            volume_24h: 245_736_322.0,
            funding_rate: None,
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
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());

        let result = execute_tool(
            "market_data",
            json!({
                "stockCode": "600000",
                "limit": 1
            }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &test_financial_report_service(),
        )
        .await
        .expect("market data tool should resolve cached A-share row");

        let payload: Value =
            serde_json::from_str(&result).expect("tool result should be valid json");
        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["rows"][0]["stockCode"], json!("SHSE.600000"));
        assert!(payload["rows"][0].get("marketType").is_none());
        assert!(payload["rows"][0].get("fundingRate").is_none());

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn market_data_tool_accepts_stock_code_argument() {
        let settings_path = unique_temp_path("assistant-settings", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let market_data = MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "600000".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 9.16,
            change_24h: -0.22,
            volume_24h: 245_736_322.0,
            funding_rate: None,
            spread_bps: 3.4,
            exchanges: vec!["akshare".into()],
            updated_at: "2026-05-07T10:00:00+08:00".into(),
            stale: false,
            venue_snapshots: Vec::new(),
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 0,
            fdv_usd: None,
        }]);
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());

        let result = execute_tool(
            "market_data",
            json!({
                "stockCode": "600000",
                "limit": 1
            }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &test_financial_report_service(),
        )
        .await
        .expect("market data tool should resolve stock code");

        let payload: Value =
            serde_json::from_str(&result).expect("tool result should be valid json");
        assert_eq!(payload["rows"][0]["stockCode"], json!("SHSE.600000"));
        assert_eq!(payload["rows"][0]["source"], json!("akshare"));
        assert!(payload["rows"][0].get("symbol").is_none());
        assert!(payload["rows"][0].get("funding_rate").is_none());

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn financial_report_tool_returns_cached_analysis_without_fetching() {
        let settings_path = unique_temp_path("assistant-settings-financial", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations-financial", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.use_financial_report_data = true;
        settings.save_runtime_settings(runtime.clone()).unwrap();
        let market_data = MarketDataService::with_static_rows(Vec::new());
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());
        let financial_reports = test_financial_report_service();
        let revision = financial_reports
            .snapshot("SHSE.600000")
            .unwrap()
            .source_revision;
        financial_reports
            .save_analysis(
                "SHSE.600000",
                &revision,
                "收入和利润稳定",
                "现金流改善",
                "费用率上升",
                "暂无明显异常",
                &runtime,
            )
            .unwrap();

        let result = execute_tool(
            "financial_report_info",
            json!({ "stockCode": "600000" }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &financial_reports,
        )
        .await
        .unwrap();
        let payload: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["stockCode"], json!("SHSE.600000"));
        assert_eq!(payload["analysis"]["关键信息总结"], json!("收入和利润稳定"));
        assert!(payload.get("raw").is_none());

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn financial_report_tool_reports_missing_cache_in_chinese() {
        let settings_path = unique_temp_path("assistant-settings-financial-empty", "json");
        let recommendation_path =
            unique_temp_path("assistant-recommendations-financial-empty", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.use_financial_report_data = true;
        settings.save_runtime_settings(runtime).unwrap();
        let market_data = MarketDataService::with_static_rows(Vec::new());
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());
        let financial_reports = test_financial_report_service();

        let result = execute_tool(
            "financial_report_info",
            json!({ "stockCode": "600000" }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &financial_reports,
        )
        .await
        .unwrap();
        let payload: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(payload["ok"], json!(false));
        assert!(payload["message"]
            .as_str()
            .unwrap()
            .contains("财报分析页面"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn positions_tool_uses_portfolio_view_when_paper_only_is_false() {
        let settings_path = unique_temp_path("assistant-settings", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.account_mode = "dual".into();
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should persist");
        let market_data = MarketDataService::with_static_rows(vec![MarketListRow {
            symbol: "SHSE.600000".into(),
            base_asset: "浦发银行".into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 9.16,
            change_24h: -0.22,
            volume_24h: 245_736_322.0,
            funding_rate: None,
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
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());

        paper
            .create_draft_from_recommendation(
                &sample_recommendation_run(
                    "rec-paper-600000",
                    "SHSE.600000",
                    "2026-05-04T10:00:00+08:00",
                ),
                "paper-cash",
            )
            .await
            .expect("paper order draft should seed a position");

        let result = execute_tool(
            "positions",
            json!({
                "stockCode": "600000",
                "paperOnly": false
            }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &test_financial_report_service(),
        )
        .await
        .expect("positions tool should return combined portfolio positions");

        let payload: Value =
            serde_json::from_str(&result).expect("tool result should be valid json");
        assert_eq!(payload["ok"], json!(true));
        assert_eq!(payload["positions"][0]["account"], json!("人民币现金"));
        assert_eq!(payload["orders"][0]["account"], json!("人民币现金"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn recommendation_history_honors_symbol_and_latest_only() {
        let settings_path = unique_temp_path("assistant-settings", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let market_data = MarketDataService::with_static_rows(vec![
            sample_market_row("SHSE.600000"),
            sample_market_row("SZSE.000001"),
        ]);
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());

        recommendations
            .store_run(sample_recommendation_run(
                "rec-000001-1",
                "SZSE.000001",
                "2026-05-04T09:00:00+08:00",
            ))
            .await
            .expect("first recommendation should store");
        recommendations
            .store_run(sample_recommendation_run(
                "rec-000001-2",
                "SZSE.000001",
                "2026-05-04T10:00:00+08:00",
            ))
            .await
            .expect("second recommendation should store");
        recommendations
            .store_run(sample_recommendation_run(
                "rec-600000-1",
                "SHSE.600000",
                "2026-05-04T11:00:00+08:00",
            ))
            .await
            .expect("other recommendation should store");

        let result = execute_tool(
            "recommendation_history",
            json!({
                "stockCode": "000001",
                "latestOnly": true
            }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &test_financial_report_service(),
        )
        .await
        .expect("recommendation history tool should resolve latest filtered symbol");

        let payload: Value =
            serde_json::from_str(&result).expect("tool result should be valid json");
        assert_eq!(payload["latest"]["stockCode"], json!("SZSE.000001"));
        assert_eq!(payload["latest"]["recommendationId"], json!("rec-000001-2"));
        assert_eq!(payload["rows"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            payload["rows"][0]["recommendationId"],
            json!("rec-000001-2")
        );

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    #[tokio::test]
    async fn risk_calculator_honors_symbol_when_selecting_recommendation() {
        let settings_path = unique_temp_path("assistant-settings", "json");
        let recommendation_path = unique_temp_path("assistant-recommendations", "sqlite3");
        let settings = SettingsService::new(settings_path.clone());
        let market_data = MarketDataService::with_static_rows(vec![
            sample_market_row("SHSE.600000"),
            sample_market_row("SZSE.000001"),
        ]);
        let paper = PaperService::default();
        let portfolio = PortfolioService::new(paper.clone());
        let recommendations = RecommendationService::new(recommendation_path.clone());

        recommendations
            .store_run(sample_recommendation_run(
                "rec-000001-risk",
                "SZSE.000001",
                "2026-05-04T10:00:00+08:00",
            ))
            .await
            .expect("requested recommendation should store");
        recommendations
            .store_run(sample_recommendation_run(
                "rec-600000-risk",
                "SHSE.600000",
                "2026-05-04T11:00:00+08:00",
            ))
            .await
            .expect("other recommendation should store");

        let result = execute_tool(
            "risk_calculator",
            json!({
                "stockCode": "000001"
            }),
            &market_data,
            &portfolio,
            &paper,
            &recommendations,
            &settings,
            &test_signal_service(),
            &test_financial_report_service(),
        )
        .await
        .expect("risk calculator should resolve requested symbol");

        let payload: Value =
            serde_json::from_str(&result).expect("tool result should be valid json");
        assert_eq!(payload["stockCode"], json!("SZSE.000001"));
        assert_eq!(payload["recommendationId"], json!("rec-000001-risk"));

        let _ = std::fs::remove_file(settings_path);
        let _ = std::fs::remove_file(recommendation_path);
    }

    fn unique_temp_path(label: &str, extension: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        let counter = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "kittyred-{label}-{}-{nanos}-{counter}.{extension}",
            std::process::id()
        ))
    }

    fn sample_market_row(symbol: &str) -> MarketListRow {
        MarketListRow {
            symbol: symbol.into(),
            base_asset: if symbol.ends_with("600000") {
                "浦发银行"
            } else {
                "平安银行"
            }
            .into(),
            market_type: "ashare".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "small".into(),
            last_price: 9.16,
            change_24h: -0.22,
            volume_24h: 245_736_322.0,
            funding_rate: None,
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
        }
    }

    fn sample_recommendation_run(
        recommendation_id: &str,
        symbol: &str,
        generated_at: &str,
    ) -> crate::models::RecommendationRunDto {
        crate::models::RecommendationRunDto {
            recommendation_id: recommendation_id.into(),
            status: "completed".into(),
            trigger_type: "manual".into(),
            has_trade: true,
            symbol: Some(symbol.into()),
            stock_name: Some(symbol.into()),
            direction: Some("Long".into()),
            market_type: "ashare".into(),
            exchanges: vec!["akshare".into(), "人民币现金".into()],
            confidence_score: 78.0,
            rationale: format!("A-share recommendation for {symbol}"),
            symbol_recommendations: Vec::new(),
            risk_status: "approved".into(),
            entry_low: Some(9.10),
            entry_high: Some(9.20),
            stop_loss: Some(8.92),
            take_profit: Some("9.60 / 9.90".into()),
            leverage: None,
            amount_cny: Some(18_000.0),
            invalidation: Some("Invalidation".into()),
            max_loss_cny: Some(47.4),
            no_trade_reason: None,
            risk_details: crate::models::RiskDecisionDto {
                status: "approved".into(),
                risk_score: 42,
                max_loss_estimate: Some("0.47%".into()),
                checks: Vec::new(),
                modifications: Vec::new(),
                block_reasons: Vec::new(),
            },
            data_snapshot_at: generated_at.into(),
            model_provider: "System".into(),
            model_name: "heuristic-fallback".into(),
            prompt_version: "recommendation-system-v2".into(),
            user_preference_version: "prefs-assistant".into(),
            generated_at: generated_at.into(),
        }
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Emitter;

use self::context::{
    estimate_context, AssistantStoredMessage, ThinkBlockStreamFilter, ThinkStreamEvent,
};
use crate::financial_reports::FinancialReportService;
use crate::market::{akshare, MarketDataService};
use crate::models::RecommendationRunDto;
use crate::paper::{paper_account_id, PaperService};
use crate::portfolio::PortfolioService;
use crate::recommendations::RecommendationService;
use crate::settings::SettingsService;
use crate::signals::SignalService;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantEvent {
    pub session_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_preview: Option<String>,
}

impl AssistantEvent {
    fn new(session_id: &str, event_type: &str) -> Self {
        Self {
            session_id: session_id.into(),
            event_type: event_type.into(),
            delta: None,
            status: None,
            reply: None,
            error: None,
            context: None,
            tool_call_id: None,
            name: None,
            summary: None,
            arguments: None,
            result_preview: None,
        }
    }
}

pub trait AssistantEventEmitter: Send + Sync + 'static {
    fn emit(&self, event: &AssistantEvent);
}

#[derive(Clone)]
struct TauriAssistantEmitter {
    app_handle: tauri::AppHandle,
}

impl AssistantEventEmitter for TauriAssistantEmitter {
    fn emit(&self, event: &AssistantEvent) {
        let _ = self.app_handle.emit("assistant://event", event);
    }
}

#[derive(Clone)]
pub struct AssistantService {
    inner: Arc<AssistantServiceInner>,
}

struct AssistantServiceInner {
    emitter: Arc<dyn AssistantEventEmitter>,
    sessions: Mutex<HashMap<String, AssistantSession>>,
}

#[derive(Default)]
struct AssistantSession {
    cancelled: bool,
    run_id: usize,
    messages: Vec<AssistantStoredMessage>,
}

enum ConversationItem {
    User(String),
    AssistantText(String),
    AssistantToolCalls {
        content: String,
        tool_calls: Vec<llm::AssistantToolCall>,
    },
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

impl AssistantService {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self::new_with_emitter(TauriAssistantEmitter { app_handle })
    }

    #[cfg(test)]
    pub fn new_for_tests<E>(emitter: E) -> Self
    where
        E: AssistantEventEmitter,
    {
        Self::new_with_emitter(emitter)
    }

    fn new_with_emitter<E>(emitter: E) -> Self
    where
        E: AssistantEventEmitter,
    {
        Self {
            inner: Arc::new(AssistantServiceInner {
                emitter: Arc::new(emitter),
                sessions: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start_run(
        &self,
        session_id: String,
        message: String,
        market_data_service: MarketDataService,
        portfolio_service: PortfolioService,
        paper_service: PaperService,
        recommendation_service: RecommendationService,
        settings_service: SettingsService,
        signal_service: SignalService,
        financial_report_service: FinancialReportService,
    ) {
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = service
                .run_inner(
                    &session_id,
                    &message,
                    &market_data_service,
                    &portfolio_service,
                    &paper_service,
                    &recommendation_service,
                    &settings_service,
                    &signal_service,
                    &financial_report_service,
                )
                .await
            {
                let runtime = settings_service.get_runtime_settings();
                let system_prompt = assistant_system_prompt(&settings_service);
                service.emit_error(
                    &session_id,
                    &error.to_string(),
                    Some(service.context_value(
                        &session_id,
                        &system_prompt,
                        runtime.model_max_context.max(1024) as usize,
                    )),
                );
            }
        });
    }

    pub fn stop_run(&self, session_id: &str) -> bool {
        let mut sessions = self
            .inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned");
        let session = sessions.entry(session_id.into()).or_default();
        session.cancelled = true;
        true
    }

    pub fn clear_session(&self, session_id: &str) {
        self.stop_run(session_id);
        let mut sessions = self
            .inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned");
        sessions.insert(session_id.into(), AssistantSession::default());
    }

    #[cfg(test)]
    fn is_cancelled(&self, session_id: &str) -> bool {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .is_some_and(|session| session.cancelled)
    }

    #[cfg(test)]
    fn session_messages(&self, session_id: &str) -> Vec<AssistantStoredMessage> {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .map(|session| session.messages.clone())
            .unwrap_or_default()
    }

    #[cfg(test)]
    fn record_session_messages(&self, session_id: &str, messages: Vec<AssistantStoredMessage>) {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .insert(
                session_id.into(),
                AssistantSession {
                    messages,
                    ..AssistantSession::default()
                },
            );
    }

    async fn run_inner(
        &self,
        session_id: &str,
        user_input: &str,
        market_data_service: &MarketDataService,
        portfolio_service: &PortfolioService,
        paper_service: &PaperService,
        recommendation_service: &RecommendationService,
        settings_service: &SettingsService,
        signal_service: &SignalService,
        financial_report_service: &FinancialReportService,
    ) -> anyhow::Result<()> {
        let api_key = settings_service
            .model_api_key()?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("No model API key configured for Assistant."))?;
        let runtime = settings_service.get_runtime_settings();
        let max_context = runtime.model_max_context.max(1024) as usize;
        let system_prompt = assistant_system_prompt(settings_service);
        let run_id = self.begin_user_turn(session_id, user_input);
        self.emit_status(session_id, "running", &system_prompt, max_context);
        let mut conversation = self.conversation_from_session(session_id);

        for _ in 0..12 {
            if !self.is_current_run(session_id, run_id) {
                return Ok(());
            }
            if self.is_cancelled_now(session_id) {
                self.emit_cancelled(session_id, &system_prompt, max_context);
                return Ok(());
            }

            let openai_tools = tools::openai_tool_schemas_for_runtime(&runtime);
            let anthropic_tools = tools::anthropic_tool_schemas_for_runtime(&runtime);
            let is_anthropic = runtime
                .model_provider
                .eq_ignore_ascii_case("anthropic-compatible");
            let messages = if is_anthropic {
                anthropic_messages(&conversation)
            } else {
                openai_messages(&system_prompt, &conversation)
            };

            let think_filter = Arc::new(Mutex::new(ThinkBlockStreamFilter::default()));
            let token_service = self.clone();
            let token_session_id = session_id.to_string();
            let token_think_filter = think_filter.clone();
            let mut on_token = move |token: &str| {
                let events = token_think_filter
                    .lock()
                    .expect("assistant think filter lock poisoned")
                    .consume(token);
                for event in events {
                    match event {
                        ThinkStreamEvent::Visible(delta) => {
                            let mut payload = AssistantEvent::new(&token_session_id, "token");
                            payload.delta = Some(delta);
                            token_service.emit(payload);
                        }
                        ThinkStreamEvent::ThinkingStatus(status) => {
                            token_service.emit_thinking_status(&token_session_id, &status);
                        }
                        ThinkStreamEvent::ThinkingDelta(delta) => {
                            token_service.emit_thinking_delta(&token_session_id, &delta);
                        }
                    }
                }
            };
            let reasoning_service = self.clone();
            let reasoning_session_id = session_id.to_string();
            let mut on_reasoning = move |delta: &str| {
                if !delta.is_empty() {
                    reasoning_service.emit_thinking_status(&reasoning_session_id, "running");
                    reasoning_service.emit_thinking_delta(&reasoning_session_id, delta);
                }
            };
            let continue_service = self.clone();
            let continue_session_id = session_id.to_string();
            let continue_run_id = run_id;
            let response = llm::request_stream(
                &runtime,
                &api_key,
                &system_prompt,
                messages,
                if is_anthropic {
                    anthropic_tools
                } else {
                    openai_tools
                },
                &mut on_token,
                &mut on_reasoning,
                move || {
                    continue_service.is_current_run(&continue_session_id, continue_run_id)
                        && !continue_service.is_cancelled_now(&continue_session_id)
                },
            )
            .await?;

            if !self.is_current_run(session_id, run_id) {
                return Ok(());
            }
            if self.is_cancelled_now(session_id) {
                self.emit_cancelled(session_id, &system_prompt, max_context);
                return Ok(());
            }
            if think_filter
                .lock()
                .expect("assistant think filter lock poisoned")
                .needs_finish_event()
            {
                self.emit_thinking_status(session_id, "finished");
            }
            if !response.reasoning_content.is_empty() {
                self.emit_thinking_status(session_id, "finished");
            }

            if response.tool_calls.is_empty() {
                self.append_assistant_message(session_id, &response.content);
                let reply = strip_think_blocks(&response.content).trim().to_string();
                let mut payload = AssistantEvent::new(session_id, "done");
                payload.reply = Some(reply);
                payload.context = Some(self.context_value(session_id, &system_prompt, max_context));
                self.emit(payload);
                return Ok(());
            }

            self.append_assistant_tool_context(session_id, &response.content);
            conversation.push(ConversationItem::AssistantToolCalls {
                content: response.content.clone(),
                tool_calls: response.tool_calls.clone(),
            });

            for tool_call in response.tool_calls {
                if !self.is_current_run(session_id, run_id) {
                    return Ok(());
                }
                if self.is_cancelled_now(session_id) {
                    self.emit_cancelled(session_id, &system_prompt, max_context);
                    return Ok(());
                }

                let mut start = AssistantEvent::new(session_id, "tool_start");
                start.tool_call_id = Some(tool_call.id.clone());
                start.name = Some(tool_call.name.clone());
                start.arguments = Some(tool_call.arguments.clone());
                start.summary = Some(summarize_tool_call(&tool_call.name, &tool_call.arguments));
                self.emit(start);

                let result = match execute_tool(
                    &tool_call.name,
                    tool_call.arguments.clone(),
                    market_data_service,
                    portfolio_service,
                    paper_service,
                    recommendation_service,
                    settings_service,
                    signal_service,
                    financial_report_service,
                )
                .await
                {
                    Ok(value) => value,
                    Err(error) => format!("Error: {error}"),
                };

                self.append_tool_result(session_id, &result);
                conversation.push(ConversationItem::ToolResult {
                    tool_call_id: tool_call.id.clone(),
                    content: result.clone(),
                });

                let mut output = AssistantEvent::new(session_id, "tool_output");
                output.tool_call_id = Some(tool_call.id.clone());
                output.delta = Some(result.clone());
                self.emit(output);

                let mut end = AssistantEvent::new(session_id, "tool_end");
                end.tool_call_id = Some(tool_call.id);
                end.name = Some(tool_call.name);
                end.status = Some(if result.starts_with("Error:") {
                    "error".into()
                } else {
                    "done".into()
                });
                end.result_preview = Some(preview_tool_result(&result));
                end.context = Some(self.context_value(session_id, &system_prompt, max_context));
                self.emit(end);
            }
        }

        let mut payload = AssistantEvent::new(session_id, "done");
        payload.reply = Some("(reached maximum tool-call rounds)".into());
        payload.context = Some(self.context_value(session_id, &system_prompt, max_context));
        self.emit(payload);
        Ok(())
    }

    fn begin_user_turn(&self, session_id: &str, user_input: &str) -> usize {
        let mut sessions = self
            .inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned");
        let session = sessions.entry(session_id.into()).or_default();
        session.run_id += 1;
        session.cancelled = false;
        session
            .messages
            .push(AssistantStoredMessage::new("user", user_input));
        session.run_id
    }

    fn is_current_run(&self, session_id: &str, run_id: usize) -> bool {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .is_some_and(|session| session.run_id == run_id)
    }

    fn is_cancelled_now(&self, session_id: &str) -> bool {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .is_some_and(|session| session.cancelled)
    }

    fn conversation_from_session(&self, session_id: &str) -> Vec<ConversationItem> {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .map(|session| {
                session
                    .messages
                    .iter()
                    .filter_map(|message| match message.role.as_str() {
                        "user" => Some(ConversationItem::User(message.content.clone())),
                        "assistant" => {
                            Some(ConversationItem::AssistantText(message.content.clone()))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn append_assistant_tool_context(&self, session_id: &str, content: &str) {
        if content.trim().is_empty() {
            return;
        }
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .entry(session_id.into())
            .or_default()
            .messages
            .push(AssistantStoredMessage::new("assistant", content));
    }

    fn append_assistant_message(&self, session_id: &str, content: &str) {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .entry(session_id.into())
            .or_default()
            .messages
            .push(AssistantStoredMessage::new("assistant", content));
    }

    fn append_tool_result(&self, session_id: &str, result: &str) {
        self.inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .entry(session_id.into())
            .or_default()
            .messages
            .push(AssistantStoredMessage::new("tool", result));
    }

    fn emit_status(&self, session_id: &str, status: &str, system_prompt: &str, max_context: usize) {
        let mut payload = AssistantEvent::new(session_id, "status");
        payload.status = Some(status.into());
        payload.context = Some(self.context_value(session_id, system_prompt, max_context));
        self.emit(payload);
    }

    fn emit_cancelled(&self, session_id: &str, system_prompt: &str, max_context: usize) {
        let mut payload = AssistantEvent::new(session_id, "cancelled");
        payload.context = Some(self.context_value(session_id, system_prompt, max_context));
        self.emit(payload);
    }

    fn emit_error(&self, session_id: &str, error: &str, context: Option<Value>) {
        let mut payload = AssistantEvent::new(session_id, "error");
        payload.error = Some(error.into());
        payload.context = context;
        self.emit(payload);
    }

    fn emit_thinking_status(&self, session_id: &str, status: &str) {
        let mut payload = AssistantEvent::new(session_id, "thinking_status");
        payload.status = Some(status.into());
        self.emit(payload);
    }

    fn emit_thinking_delta(&self, session_id: &str, delta: &str) {
        let mut payload = AssistantEvent::new(session_id, "thinking_delta");
        payload.delta = Some(delta.into());
        self.emit(payload);
    }

    fn context_value(&self, session_id: &str, system_prompt: &str, max_context: usize) -> Value {
        let messages = self
            .inner
            .sessions
            .lock()
            .expect("assistant sessions lock poisoned")
            .get(session_id)
            .map(|session| session.messages.clone())
            .unwrap_or_default();
        serde_json::to_value(estimate_context(system_prompt, &messages, max_context))
            .unwrap_or_else(|_| json!({}))
    }

    fn emit(&self, event: AssistantEvent) {
        self.inner.emitter.emit(&event);
    }
}

async fn execute_tool(
    name: &str,
    arguments: Value,
    market_data_service: &MarketDataService,
    portfolio_service: &PortfolioService,
    paper_service: &PaperService,
    recommendation_service: &RecommendationService,
    settings_service: &SettingsService,
    signal_service: &SignalService,
    financial_report_service: &FinancialReportService,
) -> anyhow::Result<String> {
    let enabled_exchanges = settings_service.enabled_exchanges();
    match name {
        "market_data" => {
            let symbol =
                resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges);
            let limit = optional_usize(&arguments, "limit").unwrap_or(5).clamp(1, 10);
            let mut rows = market_data_service.cached_market_rows_for_exchanges(&enabled_exchanges);
            if let Some(symbol) = symbol.as_deref() {
                rows.retain(|row| row.symbol.eq_ignore_ascii_case(symbol));
            }
            rows.truncate(limit);
            let rows = rows.iter().map(a_share_market_row_json).collect::<Vec<_>>();
            Ok(json!({
                "ok": !rows.is_empty(),
                "message": if rows.is_empty() {
                    "没有可用的自选股缓存行情。请先打开行情页或刷新自选股行情。"
                } else {
                    "已读取自选股缓存行情。"
                },
                "rows": rows,
            })
            .to_string())
        }
        "stock_info" => {
            let symbol = resolved_symbol(
                optional_stock_code(&arguments),
                market_data_service,
                &enabled_exchanges,
            )
                .or_else(|| optional_stock_code(&arguments))
                .ok_or_else(|| anyhow!("stock_info requires stockCode"))?;
            let info = akshare::fetch_stock_info(&symbol)?;
            Ok(json!({
                "ok": true,
                "stockCode": symbol,
                "stockInfo": info,
            })
            .to_string())
        }
        "recommendation_history" => {
            let symbol =
                resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges);
            let latest_only = optional_bool(&arguments, "latestOnly").unwrap_or(false);
            let history = recommendation_service.list_history_snapshot(10).await?;
            let mut filtered = filter_recommendation_history(history, symbol.as_deref());
            let latest = resolve_recommendation_for_symbol(
                recommendation_service,
                symbol.as_deref(),
                filtered.first(),
            )
            .await?;
            if latest_only {
                filtered.truncate(1);
            }
            let rows = filtered.iter().map(recommendation_history_json).collect::<Vec<_>>();
            let latest = latest.as_ref().map(recommendation_run_json);
            Ok(json!({
                "ok": latest.is_some() || !rows.is_empty(),
                "latest": latest,
                "rows": rows,
            })
            .to_string())
        }
        "risk_calculator" => {
            let symbol =
                resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges);
            let history_hint = if let Some(symbol) = symbol.as_deref() {
                let history = recommendation_service.list_history_snapshot(10).await?;
                filter_recommendation_history(history, Some(symbol))
                    .into_iter()
                    .next()
            } else {
                None
            };
            let latest = resolve_recommendation_for_symbol(
                recommendation_service,
                symbol.as_deref(),
                history_hint.as_ref(),
            )
            .await?
            .ok_or_else(|| {
                symbol.as_ref().map_or_else(
                    || anyhow!("No recommendation is available for risk review."),
                    |symbol| anyhow!("No recommendation is available for risk review for {symbol}."),
                )
            })?;
            Ok(json!({
                "ok": true,
                "recommendationId": latest.recommendation_id,
                "stockCode": latest.symbol,
                "direction": latest.direction,
                "confidenceScore": latest.confidence_score,
                "riskStatus": latest.risk_status,
                "riskDetails": latest.risk_details,
                "stopLoss": latest.stop_loss,
                "maxLossCny": latest.max_loss_cny,
                "invalidation": latest.invalidation,
                "generatedAt": latest.generated_at,
            })
            .to_string())
        }
        "portfolio" => Ok(json!({
            "ok": true,
            "accountMode": settings_service.get_runtime_settings().account_mode,
            "paperAccounts": paper_service.list_accounts_snapshot(),
            "paperPositions": paper_service.list_positions_snapshot(),
            "paperOrders": paper_service.list_orders_snapshot(),
            "message": "Portfolio tool returns the safe local paper snapshot. Real-account live sync is not pulled from Assistant."
        })
        .to_string()),
        "positions" => {
            let symbol =
                resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges);
            let exchange = optional_string(&arguments, "account");
            let paper_only = optional_bool(&arguments, "paperOnly").unwrap_or(true);
            let (mut positions, mut orders) = if paper_only {
                (
                    paper_service.list_positions_snapshot(),
                    paper_service.list_orders_snapshot(),
                )
            } else {
                (
                    portfolio_service
                        .list_positions(market_data_service, settings_service)
                        .await?,
                    portfolio_service.list_orders(settings_service).await?,
                )
            };

            if let Some(symbol) = symbol.as_deref() {
                positions.retain(|item| item.symbol.eq_ignore_ascii_case(symbol));
                orders.retain(|item| item.symbol.eq_ignore_ascii_case(symbol));
            }
            if let Some(exchange) = exchange.as_deref() {
                positions.retain(|item| matches_exchange_name(&item.exchange, exchange));
                orders.retain(|item| matches_exchange_name(&item.exchange, exchange));
            }

            Ok(json!({
                "ok": !positions.is_empty() || !orders.is_empty(),
                "positions": positions.iter().map(position_json).collect::<Vec<_>>(),
                "orders": orders.iter().map(order_json).collect::<Vec<_>>(),
                "message": if positions.is_empty() && orders.is_empty() {
                    "没有找到匹配的模拟持仓或委托。"
                } else {
                    "已读取匹配的模拟持仓和委托。"
                }
            })
            .to_string())
        }
        "paper_order_draft" => {
            let latest = recommendation_service
                .get_latest()
                .await?
                .into_iter()
                .find(|run| run.has_trade)
                .ok_or_else(|| anyhow!("No recommendation available for paper draft creation."))?;
            if !latest.has_trade {
                bail!("The latest recommendation is a no-trade result.");
            }
            let exchange = optional_string(&arguments, "account")
                .or_else(|| latest.exchanges.first().cloned())
                .unwrap_or_else(|| "akshare".into());
            let draft = paper_service
                .create_draft_from_recommendation(&latest, &paper_account_id(&exchange))
                .await?;
            Ok(json!({
                "ok": true,
                "draft": draft,
                "sourceRecommendationId": latest.recommendation_id,
                "generatedAt": latest.generated_at,
            })
            .to_string())
        }
        "signal_scan" => {
            let symbol = resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges);
            let runtime = settings_service.get_runtime_settings();
            let account_equity_usdt = 10_000.0;

            if let Some(sym) = symbol.as_deref() {
                match signal_service
                    .scan_single(
                        sym,
                        "ashare",
                        &enabled_exchanges,
                        market_data_service,
                        &runtime,
                        account_equity_usdt,
                    )
                    .await
                {
                    Ok((strategy_signals, unified)) => {
                        Ok(json!({
                            "ok": true,
                            "stockCode": sym,
                            "unifiedSignal": unified.as_ref().map(unified_signal_json),
                            "strategySignals": strategy_signals,
                            "message": if unified.is_some() {
                                "A 股信号扫描完成，触发统一信号。"
                            } else {
                                "A 股信号扫描完成，未触发统一信号，已返回原始策略信号。"
                            }
                        })
                        .to_string())
                    }
                    Err(e) => Ok(json!({
                        "ok": false,
                        "stockCode": sym,
                        "message": format!("A 股信号扫描失败: {e}"),
                        "strategySignals": []
                    })
                    .to_string()),
                }
            } else {
                match signal_service
                    .scan_all_with_strategy_signals(
                        &enabled_exchanges,
                        market_data_service,
                        &runtime,
                        account_equity_usdt,
                    )
                    .await
                {
                    Ok(outcome) => Ok(json!({
                        "ok": true,
                        "message": format!("A 股信号扫描完成，发现 {} 条信号。", outcome.signals.len()),
                        "signals": outcome.signals.iter().map(unified_signal_json).collect::<Vec<_>>(),
                        "strategySignalSets": outcome.strategy_signal_sets.iter().map(strategy_signal_set_json).collect::<Vec<_>>(),
                    })
                    .to_string()),
                    Err(e) => Ok(json!({
                        "ok": false,
                        "message": format!("A 股信号扫描失败: {e}"),
                        "signals": [],
                        "strategySignalSets": []
                    })
                    .to_string()),
                }
            }
        }
        "bid_ask" => {
            let symbol = resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges)
                .or_else(|| optional_stock_code(&arguments))
                .ok_or_else(|| anyhow!("bid_ask requires stockCode"))?;
            Ok(json!({
                "ok": true,
                "stockCode": symbol,
                "bidAsk": akshare::fetch_bid_ask(&symbol)?,
            })
            .to_string())
        }
        "kline_data" => {
            let symbol = resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges)
                .or_else(|| optional_stock_code(&arguments))
                .ok_or_else(|| anyhow!("kline_data requires stockCode"))?;
            let count = optional_usize(&arguments, "count").unwrap_or(60).clamp(1, 120);
            Ok(json!({
                "ok": true,
                "stockCode": symbol,
                "klines": akshare::fetch_multi_frequency_bars(&symbol, count)?,
            })
            .to_string())
        }
        "financial_report_info" => {
            if !settings_service.get_runtime_settings().use_financial_report_data {
                return Ok(json!({
                    "ok": false,
                    "message": "设置中尚未启用使用财报数据。请先在设置的 AI交易 中开启使用财报数据。"
                })
                .to_string());
            }
            let symbol = resolved_symbol(optional_stock_code(&arguments), market_data_service, &enabled_exchanges)
                .or_else(|| optional_stock_code(&arguments))
                .ok_or_else(|| anyhow!("financial_report_info requires stockCode"))?;
            let snapshot = financial_report_service.snapshot(&symbol)?;
            match snapshot.analysis {
                Some(analysis) => Ok(json!({
                    "ok": true,
                    "stockCode": analysis.stock_code,
                    "analysis": {
                        "关键信息总结": analysis.key_summary,
                        "财报正向因素": analysis.positive_factors,
                        "财报负向因素": analysis.negative_factors,
                        "财报造假嫌疑点": analysis.fraud_risk_points
                    },
                    "rawSections": snapshot.sections,
                    "sourceRevision": analysis.source_revision,
                    "generatedAt": analysis.generated_at,
                    "message": "已读取本地缓存的财报原始数据和 AI 分析结论。"
                })
                .to_string()),
                None => Ok(json!({
                    "ok": false,
                    "stockCode": symbol,
                    "rawSections": snapshot.sections,
                    "message": "暂无该股票的财报 AI 分析缓存。请先在财报分析页面拉取全量财报并分析自选股票池。"
                })
                .to_string()),
            }
        }
        unknown => bail!("unsupported assistant tool: {unknown}"),
    }
}

fn openai_messages(system_prompt: &str, conversation: &[ConversationItem]) -> Vec<Value> {
    let mut messages = vec![json!({ "role": "system", "content": system_prompt })];
    for item in conversation {
        match item {
            ConversationItem::User(content) => {
                messages.push(json!({ "role": "user", "content": content }));
            }
            ConversationItem::AssistantText(content) => {
                messages.push(json!({ "role": "assistant", "content": content }));
            }
            ConversationItem::AssistantToolCalls {
                content,
                tool_calls,
            } => {
                let tool_calls = tool_calls
                    .iter()
                    .map(|tool_call| {
                        json!({
                            "id": tool_call.id,
                            "type": "function",
                            "function": {
                                "name": tool_call.name,
                                "arguments": tool_call.arguments.to_string(),
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                messages.push(json!({
                    "role": "assistant",
                    "content": if content.trim().is_empty() { Value::Null } else { Value::String(content.clone()) },
                    "tool_calls": tool_calls,
                }));
            }
            ConversationItem::ToolResult {
                tool_call_id,
                content,
            } => {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": content,
                }));
            }
        }
    }
    messages
}

fn anthropic_messages(conversation: &[ConversationItem]) -> Vec<Value> {
    let mut messages = Vec::new();
    for item in conversation {
        match item {
            ConversationItem::User(content) => {
                messages.push(json!({ "role": "user", "content": content }));
            }
            ConversationItem::AssistantText(content) => {
                messages.push(json!({ "role": "assistant", "content": content }));
            }
            ConversationItem::AssistantToolCalls {
                content,
                tool_calls,
            } => {
                let mut blocks = Vec::new();
                if !content.trim().is_empty() {
                    blocks.push(json!({
                        "type": "text",
                        "text": content,
                    }));
                }
                for tool_call in tool_calls {
                    blocks.push(json!({
                        "type": "tool_use",
                        "id": tool_call.id,
                        "name": tool_call.name,
                        "input": tool_call.arguments,
                    }));
                }
                messages.push(json!({
                    "role": "assistant",
                    "content": blocks,
                }));
            }
            ConversationItem::ToolResult {
                tool_call_id,
                content,
            } => {
                messages.push(json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": tool_call_id,
                        "content": content,
                    }],
                }));
            }
        }
    }
    messages
}

fn assistant_system_prompt(settings_service: &SettingsService) -> String {
    let runtime = settings_service.get_runtime_settings();
    if !runtime.assistant_system_prompt.trim().is_empty() {
        return format!(
            "{} 可用工具: {}.",
            runtime.assistant_system_prompt.trim(),
            tools::allowed_tools().join(", ")
        );
    }

    let mut prompt = format!(
        "你是 KittyRed Assistant，只服务沪深 A 股和本地模拟投资。需要行情、个股资料、盘口、K 线、组合、持仓、建议或风险事实时必须调用工具，不要猜测。\
用简洁中文 Markdown 回答。\
如果缓存行情不可用，要明确说明并建议用户刷新自选股行情，不要编造实时行情。\
只有用户明确要求创建模拟委托草稿时，才调用 paper_order_draft。\
可用工具: {}.",
        tools::allowed_tools().join(", ")
    );

    let extension = settings_service.prompt_extension();
    if !extension.trim().is_empty() {
        prompt.push_str(" 用户补充偏好: ");
        prompt.push_str(extension.trim());
    }

    prompt
}

fn a_share_market_row_json(row: &crate::models::MarketListRow) -> Value {
    json!({
        "stockCode": row.symbol,
        "stockName": row.base_asset,
        "lastPrice": row.last_price,
        "changePercent": row.change_24h,
        "volume": row.volume_24h,
        "source": row.exchanges.first().cloned().unwrap_or_else(|| "akshare".into()),
        "updatedAt": row.updated_at,
        "stale": row.stale,
    })
}

fn position_json(position: &crate::models::PositionDto) -> Value {
    json!({
        "account": position.exchange,
        "stockCode": position.symbol,
        "side": position.side,
        "quantity": position.size,
        "entryPrice": position.entry_price,
        "markPrice": position.mark_price,
        "pnlPercent": position.pnl_percent,
        "leverage": position.leverage,
    })
}

fn order_json(order: &crate::models::PaperOrderRowDto) -> Value {
    json!({
        "orderId": order.order_id,
        "account": order.exchange,
        "stockCode": order.symbol,
        "orderType": order.order_type,
        "status": order.status,
        "quantity": order.quantity,
        "estimatedFillPrice": order.estimated_fill_price,
        "realizedPnlCny": order.realized_pnl_usdt,
        "updatedAt": order.updated_at,
    })
}

fn recommendation_run_json(run: &RecommendationRunDto) -> Value {
    json!({
        "recommendationId": run.recommendation_id,
        "status": run.status,
        "triggerType": run.trigger_type,
        "hasTrade": run.has_trade,
        "stockCode": run.symbol,
        "direction": run.direction,
        "accounts": run.exchanges,
        "confidenceScore": run.confidence_score,
        "rationale": run.rationale,
        "riskStatus": run.risk_status,
        "entryLow": run.entry_low,
        "entryHigh": run.entry_high,
        "stopLoss": run.stop_loss,
        "takeProfit": run.take_profit,
        "amountCny": run.amount_cny,
        "maxLossCny": run.max_loss_cny,
        "invalidation": run.invalidation,
        "noTradeReason": run.no_trade_reason,
        "riskDetails": run.risk_details,
        "generatedAt": run.generated_at,
    })
}

fn recommendation_history_json(row: &crate::models::RecommendationHistoryRowDto) -> Value {
    json!({
        "recommendationId": row.recommendation_id,
        "createdAt": row.created_at,
        "triggerType": row.trigger_type,
        "stockCode": row.symbol,
        "shortlist": row.shortlist,
        "account": row.exchange,
        "direction": row.direction,
        "riskStatus": row.risk_status,
        "result": row.result,
        "entryLow": row.entry_low,
        "entryHigh": row.entry_high,
        "stopLoss": row.stop_loss,
        "takeProfit": row.take_profit,
        "confidenceScore": row.confidence_score,
        "executed": row.executed,
        "modified": row.modified,
        "outcome": row.outcome,
    })
}

fn unified_signal_json(signal: &crate::signals::UnifiedSignal) -> Value {
    json!({
        "signalId": signal.signal_id,
        "stockCode": signal.symbol,
        "direction": signal.direction,
        "score": signal.score,
        "strength": signal.strength,
        "categoryBreakdown": signal.category_breakdown,
        "contributors": signal.contributors,
        "entryZoneLow": signal.entry_zone_low,
        "entryZoneHigh": signal.entry_zone_high,
        "stopLoss": signal.stop_loss,
        "takeProfit": signal.take_profit,
        "reasonSummary": signal.reason_summary,
        "riskStatus": signal.risk_status,
        "generatedAt": signal.generated_at,
    })
}

fn strategy_signal_set_json(set: &crate::signals::StrategySignalSet) -> Value {
    json!({
        "stockCode": set.symbol,
        "strategySignals": set.strategy_signals,
    })
}

fn optional_string(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn optional_stock_code(arguments: &Value) -> Option<String> {
    optional_string(arguments, "stockCode")
}

fn optional_usize(arguments: &Value, key: &str) -> Option<usize> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as usize)
}

fn optional_bool(arguments: &Value, key: &str) -> Option<bool> {
    arguments.get(key).and_then(Value::as_bool)
}

fn preview_tool_result(result: &str) -> String {
    let trimmed = result.trim();
    if trimmed.len() <= 220 {
        trimmed.to_string()
    } else {
        format!("{}...", &trimmed[..220])
    }
}

fn summarize_tool_call(name: &str, arguments: &Value) -> String {
    match name {
        "market_data" => {
            let symbol = optional_stock_code(arguments).unwrap_or_else(|| "自选股缓存".into());
            format!("读取 {symbol} 的缓存行情")
        }
        "stock_info" => {
            let symbol = optional_stock_code(arguments).unwrap_or_else(|| "指定股票".into());
            format!("读取 {symbol} 的个股资料")
        }
        "bid_ask" => {
            let symbol = optional_stock_code(arguments).unwrap_or_else(|| "指定股票".into());
            format!("读取 {symbol} 的五档盘口")
        }
        "kline_data" => {
            let symbol = optional_stock_code(arguments).unwrap_or_else(|| "指定股票".into());
            format!("读取 {symbol} 的多周期 K 线")
        }
        "recommendation_history" => "读取投资建议历史".into(),
        "risk_calculator" => "汇总投资建议风险".into(),
        "portfolio" => "读取本地模拟组合".into(),
        "positions" => "读取模拟持仓和委托".into(),
        "paper_order_draft" => {
            let account =
                optional_string(arguments, "account").unwrap_or_else(|| "默认模拟账户".into());
            format!("创建 {account} 的模拟委托草稿")
        }
        _ => format!("Run {name}"),
    }
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

fn extract_symbol(message: &str, available_symbols: &[String]) -> Option<String> {
    let lower = message.to_lowercase();
    let tokens = lower
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    for symbol in available_symbols {
        let symbol_lower = symbol.to_lowercase();
        let base = symbol_lower.split('/').next().unwrap_or_default();
        let stock_code = symbol_lower.split('.').next_back().unwrap_or_default();

        if lower.contains(&symbol_lower)
            || tokens.iter().any(|token| *token == base)
            || tokens.iter().any(|token| *token == stock_code)
        {
            return Some(symbol.clone());
        }
    }

    None
}

fn resolved_symbol(
    requested_symbol: Option<String>,
    market_data_service: &MarketDataService,
    enabled_exchanges: &[String],
) -> Option<String> {
    let requested_symbol = requested_symbol?;
    let available_symbols = market_data_service
        .cached_market_rows_for_exchanges(enabled_exchanges)
        .into_iter()
        .map(|row| row.symbol)
        .collect::<Vec<_>>();

    extract_symbol(&requested_symbol, &available_symbols).or(Some(requested_symbol))
}

fn filter_recommendation_history(
    history: Vec<crate::models::RecommendationHistoryRowDto>,
    symbol: Option<&str>,
) -> Vec<crate::models::RecommendationHistoryRowDto> {
    if let Some(symbol) = symbol {
        history
            .into_iter()
            .filter(|row| row.symbol.eq_ignore_ascii_case(symbol))
            .collect()
    } else {
        history
    }
}

async fn resolve_recommendation_for_symbol(
    recommendation_service: &RecommendationService,
    symbol: Option<&str>,
    history_hint: Option<&crate::models::RecommendationHistoryRowDto>,
) -> anyhow::Result<Option<RecommendationRunDto>> {
    let latest = recommendation_service.get_latest().await?;
    let Some(symbol) = symbol else {
        return Ok(latest.into_iter().next());
    };

    if latest.iter().any(|run| {
        run.symbol
            .as_deref()
            .is_some_and(|item| item.eq_ignore_ascii_case(symbol))
    }) {
        return Ok(latest.into_iter().find(|run| {
            run.symbol
                .as_deref()
                .is_some_and(|item| item.eq_ignore_ascii_case(symbol))
        }));
    }

    if let Some(history_row) = history_hint {
        return recommendation_service
            .resolve_recommendation(&history_row.recommendation_id)
            .await;
    }

    Ok(None)
}

fn normalized_exchange_name(exchange: &str) -> String {
    exchange
        .strip_prefix("Paper: ")
        .unwrap_or(exchange)
        .to_string()
}

fn matches_exchange_name(exchange: &str, requested_exchange: &str) -> bool {
    normalized_exchange_name(exchange).eq_ignore_ascii_case(requested_exchange)
}
