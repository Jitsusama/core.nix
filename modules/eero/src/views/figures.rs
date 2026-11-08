//! Figures as the views set them: each its number and, muted beside it,
//! its unit, sized to the unit people read it in, and a muted dash where
//! eero didn't say. A measure keeps three significant digits and takes the
//! next unit past 999, as docs/design.md's Set, Not Styled has every
//! surface do, so the command line reads as the launcher's place does.

use super::marks::marks;
use super::text::{Line, muted, plain};

/// What a cell holds when eero gave no figure.
pub fn unknown() -> Line {
    vec![muted("–")]
}

/// A figure and its unit.
pub fn figure(value: impl Into<String>, unit: &str) -> Line {
    if unit.is_empty() {
        return vec![plain(value)];
    }
    vec![plain(value), muted(format!(" {unit}"))]
}

/// A figure eero may not have given, set by `set` when it did.
pub fn maybe<T>(value: Option<T>, set: impl FnOnce(T) -> Line) -> Line {
    value.map_or_else(unknown, set)
}

/// A whole number, its thousands set apart by commas: 6,072,645.
pub fn count(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// A signed figure with a true minus, as a typesetter sets one: −48.
pub fn signed(value: i64) -> String {
    if value < 0 {
        format!("\u{2212}{}", value.unsigned_abs())
    } else {
        value.to_string()
    }
}

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
/// 370, 11.9, 2.40 and 0.0240, and the real minus on a negative one.
pub fn significant(value: f64) -> String {
    let near = rounded(value);
    if !near.is_normal() {
        return "0".into();
    }
    let places = usize::try_from((DIGITS - 1 - magnitude(near)).max(0)).unwrap_or(0);
    let set = format!("{:.places$}", near.abs());
    if near < 0.0 {
        format!("\u{2212}{set}")
    } else {
        set
    }
}

/// A measure in the largest of its units it is at least one of, once
/// rounded, so 999.96 Mb/s reads 1.00 Gb/s rather than 1000 Mb/s.
fn scaled(value: f64, units: &[(f64, &str)], least: (f64, &str)) -> Line {
    let (scale, unit) = units
        .iter()
        .find(|(scale, _)| rounded(value / scale) >= 1.0)
        .copied()
        .unwrap_or(least);
    figure(significant(value / scale), unit)
}

/// Bytes in the powers of ten networks count in: 167 GB, 3.68 MB.
pub fn bytes(value: u64) -> Line {
    const UNITS: [(f64, &str); 4] = [(1e12, "TB"), (1e9, "GB"), (1e6, "MB"), (1e3, "kB")];
    #[expect(clippy::cast_precision_loss, reason = "a size shown to three digits")]
    let size = value as f64;
    if rounded(size) < 1e3 {
        return figure(count(value), "B");
    }
    scaled(size, &UNITS, (1.0, "B"))
}

/// A rate given in Mb/s, as data moves or a link runs, in the unit that
/// reads it in three digits: 370 Mb/s, 1.20 Gb/s, 145 kb/s, 98.2 b/s.
pub fn rate(mbps: f64) -> Line {
    const UNITS: [(f64, &str); 3] = [(1e3, "Gb/s"), (1.0, "Mb/s"), (1e-3, "kb/s")];
    if !mbps.is_normal() {
        return figure("0", "b/s");
    }
    scaled(mbps, &UNITS, (1e-6, "b/s"))
}

/// A switch: on or off.
pub fn switch(value: Option<bool>) -> Line {
    maybe(value, |on| vec![plain(if on { "on" } else { "off" })])
}

/// A question eero answers: yes or no.
pub fn yes(value: Option<bool>) -> Line {
    maybe(value, |yes| vec![plain(if yes { "yes" } else { "no" })])
}

/// Text eero gave, or the dash.
pub fn said(value: Option<&str>) -> Line {
    maybe(value, |text| vec![plain(text)])
}

/// eero's word for a kind of thing, as words: `laptop_computer` is a
/// laptop computer.
pub fn words(kind: &str) -> String {
    kind.replace('_', " ")
}

/// eero's name for a radio, set as a unit is: 5.8GHz is 5.8 GHz.
pub fn radio(said: &str) -> String {
    said.find(|c: char| c.is_ascii_alphabetic())
        .filter(|at| *at > 0 && !said[..*at].ends_with(' '))
        .map_or_else(
            || said.to_owned(),
            |at| format!("{} {}", &said[..at], &said[at..]),
        )
}

/// Several figures on one line, a muted dot between each.
pub fn joined(parts: Vec<Line>) -> Line {
    let mut line = Vec::new();
    for (at, part) in parts.into_iter().enumerate() {
        if at > 0 {
            line.push(muted(format!(" {} ", marks().separator)));
        }
        line.extend(part);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &Line) -> String {
        line.iter().map(|run| run.text.as_str()).collect()
    }

    #[test]
    fn a_figure_is_sized_to_the_unit_people_read_it_in() {
        assert_eq!(text(&bytes(167_091_577_600)), "167 GB");
        assert_eq!(text(&bytes(3_680_059)), "3.68 MB");
        assert_eq!(text(&bytes(2_400_000)), "2.40 MB");
        assert_eq!(text(&bytes(999_960)), "1.00 MB");
        assert_eq!(text(&bytes(999)), "999 B");
        assert_eq!(text(&rate(11.452_299)), "11.5 Mb/s");
        assert_eq!(text(&rate(369.86)), "370 Mb/s");
        assert_eq!(text(&rate(48.0)), "48.0 Mb/s");
        assert_eq!(text(&rate(1_200.9)), "1.20 Gb/s");
        assert_eq!(text(&rate(999.96)), "1.00 Gb/s");
        assert_eq!(text(&rate(0.145_383_8)), "145 kb/s");
        assert_eq!(text(&rate(0.023_960_4)), "24.0 kb/s");
        assert_eq!(text(&rate(0.000_098_2)), "98.2 b/s");
        assert_eq!(text(&rate(0.0)), "0 b/s");
    }

    #[test]
    fn a_measure_keeps_three_significant_digits() {
        assert_eq!(significant(369.86), "370");
        assert_eq!(significant(11.93), "11.9");
        assert_eq!(significant(2.4), "2.40");
        assert_eq!(significant(0.024), "0.0240");
        assert_eq!(significant(1_234.0), "1230");
        assert_eq!(significant(-48.0), "\u{2212}48.0");
        assert_eq!(significant(0.0), "0");
    }

    #[test]
    fn numbers_are_set_as_a_typesetter_sets_them() {
        assert_eq!(count(6_072_645), "6,072,645");
        assert_eq!(count(735), "735");
        assert_eq!(signed(-48), "\u{2212}48");
        assert_eq!(radio("5.8GHz"), "5.8 GHz");
        assert_eq!(radio("5 GHz"), "5 GHz");
        assert_eq!(words("laptop_computer"), "laptop computer");
    }
}
