use crate::app_state::AppState;
use crate::errors::CommandResult;
use crate::models::{
    AShareSymbolSearchResultDto, ArbitrageOpportunityPageDto, CandleSeriesDto, CoinInfoDto,
    MarketListRow, PairDetailDto, PairVenueSnapshot, SpreadOpportunityDto,
};
use serde::Serialize;
use tauri::Emitter;

#[cfg(test)]
mod tests {
    use super::pair_detail_from_row;
    use crate::models::{MarketListRow, VenueTickerSnapshot};

    #[test]
    fn pair_detail_uses_row_quote_when_cached_venue_snapshots_are_missing() {
        let detail = pair_detail_from_row(
            MarketListRow {
                symbol: "SHSE.600000".into(),
                base_asset: "浦发银行".into(),
                market_type: "ashare".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: "large".into(),
                last_price: 8.88,
                change_24h: 1.23,
                volume_24h: 123_456_789.0,
                funding_rate: None,
                spread_bps: 0.0,
                exchanges: vec!["akshare".into()],
                updated_at: "2026-05-07T10:30:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 1,
                fdv_usd: None,
            },
            "ashare",
            None,
        );

        assert_eq!(detail.coin_info.name, "浦发银行");
        assert_eq!(detail.venues.len(), 1);
        assert_eq!(detail.venues[0].last_price, 8.88);
        assert_eq!(detail.venues[0].change_pct, 1.23);
        assert_eq!(detail.venues[0].volume_24h, 123_456_789.0);
    }

    #[test]
    fn pair_detail_preserves_realtime_venue_snapshot_values() {
        let detail = pair_detail_from_row(
            MarketListRow {
                symbol: "SHSE.600000".into(),
                base_asset: "浦发银行".into(),
                market_type: "ashare".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: "large".into(),
                last_price: 8.88,
                change_24h: 1.23,
                volume_24h: 123_456_789.0,
                funding_rate: None,
                spread_bps: 0.0,
                exchanges: vec!["akshare".into()],
                updated_at: "2026-05-07T10:30:00+08:00".into(),
                stale: false,
                venue_snapshots: vec![VenueTickerSnapshot {
                    exchange: "akshare".into(),
                    last_price: 8.88,
                    bid_price: 8.87,
                    ask_price: 8.89,
                    volume_24h: 123_456_789.0,
                    funding_rate: None,
                    mark_price: None,
                    index_price: None,
                    updated_at: "2026-05-07T10:30:00+08:00".into(),
                    stale: false,
                }],
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 1,
                fdv_usd: None,
            },
            "ashare",
            None,
        );

        assert_eq!(detail.venues[0].last_price, 8.88);
        assert_eq!(detail.venues[0].volume_24h, 123_456_789.0);
    }
}

#[tauri::command]
pub async fn list_markets(state: tauri::State<'_, AppState>) -> CommandResult<Vec<MarketListRow>> {
    let settings = state.settings_service.get_runtime_settings();
    let watchlist = normalize_watchlist(&settings.watchlist_symbols);
    Ok(state
        .market_data_service
        .cached_market_rows_for_watchlist(&watchlist))
}

#[tauri::command]
pub async fn list_market_symbols(state: tauri::State<'_, AppState>) -> CommandResult<Vec<String>> {
    let settings = state.settings_service.get_runtime_settings();
    Ok(normalize_watchlist(&settings.watchlist_symbols))
}

#[tauri::command]
pub async fn search_a_share_symbols(
    state: tauri::State<'_, AppState>,
    query: String,
) -> CommandResult<Vec<AShareSymbolSearchResultDto>> {
    state
        .market_data_service
        .search_a_share_symbols(&query)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn refresh_watchlist_tickers(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<MarketListRow>> {
    let settings = state.settings_service.get_runtime_settings();
    let watchlist = normalize_watchlist(&settings.watchlist_symbols);
    state
        .market_data_service
        .refresh_ticker_cache_from_akshare(&state.settings_service, &watchlist)
        .await
        .map_err(|error| error.to_string())?;
    Ok(state
        .market_data_service
        .cached_market_rows_for_watchlist(&watchlist))
}

#[tauri::command]
pub async fn get_pair_detail(
    state: tauri::State<'_, AppState>,
    symbol: String,
    market_type: Option<String>,
    exchange: Option<String>,
) -> CommandResult<PairDetailDto> {
    let settings = state.settings_service.get_runtime_settings();
    let normalized = symbol.trim().to_ascii_uppercase();
    let watchlist = normalize_watchlist(&settings.watchlist_symbols);
    if !watchlist.contains(&normalized) {
        return Err(format!("{normalized} 不在自选股中"));
    }

    let refresh_symbols = vec![normalized.clone()];
    let _ = state
        .market_data_service
        .refresh_ticker_cache_from_akshare(&state.settings_service, &refresh_symbols)
        .await;

    let row = state
        .market_data_service
        .cached_market_rows_for_watchlist(&watchlist)
        .into_iter()
        .find(|row| row.symbol == normalized)
        .ok_or_else(|| format!("{normalized} 暂无缓存行情，请刷新 AKShare 数据"))?;

    Ok(pair_detail_from_row(
        row,
        market_type.as_deref().unwrap_or("ashare"),
        exchange,
    ))
}

#[tauri::command]
pub async fn get_pair_candles(
    state: tauri::State<'_, AppState>,
    symbol: String,
    market_type: Option<String>,
    interval: String,
    exchange: Option<String>,
) -> CommandResult<CandleSeriesDto> {
    let settings = state.settings_service.get_runtime_settings();
    let normalized = symbol.trim().to_ascii_uppercase();
    let watchlist = normalize_watchlist(&settings.watchlist_symbols);
    if !watchlist.contains(&normalized) {
        return Ok(empty_candle_series(&normalized, &interval));
    }

    let frequency = akshare_frequency(&interval);
    let count = candle_count(&interval);
    let market_type = market_type.unwrap_or_else(|| "ashare".into());
    let exchange = exchange.unwrap_or_else(|| "akshare".into());
    let cached = state
        .market_data_service
        .cached_candle_bars(&normalized, &frequency, count);
    if !cached.is_empty() {
        refresh_candle_cache_in_background(
            state.app_handle.clone(),
            state.settings_service.clone(),
            state.market_data_service.clone(),
            normalized.clone(),
            interval.clone(),
            frequency,
            count,
        );
        return Ok(candle_series_from_bars(
            exchange,
            normalized,
            market_type,
            interval,
            cached,
        ));
    }

    let bars = crate::market::akshare::fetch_history_bars_with_settings(
        &state.settings_service,
        &normalized,
        &frequency,
        count,
    )
    .map_err(|error| error.to_string())?;
    state
        .market_data_service
        .cache_candle_bars(&normalized, &frequency, &bars)
        .map_err(|error| error.to_string())?;

    Ok(candle_series_from_bars(
        exchange,
        normalized,
        market_type,
        interval,
        bars,
    ))
}

fn candle_series_from_bars(
    exchange: String,
    symbol: String,
    market_type: String,
    interval: String,
    bars: Vec<crate::models::OhlcvBar>,
) -> CandleSeriesDto {
    let updated_at = bars
        .last()
        .map(|bar| bar.open_time.clone())
        .unwrap_or_default();

    CandleSeriesDto {
        exchange,
        symbol,
        market_type,
        interval,
        bars,
        updated_at,
    }
}

fn refresh_candle_cache_in_background(
    app_handle: tauri::AppHandle,
    settings_service: crate::settings::SettingsService,
    market_data_service: crate::market::MarketDataService,
    symbol: String,
    interval: String,
    frequency: String,
    count: usize,
) {
    tauri::async_runtime::spawn(async move {
        let fetch_symbol = symbol.clone();
        let fetch_frequency = frequency.clone();
        let settings_service = settings_service.clone();
        let fetched = tokio::task::spawn_blocking(move || {
            crate::market::akshare::fetch_history_bars_with_settings(
                &settings_service,
                &fetch_symbol,
                &fetch_frequency,
                count,
            )
        })
        .await;
        if let Ok(Ok(bars)) = fetched {
            let _ = market_data_service.cache_candle_bars(&symbol, &frequency, &bars);
            let _ = app_handle.emit(
                "market://candle-cache-updated",
                CandleCacheUpdatedEvent { symbol, interval },
            );
        }
    });
}

#[derive(Clone, Serialize)]
struct CandleCacheUpdatedEvent {
    symbol: String,
    interval: String,
}

#[tauri::command]
pub async fn list_spread_opportunities(
    state: tauri::State<'_, AppState>,
) -> CommandResult<Vec<SpreadOpportunityDto>> {
    let _ = state;
    Ok(Vec::new())
}

#[tauri::command]
pub async fn list_arbitrage_opportunities(
    state: tauri::State<'_, AppState>,
    page: Option<usize>,
    page_size: Option<usize>,
    type_filter: Option<String>,
) -> CommandResult<ArbitrageOpportunityPageDto> {
    let _ = state;
    let _ = type_filter;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(25);
    Ok(ArbitrageOpportunityPageDto {
        items: Vec::new(),
        total: 0,
        page,
        page_size,
        total_pages: 0,
    })
}

fn normalize_watchlist(symbols: &[String]) -> Vec<String> {
    let mut values = symbols
        .iter()
        .map(|symbol| symbol.trim().to_ascii_uppercase())
        .filter(|symbol| symbol.starts_with("SHSE.") || symbol.starts_with("SZSE."))
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn akshare_frequency(interval: &str) -> String {
    match interval.to_ascii_lowercase().as_str() {
        "1m" | "5m" | "15m" | "30m" | "60m" => interval.to_ascii_lowercase(),
        "1h" => "60m".into(),
        "1w" => "1w".into(),
        _ => "1d".into(),
    }
}

fn candle_count(interval: &str) -> usize {
    match interval.to_ascii_lowercase().as_str() {
        "1m" | "5m" => 120,
        "15m" | "30m" | "1h" | "60m" => 96,
        "1w" => 80,
        _ => 120,
    }
}

fn empty_candle_series(symbol: &str, interval: &str) -> CandleSeriesDto {
    CandleSeriesDto {
        exchange: "akshare".into(),
        symbol: symbol.into(),
        market_type: "ashare".into(),
        interval: interval.into(),
        bars: Vec::new(),
        updated_at: String::new(),
    }
}

fn pair_detail_from_row(
    row: MarketListRow,
    market_type: &str,
    exchange: Option<String>,
) -> PairDetailDto {
    let venues = if row.venue_snapshots.is_empty() {
        vec![PairVenueSnapshot {
            exchange: "akshare".into(),
            last_price: row.last_price,
            bid_price: row.best_bid_price.unwrap_or(row.last_price),
            ask_price: row.best_ask_price.unwrap_or(row.last_price),
            change_pct: row.change_24h,
            volume_24h: row.volume_24h,
            funding_rate: None,
            mark_price: None,
            index_price: None,
            open_interest: None,
            next_funding_at: None,
            updated_at: row.updated_at.clone(),
        }]
    } else {
        row.venue_snapshots
            .iter()
            .map(|venue| PairVenueSnapshot {
                exchange: venue.exchange.clone(),
                last_price: venue.last_price,
                bid_price: venue.bid_price,
                ask_price: venue.ask_price,
                change_pct: row.change_24h,
                volume_24h: venue.volume_24h,
                funding_rate: None,
                mark_price: None,
                index_price: None,
                open_interest: None,
                next_funding_at: None,
                updated_at: venue.updated_at.clone(),
            })
            .collect::<Vec<_>>()
    };
    let listed_exchanges = if let Some(exchange) = exchange {
        vec![exchange]
    } else {
        row.exchanges.clone()
    };

    PairDetailDto {
        symbol: row.symbol.clone(),
        market_type: market_type.into(),
        thesis: "该标的来自用户自选股，行情与 K 线由 AKShare 提供。".into(),
        source_note: "A 股版仅使用 AKShare 数据源。".into(),
        coin_info: CoinInfoDto {
            name: row.base_asset.clone(),
            symbol: row.symbol,
            summary: "沪深 A 股自选标的。".into(),
            website: None,
            whitepaper: None,
            explorer: None,
            ecosystem: "A 股".into(),
            market_cap: None,
            fdv: None,
            circulating_supply: None,
            total_supply: None,
            max_supply: None,
            volume_24h: Some(format!("{:.0}", row.volume_24h)),
            listed_exchanges,
            risk_tags: Vec::new(),
            github: None,
        },
        venues,
        orderbooks: Vec::new(),
        recent_trades: Vec::new(),
        spreads: Vec::new(),
    }
}
