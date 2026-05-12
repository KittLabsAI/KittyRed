use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::{AShareSymbolSearchResultDto, OhlcvBar};
use crate::settings::SettingsService;

pub const DATA_SOURCE_NAME: &str = "akshare";

#[derive(Debug, Clone, Default)]
pub struct AkshareRequestOverrides {
    pub intraday_data_source: Option<String>,
    pub historical_data_source: Option<String>,
    pub xueqiu_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

pub fn fetch_current_quote(symbol: &str) -> anyhow::Result<serde_json::Value> {
    let settings = SettingsService::default();
    fetch_current_quote_with_settings(&settings, symbol)
}

pub fn fetch_current_quote_with_settings(
    settings: &SettingsService,
    symbol: &str,
) -> anyhow::Result<serde_json::Value> {
    fetch_current_quote_with_overrides(settings, symbol, &AkshareRequestOverrides::default())
}

pub fn fetch_current_quote_with_overrides(
    settings: &SettingsService,
    symbol: &str,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let response = call_python_with_settings(
        json!({
            "action": "current_quote",
            "symbol": symbol,
        }),
        settings,
        overrides,
    )?;
    let parsed: AkshareResponse<AkshareQuote> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 行情请求失败".into()));
    }
    Ok(json!({
        "ok": true,
        "data": parsed.data,
    }))
}

pub fn fetch_current_quotes(symbols: &[String]) -> anyhow::Result<Vec<AkshareQuote>> {
    let settings = SettingsService::default();
    fetch_current_quotes_with_settings(&settings, symbols)
}

pub fn fetch_current_quotes_with_settings(
    settings: &SettingsService,
    symbols: &[String],
) -> anyhow::Result<Vec<AkshareQuote>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python_with_settings(
        json!({
            "action": "current_quotes",
            "symbols": symbols,
        }),
        settings,
        &AkshareRequestOverrides::default(),
    )?;
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
    let settings = SettingsService::default();
    fetch_history_bars_with_settings(&settings, symbol, frequency, count)
}

pub fn fetch_history_bars_with_settings(
    settings: &SettingsService,
    symbol: &str,
    frequency: &str,
    count: usize,
) -> anyhow::Result<Vec<OhlcvBar>> {
    fetch_history_bars_in_range_with_overrides(
        settings,
        symbol,
        frequency,
        count,
        None,
        None,
        &AkshareRequestOverrides::default(),
    )
}

pub fn fetch_history_bars_in_range(
    symbol: &str,
    frequency: &str,
    count: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> anyhow::Result<Vec<OhlcvBar>> {
    let settings = SettingsService::default();
    fetch_history_bars_in_range_with_settings(
        &settings, symbol, frequency, count, start_date, end_date,
    )
}

pub fn fetch_history_bars_in_range_with_settings(
    settings: &SettingsService,
    symbol: &str,
    frequency: &str,
    count: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> anyhow::Result<Vec<OhlcvBar>> {
    fetch_history_bars_in_range_with_overrides(
        settings,
        symbol,
        frequency,
        count,
        start_date,
        end_date,
        &AkshareRequestOverrides::default(),
    )
}

pub fn fetch_history_bars_in_range_with_overrides(
    settings: &SettingsService,
    symbol: &str,
    frequency: &str,
    count: usize,
    start_date: Option<&str>,
    end_date: Option<&str>,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<Vec<OhlcvBar>> {
    if symbol.trim().is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python_with_settings(
        json!({
            "action": "history_bars",
            "symbol": symbol,
            "frequency": frequency,
            "count": count,
            "start_date": start_date,
            "end_date": end_date,
        }),
        settings,
        overrides,
    )?;
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
    let settings = SettingsService::default();
    fetch_stock_info_with_settings(&settings, symbol)
}

pub fn fetch_stock_info_with_settings(
    settings: &SettingsService,
    symbol: &str,
) -> anyhow::Result<serde_json::Value> {
    fetch_stock_info_with_overrides(settings, symbol, &AkshareRequestOverrides::default())
}

pub fn fetch_stock_info_with_overrides(
    settings: &SettingsService,
    symbol: &str,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let response = call_python_with_settings(
        json!({
            "action": "stock_info",
            "symbol": symbol,
        }),
        settings,
        overrides,
    )?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 个股资料请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}


pub fn fetch_multi_frequency_bars(symbol: &str, count: usize) -> anyhow::Result<serde_json::Value> {
    let settings = SettingsService::default();
    fetch_multi_frequency_bars_with_settings(&settings, symbol, count)
}

pub fn fetch_multi_frequency_bars_with_settings(
    settings: &SettingsService,
    symbol: &str,
    count: usize,
) -> anyhow::Result<serde_json::Value> {
    let response = call_python_with_settings(
        json!({
            "action": "multi_frequency_bars",
            "symbol": symbol,
            "count": count,
        }),
        settings,
        &AkshareRequestOverrides::default(),
    )?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 多周期 K 线请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn fetch_financial_reports(years: u32) -> anyhow::Result<serde_json::Value> {
    let settings = SettingsService::default();
    fetch_financial_reports_with_settings(&settings, years)
}

pub fn fetch_financial_reports_with_settings(
    settings: &SettingsService,
    years: u32,
) -> anyhow::Result<serde_json::Value> {
    fetch_financial_reports_with_overrides(settings, years, &AkshareRequestOverrides::default())
}

pub fn fetch_financial_reports_with_overrides(
    settings: &SettingsService,
    years: u32,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let response = call_python_with_settings(
        json!({
            "action": "financial_reports",
            "years": years,
        }),
        settings,
        overrides,
    )?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 财报请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn fetch_financial_report_probe_with_overrides(
    settings: &SettingsService,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let response = call_python_with_settings(
        json!({
            "action": "financial_report_probe",
        }),
        settings,
        overrides,
    )?;
    let parsed: AkshareResponse<serde_json::Value> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed.error.unwrap_or_else(|| "AKShare 财报请求失败".into()));
    }
    Ok(parsed.data.unwrap_or_else(|| json!({})))
}

pub fn search_stocks(query: &str) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
    let settings = SettingsService::default();
    search_stocks_with_settings(&settings, query)
}

pub fn search_stocks_with_settings(
    settings: &SettingsService,
    query: &str,
) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let response = call_python_with_settings(
        json!({
            "action": "search_stocks",
            "query": query,
        }),
        settings,
        &AkshareRequestOverrides::default(),
    )?;
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
    let settings = SettingsService::default();
    fetch_stock_universe_with_settings(&settings)
}

pub fn fetch_stock_universe_with_settings(
    settings: &SettingsService,
) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
    let response = call_python_with_settings(
        json!({
            "action": "stock_universe",
        }),
        settings,
        &AkshareRequestOverrides::default(),
    )?;
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
    let settings = SettingsService::default();
    is_trade_date_with_settings(&settings, date)
}

pub fn is_trade_date_with_settings(settings: &SettingsService, date: &str) -> anyhow::Result<bool> {
    is_trade_date_with_overrides(settings, date, &AkshareRequestOverrides::default())
}

pub fn is_trade_date_with_overrides(
    settings: &SettingsService,
    date: &str,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<bool> {
    let response = call_python_with_settings(
        json!({
            "action": "is_trade_date",
            "date": date,
        }),
        settings,
        overrides,
    )?;
    let parsed: AkshareResponse<bool> = serde_json::from_value(response)?;
    if !parsed.ok {
        bail!(parsed
            .error
            .unwrap_or_else(|| "AKShare 交易日历请求失败".into()));
    }
    Ok(parsed.data.unwrap_or(false))
}

pub fn warm_stock_universe_cache() -> anyhow::Result<usize> {
    let settings = SettingsService::default();
    warm_stock_universe_cache_with_settings(&settings)
}

pub fn warm_stock_universe_cache_with_settings(
    settings: &SettingsService,
) -> anyhow::Result<usize> {
    Ok(fetch_stock_universe_with_settings(settings)?.len())
}

fn call_python_with_settings(
    request: serde_json::Value,
    settings: &SettingsService,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| anyhow!("failed to resolve project root"))?;
    let request = request_with_runtime_overrides(request, settings, overrides)?;
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

fn request_with_runtime_settings(
    request: serde_json::Value,
    settings: &SettingsService,
) -> anyhow::Result<serde_json::Value> {
    request_with_runtime_overrides(request, settings, &AkshareRequestOverrides::default())
}

fn request_with_runtime_overrides(
    mut request: serde_json::Value,
    settings: &SettingsService,
    overrides: &AkshareRequestOverrides,
) -> anyhow::Result<serde_json::Value> {
    let runtime = settings.get_runtime_settings();
    if let Some(map) = request.as_object_mut() {
        map.insert(
            "intraday_data_source".into(),
            json!(overrides
                .intraday_data_source
                .as_deref()
                .unwrap_or(runtime.intraday_data_source.as_str())),
        );
        map.insert(
            "historical_data_source".into(),
            json!(overrides
                .historical_data_source
                .as_deref()
                .unwrap_or(runtime.historical_data_source.as_str())),
        );
    }
    let token = if let Some(token) = overrides
        .xueqiu_token
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(token.clone())
    } else {
        settings
            .xueqiu_token()?
            .filter(|value| !value.trim().is_empty())
    };
    if let Some(token) = token {
        if let Some(map) = request.as_object_mut() {
            map.insert("xueqiu_token".into(), json!(token));
        }
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::{
        request_with_runtime_overrides, request_with_runtime_settings, AkshareRequestOverrides,
    };
    use crate::models::RuntimeSecretsSyncDto;
    use crate::settings::SettingsService;
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn injects_runtime_settings_and_xueqiu_token_into_current_quote_requests() {
        let path = unique_temp_settings_path("akshare-token-request");
        let secret_path = path.with_extension("secrets.json");
        let settings = SettingsService::new(path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.intraday_data_source = "eastmoney".into();
        runtime.historical_data_source = "tencent".into();
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should save");
        settings
            .sync_runtime_secrets(RuntimeSecretsSyncDto {
                persist: true,
                model_api_key: None,
                xueqiu_token: Some("xq-token-test".into()),
                exchanges: Vec::new(),
            })
            .expect("xueqiu token should sync");

        let request =
            request_with_runtime_settings(json!({ "action": "current_quote" }), &settings)
                .expect("request should include runtime settings");

        assert_eq!(request["xueqiu_token"], json!("xq-token-test"));
        assert_eq!(request["intraday_data_source"], json!("eastmoney"));
        assert_eq!(request["historical_data_source"], json!("tencent"));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path);
    }

    #[test]
    fn request_overrides_replace_runtime_sources_and_optional_xueqiu_token() {
        let path = unique_temp_settings_path("akshare-test-overrides");
        let secret_path = path.with_extension("secrets.json");
        let settings = SettingsService::new(path.clone());
        let mut runtime = settings.get_runtime_settings();
        runtime.intraday_data_source = "sina".into();
        runtime.historical_data_source = "eastmoney".into();
        settings
            .save_runtime_settings(runtime)
            .expect("runtime settings should save");

        let request = request_with_runtime_overrides(
            json!({ "action": "history_bars" }),
            &settings,
            &AkshareRequestOverrides {
                intraday_data_source: Some("eastmoney".into()),
                historical_data_source: Some("tencent".into()),
                xueqiu_token: Some("override-token".into()),
            },
        )
        .expect("request should apply explicit overrides");

        assert_eq!(request["intraday_data_source"], json!("eastmoney"));
        assert_eq!(request["historical_data_source"], json!("tencent"));
        assert_eq!(request["xueqiu_token"], json!("override-token"));

        let request_without_token = request_with_runtime_overrides(
            json!({ "action": "current_quote" }),
            &settings,
            &AkshareRequestOverrides {
                intraday_data_source: Some("eastmoney".into()),
                historical_data_source: Some("tencent".into()),
                xueqiu_token: None,
            },
        )
        .expect("request should fall back to runtime token handling");

        assert!(request_without_token.get("xueqiu_token").is_none());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(secret_path);
    }

    fn unique_temp_settings_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("kittyred-{label}-{nanos}.json"))
    }
}
