use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::signals::indicators::sma;
use std::collections::HashMap;

pub struct MaCrossStrategy;

impl Strategy for MaCrossStrategy {
    fn id(&self) -> &'static str {
        "ma_cross"
    }
    fn name(&self) -> &'static str {
        "MA Cross"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Trend
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects golden cross / death cross between two moving averages"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("ma_cross")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let fast = params.get("fast_period").copied().unwrap_or(5.0);
        let slow = params.get("slow_period").copied().unwrap_or(20.0);
        if fast < 2.0 {
            return Err("fast_period must be ≥ 2".into());
        }
        if slow < 2.0 {
            return Err("slow_period must be ≥ 2".into());
        }
        if fast >= slow {
            return Err("fast_period must be < slow_period".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let fast_period = params.get("fast_period").copied().unwrap_or(5.0) as usize;
        let slow_period = params.get("slow_period").copied().unwrap_or(20.0) as usize;
        let min_confidence = params.get("min_confidence").copied().unwrap_or(55.0);
        let min_strength = params.get("min_strength").copied().unwrap_or(0.1);

        if ctx.candles.len() < slow_period {
            return None;
        }
        let ma_fast = sma(ctx.candles, fast_period);
        let ma_slow = sma(ctx.candles, slow_period);
        let len = ctx.candles.len();
        let prev_fast = ma_fast[len - 2];
        let prev_slow = ma_slow[len - 2];
        let curr_fast = ma_fast[len - 1];
        let curr_slow = ma_slow[len - 1];

        if curr_fast.is_nan() || curr_slow.is_nan() || prev_fast.is_nan() || prev_slow.is_nan() {
            return None;
        }

        let direction;
        let summary;
        if prev_fast <= prev_slow && curr_fast > curr_slow {
            direction = SignalDirection::Buy;
            summary = format!("MA{fast_period} ({curr_fast:.2}) crossed above MA{slow_period} ({curr_slow:.2}) — golden cross");
        } else if prev_fast >= prev_slow && curr_fast < curr_slow {
            direction = SignalDirection::Sell;
            summary = format!("MA{fast_period} ({curr_fast:.2}) crossed below MA{slow_period} ({curr_slow:.2}) — death cross");
        } else {
            return None;
        }

        let spread_strength = ((curr_fast - curr_slow).abs() / curr_slow * 100.0).min(1.0);
        let strength = (spread_strength * 0.5).clamp(min_strength, 1.0);
        let confidence = (min_confidence + spread_strength * 20.0).clamp(min_confidence, 85.0);

        let mut metrics = HashMap::new();
        metrics.insert(format!("ma{fast_period}"), curr_fast);
        metrics.insert(format!("ma{slow_period}"), curr_slow);

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
    use super::MaCrossStrategy;
    use crate::models::{MarketListRow, OhlcvBar};
    use crate::signals::strategies::{SignalContext, SignalDirection, Strategy};

    fn fake_row() -> MarketListRow {
        MarketListRow {
            symbol: "BTC/USDT".into(),
            base_asset: "BTC".into(),
            market_type: "perpetual".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 100.0,
            change_24h: 2.0,
            volume_24h: 1_000_000_000.0,
            funding_rate: None,
            spread_bps: 2.0,
            exchanges: vec!["akshare".into()],
            updated_at: "2026-05-04T10:00:00+08:00".into(),
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
    fn detects_golden_cross() {
        let mut closes = vec![100.0; 25];
        closes[24] = 105.0;
        let bars: Vec<OhlcvBar> = closes
            .iter()
            .map(|&c| OhlcvBar {
                open_time: String::new(),
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 100.0,
                turnover: None,
            })
            .collect();
        let ctx = SignalContext {
            symbol: "BTC/USDT",
            market_type: "perpetual",
            row: &fake_row(),
            candles: &bars,
            venue_snapshots: &[],
        };
        let signal = MaCrossStrategy
            .evaluate(&ctx, &MaCrossStrategy.default_params())
            .unwrap();
        assert_eq!(signal.direction, SignalDirection::Buy);
        assert!(signal.summary.contains("golden cross"));
    }

    #[test]
    fn detects_death_cross() {
        let mut closes = vec![100.0; 25];
        closes[24] = 95.0;
        let bars: Vec<OhlcvBar> = closes
            .iter()
            .map(|&c| OhlcvBar {
                open_time: String::new(),
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 100.0,
                turnover: None,
            })
            .collect();
        let ctx = SignalContext {
            symbol: "BTC/USDT",
            market_type: "perpetual",
            row: &fake_row(),
            candles: &bars,
            venue_snapshots: &[],
        };
        let signal = MaCrossStrategy
            .evaluate(&ctx, &MaCrossStrategy.default_params())
            .unwrap();
        assert_eq!(signal.direction, SignalDirection::Sell);
        assert!(signal.summary.contains("death cross"));
    }

    #[test]
    fn returns_none_when_no_cross() {
        let closes = vec![100.0; 25];
        let bars: Vec<OhlcvBar> = closes
            .iter()
            .map(|&c| OhlcvBar {
                open_time: String::new(),
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 100.0,
                turnover: None,
            })
            .collect();
        let ctx = SignalContext {
            symbol: "BTC/USDT",
            market_type: "perpetual",
            row: &fake_row(),
            candles: &bars,
            venue_snapshots: &[],
        };
        assert!(MaCrossStrategy
            .evaluate(&ctx, &MaCrossStrategy.default_params())
            .is_none());
    }
}
