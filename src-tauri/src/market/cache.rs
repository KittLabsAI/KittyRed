use std::path::Path;

use super::asset_metadata::{
    base_asset_from_symbol, classify_market_size_tier, AssetMetadataRecord,
};
use crate::db::Database;
use crate::models::{
    AShareSymbolSearchResultDto, MarketListRow, MarketSnapshot, OhlcvBar, OrderBookSnapshot,
    RecentTrade,
};
use rusqlite::OptionalExtension;

#[derive(Default)]
pub struct MarketCache {
    pub snapshots: dashmap::DashMap<String, MarketSnapshot>,
    pub rows: dashmap::DashMap<String, MarketListRow>,
    pub orderbooks: dashmap::DashMap<String, OrderBookSnapshot>,
    pub trades: dashmap::DashMap<String, Vec<RecentTrade>>,
}

pub struct SqliteMarketTickerCache {
    db: Database,
}

#[cfg(test)]
mod tests {
    use super::SqliteMarketTickerCache;
    use crate::models::{AShareSymbolSearchResultDto, OhlcvBar};

    #[test]
    fn searches_cached_a_share_symbols_without_live_fetch() {
        let cache = SqliteMarketTickerCache::in_memory().unwrap();
        cache
            .upsert_a_share_symbols(&[
                AShareSymbolSearchResultDto {
                    symbol: "SHSE.600000".into(),
                    name: "浦发银行".into(),
                    market: "沪市A股".into(),
                },
                AShareSymbolSearchResultDto {
                    symbol: "SZSE.000001".into(),
                    name: "平安银行".into(),
                    market: "深市A股".into(),
                },
            ])
            .unwrap();

        assert_eq!(
            cache.search_a_share_symbols("600000", 20).unwrap(),
            vec![AShareSymbolSearchResultDto {
                symbol: "SHSE.600000".into(),
                name: "浦发银行".into(),
                market: "沪市A股".into(),
            }]
        );
        assert_eq!(
            cache.search_a_share_symbols("平安", 20).unwrap()[0].symbol,
            "SZSE.000001"
        );
    }

    #[test]
    fn upserts_and_reads_cached_candles_by_symbol_and_interval() {
        let cache = SqliteMarketTickerCache::in_memory().unwrap();
        cache
            .upsert_candles(
                "SHSE.600000",
                "60m",
                &[OhlcvBar {
                    open_time: "2026-05-07 11:30:00".into(),
                    open: 9.15,
                    high: 9.16,
                    low: 9.15,
                    close: 9.15,
                    volume: 167_100.0,
                    turnover: Some(1_530_125.991),
                }],
            )
            .unwrap();

        let candles = cache.list_candles("SHSE.600000", "60m", 120).unwrap();

        assert_eq!(candles.len(), 1);
        assert_eq!(candles[0].open_time, "2026-05-07 11:30:00");
        assert_eq!(candles[0].close, 9.15);
    }
}

impl SqliteMarketTickerCache {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            db: Database::open(path)?,
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> anyhow::Result<Self> {
        let db = Database::in_memory()?;
        db.run_migrations()?;
        Ok(Self { db })
    }

    pub fn upsert_rows(&self, rows: &[MarketListRow]) -> anyhow::Result<()> {
        for row in rows {
            self.db.connection().execute(
                "INSERT INTO market_ticker_cache (
                  symbol, base_asset, market_type, last_price, bid_price, ask_price, change_24h,
                  volume_24h, funding_rate, spread_bps, exchanges_json, updated_at, stale,
                  venue_snapshots_json, best_bid_exchange, best_ask_exchange, best_bid_price,
                  best_ask_price, responded_exchange_count, market_cap_usd, fdv_usd
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                ON CONFLICT(symbol, market_type) DO UPDATE SET
                  base_asset = excluded.base_asset,
                  last_price = excluded.last_price,
                  bid_price = excluded.bid_price,
                  ask_price = excluded.ask_price,
                  change_24h = excluded.change_24h,
                  volume_24h = excluded.volume_24h,
                  funding_rate = excluded.funding_rate,
                  spread_bps = excluded.spread_bps,
                  exchanges_json = excluded.exchanges_json,
                  venue_snapshots_json = excluded.venue_snapshots_json,
                  best_bid_exchange = excluded.best_bid_exchange,
                  best_ask_exchange = excluded.best_ask_exchange,
                  best_bid_price = excluded.best_bid_price,
                  best_ask_price = excluded.best_ask_price,
                  responded_exchange_count = excluded.responded_exchange_count,
                  market_cap_usd = excluded.market_cap_usd,
                  fdv_usd = excluded.fdv_usd,
                  updated_at = excluded.updated_at,
                  stale = excluded.stale",
                rusqlite::params![
                    row.symbol,
                    row.base_asset,
                    row.market_type,
                    row.last_price,
                    row.venue_snapshots.first().map(|snapshot| snapshot.bid_price).unwrap_or(row.last_price),
                    row.venue_snapshots.first().map(|snapshot| snapshot.ask_price).unwrap_or(row.last_price),
                    row.change_24h,
                    row.volume_24h,
                    row.funding_rate,
                    row.spread_bps,
                    serde_json::to_string(&row.exchanges)?,
                    row.updated_at,
                    if row.stale { 1 } else { 0 },
                    serde_json::to_string(&row.venue_snapshots)?,
                    row.best_bid_exchange,
                    row.best_ask_exchange,
                    row.best_bid_price,
                    row.best_ask_price,
                    row.responded_exchange_count,
                    row.market_cap_usd,
                    row.fdv_usd,
                ],
            )?;
        }

        Ok(())
    }

    pub fn list_rows(&self) -> anyhow::Result<Vec<MarketListRow>> {
        let mut statement = self.db.connection().prepare(
            "SELECT symbol, base_asset, market_type, last_price, change_24h, volume_24h,
                    funding_rate, spread_bps, exchanges_json, updated_at, stale,
                    venue_snapshots_json, best_bid_exchange, best_ask_exchange, best_bid_price,
                    best_ask_price, responded_exchange_count, market_cap_usd, fdv_usd
             FROM market_ticker_cache
             ORDER BY symbol, market_type",
        )?;
        let rows = statement.query_map([], |row| {
            let symbol: String = row.get(0)?;
            let exchanges_json: String = row.get(8)?;
            let exchanges =
                serde_json::from_str::<Vec<String>>(&exchanges_json).unwrap_or_default();
            let venue_snapshots_json: String = row.get(11)?;
            let venue_snapshots = serde_json::from_str(&venue_snapshots_json).unwrap_or_default();
            Ok(MarketListRow {
                symbol: symbol.clone(),
                base_asset: {
                    let stored: String = row.get(1)?;
                    if stored.trim().is_empty() {
                        base_asset_from_symbol(&symbol)
                    } else {
                        stored
                    }
                },
                market_type: row.get(2)?,
                market_cap_usd: row.get(17)?,
                market_cap_rank: None,
                market_size_tier: classify_market_size_tier(None).into(),
                last_price: row.get(3)?,
                change_24h: row.get(4)?,
                volume_24h: row.get(5)?,
                funding_rate: row.get(6)?,
                spread_bps: row.get(7)?,
                exchanges,
                updated_at: row.get(9)?,
                stale: row.get::<_, i64>(10)? != 0,
                venue_snapshots,
                best_bid_exchange: row.get(12)?,
                best_ask_exchange: row.get(13)?,
                best_bid_price: row.get(14)?,
                best_ask_price: row.get(15)?,
                responded_exchange_count: row.get(16)?,
                fdv_usd: row.get(18)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn upsert_a_share_symbols(
        &self,
        rows: &[AShareSymbolSearchResultDto],
    ) -> anyhow::Result<()> {
        for row in rows {
            self.db.connection().execute(
                "INSERT INTO a_share_symbol_cache (symbol, name, market, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(symbol) DO UPDATE SET
                   name = excluded.name,
                   market = excluded.market,
                   updated_at = excluded.updated_at",
                rusqlite::params![row.symbol, row.name, row.market, current_cache_timestamp()],
            )?;
        }
        Ok(())
    }

    pub fn search_a_share_symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
        let cleaned = query.trim().to_uppercase();
        if cleaned.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("%{cleaned}%");
        let mut statement = self.db.connection().prepare(
            "SELECT symbol, name, market
             FROM a_share_symbol_cache
             WHERE UPPER(symbol) LIKE ?1
                OR UPPER(REPLACE(symbol, '.', '')) LIKE ?1
                OR UPPER(name) LIKE ?1
             ORDER BY
               CASE
                 WHEN UPPER(symbol) = ?2 THEN 0
                 WHEN UPPER(REPLACE(symbol, '.', '')) = ?2 THEN 1
                 WHEN UPPER(symbol) LIKE ?1 THEN 2
                 ELSE 3
               END,
               symbol
             LIMIT ?3",
        )?;
        let rows =
            statement.query_map(rusqlite::params![pattern, cleaned, limit as i64], |row| {
                Ok(AShareSymbolSearchResultDto {
                    symbol: row.get(0)?,
                    name: row.get(1)?,
                    market: row.get(2)?,
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn a_share_symbol_count(&self) -> anyhow::Result<usize> {
        Ok(self.db.connection().query_row(
            "SELECT COUNT(*) FROM a_share_symbol_cache",
            [],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    pub fn upsert_candles(
        &self,
        symbol: &str,
        interval: &str,
        bars: &[OhlcvBar],
    ) -> anyhow::Result<()> {
        let updated_at = current_cache_timestamp();
        for bar in bars {
            self.db.connection().execute(
                "INSERT INTO market_candle_cache (
                  symbol, interval, open_time, open, high, low, close, volume, turnover, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                ON CONFLICT(symbol, interval, open_time) DO UPDATE SET
                  open = excluded.open,
                  high = excluded.high,
                  low = excluded.low,
                  close = excluded.close,
                  volume = excluded.volume,
                  turnover = excluded.turnover,
                  updated_at = excluded.updated_at",
                rusqlite::params![
                    symbol,
                    interval,
                    bar.open_time,
                    bar.open,
                    bar.high,
                    bar.low,
                    bar.close,
                    bar.volume,
                    bar.turnover,
                    updated_at,
                ],
            )?;
        }
        Ok(())
    }

    pub fn list_candles(
        &self,
        symbol: &str,
        interval: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<OhlcvBar>> {
        let mut statement = self.db.connection().prepare(
            "SELECT open_time, open, high, low, close, volume, turnover
             FROM (
               SELECT open_time, open, high, low, close, volume, turnover
               FROM market_candle_cache
               WHERE symbol = ?1 AND interval = ?2
               ORDER BY open_time DESC
               LIMIT ?3
             )
             ORDER BY open_time ASC",
        )?;
        let rows =
            statement.query_map(rusqlite::params![symbol, interval, limit as i64], |row| {
                Ok(OhlcvBar {
                    open_time: row.get(0)?,
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                    turnover: row.get(6)?,
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn upsert_asset_metadata(&self, rows: &[AssetMetadataRecord]) -> anyhow::Result<()> {
        for row in rows {
            self.db.connection().execute(
                "INSERT INTO market_asset_metadata (
                  base_asset, provider, provider_asset_id, provider_symbol, provider_name,
                  market_cap_usd, market_cap_rank, fetched_at, source_updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(base_asset) DO UPDATE SET
                  provider = excluded.provider,
                  provider_asset_id = excluded.provider_asset_id,
                  provider_symbol = excluded.provider_symbol,
                  provider_name = excluded.provider_name,
                  market_cap_usd = excluded.market_cap_usd,
                  market_cap_rank = excluded.market_cap_rank,
                  fetched_at = excluded.fetched_at,
                  source_updated_at = excluded.source_updated_at",
                rusqlite::params![
                    row.base_asset,
                    row.provider,
                    row.provider_asset_id,
                    row.provider_symbol,
                    row.provider_name,
                    row.market_cap_usd,
                    row.market_cap_rank,
                    row.fetched_at,
                    row.source_updated_at,
                ],
            )?;
        }

        Ok(())
    }

    pub fn list_asset_metadata(&self) -> anyhow::Result<Vec<AssetMetadataRecord>> {
        let mut statement = self.db.connection().prepare(
            "SELECT base_asset, provider, provider_asset_id, provider_symbol, provider_name,
                    market_cap_usd, market_cap_rank, fetched_at, source_updated_at
             FROM market_asset_metadata
             ORDER BY COALESCE(market_cap_rank, 9223372036854775807), base_asset",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AssetMetadataRecord {
                base_asset: row.get(0)?,
                provider: row.get(1)?,
                provider_asset_id: row.get(2)?,
                provider_symbol: row.get(3)?,
                provider_name: row.get(4)?,
                market_cap_usd: row.get(5)?,
                market_cap_rank: row.get(6)?,
                fetched_at: row.get(7)?,
                source_updated_at: row.get(8)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn latest_asset_metadata_fetched_at(&self) -> anyhow::Result<Option<String>> {
        Ok(self
            .db
            .connection()
            .query_row(
                "SELECT fetched_at
             FROM market_asset_metadata
             ORDER BY fetched_at DESC, market_cap_rank ASC
             LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    }
}

fn current_cache_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".into())
}
