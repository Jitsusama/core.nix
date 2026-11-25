//! The asks the list parked on a ruling, such as one set aside until load
//! is spread. Its keeper marks one with `parked`, the ruling's message id,
//! and the slips measure leaves it out, since its owner owes no date while
//! the ruling stands. They are counted here by ruling, and the card names
//! each ruling and its count, so none drops out of sight and a parking
//! left standing once its ruling is spent is read there. Cadence can't
//! read the mesh a ruling was said on, so whether it still stands is for
//! whoever reads the card. Taking the field off brings the ask's dates
//! back to the slips measure, read as they stand, so an ask let go is
//! given a new date with it.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Class, Measure, count, lines, seconds};
use crate::tracking::{self, Tracking};

/// The key of Joel's parked asks; the list's own add their kind to it.
pub const KEY: &str = "slips-parked";

/// The ruling `ask` is parked on, or none when it isn't parked: a
/// `parked` that is blank, or not words, parks nothing.
#[must_use]
pub fn ruling(ask: &Value) -> Option<&str> {
    ask["parked"]
        .as_str()
        .map(str::trim)
        .filter(|r| !r.is_empty())
}

/// How many asks of the kind `which` in the asks list `text` are parked on
/// a ruling, each ask by its latest line, named by ruling, the most first.
/// One landed waits on nothing, and one closed without landing is put
/// down, not parked, so neither is counted.
#[must_use]
pub fn parked(text: &str, which: Tracking) -> Measure {
    let mut latest: BTreeMap<[String; 2], Option<String>> = BTreeMap::new();
    for ask in lines(text) {
        let ours = tracking::of(&ask) == Some(which) && seconds(&ask["landed"]).is_none();
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
        key: which.key(KEY),
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
    fn an_ask_that_slipped_then_was_parked_counts_none_of_its_dates() {
        let undated = json!({"parked": "b95ea57b", "expected": null});
        let text = list(&[
            ask("spread load", "A", &json!({})),
            ask("spread load", "A", &undated),
            ask("zstd restart", "B", &json!({})),
            ask(
                "zstd restart",
                "B",
                &json!({"parked": "b95ea57b", "expected": at(NOW - DAY - 60)}),
            ),
        ]);
        assert_eq!(
            slips(&text, NOW, Tracking::His).behind,
            Vec::<String>::new(),
            "a date given before an ask was parked, or moved as it was, still counted as a slip"
        );
        assert_eq!(
            parked(&text, Tracking::His).now,
            2.0,
            "an ask parked on a line with no date, or a moved one, wasn't counted parked"
        );
    }

    #[test]
    fn an_ask_let_go_of_its_ruling_counts_its_dates_as_they_stand() {
        let text = list(&[
            ask("unparked", "D", &json!({"parked": "1885d8c5"})),
            ask("unparked", "D", &json!({})),
        ]);
        assert_eq!(
            slips(&text, NOW, Tracking::His).behind,
            ["unparked, due 2 days ago · D"],
            "an ask whose latest line has no parked field stayed out of the slips"
        );
        assert_eq!(
            parked(&text, Tracking::His).now,
            0.0,
            "an ask let go of its ruling was still counted parked"
        );
    }

    #[test]
    fn parked_asks_are_named_by_ruling_the_most_first() {
        let text = list(&[
            ask("spread load", "A", &json!({"parked": "b95ea57b"})),
            ask("zstd restart", "B", &json!({"parked": " b95ea57b "})),
            ask("offload features", "C", &json!({"parked": "1885d8c5"})),
        ]);
        let his = parked(&text, Tracking::His);
        let by_ruling = vec!["b95ea57b: 2 asks".to_owned(), "1885d8c5: 1 ask".to_owned()];
        assert_eq!(
            (his.key.as_str(), his.now, his.target, his.behind),
            (KEY, 3.0, None, by_ruling),
            "parked asks weren't grouped by their trimmed ruling, the most first, unjudged"
        );
    }

    #[test]
    fn an_ask_landed_put_down_or_parked_on_nothing_is_not_counted_parked() {
        let landed = json!({"parked": "b95ea57b", "landed": at(NOW - DAY)});
        let put_down = json!({"parked": "b95ea57b", "closed": "put down"});
        let text = list(&[
            ask("landed", "A", &landed),
            ask("put down", "B", &put_down),
            ask("blank", "C", &json!({"parked": " "})),
            ask("a number", "D", &json!({"parked": 5})),
            ask("null", "E", &json!({"parked": null})),
        ]);
        assert_eq!(
            parked(&text, Tracking::His).now,
            0.0,
            "an ask landed or put down, or with no ruling in its field, was counted parked"
        );
        assert_eq!(
            slips(&text, NOW, Tracking::His).behind,
            [
                "blank, due 2 days ago · C",
                "a number, due 2 days ago · D",
                "null, due 2 days ago · E"
            ],
            "a parked field naming no ruling took an ask out of the slips"
        );
    }

    #[test]
    fn a_derived_ask_is_parked_apart_from_joels() {
        let derived = json!({"parked": "5fa32250", "tracking": "derived"});
        let text = list(&[ask("methuselah", "F", &derived)]);
        assert_eq!(
            slips(&text, NOW, Tracking::Derived).behind,
            Vec::<String>::new(),
            "a parked derived ask still counted in the list's own slips"
        );
        assert_eq!(
            (
                parked(&text, Tracking::Derived).behind,
                parked(&text, Tracking::His).now
            ),
            (vec!["5fa32250: 1 ask".to_owned()], 0.0),
            "a parked derived ask wasn't counted as the list's own, or was counted as Joel's"
        );
    }
}
