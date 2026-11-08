//! Figures as the views set them: each its number and, muted beside it,
//! its unit, sized to the unit people read it in, and a muted dash where
//! eero didn't say.

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

/// How fast data moves, in the unit that keeps it readable: 11.5 Mbit/s,
/// 145.4 kbit/s, 98 bit/s.
pub fn speed(mbps: f64) -> Line {
    if mbps >= 1.0 {
        figure(format!("{mbps:.1}"), "Mbit/s")
    } else if mbps >= 0.001 {
        figure(format!("{:.1}", mbps * 1e3), "kbit/s")
    } else {
        figure(format!("{:.0}", mbps * 1e6), "bit/s")
    }
}

/// A link's rate, always in Mbit/s to its tenth, as eero gives it.
pub fn rate(mbps: f64) -> Line {
    figure(format!("{mbps:.1}"), "Mbit/s")
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
        assert_eq!(text(&speed(11.452_299)), "11.5 Mbit/s");
        assert_eq!(text(&speed(0.145_383_8)), "145.4 kbit/s");
        assert_eq!(text(&speed(0.000_098_2)), "98 bit/s");
        assert_eq!(text(&speed(0.0)), "0 bit/s");
    }

    #[test]
    fn numbers_are_set_as_a_typesetter_sets_them() {
        assert_eq!(count(6_072_645), "6,072,645");
        assert_eq!(count(735), "735");
        assert_eq!(signed(-48), "\u{2212}48");
        assert_eq!(words("laptop_computer"), "laptop computer");
    }
}
