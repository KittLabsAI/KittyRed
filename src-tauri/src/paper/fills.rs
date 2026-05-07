#[cfg(test)]
mod tests {
    use super::estimate_market_fill_price;

    #[test]
    fn uses_orderbook_depth_to_estimate_fill() {
        let price = estimate_market_fill_price(&[(100.0, 1.0), (101.0, 2.0)], 1.5).unwrap();
        assert!(price > 100.0 && price < 101.0);
    }
}

pub fn estimate_market_fill_price(levels: &[(f64, f64)], quantity: f64) -> Option<f64> {
    let mut remaining = quantity;
    let mut cost = 0.0;

    for (price, size) in levels {
        let taken = remaining.min(*size);
        cost += taken * price;
        remaining -= taken;

        if remaining <= 0.0 {
            return Some(cost / quantity);
        }
    }

    None
}
