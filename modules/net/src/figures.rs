//! How net sets a rate in words: three significant digits, in the unit that
//! reads it so, as the design asks ("370 Mb/s", "1.20 Gb/s"). eero's
//! views/figures.rs sets its rates the same way, so the two read alike.

/// How many significant digits a measure keeps.
const DIGITS: u8 = 3;

/// Enough digits to write any f64 exactly: none has more than 767.
const EXACT: usize = 800;

/// A measure rounded to three significant digits from its exact decimal
/// value, a half going up, as the page's `toPrecision` rounds. Scaling
/// by a power of ten first would round 10.35, which is a hair under it,
/// up to 10.4, and Rust's own formatter takes a tie to the even digit.
fn rounded(value: f64) -> f64 {
    if !value.is_normal() {
        return value;
    }
    let exact = format!("{:.EXACT$e}", value.abs());
    let Some((mantissa, power)) = exact.split_once('e') else {
        return value;
    };
    let Ok(power) = power.parse::<i32>() else {
        return value;
    };
    let mut digits = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .map(|digit| u32::from(digit - b'0'));
    let kept = digits
        .by_ref()
        .take(usize::from(DIGITS))
        .fold(0, |n, digit| n * 10 + digit);
    let up = digits.next().is_some_and(|digit| digit >= 5);
    let near = format!("{}e{}", kept + u32::from(up), power - i32::from(DIGITS) + 1);
    near.parse::<f64>()
        .map_or(value, |near| near.copysign(value))
}

/// The power of ten a measure's first digit stands at, read from its
/// exact decimal rather than a logarithm's rounding: 2 for 370.
fn magnitude(value: f64) -> i32 {
    format!("{value:e}")
        .split_once('e')
        .and_then(|(_, power)| power.parse().ok())
        .unwrap_or(0)
}

/// A measure already rounded, in three significant digits, its trailing
/// zeros kept, as 370, 11.9, 2.40 and 0.0240.
fn significant(near: f64) -> String {
    let places = usize::try_from(i32::from(DIGITS) - 1 - magnitude(near)).unwrap_or(0);
    format!("{:.places$}", near.abs())
}

/// A rate given in Mb/s, in the largest unit it is at least one of once
/// rounded, so 999.96 Mb/s reads 1.00 Gb/s rather than 1000 Mb/s: 370 Mb/s,
/// 1.20 Gb/s, 145 kb/s, 98.2 b/s, and 0 b/s for none. Under a bit a
/// second is none too, so no rate needs more than two places.
pub fn rate(mbps: f64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e3, "Gb/s"), (1.0, "Mb/s"), (1e-3, "kb/s"), (1e-6, "b/s")];
    if !mbps.is_finite() || rounded(mbps / 1e-6) < 1.0 {
        return "0 b/s".into();
    }
    let (scale, unit) = UNITS
        .iter()
        .find(|(scale, _)| rounded(mbps / scale) >= 1.0)
        .copied()
        .unwrap_or(UNITS[UNITS.len() - 1]);
    format!("{} {unit}", significant(rounded(mbps / scale)))
}

#[cfg(test)]
#[path = "figures_tests.rs"]
mod tests;
