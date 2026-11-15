//! A person's drawer, made ready here as every daemon's are
//! (ready-rows.md), so the shell merges nobody to show one: with sources
//! merged on the GUI thread, showing someone's drawer merged everyone for
//! their facts (Y2YBTS, 2026-11-10). It's their address, then the facts
//! each source says of them, in the order the sources are consulted, and
//! after a time zone the time there, left to the shell's clock to word.
//!
//! It's a record of its own, `drawer:people:<known>`, beside their row,
//! which names it, so a fact that changes, as Slack's presence does, is
//! told without the row.
//!
//! A source that's down says so, never what it last said, which may be
//! hours old (VKCF3Z, 2026-11-11): its facts give way to its name and
//! "not answering", as a source names itself in the launcher's words.
//!
//! Who they are, their title and then their team, is said under their
//! name rather than among their facts (VKCF3Z, a807ddcd): a quiet line
//! that gives way from its end, served here so the shell words no
//! drawer it's given (Y2YBTS, 2026-11-12). A fact is found for it by
//! the `wordsId` it says it is, never by its label, so renaming a word
//! keeps the line. Slack says its Title so; the pages source, kept
//! outside core.nix, says no `wordsId` yet, so a team only it knows
//! stays among the facts until it does.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::listing::{self, Person, Stands};
use crate::{Record, Standing};

/// The word for their address, in the launcher's vocabulary.
pub const EMAIL: &str = "people:email";
/// The word for the time where their zone is.
pub const THERE: &str = "people:there";
/// What a source's name goes by in the launcher's words, after the
/// source's own name, as `slack:person:source` is Slack's.
pub const SOURCE: &str = ":person:source";
/// What a fact says it is where it's their title, as Slack's Title is.
pub const TITLE: &str = "people:title";
/// What a fact says it is where it's their team.
pub const TEAM: &str = "people:team";
/// What joins their title and their team under their name.
const JOINT: &str = " \u{b7} ";

/// The names a drawer gives what it adds to the sources' facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    /// "Email", their address.
    pub email: String,
    /// "Time There", the time in their zone, after the zone.
    pub there: String,
    /// "not answering", said of a source that's down.
    pub silent: String,
    /// Each source's name, by the name it runs as, for one that's down.
    pub sources: BTreeMap<String, String>,
}

impl Named {
    /// As the launcher's vocabulary (words.json) names them, each by its
    /// id where it doesn't, so a drawer never says nothing.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Named {
        fn id(thing: &Value) -> Option<&str> {
            thing.get("id").and_then(Value::as_str)
        }
        fn named(thing: &Value) -> Option<String> {
            thing.get("name").and_then(Value::as_str).map(str::to_owned)
        }
        let things = vocabulary
            .get("things")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        let name = |id_of: &str| {
            things
                .iter()
                .find(|thing| id(thing) == Some(id_of))
                .and_then(named)
                .unwrap_or_else(|| id_of.to_owned())
        };
        Named {
            email: name(EMAIL),
            there: name(THERE),
            silent: listing::words(vocabulary)
                .remove("silent")
                .unwrap_or_else(|| "silent".to_owned()),
            sources: things
                .iter()
                .filter_map(|thing| {
                    Some((id(thing)?.strip_suffix(SOURCE)?.to_owned(), named(thing)?))
                })
                .collect(),
        }
    }
}

/// People's words and a drawer's names, from the launcher's vocabulary at
/// `path`, the daemon's WORDS; each said by its name where there's none,
/// or it can't be read.
#[must_use]
pub fn words_at(path: Option<&str>) -> (BTreeMap<String, String>, Named) {
    let read = path.map(|path| {
        std::fs::read_to_string(path)
            .map_err(|error| error.to_string())
            .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()))
    });
    match read {
        Some(Ok(vocabulary)) => (listing::words(&vocabulary), Named::read(&vocabulary)),
        Some(Err(why)) => {
            eprintln!("<4>couldn't read the launcher's words, so each is said by name: {why}");
            (BTreeMap::default(), Named::read(&Value::Null))
        }
        None => (BTreeMap::default(), Named::read(&Value::Null)),
    }
}

/// The key someone's drawer goes by, as their row names it.
#[must_use]
pub fn key(person: &Person) -> String {
    let known = person.key.strip_prefix("person:").unwrap_or(&person.key);
    format!("drawer:people:{known}")
}

/// A record as a search answers it, with the row it's found as, the one
/// it's listed as, the drawer the row names, and how each source has them
/// stand (listing.rs's `sourced`).
#[must_use]
pub fn found(
    record: &Record,
    sources: &[(String, Option<Stands>)],
    named: &Named,
    standings: &BTreeMap<String, Standing>,
) -> Value {
    let mut said = record.json();
    said["sources"] = listing::sourced(record, sources);
    let one = listing::person(record, sources);
    said["row"] = one.row();
    said["drawer"] = drawer(&one, record, sources, named, standings)["drawer"].take();
    said
}

/// Their drawer's record: their address, then each source's facts in the
/// order the sources are consulted, the first of each name, a time zone
/// said by its name and followed by the time there. A source signed out
/// or refused is down, as `Book::ready` takes it, and one the launcher
/// names says it's not answering in place of its facts.
#[must_use]
pub fn drawer(
    person: &Person,
    record: &Record,
    sources: &[(String, Option<Stands>)],
    named: &Named,
    standings: &BTreeMap<String, Standing>,
) -> Value {
    let mut facts: Vec<Value> = Vec::new();
    if !person.email.is_empty() {
        facts.push(json!({"name": named.email, "said": person.email}));
    }
    let down = |source: &str| {
        matches!(
            standings.get(source),
            Some(Standing::SignedOut | Standing::Refused(_))
        )
    };
    let told: Vec<Value> = sources
        .iter()
        .flat_map(|(source, _)| match named.sources.get(source) {
            Some(name) if down(source) => vec![json!({"name": name, "said": named.silent})],
            _ => record
                .sources
                .get(source)
                .and_then(|entry| entry.get("facts"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        })
        .collect();
    let mut seen: Vec<&Value> = Vec::new();
    for fact in &told {
        if let Some(name) = fact.get("name") {
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
        }
        match fact.get("zone").and_then(Value::as_str) {
            Some(zone) if fact.get("as").and_then(Value::as_str) == Some("zone") => {
                facts.push(json!({"name": fact["name"], "said": zone}));
                facts.push(json!({"name": named.there, "zone": zone, "as": "zone"}));
            }
            _ => facts.push(fact.clone()),
        }
    }
    let head = headed(&mut facts);
    let mut drawn = json!({"title": person.name, "facts": facts, "body": ""});
    if let Some(preview) = head {
        drawn["preview"] = preview;
    }
    json!({"key": key(person), "kind": "drawer", "drawer": drawn})
}

/// Their title and their team, each the first a source says, as the quiet
/// line under their name that gives way from its end, the team first;
/// both taken out of `facts`, since a drawer never says a thing twice.
/// None where no source says either.
fn headed(facts: &mut Vec<Value>) -> Option<Value> {
    let is = |fact: &Value, id: &str| fact.get("wordsId").and_then(Value::as_str) == Some(id);
    let first = |id: &str| {
        facts
            .iter()
            .filter(|fact| is(fact, id))
            .filter_map(|fact| fact.get("said").and_then(Value::as_str))
            .find(|said| !said.is_empty())
            .map(str::to_owned)
    };
    let items: Vec<String> = [first(TITLE), first(TEAM)].into_iter().flatten().collect();
    facts.retain(|fact| !is(fact, TITLE) && !is(fact, TEAM));
    (!items.is_empty()).then(|| json!({"lines": [items.join(JOINT)], "quiet": true, "gives": true}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listing::tests::{record, vocabulary};

    fn named() -> Named {
        Named::read(&vocabulary())
    }

    fn sources() -> Vec<(String, Option<Stands>)> {
        vec![
            ("slack".to_owned(), None),
            ("pages".to_owned(), None),
            ("tuple".to_owned(), None),
        ]
    }

    fn drawn(record: &Record) -> Value {
        drawn_while(record, &BTreeMap::new())
    }

    fn drawn_while(record: &Record, standings: &BTreeMap<String, Standing>) -> Value {
        drawer(
            &listing::person(record, &sources()),
            record,
            &sources(),
            &named(),
            standings,
        )
    }

    #[test]
    fn their_address_comes_first_then_each_sources_facts_in_order() {
        let ada = record(
            "person:ada@example.com",
            &[
                (
                    "tuple",
                    json!({"name": "Ada", "facts": [{"name": "Team", "said": "Calls"}]}),
                ),
                (
                    "slack",
                    json!({"facts": [
                        {"name": "Status", "said": "🌴 away"},
                        {"name": "Time Zone", "zone": "Asia/Kolkata", "as": "zone"}
                    ]}),
                ),
                (
                    "pages",
                    json!({"facts": [{"name": "Team", "said": "Shell"}]}),
                ),
            ],
        );
        assert_eq!(
            drawn(&ada),
            json!({
                "key": "drawer:people:ada@example.com", "kind": "drawer",
                "drawer": {"title": "Ada", "body": "", "facts": [
                    {"name": "Email", "said": "ada@example.com"},
                    {"name": "Status", "said": "🌴 away"},
                    {"name": "Time Zone", "said": "Asia/Kolkata"},
                    {"name": "Time There", "zone": "Asia/Kolkata", "as": "zone"},
                    {"name": "Team", "said": "Shell"},
                ]},
            }),
            "Slack's facts first, as it's consulted first, the time there after the zone, and \
             the first source's Team over Tuple's"
        );
    }

    #[test]
    fn a_source_down_says_so_rather_than_what_it_said_last() {
        let ada = record(
            "person:ada@example.com",
            &[
                (
                    "slack",
                    json!({"facts": [{"name": "Presence", "said": "active"}]}),
                ),
                (
                    "tuple",
                    json!({"facts": [{"name": "Team", "said": "Calls"}]}),
                ),
            ],
        );
        let down = BTreeMap::from([
            (
                "slack".to_owned(),
                Standing::Refused("it stopped".to_owned()),
            ),
            ("tuple".to_owned(), Standing::SignedOut),
        ]);
        assert_eq!(
            drawn_while(&ada, &down)["drawer"]["facts"],
            json!([
                {"name": "Email", "said": "ada@example.com"},
                {"name": "Slack", "said": "not answering"},
                {"name": "Team", "said": "Calls"},
            ]),
            "Slack, named in the launcher's words, says it's not answering; Tuple, named by \
             none, keeps what it said"
        );
        let up = BTreeMap::from([("slack".to_owned(), Standing::Answering)]);
        assert_eq!(
            drawn_while(&ada, &up)["drawer"]["facts"][1],
            json!({"name": "Presence", "said": "active"}),
            "once it answers, what it says"
        );
    }

    #[test]
    fn who_they_are_goes_under_their_name_by_what_each_fact_says_it_is() {
        let ada = record(
            "person:ada@example.com",
            &[
                (
                    "slack",
                    json!({"facts": [
                        {"name": "Title", "said": "Analyst", "wordsId": TITLE},
                        {"name": "Status", "said": "\u{1f334} away"}
                    ]}),
                ),
                (
                    "pages",
                    json!({"facts": [
                        {"name": "Role", "said": "Engineer", "wordsId": TITLE},
                        {"name": "Squad", "said": "Shell", "wordsId": TEAM}
                    ]}),
                ),
            ],
        );
        let drawer = drawn(&ada)["drawer"].take();
        assert_eq!(
            (drawer["preview"].clone(), drawer["facts"].clone()),
            (
                json!({"lines": ["Analyst \u{b7} Shell"], "quiet": true, "gives": true}),
                json!([
                    {"name": "Email", "said": "ada@example.com"},
                    {"name": "Status", "said": "\u{1f334} away"},
                ]),
            ),
            "Slack's title, as it's asked first, then the team, whatever each is labelled, \
             and neither said again among the facts"
        );
    }

    #[test]
    fn a_fact_is_under_their_name_only_by_what_it_says_it_is() {
        let ada = record(
            "person:ada@example.com",
            &[(
                "pages",
                json!({"facts": [
                    {"name": "Title", "said": "", "wordsId": TITLE},
                    {"name": "Team", "said": "Shell"}
                ]}),
            )],
        );
        let drawer = drawn(&ada)["drawer"].take();
        assert_eq!(
            (drawer.get("preview"), drawer["facts"].clone()),
            (
                None,
                json!([
                    {"name": "Email", "said": "ada@example.com"},
                    {"name": "Team", "said": "Shell"},
                ]),
            ),
            "a title saying nothing makes no line, and a team saying no wordsId stays a fact"
        );
    }

    #[test]
    fn one_known_by_no_address_says_none() {
        let gus = record(
            "person:tuple:7",
            &[("tuple", json!({"name": "Gus", "id": 7}))],
        );
        assert_eq!(
            (
                drawn(&gus)["key"].clone(),
                drawn(&gus)["drawer"]["facts"].clone()
            ),
            (json!("drawer:people:tuple:7"), json!([])),
            "known by Tuple's id, as their row names their drawer, with nothing to say"
        );
    }

    #[test]
    fn a_drawers_names_are_the_launchers_or_their_ids() {
        // The launcher's words, less one, to see a word it lacks said by
        // its id.
        let mut lacking = vocabulary();
        if let Some(things) = lacking["things"].as_array_mut() {
            things.retain(|thing| thing["id"] != THERE);
        }
        assert_eq!(
            Named::read(&lacking),
            Named {
                email: "Email".into(),
                there: THERE.into(),
                silent: "not answering".into(),
                sources: BTreeMap::from([("slack".to_owned(), "Slack".to_owned())]),
            },
            "the launcher's words, a source named by its own, and a word it lacks said by its id"
        );
    }
}
