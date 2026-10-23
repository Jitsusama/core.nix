//! The statistics the detectors share: robust location and scale, the
//! normal's tail, Benjamini-Hochberg's false discovery control, and CRPS,
//! the proper score a forecast is judged by.

/// A count of things held in memory as a float, for averages, ranks and
/// shares.
#[expect(
    clippy::cast_precision_loss,
    reason = "a count of values held in memory is far below 2^52, so f64 holds it exactly"
)]
pub fn count(n: usize) -> f64 {
    n as f64
}

/// A count or index of things held in memory as an `i64`, for arithmetic on
/// day and hour numbers. Nothing in memory holds more than `isize::MAX`
/// things, so it always fits and the saturation is never reached.
pub fn signed(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The q-quantile of sorted values, interpolated; none gives nothing.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the position lies in [0, len - 1], so its floor and ceiling are indices"
)]
pub fn quantile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let at = q.clamp(0.0, 1.0) * count(sorted.len() - 1);
    let low = at.floor() as usize;
    let high = at.ceil() as usize;
    sorted[low] + (sorted[high] - sorted[low]) * (at - count(low))
}

pub fn quantile(values: &[f64], q: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    quantile_sorted(&sorted, q)
}

pub fn median(values: &[f64]) -> f64 {
    quantile(values, 0.5)
}

/// The median absolute deviation, scaled to a normal's standard deviation.
pub fn mad(values: &[f64]) -> f64 {
    let centre = median(values);
    let deviations: Vec<f64> = values.iter().map(|value| (value - centre).abs()).collect();
    1.4826 * median(&deviations)
}

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / count(values.len())
    }
}

/// The chance a standard normal exceeds z, by Abramowitz and Stegun's
/// 26.2.17, good to 7.5e-8.
pub fn upper_tail(z: f64) -> f64 {
    if z < 0.0 {
        return 1.0 - upper_tail(-z);
    }
    let t = 1.0 / (1.0 + 0.231_641_9 * z);
    let poly = t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    (-z * z / 2.0).exp() / (std::f64::consts::TAU).sqrt() * poly
}

/// Which of many p-values hold at false discovery rate q, by Benjamini and
/// Hochberg: so testing every session at once doesn't flag more of them
/// just for being many.
pub fn discoveries(p: &[f64], q: f64) -> Vec<bool> {
    let mut order: Vec<usize> = (0..p.len()).collect();
    order.sort_by(|&one, &other| p[one].total_cmp(&p[other]));
    let m = count(p.len());
    let mut cut = None;
    for (rank, &index) in order.iter().enumerate() {
        if p[index] <= count(rank + 1) / m * q {
            cut = Some(rank);
        }
    }
    let mut out = vec![false; p.len()];
    if let Some(cut) = cut {
        for &index in &order[..=cut] {
            out[index] = true;
        }
    }
    out
}

/// The continuous ranked probability score of a forecast, as samples,
/// against what happened: in the outcome's own units, lower is better.
pub fn crps(samples: &[f64], outcome: f64) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = count(sorted.len());
    let spread: f64 = sorted
        .iter()
        .map(|value| (value - outcome).abs())
        .sum::<f64>()
        / n;
    let pairs: f64 = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (2.0 * (count(index) + 1.0) - n - 1.0) * value)
        .sum::<f64>()
        / (n * n);
    spread - pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "the median of an odd count is one of the values, untouched"
    )]
    fn a_median_and_its_scale_ignore_one_wild_value() {
        let values = [10.0, 11.0, 9.0, 10.0, 1000.0];
        assert_eq!(median(&values), 10.0);
        assert!((mad(&values) - 1.4826).abs() < 1e-9);
    }

    #[test]
    fn the_normal_tail_is_right() {
        assert!((upper_tail(1.959_964) - 0.025).abs() < 1e-6);
        assert!((upper_tail(0.0) - 0.5).abs() < 1e-7);
        assert!((upper_tail(-1.0) - 0.841_344_7).abs() < 1e-6);
    }

    #[test]
    fn benjamini_hochberg_keeps_only_what_survives_many_tests() {
        let p = [
            0.001, 0.008, 0.039, 0.041, 0.042, 0.06, 0.074, 0.205, 0.212, 0.216,
        ];
        let found = discoveries(&p, 0.05);
        assert_eq!(found.iter().filter(|&&found| found).count(), 2);
        assert!(found[0] && found[1]);
    }

    #[test]
    fn a_sharp_forecast_on_the_mark_scores_nearly_nothing() {
        assert!(crps(&[5.0; 100], 5.0).abs() < 1e-12);
        // A point forecast's score is its error.
        assert!((crps(&[5.0; 100], 7.0) - 2.0).abs() < 1e-12);
    }
}
