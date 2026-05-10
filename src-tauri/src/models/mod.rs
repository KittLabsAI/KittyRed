#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeCredentialSummary {
    pub exchange: String,
    pub status: String,
    pub permission_read: bool,
    pub permission_trade: bool,
    pub permission_withdraw: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExchangeSettingsDto {
    pub exchange: String,
    pub enabled: bool,
    pub has_stored_api_key: bool,
    pub has_stored_api_secret: bool,
    pub has_stored_extra_passphrase: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeNotificationSettingsDto {
    pub recommendations: bool,
    pub spreads: bool,
    pub paper_orders: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettingsDto {
    pub exchanges: Vec<RuntimeExchangeSettingsDto>,
    pub model_provider: String,
    pub model_name: String,
    pub model_base_url: String,
    pub model_temperature: f64,
    pub model_max_tokens: u32,
    pub model_max_context: u32,
    pub has_stored_model_api_key: bool,
    pub auto_analyze_enabled: bool,
    pub auto_analyze_frequency: String,
    pub scan_scope: String,
    pub watchlist_symbols: Vec<String>,
    pub daily_max_ai_calls: u32,
    #[serde(default = "default_use_bid_ask_data")]
    pub use_bid_ask_data: bool,
    #[serde(default)]
    pub use_financial_report_data: bool,
    #[serde(default = "default_ai_kline_bar_count")]
    pub ai_kline_bar_count: u32,
    #[serde(default = "default_ai_kline_frequencies")]
    pub ai_kline_frequencies: Vec<String>,
    pub pause_after_consecutive_losses: u32,
    pub min_confidence_score: f64,
    pub allowed_markets: String,
    pub allowed_direction: String,
    pub max_leverage: f64,
    pub max_loss_per_trade_percent: f64,
    pub max_daily_loss_percent: f64,
    pub min_risk_reward_ratio: f64,
    pub min_volume_24h: f64,
    pub max_spread_bps: f64,
    pub allow_meme_coins: bool,
    pub whitelist_symbols: Vec<String>,
    pub blacklist_symbols: Vec<String>,
    pub prompt_extension: String,
    #[serde(default = "default_assistant_system_prompt")]
    pub assistant_system_prompt: String,
    #[serde(default = "default_recommendation_system_prompt")]
    pub recommendation_system_prompt: String,
    pub account_mode: String,
    pub auto_paper_execution: bool,
    pub notifications: RuntimeNotificationSettingsDto,
    #[serde(default)]
    pub signals_enabled: bool,
    #[serde(default = "default_signal_scan_frequency")]
    pub signal_scan_frequency: String,
    #[serde(default = "default_signal_min_score")]
    pub signal_min_score: f64,
    #[serde(default = "default_signal_cooldown_minutes")]
    pub signal_cooldown_minutes: u32,
    #[serde(default = "default_signal_daily_max")]
    pub signal_daily_max: u32,
    #[serde(default)]
    pub signal_auto_execute: bool,
    #[serde(default)]
    pub signal_notifications: bool,
    #[serde(default)]
    pub signal_watchlist_symbols: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeExchangeSecretDto {
    pub exchange: String,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub extra_passphrase: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSecretsSyncDto {
    pub persist: bool,
    pub model_api_key: Option<String>,
    pub exchanges: Vec<RuntimeExchangeSecretDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBacktestDatasetRequestDto {
    pub name: String,
    pub symbols: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub interval_minutes: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestDatasetDto {
    pub dataset_id: String,
    pub name: String,
    pub status: String,
    pub symbols: Vec<String>,
    pub start_date: String,
    pub end_date: String,
    pub interval_minutes: u32,
    pub total_snapshots: u32,
    pub fetched_count: u32,
    pub estimated_llm_calls: u32,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestFetchFailureDto {
    pub failure_id: String,
    pub dataset_id: String,
    pub symbol: String,
    pub captured_at: Option<String>,
    pub timeframe: String,
    pub stage: String,
    pub reason: String,
    pub error_detail: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestFetchProgressDto {
    pub dataset_id: String,
    pub status: String,
    pub total_snapshots: u32,
    pub fetched_count: u32,
    pub failure_count: u32,
    pub error_message: Option<String>,
    pub recent_failures: Vec<BacktestFetchFailureDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportRowDto {
    #[serde(alias = "stock_code")]
    pub stock_code: String,
    #[serde(alias = "report_date")]
    pub report_date: Option<String>,
    #[serde(alias = "stock_name")]
    pub stock_name: Option<String>,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportSectionDto {
    pub section: String,
    pub label: String,
    pub source: String,
    pub rows: Vec<FinancialReportRowDto>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportSnapshotDto {
    pub stock_code: String,
    pub stock_name: Option<String>,
    pub sections: Vec<FinancialReportSectionDto>,
    pub source_revision: String,
    pub refreshed_at: Option<String>,
    pub metric_series: Vec<FinancialReportMetricSeriesDto>,
    pub analysis: Option<FinancialReportAnalysisDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportSectionSummaryDto {
    pub section: String,
    pub label: String,
    pub source: String,
    pub row_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportOverviewDto {
    pub stock_count: u32,
    pub row_count: u32,
    pub refreshed_at: Option<String>,
    pub sections: Vec<FinancialReportSectionSummaryDto>,
    pub analyses: Vec<FinancialReportAnalysisDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportAnalysisDto {
    pub stock_code: String,
    pub stock_name: Option<String>,
    pub financial_score: u32,
    pub category_scores: FinancialReportCategoryScoresDto,
    pub radar_scores: FinancialReportRadarScoresDto,
    pub source_revision: String,
    pub key_summary: String,
    pub positive_factors: String,
    pub negative_factors: String,
    pub fraud_risk_points: String,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub generated_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportCategoryScoresDto {
    pub revenue_quality: u32,
    pub gross_margin: u32,
    pub net_profit_return: u32,
    pub earnings_manipulation: u32,
    pub solvency: u32,
    pub cash_flow: u32,
    pub growth: u32,
    pub research_capital: u32,
    pub operating_efficiency: u32,
    pub asset_quality: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportRadarScoresDto {
    pub profitability: f64,
    pub authenticity: f64,
    pub cash_generation: f64,
    pub safety: f64,
    pub growth_potential: f64,
    pub operating_efficiency: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct AiFinancialReportContextDto {
    pub key_summary: String,
    pub positive_factors: String,
    pub negative_factors: String,
    pub fraud_risk_points: String,
    pub radar_scores: FinancialReportRadarScoresDto,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportAnalysisProgressItemDto {
    pub stock_code: String,
    pub short_name: String,
    pub status: String,
    pub attempt: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportAnalysisProgressDto {
    pub status: String,
    pub completed_count: u32,
    pub total_count: u32,
    pub message: String,
    pub items: Vec<FinancialReportAnalysisProgressItemDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportMetricPointDto {
    pub report_date: String,
    pub value: f64,
    pub yoy: Option<f64>,
    pub qoq: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportMetricSeriesDto {
    pub metric_key: String,
    pub metric_label: String,
    pub unit: String,
    pub points: Vec<FinancialReportMetricPointDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialReportFetchProgressDto {
    pub stock_code: String,
    pub status: String,
    pub completed_sections: u32,
    pub total_sections: u32,
    pub message: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBacktestRequestDto {
    pub dataset_id: String,
    pub name: String,
    pub max_holding_days: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRunDto {
    pub backtest_id: String,
    pub dataset_id: String,
    pub name: String,
    pub status: String,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub max_holding_days: u32,
    pub total_ai_calls: u32,
    pub processed_ai_calls: u32,
    pub total_timepoints: u32,
    pub processed_timepoints: u32,
    pub total_signals: u32,
    pub trade_signals: u32,
    pub open_trades: u32,
    pub win_count: u32,
    pub loss_count: u32,
    pub flat_count: u32,
    pub total_pnl_cny: f64,
    pub total_pnl_percent: f64,
    pub max_drawdown_percent: f64,
    pub profit_factor: Option<f64>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestSignalDto {
    pub signal_id: String,
    pub backtest_id: String,
    pub symbol: String,
    pub stock_name: Option<String>,
    pub captured_at: String,
    pub has_trade: bool,
    pub direction: Option<String>,
    pub confidence_score: Option<f64>,
    pub risk_status: Option<String>,
    pub entry_low: Option<f64>,
    pub entry_high: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<String>,
    pub amount_cny: Option<f64>,
    pub max_loss_cny: Option<f64>,
    pub rationale: Option<String>,
    pub result: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestTradeDto {
    pub trade_id: String,
    pub backtest_id: String,
    pub signal_id: Option<String>,
    pub symbol: String,
    pub stock_name: Option<String>,
    pub direction: String,
    pub entry_price: f64,
    pub entry_at: String,
    pub exit_price: f64,
    pub exit_at: String,
    pub exit_reason: String,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub amount_cny: Option<f64>,
    pub holding_periods: u32,
    pub pnl_cny: f64,
    pub pnl_percent: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestEquityPointDto {
    pub captured_at: String,
    pub cumulative_pnl_percent: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestOpenPositionDto {
    pub signal_id: String,
    pub symbol: String,
    pub stock_name: Option<String>,
    pub entry_price: f64,
    pub entry_at: String,
    pub mark_price: f64,
    pub amount_cny: f64,
    pub holding_periods: u32,
    pub unrealized_pnl_cny: f64,
    pub unrealized_pnl_percent: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestSummaryDto {
    pub backtest_id: String,
    pub total_signals: u32,
    pub trade_count: u32,
    pub win_rate: f64,
    pub total_pnl_cny: f64,
    pub total_pnl_percent: f64,
    pub max_drawdown_percent: f64,
    pub profit_factor: Option<f64>,
    pub equity_curve: Vec<BacktestEquityPointDto>,
    pub open_positions: Vec<BacktestOpenPositionDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelConnectionTestPayloadDto {
    pub model_provider: String,
    pub model_name: String,
    pub model_base_url: String,
    pub model_api_key: String,
    pub model_temperature: f64,
    pub model_max_tokens: u32,
    pub model_max_context: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestResultDto {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeConnectionTestPayloadDto {
    pub exchange: String,
    pub api_key: String,
    pub api_secret: String,
    pub extra_passphrase: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeConnectionTestResultDto {
    pub status: String,
    pub permission_read: bool,
    pub permission_trade: bool,
    pub permission_withdraw: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SettingsSnapshotDto {
    pub exchange_credentials: Vec<ExchangeCredentialSummary>,
    pub active_model_provider: String,
    pub model_name: String,
    pub notification_recommendations_enabled: bool,
    pub notification_spreads_enabled: bool,
    pub notification_paper_orders_enabled: bool,
    pub account_mode: String,
    pub risk_max_leverage: f64,
    pub prompt_profile: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationEventDto {
    pub event_id: String,
    pub channel: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobRecord {
    pub id: i64,
    pub kind: String,
    pub status: String,
    pub message: String,
    pub started_at: String,
    pub updated_at: String,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub input_params_json: Option<String>,
    pub result_summary: Option<String>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketSnapshot {
    pub exchange: String,
    pub symbol: String,
    pub market_type: String,
    pub last_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub volume_24h: f64,
    pub change_24h: f64,
    pub updated_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VenueTickerSnapshot {
    pub exchange: String,
    pub last_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub volume_24h: f64,
    pub funding_rate: Option<f64>,
    pub mark_price: Option<f64>,
    pub index_price: Option<f64>,
    pub updated_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketListRow {
    pub symbol: String,
    pub base_asset: String,
    pub market_type: String,
    pub market_cap_usd: Option<f64>,
    pub market_cap_rank: Option<i64>,
    pub market_size_tier: String,
    pub last_price: f64,
    pub change_24h: f64,
    pub volume_24h: f64,
    pub funding_rate: Option<f64>,
    pub spread_bps: f64,
    pub exchanges: Vec<String>,
    pub updated_at: String,
    pub stale: bool,
    pub venue_snapshots: Vec<VenueTickerSnapshot>,
    pub best_bid_exchange: Option<String>,
    pub best_ask_exchange: Option<String>,
    pub best_bid_price: Option<f64>,
    pub best_ask_price: Option<f64>,
    pub responded_exchange_count: u32,
    pub fdv_usd: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct AShareSymbolSearchResultDto {
    pub symbol: String,
    pub name: String,
    pub market: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderBookSnapshot {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderBookVenueSnapshot {
    pub exchange: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecentTrade {
    pub exchange: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OhlcvBar {
    pub open_time: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub turnover: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DerivativesSnapshot {
    pub funding_rate: f64,
    pub mark_price: f64,
    pub index_price: f64,
    pub open_interest: String,
    pub next_funding_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NormalizedInstrument {
    pub instrument_id: String,
    pub exchange: String,
    pub exchange_symbol: String,
    pub symbol_normalized: String,
    pub market_type: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub settle_asset: Option<String>,
    pub contract_size: Option<String>,
    pub tick_size: String,
    pub lot_size: String,
    pub min_notional: Option<String>,
    pub max_leverage: Option<i64>,
    pub status: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpreadOpportunityDto {
    pub symbol: String,
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub gross_spread_pct: f64,
    pub net_spread_pct: f64,
    pub funding_context: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageOpportunityDto {
    pub symbol: String,
    pub opportunity_type: String,
    pub primary_market_type: String,
    pub secondary_market_type: Option<String>,
    pub buy_exchange: String,
    pub buy_market_type: String,
    pub buy_price: f64,
    pub sell_exchange: String,
    pub sell_market_type: String,
    pub sell_price: f64,
    pub fee_adjusted_net_spread_pct: f64,
    pub simulated_carry_pct: f64,
    pub simulated_total_yield_pct: f64,
    pub liquidity_usdt_24h: f64,
    pub market_cap_usd: Option<f64>,
    pub funding_rate: Option<f64>,
    pub borrow_rate_daily: Option<f64>,
    pub recommendation_score: f64,
    pub updated_at: String,
    pub stale: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageOpportunityPageDto {
    pub items: Vec<ArbitrageOpportunityDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairVenueSnapshot {
    pub exchange: String,
    pub last_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub change_pct: f64,
    pub volume_24h: f64,
    pub funding_rate: Option<f64>,
    pub mark_price: Option<f64>,
    pub index_price: Option<f64>,
    pub open_interest: Option<String>,
    pub next_funding_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoinInfoDto {
    pub name: String,
    pub symbol: String,
    pub summary: String,
    pub website: Option<String>,
    pub whitepaper: Option<String>,
    pub explorer: Option<String>,
    pub ecosystem: String,
    pub market_cap: Option<String>,
    pub fdv: Option<String>,
    pub circulating_supply: Option<String>,
    pub total_supply: Option<String>,
    pub max_supply: Option<String>,
    pub volume_24h: Option<String>,
    pub listed_exchanges: Vec<String>,
    pub risk_tags: Vec<String>,
    pub github: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PairDetailDto {
    pub symbol: String,
    pub market_type: String,
    pub thesis: String,
    pub source_note: String,
    pub coin_info: CoinInfoDto,
    pub venues: Vec<PairVenueSnapshot>,
    pub orderbooks: Vec<OrderBookVenueSnapshot>,
    pub recent_trades: Vec<RecentTrade>,
    pub spreads: Vec<SpreadOpportunityDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CandleSeriesDto {
    pub exchange: String,
    pub symbol: String,
    pub market_type: String,
    pub interval: String,
    pub bars: Vec<OhlcvBar>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExchangeEquity {
    pub exchange: String,
    pub equity_usdt: f64,
    pub change_percent: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortfolioOverviewDto {
    pub total_equity_usdt: f64,
    pub total_market_value_usdt: f64,
    pub total_pnl_usdt: f64,
    pub daily_pnl_usdt: f64,
    pub daily_pnl_percent: f64,
    pub account_mode: String,
    pub risk_summary: String,
    pub exchanges: Vec<ExchangeEquity>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RiskCheckDto {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RiskDecisionDto {
    pub status: String,
    pub risk_score: u32,
    pub max_loss_estimate: Option<String>,
    pub checks: Vec<RiskCheckDto>,
    pub modifications: Vec<String>,
    pub block_reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PositionDto {
    pub position_id: String,
    pub account_id: String,
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub size: String,
    pub entry_price: f64,
    pub mark_price: f64,
    pub pnl_percent: f64,
    pub leverage: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolRecommendationDto {
    pub symbol: String,
    #[serde(default)]
    pub stock_name: Option<String>,
    pub direction: Option<String>,
    pub rationale: String,
    pub risk_status: String,
    pub has_trade: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecommendationRunDto {
    pub recommendation_id: String,
    pub status: String,
    pub trigger_type: String,
    pub has_trade: bool,
    pub symbol: Option<String>,
    #[serde(default)]
    pub stock_name: Option<String>,
    pub direction: Option<String>,
    pub market_type: String,
    pub exchanges: Vec<String>,
    pub confidence_score: f64,
    pub rationale: String,
    #[serde(default)]
    pub symbol_recommendations: Vec<SymbolRecommendationDto>,
    pub risk_status: String,
    pub entry_low: Option<f64>,
    pub entry_high: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<String>,
    pub leverage: Option<f64>,
    pub amount_cny: Option<f64>,
    pub invalidation: Option<String>,
    pub max_loss_cny: Option<f64>,
    pub no_trade_reason: Option<String>,
    #[serde(default)]
    pub risk_details: RiskDecisionDto,
    pub data_snapshot_at: String,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub user_preference_version: String,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecommendationHistoryRowDto {
    pub recommendation_id: String,
    pub created_at: String,
    pub trigger_type: String,
    pub symbol: String,
    pub stock_name: String,
    pub shortlist: Vec<String>,
    pub exchange: String,
    pub market_type: String,
    pub direction: String,
    pub rationale: String,
    pub risk_status: String,
    pub result: String,
    pub entry_low: Option<f64>,
    pub entry_high: Option<f64>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<String>,
    pub leverage: Option<f64>,
    pub amount_cny: Option<f64>,
    pub confidence_score: f64,
    pub model_name: String,
    pub prompt_version: String,
    pub executed: bool,
    pub modified: bool,
    pub pnl_5m: f64,
    pub pnl_10m: f64,
    pub pnl_30m: f64,
    pub pnl_60m: f64,
    pub pnl_24h: f64,
    pub pnl_7d: f64,
    pub outcome: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecommendationAuditDto {
    pub recommendation_id: String,
    pub trigger_type: String,
    pub symbol: String,
    pub exchange: String,
    pub market_type: String,
    pub created_at: String,
    pub model_provider: String,
    pub model_name: String,
    pub prompt_version: String,
    pub user_preference_version: String,
    pub ai_raw_output: String,
    pub ai_structured_output: String,
    pub risk_result: String,
    pub market_snapshot: String,
    pub account_snapshot: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationGenerationProgressItemDto {
    pub stock_code: String,
    pub short_name: String,
    pub status: String,
    pub attempt: u32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationGenerationProgressDto {
    pub status: String,
    pub completed_count: u32,
    pub total_count: u32,
    pub message: String,
    pub items: Vec<RecommendationGenerationProgressItemDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaperOrderDraftDto {
    pub order_id: String,
    pub account_id: String,
    pub exchange: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub estimated_fill_price: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManualPaperOrderRequestDto {
    pub account_id: String,
    pub symbol: String,
    pub market_type: String,
    pub side: String,
    pub quantity: f64,
    pub entry_price: Option<f64>,
    pub leverage: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaperAccountDto {
    pub account_id: String,
    pub exchange: String,
    pub available_usdt: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaperOrderRowDto {
    pub order_id: String,
    pub exchange: String,
    pub symbol: String,
    pub order_type: String,
    pub status: String,
    pub quantity: String,
    pub estimated_fill_price: f64,
    pub realized_pnl_usdt: Option<f64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantMessageDto {
    pub id: String,
    pub role: String,
    pub content: String,
    pub tools_used: Vec<String>,
    pub cited_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantRunDto {
    pub assistant_session_id: String,
    pub answer: String,
    pub tools_used: Vec<String>,
    pub cited_at: String,
    pub messages: Vec<AssistantMessageDto>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSignalDto {
    pub signal_id: String,
    pub symbol: String,
    pub market_type: String,
    pub direction: String,
    pub score: f64,
    pub strength: f64,
    pub category_breakdown: std::collections::HashMap<String, f64>,
    pub contributors: Vec<String>,
    pub entry_zone_low: f64,
    pub entry_zone_high: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub reason_summary: String,
    pub risk_status: String,
    pub risk_result: Option<RiskDecisionDto>,
    pub executed: bool,
    pub modified: bool,
    pub generated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalHistoryPageDto {
    pub items: Vec<UnifiedSignalDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyMetaDto {
    pub strategy_id: String,
    pub name: String,
    pub category: String,
    pub applicable_markets: Vec<String>,
    pub description: String,
    pub default_params: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyConfigDto {
    pub strategy_id: String,
    pub enabled: bool,
    pub params: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyStatsDto {
    pub strategy_id: String,
    pub total_signals: u32,
    pub buy_count: u32,
    pub sell_count: u32,
    pub neutral_count: u32,
    pub avg_score: f64,
    pub last_generated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunRowDto {
    pub id: u32,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub symbols_scanned: u32,
    pub signals_found: u32,
    pub duration_ms: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunHistoryPageDto {
    pub items: Vec<ScanRunRowDto>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
}

fn default_signal_scan_frequency() -> String {
    "15m".into()
}

pub fn default_assistant_system_prompt() -> String {
    "你是 KittyRed Assistant，只服务沪深 A 股和本地模拟投资。需要行情、个股资料、盘口、K 线、组合、持仓、建议或风险事实时必须调用工具，不要猜测。用简洁中文 Markdown 回答。如果缓存行情不可用，要明确说明并建议用户刷新自选股行情，不要编造实时行情。只有用户明确要求创建模拟委托草稿时，才调用 paper_order_draft。".into()
}

pub fn default_recommendation_system_prompt() -> String {
    "你是 KittyRed 的沪深 A 股模拟投资助手。只输出 JSON，不要输出 Markdown 或解释性前后缀。必须始终提供 rationale。没有清晰机会时返回 has_trade=false，并在 rationale 里说明最重要的 2 到 3 个未满足条件。如果 has_trade=true，只能给本地模拟买入或已有持仓卖出计划，必须包含 direction、confidence_score、rationale、entry_low、entry_high、stop_loss、take_profit、amount_cny、invalidation、max_loss_cny。卖出只适用于 position_context 存在的股票，代表退出或减仓本地模拟持仓，不代表开空仓；无持仓股票只能返回买入或观望。不要输出杠杆、真实交易、券商账户、其他市场或套利建议。has_trade=false 时不要只写“暂无机会”，要结合输入中的价格、成交额、价差、K 线或风控阈值说明原因。".into()
}

fn default_signal_min_score() -> f64 {
    30.0
}

fn default_signal_cooldown_minutes() -> u32 {
    15
}

fn default_signal_daily_max() -> u32 {
    50
}

fn default_use_bid_ask_data() -> bool {
    true
}

fn default_ai_kline_bar_count() -> u32 {
    60
}

pub fn default_ai_kline_frequencies() -> Vec<String> {
    vec!["5m".into(), "1h".into(), "1d".into(), "1w".into()]
}
