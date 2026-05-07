use std::collections::HashMap;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::models::{
    ArbitrageOpportunityDto, ArbitrageOpportunityPageDto, MarketListRow, VenueTickerSnapshot,
};

const BUY_FEE_RATE: f64 = 0.001;
const SELL_FEE_RATE: f64 = 0.001;
const CROSS_EXCHANGE_TRANSFER_PENALTY_PCT: f64 = 0.05;
const DEFAULT_BORROW_RATE_DAILY_PCT: f64 = 0.03;
const HOLD_DAYS: f64 = 8.0 / 24.0;
const MIN_LIQUIDITY_USDT_24H: f64 = 20_000_000.0;
const MAX_CANDIDATE_AGE_MS: i128 = 45_000;
const MISSING_MARKET_CAP_COMPONENT: f64 = 0.20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArbitrageTypeFilter {
    All,
    Spot,
    Perpetual,
    CrossMarket,
}

impl ArbitrageTypeFilter {
    pub fn from_query(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "spot" => Self::Spot,
            "perpetual" => Self::Perpetual,
            "cross_market" => Self::CrossMarket,
            _ => Self::All,
        }
    }

    fn matches(self, item: &ArbitrageOpportunityDto) -> bool {
        match self {
            Self::All => true,
            Self::Spot => item.opportunity_type == "spot_cross_exchange",
            Self::Perpetual => item.opportunity_type == "perpetual_cross_exchange",
            Self::CrossMarket => item.secondary_market_type.is_some(),
        }
    }
}

#[derive(Clone, Copy)]
struct LegRef<'a> {
    snapshot: &'a VenueTickerSnapshot,
    market_type: &'static str,
    row_stale: bool,
}

#[derive(Default)]
struct SymbolRows<'a> {
    spot: Option<&'a MarketListRow>,
    perpetual: Option<&'a MarketListRow>,
}

pub fn build_arbitrage_candidates(
    rows: &[MarketListRow],
    filter: ArbitrageTypeFilter,
) -> Vec<ArbitrageOpportunityDto> {
    let mut grouped = HashMap::<String, SymbolRows<'_>>::new();
    for row in rows {
        let entry = grouped.entry(row.symbol.clone()).or_default();
        if row.market_type.eq_ignore_ascii_case("spot") {
            entry.spot = Some(row);
        } else if row.market_type.eq_ignore_ascii_case("perpetual") {
            entry.perpetual = Some(row);
        }
    }

    let mut items = grouped
        .into_iter()
        .flat_map(|(symbol, rows)| build_symbol_candidates(&symbol, rows))
        .filter(passes_hard_filters)
        .collect::<Vec<_>>();

    apply_recommendation_scores(&mut items);
    items.sort_by(|left, right| {
        right
            .recommendation_score
            .total_cmp(&left.recommendation_score)
            .then(
                right
                    .fee_adjusted_net_spread_pct
                    .total_cmp(&left.fee_adjusted_net_spread_pct),
            )
            .then(right.liquidity_usdt_24h.total_cmp(&left.liquidity_usdt_24h))
            .then(
                right
                    .market_cap_usd
                    .unwrap_or(f64::MIN)
                    .total_cmp(&left.market_cap_usd.unwrap_or(f64::MIN)),
            )
            .then(
                parse_timestamp_millis(&right.updated_at)
                    .unwrap_or_default()
                    .cmp(&parse_timestamp_millis(&left.updated_at).unwrap_or_default()),
            )
    });

    items
        .into_iter()
        .filter(|item| filter.matches(item))
        .collect()
}

pub fn paginate_candidates(
    items: Vec<ArbitrageOpportunityDto>,
    page: usize,
    page_size: usize,
) -> ArbitrageOpportunityPageDto {
    let page_size = page_size.max(1);
    let page = page.max(1);
    let total = items.len();
    let total_pages = total.max(1).div_ceil(page_size);
    let start = (page - 1) * page_size;
    let paged = items.into_iter().skip(start).take(page_size).collect();

    ArbitrageOpportunityPageDto {
        items: paged,
        total,
        page,
        page_size,
        total_pages,
    }
}

fn build_symbol_candidates(symbol: &str, rows: SymbolRows<'_>) -> Vec<ArbitrageOpportunityDto> {
    let mut items = Vec::new();
    let market_cap_usd = rows
        .spot
        .and_then(preferred_market_cap)
        .or_else(|| rows.perpetual.and_then(preferred_market_cap));

    if let Some(spot_row) = rows.spot {
        if let Some(item) = best_same_market_candidate(
            symbol,
            "spot_cross_exchange",
            "spot",
            &spot_row.venue_snapshots,
            market_cap_usd,
            false,
        ) {
            items.push(item);
        }
    }

    if let Some(perpetual_row) = rows.perpetual {
        if let Some(item) = best_same_market_candidate(
            symbol,
            "perpetual_cross_exchange",
            "perpetual",
            &perpetual_row.venue_snapshots,
            market_cap_usd,
            true,
        ) {
            items.push(item);
        }
    }

    if let (Some(spot_row), Some(perpetual_row)) = (rows.spot, rows.perpetual) {
        if let Some(item) = best_same_exchange_cross_market_candidate(
            symbol,
            "spot_long_perp_short_same_exchange",
            "spot",
            Some("perpetual"),
            &spot_row.venue_snapshots,
            &perpetual_row.venue_snapshots,
            market_cap_usd,
            true,
        ) {
            items.push(item);
        }

        if let Some(item) = best_cross_exchange_cross_market_candidate(
            symbol,
            "spot_long_perp_short_cross_exchange",
            "spot",
            Some("perpetual"),
            &spot_row.venue_snapshots,
            &perpetual_row.venue_snapshots,
            market_cap_usd,
            true,
        ) {
            items.push(item);
        }

        if let Some(item) = best_same_exchange_cross_market_candidate(
            symbol,
            "perp_long_spot_short_same_exchange",
            "perpetual",
            Some("spot"),
            &perpetual_row.venue_snapshots,
            &spot_row.venue_snapshots,
            market_cap_usd,
            false,
        ) {
            items.push(item);
        }

        if let Some(item) = best_cross_exchange_cross_market_candidate(
            symbol,
            "perp_long_spot_short_cross_exchange",
            "perpetual",
            Some("spot"),
            &perpetual_row.venue_snapshots,
            &spot_row.venue_snapshots,
            market_cap_usd,
            false,
        ) {
            items.push(item);
        }
    }

    items
}

fn preferred_market_cap(row: &MarketListRow) -> Option<f64> {
    row.market_cap_usd.or(row.fdv_usd)
}

fn best_same_market_candidate(
    symbol: &str,
    opportunity_type: &str,
    market_type: &'static str,
    venues: &[VenueTickerSnapshot],
    market_cap_usd: Option<f64>,
    use_perp_carry: bool,
) -> Option<ArbitrageOpportunityDto> {
    let mut best = None;
    for buy in venues {
        for sell in venues {
            if buy.exchange == sell.exchange {
                continue;
            }

            let buy_leg = LegRef {
                snapshot: buy,
                market_type,
                row_stale: false,
            };
            let sell_leg = LegRef {
                snapshot: sell,
                market_type,
                row_stale: false,
            };
            let candidate = build_candidate(
                symbol,
                opportunity_type,
                market_type,
                None,
                &buy_leg,
                &sell_leg,
                market_cap_usd,
                use_perp_carry,
            );
            best = select_better_candidate(best, candidate);
        }
    }

    best
}

fn best_same_exchange_cross_market_candidate(
    symbol: &str,
    opportunity_type: &str,
    primary_market_type: &'static str,
    secondary_market_type: Option<&'static str>,
    buy_venues: &[VenueTickerSnapshot],
    sell_venues: &[VenueTickerSnapshot],
    market_cap_usd: Option<f64>,
    spot_buy_perp_sell: bool,
) -> Option<ArbitrageOpportunityDto> {
    let mut sell_by_exchange = HashMap::new();
    for sell in sell_venues {
        sell_by_exchange.insert(sell.exchange.as_str(), sell);
    }

    let mut best = None;
    for buy in buy_venues {
        let Some(sell) = sell_by_exchange.get(buy.exchange.as_str()) else {
            continue;
        };

        let (buy_market_type, sell_market_type) = if spot_buy_perp_sell {
            ("spot", "perpetual")
        } else {
            ("perpetual", "spot")
        };
        let buy_leg = LegRef {
            snapshot: buy,
            market_type: buy_market_type,
            row_stale: false,
        };
        let sell_leg = LegRef {
            snapshot: sell,
            market_type: sell_market_type,
            row_stale: false,
        };
        let candidate = build_candidate(
            symbol,
            opportunity_type,
            primary_market_type,
            secondary_market_type,
            &buy_leg,
            &sell_leg,
            market_cap_usd,
            true,
        );
        best = select_better_candidate(best, candidate);
    }

    best
}

fn best_cross_exchange_cross_market_candidate(
    symbol: &str,
    opportunity_type: &str,
    primary_market_type: &'static str,
    secondary_market_type: Option<&'static str>,
    buy_venues: &[VenueTickerSnapshot],
    sell_venues: &[VenueTickerSnapshot],
    market_cap_usd: Option<f64>,
    spot_buy_perp_sell: bool,
) -> Option<ArbitrageOpportunityDto> {
    let mut best = None;
    for buy in buy_venues {
        for sell in sell_venues {
            if buy.exchange == sell.exchange {
                continue;
            }

            let (buy_market_type, sell_market_type) = if spot_buy_perp_sell {
                ("spot", "perpetual")
            } else {
                ("perpetual", "spot")
            };
            let buy_leg = LegRef {
                snapshot: buy,
                market_type: buy_market_type,
                row_stale: false,
            };
            let sell_leg = LegRef {
                snapshot: sell,
                market_type: sell_market_type,
                row_stale: false,
            };
            let candidate = build_candidate(
                symbol,
                opportunity_type,
                primary_market_type,
                secondary_market_type,
                &buy_leg,
                &sell_leg,
                market_cap_usd,
                true,
            );
            best = select_better_candidate(best, candidate);
        }
    }

    best
}

fn select_better_candidate(
    current: Option<ArbitrageOpportunityDto>,
    next: Option<ArbitrageOpportunityDto>,
) -> Option<ArbitrageOpportunityDto> {
    match (current, next) {
        (Some(left), Some(right)) => {
            if right
                .fee_adjusted_net_spread_pct
                .total_cmp(&left.fee_adjusted_net_spread_pct)
                .is_gt()
            {
                Some(right)
            } else {
                Some(left)
            }
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn build_candidate(
    symbol: &str,
    opportunity_type: &str,
    primary_market_type: &'static str,
    secondary_market_type: Option<&'static str>,
    buy: &LegRef<'_>,
    sell: &LegRef<'_>,
    market_cap_usd: Option<f64>,
    allow_carry: bool,
) -> Option<ArbitrageOpportunityDto> {
    if buy.snapshot.ask_price <= 0.0 || sell.snapshot.bid_price <= 0.0 {
        return None;
    }

    let gross = sell.snapshot.bid_price - buy.snapshot.ask_price;
    let buy_slippage = slippage_rate(buy.snapshot.volume_24h);
    let sell_slippage = slippage_rate(sell.snapshot.volume_24h);
    let fees = sell.snapshot.bid_price * SELL_FEE_RATE + buy.snapshot.ask_price * BUY_FEE_RATE;
    let slippage = sell.snapshot.bid_price * sell_slippage + buy.snapshot.ask_price * buy_slippage;
    let fee_adjusted_net_spread_abs = gross - fees - slippage;
    let fee_adjusted_net_spread_pct =
        (fee_adjusted_net_spread_abs / buy.snapshot.ask_price) * 100.0;

    let funding_carry_pct = if allow_carry {
        funding_carry_pct(buy, sell)
    } else {
        0.0
    };
    let borrow_rate_daily = short_spot_borrow_rate(buy, sell);
    let borrow_cost_pct = borrow_rate_daily.unwrap_or(0.0) * HOLD_DAYS;
    let transfer_penalty_pct = if buy.snapshot.exchange != sell.snapshot.exchange {
        CROSS_EXCHANGE_TRANSFER_PENALTY_PCT
    } else {
        0.0
    };
    let simulated_carry_pct = funding_carry_pct - borrow_cost_pct - transfer_penalty_pct;
    let liquidity_usdt_24h = buy.snapshot.volume_24h.min(sell.snapshot.volume_24h);
    let updated_at = oldest_updated_at(&buy.snapshot.updated_at, &sell.snapshot.updated_at);
    let stale = buy.snapshot.stale || sell.snapshot.stale || buy.row_stale || sell.row_stale;

    Some(ArbitrageOpportunityDto {
        symbol: symbol.to_string(),
        opportunity_type: opportunity_type.to_string(),
        primary_market_type: primary_market_type.to_string(),
        secondary_market_type: secondary_market_type.map(str::to_string),
        buy_exchange: buy.snapshot.exchange.clone(),
        buy_market_type: buy.market_type.to_string(),
        buy_price: buy.snapshot.ask_price,
        sell_exchange: sell.snapshot.exchange.clone(),
        sell_market_type: sell.market_type.to_string(),
        sell_price: sell.snapshot.bid_price,
        fee_adjusted_net_spread_pct,
        simulated_carry_pct,
        simulated_total_yield_pct: fee_adjusted_net_spread_pct + simulated_carry_pct,
        liquidity_usdt_24h,
        market_cap_usd,
        funding_rate: representative_funding_rate(buy, sell),
        borrow_rate_daily,
        recommendation_score: 0.0,
        updated_at,
        stale,
    })
}

fn slippage_rate(volume_24h: f64) -> f64 {
    if volume_24h >= 1_000_000_000.0 {
        0.0001
    } else if volume_24h >= 250_000_000.0 {
        0.0002
    } else if volume_24h >= 50_000_000.0 {
        0.0004
    } else {
        0.0008
    }
}

fn funding_carry_pct(buy: &LegRef<'_>, sell: &LegRef<'_>) -> f64 {
    match (buy.market_type, sell.market_type) {
        ("spot", "perpetual") => sell.snapshot.funding_rate.unwrap_or(0.0),
        ("perpetual", "spot") => -buy.snapshot.funding_rate.unwrap_or(0.0),
        ("perpetual", "perpetual") => {
            sell.snapshot.funding_rate.unwrap_or(0.0) - buy.snapshot.funding_rate.unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

fn representative_funding_rate(buy: &LegRef<'_>, sell: &LegRef<'_>) -> Option<f64> {
    match (buy.market_type, sell.market_type) {
        ("spot", "perpetual") => sell.snapshot.funding_rate,
        ("perpetual", "spot") => buy.snapshot.funding_rate,
        ("perpetual", "perpetual") => {
            if buy.snapshot.funding_rate.is_some() || sell.snapshot.funding_rate.is_some() {
                Some(
                    sell.snapshot.funding_rate.unwrap_or(0.0)
                        - buy.snapshot.funding_rate.unwrap_or(0.0),
                )
            } else {
                None
            }
        }
        _ => None,
    }
}

fn short_spot_borrow_rate(buy: &LegRef<'_>, sell: &LegRef<'_>) -> Option<f64> {
    if buy.market_type == "perpetual" && sell.market_type == "spot" {
        Some(DEFAULT_BORROW_RATE_DAILY_PCT)
    } else {
        None
    }
}

fn passes_hard_filters(item: &ArbitrageOpportunityDto) -> bool {
    if item.stale {
        return false;
    }
    if let Some(updated_at) = parse_timestamp_millis(&item.updated_at) {
        let now = current_millis();
        if now.saturating_sub(updated_at) > MAX_CANDIDATE_AGE_MS {
            return false;
        }
    }
    item.buy_price > 0.0
        && item.sell_price > 0.0
        && item.fee_adjusted_net_spread_pct > 0.0
        && item.liquidity_usdt_24h >= MIN_LIQUIDITY_USDT_24H
        && !(item.buy_exchange == item.sell_exchange
            && item.buy_market_type == item.sell_market_type)
}

fn apply_recommendation_scores(items: &mut [ArbitrageOpportunityDto]) {
    if items.is_empty() {
        return;
    }

    let spread_values = items
        .iter()
        .map(|item| item.fee_adjusted_net_spread_pct)
        .collect::<Vec<_>>();
    let liquidity_values = items
        .iter()
        .map(|item| item.liquidity_usdt_24h.max(1.0).log10())
        .collect::<Vec<_>>();
    let market_cap_values = items
        .iter()
        .filter_map(|item| item.market_cap_usd.filter(|value| *value > 0.0))
        .map(f64::log10)
        .collect::<Vec<_>>();

    for item in items {
        let spread_component = percentile_rank(&spread_values, item.fee_adjusted_net_spread_pct);
        let liquidity_component =
            percentile_rank(&liquidity_values, item.liquidity_usdt_24h.max(1.0).log10());
        let market_cap_component = item
            .market_cap_usd
            .filter(|value| *value > 0.0)
            .map(|value| percentile_rank(&market_cap_values, value.log10()))
            .unwrap_or(MISSING_MARKET_CAP_COMPONENT);

        item.recommendation_score = round_to_one_decimal(
            100.0
                * (0.55 * spread_component
                    + 0.30 * liquidity_component
                    + 0.15 * market_cap_component),
        );
    }
}

fn percentile_rank(values: &[f64], target: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if values.len() == 1 {
        return 1.0;
    }

    let less = values.iter().filter(|value| **value < target).count() as f64;
    let equal = values
        .iter()
        .filter(|value| (**value - target).abs() < f64::EPSILON)
        .count() as f64;
    (less + ((equal - 1.0).max(0.0) / 2.0)) / (values.len() as f64 - 1.0)
}

fn round_to_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn oldest_updated_at(left: &str, right: &str) -> String {
    match (parse_timestamp_millis(left), parse_timestamp_millis(right)) {
        (Some(left_ms), Some(right_ms)) => {
            if left_ms <= right_ms {
                left.to_string()
            } else {
                right.to_string()
            }
        }
        _ => left.to_string(),
    }
}

fn parse_timestamp_millis(value: &str) -> Option<i128> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(rest) = trimmed.strip_prefix("epoch:") {
        return parse_numeric_timestamp(rest);
    }
    parse_numeric_timestamp(trimmed).or_else(|| {
        OffsetDateTime::parse(trimmed, &Rfc3339)
            .ok()
            .map(|item| item.unix_timestamp_nanos() / 1_000_000)
    })
}

fn parse_numeric_timestamp(value: &str) -> Option<i128> {
    let raw = value.parse::<i128>().ok()?;
    Some(if raw < 1_000_000_000_000 {
        raw * 1_000
    } else {
        raw
    })
}

fn current_millis() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000
}

#[cfg(test)]
mod tests {
    use super::{build_arbitrage_candidates, paginate_candidates, ArbitrageTypeFilter};
    use crate::models::{ArbitrageOpportunityDto, MarketListRow, VenueTickerSnapshot};
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;

    fn current_rfc3339() -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("timestamp should format")
    }

    fn sample_candidate(symbol: &str, recommendation_score: f64) -> ArbitrageOpportunityDto {
        ArbitrageOpportunityDto {
            symbol: symbol.into(),
            opportunity_type: "spot_cross_exchange".into(),
            primary_market_type: "spot".into(),
            secondary_market_type: None,
            buy_exchange: "akshare".into(),
            buy_market_type: "spot".into(),
            buy_price: 100.0,
            sell_exchange: "人民币现金".into(),
            sell_market_type: "spot".into(),
            sell_price: 101.0,
            fee_adjusted_net_spread_pct: 0.20,
            simulated_carry_pct: 0.00,
            simulated_total_yield_pct: 0.20,
            liquidity_usdt_24h: 100_000_000.0,
            market_cap_usd: Some(1_000_000_000.0),
            funding_rate: None,
            borrow_rate_daily: None,
            recommendation_score,
            updated_at: current_rfc3339(),
            stale: false,
        }
    }

    #[test]
    fn builds_cross_market_arbitrage_candidates() {
        let updated_at = current_rfc3339();
        let rows = vec![
            MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "spot".into(),
                market_cap_rank: Some(1),
                market_size_tier: "large".into(),
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
                market_cap_usd: Some(1_300_000_000_000.0),
                fdv_usd: Some(1_320_000_000_000.0),
            },
            MarketListRow {
                symbol: "BTC/USDT".into(),
                base_asset: "BTC".into(),
                market_type: "perpetual".into(),
                market_cap_rank: Some(1),
                market_size_tier: "large".into(),
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
                market_cap_usd: Some(1_300_000_000_000.0),
                fdv_usd: Some(1_320_000_000_000.0),
            },
        ];

        let items = build_arbitrage_candidates(&rows, ArbitrageTypeFilter::All);

        assert!(items
            .iter()
            .any(|item| { item.opportunity_type == "spot_long_perp_short_cross_exchange" }));
    }

    #[test]
    fn paginates_candidates_after_score_sorting() {
        let page = paginate_candidates(
            vec![
                sample_candidate("BTC/USDT", 91.2),
                sample_candidate("ETH/USDT", 78.0),
                sample_candidate("SOL/USDT", 64.4),
            ],
            1,
            2,
        );

        assert_eq!(page.total, 3);
        assert_eq!(page.total_pages, 2);
        assert_eq!(page.items[0].symbol, "BTC/USDT");
        assert_eq!(page.items[1].symbol, "ETH/USDT");
    }
}
