use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail};
use serde::Deserialize;
use serde_json::json;

use crate::models::{AShareSymbolSearchResultDto, OhlcvBar};

pub const DATA_SOURCE_NAME: &str = "akshare";

#[derive(Debug, Clone, Deserialize)]
pub struct AkshareQuote {
    pub symbol: String,
    pub name: Option<String>,
    pub last: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub change_pct: Option<f64>,
    pub volume: f64,
    pub amount: f64,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct AkshareResponse<T> {
    ok: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AkshareBar {
    open_time: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    turnover: Option<f64>,
}

pub fn fetch_current_quotes(symbols: &[String]) -> anyhow::Result<Vec<AkshareQuote>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python(json!({
        "action": "current_quotes",
        "symbols": symbols,
    }))?;
    let parsed: AkshareResponse<Vec<AkshareQuote>> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 行情请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_default())
}

pub fn fetch_history_bars(
    symbol: &str,
    frequency: &str,
    count: usize,
) -> anyhow::Result<Vec<OhlcvBar>> {
    if symbol.trim().is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python(json!({
        "action": "history_bars",
        "symbol": symbol,
        "frequency": frequency,
        "count": count,
    }))?;
    let parsed: AkshareResponse<Vec<AkshareBar>> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare K 线请求失败".into()));
    }
    Ok(parsed
        .data
        .unwrap_or_default()
        .into_iter()
        .map(|bar| OhlcvBar {
            open_time: bar.open_time,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            turnover: bar.turnover,
        })
        .collect())
}

pub fn fetch_stock_info(symbol: &str) -> anyhow::Result<serde_json::Value> {
    let response = call_python(json!({
        "action": "stock_info",
        "symbol": symbol,
    }))?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 个股资料请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn fetch_bid_ask(symbol: &str) -> anyhow::Result<serde_json::Value> {
    let response = call_python(json!({
        "action": "bid_ask",
        "symbol": symbol,
    }))?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 五档盘口请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn fetch_multi_frequency_bars(symbol: &str, count: usize) -> anyhow::Result<serde_json::Value> {
    let response = call_python(json!({
        "action": "multi_frequency_bars",
        "symbol": symbol,
        "count": count,
    }))?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 多周期 K 线请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn fetch_financial_reports(years: u32) -> anyhow::Result<serde_json::Value> {
    let response = call_python(json!({
        "action": "financial_reports",
        "years": years,
    }))?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 财报请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn search_stocks(query: &str) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python(json!({
        "action": "search_stocks",
        "query": query,
    }))?;
    let parsed: AkshareResponse<Vec<AShareSymbolSearchResultDto>> =
        serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 股票搜索失败".into()));
    }
    Ok(parsed.data.unwrap_or_default())
}

pub fn fetch_stock_universe() -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
    let response = call_python(json!({
        "action": "stock_universe",
    }))?;
    let parsed: AkshareResponse<Vec<AShareSymbolSearchResultDto>> =
        serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 股票列表缓存失败".into()));
    }
    Ok(parsed.data.unwrap_or_default())
}

pub fn is_trade_date(date: &str) -> anyhow::Result<bool> {
    let response = call_python(json!({
        "action": "is_trade_date",
        "date": date,
    }))?;
    let parsed: AkshareResponse<bool> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 交易日历请求失败".into()));
    }
    Ok(parsed.data.unwrap_or(false))
}

pub fn warm_stock_universe_cache() -> anyhow::Result<usize> {
    Ok(fetch_stock_universe()?.len())
}

fn call_python(request: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("failed to resolve project root"))?;
    let mut child = Command::new(python)
        .arg("-m")
        .arg("backend.akshare_service")
        .current_dir(project_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(request.to_string().as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        bail!(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
