use anyhow::bail;
use reqwest::Client;

use super::MarketTarget;
use crate::models::{
    DerivativesSnapshot, MarketSnapshot, OhlcvBar, OrderBookSnapshot, RecentTrade,
};

#[derive(Clone, Debug)]
pub struct BatchTickerSnapshot {
    pub snapshot: MarketSnapshot,
    pub funding_rate: Option<f64>,
}

pub fn supported_exchanges() -> &'static [&'static str] {
    &["akshare"]
}

pub fn http_client() -> Client {
    Client::builder()
        .user_agent("KittyRed/0.1.0")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("failed to build reqwest client")
}

pub async fn discover_market_targets(
    _enabled_exchanges: &[&str],
) -> anyhow::Result<Vec<MarketTarget>> {
    Ok(Vec::new())
}

pub async fn fetch_market_snapshot(
    _client: &Client,
    _exchange: &str,
    _symbol: &str,
    _market_type: &str,
) -> anyhow::Result<MarketSnapshot> {
    bail!("行情数据源已切换为 AKShare，请使用自选股刷新接口")
}

pub async fn fetch_batch_market_snapshots(
    _client: &Client,
    _exchange: &str,
) -> anyhow::Result<Vec<BatchTickerSnapshot>> {
    bail!("批量行情数据源已切换为 AKShare")
}

pub async fn fetch_derivatives_snapshot(
    _client: &Client,
    _exchange: &str,
    _symbol: &str,
) -> anyhow::Result<DerivativesSnapshot> {
    bail!("A 股模式不支持衍生品行情")
}

pub async fn fetch_all_orderbooks(
    _client: &Client,
    _symbol: &str,
    _market_type: &str,
    _preferred_exchanges: &[&str],
) -> Vec<(String, OrderBookSnapshot)> {
    Vec::new()
}

pub async fn fetch_all_trades(
    _client: &Client,
    _symbol: &str,
    _market_type: &str,
    _preferred_exchanges: &[&str],
) -> Vec<(String, Vec<RecentTrade>)> {
    Vec::new()
}

pub async fn fetch_candles(
    _client: &Client,
    _symbol: &str,
    _market_type: &str,
    _interval: &str,
    _exchange: Option<&str>,
    _preferred_exchanges: &[&str],
) -> anyhow::Result<(String, Vec<OhlcvBar>)> {
    bail!("K 线数据源已切换为 AKShare")
}
