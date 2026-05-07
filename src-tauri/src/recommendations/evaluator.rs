#[cfg(test)]
mod tests {
    use super::estimate_pnl_percent;

    #[test]
    fn computes_long_pnl_percent() {
        let pnl = estimate_pnl_percent(100.0, 105.0, "long", 0.1);
        assert!(pnl > 0.0);
    }
}

pub fn estimate_pnl_percent(entry: f64, exit: f64, direction: &str, total_cost_rate: f64) -> f64 {
    let gross = match direction {
        "long" | "spot_buy" => (exit - entry) / entry,
        "short" => (entry - exit) / entry,
        _ => 0.0,
    };

    gross - (total_cost_rate / 100.0)
}
