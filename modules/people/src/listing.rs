//! Those People lists with nothing typed, made here rather than on the
//! shell's GUI thread: each record as one person, how they stand by every
//! source that says, whether they're listed, and in the order things
//! returned to keep (the shell's Order.js, `returned`). Merging, standing
//! and ordering 600 people there took 26 to 38 ms of every change to any
//! source's people, open or shut, and listing them as much again
//! (2026-11-10), so the shell now maps what this says to rows by key.
//!
//! A record is one person already: the book keys everyone by their
//! address, or by a source's own id for one it knows by no address, as the
//! shell's Person.js joins them, and no source says more than one address
//! for anyone.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use ready_rows::kept::{Kept, Moved};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::drawer::{self, Named};
use crate::name::{self, bare};
use crate::{Book, LISTING, Record, address};

/// How a source's entry says someone stands, as its module tells the
/// daemon in the sources file: the entry's field that says it, which of
/// People's words each word it says is and its order, whether a word it
/// says that isn't among them is said as it is (Slack's presence) or not
/// at all (Tuple's "unknown"), and the field holding a mark to say after
/// it (Slack's status emoji). A source with none says nothing of how
/// anyone stands, as Google doesn't. People's words themselves are the
/// launcher's (words.json), handed to the daemon, so no module words them.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Stands {
    pub field: String,
    #[serde(default)]
    pub words: BTreeMap<String, Said>,
    #[serde(default)]
    pub others: bool,
    #[serde(default)]
    pub mark: Option<String>,
    /// People's words, by the names `words` gives them (`worded`).
    #[serde(skip)]
    pub said: BTreeMap<String, String>,
}

impl Stands {
    /// These stands, saying People's words as the launcher words them.
    #[must_use]
    pub fn worded(mut self, said: &BTreeMap<String, String>) -> Stands {
        self.said.clone_from(said);
        self
    }
}

/// Which of People's words a source's word is, by its name in words.json
/// (`available`, `call`), and its place in the order of who can be
/// reached first: available 0, busy 1, away 2, offline 3.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Said {
    pub word: String,
    pub order: u8,
}

/// People's words as the launcher's vocabulary (words.json) says them:
/// what its `people` thing says, by name, and none when it says nothing.
pub fn words(vocabulary: &Value) -> BTreeMap<String, String> {
    vocabulary
        .get("things")
        .and_then(Value::as_array)
        .and_then(|things| {
            things
                .iter()
                .find(|thing| thing.get("id").and_then(Value::as_str) == Some("people"))
        })
        .and_then(|people| people.get("said"))
        .and_then(Value::as_object)
        .map(|said| {
            said.iter()
                .filter_map(|(name, word)| Some((name.clone(), word.as_str()?.to_owned())))
                .collect()
        })
        .unwrap_or_default()
}

/// How someone stands, as Person.js's `standing` says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stood {
    pub said: String,
    pub order: u8,
    pub mark: Option<String>,
    pub favourite: bool,
    pub recent: bool,
}

/// The order of someone nobody says anything of, after everyone who's
/// said to stand anyhow.
pub const UNSAID: u8 = 4;

/// How much each order limits reaching them, by order: available 0, busy
/// 1, away 2, offline 3, unsaid 4. What most limits reaching them is said.
const LIMITS: [u8; 5] = [2, 4, 3, 1, 0];

impl Stood {
    fn unsaid() -> Stood {
        Stood {
            said: String::new(),
            order: UNSAID,
            mark: None,
            favourite: false,
            recent: false,
        }
    }

    fn limit(&self) -> u8 {
        if self.said.is_empty() {
            0
        } else {
            LIMITS.get(usize::from(self.order)).copied().unwrap_or(0)
        }
    }
}

fn text<'a>(entry: &'a Value, field: &str) -> Option<&'a str> {
    entry
        .get(field)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

fn yes(entry: &Value, field: &str) -> bool {
    entry.get(field).and_then(Value::as_bool) == Some(true)
}

/// How one source's entry says someone stands, or nothing where it says
/// nothing: a source that says how anyone stands says it of everyone it
/// has a word or a mark for, or calls a favourite or dealt with lately.
pub fn stood(entry: &Value, stands: &Stands) -> Option<Stood> {
    let word = text(entry, &stands.field);
    let mark = stands
        .mark
        .as_deref()
        .and_then(|field| text(entry, field))
        .map(str::to_owned);
    let (favourite, recent) = (yes(entry, "favourite"), yes(entry, "recent"));
    if word.is_none() && mark.is_none() && !favourite && !recent {
        return None;
    }
    let (said, order) = match word.map(|word| (word, stands.words.get(word))) {
        Some((_, Some(said))) => (
            stands
                .said
                .get(&said.word)
                .cloned()
                .unwrap_or_else(|| said.word.clone()),
            said.order,
        ),
        Some((word, None)) if stands.others => (word.to_owned(), UNSAID),
        _ => (String::new(), UNSAID),
    };
    Some(Stood {
        said,
        order,
        mark,
        favourite,
        recent,
    })
}

/// How someone stands by every source that says, in the order they're
/// consulted: what most limits reaching them, a tie to the source asked
/// first, the first mark after it, and a favourite or recent if any says
/// so.
pub fn standing(said: &[Stood]) -> Stood {
    let Some(first) = said.first() else {
        return Stood::unsaid();
    };
    let most = said.iter().fold(first, |held, stood| {
        if stood.limit() > held.limit() {
            stood
        } else {
            held
        }
    });
    let mark = said.iter().find_map(|stood| stood.mark.clone());
    let words: Vec<&str> = [Some(most.said.as_str()), mark.as_deref()]
        .into_iter()
        .flatten()
        .filter(|word| !word.is_empty())
        .collect();
    Stood {
        said: words.join(" "),
        order: if most.said.is_empty() {
            UNSAID
        } else {
            most.order
        },
        mark,
        favourite: said.iter().any(|stood| stood.favourite),
        recent: said.iter().any(|stood| stood.recent),
    }
}

/// When one thing with someone was, from its start to its end, in
/// milliseconds since the epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub from: f64,
    pub to: f64,
}

/// Someone as People lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    /// The record's key, `person:` and what they're known by.
    pub key: String,
    pub email: String,
    pub name: String,
    pub recent: bool,
    pub spans: Vec<Span>,
    /// The sources that know them, in the order they're consulted.
    pub from: Vec<String>,
    pub stood: Stood,
    /// Their time zone by tzdata's name, the first a source's facts give,
    /// or "" for none.
    pub zone: String,
}

/// A record as one person, by its sources in the order they're consulted,
/// each with how it says anyone stands.
pub fn person(record: &Record, sources: &[(String, Option<Stands>)]) -> Person {
    let email = address(&record.key)
        .filter(|kept| kept.contains('@'))
        .unwrap_or("")
        .to_owned();
    let known = record.key.strip_prefix("person:").unwrap_or(&record.key);
    let entries: Vec<(&String, &Value, Option<&Stands>)> = sources
        .iter()
        .filter_map(|(name, stands)| {
            record
                .sources
                .get(name)
                .map(|entry| (name, entry, stands.as_ref()))
        })
        .collect();
    let name = entries
        .iter()
        .filter_map(|(_, entry, _)| entry.get("name").and_then(Value::as_str))
        .map(str::trim)
        .find(|name| !bare(name))
        .map_or_else(
            || {
                if email.is_empty() {
                    known.to_owned()
                } else {
                    name::by_address(&email)
                }
            },
            str::to_owned,
        );
    let spans = entries
        .iter()
        .filter_map(|(_, entry, _)| entry.get("spans").and_then(Value::as_array))
        .flatten()
        .filter_map(|span| {
            let from = span.get("from").and_then(Value::as_f64)?;
            let to = span.get("to").and_then(Value::as_f64)?;
            (from.is_finite() && to.is_finite()).then_some(Span { from, to })
        })
        .collect();
    let said: Vec<Stood> = entries
        .iter()
        .filter_map(|(_, entry, stands)| stands.and_then(|stands| stood(entry, stands)))
        .collect();
    Person {
        key: record.key.clone(),
        email,
        name,
        recent: entries.iter().any(|(_, entry, _)| yes(entry, "recent")),
        spans,
        from: entries.iter().map(|(name, ..)| (*name).clone()).collect(),
        stood: standing(&said),
        zone: entries
            .iter()
            .filter_map(|(_, entry, _)| entry.get("facts").and_then(Value::as_array))
            .flatten()
            .filter(|fact| fact.get("as").and_then(Value::as_str) == Some("zone"))
            .find_map(|fact| fact.get("zone").and_then(Value::as_str))
            .unwrap_or("")
            .to_owned(),
    }
}

impl Person {
    /// When the last thing between them and Joel was: the end of a meeting
    /// they had, now for one under way, or when a mail came; or nothing
    /// when there's been nothing yet, as for someone only asked to a
    /// meeting to come.
    pub fn last(&self, now: f64) -> Option<f64> {
        self.spans
            .iter()
            .filter(|span| span.from <= now)
            .map(|span| span.to.min(now))
            .reduce(f64::max)
    }

    /// Whether they're listed with nothing typed: a favourite, someone
    /// there's been something with, or someone a source says Joel dealt
    /// with lately.
    pub fn listed(&self, now: f64) -> bool {
        self.stood.favourite || self.stood.recent || self.recent || self.last(now).is_some()
    }

    /// As the shell reads them: Person.js's person and how they stand.
    /// When the last thing with them was isn't said, since one under way
    /// moves with the clock, and would be said again every time.
    pub fn json(&self) -> Value {
        let spans: Vec<Value> = self
            .spans
            .iter()
            .map(|span| json!({"from": span.from, "to": span.to}))
            .collect();
        let emails: Vec<&str> = if self.email.is_empty() {
            Vec::new()
        } else {
            vec![self.email.as_str()]
        };
        json!({
            "key": self.key.strip_prefix("person:").unwrap_or(&self.key),
            "email": self.email, "emails": emails, "name": self.name, "recent": self.recent,
            "spans": spans, "from": self.from,
            "stood": {
                "said": self.stood.said, "order": self.stood.order,
                "favourite": self.stood.favourite, "recent": self.stood.recent,
            },
        })
    }

    /// Their row as the launcher draws it, ready, in the form every
    /// daemon's rows take (ready-rows.md): its key, the person it is as
    /// `thing`, which Joel's picks remember, name, glyph and hue by the
    /// theme's roles, how they stand as its side's items, their address
    /// under it unless that's their name, and the key their drawer is told
    /// by. So the shell words and builds nothing of it. Their zone ends the
    /// side, as a time the shell's clock says, and only where it's apart
    /// from Joel's (VKCF3Z, 0cea8a93: "London · 17:02").
    pub fn row(&self) -> Value {
        let known = self.key.strip_prefix("person:").unwrap_or(&self.key);
        let mut side: Vec<Value> = Vec::new();
        if !self.stood.said.is_empty() {
            side.push(json!({"said": self.stood.said}));
        }
        if !self.zone.is_empty() {
            side.push(json!({"zone": self.zone, "as": "zone"}));
        }
        json!({
            "key": format!("row:people:{known}"), "thing": self.key,
            "place": "people", "kind": "person", "name": self.name,
            "glyph": if self.stood.favourite { "favourite" } else { "person" },
            "hue": "person",
            "side": side,
            "note": if self.name == self.email { "" } else { self.email.as_str() },
            "drawer": drawer::key(self),
        })
    }

    /// As `row`, found among everyone by a search, so saying nothing of how they stand.
    pub fn found(&self) -> Value {
        let mut row = self.row();
        row["glyph"] = json!("person");
        row["side"] = json!([]);
        row
    }
}

/// Those listed, in the order things returned to keep: favourites first,
/// then by the last thing with each, the newest first, those with nothing
/// yet after, then A to Z, and those alike in all three in the order heard.
pub fn ordered(persons: &[Person], now: f64) -> Vec<&Person> {
    /// Someone listed, with what they're ordered by, each measured once.
    struct Measured<'a> {
        favourite: bool,
        last: Option<f64>,
        pieces: Vec<String>,
        heard: usize,
        person: &'a Person,
    }
    let mut measured: Vec<Measured> = persons
        .iter()
        .enumerate()
        .filter(|(_, person)| person.listed(now))
        .map(|(heard, person)| Measured {
            favourite: person.stood.favourite,
            last: person.last(now),
            pieces: name::pieces(&person.name),
            heard,
            person,
        })
        .collect();
    measured.sort_by(|one, other| {
        other
            .favourite
            .cmp(&one.favourite)
            .then_with(|| match (one.last, other.last) {
                (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| name::by_pieces(&one.pieces, &other.pieces))
            .then_with(|| one.heard.cmp(&other.heard))
    });
    measured.into_iter().map(|one| one.person).collect()
}

/// When who's listed, and their order, next change with the clock alone:
/// the first moment after now that something with someone starts or
/// ends, or nothing when nothing will.
pub fn changes_at(persons: &[Person], now: f64) -> Option<f64> {
    persons
        .iter()
        .flat_map(|person| &person.spans)
        .map(|span| if span.from > now { span.from } else { span.to })
        .filter(|at| *at > now)
        .reduce(f64::min)
}

/// What a watch of those listed was last told, kept as every daemon keeps
/// a place's rows (ready-rows' `Kept`), so it's told only what changed.
#[derive(Debug, Default)]
pub struct Told {
    kept: Kept,
    /// When the clock alone next changes who's listed or their order.
    pub until: Option<f64>,
}

impl Told {
    /// Those listed now: each one's row in the order shown, its `order`
    /// kept where it still rises, so one person moving is told alone, and
    /// beside it their record, as one person with what each source said,
    /// which the shell reads for who they are, and their drawer. Answered
    /// with the records that read differently and the keys that went.
    pub fn changes(
        &mut self,
        book: &Book,
        sources: &[(String, Option<Stands>)],
        named: &Named,
        now: f64,
    ) -> Moved {
        let persons: Vec<Person> = book
            .everyone()
            .map(|record| person(record, sources))
            .collect();
        let listed = ordered(&persons, now);
        self.until = changes_at(&persons, now);
        let rows = listed.iter().map(|person| person.row()).collect();
        let beside = listed
            .iter()
            .filter_map(|person| book.record(&person.key).map(|record| (person, record)))
            .flat_map(|(person, record)| {
                [
                    json!({
                        "key": person.key, "kind": LISTING, "person": person.json(),
                        "sources": record.sources,
                    }),
                    drawer::drawer(person, record, sources, named, book.standings()),
                ]
            })
            .collect();
        self.kept.take(rows, beside)
    }

    /// Everything told, as a listing is answered whole.
    pub fn all(&self) -> Vec<Value> {
        self.kept.all().cloned().collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The launcher's own words for what People's cases say
    /// (tests/words.json), which `the-daemons-test-the-launchers-words`
    /// holds to words.json, so a case holds the daemon to the words Joel
    /// reads. drawer.rs's tests take them too.
    pub(crate) fn vocabulary() -> Value {
        serde_json::from_str(include_str!("../tests/words.json"))
            .expect("the launcher's words parse")
    }

    fn tuple() -> Stands {
        serde_json::from_value::<Stands>(json!({
            "field": "status",
            "words": {
                "online": {"word": "available", "order": 0},
                "busy": {"word": "call", "order": 1},
                "unavailable": {"word": "away", "order": 2},
                "offline": {"word": "offline", "order": 3}
            }
        }))
        .unwrap()
        .worded(&words(&vocabulary()))
    }

    fn slack() -> Stands {
        serde_json::from_value::<Stands>(json!({
            "field": "presence",
            "words": {
                "active": {"word": "active", "order": 0},
                "in a huddle": {"word": "huddle", "order": 1},
                "away": {"word": "away", "order": 2}
            },
            "others": true,
            "mark": "emoji"
        }))
        .unwrap()
        .worded(&words(&vocabulary()))
    }

    #[test]
    fn peoples_words_are_the_launchers() {
        // A number among People's words, which the launcher never says.
        let mut counted = vocabulary();
        for thing in counted["things"].as_array_mut().into_iter().flatten() {
            if thing["id"] == "people" {
                thing["said"]["count"] = json!(3);
            }
        }
        let said = words(&counted);
        assert_eq!(
            (said.get("call").map(String::as_str), said.get("count")),
            (Some("in a call"), None),
            "People's own words, and nothing that isn't a word"
        );
        assert!(
            words(&json!({"things": []})).is_empty(),
            "no People, no words"
        );
        let unworded: Stands = serde_json::from_value(json!({
            "field": "status", "words": {"busy": {"word": "call", "order": 1}}
        }))
        .unwrap();
        assert_eq!(
            stood(&json!({"status": "busy"}), &unworded).map(|stood| stood.said),
            Some("call".to_owned()),
            "a word the launcher doesn't say is said by its name, never as nothing"
        );
    }

    fn sources() -> Vec<(String, Option<Stands>)> {
        vec![
            ("tuple".to_owned(), Some(tuple())),
            ("slack".to_owned(), Some(slack())),
            ("google".to_owned(), None),
        ]
    }

    /// A person's record holding each source's entry, as drawer.rs's
    /// tests take it too.
    pub(crate) fn record(key: &str, entries: &[(&str, Value)]) -> Record {
        let mut record = Record::of(key.to_owned(), 0);
        for (source, entry) in entries {
            record.sources.insert((*source).to_owned(), entry.clone());
        }
        record
    }

    #[test]
    fn a_source_says_how_someone_stands_in_peoples_words() {
        let tuple_said = stood(&json!({"status": "busy", "favourite": true}), &tuple()).unwrap();
        assert_eq!(
            (
                tuple_said.said.as_str(),
                tuple_said.order,
                tuple_said.favourite
            ),
            ("in a call", 1, true),
            "Tuple's busy is in a call, second in order"
        );
        let unknown = stood(&json!({"status": "unknown"}), &tuple()).unwrap();
        assert_eq!(
            (unknown.said.as_str(), unknown.order),
            ("", UNSAID),
            "a word Tuple says that People doesn't know says nothing"
        );
        let slack_said = stood(&json!({"presence": "lurking", "emoji": "🌴"}), &slack()).unwrap();
        assert_eq!(
            (
                slack_said.said.as_str(),
                slack_said.order,
                slack_said.mark.as_deref()
            ),
            ("lurking", UNSAID, Some("🌴")),
            "a word Slack says that People doesn't know is said as it is, with its mark"
        );
        assert_eq!(
            stood(&json!({"name": "Ada"}), &slack()),
            None,
            "no word, no mark, no stand"
        );
        let favourite = stood(&json!({"status": "unknown", "favourite": true}), &tuple());
        assert_eq!(
            favourite.map(|stood| (stood.order, stood.favourite)),
            Some((UNSAID, true)),
            "a favourite stands as one, whatever is said of them"
        );
    }

    #[test]
    fn what_most_limits_reaching_them_is_said_with_the_first_mark() {
        let person = person(
            &record(
                "person:ada@example.com",
                &[
                    ("tuple", json!({"status": "online"})),
                    ("slack", json!({"presence": "in a huddle", "emoji": "🎧"})),
                ],
            ),
            &sources(),
        );
        assert_eq!(
            (person.stood.said.as_str(), person.stood.order),
            ("in a huddle 🎧", 1),
            "a huddle limits more than being available, and its mark follows"
        );
        let nobody = person_of(&[("google", json!({"name": "Bo"}))]);
        assert_eq!(
            nobody.stood,
            Stood::unsaid(),
            "a source that says nothing stands nobody"
        );
    }

    fn person_of(entries: &[(&str, Value)]) -> Person {
        person(&record("person:bo@example.com", entries), &sources())
    }

    #[test]
    fn the_first_real_name_a_source_gives_is_theirs() {
        let named = person_of(&[
            ("tuple", json!({"name": "bo@example.com"})),
            ("slack", json!({"name": " Bo Diddley "})),
        ]);
        assert_eq!(named.name, "Bo Diddley", "an address loses to a real name");
        let bare = person_of(&[("google", json!({"name": "bo@example.com"}))]);
        assert_eq!(bare.name, "Bo", "with no name, one from their address");
        let by_id = person(
            &record("person:tuple:7", &[("tuple", json!({"id": 7}))]),
            &sources(),
        );
        assert_eq!(
            (by_id.name.as_str(), by_id.email.as_str()),
            ("tuple:7", ""),
            "one known only by a source's id is known by it"
        );
    }

    #[test]
    fn their_row_is_ready_to_draw() {
        let favourite = person_of(&[(
            "tuple",
            json!({"name": "Bo Diddley", "status": "busy", "favourite": true}),
        )]);
        assert_eq!(
            favourite.row(),
            json!({
                "key": "row:people:bo@example.com", "thing": "person:bo@example.com",
                "place": "people", "kind": "person", "name": "Bo Diddley", "glyph": "favourite", "hue": "person",
                "side": [{"said": "in a call"}], "note": "bo@example.com",
                "drawer": "drawer:people:bo@example.com",
            }),
            "a favourite's row wears the star, says how they stand and their address"
        );
        let bare = person_of(&[("google", json!({}))]);
        assert_eq!(
            (
                bare.row()["glyph"].clone(),
                bare.row()["name"].clone(),
                bare.row()["note"].clone()
            ),
            (json!("person"), json!("Bo"), json!("bo@example.com")),
            "one no source names is named from their address, and says it under"
        );
        let away = person_of(&[(
            "slack",
            json!({"facts": [{"name": "Time Zone", "zone": "Europe/London", "as": "zone"}]}),
        )]);
        assert_eq!(
            away.row()["side"],
            json!([{"zone": "Europe/London", "as": "zone"}]),
            "their zone is the last of their side, for the shell's clock to say"
        );
        let found = favourite.found();
        assert_eq!(
            (found["glyph"].clone(), found["side"].clone()),
            (json!("person"), json!([])),
            "found among everyone, they say nothing of how they stand"
        );
    }

    fn span(from: f64, to: f64) -> Value {
        json!({"from": from, "to": to})
    }

    #[test]
    fn those_listed_are_favourites_then_newest_then_by_name() {
        let now = 1_000.0;
        let persons: Vec<Person> = [
            (
                "person:a@x",
                vec![(
                    "google",
                    json!({"name": "Tab 10", "spans": [span(100.0, 200.0)]}),
                )],
            ),
            (
                "person:b@x",
                vec![(
                    "google",
                    json!({"name": "Tab 9", "spans": [span(100.0, 200.0)]}),
                )],
            ),
            (
                "person:c@x",
                vec![(
                    "google",
                    json!({"name": "Newest", "spans": [span(900.0, 2_000.0)]}),
                )],
            ),
            (
                "person:d@x",
                vec![(
                    "tuple",
                    json!({"name": "Fave", "status": "offline", "favourite": true}),
                )],
            ),
            (
                "person:e@x",
                vec![(
                    "google",
                    json!({"name": "To Come", "spans": [span(5_000.0, 6_000.0)]}),
                )],
            ),
            (
                "person:f@x",
                vec![(
                    "tuple",
                    json!({"name": "Lately", "status": "online", "recent": true}),
                )],
            ),
        ]
        .iter()
        .map(|(key, entries)| person(&record(key, entries), &sources()))
        .collect();
        let names: Vec<&str> = ordered(&persons, now)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["Fave", "Newest", "Tab 9", "Tab 10", "Lately"],
            "a favourite first, then the newest thing (a meeting under way as now), numbers by \
             value, someone dealt with lately but with nothing timed last, and nobody only to come"
        );
        assert_eq!(
            changes_at(&persons, now),
            Some(2_000.0),
            "the next edge is the meeting under way ending"
        );
    }
}
