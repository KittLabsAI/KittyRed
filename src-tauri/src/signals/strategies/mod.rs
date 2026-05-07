use crate::models::{MarketListRow, OhlcvBar, VenueTickerSnapshot};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SignalCategory {
    Trend,
    Momentum,
    Arbitrage,
    Funding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicableMarket {
    Spot,
    Perpetual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalDirection {
    Buy,
    Sell,
    Neutral,
}

pub struct SignalContext<'a> {
    pub symbol: &'a str,
    pub market_type: &'a str,
    pub row: &'a MarketListRow,
    pub candles: &'a [OhlcvBar],
    pub venue_snapshots: &'a [VenueTickerSnapshot],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrategySignal {
    pub strategy_id: String,
    pub category: SignalCategory,
    pub direction: SignalDirection,
    pub strength: f64,
    pub confidence: f64,
    pub summary: String,
    pub metrics: HashMap<String, f64>,
}

pub trait Strategy: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn category(&self) -> SignalCategory;
    fn applicable_to(&self) -> Vec<ApplicableMarket>;
    fn description(&self) -> &'static str;
    fn default_params(&self) -> HashMap<String, f64>;
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String>;
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal>;
}

pub mod basis_deviation;
pub mod bollinger_break;
pub mod cross_market_arbitrage;
pub mod funding_rate;
pub mod ma_cross;
pub mod macd_divergence;
pub mod rsi_extreme;
pub mod spread_arbitrage;
pub mod volume_surge;
