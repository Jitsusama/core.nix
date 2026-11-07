//! Medians and ninetieth percentiles, as the card states them.

use std::fmt;

/// How a set of durations spread: how many, the median, the p90 and the
/// worst.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spread {
    pub n: usize,
    pub median: f64,
    pub p90: f64,
    pub max: f64,
}

/// The value at quantile `p` of `xs`, by nearest rank on the sorted values,
/// or nothing for none.
#[must_use]
pub fn quantile(xs: &[f64], p: f64) -> Option<f64> {
    if xs.is_empty() {
        return None;
    }
    let mut sorted = xs.to_vec();
    sorted.sort_by(f64::total_cmp);
    let last = sorted.len() - 1;
    // Rounded to the nearest rank, which a card reads the same way twice.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "a rank within a list far shorter than 2^52"
    )]
    let rank = ((p * last as f64).round() as usize).min(last);
    Some(sorted[rank])
}

/// The spread of `xs`, or nothing for none.
#[must_use]
pub fn spread(xs: &[f64]) -> Option<Spread> {
    Some(Spread {
        n: xs.len(),
        median: quantile(xs, 0.5)?,
        p90: quantile(xs, 0.9)?,
        max: xs.iter().copied().fold(f64::MIN, f64::max),
    })
}

impl fmt::Display for Spread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "median {:.1}, p90 {:.1}, max {:.1} min (n {})",
            self.median, self.p90, self.max, self.n
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_median_of_an_odd_count_is_its_middle() {
        assert_eq!(quantile(&[5.0, 1.0, 3.0], 0.5), Some(3.0));
    }

    #[test]
    fn the_p90_of_ten_is_the_ninth_nearest_rank() {
        let xs: Vec<f64> = (1..=10).map(f64::from).collect();
        assert_eq!(quantile(&xs, 0.9), Some(9.0));
    }

    #[test]
    fn nothing_has_no_spread() {
        assert_eq!(spread(&[]), None);
    }

    #[test]
    fn a_spread_reads_as_the_card_says_it() {
        let s = spread(&[1.0, 2.0, 30.0]).unwrap();
        assert_eq!(s.to_string(), "median 2.0, p90 30.0, max 30.0 min (n 3)");
    }
}
