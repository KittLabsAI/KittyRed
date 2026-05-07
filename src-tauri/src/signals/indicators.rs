use crate::models::OhlcvBar;

pub fn sma(bars: &[OhlcvBar], period: usize) -> Vec<f64> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut result = Vec::with_capacity(closes.len());
    for i in 0..closes.len() {
        if i + 1 < period {
            result.push(f64::NAN);
        } else {
            let sum: f64 = closes[i + 1 - period..=i].iter().sum();
            result.push(sum / period as f64);
        }
    }
    result
}

pub fn ema(bars: &[OhlcvBar], period: usize) -> Vec<f64> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut result = Vec::with_capacity(closes.len());
    let multiplier = 2.0 / (period as f64 + 1.0);
    for i in 0..closes.len() {
        if i == 0 {
            result.push(closes[0]);
        } else if i + 1 < period {
            result.push(f64::NAN);
        } else {
            let prev = result[i - 1];
            result.push((closes[i] - prev) * multiplier + prev);
        }
    }
    result
}

pub fn rsi(bars: &[OhlcvBar], period: usize) -> Vec<f64> {
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut result = Vec::with_capacity(closes.len());
    let mut gains = Vec::with_capacity(closes.len());
    let mut losses = Vec::with_capacity(closes.len());

    for i in 0..closes.len() {
        if i == 0 {
            gains.push(0.0);
            losses.push(0.0);
            result.push(f64::NAN);
        } else {
            let delta = closes[i] - closes[i - 1];
            gains.push(if delta > 0.0 { delta } else { 0.0 });
            losses.push(if delta < 0.0 { -delta } else { 0.0 });
            if i < period {
                result.push(f64::NAN);
            } else {
                let avg_gain: f64 = gains[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
                let avg_loss: f64 = losses[i + 1 - period..=i].iter().sum::<f64>() / period as f64;
                if avg_loss == 0.0 {
                    result.push(100.0);
                } else {
                    let rs = avg_gain / avg_loss;
                    result.push(100.0 - 100.0 / (1.0 + rs));
                }
            }
        }
    }
    result
}

pub fn macd(bars: &[OhlcvBar]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let ema12 = ema(bars, 12);
    let ema26 = ema(bars, 26);
    let mut macd_line = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if ema12[i].is_nan() || ema26[i].is_nan() {
            macd_line.push(f64::NAN);
        } else {
            macd_line.push(ema12[i] - ema26[i]);
        }
    }
    // Signal line = 9-period EMA of macd_line
    let closes_for_signal: Vec<OhlcvBar> = macd_line
        .iter()
        .map(|&v| OhlcvBar {
            open_time: String::new(),
            open: v,
            high: v,
            low: v,
            close: v,
            volume: 0.0,
            turnover: None,
        })
        .collect();
    let signal_line = ema(&closes_for_signal, 9);
    let mut histogram = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if macd_line[i].is_nan() || signal_line[i].is_nan() {
            histogram.push(f64::NAN);
        } else {
            histogram.push(macd_line[i] - signal_line[i]);
        }
    }
    (macd_line, signal_line, histogram)
}

pub fn bollinger_bands(
    bars: &[OhlcvBar],
    period: usize,
    std_dev: f64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let middle = sma(bars, period);
    let closes: Vec<f64> = bars.iter().map(|b| b.close).collect();
    let mut upper = Vec::with_capacity(bars.len());
    let mut lower = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if i + 1 < period || middle[i].is_nan() {
            upper.push(f64::NAN);
            lower.push(f64::NAN);
        } else {
            let slice = &closes[i + 1 - period..=i];
            let mean = slice.iter().sum::<f64>() / period as f64;
            let variance = slice.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / period as f64;
            let std = variance.sqrt();
            upper.push(middle[i] + std_dev * std);
            lower.push(middle[i] - std_dev * std);
        }
    }
    (upper, middle, lower)
}

pub fn avg_volume(bars: &[OhlcvBar], period: usize) -> Vec<f64> {
    let mut result = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if i + 1 < period {
            result.push(f64::NAN);
        } else {
            let sum: f64 = bars[i + 1 - period..=i].iter().map(|b| b.volume).sum();
            result.push(sum / period as f64);
        }
    }
    result
}

pub fn atr(bars: &[OhlcvBar], period: usize) -> Vec<f64> {
    let mut true_ranges = Vec::with_capacity(bars.len());
    for i in 0..bars.len() {
        if i == 0 {
            true_ranges.push(bars[i].high - bars[i].low);
        } else {
            let tr1 = bars[i].high - bars[i].low;
            let tr2 = (bars[i].high - bars[i - 1].close).abs();
            let tr3 = (bars[i].low - bars[i - 1].close).abs();
            true_ranges.push(tr1.max(tr2).max(tr3));
        }
    }
    let mut result = Vec::with_capacity(bars.len());
    for i in 0..true_ranges.len() {
        if i + 1 < period {
            result.push(f64::NAN);
        } else {
            let sum: f64 = true_ranges[i + 1 - period..=i].iter().sum();
            result.push(sum / period as f64);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bars(closes: &[f64]) -> Vec<OhlcvBar> {
        closes
            .iter()
            .map(|&c| OhlcvBar {
                open_time: String::new(),
                open: c,
                high: c,
                low: c,
                close: c,
                volume: 100.0,
                turnover: None,
            })
            .collect()
    }

    #[test]
    fn sma_computes_correctly() {
        let bars = make_bars(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = sma(&bars, 3);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!((result[2] - 2.0).abs() < 0.001);
        assert!((result[3] - 3.0).abs() < 0.001);
        assert!((result[4] - 4.0).abs() < 0.001);
    }

    #[test]
    fn rsi_extremes() {
        let mut closes = vec![100.0];
        for _ in 0..14 {
            closes.push(closes.last().unwrap() + 1.0);
        }
        let bars = make_bars(&closes);
        let result = rsi(&bars, 14);
        let last = result.last().unwrap();
        assert!(*last > 70.0);
    }

    #[test]
    fn bollinger_bands_produces_upper_lower() {
        let bars = make_bars(&[10.0, 11.0, 12.0, 11.0, 10.0]);
        let (upper, _middle, lower) = bollinger_bands(&bars, 3, 2.0);
        let last_upper = upper.last().unwrap();
        let last_lower = lower.last().unwrap();
        assert!(!last_upper.is_nan());
        assert!(!last_lower.is_nan());
        assert!(last_upper > last_lower);
    }
}
