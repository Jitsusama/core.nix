//! Everyone Joel's meetings and mail name, as a People source the people
//! daemon runs (google-people, src/bin): each who held or was asked to a
//! meeting from a month ago to a week on, and each who sent mail that
//! wants him, with when each thing with them was, as the shell's
//! Google.js `people` finds them. The daemon then lists and orders them
//! with everyone else's people, rather than the shell doing it on its GUI
//! thread at every change (modules/people/src/listing.rs).

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// Someone the meetings and mail name: the first name given them, and
/// when each thing with them was, from its start to its end, in
/// milliseconds since the epoch; a mail's is when it came.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Met {
    pub name: String,
    pub spans: Vec<(i64, i64)>,
}

fn of<'a>(records: &'a BTreeMap<String, Value>, kind: &'a str) -> Vec<&'a Value> {
    records
        .values()
        .filter(|record| record.get("kind").and_then(Value::as_str) == Some(kind))
        .filter_map(|record| record.get(kind))
        .filter(|thing| thing.is_object())
        .collect()
}

fn number(thing: &Value, field: &str) -> Option<i64> {
    thing.get(field).and_then(Value::as_i64)
}

fn text<'a>(thing: &'a Value, field: &str) -> &'a str {
    thing.get(field).and_then(Value::as_str).unwrap_or("")
}

/// Everyone the daemon's records name, by address in lower case: the
/// meetings held in the month before today, then those from today to a
/// week on, each by its start, then the mail that wants Joel, newest
/// first, so the first name given anyone is the one the shell gave.
pub fn people(records: &BTreeMap<String, Value>) -> BTreeMap<String, Met> {
    let mut found: BTreeMap<String, Met> = BTreeMap::new();
    let mut add = |email: &str, name: &str, span: (i64, i64)| {
        let address = email.trim().to_lowercase();
        if address.is_empty() {
            return;
        }
        found
            .entry(address)
            .and_modify(|met| met.spans.push(span))
            .or_insert_with(|| Met {
                name: name.to_owned(),
                spans: vec![span],
            });
    };
    let by_start = |kind| {
        let mut meetings = of(records, kind);
        meetings.sort_by_key(|meeting| number(meeting, "start"));
        meetings
    };
    for meeting in by_start("held").into_iter().chain(by_start("meeting")) {
        let (Some(start), Some(end)) = (number(meeting, "start"), number(meeting, "end")) else {
            continue;
        };
        let asked = meeting.get("organizer").into_iter().chain(
            meeting
                .get("guests")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        );
        for person in asked {
            add(text(person, "email"), text(person, "name"), (start, end));
        }
    }
    // Mail done is still a mail record, saying how it went, so what wants
    // Joel is the mail without a `done` (src/done.rs).
    let mut mail: Vec<&Value> = of(records, "mail")
        .into_iter()
        .filter(|mail| mail.get("done").is_none_or(Value::is_null))
        .collect();
    mail.sort_by_key(|mail| std::cmp::Reverse(number(mail, "at")));
    for one in mail {
        if let Some(at) = number(one, "at") {
            add(text(one, "address"), text(one, "from"), (at, at));
        }
    }
    found
}

/// Someone as a line of People's protocol.
pub fn line(email: &str, met: &Met) -> Value {
    let spans: Vec<Value> = met
        .spans
        .iter()
        .map(|(from, to)| json!({"from": from, "to": to}))
        .collect();
    json!({"email": email, "name": met.name, "spans": spans})
}

/// The lines saying what changed from those told to those found: each
/// found who's new or changed, and each told who's no longer found, gone.
pub fn changes(told: &BTreeMap<String, Met>, found: &BTreeMap<String, Met>) -> Vec<Value> {
    let changed = found
        .iter()
        .filter(|(email, met)| told.get(*email) != Some(met))
        .map(|(email, met)| line(email, met));
    let gone = told
        .keys()
        .filter(|email| !found.contains_key(*email))
        .map(|email| json!({"email": email, "gone": true}));
    changed.chain(gone).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(things: &[(&str, Value)]) -> BTreeMap<String, Value> {
        things
            .iter()
            .enumerate()
            .map(|(at, (kind, thing))| {
                let key = format!("{kind}:{at}");
                (key.clone(), json!({"key": key, "kind": kind, *kind: thing}))
            })
            .collect()
    }

    #[test]
    fn everyone_a_meeting_asks_or_mail_wanting_joel_names_is_someone() {
        let found = people(&records(&[
            (
                "meeting",
                json!({"start": 300, "end": 400, "organizer": {"name": "Ada", "email": "Ada@X"},
                       "guests": [{"name": "Bo", "email": "bo@x"}, {"name": "Room", "email": null}]}),
            ),
            (
                "held",
                json!({"start": 100, "end": 200, "organizer": null,
                       "guests": [{"name": "ada@x", "email": "ada@x"}]}),
            ),
            ("mail", json!({"address": "cy@x", "from": "Cy", "at": 50})),
            (
                "mail",
                json!({"address": "di@x", "from": "Di", "at": 60, "done": {"how": "read"}}),
            ),
            ("file", json!({"owner": "ed@x"})),
        ]));
        assert_eq!(
            found.keys().collect::<Vec<_>>(),
            ["ada@x", "bo@x", "cy@x"],
            "the asked and those mailing, by address in lower case, never mail done or files"
        );
        assert_eq!(
            found["ada@x"],
            Met {
                name: "ada@x".to_owned(),
                spans: vec![(100, 200), (300, 400)]
            },
            "a meeting held comes first, so its name for her is the one kept, as the shell's"
        );
        assert_eq!(
            found["cy@x"].spans,
            [(50, 50)],
            "a mail's time is when it came"
        );
    }

    #[test]
    fn only_whoever_changed_is_said_and_whoever_went_gone() {
        let met = |name: &str| Met {
            name: name.to_owned(),
            spans: vec![(1, 2)],
        };
        let told = BTreeMap::from([("a@x".to_owned(), met("A")), ("b@x".to_owned(), met("B"))]);
        let found = BTreeMap::from([("a@x".to_owned(), met("A")), ("c@x".to_owned(), met("C"))]);
        assert_eq!(
            changes(&told, &found),
            [
                json!({"email": "c@x", "name": "C", "spans": [{"from": 1, "to": 2}]}),
                json!({"email": "b@x", "gone": true})
            ]
        );
    }
}
