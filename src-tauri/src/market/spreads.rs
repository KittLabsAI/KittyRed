#[cfg(test)]
mod tests {
    use super::compute_net_spread;

    #[test]
    fn computes_fee_adjusted_spread() {
        let result = compute_net_spread(101.0, 100.0, 0.001, 0.001, 0.0005);
        assert!(result > 0.0);
    }
}

pub fn compute_net_spread(
    bid_on_sell_exchange: f64,
    ask_on_buy_exchange: f64,
    sell_fee: f64,
    buy_fee: f64,
    slippage: f64,
) -> f64 {
    let gross = bid_on_sell_exchange - ask_on_buy_exchange;
    let costs = bid_on_sell_exchange * sell_fee
        + ask_on_buy_exchange * buy_fee
        + ask_on_buy_exchange * slippage;
    gross - costs
}
