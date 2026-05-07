use super::{
    ApplicableMarket, SignalCategory, SignalContext, SignalDirection, Strategy, StrategySignal,
};
use std::collections::HashMap;

pub struct CrossMarketArbitrageStrategy;

impl Strategy for CrossMarketArbitrageStrategy {
    fn id(&self) -> &'static str {
        "cross_market_arbitrage"
    }
    fn name(&self) -> &'static str {
        "Cross Market Arbitrage"
    }
    fn category(&self) -> SignalCategory {
        SignalCategory::Arbitrage
    }
    fn applicable_to(&self) -> Vec<ApplicableMarket> {
        vec![ApplicableMarket::Spot, ApplicableMarket::Perpetual]
    }
    fn description(&self) -> &'static str {
        "Detects spot-perpetual cross-market arbitrage opportunities"
    }
    fn default_params(&self) -> HashMap<String, f64> {
        crate::signals::config::default_params_for("cross_market_arbitrage")
    }
    fn validate_params(&self, params: &HashMap<String, f64>) -> Result<(), String> {
        let min_yield = params.get("min_yield_pct").copied().unwrap_or(0.5);
        if min_yield <= 0.0 {
            return Err("min_yield_pct must be > 0".into());
        }
        Ok(())
    }
    fn evaluate(
        &self,
        ctx: &SignalContext,
        params: &HashMap<String, f64>,
    ) -> Option<StrategySignal> {
        let min_yield_pct = params.get("min_yield_pct").copied().unwrap_or(0.5);
        let _min_liquidity = params.get("min_liquidity").copied().unwrap_or(10000.0);

        let perp_venues: Vec<_> = ctx
            .venue_snapshots
            .iter()
            .filter(|v| v.mark_price.is_some())
            .collect();
        if perp_venues.is_empty() {
            return None;
        }

        let perp = perp_venues[0];
        let mark = perp.mark_price?;
        let spot_price = ctx.row.last_price;
        if spot_price <= 0.0 || mark <= 0.0 {
            return None;
        }

        let basis_pct = (mark - spot_price) / spot_price * 100.0;

        let (direction, summary) = if basis_pct > min_yield_pct {
            (
                SignalDirection::Sell,
                format!(
                    "Perp premium {:.3}% over spot — short perp, buy spot basis trade",
                    basis_pct
                ),
            )
        } else if basis_pct < -min_yield_pct {
            (
                SignalDirection::Buy,
                format!(
                    "Perp discount {:.3}% under spot — long perp, short spot basis trade",
                    basis_pct
                ),
            )
        } else {
            return None;
        };

        let strength = (basis_pct.abs() / 2.0).clamp(0.1, 1.0);
        let confidence = (60.0 + basis_pct.abs() * 20.0).clamp(55.0, 90.0);
        let mut metrics = HashMap::new();
        metrics.insert("basis_pct".to_string(), basis_pct);
        metrics.insert("spot_price".to_string(), spot_price);
        metrics.insert("mark_price".to_string(), mark);

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
