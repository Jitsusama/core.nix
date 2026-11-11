//! A figure said as the launcher says it, so a daemon's measured rows
//! read as the shell's own did: three significant digits, zeros kept,
//! grouped and pointed as the account's number settings say, with the
//! real minus (VKCF3Z, 2026-11-07 and 2026-11-08). The settings come to a
//! daemon by store path, `{ "group", "point" }`, read once. Its cases are
//! `numbers.json`, each with the group and point it's said in, empty for
//! the comma and the full stop; they're Numbers.js's cases, which stay
//! beside it until the last place that words figures in QML moves to a
//! daemon. A figure that reads
//! the clock is never said here: it travels as a [`crate::time`].

use serde_json::Value;

/// The digits a measured figure is said to, and the places after the
/// first that an exponent's form writes them in.
const SIGNIFICANT: i32 = 3;
const AFTER_FIRST: usize = 2;
/// The real minus a measure under nothing takes.
const MINUS: char = '\u{2212}';

/// How the account groups and points numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct Numbers {
    group: String,
    point: String,
}

impl Default for Numbers {
    /// The comma and the full stop, as an account that set none says.
    fn default() -> Numbers {
        Numbers::new("", "")
    }
}

impl Numbers {
    /// The settings as given, an empty group or point taking the comma
    /// and the full stop.
    pub fn new(group: &str, point: &str) -> Numbers {
        let or = |said: &str, usual: &str| if said.is_empty() { usual } else { said }.to_owned();
        Numbers {
            group: or(group, ","),
            point: or(point, "."),
        }
    }

    /// The settings as the store path holds them, `{ "group", "point" }`,
    /// either left out taking its usual mark.
    pub fn read(settings: &Value) -> Numbers {
        let mark = |name: &str| settings[name].as_str().unwrap_or("");
        Numbers::new(mark("group"), mark("point"))
    }

    /// A number grouped in threes, with at most `places` places, trailing
    /// zeros dropped, and the plain hyphen a typed number takes.
    pub fn number(&self, value: f64, places: usize) -> String {
        let sign = if value < 0.0 { "-" } else { "" };
        let exact = format!("{:.*}", places, value.abs());
        let fixed = if exact.contains('.') {
            exact.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            exact
        };
        let (whole, fraction) = fixed.split_once('.').unwrap_or((&fixed, ""));
        let mut grouped = String::new();
        for (at, digit) in whole.chars().enumerate() {
            if at > 0 && (whole.len() - at).is_multiple_of(3) {
                grouped.push_str(&self.group);
            }
            grouped.push(digit);
        }
        let pointed = if fraction.is_empty() {
            String::new()
        } else {
            format!("{}{fraction}", self.point)
        };
        format!("{sign}{grouped}{pointed}")
    }

    /// A measured figure to three significant digits, its zeros kept so
    /// each says as much as the next: 28.47 as "28.5", 5.2 as "5.20", 0
    /// as "0", -3.2 as "−3.20", 1234 as "1,230". A figure that's a name,
    /// as a version or a set point, keeps its own digits and never comes
    /// here.
    pub fn significant(&self, value: f64) -> String {
        let rounded: f64 = format!("{value:.AFTER_FIRST$e}").parse().unwrap_or(value);
        if rounded == 0.0 || !rounded.is_finite() {
            return "0".to_owned();
        }
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a finite figure's power of ten is within an i32"
        )]
        let magnitude = rounded.abs().log10().floor() as i32;
        let places = usize::try_from(SIGNIFICANT - 1 - magnitude).unwrap_or(0);
        let said = self
            .number(rounded, places)
            .replacen('-', &MINUS.to_string(), 1);
        if places == 0 {
            return said;
        }
        let (whole, fraction) = said.split_once(&self.point).unwrap_or((&said, ""));
        format!("{whole}{}{fraction:0<places$}", self.point)
    }

    /// A figure said to `places` places, its zeros kept, grouped, with the
    /// real minus: 0.06 to 3 as "0.060", as a chromaticity's coordinate
    /// is said.
    #[must_use]
    pub fn fixed(&self, value: f64, places: usize) -> String {
        let said = self
            .number(value, places)
            .replacen('-', &MINUS.to_string(), 1);
        if places == 0 {
            return said;
        }
        let (whole, fraction) = said.split_once(&self.point).unwrap_or((&said, ""));
        format!("{whole}{}{fraction:0<places$}", self.point)
    }

    /// A reading whole by nature, as a signal's dBm is, rounded and
    /// grouped with the real minus: -14.4 as "−14".
    pub fn whole(&self, value: f64) -> String {
        self.number(value.round(), 0)
            .replacen('-', &MINUS.to_string(), 1)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The cases the launcher's figures are said by, kept once.
    fn cases(kind: &str) -> Vec<Value> {
        let all: Value =
            serde_json::from_str(include_str!("../numbers.json")).expect("numbers.json reads");
        all[kind].as_array().expect("a list of cases").clone()
    }

    fn numbers_of(case: &Value) -> Numbers {
        Numbers::new(
            case["group"].as_str().unwrap_or(""),
            case["point"].as_str().unwrap_or(""),
        )
    }

    fn value_of(case: &Value) -> f64 {
        case["value"].as_f64().expect("a case's value is a number")
    }

    #[test]
    fn each_kind_of_figure_is_said_as_its_cases_say() {
        for case in cases("significant") {
            let said = numbers_of(&case).significant(value_of(&case));
            assert_eq!(said, case["said"], "three digits of {case}");
        }
        for case in cases("number") {
            let places = usize::try_from(case["places"].as_u64().unwrap_or(0)).unwrap_or(0);
            let said = numbers_of(&case).number(value_of(&case), places);
            assert_eq!(said, case["said"], "a number of {case}");
        }
        for case in cases("fixed") {
            let places = usize::try_from(case["places"].as_u64().unwrap_or(0)).unwrap_or(0);
            let said = numbers_of(&case).fixed(value_of(&case), places);
            assert_eq!(said, case["said"], "a fixed figure of {case}");
        }
        for case in cases("whole") {
            let said = numbers_of(&case).whole(value_of(&case));
            assert_eq!(said, case["said"], "a whole reading of {case}");
        }
    }

    #[test]
    fn settings_are_read_from_what_the_store_path_holds() {
        let set = Numbers::read(&json!({ "group": ".", "point": "," }));
        assert_eq!(set, Numbers::new(".", ","));
        assert_eq!(Numbers::read(&json!({})), Numbers::default());
        assert_eq!(Numbers::default().number(1234.5, 1), "1,234.5");
    }
}
