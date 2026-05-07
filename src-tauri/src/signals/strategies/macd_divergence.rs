use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::signals::indicators::macd;
use std::collections::HashMap;

pub struct MacdDivergenceStrategy;

impl Strategy for MacdDivergenceStrategy {
    fn id(&self) -> &'static str {
        "macd_divergence"
    }
    fn name(&self) -> &'static str {
        "MACD Divergence"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Trend
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects divergence between MACD histogram and price action"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("macd_divergence")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let fast = params.get("fast").copied().unwrap_or(12.0);
        let slow = params.get("slow").copied().unwrap_or(26.0);
        if fast >= slow {
            return Err("fast must be < slow".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let _fast = params.get("fast").copied().unwrap_or(12.0);
        let _slow = params.get("slow").copied().unwrap_or(26.0);
        let _signal = params.get("signal").copied().unwrap_or(9.0);
        let min_confidence = params.get("min_confidence").copied().unwrap_or(55.0);

        if ctx.candles.len() < 27 {
            return None;
        }
        let (macd_line, signal_line, _hist) = macd(ctx.candles);
        let len = ctx.candles.len();
        let prev_m = macd_line[len - 2];
        let prev_s = signal_line[len - 2];
        let curr_m = macd_line[len - 1];
        let curr_s = signal_line[len - 1];
        if curr_m.is_nan() || curr_s.is_nan() || prev_m.is_nan() || prev_s.is_nan() {
            return None;
        }

        let (direction, summary) = if prev_m <= prev_s && curr_m > curr_s && curr_m > 0.0 {
            (
                SignalDirection::Buy,
                format!("MACD golden cross above zero at {:.4}", curr_m),
            )
        } else if prev_m >= prev_s && curr_m < curr_s && curr_m < 0.0 {
            (
                SignalDirection::Sell,
                format!("MACD death cross below zero at {:.4}", curr_m),
            )
        } else {
            return None;
        };

        let strength = ((curr_m - curr_s).abs() / curr_s.abs().max(1e-8)).clamp(0.1, 1.0);
        let confidence = (60.0 + strength * 20.0).clamp(min_confidence, 88.0);
        let mut metrics = HashMap::new();
        metrics.insert("macd".to_string(), curr_m);
        metrics.insert("signal".to_string(), curr_s);

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
            symbol: "BTC/USDT".into(),
            base_asset: "BTC".into(),
            market_type: "perpetual".into(),
            market_cap_usd: None,
            market_cap_rank: None,
            market_size_tier: "large".into(),
            last_price: 100.0,
            change_24h: 1.0,
            volume_24h: 1_000_000_000.0,
            funding_rate: None,
            spread_bps: 2.0,
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
    fn warmup_returns_none() {
        let bars: Vec<OhlcvBar> = (0..10)
            .map(|i| OhlcvBar {
                open_time: i.to_string(),
                open: 100.0,
                high: 100.0,
                low: 100.0,
                close: 100.0,
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
        assert!(MacdDivergenceStrategy
            .evaluate(&ctx, &MacdDivergenceStrategy.default_params())
            .is_none());
    }
}
