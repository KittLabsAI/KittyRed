#[cfg(test)]
mod tests {
    use super::{evaluate_plan, CandidatePlan, RiskSettings};

    #[test]
    fn blocks_perpetual_plan_without_stop_loss() {
        let plan = CandidatePlan::perpetual_long("BTC/USDT", 3.0, None);
        let settings = RiskSettings::default();
        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
    }

    #[test]
    fn blocks_blacklisted_symbol() {
        let plan = CandidatePlan {
            symbol: "DOGE/USDT".into(),
            market_type: "spot".into(),
            direction: "long".into(),
            leverage: 1.0,
            stop_loss: Some(0.18),
            entry_low: Some(0.19),
            entry_high: Some(0.191),
            take_profit_targets: vec![0.205],
            amount_cny: Some(1_000.0),
            volume_24h: 120_000_000.0,
            spread_bps: 8.0,
            confidence_score: 72.0,
            risk_tags: vec!["Meme".into()],
        };
        let settings = RiskSettings {
            blacklist_symbols: vec!["DOGE/USDT".into()],
            ..RiskSettings::default()
        };

        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
        assert_eq!(
            result.primary_reason().as_deref(),
            Some("symbol_blacklisted")
        );
    }

    #[test]
    fn blocks_trade_when_projected_loss_exceeds_risk_budget() {
        let plan = CandidatePlan {
            symbol: "BTC/USDT".into(),
            market_type: "perpetual".into(),
            direction: "long".into(),
            leverage: 3.0,
            stop_loss: Some(95.0),
            entry_low: Some(100.0),
            entry_high: Some(100.0),
            take_profit_targets: vec![112.0],
            amount_cny: Some(2_000.0),
            volume_24h: 180_000_000.0,
            spread_bps: 4.0,
            confidence_score: 78.0,
            risk_tags: Vec::new(),
        };
        let settings = RiskSettings {
            account_equity_usdt: 10_000.0,
            max_loss_per_trade_percent: 1.0,
            ..RiskSettings::default()
        };

        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
        assert_eq!(
            result.primary_reason().as_deref(),
            Some("max_single_trade_loss_exceeded")
        );
        assert!(result
            .block_reasons
            .contains(&"max_single_trade_loss_exceeded".to_string()));
        assert!(result.max_loss_estimate.is_some());
    }

    #[test]
    fn blocks_shorts_when_direction_is_long_only() {
        let plan = CandidatePlan {
            symbol: "BTC/USDT".into(),
            market_type: "perpetual".into(),
            direction: "short".into(),
            leverage: 2.0,
            stop_loss: Some(103.0),
            entry_low: Some(100.0),
            entry_high: Some(100.0),
            take_profit_targets: vec![94.0],
            amount_cny: Some(1_000.0),
            volume_24h: 180_000_000.0,
            spread_bps: 4.0,
            confidence_score: 78.0,
            risk_tags: Vec::new(),
        };
        let settings = RiskSettings {
            allowed_direction: "long_only".into(),
            ..RiskSettings::default()
        };

        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
        assert_eq!(
            result.primary_reason().as_deref(),
            Some("direction_not_allowed")
        );
    }

    #[test]
    fn blocks_low_risk_reward_and_excessive_spread() {
        let plan = CandidatePlan {
            symbol: "SUI/USDT".into(),
            market_type: "perpetual".into(),
            direction: "long".into(),
            leverage: 2.0,
            stop_loss: Some(0.95),
            entry_low: Some(1.0),
            entry_high: Some(1.0),
            take_profit_targets: vec![1.04],
            amount_cny: Some(1_000.0),
            volume_24h: 40_000_000.0,
            spread_bps: 18.0,
            confidence_score: 74.0,
            risk_tags: Vec::new(),
        };
        let settings = RiskSettings {
            min_risk_reward_ratio: 1.5,
            max_spread_bps: 12.0,
            ..RiskSettings::default()
        };

        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
        assert_eq!(
            result.primary_reason().as_deref(),
            Some("spread_above_threshold")
        );
    }

    #[test]
    fn blocks_meme_assets_when_user_disables_them() {
        let plan = CandidatePlan {
            symbol: "DOGE/USDT".into(),
            market_type: "spot".into(),
            direction: "long".into(),
            leverage: 1.0,
            stop_loss: Some(0.18),
            entry_low: Some(0.19),
            entry_high: Some(0.191),
            take_profit_targets: vec![0.205],
            amount_cny: Some(800.0),
            volume_24h: 180_000_000.0,
            spread_bps: 4.0,
            confidence_score: 80.0,
            risk_tags: vec!["Meme".into()],
        };
        let settings = RiskSettings {
            allow_meme_coins: false,
            ..RiskSettings::default()
        };

        let result = evaluate_plan(&plan, &settings);
        assert_eq!(result.status, "blocked");
        assert_eq!(
            result.primary_reason().as_deref(),
            Some("meme_assets_disabled")
        );
    }

    #[test]
    fn returns_structured_risk_checks_for_approved_plan() {
        let plan = CandidatePlan {
            symbol: "BTC/USDT".into(),
            market_type: "perpetual".into(),
            direction: "long".into(),
            leverage: 2.0,
            stop_loss: Some(98.0),
            entry_low: Some(100.0),
            entry_high: Some(101.0),
            take_profit_targets: vec![107.0, 110.0],
            amount_cny: Some(1_000.0),
            volume_24h: 180_000_000.0,
            spread_bps: 4.0,
            confidence_score: 78.0,
            risk_tags: Vec::new(),
        };

        let result = evaluate_plan(&plan, &RiskSettings::default());

        assert_eq!(result.status, "approved");
        assert!(result.block_reasons.is_empty());
        assert!(result.risk_score > 0);
        assert!(result.max_loss_estimate.is_some());
        assert!(result
            .checks
            .iter()
            .any(|check| check.name == "max_leverage" && check.status == "passed"));
        assert!(result
            .checks
            .iter()
            .any(|check| check.name == "risk_reward_ratio" && check.status == "passed"));
    }
}

#[derive(Debug, Clone)]
pub struct CandidatePlan {
    pub symbol: String,
    pub market_type: String,
    pub direction: String,
    pub leverage: f64,
    pub stop_loss: Option<f64>,
    pub entry_low: Option<f64>,
    pub entry_high: Option<f64>,
    pub take_profit_targets: Vec<f64>,
    pub amount_cny: Option<f64>,
    pub volume_24h: f64,
    pub spread_bps: f64,
    pub confidence_score: f64,
    pub risk_tags: Vec<String>,
}

impl CandidatePlan {
    pub fn perpetual_long(symbol: &str, leverage: f64, stop_loss: Option<f64>) -> Self {
        Self {
            symbol: symbol.to_string(),
            market_type: "perpetual".into(),
            direction: "long".into(),
            leverage,
            stop_loss,
            entry_low: None,
            entry_high: None,
            take_profit_targets: Vec::new(),
            amount_cny: None,
            volume_24h: 0.0,
            spread_bps: 0.0,
            confidence_score: 0.0,
            risk_tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskSettings {
    pub allowed_markets: String,
    pub allowed_direction: String,
    pub max_leverage: f64,
    pub max_loss_per_trade_percent: f64,
    pub max_daily_loss_percent: f64,
    pub account_equity_usdt: f64,
    pub min_risk_reward_ratio: f64,
    pub min_volume_24h: f64,
    pub max_spread_bps: f64,
    pub min_confidence_score: f64,
    pub allow_meme_coins: bool,
    pub whitelist_symbols: Vec<String>,
    pub blacklist_symbols: Vec<String>,
}

impl Default for RiskSettings {
    fn default() -> Self {
        Self {
            allowed_markets: "all".into(),
            allowed_direction: "long_short".into(),
            max_leverage: 3.0,
            max_loss_per_trade_percent: 1.0,
            max_daily_loss_percent: 3.0,
            account_equity_usdt: 10_000.0,
            min_risk_reward_ratio: 1.5,
            min_volume_24h: 20_000_000.0,
            max_spread_bps: 12.0,
            min_confidence_score: 60.0,
            allow_meme_coins: true,
            whitelist_symbols: Vec::new(),
            blacklist_symbols: Vec::new(),
        }
    }
}

pub fn risk_settings_from_runtime(
    runtime: &RuntimeSettingsDto,
    account_equity_usdt: f64,
) -> RiskSettings {
    RiskSettings {
        allowed_markets: runtime.allowed_markets.clone(),
        allowed_direction: runtime.allowed_direction.clone(),
        max_leverage: runtime.max_leverage,
        max_loss_per_trade_percent: runtime.max_loss_per_trade_percent,
        max_daily_loss_percent: runtime.max_daily_loss_percent,
        account_equity_usdt,
        min_risk_reward_ratio: runtime.min_risk_reward_ratio,
        min_volume_24h: runtime.min_volume_24h,
        max_spread_bps: runtime.max_spread_bps,
        min_confidence_score: runtime.min_confidence_score,
        allow_meme_coins: runtime.allow_meme_coins,
        whitelist_symbols: runtime.whitelist_symbols.clone(),
        blacklist_symbols: runtime.blacklist_symbols.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct RiskResult {
    pub status: String,
    pub risk_score: u32,
    pub max_loss_estimate: Option<String>,
    pub checks: Vec<RiskCheckDto>,
    pub modifications: Vec<String>,
    pub block_reasons: Vec<String>,
}

impl RiskResult {
    pub fn primary_reason(&self) -> Option<String> {
        self.block_reasons.first().cloned()
    }

    pub fn to_decision_dto(&self) -> RiskDecisionDto {
        RiskDecisionDto {
            status: self.status.clone(),
            risk_score: self.risk_score,
            max_loss_estimate: self.max_loss_estimate.clone(),
            checks: self.checks.clone(),
            modifications: self.modifications.clone(),
            block_reasons: self.block_reasons.clone(),
        }
    }
}

pub fn evaluate_plan(plan: &CandidatePlan, settings: &RiskSettings) -> RiskResult {
    let mut checks = Vec::new();
    let mut block_reasons = Vec::new();
    let modifications = Vec::new();

    let market_allowed = match settings.allowed_markets.as_str() {
        "spot" => plan.market_type == "spot",
        "perpetual" => plan.market_type == "perpetual",
        _ => true,
    };
    record_check(
        &mut checks,
        &mut block_reasons,
        "market_type_allowed",
        market_allowed,
        Some("market_not_allowed"),
        Some(format!(
            "requested {}, allowed {}",
            plan.market_type, settings.allowed_markets
        )),
    );

    let direction_allowed = if settings.allowed_direction == "observe_only" {
        false
    } else {
        !(settings.allowed_direction == "long_only" && plan.direction.eq_ignore_ascii_case("short"))
    };
    let direction_block_reason = if settings.allowed_direction == "observe_only" {
        Some("observe_only_mode")
    } else {
        Some("direction_not_allowed")
    };
    record_check(
        &mut checks,
        &mut block_reasons,
        "direction_allowed",
        direction_allowed,
        direction_block_reason,
        Some(format!(
            "requested {}, mode {}",
            plan.direction, settings.allowed_direction
        )),
    );

    record_check(
        &mut checks,
        &mut block_reasons,
        "symbol_blacklist",
        !symbol_is_listed(&plan.symbol, &settings.blacklist_symbols),
        Some("symbol_blacklisted"),
        Some(plan.symbol.clone()),
    );

    let whitelist_ok = settings.whitelist_symbols.is_empty()
        || symbol_is_listed(&plan.symbol, &settings.whitelist_symbols);
    record_check(
        &mut checks,
        &mut block_reasons,
        "symbol_whitelist",
        whitelist_ok,
        Some("symbol_not_whitelisted"),
        Some(if settings.whitelist_symbols.is_empty() {
            "no whitelist configured".into()
        } else {
            plan.symbol.clone()
        }),
    );

    let meme_allowed = settings.allow_meme_coins
        || !plan
            .risk_tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case("meme"));
    record_check(
        &mut checks,
        &mut block_reasons,
        "meme_asset_policy",
        meme_allowed,
        Some("meme_assets_disabled"),
        Some(if settings.allow_meme_coins {
            "meme assets enabled".into()
        } else {
            plan.risk_tags.join(", ")
        }),
    );

    let stop_loss_required = plan.market_type != "perpetual" || plan.stop_loss.is_some();
    record_check(
        &mut checks,
        &mut block_reasons,
        "stop_loss_required",
        stop_loss_required,
        Some("missing_stop_loss"),
        Some(plan.market_type.clone()),
    );

    record_check(
        &mut checks,
        &mut block_reasons,
        "max_leverage",
        plan.leverage <= settings.max_leverage,
        Some("max_leverage_exceeded"),
        Some(format!(
            "requested {:.2}x, max {:.2}x",
            plan.leverage, settings.max_leverage
        )),
    );

    record_check(
        &mut checks,
        &mut block_reasons,
        "min_volume_24h",
        plan.volume_24h >= settings.min_volume_24h,
        Some("volume_below_threshold"),
        Some(format!(
            "observed {:.0}, min {:.0}",
            plan.volume_24h, settings.min_volume_24h
        )),
    );

    record_check(
        &mut checks,
        &mut block_reasons,
        "max_spread_bps",
        plan.spread_bps <= settings.max_spread_bps,
        Some("spread_above_threshold"),
        Some(format!(
            "observed {:.2} bps, max {:.2} bps",
            plan.spread_bps, settings.max_spread_bps
        )),
    );

    record_check(
        &mut checks,
        &mut block_reasons,
        "min_confidence_score",
        plan.confidence_score >= settings.min_confidence_score,
        Some("confidence_below_threshold"),
        Some(format!(
            "observed {:.1}, min {:.1}",
            plan.confidence_score, settings.min_confidence_score
        )),
    );

    let max_loss_percent = projected_loss_percent_of_equity(plan, settings);
    match max_loss_percent {
        Some(value) => record_check(
            &mut checks,
            &mut block_reasons,
            "max_single_trade_loss",
            value <= settings.max_loss_per_trade_percent,
            Some("max_single_trade_loss_exceeded"),
            Some(format!(
                "projected {:.2}%, max {:.2}%",
                value, settings.max_loss_per_trade_percent
            )),
        ),
        None => record_skipped_check(
            &mut checks,
            "max_single_trade_loss",
            "entry, stop, or account context missing".into(),
        ),
    }

    let risk_reward_ratio = projected_risk_reward_ratio(plan);
    match risk_reward_ratio {
        Some(value) => record_check(
            &mut checks,
            &mut block_reasons,
            "risk_reward_ratio",
            value >= settings.min_risk_reward_ratio,
            Some("risk_reward_below_threshold"),
            Some(format!(
                "projected {:.2}, min {:.2}",
                value, settings.min_risk_reward_ratio
            )),
        ),
        None => record_skipped_check(
            &mut checks,
            "risk_reward_ratio",
            "take-profit or stop context missing".into(),
        ),
    }

    RiskResult {
        status: if block_reasons.is_empty() {
            "approved".into()
        } else {
            "blocked".into()
        },
        risk_score: compute_risk_score(
            plan,
            settings,
            max_loss_percent,
            risk_reward_ratio,
            block_reasons.len(),
        ),
        max_loss_estimate: max_loss_percent.map(|value| format!("{value:.2}%")),
        checks,
        modifications,
        block_reasons,
    }
}

fn record_check(
    checks: &mut Vec<RiskCheckDto>,
    block_reasons: &mut Vec<String>,
    name: &str,
    passed: bool,
    block_reason: Option<&str>,
    detail: Option<String>,
) {
    checks.push(RiskCheckDto {
        name: name.into(),
        status: if passed {
            "passed".into()
        } else {
            "blocked".into()
        },
        detail,
    });

    if !passed {
        if let Some(reason) = block_reason {
            block_reasons.push(reason.into());
        }
    }
}

fn record_skipped_check(checks: &mut Vec<RiskCheckDto>, name: &str, detail: String) {
    checks.push(RiskCheckDto {
        name: name.into(),
        status: "skipped".into(),
        detail: Some(detail),
    });
}

fn compute_risk_score(
    plan: &CandidatePlan,
    settings: &RiskSettings,
    max_loss_percent: Option<f64>,
    risk_reward_ratio: Option<f64>,
    block_count: usize,
) -> u32 {
    let leverage_component =
        ((plan.leverage / settings.max_leverage.max(1.0)) * 24.0).clamp(0.0, 24.0);
    let loss_component = max_loss_percent
        .map(|value| (value / settings.max_loss_per_trade_percent.max(0.1)) * 28.0)
        .unwrap_or(12.0)
        .clamp(0.0, 28.0);
    let spread_component =
        ((plan.spread_bps / settings.max_spread_bps.max(1.0)) * 18.0).clamp(0.0, 18.0);
    let confidence_component =
        (((100.0 - plan.confidence_score).max(0.0) / 100.0) * 16.0).clamp(0.0, 16.0);
    let reward_component = risk_reward_ratio
        .map(|value| ((settings.min_risk_reward_ratio / value.max(0.25)) * 10.0).clamp(0.0, 14.0))
        .unwrap_or(8.0);
    let block_penalty = (block_count as f64 * 12.0).clamp(0.0, 32.0);

    (leverage_component
        + loss_component
        + spread_component
        + confidence_component
        + reward_component
        + block_penalty)
        .round()
        .clamp(0.0, 100.0) as u32
}

fn symbol_is_listed(symbol: &str, list: &[String]) -> bool {
    list.iter().any(|item| item.eq_ignore_ascii_case(symbol))
}

fn projected_loss_percent_of_equity(plan: &CandidatePlan, settings: &RiskSettings) -> Option<f64> {
    let entry = average_entry_price(plan.entry_low, plan.entry_high)?;
    let stop = plan.stop_loss?;
    let amount_cny = plan.amount_cny?;

    if settings.account_equity_usdt <= f64::EPSILON || entry <= f64::EPSILON {
        return None;
    }

    let modeled_loss_usdt = amount_cny * plan.leverage * ((entry - stop).abs() / entry);
    Some((modeled_loss_usdt / settings.account_equity_usdt) * 100.0)
}

fn projected_risk_reward_ratio(plan: &CandidatePlan) -> Option<f64> {
    let entry = average_entry_price(plan.entry_low, plan.entry_high)?;
    let stop = plan.stop_loss?;
    let risk = (entry - stop).abs();
    if risk <= f64::EPSILON {
        return None;
    }

    let best_reward = plan
        .take_profit_targets
        .iter()
        .filter_map(|target| {
            let reward = if plan.direction.eq_ignore_ascii_case("short") {
                entry - target
            } else {
                target - entry
            };
            (reward > 0.0).then_some(reward)
        })
        .fold(None::<f64>, |best, reward| match best {
            Some(current) => Some(current.max(reward)),
            None => Some(reward),
        })?;

    Some(best_reward / risk)
}

fn average_entry_price(entry_low: Option<f64>, entry_high: Option<f64>) -> Option<f64> {
    match (entry_low, entry_high) {
        (Some(low), Some(high)) => Some((low + high) / 2.0),
        (Some(low), None) => Some(low),
        (None, Some(high)) => Some(high),
        _ => None,
    }
}
use crate::models::{RiskCheckDto, RiskDecisionDto, RuntimeSettingsDto};
