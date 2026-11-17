//! How net sets a rate in words: three significant digits, in the unit that
//! reads it so, as the design asks ("370 Mb/s", "1.20 Gb/s"). eero's
//! views/figures.rs sets its rates the same way, so the two read alike.

/// How many significant digits a measure keeps.
const DIGITS: i32 = 3;

/// A measure rounded to three significant digits.
fn rounded(value: f64) -> f64 {
    if !value.is_normal() {
        return value;
    }
    let scale = 10f64.powi(DIGITS - 1 - magnitude(value));
    (value * scale).round() / scale
}

/// The power of ten a measure's first digit stands at: 2 for 370.
#[expect(
    clippy::cast_possible_truncation,
    reason = "a figure's power of ten is small"
)]
fn magnitude(value: f64) -> i32 {
    value.abs().log10().floor() as i32
}

/// A measure in three significant digits, its trailing zeros kept, as
/// 370, 11.9, 2.40 and 0.0240.
fn significant(value: f64) -> String {
    let near = rounded(value);
    if !near.is_normal() {
        return "0".into();
    }
    let places = usize::try_from((DIGITS - 1 - magnitude(near)).max(0)).unwrap_or(0);
    format!("{:.places$}", near.abs())
}

/// A rate given in Mb/s, in the largest unit it is at least one of once
/// rounded, so 999.96 Mb/s reads 1.00 Gb/s rather than 1000 Mb/s: 370 Mb/s,
/// 1.20 Gb/s, 145 kb/s, 98.2 b/s, and 0 b/s for none.
pub fn rate(mbps: f64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e3, "Gb/s"), (1.0, "Mb/s"), (1e-3, "kb/s"), (1e-6, "b/s")];
    if !mbps.is_normal() || mbps < 0.0 {
        return "0 b/s".into();
    }
    let (scale, unit) = UNITS
        .iter()
        .find(|(scale, _)| rounded(mbps / scale) >= 1.0)
        .copied()
        .unwrap_or(UNITS[UNITS.len() - 1]);
    format!("{} {unit}", significant(mbps / scale))
}

#[cfg(test)]
#[path = "figures_tests.rs"]
mod tests;
