//! The asks the list parked on a ruling, such as one set aside until load
//! is spread. Its keeper marks one with `parked`, the ruling's message id,
//! and the slips measure leaves it out, since its owner owes no date while
//! the ruling stands. They are counted here by ruling, so none drops out of
//! sight. Taking the field off brings its dates back to the slips measure,
//! read as they stand, so an ask let go is given a new date with it.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Class, Measure, count, lines};
use crate::tracking::{self, Tracking};

/// The ruling `ask` is parked on, or none when it isn't parked.
#[must_use]
pub fn ruling(ask: &Value) -> Option<&str> {
    ask["parked"]
        .as_str()
        .map(str::trim)
        .filter(|r| !r.is_empty())
}

/// How many asks of the kind `which` in the asks list `text` are parked on
/// a ruling, each ask by its latest line, named by ruling, the most first.
/// One closed without landing is put down, not parked, so it isn't counted.
#[must_use]
pub fn parked(text: &str, which: Tracking) -> Measure {
    let mut latest: BTreeMap<[String; 2], Option<String>> = BTreeMap::new();
    for ask in lines(text) {
        let ours = tracking::of(&ask) == Some(which);
        let asked = [ask["at"].to_string(), ask["words"].to_string()];
        latest.insert(asked, ruling(&ask).filter(|_| ours).map(str::to_owned));
    }
    let mut by_ruling: BTreeMap<String, usize> = BTreeMap::new();
    for ruling in latest.into_values().flatten() {
        *by_ruling.entry(ruling).or_default() += 1;
    }
    let mut rulings: Vec<(String, usize)> = by_ruling.into_iter().collect();
    rulings.sort_by(|(a, n), (b, m)| m.cmp(n).then(a.cmp(b)));
    Measure {
        key: which.key("slips-parked"),
        class: Class::Slip,
        says: "asks parked on a ruling, out of the slips count while it stands".to_owned(),
        now: count(rulings.iter().map(|(_, n)| n).sum()),
        target: None,
        owner: String::new(),
        behind: rulings
            .iter()
            .map(|(ruling, n)| format!("{ruling}: {n} ask{}", if *n == 1 { "" } else { "s" }))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde_json::json;

    use super::super::slips;
    use super::*;

    const NOW: i64 = 1_794_700_000;
    const DAY: i64 = 86_400;

    fn at(seconds: i64) -> String {
        Timestamp::from_second(seconds)
            .map(|t| t.to_string())
            .unwrap_or_default()
    }

    /// An ask of Joel's due two days ago, by `owner`, with `more` fields.
    fn ask(words: &str, owner: &str, more: &Value) -> Value {
        let mut ask = json!({"at": at(NOW - 3 * DAY), "words": words, "owner": owner,
            "expected": at(NOW - 2 * DAY)});
        if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
            ask.extend(more.clone());
        }
        ask
    }

    fn list(asks: &[Value]) -> String {
        asks.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn an_ask_parked_on_a_ruling_leaves_the_slips_and_is_counted_by_its_ruling() {
        let put_down = json!({"parked": "b95ea57b", "closed": "put down"});
        let derived = json!({"parked": "b95ea57b", "tracking": "derived"});
        let text = list(&[
            ask("spread load", "A", &json!({"parked": "b95ea57b"})),
            ask("zstd restart", "B", &json!({"parked": " b95ea57b "})),
            ask("offload features", "C", &json!({"parked": "1885d8c5"})),
            ask("unparked", "D", &json!({"parked": "1885d8c5"})),
            ask("unparked", "D", &json!({})),
            ask("parked blank", "E", &json!({"parked": " "})),
            ask("put down", "F", &put_down),
            ask("derived", "G", &derived),
        ]);
        assert_eq!(
            slips(&text, NOW, Tracking::His).behind,
            [
                "unparked, due 2 days ago · D",
                "parked blank, due 2 days ago · E"
            ],
            "a parked ask was counted as slipped, or one let go or blank was left out"
        );
        let his = parked(&text, Tracking::His);
        let by_ruling = vec!["b95ea57b: 2 asks".to_owned(), "1885d8c5: 1 ask".to_owned()];
        assert_eq!(
            (his.now, his.target, his.behind),
            (3.0, None, by_ruling),
            "parked asks miscounted: one let go, put down or of another kind was counted"
        );
        assert_eq!(
            parked(&text, Tracking::Derived).behind,
            ["b95ea57b: 1 ask"],
            "a derived ask's parking was read as Joel's, or lost"
        );
    }
}
