pub mod akshare;
pub mod arbitrage;
pub mod asset_metadata;
pub mod cache;
pub mod coin_info;
pub mod live;
pub mod spreads;

#[cfg(test)]
mod tests {
    use super::asset_metadata::{
        AssetMetadataRecord, MARKET_SIZE_TIER_LARGE, MARKET_SIZE_TIER_SMALL,
    };
    use super::cache::SqliteMarketTickerCache;
    use super::live::BatchTickerSnapshot;
    use super::{
        aggregate_recent_trades, apply_asset_metadata, build_market_row,
        build_market_rows_from_batch_tickers, dedupe_market_targets, enabled_exchange_universe,
        limited_market_targets, market_symbols_from_targets, perpetual_primary_venue_rows,
        MarketDataService, MarketTarget,
    };
    use crate::models::{MarketListRow, MarketSnapshot, RecentTrade, VenueTickerSnapshot};
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    fn current_rfc3339() -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("timestamp should format")
    }
    #[test]
    fn aggregates_cross_exchange_snapshots_into_market_row() {
        let row = build_market_row(
            "BTC/USDT",
            "spot",
            &[
                MarketSnapshot {
                    exchange: "akshare".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "spot".into(),
                    last_price: 100.0,
                    bid_price: 99.8,
                    ask_price: 100.2,
                    volume_24h: 1000.0,
                    change_24h: 1.0,
                    updated_at: "t1".into(),
                    stale: false,
                },
                MarketSnapshot {
                    exchange: "人民币现金".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "spot".into(),
                    last_price: 101.0,
                    bid_price: 100.9,
                    ask_price: 101.1,
                    volume_24h: 2000.0,
                    change_24h: 2.0,
                    updated_at: "t2".into(),
                    stale: false,
                },
            ],
        )
        .unwrap();

        assert_eq!(row.symbol, "BTC/USDT");
        assert_eq!(row.base_asset, "BTC");
        assert_eq!(row.market_type, "spot");
        assert_eq!(
            row.exchanges,
            vec!["akshare".to_string(), "人民币现金".to_string()]
        );
        assert!((row.last_price - 100.5).abs() < 0.0001);
        assert!((row.spread_bps - 69.3069).abs() < 0.01);
        assert_eq!(row.market_size_tier, MARKET_SIZE_TIER_SMALL);
    }

    #[test]
    fn akshare_quote_rows_use_stock_name_and_provider_change_percent() {
        let row = super::akshare_quote_to_market_row(super::akshare::AkshareQuote {
            symbol: "SHSE.600000".into(),
            name: Some("浦发银行".into()),
            last: 8.72,
            open: 8.7,
            high: 8.8,
            low: 8.6,
            change_pct: Some(1.25),
            volume: 100_000.0,
            amount: 872_000.0,
            updated_at: "2026-05-06 10:00:00".into(),
        });

        assert_eq!(row.base_asset, "浦发银行");
        assert_eq!(row.change_24h, 1.25);
        assert_eq!(row.volume_24h, 872_000.0);
    }

    #[test]
    fn aggregates_batch_tickers_and_preserves_perpetual_funding() {
        let rows = build_market_rows_from_batch_tickers(vec![
            BatchTickerSnapshot {
                snapshot: MarketSnapshot {
                    exchange: "上海证券交易所".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "perpetual".into(),
                    last_price: 100.0,
                    bid_price: 99.8,
                    ask_price: 100.2,
                    volume_24h: 1000.0,
                    change_24h: 1.0,
                    updated_at: "1000".into(),
                    stale: false,
                },
                funding_rate: Some(0.01),
            },
            BatchTickerSnapshot {
                snapshot: MarketSnapshot {
                    exchange: "人民币现金".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "perpetual".into(),
                    last_price: 101.0,
                    bid_price: 100.9,
                    ask_price: 101.1,
                    volume_24h: 2000.0,
                    change_24h: 2.0,
                    updated_at: "2000".into(),
                    stale: false,
                },
                funding_rate: Some(0.03),
            },
        ])
        .expect("batch rows should aggregate");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC/USDT");
        assert_eq!(
            rows[0].exchanges,
            vec!["上海证券交易所".to_string(), "人民币现金".to_string()]
        );
        assert_eq!(rows[0].funding_rate, Some(0.02));
        assert_eq!(rows[0].venue_snapshots.len(), 2);
        assert_eq!(rows[0].venue_snapshots[0].exchange, "上海证券交易所");
        assert_eq!(rows[0].venue_snapshots[0].bid_price, 99.8);
        assert_eq!(rows[0].venue_snapshots[0].ask_price, 100.2);
        assert_eq!(rows[0].venue_snapshots[0].funding_rate, Some(0.01));
        assert_eq!(rows[0].venue_snapshots[1].exchange, "人民币现金");
        assert_eq!(rows[0].venue_snapshots[1].bid_price, 100.9);
        assert_eq!(rows[0].venue_snapshots[1].ask_price, 101.1);
        assert_eq!(rows[0].venue_snapshots[1].funding_rate, Some(0.03));
        assert_eq!(rows[0].best_bid_exchange.as_deref(), Some("人民币现金"));
        assert_eq!(rows[0].best_ask_exchange.as_deref(), Some("上海证券交易所"));
        assert_eq!(rows[0].best_bid_price, Some(100.9));
        assert_eq!(rows[0].best_ask_price, Some(100.2));
    }

    #[test]
    fn primary_venue_row_does_not_use_aggregate_funding_fallback() {
        let rows = build_market_rows_from_batch_tickers(vec![
            BatchTickerSnapshot {
                snapshot: MarketSnapshot {
                    exchange: "akshare".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "perpetual".into(),
                    last_price: 100.0,
                    bid_price: 99.9,
                    ask_price: 100.1,
                    volume_24h: 3000.0,
                    change_24h: 1.0,
                    updated_at: "1000".into(),
                    stale: false,
                },
                funding_rate: None,
            },
            BatchTickerSnapshot {
                snapshot: MarketSnapshot {
                    exchange: "人民币现金".into(),
                    symbol: "BTC/USDT".into(),
                    market_type: "perpetual".into(),
                    last_price: 101.0,
                    bid_price: 100.9,
                    ask_price: 101.1,
                    volume_24h: 2000.0,
                    change_24h: 2.0,
                    updated_at: "2000".into(),
                    stale: false,
                },
                funding_rate: Some(0.03),
            },
        ])
        .expect("batch rows should aggregate");

        let selected = perpetual_primary_venue_rows(&rows, &[]);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].exchanges, vec!["akshare".to_string()]);
        assert_eq!(selected[0].funding_rate, None);
    }

    #[test]
    fn limits_exchange_universe_to_akshare_source() {
        let enabled = vec!["akshare".to_string(), "Unknown".to_string()];
        assert_eq!(enabled_exchange_universe(&enabled), vec!["akshare"]);

        let disabled_all = Vec::<String>::new();
        assert!(enabled_exchange_universe(&disabled_all).is_empty());
    }

    #[test]
    fn dedupes_discovered_market_targets_and_keeps_broader_symbols() {
        let targets = dedupe_market_targets(vec![
            MarketTarget::new("DOGE/USDT", "spot"),
            MarketTarget::new("BTC/USDT", "spot"),
            MarketTarget::new("DOGE/USDT", "spot"),
            MarketTarget::new("DOGE/USDT", "perpetual"),
        ]);

        assert_eq!(
            targets,
            vec![
                MarketTarget::new("BTC/USDT", "spot"),
                MarketTarget::new("DOGE/USDT", "spot"),
                MarketTarget::new("DOGE/USDT", "perpetual"),
            ]
        );
    }

    #[test]
    fn keeps_full_discovered_market_universe_instead_of_capping_scan_targets() {
        let targets = dedupe_market_targets(
            [
                "BTC", "ETH", "SOL", "DOGE", "XRP", "ADA", "AVAX", "LINK", "SUI", "TRX", "TON",
                "BNB", "PEPE", "WIF", "ARB", "APT",
            ]
            .into_iter()
            .flat_map(|base| {
                [
                    MarketTarget::new(format!("{base}/USDT"), "spot"),
                    MarketTarget::new(format!("{base}/USDT"), "perpetual"),
                ]
            })
            .collect(),
        );

        let limited = limited_market_targets(&targets);

        assert_eq!(limited.len(), targets.len());
        assert!(limited.iter().any(|target| target.symbol == "DOGE/USDT"));
        assert!(limited.iter().any(|target| target.symbol == "PEPE/USDT"));
        assert!(limited.iter().any(|target| target.symbol == "WIF/USDT"));
    }

    #[test]
    fn derives_unique_symbol_options_from_targets() {
        let symbols = market_symbols_from_targets(&dedupe_market_targets(vec![
            MarketTarget::new("DOGE/USDT", "spot"),
            MarketTarget::new("DOGE/USDT", "perpetual"),
            MarketTarget::new("BTC/USDT", "spot"),
        ]));

        assert_eq!(
            symbols,
            vec!["BTC/USDT".to_string(), "DOGE/USDT".to_string()]
        );
    }

    #[test]
    fn returns_cached_rows_without_refreshing_live_sources() {
        let service = MarketDataService::default();
        service.cache_market_row(MarketListRow {
            symbol: "BTC/USDT".into(),
            base_asset: "BTC".into(),
            market_type: "perpetual".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: MARKET_SIZE_TIER_SMALL.into(),
            last_price: 68_420.0,
            change_24h: 2.8,
            volume_24h: 180_000_000.0,
            funding_rate: Some(0.011),
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
        });

        let rows = service.cached_market_rows_for_exchanges(&["akshare".into()]);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC/USDT");
        assert_eq!(rows[0].updated_at, "2026-05-04T10:00:00+08:00");
    }

    #[test]
    fn sqlite_ticker_cache_upserts_and_reads_rows() {
        let cache = SqliteMarketTickerCache::in_memory().expect("cache should initialize");
        cache
            .upsert_rows(&[MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "spot".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: MARKET_SIZE_TIER_SMALL.into(),
                last_price: 68_420.0,
                change_24h: 2.8,
                volume_24h: 180_000_000.0,
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
            }])
            .expect("row should upsert");

        let rows = cache.list_rows().expect("rows should load");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "BTC/USDT");
        assert_eq!(
            rows[0].exchanges,
            vec!["akshare".to_string(), "人民币现金".to_string()]
        );
    }

    #[test]
    fn sqlite_ticker_cache_persists_arbitrage_fields() {
        let cache = SqliteMarketTickerCache::in_memory().expect("cache should initialize");
        cache
            .upsert_rows(&[MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "spot".into(),
                market_cap_rank: Some(1),
                market_size_tier: MARKET_SIZE_TIER_LARGE.into(),
                last_price: 68_420.0,
                change_24h: 2.8,
                volume_24h: 180_000_000.0,
                funding_rate: None,
                spread_bps: 3.4,
                exchanges: vec!["akshare".into(), "人民币现金".into()],
                updated_at: "2026-05-04T10:00:00+08:00".into(),
                stale: false,
                venue_snapshots: vec![VenueTickerSnapshot {
                    exchange: "akshare".into(),
                    last_price: 68_420.0,
                    bid_price: 68_418.0,
                    ask_price: 68_421.0,
                    volume_24h: 120_000_000.0,
                    funding_rate: None,
                    mark_price: None,
                    index_price: None,
                    updated_at: "2026-05-04T10:00:00+08:00".into(),
                    stale: false,
                }],
                best_bid_exchange: Some("akshare".into()),
                best_ask_exchange: Some("akshare".into()),
                best_bid_price: Some(68_418.0),
                best_ask_price: Some(68_421.0),
                responded_exchange_count: 1,
                market_cap_usd: Some(1_300_000_000_000.0),
                fdv_usd: Some(1_320_000_000_000.0),
            }])
            .expect("row should upsert");

        let rows = cache.list_rows().expect("rows should load");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].venue_snapshots.len(), 1);
        assert_eq!(rows[0].venue_snapshots[0].exchange, "akshare");
        assert_eq!(rows[0].venue_snapshots[0].last_price, 68_420.0);
        assert_eq!(rows[0].venue_snapshots[0].bid_price, 68_418.0);
        assert_eq!(rows[0].venue_snapshots[0].ask_price, 68_421.0);
        assert_eq!(rows[0].venue_snapshots[0].volume_24h, 120_000_000.0);
        assert_eq!(rows[0].venue_snapshots[0].funding_rate, None);
        assert_eq!(rows[0].venue_snapshots[0].mark_price, None);
        assert_eq!(rows[0].venue_snapshots[0].index_price, None);
        assert_eq!(
            rows[0].venue_snapshots[0].updated_at,
            "2026-05-04T10:00:00+08:00"
        );
        assert!(!rows[0].venue_snapshots[0].stale);
        assert_eq!(rows[0].best_bid_exchange.as_deref(), Some("akshare"));
        assert_eq!(rows[0].best_ask_exchange.as_deref(), Some("akshare"));
        assert_eq!(rows[0].best_bid_price, Some(68_418.0));
        assert_eq!(rows[0].best_ask_price, Some(68_421.0));
        assert_eq!(rows[0].responded_exchange_count, 1);
        assert_eq!(rows[0].market_cap_usd, Some(1_300_000_000_000.0));
        assert_eq!(rows[0].fdv_usd, Some(1_320_000_000_000.0));
    }

    #[tokio::test]
    async fn list_markets_reads_sqlite_cache_without_live_scan() {
        let cache = SqliteMarketTickerCache::in_memory().expect("cache should initialize");
        cache
            .upsert_rows(&[MarketListRow {
                symbol: "ETH/USDT".into(),
                base_asset: "ETH".into(),
                market_type: "perpetual".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: MARKET_SIZE_TIER_SMALL.into(),
                last_price: 3_200.0,
                change_24h: -1.2,
                volume_24h: 90_000_000.0,
                funding_rate: Some(0.01),
                spread_bps: 4.0,
                exchanges: vec!["akshare".into()],
                updated_at: "2026-05-04T10:00:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            }])
            .expect("row should upsert");
        let service = MarketDataService::with_sqlite_cache(cache);

        let rows = service
            .list_markets_for_exchanges(&["akshare".into()])
            .await
            .expect("markets should load");
        let symbols = service
            .list_market_symbols_for_exchanges(&["akshare".into()])
            .await
            .expect("symbols should load");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].symbol, "ETH/USDT");
        assert_eq!(symbols, vec!["ETH/USDT".to_string()]);
    }

    #[tokio::test]
    async fn list_arbitrage_opportunities_reads_sqlite_cache_without_live_scan() {
        let cache = SqliteMarketTickerCache::in_memory().expect("cache should initialize");
        let updated_at = current_rfc3339();
        cache
            .upsert_rows(&[
                MarketListRow {
                    symbol: "BTC/USDT".into(),
                    base_asset: "BTC".into(),
                    market_type: "spot".into(),
                    market_cap_usd: Some(1_300_000_000_000.0),
                    market_cap_rank: Some(1),
                    market_size_tier: MARKET_SIZE_TIER_LARGE.into(),
                    last_price: 68_420.0,
                    change_24h: 2.0,
                    volume_24h: 220_000_000.0,
                    funding_rate: None,
                    spread_bps: 3.0,
                    exchanges: vec!["akshare".into()],
                    updated_at: updated_at.clone(),
                    stale: false,
                    venue_snapshots: vec![VenueTickerSnapshot {
                        exchange: "akshare".into(),
                        last_price: 68_420.0,
                        bid_price: 68_418.0,
                        ask_price: 68_420.0,
                        volume_24h: 220_000_000.0,
                        funding_rate: None,
                        mark_price: None,
                        index_price: None,
                        updated_at: updated_at.clone(),
                        stale: false,
                    }],
                    best_bid_exchange: Some("akshare".into()),
                    best_ask_exchange: Some("akshare".into()),
                    best_bid_price: Some(68_418.0),
                    best_ask_price: Some(68_420.0),
                    responded_exchange_count: 1,
                    fdv_usd: Some(1_320_000_000_000.0),
                },
                MarketListRow {
                    symbol: "BTC/USDT".into(),
                    base_asset: "BTC".into(),
                    market_type: "perpetual".into(),
                    market_cap_usd: Some(1_300_000_000_000.0),
                    market_cap_rank: Some(1),
                    market_size_tier: MARKET_SIZE_TIER_LARGE.into(),
                    last_price: 68_720.0,
                    change_24h: 2.2,
                    volume_24h: 300_000_000.0,
                    funding_rate: Some(0.0008),
                    spread_bps: 3.4,
                    exchanges: vec!["深圳证券交易所".into()],
                    updated_at: updated_at.clone(),
                    stale: false,
                    venue_snapshots: vec![VenueTickerSnapshot {
                        exchange: "深圳证券交易所".into(),
                        last_price: 68_720.0,
                        bid_price: 68_719.0,
                        ask_price: 68_722.0,
                        volume_24h: 300_000_000.0,
                        funding_rate: Some(0.0008),
                        mark_price: Some(68_718.0),
                        index_price: Some(68_421.0),
                        updated_at,
                        stale: false,
                    }],
                    best_bid_exchange: Some("深圳证券交易所".into()),
                    best_ask_exchange: Some("深圳证券交易所".into()),
                    best_bid_price: Some(68_719.0),
                    best_ask_price: Some(68_722.0),
                    responded_exchange_count: 1,
                    fdv_usd: Some(1_320_000_000_000.0),
                },
            ])
            .expect("rows should upsert");
        let service = MarketDataService::with_sqlite_cache(cache);

        let page = service
            .list_arbitrage_opportunities_for_exchanges(&[], 1, 25, "all")
            .await
            .expect("page should load");

        assert!(!page.items.is_empty());
        assert_eq!(page.page, 1);
    }

    #[test]
    fn apply_asset_metadata_enriches_market_rows() {
        let rows = apply_asset_metadata(
            vec![MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "spot".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: MARKET_SIZE_TIER_SMALL.into(),
                last_price: 68_420.0,
                change_24h: 2.8,
                volume_24h: 180_000_000.0,
                funding_rate: None,
                spread_bps: 3.4,
                exchanges: vec!["akshare".into()],
                updated_at: "2026-05-04T10:00:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            }],
            &[AssetMetadataRecord {
                base_asset: "BTC".into(),
                provider: "akshare".into(),
                provider_asset_id: "bitcoin".into(),
                provider_symbol: "BTC".into(),
                provider_name: "Bitcoin".into(),
                market_cap_usd: Some(1_300_000_000_000.0),
                market_cap_rank: Some(1),
                fetched_at: "2026-05-04T08:00:00Z".into(),
                source_updated_at: Some("2026-05-04T07:59:00Z".into()),
            }],
        );

        assert_eq!(rows[0].base_asset, "BTC");
        assert_eq!(rows[0].market_cap_rank, Some(1));
        assert_eq!(rows[0].market_size_tier, MARKET_SIZE_TIER_LARGE);
    }

    #[tokio::test]
    async fn list_markets_enriches_cached_rows_with_sqlite_asset_metadata() {
        let cache = SqliteMarketTickerCache::in_memory().expect("cache should initialize");
        cache
            .upsert_rows(&[MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "spot".into(),
                market_cap_usd: None,
                market_cap_rank: None,
                market_size_tier: MARKET_SIZE_TIER_SMALL.into(),
                last_price: 68_420.0,
                change_24h: 2.8,
                volume_24h: 180_000_000.0,
                funding_rate: None,
                spread_bps: 3.4,
                exchanges: vec!["akshare".into()],
                updated_at: "2026-05-04T10:00:00+08:00".into(),
                stale: false,
                venue_snapshots: Vec::new(),
                best_bid_exchange: None,
                best_ask_exchange: None,
                best_bid_price: None,
                best_ask_price: None,
                responded_exchange_count: 0,
                fdv_usd: None,
            }])
            .expect("row should upsert");
        cache
            .upsert_asset_metadata(&[AssetMetadataRecord {
                base_asset: "BTC".into(),
                provider: "akshare".into(),
                provider_asset_id: "bitcoin".into(),
                provider_symbol: "BTC".into(),
                provider_name: "Bitcoin".into(),
                market_cap_usd: Some(1_300_000_000_000.0),
                market_cap_rank: Some(1),
                fetched_at: "2026-05-04T08:00:00Z".into(),
                source_updated_at: Some("2026-05-04T07:59:00Z".into()),
            }])
            .expect("metadata should upsert");
        let service = MarketDataService::with_sqlite_cache(cache);

        let rows = service
            .list_markets_for_exchanges(&["akshare".into()])
            .await
            .expect("markets should load");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].market_cap_rank, Some(1));
        assert_eq!(rows[0].market_size_tier, MARKET_SIZE_TIER_LARGE);
    }

    #[test]
    fn aggregate_recent_trades_groups_by_source_side_price_and_sorts_sell_then_buy() {
        let trades = aggregate_recent_trades(vec![
            RecentTrade {
                exchange: "人民币现金".into(),
                side: "sell".into(),
                price: 101.0,
                size: 0.6,
                timestamp: "2026-05-04T10:00:00Z".into(),
            },
            RecentTrade {
                exchange: "人民币现金".into(),
                side: "sell".into(),
                price: 101.0,
                size: 0.4,
                timestamp: "2026-05-04T10:00:02Z".into(),
            },
            RecentTrade {
                exchange: "akshare".into(),
                side: "sell".into(),
                price: 100.0,
                size: 0.3,
                timestamp: "2026-05-04T10:00:01Z".into(),
            },
            RecentTrade {
                exchange: "akshare".into(),
                side: "buy".into(),
                price: 99.0,
                size: 0.2,
                timestamp: "2026-05-04T10:00:03Z".into(),
            },
            RecentTrade {
                exchange: "akshare".into(),
                side: "buy".into(),
                price: 98.0,
                size: 0.5,
                timestamp: "2026-05-04T10:00:04Z".into(),
            },
        ]);

        assert_eq!(trades.len(), 4);
        assert_eq!(trades[0].side, "sell");
        assert_eq!(trades[0].price, 100.0);
        assert_eq!(trades[1].size, 1.0);
        assert_eq!(trades[2].side, "buy");
        assert_eq!(trades[2].price, 99.0);
        assert_eq!(trades[3].price, 98.0);
    }
}

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::anyhow;
use asset_metadata::{
    base_asset_from_symbol, classify_market_size_tier, current_metadata_timestamp,
    fetch_asset_metadata, metadata_refresh_is_due, AssetMetadataRecord,
};
use cache::{MarketCache, SqliteMarketTickerCache};
use futures::future::join_all;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::models::{
    AShareSymbolSearchResultDto, ArbitrageOpportunityPageDto, CandleSeriesDto, DerivativesSnapshot,
    MarketListRow, MarketSnapshot, OhlcvBar, OrderBookVenueSnapshot, PairDetailDto,
    PairVenueSnapshot, RecentTrade, SpreadOpportunityDto, VenueTickerSnapshot,
};

const DEFAULT_MARKET_SYMBOLS: [&str; 3] = ["SHSE.600000", "SZSE.000001", "SHSE.600519"];
const PRIORITY_SYMBOLS: [&str; 3] = DEFAULT_MARKET_SYMBOLS;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MarketTarget {
    pub symbol: String,
    pub market_type: String,
}

impl MarketTarget {
    fn new(symbol: impl Into<String>, market_type: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            market_type: market_type.into(),
        }
    }
}

#[derive(Clone)]
pub struct MarketDataService {
    #[allow(dead_code)]
    cache: Arc<MarketCache>,
    ticker_cache: Option<Arc<Mutex<SqliteMarketTickerCache>>>,
    static_rows: Option<Arc<Vec<MarketListRow>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetMetadataRefreshResult {
    pub row_count: usize,
    pub partial: bool,
}

impl Default for MarketDataService {
    fn default() -> Self {
        Self {
            cache: Arc::new(MarketCache::default()),
            ticker_cache: None,
            static_rows: None,
        }
    }
}

impl MarketDataService {
    pub fn new(ticker_cache_path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            cache: Arc::new(MarketCache::default()),
            ticker_cache: Some(Arc::new(Mutex::new(SqliteMarketTickerCache::open(
                &ticker_cache_path,
            )?))),
            static_rows: None,
        })
    }

    #[cfg(test)]
    pub fn with_static_rows(rows: Vec<MarketListRow>) -> Self {
        Self {
            cache: Arc::new(MarketCache::default()),
            ticker_cache: None,
            static_rows: Some(Arc::new(rows)),
        }
    }

    #[cfg(test)]
    pub fn with_sqlite_cache(cache: SqliteMarketTickerCache) -> Self {
        Self {
            cache: Arc::new(MarketCache::default()),
            ticker_cache: Some(Arc::new(Mutex::new(cache))),
            static_rows: None,
        }
    }

    pub async fn list_markets(&self) -> anyhow::Result<Vec<MarketListRow>> {
        let enabled = live::supported_exchanges()
            .iter()
            .map(|exchange| (*exchange).to_string())
            .collect::<Vec<_>>();
        self.list_markets_for_exchanges(&enabled).await
    }

    pub async fn list_markets_for_exchanges(
        &self,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<Vec<MarketListRow>> {
        if let Some(rows) = &self.static_rows {
            return Ok(rows.as_ref().clone());
        }

        Ok(self.cached_market_rows_for_exchanges(enabled_exchanges))
    }

    pub fn asset_metadata_refresh_is_due(&self) -> bool {
        let Some(cache) = &self.ticker_cache else {
            return false;
        };
        let last_fetched_at = cache
            .lock()
            .expect("market ticker cache lock poisoned")
            .latest_asset_metadata_fetched_at()
            .ok()
            .flatten();
        metadata_refresh_is_due(last_fetched_at.as_deref(), &current_metadata_timestamp())
    }

    pub async fn refresh_asset_metadata(&self) -> anyhow::Result<AssetMetadataRefreshResult> {
        let Some(cache) = &self.ticker_cache else {
            return Ok(AssetMetadataRefreshResult {
                row_count: 0,
                partial: false,
            });
        };
        let outcome = fetch_asset_metadata(&live::http_client()).await?;
        cache
            .lock()
            .expect("market ticker cache lock poisoned")
            .upsert_asset_metadata(&outcome.rows)?;
        Ok(AssetMetadataRefreshResult {
            row_count: outcome.rows.len(),
            partial: outcome.partial,
        })
    }

    pub async fn refresh_ticker_cache_for_exchanges(
        &self,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<Vec<MarketListRow>> {
        if let Some(rows) = &self.static_rows {
            return Ok(rows.as_ref().clone());
        }

        let client = live::http_client();
        let tasks = enabled_exchange_universe(enabled_exchanges)
            .into_iter()
            .map(|exchange| {
                let client = client.clone();
                async move { live::fetch_batch_market_snapshots(&client, exchange).await }
            });
        let batch_items = join_all(tasks)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect::<Vec<_>>();
        let rows = build_market_rows_from_batch_tickers(batch_items)?;

        self.cache_market_rows(&rows);
        self.persist_market_rows(&rows)?;

        Ok(rows)
    }

    pub async fn refresh_ticker_cache_from_akshare(
        &self,
        watchlist_symbols: &[String],
    ) -> anyhow::Result<Vec<MarketListRow>> {
        let symbols = normalize_a_share_watchlist(watchlist_symbols);
        if symbols.is_empty() {
            return Ok(Vec::new());
        }

        let quotes = akshare::fetch_current_quotes(&symbols)?;
        let rows = quotes
            .into_iter()
            .map(akshare_quote_to_market_row)
            .collect::<Vec<_>>();

        self.cache_market_rows(&rows);
        self.persist_market_rows(&rows)?;

        Ok(rows)
    }

    pub fn refresh_a_share_symbol_cache_from_akshare(&self) -> anyhow::Result<usize> {
        let symbols = akshare::fetch_stock_universe()?;
        if let Some(cache) = &self.ticker_cache {
            cache
                .lock()
                .expect("market ticker cache lock poisoned")
                .upsert_a_share_symbols(&symbols)?;
        }
        Ok(symbols.len())
    }

    pub fn search_a_share_symbols(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<AShareSymbolSearchResultDto>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        if let Some(cache) = &self.ticker_cache {
            let cache = cache.lock().expect("market ticker cache lock poisoned");
            if cache.a_share_symbol_count()? > 0 {
                return cache.search_a_share_symbols(query, 20);
            }
        }

        let symbols = akshare::fetch_stock_universe()?;
        if let Some(cache) = &self.ticker_cache {
            let cache = cache.lock().expect("market ticker cache lock poisoned");
            cache.upsert_a_share_symbols(&symbols)?;
            return cache.search_a_share_symbols(query, 20);
        }

        Ok(search_a_share_symbols_in_memory(&symbols, query, 20))
    }

    pub fn cached_market_rows_for_exchanges(
        &self,
        enabled_exchanges: &[String],
    ) -> Vec<MarketListRow> {
        if let Some(rows) = &self.static_rows {
            return rows.as_ref().clone();
        }

        if let Some(cache) = &self.ticker_cache {
            let cached_rows = {
                cache
                    .lock()
                    .expect("market ticker cache lock poisoned")
                    .list_rows()
            };
            if let Ok(rows) = cached_rows {
                return apply_asset_metadata(
                    filter_market_rows_for_exchanges(rows, enabled_exchanges),
                    &self.list_asset_metadata_rows(),
                );
            }
        }

        let enabled = enabled_exchange_universe(enabled_exchanges);
        let mut rows = self
            .cache
            .rows
            .iter()
            .filter_map(|entry| {
                let row = entry.value().clone();
                if enabled.is_empty()
                    || row
                        .exchanges
                        .iter()
                        .any(|exchange| enabled.iter().any(|item| exchange == item))
                {
                    Some(row)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            left.symbol
                .cmp(&right.symbol)
                .then(left.market_type.cmp(&right.market_type))
        });
        apply_asset_metadata(rows, &self.list_asset_metadata_rows())
    }

    pub fn cached_market_rows_for_watchlist(
        &self,
        watchlist_symbols: &[String],
    ) -> Vec<MarketListRow> {
        let watchlist = normalize_a_share_watchlist(watchlist_symbols);
        if watchlist.is_empty() {
            return Vec::new();
        }
        let watchlist_set = watchlist.into_iter().collect::<HashSet<_>>();
        let mut rows = self
            .cached_market_rows_for_exchanges(&[])
            .into_iter()
            .filter(|row| watchlist_set.contains(&row.symbol))
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        rows
    }

    pub fn cache_candle_bars(
        &self,
        symbol: &str,
        interval: &str,
        bars: &[OhlcvBar],
    ) -> anyhow::Result<()> {
        if let Some(cache) = &self.ticker_cache {
            cache
                .lock()
                .expect("market ticker cache lock poisoned")
                .upsert_candles(symbol, interval, bars)?;
        }
        Ok(())
    }

    pub fn cached_candle_bars(&self, symbol: &str, interval: &str, limit: usize) -> Vec<OhlcvBar> {
        let Some(cache) = &self.ticker_cache else {
            return Vec::new();
        };
        cache
            .lock()
            .expect("market ticker cache lock poisoned")
            .list_candles(symbol, interval, limit)
            .unwrap_or_default()
    }

    pub fn cached_market_row_for_exchanges(
        &self,
        symbol: &str,
        market_type: &str,
        enabled_exchanges: &[String],
    ) -> Option<MarketListRow> {
        let rows = self.cached_market_rows_for_exchanges(enabled_exchanges);
        rows.into_iter()
            .find(|row| row.symbol == symbol && row.market_type.eq_ignore_ascii_case(market_type))
            .or_else(|| {
                self.cached_market_rows_for_exchanges(enabled_exchanges)
                    .into_iter()
                    .find(|row| row.symbol == symbol)
            })
    }

    pub async fn get_pair_detail(
        &self,
        symbol: &str,
        market_type: &str,
    ) -> anyhow::Result<PairDetailDto> {
        let enabled = live::supported_exchanges()
            .iter()
            .map(|exchange| (*exchange).to_string())
            .collect::<Vec<_>>();
        self.get_pair_detail_for_exchanges(symbol, market_type, None, &enabled)
            .await
    }

    pub async fn get_pair_detail_for_exchanges(
        &self,
        symbol: &str,
        market_type: &str,
        _detail_exchange: Option<&str>,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<PairDetailDto> {
        let exchange_universe = enabled_exchange_universe(enabled_exchanges);
        let (snapshots, errors) = self
            .fetch_snapshots(symbol, market_type, enabled_exchanges)
            .await;
        if snapshots.is_empty() {
            return Err(anyhow!(
                "no live market sources available for {symbol} {market_type}"
            ));
        }

        let derivatives = if market_type == "perpetual" {
            self.fetch_derivatives(symbol, &snapshots).await
        } else {
            HashMap::new()
        };

        let listed_exchanges = snapshots
            .iter()
            .map(|snapshot| snapshot.exchange.clone())
            .collect::<Vec<_>>();
        let coin_info = coin_info::get_coin_info(symbol, &listed_exchanges);

        let venues = snapshots
            .iter()
            .map(|snapshot| {
                let derivatives_snapshot = derivatives.get(&snapshot.exchange);
                PairVenueSnapshot {
                    exchange: snapshot.exchange.clone(),
                    last_price: snapshot.last_price,
                    bid_price: snapshot.bid_price,
                    ask_price: snapshot.ask_price,
                    change_pct: snapshot.change_24h,
                    volume_24h: snapshot.volume_24h,
                    funding_rate: derivatives_snapshot.map(|item| item.funding_rate),
                    mark_price: derivatives_snapshot.map(|item| item.mark_price),
                    index_price: derivatives_snapshot.map(|item| item.index_price),
                    open_interest: derivatives_snapshot.map(|item| item.open_interest.clone()),
                    next_funding_at: derivatives_snapshot.map(|item| item.next_funding_at.clone()),
                    updated_at: snapshot.updated_at.clone(),
                }
            })
            .collect();

        let client = live::http_client();
        let orderbooks =
            live::fetch_all_orderbooks(&client, symbol, market_type, &exchange_universe)
                .await
                .into_iter()
                .map(|(exchange, snapshot)| OrderBookVenueSnapshot {
                    exchange,
                    bids: snapshot.bids,
                    asks: snapshot.asks,
                    updated_at: snapshot.updated_at,
                })
                .collect::<Vec<_>>();
        let recent_trades = aggregate_recent_trades(
            live::fetch_all_trades(&client, symbol, market_type, &exchange_universe)
                .await
                .into_iter()
                .flat_map(|(_, trades)| trades)
                .collect(),
        );
        let spreads = build_spread_opportunity(symbol, market_type, &snapshots)
            .into_iter()
            .collect();

        let mut thesis = coin_info::describe_coin(symbol);
        thesis.push_str(&format!(
            " Live coverage: {} of {} supported exchanges responded for {}.",
            snapshots.len(),
            exchange_universe.len(),
            market_type
        ));
        if !errors.is_empty() {
            thesis.push_str(" Unavailable sources: ");
            thesis.push_str(&errors.join("; "));
            thesis.push('.');
        }
        let source_note = format!(
            "Venue matrix aggregates {} of {} enabled exchanges. Order books: {} sources. Recent trades: {} sources.",
            snapshots.len(),
            exchange_universe.len(),
            orderbooks.len(),
            recent_trades
                .iter()
                .map(|trade| trade.exchange.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len()
        );

        Ok(PairDetailDto {
            symbol: symbol.to_string(),
            market_type: market_type.to_string(),
            thesis,
            source_note,
            coin_info,
            venues,
            orderbooks,
            recent_trades,
            spreads,
        })
    }

    pub async fn get_market_row_for_exchanges(
        &self,
        symbol: &str,
        market_type: &str,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<MarketListRow> {
        let (snapshots, errors) = self
            .fetch_snapshots(symbol, market_type, enabled_exchanges)
            .await;
        if snapshots.is_empty() {
            return Err(anyhow!(
                "no live market sources available for {symbol} {market_type}"
            ));
        }

        let mut row = build_market_row(symbol, market_type, &snapshots)?;
        row.stale = !errors.is_empty() || row.stale;

        if market_type == "perpetual" {
            if let Some(primary) = snapshots
                .iter()
                .max_by(|left, right| left.volume_24h.total_cmp(&right.volume_24h))
            {
                if let Ok(derivatives) = live::fetch_derivatives_snapshot(
                    &live::http_client(),
                    &primary.exchange,
                    symbol,
                )
                .await
                {
                    row.funding_rate = Some(derivatives.funding_rate);
                }
            }
        }

        Ok(self.apply_asset_metadata_to_row(row))
    }

    pub async fn list_spreads(&self) -> anyhow::Result<Vec<SpreadOpportunityDto>> {
        let enabled = live::supported_exchanges()
            .iter()
            .map(|exchange| (*exchange).to_string())
            .collect::<Vec<_>>();
        self.list_spreads_for_exchanges(&enabled).await
    }

    pub async fn list_spreads_for_exchanges(
        &self,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<Vec<SpreadOpportunityDto>> {
        let targets = self.market_targets_for_scan(enabled_exchanges).await;
        let mut items = Vec::new();

        for target in targets {
            let (snapshots, _) = self
                .fetch_snapshots(&target.symbol, &target.market_type, enabled_exchanges)
                .await;
            if let Some(opportunity) =
                build_spread_opportunity(&target.symbol, &target.market_type, &snapshots)
            {
                if opportunity.net_spread_pct > 0.0 {
                    items.push(opportunity);
                }
            }
        }

        items.sort_by(|left, right| right.net_spread_pct.total_cmp(&left.net_spread_pct));
        Ok(items)
    }

    pub async fn list_arbitrage_opportunities_for_exchanges(
        &self,
        enabled_exchanges: &[String],
        page: usize,
        page_size: usize,
        type_filter: &str,
    ) -> anyhow::Result<ArbitrageOpportunityPageDto> {
        let rows = if let Some(static_rows) = &self.static_rows {
            static_rows.as_ref().clone()
        } else {
            self.cached_market_rows_for_exchanges(enabled_exchanges)
        };
        let filter = arbitrage::ArbitrageTypeFilter::from_query(type_filter);
        let items = arbitrage::build_arbitrage_candidates(&rows, filter);
        Ok(arbitrage::paginate_candidates(items, page, page_size))
    }

    pub async fn get_pair_candles(
        &self,
        symbol: &str,
        market_type: &str,
        interval: &str,
        exchange: Option<&str>,
    ) -> anyhow::Result<CandleSeriesDto> {
        let enabled = live::supported_exchanges()
            .iter()
            .map(|item| (*item).to_string())
            .collect::<Vec<_>>();
        self.get_pair_candles_for_exchanges(symbol, market_type, interval, exchange, &enabled)
            .await
    }

    pub async fn get_pair_candles_for_exchanges(
        &self,
        symbol: &str,
        market_type: &str,
        interval: &str,
        exchange: Option<&str>,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<CandleSeriesDto> {
        let client = live::http_client();
        let requested_exchange = exchange.filter(|value| !value.eq_ignore_ascii_case("auto"));
        let exchange_universe = enabled_exchange_universe(enabled_exchanges);
        let (resolved_exchange, bars) = live::fetch_candles(
            &client,
            symbol,
            market_type,
            interval,
            requested_exchange,
            &exchange_universe,
        )
        .await?;
        let updated_at = bars
            .last()
            .map(|bar| bar.open_time.clone())
            .unwrap_or_default();

        Ok(CandleSeriesDto {
            exchange: resolved_exchange,
            symbol: symbol.to_string(),
            market_type: market_type.to_string(),
            interval: interval.to_string(),
            bars,
            updated_at,
        })
    }

    pub async fn list_market_symbols(&self) -> anyhow::Result<Vec<String>> {
        let enabled = live::supported_exchanges()
            .iter()
            .map(|exchange| (*exchange).to_string())
            .collect::<Vec<_>>();
        self.list_market_symbols_for_exchanges(&enabled).await
    }

    pub async fn list_market_symbols_for_exchanges(
        &self,
        enabled_exchanges: &[String],
    ) -> anyhow::Result<Vec<String>> {
        if let Some(rows) = &self.static_rows {
            let targets = rows
                .iter()
                .map(|row| MarketTarget::new(row.symbol.clone(), row.market_type.clone()))
                .collect::<Vec<_>>();
            return Ok(market_symbols_from_targets(&dedupe_market_targets(targets)));
        }

        let rows = self.cached_market_rows_for_exchanges(enabled_exchanges);
        let targets = rows
            .iter()
            .map(|row| MarketTarget::new(row.symbol.clone(), row.market_type.clone()))
            .collect::<Vec<_>>();
        Ok(market_symbols_from_targets(&dedupe_market_targets(targets)))
    }

    async fn fetch_snapshots(
        &self,
        symbol: &str,
        market_type: &str,
        enabled_exchanges: &[String],
    ) -> (Vec<MarketSnapshot>, Vec<String>) {
        let client = live::http_client();
        let tasks = enabled_exchange_universe(enabled_exchanges)
            .into_iter()
            .map(|exchange| {
                let client = client.clone();
                let symbol = symbol.to_string();
                let market_type = market_type.to_string();

                async move {
                    let result =
                        live::fetch_market_snapshot(&client, exchange, &symbol, &market_type).await;
                    (exchange.to_string(), result)
                }
            });

        let results = join_all(tasks).await;
        let mut snapshots = Vec::new();
        let mut errors = Vec::new();

        for (exchange, result) in results {
            match result {
                Ok(snapshot) => {
                    self.cache_snapshot(&snapshot);
                    snapshots.push(snapshot);
                }
                Err(error) => errors.push(format!("{exchange}: {error}")),
            }
        }

        snapshots.sort_by(|left, right| left.exchange.cmp(&right.exchange));

        (snapshots, errors)
    }

    fn cache_snapshot(&self, snapshot: &MarketSnapshot) {
        self.cache
            .snapshots
            .insert(snapshot_cache_key(snapshot), snapshot.clone());
    }

    pub fn cache_market_row(&self, row: MarketListRow) {
        self.cache.rows.insert(row_cache_key(&row), row);
    }

    fn cache_market_rows(&self, rows: &[MarketListRow]) {
        for row in rows {
            self.cache_market_row(row.clone());
        }
    }

    fn persist_market_rows(&self, rows: &[MarketListRow]) -> anyhow::Result<()> {
        if let Some(cache) = &self.ticker_cache {
            cache
                .lock()
                .expect("market ticker cache lock poisoned")
                .upsert_rows(rows)?;
        }
        Ok(())
    }

    fn list_asset_metadata_rows(&self) -> Vec<AssetMetadataRecord> {
        let Some(cache) = &self.ticker_cache else {
            return Vec::new();
        };
        cache
            .lock()
            .expect("market ticker cache lock poisoned")
            .list_asset_metadata()
            .unwrap_or_default()
    }

    fn apply_asset_metadata_to_row(&self, row: MarketListRow) -> MarketListRow {
        apply_asset_metadata(vec![row], &self.list_asset_metadata_rows())
            .into_iter()
            .next()
            .expect("single market row should remain present")
    }

    async fn fetch_derivatives(
        &self,
        symbol: &str,
        snapshots: &[MarketSnapshot],
    ) -> HashMap<String, DerivativesSnapshot> {
        let client = live::http_client();
        let tasks = snapshots.iter().map(|snapshot| {
            let client = client.clone();
            let exchange = snapshot.exchange.clone();
            let symbol = symbol.to_string();

            async move {
                let result = live::fetch_derivatives_snapshot(&client, &exchange, &symbol).await;
                (exchange, result)
            }
        });

        let results = join_all(tasks).await;
        let mut map = HashMap::new();

        for (exchange, result) in results {
            if let Ok(snapshot) = result {
                map.insert(exchange, snapshot);
            }
        }

        map
    }

    async fn discovered_market_targets(&self, enabled_exchanges: &[String]) -> Vec<MarketTarget> {
        let exchange_universe = enabled_exchange_universe(enabled_exchanges);
        if exchange_universe.is_empty() {
            return Vec::new();
        }

        let discovered = live::discover_market_targets(&exchange_universe)
            .await
            .unwrap_or_default();
        let targets = dedupe_market_targets(discovered);

        if targets.is_empty() {
            fallback_market_targets()
        } else {
            targets
        }
    }

    async fn market_targets_for_scan(&self, enabled_exchanges: &[String]) -> Vec<MarketTarget> {
        limited_market_targets(&self.discovered_market_targets(enabled_exchanges).await)
    }
}

fn filter_market_rows_for_exchanges(
    mut rows: Vec<MarketListRow>,
    enabled_exchanges: &[String],
) -> Vec<MarketListRow> {
    let enabled = enabled_exchange_universe(enabled_exchanges);
    rows.retain(|row| {
        enabled.is_empty()
            || row
                .exchanges
                .iter()
                .any(|exchange| enabled.iter().any(|item| exchange == item))
    });
    rows.sort_by(|left, right| {
        left.symbol
            .cmp(&right.symbol)
            .then(left.market_type.cmp(&right.market_type))
    });
    rows
}

fn normalize_a_share_watchlist(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .iter()
        .map(|value| value.trim().to_uppercase())
        .filter(|value| {
            !value.is_empty()
                && (value.starts_with("SHSE.") || value.starts_with("SZSE."))
                && seen.insert(value.clone())
        })
        .collect()
}

fn akshare_quote_to_market_row(quote: akshare::AkshareQuote) -> MarketListRow {
    let change_24h = quote.change_pct.unwrap_or_else(|| {
        if quote.open.abs() > f64::EPSILON {
            ((quote.last - quote.open) / quote.open) * 100.0
        } else {
            0.0
        }
    });
    let bid_price = if quote.last > 0.0 {
        quote.last
    } else {
        quote.low
    };
    let ask_price = if quote.last > 0.0 {
        quote.last
    } else {
        quote.high
    };
    let updated_at = if quote.updated_at.trim().is_empty() {
        current_rfc3339_timestamp()
    } else {
        quote.updated_at
    };
    let display_name = quote
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| quote.symbol.clone());
    let venue = VenueTickerSnapshot {
        exchange: "akshare".into(),
        last_price: quote.last,
        bid_price,
        ask_price,
        volume_24h: quote.amount,
        funding_rate: None,
        mark_price: None,
        index_price: None,
        updated_at: updated_at.clone(),
        stale: false,
    };

    MarketListRow {
        symbol: quote.symbol.clone(),
        base_asset: display_name,
        market_type: "ashare".into(),
        market_cap_usd: None,
        market_cap_rank: None,
        market_size_tier: "watchlist".into(),
        last_price: quote.last,
        change_24h,
        volume_24h: quote.amount,
        funding_rate: None,
        spread_bps: 0.0,
        exchanges: vec!["akshare".into()],
        updated_at,
        stale: false,
        venue_snapshots: vec![venue],
        best_bid_exchange: Some("akshare".into()),
        best_ask_exchange: Some("akshare".into()),
        best_bid_price: Some(bid_price),
        best_ask_price: Some(ask_price),
        responded_exchange_count: 1,
        fdv_usd: None,
    }
}

fn search_a_share_symbols_in_memory(
    symbols: &[AShareSymbolSearchResultDto],
    query: &str,
    limit: usize,
) -> Vec<AShareSymbolSearchResultDto> {
    let cleaned = query.trim().to_uppercase();
    symbols
        .iter()
        .filter(|item| {
            item.symbol.to_uppercase().contains(&cleaned)
                || item
                    .symbol
                    .replace('.', "")
                    .to_uppercase()
                    .contains(&cleaned)
                || item.name.to_uppercase().contains(&cleaned)
        })
        .take(limit)
        .cloned()
        .collect()
}

fn current_rfc3339_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::new())
}

pub(crate) fn perpetual_primary_venue_rows(
    rows: &[MarketListRow],
    enabled_exchanges: &[String],
) -> Vec<MarketListRow> {
    rows.iter()
        .filter_map(|row| primary_venue_perpetual_row(row, enabled_exchanges))
        .collect()
}

pub(crate) fn perpetual_row_for_exchange(
    row: &MarketListRow,
    exchange: &str,
) -> Option<MarketListRow> {
    if !row.market_type.eq_ignore_ascii_case("perpetual") {
        return None;
    }

    let venue = row
        .venue_snapshots
        .iter()
        .find(|venue| venue.exchange.eq_ignore_ascii_case(exchange))?
        .clone();
    Some(perpetual_row_from_venue(row, venue))
}

pub(crate) fn preferred_venue_order(
    row: &MarketListRow,
    enabled_exchanges: &[String],
) -> Vec<String> {
    let mut order = sorted_enabled_venues(row, enabled_exchanges)
        .into_iter()
        .map(|venue| venue.exchange)
        .collect::<Vec<_>>();

    for exchange in &row.exchanges {
        if exchange_is_enabled(exchange, enabled_exchanges)
            && !order
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(exchange))
        {
            order.push(exchange.clone());
        }
    }

    order
}

fn primary_venue_perpetual_row(
    row: &MarketListRow,
    enabled_exchanges: &[String],
) -> Option<MarketListRow> {
    if !row.market_type.eq_ignore_ascii_case("perpetual") {
        return None;
    }

    if let Some(primary) = sorted_enabled_venues(row, enabled_exchanges)
        .into_iter()
        .next()
    {
        return Some(perpetual_row_from_venue(row, primary));
    }

    let primary_exchange = preferred_venue_order(row, enabled_exchanges)
        .into_iter()
        .next()?;
    let mut selected = row.clone();
    selected.exchanges = vec![primary_exchange.clone()];
    selected
        .venue_snapshots
        .retain(|venue| venue.exchange.eq_ignore_ascii_case(&primary_exchange));
    selected.best_bid_exchange = Some(primary_exchange.clone());
    selected.best_ask_exchange = Some(primary_exchange);
    selected.responded_exchange_count = 1;
    Some(selected)
}

fn perpetual_row_from_venue(row: &MarketListRow, primary: VenueTickerSnapshot) -> MarketListRow {
    let mut selected = row.clone();
    selected.last_price = primary.last_price;
    selected.volume_24h = primary.volume_24h;
    selected.funding_rate = primary.funding_rate;
    selected.spread_bps = single_venue_spread_bps(primary.bid_price, primary.ask_price);
    selected.exchanges = vec![primary.exchange.clone()];
    selected.updated_at = primary.updated_at.clone();
    selected.stale = primary.stale;
    selected.venue_snapshots = vec![primary.clone()];
    selected.best_bid_exchange = Some(primary.exchange.clone());
    selected.best_ask_exchange = Some(primary.exchange.clone());
    selected.best_bid_price = Some(primary.bid_price);
    selected.best_ask_price = Some(primary.ask_price);
    selected.responded_exchange_count = 1;
    selected
}

fn sorted_enabled_venues(
    row: &MarketListRow,
    enabled_exchanges: &[String],
) -> Vec<VenueTickerSnapshot> {
    let mut venues = row
        .venue_snapshots
        .iter()
        .filter(|venue| exchange_is_enabled(&venue.exchange, enabled_exchanges))
        .cloned()
        .collect::<Vec<_>>();
    venues.sort_by(|left, right| {
        right
            .volume_24h
            .total_cmp(&left.volume_24h)
            .then(right.updated_at.cmp(&left.updated_at))
            .then(left.exchange.cmp(&right.exchange))
    });
    venues
}

fn exchange_is_enabled(exchange: &str, enabled_exchanges: &[String]) -> bool {
    enabled_exchanges.is_empty()
        || enabled_exchanges
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(exchange))
}

fn single_venue_spread_bps(bid_price: f64, ask_price: f64) -> f64 {
    if bid_price <= 0.0 || ask_price <= 0.0 || ask_price < bid_price {
        return 0.0;
    }

    let mid = (bid_price + ask_price) / 2.0;
    if mid <= 0.0 {
        0.0
    } else {
        ((ask_price - bid_price) / mid) * 10_000.0
    }
}

fn enabled_exchange_universe(enabled_exchanges: &[String]) -> Vec<&'static str> {
    live::supported_exchanges()
        .iter()
        .copied()
        .filter(|exchange| {
            enabled_exchanges
                .iter()
                .any(|item| item.eq_ignore_ascii_case(exchange))
        })
        .collect()
}

fn row_cache_key(row: &MarketListRow) -> String {
    format!("{}::{}", row.symbol, row.market_type.to_lowercase())
}

fn snapshot_cache_key(snapshot: &MarketSnapshot) -> String {
    format!(
        "{}::{}::{}",
        snapshot.exchange,
        snapshot.symbol,
        snapshot.market_type.to_lowercase()
    )
}

pub fn build_market_row(
    symbol: &str,
    market_type: &str,
    snapshots: &[MarketSnapshot],
) -> anyhow::Result<MarketListRow> {
    if snapshots.is_empty() {
        return Err(anyhow!("cannot build market row without snapshots"));
    }

    let last_price =
        snapshots.iter().map(|item| item.last_price).sum::<f64>() / snapshots.len() as f64;
    let change_24h =
        snapshots.iter().map(|item| item.change_24h).sum::<f64>() / snapshots.len() as f64;
    let volume_24h = snapshots.iter().map(|item| item.volume_24h).sum::<f64>();
    let best_bid_snapshot = snapshots
        .iter()
        .max_by(|left, right| left.bid_price.total_cmp(&right.bid_price));
    let best_ask_snapshot = snapshots
        .iter()
        .min_by(|left, right| left.ask_price.total_cmp(&right.ask_price));
    let best_bid = best_bid_snapshot
        .map(|item| item.bid_price)
        .unwrap_or_default();
    let best_ask = best_ask_snapshot
        .map(|item| item.ask_price)
        .unwrap_or_default();
    let denominator = snapshots
        .iter()
        .map(|item| item.last_price)
        .fold(f64::MIN, f64::max);
    let spread_bps = if denominator.is_finite() && denominator > 0.0 && best_bid > best_ask {
        ((best_bid - best_ask) / denominator) * 10_000.0
    } else {
        0.0
    };
    let updated_at = snapshots
        .iter()
        .map(|item| item.updated_at.clone())
        .max()
        .unwrap_or_default();
    let mut exchanges = snapshots
        .iter()
        .map(|item| item.exchange.clone())
        .collect::<Vec<_>>();
    exchanges.sort();
    let venue_snapshots = snapshots
        .iter()
        .map(|item| VenueTickerSnapshot {
            exchange: item.exchange.clone(),
            last_price: item.last_price,
            bid_price: item.bid_price,
            ask_price: item.ask_price,
            volume_24h: item.volume_24h,
            funding_rate: None,
            mark_price: None,
            index_price: None,
            updated_at: item.updated_at.clone(),
            stale: item.stale,
        })
        .collect();

    Ok(MarketListRow {
        symbol: symbol.to_string(),
        base_asset: base_asset_from_symbol(symbol),
        market_type: market_type.to_string(),
        market_cap_usd: None,
        market_cap_rank: None,
        market_size_tier: classify_market_size_tier(None).into(),
        last_price,
        change_24h,
        volume_24h,
        funding_rate: None,
        spread_bps,
        exchanges,
        updated_at,
        stale: snapshots.iter().any(|item| item.stale),
        venue_snapshots,
        best_bid_exchange: best_bid_snapshot.map(|item| item.exchange.clone()),
        best_ask_exchange: best_ask_snapshot.map(|item| item.exchange.clone()),
        best_bid_price: best_bid_snapshot.map(|item| item.bid_price),
        best_ask_price: best_ask_snapshot.map(|item| item.ask_price),
        responded_exchange_count: snapshots.len() as u32,
        fdv_usd: None,
    })
}

fn build_market_rows_from_batch_tickers(
    items: Vec<live::BatchTickerSnapshot>,
) -> anyhow::Result<Vec<MarketListRow>> {
    let mut grouped: HashMap<(String, String), Vec<live::BatchTickerSnapshot>> = HashMap::new();
    for item in items {
        grouped
            .entry((
                item.snapshot.symbol.clone(),
                item.snapshot.market_type.to_lowercase(),
            ))
            .or_default()
            .push(item);
    }

    let mut rows = Vec::new();
    for ((symbol, market_type), items) in grouped {
        let snapshots = items
            .iter()
            .map(|item| item.snapshot.clone())
            .collect::<Vec<_>>();
        let mut row = build_market_row(&symbol, &market_type, &snapshots)?;
        if market_type == "perpetual" {
            let funding_by_exchange = items
                .iter()
                .filter_map(|item| {
                    item.funding_rate
                        .map(|funding_rate| (item.snapshot.exchange.clone(), funding_rate))
                })
                .collect::<HashMap<_, _>>();
            for venue_snapshot in &mut row.venue_snapshots {
                venue_snapshot.funding_rate =
                    funding_by_exchange.get(&venue_snapshot.exchange).copied();
            }
            let funding_rates = items
                .iter()
                .filter_map(|item| item.funding_rate)
                .collect::<Vec<_>>();
            if !funding_rates.is_empty() {
                row.funding_rate =
                    Some(funding_rates.iter().sum::<f64>() / funding_rates.len() as f64);
            }
        }
        rows.push(row);
    }

    rows.sort_by(|left, right| {
        left.symbol
            .cmp(&right.symbol)
            .then(left.market_type.cmp(&right.market_type))
    });
    Ok(rows)
}

fn apply_asset_metadata(
    rows: Vec<MarketListRow>,
    metadata_rows: &[AssetMetadataRecord],
) -> Vec<MarketListRow> {
    let metadata_by_asset = metadata_rows
        .iter()
        .map(|row| (row.base_asset.to_ascii_uppercase(), row))
        .collect::<HashMap<_, _>>();

    rows.into_iter()
        .map(|mut row| {
            if row.symbol.contains('/') {
                row.base_asset = base_asset_from_symbol(&row.symbol);
            }

            if let Some(metadata) = metadata_by_asset.get(&row.base_asset) {
                row.market_cap_usd = metadata.market_cap_usd;
                row.market_cap_rank = metadata.market_cap_rank;
                row.market_size_tier =
                    classify_market_size_tier(metadata.market_cap_rank).to_string();
            } else {
                row.market_cap_usd = None;
                row.market_cap_rank = None;
                row.market_size_tier = classify_market_size_tier(None).to_string();
            }

            row
        })
        .collect()
}

fn aggregate_recent_trades(trades: Vec<RecentTrade>) -> Vec<RecentTrade> {
    let mut grouped = HashMap::<(String, String, u64), RecentTrade>::new();

    for trade in trades {
        let normalized_side = trade.side.to_ascii_lowercase();
        let key = (
            trade.exchange.clone(),
            normalized_side.clone(),
            trade.price.to_bits(),
        );

        grouped
            .entry(key)
            .and_modify(|aggregated| {
                aggregated.size += trade.size;
                if aggregated.timestamp < trade.timestamp {
                    aggregated.timestamp = trade.timestamp.clone();
                }
            })
            .or_insert(RecentTrade {
                side: normalized_side,
                ..trade
            });
    }

    let mut aggregated = grouped.into_values().collect::<Vec<_>>();
    aggregated.sort_by(|left, right| {
        trade_side_rank(&left.side)
            .cmp(&trade_side_rank(&right.side))
            .then_with(|| match left.side.as_str() {
                "sell" => left.price.total_cmp(&right.price),
                "buy" => right.price.total_cmp(&left.price),
                _ => left.price.total_cmp(&right.price),
            })
            .then(left.exchange.cmp(&right.exchange))
    });
    aggregated
}

fn trade_side_rank(side: &str) -> u8 {
    match side {
        "sell" => 0,
        "buy" => 1,
        _ => 2,
    }
}

fn build_spread_opportunity(
    symbol: &str,
    market_type: &str,
    snapshots: &[MarketSnapshot],
) -> Option<SpreadOpportunityDto> {
    if snapshots.len() < 2 {
        return None;
    }

    let best_buy = snapshots
        .iter()
        .min_by(|left, right| left.ask_price.total_cmp(&right.ask_price))?;
    let best_sell = snapshots
        .iter()
        .max_by(|left, right| left.bid_price.total_cmp(&right.bid_price))?;

    if best_buy.exchange == best_sell.exchange || best_sell.bid_price <= best_buy.ask_price {
        return None;
    }

    let gross_pct = ((best_sell.bid_price - best_buy.ask_price) / best_buy.ask_price) * 100.0;
    let net_absolute = spreads::compute_net_spread(
        best_sell.bid_price,
        best_buy.ask_price,
        0.001,
        0.001,
        0.0005,
    );
    let net_pct = (net_absolute / best_buy.ask_price) * 100.0;

    Some(SpreadOpportunityDto {
        symbol: format!("{symbol} [{}]", market_type),
        buy_exchange: best_buy.exchange.clone(),
        sell_exchange: best_sell.exchange.clone(),
        gross_spread_pct: gross_pct,
        net_spread_pct: net_pct,
        funding_context: format!(
            "{} best ask on {} versus best bid on {}",
            market_type, best_buy.exchange, best_sell.exchange
        ),
    })
}

fn fallback_market_targets() -> Vec<MarketTarget> {
    DEFAULT_MARKET_SYMBOLS
        .iter()
        .flat_map(|symbol| {
            [
                MarketTarget::new(*symbol, "spot"),
                MarketTarget::new(*symbol, "perpetual"),
            ]
        })
        .collect()
}

fn dedupe_market_targets(targets: Vec<MarketTarget>) -> Vec<MarketTarget> {
    let mut seen = HashSet::new();
    let mut deduped = targets
        .into_iter()
        .filter(|target| seen.insert((target.symbol.clone(), target.market_type.clone())))
        .collect::<Vec<_>>();
    deduped.sort_by(|left, right| market_target_sort_key(left).cmp(&market_target_sort_key(right)));
    deduped
}

fn market_symbols_from_targets(targets: &[MarketTarget]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut symbols = targets
        .iter()
        .map(|target| target.symbol.clone())
        .filter(|symbol| seen.insert(symbol.clone()))
        .collect::<Vec<_>>();
    symbols.sort_by_key(|symbol| symbol_sort_key(symbol));
    symbols
}

fn limited_market_targets(targets: &[MarketTarget]) -> Vec<MarketTarget> {
    targets.to_vec()
}

fn market_target_sort_key(target: &MarketTarget) -> (usize, usize, String) {
    let market_rank = if target.market_type == "spot" { 0 } else { 1 };
    let symbol_rank = symbol_priority(&target.symbol);
    (symbol_rank, market_rank, target.symbol.clone())
}

fn symbol_sort_key(symbol: &str) -> (usize, String) {
    (symbol_priority(symbol), symbol.to_string())
}

fn symbol_priority(symbol: &str) -> usize {
    PRIORITY_SYMBOLS
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(symbol))
        .unwrap_or(usize::MAX / 2)
}
