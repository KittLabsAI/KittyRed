use super::UnifiedSignal;
use crate::models::RiskDecisionDto;
use crate::recommendations::risk_engine;

pub fn evaluate_signal(
    signal: &UnifiedSignal,
    _settings: &risk_engine::RiskSettings,
    min_score: f64,
    cooldown_minutes: u32,
    daily_max: u32,
    signals_today: u32,
    last_same_direction_at_ms: Option<i64>,
) -> RiskDecisionDto {
    let mut checks = Vec::new();
    let mut block_reasons = Vec::new();

    // Score minimum check
    let score_ok = signal.score >= min_score;
    checks.push(crate::models::RiskCheckDto {
        name: "signal_score_minimum".into(),
        status: if score_ok {
            "passed".into()
        } else {
            "blocked".into()
        },
        detail: Some(format!("score {:.1}, min {:.1}", signal.score, min_score)),
    });
    if !score_ok {
        block_reasons.push("signal_score_below_minimum".into());
    }

    // Contributor count check
    let contributor_ok = signal.contributors.len() >= 2;
    checks.push(crate::models::RiskCheckDto {
        name: "signal_contributor_minimum".into(),
        status: if contributor_ok {
            "passed".into()
        } else {
            "blocked".into()
        },
        detail: Some(format!("{} contributors, min 2", signal.contributors.len())),
    });
    if !contributor_ok {
        block_reasons.push("contributor_count_below_minimum".into());
    }

    // Cooldown check
    let cooldown_ok = match last_same_direction_at_ms {
        Some(ts) => {
            let now_ms =
                (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
            let elapsed_min = ((now_ms - ts) / 60_000).max(0) as u32;
            elapsed_min >= cooldown_minutes
        }
        None => true,
    };
    checks.push(crate::models::RiskCheckDto {
        name: "signal_cooldown".into(),
        status: if cooldown_ok {
            "passed".into()
        } else {
            "blocked".into()
        },
        detail: Some(format!("cooldown {} min", cooldown_minutes)),
    });
    if !cooldown_ok {
        block_reasons.push("signal_cooldown_active".into());
    }

    // Daily max check
    let daily_ok = signals_today < daily_max;
    checks.push(crate::models::RiskCheckDto {
        name: "signal_daily_max".into(),
        status: if daily_ok {
            "passed".into()
        } else {
            "blocked".into()
        },
        detail: Some(format!("{} today, max {}", signals_today, daily_max)),
    });
    if !daily_ok {
        block_reasons.push("daily_max_signals_exceeded".into());
    }

    let status = if block_reasons.is_empty() {
        "approved".into()
    } else {
        "blocked".into()
    };

    RiskDecisionDto {
        status,
        risk_score: (signal.score as u32).clamp(0, 100),
        max_loss_estimate: Some(format!(
            "{:.2} USDT",
            (signal.entry_zone_low - signal.stop_loss).abs().max(0.0)
        )),
        checks,
        modifications: Vec::new(),
        block_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommendations::risk_engine::RiskSettings;

    fn test_signal(score: f64, contributors: Vec<String>) -> UnifiedSignal {
        UnifiedSignal {
            signal_id: "sig-test-1".into(),
            symbol: "BTC/USDT".into(),
            market_type: "perpetual".into(),
            direction: super::super::strategies::SignalDirection::Buy,
            score,
            strength: score / 100.0,
            category_breakdown: std::collections::HashMap::new(),
            contributors,
            entry_zone_low: 99.0,
            entry_zone_high: 101.0,
            stop_loss: 97.0,
            take_profit: 107.0,
            reason_summary: "test".into(),
            risk_status: "pending".into(),
            generated_at: "x".into(),
        }
    }

    #[test]
    fn blocks_low_score() {
        let signal = test_signal(20.0, vec!["a".into(), "b".into()]);
        let result = evaluate_signal(&signal, &RiskSettings::default(), 30.0, 15, 50, 0, None);
        assert_eq!(result.status, "blocked");
        assert!(result
            .block_reasons
            .contains(&"signal_score_below_minimum".to_string()));
    }

    #[test]
    fn blocks_single_contributor() {
        let signal = test_signal(60.0, vec!["a".into()]);
        let result = evaluate_signal(&signal, &RiskSettings::default(), 30.0, 15, 50, 0, None);
        assert!(result
            .block_reasons
            .contains(&"contributor_count_below_minimum".to_string()));
    }

    #[test]
    fn approves_valid_signal() {
        let signal = test_signal(65.0, vec!["a".into(), "b".into(), "c".into()]);
        let result = evaluate_signal(&signal, &RiskSettings::default(), 30.0, 15, 50, 0, None);
        assert_eq!(result.status, "approved");
    }
}
