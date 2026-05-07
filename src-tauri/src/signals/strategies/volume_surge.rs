use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::signals::indicators::avg_volume;
use std::collections::HashMap;

pub struct VolumeSurgeStrategy;

impl Strategy for VolumeSurgeStrategy {
    fn id(&self) -> &'static str {
        "volume_surge"
    }
    fn name(&self) -> &'static str {
        "Volume Surge"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Momentum
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects abnormal volume spikes relative to historical average"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("volume_surge")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let lookback = params.get("lookback").copied().unwrap_or(20.0);
        let surge = params.get("surge_multiplier").copied().unwrap_or(2.0);
        if lookback < 5.0 {
            return Err("lookback must be ≥ 5".into());
        }
        if surge <= 1.0 {
            return Err("surge_multiplier must be > 1.0".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let lookback = params.get("lookback").copied().unwrap_or(20.0) as usize;
        let surge_multiplier = params.get("surge_multiplier").copied().unwrap_or(2.0);
        let min_confidence = params.get("min_confidence").copied().unwrap_or(55.0);

        if ctx.candles.len() < lookback + 1 {
            return None;
        }
        let avg_vol = avg_volume(ctx.candles, lookback);
        let i = ctx.candles.len() - 1;
        let current_vol = ctx.candles[i].volume;
        let avg = avg_vol[i];
        if avg.is_nan() || avg <= 0.0 {
            return None;
        }
        let ratio = current_vol / avg;
        if ratio < surge_multiplier {
            return None;
        }

        let price_change = ctx.candles[i].close - ctx.candles[i - 1].close;
        let (direction, summary) = if price_change > 0.0 {
            (
                SignalDirection::Buy,
                format!(
                    "Volume {:.1}x avg with +{:.2}% — bullish surge",
                    ratio,
                    price_change / ctx.candles[i - 1].close * 100.0
                ),
            )
        } else {
            (
                SignalDirection::Sell,
                format!(
                    "Volume {:.1}x avg with {:.2}% drop — bearish surge",
                    ratio,
                    price_change / ctx.candles[i - 1].close * 100.0
                ),
            )
        };

        let strength = ((ratio - surge_multiplier) / 3.0).clamp(0.1, 1.0);
        let confidence = (60.0 + (ratio - surge_multiplier) * 10.0).clamp(min_confidence, 90.0);
        let mut metrics = HashMap::new();
        metrics.insert("volume_ratio".to_string(), ratio);
        metrics.insert("avg_volume".to_string(), avg);

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
