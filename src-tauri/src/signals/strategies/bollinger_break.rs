use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::signals::indicators::bollinger_bands;
use std::collections::HashMap;

pub struct BollingerBreakStrategy;

impl Strategy for BollingerBreakStrategy {
    fn id(&self) -> &'static str {
        "bollinger_break"
    }
    fn name(&self) -> &'static str {
        "Bollinger Break"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Trend
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects price breaking above upper or below lower Bollinger bands"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("bollinger_break")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let period = params.get("period").copied().unwrap_or(20.0);
        let std_dev = params.get("std_dev").copied().unwrap_or(2.0);
        if period < 5.0 {
            return Err("period must be ≥ 5".into());
        }
        if std_dev <= 0.0 {
            return Err("std_dev must be > 0".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let period = params.get("period").copied().unwrap_or(20.0) as usize;
        let std_dev = params.get("std_dev").copied().unwrap_or(2.0);
        let min_confidence = params.get("min_confidence").copied().unwrap_or(55.0);

        if ctx.candles.len() < period + 1 {
            return None;
        }
        let (upper, middle, lower) = bollinger_bands(ctx.candles, period, std_dev);
        let i = ctx.candles.len() - 1;
        let price = ctx.candles[i].close;
        let up = upper[i];
        let mid = middle[i];
        let low = lower[i];
        if up.is_nan() || low.is_nan() {
            return None;
        }

        let (direction, summary) = if price < low {
            (
                SignalDirection::Buy,
                format!(
                    "Price {:.2} below lower Bollinger band {:.2} — mean reversion buy",
                    price, low
                ),
            )
        } else if price > up {
            (
                SignalDirection::Sell,
                format!(
                    "Price {:.2} above upper Bollinger band {:.2} — mean reversion sell",
                    price, up
                ),
            )
        } else {
            return None;
        };

        let deviation = (price - mid).abs() / mid.max(1e-8);
        let strength = (deviation * 10.0).clamp(0.1, 1.0);
        let confidence = (60.0 + deviation * 100.0).clamp(min_confidence, 90.0);
        let mut metrics = HashMap::new();
        metrics.insert("upper".to_string(), up);
        metrics.insert("middle".to_string(), mid);
        metrics.insert("lower".to_string(), low);

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
