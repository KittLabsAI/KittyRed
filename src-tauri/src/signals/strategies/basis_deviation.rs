use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use crate::models::OhlcvBar;
use std::collections::HashMap;

pub struct BasisDeviationStrategy;

impl Strategy for BasisDeviationStrategy {
    fn id(&self) -> &'static str {
        "basis_deviation"
    }
    fn name(&self) -> &'static str {
        "Basis Deviation"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Arbitrage
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Identifies abnormal deviations between spot and perpetual prices"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("basis_deviation")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let lookback = params.get("lookback_days").copied().unwrap_or(7.0);
        let threshold = params.get("deviation_threshold").copied().unwrap_or(2.0);
        if lookback < 1.0 {
            return Err("lookback_days must be ≥ 1".into());
        }
        if threshold <= 0.0 {
            return Err("deviation_threshold must be > 0".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let lookback = params.get("lookback_days").copied().unwrap_or(7.0) as usize;
        let deviation_threshold = params.get("deviation_threshold").copied().unwrap_or(2.0);

        let perp_venue = ctx
            .venue_snapshots
            .iter()
            .find(|v| v.index_price.is_some() && v.mark_price.is_some())?;
        let mark = perp_venue.mark_price?;
        let index = perp_venue.index_price?;
        if mark <= 0.0 || index <= 0.0 {
            return None;
        }

        let current_basis = (mark - index) / index;

        let basis_bars: Vec<OhlcvBar> = ctx
            .candles
            .iter()
            .map(|c| OhlcvBar {
                close: {
                    let idx = perp_venue.index_price.unwrap_or(c.close);
                    if idx > 0.0 {
                        (c.close - idx) / idx
                    } else {
                        0.0
                    }
                },
                ..c.clone()
            })
            .collect();

        if basis_bars.len() < lookback {
            return None;
        }

        let mean: f64 = basis_bars[basis_bars.len() - lookback..]
            .iter()
            .map(|b| b.close)
            .sum::<f64>()
            / lookback as f64;
        let variance: f64 = basis_bars[basis_bars.len() - lookback..]
            .iter()
            .map(|b| (b.close - mean).powi(2))
            .sum::<f64>()
            / lookback as f64;
        let std = variance.sqrt();
        if std <= 0.0 {
            return None;
        }

        let z_score = (current_basis - mean) / std;

        let (direction, summary) = if z_score > deviation_threshold {
            (
                SignalDirection::Sell,
                format!(
                    "Basis {:.4} is {:.1}σ above mean {:.4} — expect mean reversion lower",
                    current_basis, z_score, mean
                ),
            )
        } else if z_score < -deviation_threshold {
            (
                SignalDirection::Buy,
                format!(
                    "Basis {:.4} is {:.1}σ below mean {:.4} — expect mean reversion higher",
                    current_basis, z_score, mean
                ),
            )
        } else {
            return None;
        };

        let strength =
            ((z_score.abs() - deviation_threshold) / deviation_threshold).clamp(0.1, 1.0);
        let confidence = (60.0 + (z_score.abs() - deviation_threshold) * 10.0).clamp(55.0, 88.0);
        let mut metrics = HashMap::new();
        metrics.insert("z_score".to_string(), z_score);
        metrics.insert("basis".to_string(), current_basis);
        metrics.insert("mean_basis".to_string(), mean);
        metrics.insert("std_basis".to_string(), std);

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
