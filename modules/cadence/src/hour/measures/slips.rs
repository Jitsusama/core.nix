//! The dates the asks list was given that passed unanswered, Joel's and
//! those it set itself, each closed only on its owner's word.

use std::collections::BTreeMap;

use super::promised::promised;
use super::{Class, DAY, Measure, asks, count, first_owner, parked, seconds, short};
use crate::tracking::{self, Tracking};

/// A date that slipped: when it was due, the ask's words and its owner.
type Slip = (i64, String, String);

/// The dates promised in the orchestrator's `asks.jsonl` that passed in
/// the day before `now` without their ask landing by then, of asks of
/// the kind `which`, and every older one its owner hasn't answered since
/// by landing the ask, meeting the date or moving it: a slip closes only
/// on its owner's word, never by going a day old. Every date an ask was
/// given counts, from its `promised`, each with when it was given, so a
/// date moved in place after it passed is still a slip that day; one moved
/// before it passed isn't, nor one whose entry says it was `met` by then,
/// as a step kept before the ask moves on to the next. A line without
/// `promised` gives its `expected` alone. An older date is answered by a
/// landing or a move said by `now`, or by any later date the ask was
/// given, since a date moved by `expected` alone says no time it moved.
/// An older slip's line says how long ago it was due, so one stuck for
/// weeks reads apart from yesterday's. An ask whose latest line is
/// [`parked`] on a ruling has none of its dates counted while it stands:
/// the list writes an ask's whole record each time, as it does `closed`
/// and `tracking`, so a line without the field has let it go.
#[must_use]
pub fn slips(text: &str, now: i64, which: Tracking) -> Measure {
    // The list writes an ask again to move its date, or to re-mark, land
    // or put it down: each date of an ask, by when and what was asked, is
    // read from its latest line, in the order the list first wrote it.
    let mut latest: BTreeMap<[String; 3], (usize, Option<Slip>)> = BTreeMap::new();
    // Each ask's dates on its latest line, and whether that line parks it,
    // which takes out every date it was ever given. A date gone from them
    // was moved by a line written without `promised` to say so, which
    // answers it, while a latest line with no date it can read moves none.
    let mut dated: BTreeMap<[String; 2], (Vec<i64>, bool)> = BTreeMap::new();
    for (order, ask) in asks(text).enumerate() {
        let ours = tracking::of(&ask) == Some(which);
        let landed = seconds(&ask["landed"]);
        let given = promised(&ask);
        let asked = tracking::asked(&ask);
        let on_ruling = parked::ruling(&ask).is_some();
        dated.insert(
            asked,
            (given.iter().map(|p| p.expected).collect(), on_ruling),
        );
        let last = given.len();
        for (i, promise) in given.iter().enumerate() {
            let (expected, moved, met) = (promise.expected, promise.moved, promise.met);
            let kept = landed.is_some_and(|landed| landed <= expected)
                || met.is_some_and(|met| met <= expected)
                || moved.is_some_and(|moved| moved < expected);
            // A `met` stands only on a date moved from, so the move answers.
            let answered = landed.is_some_and(|landed| landed <= now)
                || moved.is_some_and(|moved| moved <= now)
                || i + 1 < last;
            let late = ours && expected <= now && !kept && (expected > now - DAY || !answered);
            let [at, words] = tracking::asked(&ask);
            let key = [at, words, expected.to_string()];
            let first = latest.get(&key).map_or(order, |(first, _)| *first);
            let slip = late.then(|| {
                (
                    expected,
                    ask["words"].as_str().unwrap_or_default().to_owned(),
                    first_owner(ask["owner"].as_str().unwrap_or_default()),
                )
            });
            latest.insert(key, (first, slip));
        }
    }
    let mut latest: Vec<(usize, Option<Slip>)> = latest
        .into_iter()
        .map(|([at, words, _], (first, slip))| {
            let (held, on_ruling) = dated
                .get(&[at, words])
                .map_or((None, false), |(held, on)| (Some(held), *on));
            let slip = slip.filter(|&(expected, _, _)| {
                !on_ruling
                    && (expected > now - DAY
                        || held.is_none_or(|held| held.is_empty() || held.contains(&expected)))
            });
            (first, slip)
        })
        .collect();
    latest.sort_by_key(|(order, _)| *order);
    let mut slipped: Vec<Slip> = latest.into_iter().filter_map(|(_, slip)| slip).collect();
    slipped.sort_by_key(|(expected, _, _)| *expected);
    Measure {
        key: which.key("slips"),
        class: Class::Slip,
        says: if which == Tracking::His {
            "dates promised that passed in the last day without landing, or since, unanswered"
        } else {
            "dates the list set itself (derived, mends) that passed in the last day, or since, \
             unanswered"
        }
        .to_owned(),
        now: count(slipped.len()),
        target: Some(0.0),
        owner: slipped
            .first()
            .map(|(_, _, o)| o.clone())
            .unwrap_or_default(),
        behind: slipped
            .iter()
            .map(|(expected, words, owner)| {
                format!("{}{} · {owner}", short(words), due_ago(now - expected))
            })
            .collect(),
    }
}

/// How long ago a slip `late` seconds past its date was due, or nothing
/// within the day, which the measure's own words already say.
fn due_ago(late: i64) -> String {
    match late / DAY {
        0 => String::new(),
        1 => ", due a day ago".to_owned(),
        days => format!(", due {days} days ago"),
    }
}
