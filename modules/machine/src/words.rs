//! The launcher's words and number settings as the daemon is handed them
//! (launcher/words.json and `group` and `point` by store path), and the
//! one rule a measure is said by: three significant digits, zeros kept,
//! grouped and pointed as Joel's settings say, with the real minus. Its
//! wording of numbers is Numbers.js's, until the `ready-rows` crate
//! holds both (Y2YBTS, 2026-11-10), when this takes the crate's.

use serde_json::Value;

/// The digits a measured figure is said to (VKCF3Z, 2026-11-07), and the
/// places after the first that an exponent's form writes them in.
const SIGNIFICANT: i32 = 3;
const AFTER_FIRST: usize = 2;
/// The real minus a measure under nothing takes (VKCF3Z, 2026-11-08).
const MINUS: char = '\u{2212}';

/// What a row is said in: the vocabulary's things by id, and how
/// numbers are grouped and pointed.
#[derive(Clone, Debug, Default)]
pub struct Words {
    things: serde_json::Map<String, Value>,
    group: String,
    point: String,
}

impl Words {
    /// The launcher's own words for every thing the rows' cases say,
    /// copied from words.json into tests/words.json, which the check
    /// `the-daemons-test-the-launchers-words` keeps the same as it, so a
    /// case holds a row to the words Joel reads.
    #[cfg(test)]
    pub fn launchers(group: &str, point: &str) -> Words {
        Words::new(&Words::launchers_vocabulary(), group, point)
    }

    /// The launcher's words as they're written, for a case that changes
    /// one to see what follows.
    #[cfg(test)]
    #[must_use]
    pub fn launchers_vocabulary() -> Value {
        serde_json::from_str(include_str!("../tests/words.json"))
            .expect("the launcher's words parse")
    }

    /// The vocabulary's things, each by its id, and the number settings;
    /// an empty group or point takes the comma and the full stop.
    pub fn new(vocabulary: &Value, group: &str, point: &str) -> Words {
        let things = vocabulary["things"]
            .as_array()
            .map(|things| {
                things
                    .iter()
                    .filter_map(|thing| Some((thing["id"].as_str()?.to_owned(), thing.clone())))
                    .collect()
            })
            .unwrap_or_default();
        let or = |said: &str, usual: &str| {
            if said.is_empty() {
                usual.to_owned()
            } else {
                said.to_owned()
            }
        };
        Words {
            things,
            group: or(group, ","),
            point: or(point, "."),
        }
    }

    /// Whether the vocabulary names a thing.
    pub fn has(&self, id: &str) -> bool {
        self.things.contains_key(id)
    }

    /// A thing's name, its braces filled, or its id where it has none.
    pub fn name(&self, id: &str, fill: &[(&str, &str)]) -> String {
        let named = self
            .things
            .get(id)
            .and_then(|thing| thing["name"].as_str())
            .unwrap_or(id);
        filled(named, fill)
    }

    /// A thing's name in one of its states, as Drawing's is Charging while
    /// the battery charges, else its own name.
    pub fn name_in(&self, id: &str, state: &str, fill: &[(&str, &str)]) -> String {
        let stated = self
            .things
            .get(id)
            .and_then(|thing| thing["states"][state]["name"].as_str());
        stated.map_or_else(|| self.name(id, fill), |named| filled(named, fill))
    }

    /// A phrase a thing says, its braces filled, or "" where it says none.
    pub fn said(&self, id: &str, phrase: &str, fill: &[(&str, &str)]) -> String {
        let said = self
            .things
            .get(id)
            .and_then(|thing| thing["said"][phrase].as_str());
        said.map_or_else(String::new, |said| filled(said, fill))
    }

    /// What an act that can't be taken back asks once, its braces
    /// filled, or "" where it asks nothing.
    pub fn sure(&self, id: &str, fill: &[(&str, &str)]) -> String {
        let sure = self.things.get(id).and_then(|thing| thing["sure"].as_str());
        sure.map_or_else(String::new, |sure| filled(sure, fill))
    }

    /// The ids of a thing's parts, in the order its words list them.
    pub fn parts(&self, id: &str) -> Vec<&str> {
        self.things
            .get(id)
            .and_then(|thing| thing["parts"].as_array())
            .map(|parts| parts.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// The glyph role a thing shows, or "".
    pub fn glyph(&self, id: &str) -> String {
        let role = self
            .things
            .get(id)
            .and_then(|thing| thing["glyph"].as_str());
        role.unwrap_or_default().to_owned()
    }

    /// A number grouped in threes, with at most `places` places, trailing
    /// zeros dropped.
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
    /// as "0", -3.2 as "−3.20", 1234 as "1,230".
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
}

/// A phrase with each brace filled once, as the shell's Words.name does.
fn filled(said: &str, fill: &[(&str, &str)]) -> String {
    let mut out = said.to_owned();
    for (key, value) in fill {
        out = out.replacen(&format!("{{{key}}}"), value, 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words(group: &str, point: &str) -> Words {
        let vocabulary = json!({ "things": [
            { "id": "heat", "name": "Heat", "glyph": "heat",
                "said": { "hottest": "{degrees} at the {part}" } },
            { "id": "heat:core", "name": "Core {n}" }] });
        Words::new(&vocabulary, group, point)
    }

    #[test]
    fn a_things_words_with_their_braces_filled() {
        let said = words("", "");
        assert_eq!(said.name("heat:core", &[("n", "8")]), "Core 8");
        assert_eq!(said.name("heat:nothing", &[]), "heat:nothing");
        let hottest = said.said(
            "heat",
            "hottest",
            &[("degrees", "83.0 °C"), ("part", "processor")],
        );
        assert_eq!(hottest, "83.0 °C at the processor");
        assert_eq!(said.said("heat", "nothing", &[]), "");
        assert_eq!(
            (said.glyph("heat"), said.glyph("heat:core")),
            ("heat".to_owned(), String::new())
        );
    }

    /// Numbers.js's cases, until the crate's data file holds them once.
    #[test]
    fn three_significant_digits() {
        let said = words("", "");
        let figures =
            [0.04, 1.234, 12.34, 123.4, 0.9997, 1234.0, -3.2].map(|v| said.significant(v));
        assert_eq!(
            figures,
            [
                "0.0400",
                "1.23",
                "12.3",
                "123",
                "1.00",
                "1,230",
                "\u{2212}3.20"
            ]
        );
        let set = words(".", ",");
        assert_eq!(
            format!("{} {}", set.significant(1234.5), set.significant(5.2)),
            "1.230 5,20"
        );
        assert_eq!(said.significant(0.0), "0");
    }

    #[test]
    fn a_number_grouped_in_threes() {
        let said = words("", "");
        assert_eq!(said.number(1_234_567.891, 3), "1,234,567.891");
        assert_eq!(said.number(5385.0, 0), "5,385");
        assert_eq!(said.number(12.50, 1), "12.5");
        assert_eq!(said.number(-12.0, 0), "-12");
    }
}
