use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use std::collections::HashMap;

pub struct FundingRateStrategy;

impl Strategy for FundingRateStrategy {
    fn id(&self) -> &'static str {
        "funding_rate"
    }
    fn name(&self) -> &'static str {
        "Funding Rate"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Funding
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Signals extreme funding rates suggesting contrarian positioning"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("funding_rate")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let long_t = params.get("long_threshold").copied().unwrap_or(-0.001);
        let short_t = params.get("short_threshold").copied().unwrap_or(0.001);
        if long_t >= short_t {
            return Err("long_threshold must be < short_threshold".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let long_threshold = params.get("long_threshold").copied().unwrap_or(-0.001);
        let short_threshold = params.get("short_threshold").copied().unwrap_or(0.001);

        let funding = ctx.row.funding_rate?;
        let funding_pct = funding * 100.0;

        let (direction, summary) = if funding_pct < long_threshold * 100.0 {
            (
                SignalDirection::Buy,
                format!(
                    "Funding rate extremely negative at {:.3}% — longs get paid",
                    funding_pct
                ),
            )
        } else if funding_pct > short_threshold * 100.0 {
            (
                SignalDirection::Sell,
                format!(
                    "Funding rate extremely positive at {:.3}% — shorts get paid",
                    funding_pct
                ),
            )
        } else {
            return None;
        };

        let strength = (funding_pct.abs() / 0.5).clamp(0.1, 1.0);
        let confidence = (55.0 + funding_pct.abs() * 100.0).clamp(55.0, 85.0);
        let mut metrics = HashMap::new();
        metrics.insert("funding_rate".to_string(), funding);

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
