use std::collections::HashMap;

pub const ACTIVE_STRATEGY_IDS: [&str; 5] = [
    "ma_cross",
    "rsi_extreme",
    "macd_divergence",
    "bollinger_break",
    "volume_surge",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StrategyConfig {
    pub strategy_id: String,
    pub enabled: bool,
    pub params: HashMap<String, f64>,
}

impl StrategyConfig {
    pub fn default_for(id: &str) -> Self {
        Self {
            strategy_id: id.to_string(),
            enabled: true,
            params: HashMap::new(),
        }
    }
}

/// Returns merged params: defaults overridden by user config
pub fn merged_params(
    strategy_id: &str,
    user_params: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    let mut merged = default_params_for(strategy_id);
    for (k, v) in user_params {
        merged.insert(k.clone(), *v);
    }
    merged
}

pub fn default_params_for(strategy_id: &str) -> HashMap<String, f64> {
    let mut p = HashMap::new();
    match strategy_id {
        "ma_cross" => {
            p.insert("fast_period".into(), 5.0);
            p.insert("slow_period".into(), 20.0);
            p.insert("min_confidence".into(), 55.0);
            p.insert("min_strength".into(), 0.1);
        }
        "rsi_extreme" => {
            p.insert("period".into(), 14.0);
            p.insert("oversold".into(), 30.0);
            p.insert("overbought".into(), 70.0);
            p.insert("min_confidence".into(), 55.0);
        }
        "macd_divergence" => {
            p.insert("fast".into(), 12.0);
            p.insert("slow".into(), 26.0);
            p.insert("signal".into(), 9.0);
            p.insert("min_confidence".into(), 55.0);
        }
        "bollinger_break" => {
            p.insert("period".into(), 20.0);
            p.insert("std_dev".into(), 2.0);
            p.insert("min_confidence".into(), 55.0);
        }
        "volume_surge" => {
            p.insert("lookback".into(), 20.0);
            p.insert("surge_multiplier".into(), 2.0);
            p.insert("min_confidence".into(), 55.0);
        }
        "spread_arbitrage" => {
            p.insert("min_spread_bps".into(), 5.0);
            p.insert("min_liquidity".into(), 10000.0);
        }
        "cross_market_arbitrage" => {
            p.insert("min_yield_pct".into(), 0.5);
            p.insert("min_liquidity".into(), 10000.0);
        }
        "basis_deviation" => {
            p.insert("lookback_days".into(), 7.0);
            p.insert("deviation_threshold".into(), 2.0);
        }
        _ => {}
    }
    p
}

pub fn strategy_meta() -> Vec<StrategyMeta> {
    vec![
        StrategyMeta {
            strategy_id: "ma_cross".into(),
            name: "均线交叉".into(),
            category: "Trend".into(),
            applicable_markets: vec!["a_share".into()],
            description: "识别均线金叉和死叉。".into(),
            default_params: default_params_for("ma_cross"),
        },
        StrategyMeta {
            strategy_id: "rsi_extreme".into(),
            name: "RSI 超买超卖".into(),
            category: "Momentum".into(),
            applicable_markets: vec!["a_share".into()],
            description: "识别 RSI 超买和超卖。".into(),
            default_params: default_params_for("rsi_extreme"),
        },
        StrategyMeta {
            strategy_id: "macd_divergence".into(),
            name: "MACD 背离".into(),
            category: "Momentum".into(),
            applicable_markets: vec!["a_share".into()],
            description: "识别 MACD 柱线与价格走势背离。".into(),
            default_params: default_params_for("macd_divergence"),
        },
        StrategyMeta {
            strategy_id: "bollinger_break".into(),
            name: "布林突破".into(),
            category: "Trend".into(),
            applicable_markets: vec!["a_share".into()],
            description: "识别价格突破布林带上轨或下轨。".into(),
            default_params: default_params_for("bollinger_break"),
        },
        StrategyMeta {
            strategy_id: "volume_surge".into(),
            name: "成交量异动".into(),
            category: "Momentum".into(),
            applicable_markets: vec!["a_share".into()],
            description: "识别相对历史均值的异常放量。".into(),
            default_params: default_params_for("volume_surge"),
        },
    ]
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyMeta {
    pub strategy_id: String,
    pub name: String,
    pub category: String,
    pub applicable_markets: Vec<String>,
    pub description: String,
    pub default_params: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_active_non_arbitrage_strategies_have_meta() {
        let meta = strategy_meta();
        assert_eq!(meta.len(), 5);
        for m in &meta {
            assert!(!m.strategy_id.is_empty());
            assert!(!m.name.is_empty());
            assert!(!m.description.is_empty());
            assert!(
                !m.default_params.is_empty(),
                "{} has no params",
                m.strategy_id
            );
            assert_eq!(m.applicable_markets, vec!["a_share".to_string()]);
        }
        let ids = meta
            .iter()
            .map(|item| item.strategy_id.as_str())
            .collect::<Vec<_>>();
        assert!(!ids.contains(&"funding_rate"));
        assert!(!ids.contains(&"spread_arbitrage"));
        assert!(!ids.contains(&"cross_market_arbitrage"));
        assert!(!ids.contains(&"basis_deviation"));
    }

    #[test]
    fn merged_params_overrides_defaults() {
        let mut user = HashMap::new();
        user.insert("fast_period".into(), 10.0);
        let merged = merged_params("ma_cross", &user);
        assert_eq!(merged.get("fast_period"), Some(&10.0));
        assert_eq!(merged.get("slow_period"), Some(&20.0)); // default
    }

    #[test]
    fn empty_user_params_returns_all_defaults() {
        let merged = merged_params("rsi_extreme", &HashMap::new());
        assert_eq!(merged.get("period"), Some(&14.0));
        assert_eq!(merged.get("oversold"), Some(&30.0));
    }
}
