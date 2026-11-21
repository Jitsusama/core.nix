//! What moved for each open quest since Joel last looked, so Now can say
//! it rather than his asking for updates (Joel, 2026-11-16: "i ask for
//! updatees all the time. that is a waste. i get notices, fine, those
//! could replace me asking"). It's read from the asks file, a line of JSON
//! per thing asked of a quest: `at` when it was asked, `words` what,
//! `owner` the quest owning it, `expected` when it's due, `landed` when it
//! landed, `closed` once it was put down, and `promised`, each date given
//! (`given`) for it.
//!
//! The file's keeper rewrites it whole, and cadence reads it too, so it's
//! read as cadence reads it (modules/cadence): an ask is known by its `at`
//! and `words`, and its last line written wins; its owner is the first
//! named, before those helping; and one closed without landing was put
//! down, while one closed once landed still says it landed.
//!
//! What he looked at is what he was shown of each ask, its landing and its
//! date, kept as the daemon served it. An ask moved once either is other
//! than he was shown, or once it's one he was never shown: never by its
//! stamps against a clock, since the keeper may write a landing after he
//! looked that it stamps before, or stamp one ahead of the clock. Each
//! open quest with an ask moved is one entry: named by the quest, about
//! its summary, its note what landed, else that a date moved, else what
//! it took on, and its side the next date still due.

use std::collections::BTreeMap;

use ready_rows::time::{self, Form};
use serde_json::{Value, json};

use crate::quest::{self, Quest};
use crate::words::Words;

/// The kind of the records saying what moved.
pub const KIND: &str = "moved";

/// What Joel was shown of an ask: when it landed, and when it's due.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Seen {
    pub landed: Option<i64>,
    pub expected: Option<i64>,
}

/// What he was shown of each ask, by its key.
pub type Looked = BTreeMap<String, Seen>;

/// One ask as its last line says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// Its `at` and `words`, which know it however its line is rewritten.
    pub key: String,
    /// The quest owning it, by the id or short id its owner field names
    /// first.
    pub owner: String,
    pub words: String,
    /// The latest moment its line names, in ms since 1970, which orders
    /// the quests that moved and decides nothing.
    pub latest: i64,
    /// When it landed, if it has.
    pub landed: Option<i64>,
    /// When it's due, if it's said.
    pub expected: Option<i64>,
}

impl Ask {
    /// What showing it now shows.
    #[must_use]
    pub const fn seen(&self) -> Seen {
        Seen {
            landed: self.landed,
            expected: self.expected,
        }
    }
}

/// How an ask moved since he looked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum How {
    Landed,
    Dated,
    Taken,
}

fn moment_of(line: &Value, field: &str) -> Option<i64> {
    line.get(field)
        .and_then(Value::as_str)
        .and_then(quest::moment)
}

/// The first owner an owner field names, as cadence's `first_owner`
/// reads it (modules/cadence/src/hour/measures/owner.rs): the keeper
/// types `PN2VZC (H6EDLC)` or `Y2YBTS, J7XHQN`, the one owning it before
/// those helping.
fn first_owner(owner: &str) -> &str {
    owner
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | '/' | ';' | '('))
        .find(|name| !name.is_empty())
        .unwrap_or_default()
}

/// The asks standing in the file's text, each by its last line: a line
/// that isn't JSON, or names no owner or moment, is left out, as is an
/// ask whose last line put it down, closed without landing.
#[must_use]
pub fn asks(text: &str) -> Vec<Ask> {
    let mut last: BTreeMap<String, Option<Ask>> = BTreeMap::new();
    for line in text.lines() {
        let Ok(line) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let said = |field: &str| line.get(field).and_then(Value::as_str).unwrap_or("");
        let (at, words) = (said("at"), said("words"));
        let owner = first_owner(said("owner"));
        let Some(asked) = quest::moment(at).filter(|_| !owner.is_empty()) else {
            continue;
        };
        let landed = moment_of(&line, "landed");
        let given = line["promised"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|date| moment_of(date, "given"));
        let latest = std::iter::once(asked)
            .chain(landed)
            .chain(given)
            .max()
            .unwrap_or(asked);
        let down = line["closed"].as_str().is_some_and(|why| !why.is_empty()) && landed.is_none();
        let key = format!("{at}\t{words}");
        let ask = (!down).then(|| Ask {
            key: key.clone(),
            owner: owner.to_owned(),
            words: words.to_owned(),
            latest,
            landed,
            expected: moment_of(&line, "expected"),
        });
        last.insert(key, ask);
    }
    last.into_values().flatten().collect()
}

/// What he's shown of every ask now, as a first start takes it, so it
/// says nothing until something moves rather than every ask there ever
/// was.
#[must_use]
pub fn shown(asks: &[Ask]) -> Looked {
    asks.iter()
        .map(|ask| (ask.key.clone(), ask.seen()))
        .collect()
}

/// What he looked at, as kept: none where it isn't what `kept` writes.
#[must_use]
pub fn looked_from(text: &str) -> Option<Looked> {
    let kept: Value = serde_json::from_str(text).ok()?;
    seen_in(kept.get("seen")?).map(|seen| seen.into_iter().collect())
}

/// What he looked at, to keep.
#[must_use]
pub fn kept(looked: &Looked) -> String {
    let seen: Vec<Value> = looked.iter().map(|(key, seen)| said(key, *seen)).collect();
    format!("{}\n", json!({ "seen": seen }))
}

/// Each ask a list says he was shown, as the `looked` ask and the kept
/// file give it: none where any isn't a key with a landing and a date,
/// each a moment or null.
#[must_use]
pub fn seen_in(list: &Value) -> Option<Vec<(String, Seen)>> {
    let moment = |one: &Value, field: &str| match one.get(field) {
        None | Some(Value::Null) => Some(None),
        Some(at) => at.as_i64().map(Some),
    };
    list.as_array()?
        .iter()
        .map(|one| {
            let key = one.get("key")?.as_str()?.to_owned();
            let landed = moment(one, "landed")?;
            let expected = moment(one, "expected")?;
            Some((key, Seen { landed, expected }))
        })
        .collect()
}

fn said(key: &str, seen: Seen) -> Value {
    json!({ "key": key, "landed": seen.landed, "expected": seen.expected })
}

/// How an ask moved since he looked, if it did: it landed other than he
/// was shown, or its date moved while it's still owed, or it's one he was
/// never shown. A landing taken back, or a date moved once it landed,
/// says nothing he'd act on.
fn how(ask: &Ask, looked: &Looked) -> Option<How> {
    let Some(was) = looked.get(&ask.key) else {
        return Some(if ask.landed.is_some() {
            How::Landed
        } else {
            How::Taken
        });
    };
    if ask.landed.is_some() && ask.landed != was.landed {
        Some(How::Landed)
    } else if ask.landed.is_none() && ask.expected != was.expected {
        Some(How::Dated)
    } else {
        None
    }
}

/// Whether a quest is the one an owner names: its id, or its id's last
/// part, as `QEST-20261023-Y2YBTS` is `Y2YBTS`.
fn names(quest: Quest, owner: &str) -> bool {
    quest.id() == owner || quest.id().rsplit('-').next() == Some(owner)
}

/// The soonest date still owed of any ask, after `now`: when a record made
/// now would show a date that has passed, so it's made again then, though
/// nothing else has moved.
#[must_use]
pub fn next_due(asks: &[Ask], now: i64) -> Option<i64> {
    soonest(asks.iter(), now.saturating_add(1))
}

/// The soonest date of `asks` not landed and not passed by `now`, which
/// is the next one owed; a passed one is a slip its keeper dates again.
fn soonest<'a>(asks: impl Iterator<Item = &'a Ask>, now: i64) -> Option<i64> {
    asks.filter(|ask| ask.landed.is_none())
        .filter_map(|ask| ask.expected)
        .filter(|at| *at >= now)
        .min()
}

/// What moved, as one record served beside Quests' rows, `moved:quests`,
/// holding an entry for each open quest with an ask moved since he
/// looked, and `seen`, what showing them shows of each, for the launcher
/// to give back once he's looked: one record, as `folders:quests` is,
/// since the launcher holds what's served beside a place's rows by the
/// kind and the place alone. `now` tells which dates are still to come.
#[must_use]
pub fn record(asks: &[Ask], looked: &Looked, quests: &[Quest], words: &Words, now: i64) -> Value {
    let made = entries(asks, looked, quests, words, now);
    let seen: Vec<Value> = made
        .iter()
        .flat_map(|(moved, _)| moved.iter().map(|ask| said(&ask.key, ask.seen())))
        .collect();
    let moved: Vec<Value> = made.into_iter().map(|(_, entry)| entry).collect();
    json!({ "key": format!("{KIND}:quests"), "kind": KIND, "moved": moved, "seen": seen })
}

/// One entry for each open quest with an ask moved since he looked, with
/// those asks, the quest whose line names the latest moment first.
fn entries<'a>(
    asks: &'a [Ask],
    looked: &Looked,
    quests: &[Quest],
    words: &Words,
    now: i64,
) -> Vec<(Vec<&'a Ask>, Value)> {
    let mut made: Vec<(i64, Vec<&Ask>, Value)> = quests
        .iter()
        .filter(|quest| quest.open())
        .filter_map(|quest| {
            let own: Vec<&Ask> = asks
                .iter()
                .filter(|ask| names(*quest, &ask.owner))
                .collect();
            let moved: Vec<(&Ask, How)> = own
                .iter()
                .filter_map(|ask| Some((*ask, how(ask, looked)?)))
                .collect();
            let latest = moved.iter().map(|(ask, _)| ask.latest).max()?;
            let entry = entry(*quest, &own, &moved, words, now);
            Some((
                latest,
                moved.into_iter().map(|(ask, _)| ask).collect(),
                entry,
            ))
        })
        .collect();
    made.sort_by(|(one, ..), (other, ..)| other.cmp(one));
    made.into_iter()
        .map(|(_, moved, entry)| (moved, entry))
        .collect()
}

/// A quest's entry: what landed, else that a date moved, else what it
/// took on, and the next date it still owes: the soonest still to come,
/// since a date passed unlanded is a slip its keeper dates again, never
/// the next one owed.
fn entry(quest: Quest, own: &[&Ask], moved: &[(&Ask, How)], words: &Words, now: i64) -> Value {
    let all = |how: How| -> Vec<&Ask> {
        let mut these: Vec<&Ask> = moved
            .iter()
            .filter(|(_, was)| *was == how)
            .map(|(ask, _)| *ask)
            .collect();
        these.sort_by_key(|ask| std::cmp::Reverse(ask.landed.unwrap_or(ask.latest)));
        these
    };
    let (landed, dated, taken) = (all(How::Landed), all(How::Dated), all(How::Taken));
    let note = match (landed.as_slice(), dated.first(), taken.first()) {
        ([one], ..) => words.fill("moved-landed", &[("words", &one.words)]),
        ([one, rest @ ..], ..) => words.fill(
            "moved-landed-more",
            &[("words", &one.words), ("more", &rest.len().to_string())],
        ),
        ([], Some(one), _) => words.fill("moved-dated", &[("words", &one.words)]),
        ([], None, Some(one)) => words.fill("moved-taken", &[("words", &one.words)]),
        ([], None, None) => String::new(),
    };
    let next = soonest(own.iter().copied(), now);
    let side: Vec<Value> = next
        .map(|at| time::said(at, Form::Moment, &words.fill("moved-next", &[])))
        .into_iter()
        .collect();
    json!({
        "key": format!("{KIND}:quests:{}", quest.id()), "kind": KIND,
        "thing": format!("quest:{}", quest.id()), "name": quest.name(),
        "reads": quest.summary(), "note": note, "side": side,
        "held": { "id": quest.id(), "landed": landed.len(), "moved": moved.len() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::of(&[
            ("moved-landed", "landed: {words}"),
            ("moved-landed-more", "landed: {words}, and {more} more"),
            ("moved-dated", "a date moved: {words}"),
            ("moved-taken", "taken on: {words}"),
            ("moved-next", "next {time}"),
        ])
    }

    fn line(owner: &str, at: &str, words: &str, more: &Value) -> String {
        let mut line = json!({ "at": at, "owner": owner, "words": words });
        for (field, value) in more.as_object().into_iter().flatten() {
            line[field] = value.clone();
        }
        line.to_string()
    }

    fn ms(at: &str) -> i64 {
        quest::moment(at).expect("a moment")
    }

    fn quest(id: &str, status: &str) -> Value {
        json!({ "id": id, "title": format!("{id}'s title"), "status": status,
                "summary": format!("what {id} is about") })
    }

    const AT: &str = "2026-11-20T10:00:00-05:00";

    /// When the records are made, after every ask was written.
    fn now() -> i64 {
        ms("2026-11-21T02:00:00-05:00")
    }

    #[test]
    fn an_ask_written_again_is_known_by_its_last_line() {
        let text = [
            line(
                "A",
                AT,
                "one",
                &json!({"expected": "2026-11-21T12:00:00-05:00"}),
            ),
            line(
                "A",
                AT,
                "one",
                &json!({
                "expected": "2026-11-22T12:00:00-05:00",
                "promised": [{"given": "2026-11-21T02:00:00-05:00"}]}),
            ),
            line(
                "B (A)",
                AT,
                "one",
                &json!({"expected": "2026-11-19T00:00:00-05:00"}),
            ),
            "not json".to_owned(),
            line("", AT, "no owner", &json!({})),
        ]
        .join("\n");
        let asks = asks(&text);
        assert_eq!(asks.len(), 1, "one ask, the rest left out: {asks:?}");
        assert_eq!(
            (asks[0].owner.as_str(), asks[0].expected),
            ("B", Some(ms("2026-11-19T00:00:00-05:00"))),
            "the line written last wins, its owner retyped as cadence reads it"
        );
    }

    #[test]
    fn an_owner_is_the_first_named_before_those_helping() {
        let text = [
            line("PN2VZC (H6EDLC)", AT, "one", &json!({})),
            line("Y2YBTS, J7XHQN", AT, "two", &json!({})),
            line("XUBK2E/H2VYOB", AT, "three", &json!({})),
            line("QEST-20261115-4S6QRI (JCWNFD)", AT, "four", &json!({})),
        ]
        .join("\n");
        let owners: Vec<String> = asks(&text).into_iter().map(|ask| ask.owner).collect();
        assert_eq!(
            owners,
            ["QEST-20261115-4S6QRI", "PN2VZC", "XUBK2E", "Y2YBTS"],
            "each owner as the quest it names, in the asks' order by key"
        );
        let quests = [quest("QEST-20261115-4S6QRI", "active")];
        let quests: Vec<Quest> = quests.iter().map(Quest).collect();
        let asks = asks(&text);
        let made = entries(&asks, &Looked::new(), &quests, &words(), now());
        assert_eq!(made.len(), 1, "a quest named by its id is its own");
    }

    #[test]
    fn an_ask_put_down_unlanded_stands_no_longer_and_one_landed_does() {
        let text = [
            line("A", AT, "down", &json!({"closed": "superseded"})),
            line(
                "A",
                AT,
                "landed",
                &json!({"closed": "done", "landed": "2026-11-20T11:00:00-05:00"}),
            ),
            line("A", AT, "empty", &json!({"closed": ""})),
            line("A", AT, "null", &json!({"closed": null})),
        ]
        .join("\n");
        let said: Vec<String> = asks(&text).into_iter().map(|ask| ask.words).collect();
        assert_eq!(
            said,
            ["empty", "landed", "null"],
            "only the one put down unlanded goes, as cadence puts it down"
        );
    }

    #[test]
    fn what_he_was_shown_of_an_ask_is_what_moves_it_not_its_stamps() {
        let landed = "2026-11-20T11:00:00-05:00";
        let ahead = "2099-01-01T00:00:00-05:00";
        let before = [
            line("ONE", AT, "lands", &json!({})),
            line("ONE", AT, "dated", &json!({"expected": landed})),
            line("ONE", AT, "still", &json!({"expected": landed})),
            line("ONE", AT, "ahead", &json!({"landed": ahead})),
            line("ONE", AT, "unlanded", &json!({"landed": landed})),
            line(
                "ONE",
                AT,
                "landed-dated",
                &json!({"landed": landed, "expected": landed}),
            ),
        ]
        .join("\n");
        let looked = shown(&asks(&before));
        let after = [
            // Landed after he looked, though the keeper stamps it at the
            // moment he was asked, before any looking.
            line("ONE", AT, "lands", &json!({"landed": AT})),
            line("ONE", AT, "dated", &json!({"expected": ahead})),
            line("ONE", AT, "still", &json!({"expected": landed})),
            line("ONE", AT, "ahead", &json!({"landed": ahead})),
            line("ONE", AT, "unlanded", &json!({})),
            line(
                "ONE",
                AT,
                "landed-dated",
                &json!({"landed": landed, "expected": ahead}),
            ),
            line("ONE", AT, "new", &json!({})),
        ]
        .join("\n");
        let asks = asks(&after);
        let moved: Vec<(&str, Option<How>)> = asks
            .iter()
            .map(|ask| (ask.words.as_str(), how(ask, &looked)))
            .collect();
        assert_eq!(
            moved,
            [
                ("ahead", None),
                ("dated", Some(How::Dated)),
                ("landed-dated", None),
                ("lands", Some(How::Landed)),
                ("new", Some(How::Taken)),
                ("still", None),
                ("unlanded", None),
            ],
            "moved by what showing it shows, whatever its stamps say of the clock"
        );
    }

    /// What he was shown of five quests' asks, before they moved.
    fn shown_before() -> Looked {
        let before = [
            line("ONE", "2026-11-20T09:00:00-05:00", "old", &json!({})),
            line("ONE", "2026-11-20T09:00:00-05:00", "fix {more}", &json!({})),
            line("ONE", "2026-11-20T09:30:00-05:00", "more", &json!({})),
            line("ONE", AT, "overdue", &json!({"expected": OVERDUE})),
            line("TWO", AT, "moved", &json!({"expected": AT})),
            line("THREE", AT, "known", &json!({})),
            line("STILL", "2026-11-19T10:00:00-05:00", "quiet", &json!({})),
        ]
        .join("\n");
        shown(&asks(&before))
    }

    /// A date ONE owed that passed unlanded, before the records are made.
    const OVERDUE: &str = "2026-11-20T12:00:00-05:00";

    /// A date ONE owes after its soonest, which is never the next.
    const LATER: &str = "2026-11-25T18:00:00-05:00";

    /// Their asks once moved: ONE's two landed and took one on, TWO's
    /// date moved, THREE took one on, STILL's didn't move and ENDED ended.
    fn moved_after() -> String {
        [
            line("ONE", "2026-11-20T09:00:00-05:00", "old", &json!({})),
            line(
                "ONE",
                "2026-11-20T09:00:00-05:00",
                "fix {more}",
                &json!({"landed": "2026-11-21T01:00:00-05:00"}),
            ),
            line(
                "ONE",
                "2026-11-20T09:30:00-05:00",
                "more",
                &json!({"landed": "2026-11-21T00:30:00-05:00"}),
            ),
            line(
                "ONE",
                AT,
                "owed",
                &json!({"expected": "2026-11-21T18:00:00-05:00"}),
            ),
            line("ONE", AT, "overdue", &json!({"expected": OVERDUE})),
            line("ONE", AT, "later", &json!({"expected": LATER})),
            line(
                "TWO",
                AT,
                "moved",
                &json!({"expected": "2026-11-22T18:00:00-05:00",
                        "promised": [{"given": "2026-11-21T02:00:00-05:00"}]}),
            ),
            line("THREE", AT, "known", &json!({})),
            line(
                "THREE",
                "2026-11-20T11:00:00-05:00",
                "new {more}",
                &json!({}),
            ),
            line("STILL", "2026-11-19T10:00:00-05:00", "quiet", &json!({})),
            line("ENDED", "2026-11-21T01:00:00-05:00", "gone", &json!({})),
        ]
        .join("\n")
    }

    fn five_quests() -> Vec<Value> {
        vec![
            quest("QEST-20261023-ONE", "active"),
            quest("QEST-20261024-TWO", "active"),
            quest("QEST-20261024-THREE", "active"),
            quest("QEST-20261025-STILL", "active"),
            quest("QEST-20261026-ENDED", "concluded"),
        ]
    }

    #[test]
    fn a_record_is_made_again_as_the_soonest_date_still_owed_passes() {
        let asks = asks(&moved_after());
        let soonest = ms("2026-11-21T18:00:00-05:00");
        assert_eq!(
            next_due(&asks, now()),
            Some(soonest),
            "the soonest owed, not LATER"
        );
        assert_eq!(
            next_due(&asks, soonest),
            Some(ms("2026-11-22T18:00:00-05:00")),
            "once it's here, the one after"
        );
        assert_eq!(
            next_due(&asks, ms(LATER)),
            None,
            "none once the last has come, nor a landed one's"
        );
    }

    #[test]
    fn each_open_quest_that_moved_is_one_entry_latest_first() {
        let quests = five_quests();
        let quests: Vec<Quest> = quests.iter().map(Quest).collect();
        let made = record(
            &asks(&moved_after()),
            &shown_before(),
            &quests,
            &words(),
            now(),
        );
        let keys: Vec<&str> = made["moved"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|one| one["key"].as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "moved:quests:QEST-20261024-TWO",
                "moved:quests:QEST-20261023-ONE",
                "moved:quests:QEST-20261024-THREE",
            ],
            "only open quests that moved, the latest line first"
        );
        let one = &made["moved"][1];
        assert_eq!(one["name"], "QEST-20261023-ONE's title");
        assert_eq!(one["reads"], "what QEST-20261023-ONE is about");
        assert_eq!(
            one["note"], "landed: fix {more}, and 1 more",
            "the latest landing named, over the ask it took on, its own words said as they are"
        );
        assert_eq!(
            one["side"],
            json!([{"at": ms("2026-11-21T18:00:00-05:00"), "as": "moment", "said": "next {time}"}]),
            "the soonest date still to come, not one passed unlanded"
        );
        assert_eq!(made["moved"][0]["note"], "a date moved: moved");
        assert_eq!(made["moved"][2]["note"], "taken on: new {more}");
        assert_eq!(
            made["moved"][2]["side"],
            json!([]),
            "nothing owed says no date"
        );
    }

    #[test]
    fn what_it_showed_given_back_as_looked_at_moves_nothing() {
        let quests = five_quests();
        let quests: Vec<Quest> = quests.iter().map(Quest).collect();
        let after = asks(&moved_after());
        let mut looked = shown_before();
        let made = record(&after, &looked, &quests, &words(), now());
        let seen = seen_in(&made["seen"]).expect("what it shows reads back");
        assert_eq!(seen.len(), 6, "every ask moved is shown: {seen:?}");
        looked.extend(seen);
        assert_eq!(
            record(&after, &looked, &quests, &words(), now())["moved"],
            json!([]),
            "once looked at, nothing moved"
        );
    }

    #[test]
    fn what_he_looked_at_is_kept_and_read_back() {
        let looked: Looked = [
            (
                "a\tone".to_owned(),
                Seen {
                    landed: Some(1),
                    expected: None,
                },
            ),
            (
                "b\ttwo".to_owned(),
                Seen {
                    landed: None,
                    expected: Some(2),
                },
            ),
        ]
        .into_iter()
        .collect();
        assert_eq!(looked_from(&kept(&looked)), Some(looked));
        assert_eq!(
            looked_from("1700000000000\n"),
            None,
            "a bare moment isn't it"
        );
        assert_eq!(
            seen_in(&json!([{"key": "a", "landed": "soon"}])),
            None,
            "a landing that's no moment isn't taken"
        );
    }

    #[test]
    fn nothing_moved_since_he_looked_is_an_empty_record() {
        let text = line("ONE", AT, "fix", &json!({"landed": AT}));
        let quests = [quest("QEST-20261023-ONE", "active")];
        let quests: Vec<Quest> = quests.iter().map(Quest).collect();
        let asks = asks(&text);
        assert_eq!(
            record(&asks, &shown(&asks), &quests, &words(), now()),
            json!({"key": "moved:quests", "kind": "moved", "moved": [], "seen": []}),
            "what moved is still served, holding nothing"
        );
    }
}
