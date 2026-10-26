//! How a signal's times spread: the middle, the ninety-fifth and
//! ninety-ninth percentiles and the worst, which the signals timed one by
//! one (keys, saves, calls) share.

use std::time::Duration;

/// A run's times, summed.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Spread {
    pub count: usize,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub worst_ms: f64,
}

impl Spread {
    /// Sums `times`, each one key's echo, one save or one call's round trip.
    pub fn of(times: &[Duration]) -> Spread {
        let mut ms: Vec<f64> = times.iter().map(|time| time.as_secs_f64() * 1e3).collect();
        ms.sort_by(f64::total_cmp);
        Spread {
            count: ms.len(),
            p50_ms: rank(&ms, 50),
            p95_ms: rank(&ms, 95),
            p99_ms: rank(&ms, 99),
            worst_ms: ms.last().copied().unwrap_or(0.0),
        }
    }
}

/// The nearest-rank `percent`ile of `sorted`, or nothing when it's empty.
fn rank(sorted: &[f64], percent: usize) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let at = (sorted.len() * percent).div_ceil(100).max(1) - 1;
    sorted[at]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(values: &[u64]) -> Vec<Duration> {
        values.iter().map(|&v| Duration::from_millis(v)).collect()
    }

    #[test]
    fn nothing_timed_sums_to_nothing() {
        assert_eq!(Spread::of(&[]), Spread::default());
    }

    #[test]
    fn the_middle_the_ninety_fifth_the_ninety_ninth_and_the_worst_are_read_from_the_times() {
        // A hundred times at 1 ms to 100 ms, in no order.
        let mut times: Vec<u64> = (1..=100).collect();
        times.reverse();
        let spread = Spread::of(&ms(&times));
        assert_eq!(spread.count, 100);
        assert!((spread.p50_ms - 50.0).abs() < 1e-9, "{spread:?}");
        assert!((spread.p95_ms - 95.0).abs() < 1e-9, "{spread:?}");
        assert!((spread.p99_ms - 99.0).abs() < 1e-9, "{spread:?}");
        assert!((spread.worst_ms - 100.0).abs() < 1e-9, "{spread:?}");
    }

    #[test]
    fn one_slow_time_in_a_few_is_the_ninety_ninth() {
        let spread = Spread::of(&ms(&[1, 1, 1, 40]));
        assert!((spread.p50_ms - 1.0).abs() < 1e-9, "{spread:?}");
        assert!((spread.p99_ms - 40.0).abs() < 1e-9, "{spread:?}");
    }
}
