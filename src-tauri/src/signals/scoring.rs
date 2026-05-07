use super::strategies::{SignalCategory, SignalDirection, StrategySignal};
use super::UnifiedSignal;
use std::collections::HashMap;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub struct ScoringConfig {
    pub trend_weight: f64,
    pub momentum_weight: f64,
    pub funding_weight: f64,
    pub arbitrage_weight: f64,
    pub min_volume_usdt: f64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            trend_weight: 0.35,
            momentum_weight: 0.25,
            funding_weight: 0.15,
            arbitrage_weight: 0.25,
            min_volume_usdt: 50_000_000.0,
        }
    }
}

fn category_weight(category: SignalCategory, config: &ScoringConfig) -> f64 {
    match category {
        SignalCategory::Trend => config.trend_weight,
        SignalCategory::Momentum => config.momentum_weight,
        SignalCategory::Funding => config.funding_weight,
        SignalCategory::Arbitrage => config.arbitrage_weight,
    }
}

pub fn aggregate(
    signals: Vec<StrategySignal>,
    symbol: &str,
    market_type: &str,
    last_price: f64,
    volume_24h: f64,
    config: &ScoringConfig,
) -> Option<UnifiedSignal> {
    if signals.is_empty() {
        return None;
    }

    let buy_count = signals
        .iter()
        .filter(|s| s.direction == SignalDirection::Buy)
        .count();
    let sell_count = signals
        .iter()
        .filter(|s| s.direction == SignalDirection::Sell)
        .count();
    let total = buy_count + sell_count;
    if total == 0 {
        return None;
    }

    let (winning_direction, agreed_count) = if buy_count > sell_count {
        (SignalDirection::Buy, buy_count)
    } else {
        (SignalDirection::Sell, sell_count)
    };

    let consensus_ratio = agreed_count as f64 / total as f64;
    if consensus_ratio < 0.6 {
        return None;
    }

    let mut weighted_score = 0.0;
    let mut total_weight = 0.0;
    let mut category_breakdown = HashMap::new();
    let mut contributors = Vec::new();

    for signal in &signals {
        if signal.direction != winning_direction {
            continue;
        }
        let weight = category_weight(signal.category, config);
        weighted_score += signal.strength * signal.confidence * weight;
        total_weight += weight;
        *category_breakdown
            .entry(format!("{:?}", signal.category).to_lowercase())
            .or_insert(0.0) += signal.strength * signal.confidence * weight;
        contributors.push(signal.strategy_id.clone());
    }

    let raw_score = if total_weight > 0.0 {
        weighted_score / total_weight
    } else {
        0.0
    };
    let consistency_bonus = 1.0 + consensus_ratio * 0.3;
    let volume_penalty = if volume_24h < config.min_volume_usdt {
        (0.6 + 0.4 * volume_24h / config.min_volume_usdt).max(0.6)
    } else {
        1.0
    };
    let score = (raw_score * consistency_bonus * volume_penalty).clamp(0.0, 100.0);
    let strength = (score / 100.0).clamp(0.0, 1.0);

    let tick_size = if last_price >= 1_000.0 {
        0.01
    } else if last_price >= 1.0 {
        0.001
    } else {
        0.0001
    };
    let atr_estimate = last_price * 0.015;
    let entry_zone_low = if winning_direction == SignalDirection::Buy {
        (last_price * 0.998 / tick_size).round() * tick_size
    } else {
        (last_price * 1.002 / tick_size).round() * tick_size
    };
    let entry_zone_high = if winning_direction == SignalDirection::Buy {
        (last_price * 1.002 / tick_size).round() * tick_size
    } else {
        (last_price * 0.998 / tick_size).round() * tick_size
    };
    let stop_loss = if winning_direction == SignalDirection::Buy {
        ((entry_zone_low - atr_estimate * 1.5) / tick_size).round() * tick_size
    } else {
        ((entry_zone_high + atr_estimate * 1.5) / tick_size).round() * tick_size
    };
    let take_profit = if winning_direction == SignalDirection::Buy {
        ((entry_zone_high + atr_estimate * 3.0) / tick_size).round() * tick_size
    } else {
        ((entry_zone_low - atr_estimate * 3.0) / tick_size).round() * tick_size
    };

    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::new());
    let signal_id = format!(
        "sig-{}-{}-{}",
        symbol.replace('/', "-").to_lowercase(),
        market_type,
        generated_at
    );

    let reason_parts: Vec<String> = signals
        .iter()
        .filter(|s| s.direction == winning_direction)
        .map(|s| s.summary.clone())
        .collect();
    let reason_summary = reason_parts.join(" | ");

    Some(UnifiedSignal {
        signal_id,
        symbol: symbol.to_string(),
        market_type: market_type.to_string(),
        direction: winning_direction,
        score,
        strength,
        category_breakdown,
        contributors,
        entry_zone_low,
        entry_zone_high,
        stop_loss,
        take_profit,
        reason_summary,
        risk_status: "pending".to_string(),
        generated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::strategies::{SignalCategory, SignalDirection, StrategySignal};
    use std::collections::HashMap;

    fn make_signal(
        id: &str,
        category: SignalCategory,
        direction: SignalDirection,
        strength: f64,
        confidence: f64,
    ) -> StrategySignal {
        StrategySignal {
            strategy_id: id.into(),
            category,
            direction,
            strength,
            confidence,
            summary: format!("{} signal", id),
            metrics: HashMap::new(),
        }
    }

    #[test]
    fn aggregates_buy_signals() {
        let signals = vec![
            make_signal(
                "ma_cross",
                SignalCategory::Trend,
                SignalDirection::Buy,
                0.8,
                75.0,
            ),
            make_signal(
                "rsi_extreme",
                SignalCategory::Momentum,
                SignalDirection::Buy,
                0.6,
                65.0,
            ),
        ];
        let result = aggregate(
            signals,
            "BTC/USDT",
            "perpetual",
            100.0,
            200_000_000.0,
            &ScoringConfig::default(),
        );
        assert!(result.is_some());
        let signal = result.unwrap();
        assert_eq!(signal.direction, SignalDirection::Buy);
        assert_eq!(signal.contributors.len(), 2);
        assert!(signal.score > 0.0);
        assert!(signal.entry_zone_low < signal.entry_zone_high);
    }

    #[test]
    fn rejects_mixed_direction_signals() {
        let signals = vec![
            make_signal(
                "ma_cross",
                SignalCategory::Trend,
                SignalDirection::Buy,
                0.8,
                75.0,
            ),
            make_signal(
                "bollinger",
                SignalCategory::Trend,
                SignalDirection::Sell,
                0.7,
                70.0,
            ),
        ];
        let result = aggregate(
            signals,
            "BTC/USDT",
            "perpetual",
            100.0,
            200_000_000.0,
            &ScoringConfig::default(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn low_volume_penalizes_score() {
        let signals = vec![make_signal(
            "ma_cross",
            SignalCategory::Trend,
            SignalDirection::Buy,
            0.9,
            80.0,
        )];
        let high_vol = aggregate(
            signals.clone(),
            "X",
            "spot",
            10.0,
            200_000_000.0,
            &ScoringConfig::default(),
        )
        .unwrap();
        let low_vol = aggregate(
            signals,
            "X",
            "spot",
            10.0,
            10_000_000.0,
            &ScoringConfig::default(),
        )
        .unwrap();
        assert!(high_vol.score > low_vol.score);
    }

    #[test]
    fn empty_signals_returns_none() {
        assert!(aggregate(
            vec![],
            "X",
            "spot",
            10.0,
            100_000_000.0,
            &ScoringConfig::default()
        )
        .is_none());
    }
}
