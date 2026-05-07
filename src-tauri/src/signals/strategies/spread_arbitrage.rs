use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use std::collections::HashMap;

const FEE_RATE: f64 = 0.001;

pub struct SpreadArbitrageStrategy;

impl Strategy for SpreadArbitrageStrategy {
    fn id(&self) -> &'static str {
        "spread_arbitrage"
    }
    fn name(&self) -> &'static str {
        "Spread Arbitrage"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Arbitrage
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects profitable cross-exchange spread opportunities"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("spread_arbitrage")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let min_spread = params.get("min_spread_bps").copied().unwrap_or(5.0);
        if min_spread < 0.0 {
            return Err("min_spread_bps must be ≥ 0".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let min_spread_pct = params.get("min_spread_bps").copied().unwrap_or(5.0) / 100.0;
        let _min_liquidity = params.get("min_liquidity").copied().unwrap_or(10000.0);

        if ctx.venue_snapshots.len() < 2 {
            return None;
        }

        let mut best_bid: Option<&crate::models::VenueTickerSnapshot> = None;
        let mut best_ask: Option<&crate::models::VenueTickerSnapshot> = None;

        for venue in ctx.venue_snapshots {
            if venue.bid_price <= 0.0 || venue.ask_price <= 0.0 {
                continue;
            }
            match best_bid {
                None => best_bid = Some(venue),
                Some(current) if venue.bid_price > current.bid_price => best_bid = Some(venue),
                _ => {}
            }
            match best_ask {
                None => best_ask = Some(venue),
                Some(current) if venue.ask_price < current.ask_price => best_ask = Some(venue),
                _ => {}
            }
        }

        let bid = best_bid?;
        let ask = best_ask?;
        if bid.exchange == ask.exchange {
            return None;
        }

        let gross_pct = (bid.bid_price - ask.ask_price) / ask.ask_price * 100.0;
        let net_pct = gross_pct - FEE_RATE * 200.0;

        if net_pct < min_spread_pct {
            return None;
        }

        let strength = ((net_pct - min_spread_pct) / 1.0).clamp(0.1, 1.0);
        let confidence = (60.0 + net_pct * 30.0).clamp(55.0, 95.0);

        let mut metrics = HashMap::new();
        metrics.insert("net_spread_pct".to_string(), net_pct);
        metrics.insert("bid_price".to_string(), bid.bid_price);
        metrics.insert("ask_price".to_string(), ask.ask_price);

        Some(StrategySignal {
            strategy_id: self.id().to_string(),
            category: self.category(),
            direction: SignalDirection::Buy,
            strength,
            confidence,
            summary: format!(
                "Buy {} on {} @ {:.4}, Sell on {} @ {:.4} — net spread {:.3}%",
                ctx.symbol, ask.exchange, ask.ask_price, bid.exchange, bid.bid_price, net_pct
            ),
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MarketListRow, VenueTickerSnapshot};

    #[test]
    fn detects_cross_exchange_spread() {
        let row = MarketListRow {
            symbol: "BTC/USDT".into(),
            base_asset: "BTC".into(),
            market_type: "spot".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 100.0,
            change_24h: 1.0,
            volume_24h: 1_000_000_000.0,
            funding_rate: None,
            spread_bps: 10.0,
            exchanges: vec!["akshare".into(), "人民币现金".into()],
            updated_at: "x".into(),
            stale: false,
            venue_snapshots: vec![],
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 2,
            fdv_usd: None,
        };
        let venues = vec![
            VenueTickerSnapshot {
                exchange: "人民币现金".into(),
                last_price: 101.0,
                bid_price: 101.0,
                ask_price: 100.5,
                volume_24h: 500_000_000.0,
                funding_rate: None,
                mark_price: None,
                index_price: None,
                updated_at: "x".into(),
                stale: false,
            },
            VenueTickerSnapshot {
                exchange: "akshare".into(),
                last_price: 100.0,
                bid_price: 99.5,
                ask_price: 99.0,
                volume_24h: 500_000_000.0,
                funding_rate: None,
                mark_price: None,
                index_price: None,
                updated_at: "x".into(),
                stale: false,
            },
        ];
        let ctx = SignalContext {
            symbol: "BTC/USDT",
            market_type: "spot",
            row: &row,
            candles: &[],
            venue_snapshots: &venues,
        };
        let signal = SpreadArbitrageStrategy
            .evaluate(&ctx, &SpreadArbitrageStrategy.default_params())
            .unwrap();
        assert_eq!(signal.category, SignalCategory::Arbitrage);
        assert!(signal.summary.contains("人民币现金"));
        assert!(signal.summary.contains("akshare"));
    }
}
