use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::signals::indicators::rsi;
use std::collections::HashMap;

pub struct RsiExtremeStrategy;

impl Strategy for RsiExtremeStrategy {
    fn id(&self) -> &'static str {
        "rsi_extreme"
    }
    fn name(&self) -> &'static str {
        "RSI Extreme"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Momentum
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Identifies overbought / oversold RSI extremes"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("rsi_extreme")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let oversold = params.get("oversold").copied().unwrap_or(30.0);
        let overbought = params.get("overbought").copied().unwrap_or(70.0);
        if oversold <= 0.0 || oversold >= 50.0 {
            return Err("oversold must be 0 < val < 50".into());
        }
        if overbought <= 50.0 || overbought >= 100.0 {
            return Err("overbought must be 50 < val < 100".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let period = params.get("period").copied().unwrap_or(14.0) as usize;
        let oversold = params.get("oversold").copied().unwrap_or(30.0);
        let overbought = params.get("overbought").copied().unwrap_or(70.0);
        let min_confidence = params.get("min_confidence").copied().unwrap_or(55.0);

        if ctx.candles.len() < period + 1 {
            return None;
        }
        let rsi_values = rsi(ctx.candles, period);
        let current_rsi = *rsi_values.last()?;

        if current_rsi.is_nan() {
            return None;
        }

        let (direction, summary, strength, confidence) = if current_rsi < oversold {
            (
                SignalDirection::Buy,
                format!("RSI oversold at {:.1} — bounce expected", current_rsi),
                ((oversold - current_rsi) / oversold).clamp(0.1, 1.0),
                (min_confidence + (oversold - current_rsi) * 1.5).clamp(min_confidence, 90.0),
            )
        } else if current_rsi > overbought {
            (
                SignalDirection::Sell,
                format!("RSI overbought at {:.1} — pullback expected", current_rsi),
                ((current_rsi - overbought) / (100.0 - overbought)).clamp(0.1, 1.0),
                (min_confidence + (current_rsi - overbought) * 1.5).clamp(min_confidence, 90.0),
            )
        } else {
            return None;
        };

        let mut metrics = HashMap::new();
        metrics.insert("rsi".to_string(), current_rsi);

        Some(StrategySignal {
            strategy_id: self.id().to_string(),
            category: self.category(),
            direction,
            strength,
            confidence,
            summary,
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{MarketListRow, OhlcvBar};

    fn fake_row() -> MarketListRow {
        MarketListRow {
            symbol: "ETH/USDT".into(),
            base_asset: "ETH".into(),
            market_type: "spot".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 50.0,
            change_24h: -5.0,
            volume_24h: 500_000_000.0,
            funding_rate: None,
            spread_bps: 1.0,
            exchanges: vec!["akshare".into()],
            updated_at: "x".into(),
            stale: false,
            venue_snapshots: vec![],
            best_bid_exchange: None,
            best_ask_exchange: None,
            best_bid_price: None,
            best_ask_price: None,
            responded_exchange_count: 1,
            fdv_usd: None,
        }
    }

    #[test]
    fn detects_oversold() {
        let mut bars: Vec<OhlcvBar> = vec![];
        let mut price = 100.0;
        for i in 0..20 {
            if i < 15 {
                price -= 3.0;
            } else {
                price += 0.5;
            }
            bars.push(OhlcvBar {
                open_time: i.to_string(),
                open: price,
                high: price,
                low: price,
                close: price,
                volume: 100.0,
                turnover: None,
            });
        }
        let ctx = SignalContext {
            symbol: "ETH/USDT",
            market_type: "spot",
            row: &fake_row(),
            candles: &bars,
            venue_snapshots: &[],
        };
        let signal = RsiExtremeStrategy
            .evaluate(&ctx, &RsiExtremeStrategy.default_params())
            .unwrap();
        assert_eq!(signal.direction, SignalDirection::Buy);
        assert!(*signal.metrics.get("rsi").unwrap() < 30.0);
    }

    #[test]
    fn detects_overbought() {
        let mut bars: Vec<OhlcvBar> = vec![];
        let mut price = 100.0;
        for i in 0..20 {
            if i < 15 {
                price += 3.0;
            } else {
                price -= 0.5;
            }
            bars.push(OhlcvBar {
                open_time: i.to_string(),
                open: price,
                high: price,
                low: price,
                close: price,
                volume: 100.0,
                turnover: None,
            });
        }
        let ctx = SignalContext {
            symbol: "ETH/USDT",
            market_type: "spot",
            row: &fake_row(),
            candles: &bars,
            venue_snapshots: &[],
        };
        let signal = RsiExtremeStrategy
            .evaluate(&ctx, &RsiExtremeStrategy.default_params())
            .unwrap();
        assert_eq!(signal.direction, SignalDirection::Sell);
        assert!(*signal.metrics.get("rsi").unwrap() > 70.0);
    }
}
