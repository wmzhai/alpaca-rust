//! Historical IV window statistics and term-curve interpolation.
//!
//! The percentile is an empirical rank: count of samples strictly below the
//! reference, divided by the sample count. It is not IV rank and not the IV
//! value that sits at a requested percentile.

use alpaca_time::range;

pub const IV_LOOKBACK_DAYS: i32 = 365;
pub const YEAR_DAYS: f64 = 365.0;

#[derive(Debug, Clone, PartialEq)]
pub struct IvWindowMetrics {
    pub current_iv: f64,
    pub iv_percentile: f64,
    pub iv_rank: f64,
    pub iv_max: f64,
    pub iv_min: f64,
    pub sample_size: i32,
    pub as_of_date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermIvQuality {
    Observed,
    Interpolated,
    Extrapolated,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TermIvValue {
    pub iv: Option<f64>,
    pub quality: TermIvQuality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IvWindowError {
    Empty,
    BadWindow,
}

impl std::fmt::Display for IvWindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(formatter, "窗口内没有 IV 样本"),
            Self::BadWindow => write!(formatter, "IV 窗口日期无效"),
        }
    }
}

/// `points` are `(YYYY-MM-DD, sigma)`. Samples after `window_end` are excluded.
/// The cutoff is `window_end` minus `lookback_days` calendar days, inclusive.
pub fn iv_window_metrics(
    points: &[(&str, f64)],
    window_end: &str,
    lookback_days: i32,
) -> Result<IvWindowMetrics, IvWindowError> {
    if lookback_days < 0 || window_end.len() != 10 {
        return Err(IvWindowError::BadWindow);
    }
    let cutoff = range::add_days(window_end, -lookback_days).map_err(|_| IvWindowError::BadWindow)?;
    let mut filtered = points
        .iter()
        .filter(|(date, _)| date.len() == 10 && *date >= cutoff.as_str() && *date <= window_end)
        .copied()
        .collect::<Vec<_>>();
    filtered.sort_by(|left, right| left.0.cmp(right.0));
    if filtered.is_empty() {
        return Err(IvWindowError::Empty);
    }
    let ivs = filtered.iter().map(|(_, iv)| *iv).collect::<Vec<_>>();
    let current_iv = filtered.last().map(|(_, iv)| *iv).unwrap_or(0.0);
    let as_of_date = filtered.last().map(|(date, _)| (*date).to_string()).unwrap_or_default();
    let iv_max = ivs.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0);
    let iv_min = ivs.iter().copied().fold(f64::INFINITY, f64::min).max(0.0);
    let below = ivs.iter().filter(|iv| **iv < current_iv).count();
    Ok(IvWindowMetrics {
        current_iv,
        iv_percentile: below as f64 / ivs.len() as f64,
        iv_rank: if iv_max > iv_min {
            (current_iv - iv_min) / (iv_max - iv_min)
        } else {
            0.5
        },
        iv_max,
        iv_min,
        sample_size: ivs.len() as i32,
        as_of_date,
    })
}

/// Live reference against an already-truncated historical sample. `None` when
/// the sample or the reference cannot support a rank.
pub fn empirical_percentile(samples: &[f64], reference: f64) -> Option<f64> {
    if samples.is_empty() || !reference.is_finite() {
        return None;
    }
    if samples.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let below = samples.iter().filter(|value| **value < reference).count();
    Some(below as f64 / samples.len() as f64)
}

pub fn round_iv(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

/// Same numeric rules as the option-service term curve. Exact nodes are
/// observed. Interior gaps are interpolated. Outside the node span, or a flat
/// copy of the nearest node, is extrapolated and must not drive a new risk
/// suggestion. `days < 7` without an exact node is unavailable.
pub fn interpolate_term_iv(points: &[(i32, f64)], days: i32) -> TermIvValue {
    if points.is_empty() {
        return TermIvValue { iv: None, quality: TermIvQuality::Unavailable };
    }
    let mut nodes = points.to_vec();
    nodes.sort_by_key(|point| point.0);
    nodes.dedup_by_key(|point| point.0);
    if let Some((_, iv)) = nodes.iter().find(|point| point.0 == days) {
        return TermIvValue {
            iv: Some(round_iv(*iv)),
            quality: TermIvQuality::Observed,
        };
    }
    if days < 7 {
        return TermIvValue { iv: None, quality: TermIvQuality::Unavailable };
    }
    let Some(first) = nodes.first().copied() else {
        return TermIvValue { iv: None, quality: TermIvQuality::Unavailable };
    };
    let Some(last) = nodes.last().copied() else {
        return TermIvValue { iv: None, quality: TermIvQuality::Unavailable };
    };
    if days < first.0 {
        return TermIvValue {
            iv: Some(round_iv(first.1)),
            quality: TermIvQuality::Extrapolated,
        };
    }
    if days > last.0 {
        let iv = if nodes.len() >= 2 {
            let previous = nodes[nodes.len() - 2];
            let slope = (last.1 - previous.1) / ((last.0 - previous.0) as f64);
            round_iv((last.1 + slope * ((days - last.0) as f64)).max(0.0))
        } else {
            round_iv(last.1)
        };
        return TermIvValue { iv: Some(iv), quality: TermIvQuality::Extrapolated };
    }
    for window in nodes.windows(2) {
        let (left_days, left_iv) = window[0];
        let (right_days, right_iv) = window[1];
        if days > left_days && days < right_days {
            let ratio = (days - left_days) as f64 / (right_days - left_days) as f64;
            return TermIvValue {
                iv: Some(round_iv(left_iv + ratio * (right_iv - left_iv))),
                quality: TermIvQuality::Interpolated,
            };
        }
    }
    TermIvValue { iv: None, quality: TermIvQuality::Unavailable }
}

pub fn term_ratio(sigma_short: f64, sigma_long: f64) -> Option<f64> {
    if sigma_short.is_finite() && sigma_long.is_finite() && sigma_short > 0.0 && sigma_long > 0.0 {
        Some(sigma_short / sigma_long)
    } else {
        None
    }
}

pub fn expected_move(spot: f64, sigma: f64, horizon_days: f64) -> Option<f64> {
    if spot.is_finite() && sigma.is_finite() && horizon_days.is_finite()
        && spot > 0.0 && sigma > 0.0 && horizon_days > 0.0
    {
        Some(spot * sigma * (horizon_days / YEAR_DAYS).sqrt())
    } else {
        None
    }
}

pub fn sigma_30_move_scale(sigma_30: f64) -> Option<f64> {
    if sigma_30.is_finite() && sigma_30 > 0.0 {
        Some(sigma_30 * (30.0 / YEAR_DAYS).sqrt())
    } else {
        None
    }
}

pub fn iv_momentum(sigma: f64, ema: f64) -> Option<f64> {
    if sigma.is_finite() && ema.is_finite() && ema > 0.0 {
        Some((sigma - ema) / ema)
    } else {
        None
    }
}

pub fn price_distance_atr(spot: f64, ema: f64, atr: f64) -> Option<f64> {
    if spot.is_finite() && ema.is_finite() && atr.is_finite() && atr > 0.0 {
        Some((spot - ema) / atr)
    } else {
        None
    }
}

pub fn drawdown_from_high(high: f64, spot: f64) -> Option<f64> {
    if high.is_finite() && spot.is_finite() && high > 0.0 {
        Some((high - spot) / high)
    } else {
        None
    }
}

/// SMA 起算后按 `2/(period+1)` 递推，返回最后一点的 EMA。
pub fn ema_last(values: &[f64], period: usize) -> Option<f64> {
    if period == 0 || values.len() < period || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut ema = values[..period].iter().sum::<f64>() / period as f64;
    let k = 2.0 / (period as f64 + 1.0);
    for value in &values[period..] {
        ema = value * k + ema * (1.0 - k);
    }
    ema.is_finite().then_some(ema)
}

/// Wilder ATR。`highs/lows/closes` 按时间从早到晚，至少 `period + 1` 根。
pub fn wilder_atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64> {
    if period == 0 || highs.len() != lows.len() || highs.len() != closes.len() || highs.len() < period + 1 {
        return None;
    }
    let mut trs = Vec::with_capacity(highs.len() - 1);
    for index in 1..highs.len() {
        if ![highs[index], lows[index], closes[index], closes[index - 1]].iter().all(|value| value.is_finite()) {
            return None;
        }
        let range = (highs[index] - lows[index])
            .max((highs[index] - closes[index - 1]).abs())
            .max((lows[index] - closes[index - 1]).abs());
        trs.push(range);
    }
    if trs.len() < period {
        return None;
    }
    let mut atr = trs[..period].iter().sum::<f64>() / period as f64;
    for range in &trs[period..] {
        atr = (atr * (period as f64 - 1.0) + range) / period as f64;
    }
    (atr.is_finite() && atr > 0.0).then_some(atr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_is_strict_and_rank_is_separate() {
        let points = [
            ("2026-01-01", 0.20),
            ("2026-01-02", 0.30),
            ("2026-01-03", 0.30),
            ("2026-01-04", 0.40),
        ];
        let metrics = iv_window_metrics(&points, "2026-01-04", 365).expect("window");
        assert_eq!(metrics.current_iv, 0.40);
        assert_eq!(metrics.sample_size, 4);
        assert_eq!(metrics.iv_percentile, 0.75);
        assert!((metrics.iv_rank - 1.0).abs() < 1e-12);
        let live = empirical_percentile(&[0.20, 0.30, 0.30], 0.30).unwrap();
        assert!((live - (1.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn replay_drops_samples_after_the_window() {
        let points = [("2026-01-01", 0.20), ("2026-02-01", 0.50)];
        let metrics = iv_window_metrics(&points, "2026-01-15", 365).expect("truncated");
        assert_eq!(metrics.sample_size, 1);
        assert_eq!(metrics.current_iv, 0.20);
        assert_eq!(metrics.iv_percentile, 0.0);
    }

    #[test]
    fn curve_marks_extrapolation() {
        let points = [(30, 0.20), (90, 0.30)];
        assert_eq!(interpolate_term_iv(&points, 30).quality, TermIvQuality::Observed);
        assert_eq!(interpolate_term_iv(&points, 60).quality, TermIvQuality::Interpolated);
        assert_eq!(interpolate_term_iv(&points, 120).quality, TermIvQuality::Extrapolated);
        assert_eq!(interpolate_term_iv(&points, 3).quality, TermIvQuality::Unavailable);
        let move_30 = expected_move(100.0, 0.20, 30.0).unwrap();
        let scaled = 100.0 * sigma_30_move_scale(0.20).unwrap();
        assert!((move_30 - scaled).abs() < 1e-12);
    }
}
