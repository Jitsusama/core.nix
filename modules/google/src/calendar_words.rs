//! The words Calendar's rows are said in, read from the launcher's
//! vocabulary, and how a day, a month, an ordinal and a meeting's repeats
//! are said in them, apart from the rows (`calendar_rows.rs`) that say
//! them.

use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::Value;

use crate::calendar::{Answer, Every, Repeats};
use crate::rows::fill;

/// Between the members of a list said in one fact.
pub(crate) const COMMA: &str = ", ";

/// The words Calendar's rows are said in, from the launcher's vocabulary
/// (launcher/words.json), every one: nothing Joel reads is written here
/// but what the meeting itself says, the clock's figures and the marks
/// that join a run, which `words_are_all_the_vocabularys` holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// What its own words say of a meeting, by their key in its `said`.
    pub said: BTreeMap<String, String>,
    /// Each fact's name, by the fact's id after `calendar:` and its state.
    pub facts: BTreeMap<(String, String), String>,
    /// The days of the week from Sunday, and the months from January.
    pub weekdays: Vec<String>,
    pub months: Vec<String>,
    /// The drawer's eyebrow over a call in Meet's Recent.
    pub call_eyebrow: String,
    /// A row's name for a meeting with no title.
    pub untitled: String,
    /// The glyph role every meeting's row shows.
    pub glyph: String,
    /// "{count} in the past 30 days", how many of a series came before.
    pub earlier: String,
    /// What Cancel's sure row starts with, "Cancel It Now".
    pub cancel_sure: String,
    /// What Move's sure row starts with, "Move It".
    pub move_sure: String,
    /// The later times Move offers, by name, in `LATER`'s order.
    pub move_later: Vec<String>,
    /// "{sure} to {when}", where a sure row moves it to.
    pub move_to: String,
    /// "clashes with {title}", the meeting a time would overlap.
    pub move_clashes: String,
    /// The glyph role every call in Meet's Recent shows.
    pub call_glyph: String,
    /// "{first} and {second}", who a call was with.
    pub with_two: String,
    /// "{first} and {count} others".
    pub with_others: String,
    /// Each act's name and glyph role, by its id, for the rows inside a
    /// meeting that do them.
    pub acts: BTreeMap<String, (String, String)>,
}

/// How far Move the Meeting offers to take one: by minutes, or by days
/// at the same time on the clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Later {
    Minutes(i64),
    Days(i64),
}

/// The later times Move offers, each by its state of `verb:calendar:move`
/// in the vocabulary (Calendar.js's `LATER`).
pub(crate) const LATER: [(&str, Later); 5] = [
    ("quarter", Later::Minutes(15)),
    ("half", Later::Minutes(30)),
    ("hour", Later::Minutes(60)),
    ("tomorrow", Later::Days(1)),
    ("week", Later::Days(7)),
];

/// Each act a row inside a meeting does, by its id in the vocabulary.
const ACTS: [&str; 16] = [
    "verb:calendar:join",
    "verb:calendar:write",
    "verb:calendar:see",
    "verb:calendar:open",
    "verb:calendar:transcript",
    "verb:calendar:yes",
    "verb:calendar:maybe",
    "verb:calendar:no",
    "verb:calendar:move",
    "verb:calendar:guests",
    "verb:calendar:attachments",
    "verb:calendar:cancel",
    "verb:calendar:link",
    "verb:calendar:address",
    "verb:person:mail",
    "calendar:attachment",
];

/// The words a meeting's own `said` must hold, each one Calendar's rows
/// say.
const SAID: [&str; 45] = [
    "earlier",
    "eyebrow",
    "going",
    "maybe",
    "notGoing",
    "unanswered",
    "allDay",
    "span",
    "dated",
    "pin",
    "guest",
    "tally",
    "callSide",
    "pageSide",
    "weekday",
    "every",
    "everyMany",
    "day",
    "days",
    "week",
    "weeks",
    "month",
    "months",
    "year",
    "years",
    "on",
    "nth",
    "monthDay",
    "nthDay",
    "times",
    "until",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "last",
    "nthLast",
    "st",
    "nd",
    "rd",
    "th",
    "and",
    "yourAnswer",
    "aboutSide",
];

/// Each fact a drawer names, `calendar:<fact>` in the vocabulary, with the
/// states it's named in besides its own; Telling's names are Move's and
/// Cancel's sure rows' ends.
const FACTS: [(&str, &[&str]); 12] = [
    ("when", &[]),
    ("repeats", &[]),
    ("earlier", &[]),
    ("where", &[]),
    ("call", &[]),
    ("dial", &[]),
    ("answer", &[]),
    ("holder", &[]),
    ("answers", &[]),
    ("asked", &[]),
    ("attached", &["several"]),
    (
        "telling",
        &["one", "two", "three", "more", "guest", "guests"],
    ),
];

impl Words {
    /// Calendar's words from the whole vocabulary, or the first one it
    /// lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let thing = |id: &str| -> Result<&Value, String> {
            vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let word = |id: &str, at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(thing(id)?, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let list = |field: &str, length: usize| -> Result<Vec<String>, String> {
            let listed: Vec<String> = thing("calendar:meeting")?[field]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            (listed.len() == length)
                .then_some(listed)
                .ok_or_else(|| format!("the vocabulary's calendar:meeting hasn't {length} {field}"))
        };
        let said = SAID
            .iter()
            .map(|key| Ok(((*key).to_owned(), word("calendar:meeting", &["said", key])?)))
            .collect::<Result<_, String>>()?;
        let mut facts = BTreeMap::new();
        for (fact, states) in FACTS {
            let id = format!("calendar:{fact}");
            facts.insert((fact.to_owned(), String::new()), word(&id, &["name"])?);
            for state in states {
                let name = word(&id, &["states", state, "name"])?;
                facts.insert((fact.to_owned(), (*state).to_owned()), name);
            }
        }
        facts.insert(
            ("telling".to_owned(), "sure".to_owned()),
            word("calendar:telling", &["said", "sure"])?,
        );
        let acts = ACTS
            .iter()
            .map(|id| {
                Ok((
                    (*id).to_owned(),
                    (word(id, &["name"])?, word(id, &["glyph"])?),
                ))
            })
            .collect::<Result<_, String>>()?;
        Ok(Words {
            said,
            facts,
            weekdays: list("weekdays", 7)?,
            months: list("months", 12)?,
            call_eyebrow: word("meet:held", &["said", "eyebrow"])?,
            untitled: word("calendar:meeting", &["states", "untitled", "name"])?,
            glyph: word("calendar:meeting", &["glyph"])?,
            earlier: word("calendar:meeting", &["said", "earlier"])?,
            cancel_sure: word("verb:calendar:cancel", &["sure"])?,
            move_sure: word("verb:calendar:move", &["sure"])?,
            move_later: LATER
                .iter()
                .map(|(state, _)| word("verb:calendar:move", &["states", state, "name"]))
                .collect::<Result<_, String>>()?,
            move_to: word("verb:calendar:move", &["said", "to"])?,
            move_clashes: word("verb:calendar:move", &["said", "clashes"])?,
            call_glyph: word("meet:held", &["glyph"])?,
            with_two: word("meet:held", &["said", "two"])?,
            with_others: word("meet:held", &["said", "others"])?,
            acts,
        })
    }

    /// A meeting's word by its key in `said`, which `read` made sure of.
    pub(crate) fn word(&self, key: &str) -> &str {
        self.said.get(key).map_or("", String::as_str)
    }

    /// A fact's name in a state, "" for its own.
    pub(crate) fn fact(&self, fact: &str, state: &str) -> &str {
        self.facts
            .get(&(fact.to_owned(), state.to_owned()))
            .map_or("", String::as_str)
    }

    /// An act's name and glyph role, which `read` made sure of.
    pub(crate) fn act(&self, id: &str) -> (&str, &str) {
        self.acts
            .get(id)
            .map_or(("", ""), |(name, glyph)| (name.as_str(), glyph.as_str()))
    }

    /// How Joel answered, as his calendar would say it to him.
    pub(crate) fn answer(&self, answer: Answer) -> &str {
        self.word(match answer {
            Answer::Yes => "going",
            Answer::Maybe => "maybe",
            Answer::No => "notGoing",
            Answer::Waiting => "unanswered",
        })
    }
}

/// A day of the week by its number from Sunday, by name.
pub(crate) fn weekday(day: i8, words: &Words) -> &str {
    words
        .weekdays
        .get(usize::from(day.unsigned_abs()) % 7)
        .map_or("", String::as_str)
}

/// A month by its number from January, 1, by name.
pub(crate) fn month(month: i8, words: &Words) -> &str {
    usize::from(month.unsigned_abs())
        .checked_sub(1)
        .and_then(|at| words.months.get(at))
        .map_or("", String::as_str)
}

/// 1st, 2nd, 3rd, 4th, 11th, 21st: which form English's rule picks is
/// this file's, and the forms are the vocabulary's.
pub(crate) fn ordinal(n: i64, words: &Words) -> String {
    let ending = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    fill(words.word(ending), &[("n", &n.to_string())])
}

/// Which of a day in its month: first, or from the end, last and second
/// last.
fn which(n: i8, words: &Words) -> String {
    const WHICH: [&str; 5] = ["first", "second", "third", "fourth", "fifth"];
    let at = usize::from(n.unsigned_abs());
    let from_start = at.checked_sub(1).and_then(|at| WHICH.get(at)).map_or_else(
        || ordinal(i64::from(n.unsigned_abs()), words),
        |key| words.word(key).to_owned(),
    );
    match n {
        1.. => from_start,
        -1 => words.word("last").to_owned(),
        _ => fill(words.word("nthLast"), &[("which", &from_start)]),
    }
}

/// Words in a list, as one says them: "a", "a and b", "a, b and c".
pub(crate) fn listed_words(listed: &[String], words: &Words) -> String {
    match listed {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => fill(
            words.word("and"),
            &[("some", &first.join(COMMA)), ("last", last)],
        ),
    }
}

/// How a meeting repeats, as its details say it: "every weekday", "every
/// 2 weeks on Monday and Wednesday", "every month on the last Friday,
/// until 31 December 2026" (Repeats.js's repeated), lowercase as every
/// value the launcher says is.
pub(crate) fn repeated(rule: &Repeats, zone: &TimeZone, words: &Words) -> String {
    let (one, many) = match rule.every {
        Every::Day => ("day", "days"),
        Every::Week => ("week", "weeks"),
        Every::Month => ("month", "months"),
        Every::Year => ("year", "years"),
    };
    let plain = rule.days.iter().all(|day| day.nth.is_none());
    let mut days: Vec<u8> = rule.days.iter().map(|day| day.day).collect();
    days.sort_unstable();
    let weekdays = days == [1, 2, 3, 4, 5];
    let mut said = if rule.every == Every::Week && rule.interval == 1 && plain && weekdays {
        words.word("weekday").to_owned()
    } else {
        let said = if rule.interval == 1 {
            fill(words.word("every"), &[("unit", words.word(one))])
        } else {
            fill(
                words.word("everyMany"),
                &[
                    ("count", &rule.interval.to_string()),
                    ("units", words.word(many)),
                ],
            )
        };
        let on: Vec<String> = rule
            .days
            .iter()
            .map(|day| {
                let name = weekday(i8::try_from(day.day).unwrap_or_default(), words);
                day.nth.map_or_else(
                    || name.to_owned(),
                    |nth| {
                        fill(
                            words.word("nth"),
                            &[("which", &which(nth, words)), ("day", name)],
                        )
                    },
                )
            })
            .chain(rule.month_days.iter().map(|&day| {
                if day > 0 {
                    let nth = ordinal(i64::from(day), words);
                    fill(words.word("monthDay"), &[("nth", &nth)])
                } else {
                    fill(words.word("nthDay"), &[("which", &which(day, words))])
                }
            }))
            .collect();
        if on.is_empty() {
            said
        } else {
            fill(
                words.word("on"),
                &[("every", &said), ("days", &listed_words(&on, words))],
            )
        }
    };
    if let Some(times) = rule.times {
        said = fill(
            words.word("times"),
            &[("every", &said), ("count", &times.to_string())],
        );
    } else if let Some(last) = rule
        .until
        .and_then(|until| Timestamp::from_millisecond(until).ok())
    {
        let last = last.to_zoned(zone.clone());
        said = fill(
            words.word("until"),
            &[
                ("every", &said),
                ("day", &last.day().to_string()),
                ("month", month(last.month(), words)),
                ("year", &last.year().to_string()),
            ],
        );
    }
    said
}
