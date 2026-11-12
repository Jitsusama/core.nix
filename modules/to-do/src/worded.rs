//! Each to-do worded as For You says it, beside its item on its record
//! (stream.rs), so the agents daemon makes For You's rows from words
//! already said and the shell words only its acts, by their keys, as it
//! words every served act (Y2YBTS, 2026-11-10: the to-do
//! daemon serves each to-do ready, the agents daemon joins it with its
//! session, quest and window, weighs it and serves the rows). It's what
//! Now.qml's `item`, `drawerOf` and NeedsYou.js's `note` say from the
//! to-do alone, in their order:
//!
//! - `name`, `glyph` and `hue`: its title, the mark of what waits on Joel,
//!   and strong while he can do it now, else muted;
//! - `side`: its owner gone, then for one not ready what it waits for,
//!   else how long it takes, as Later's rows say it, unweighed;
//! - `parts`: what its note is made of, which only the agents daemon can
//!   finish, since it alone knows the side a weighed row shows and
//!   whether the session that raised it still runs: `when`, its owner's
//!   words for when; `nobody`, said when nothing is watching it, and
//!   `seen`, whether this daemon will see it done by itself; `why`, said
//!   when nothing else is, in a file older than `mesh_wait`'s; `waited`,
//!   how long it's waited, a time;
//! - `drawer`: its title, its facts and its body, its steps numbered then
//!   why only Joel can do it; each fact names its id, so the agents
//!   daemon lays its own (weight, where, session) among them in place;
//! - `acts`: Mark It Done and Put It Off, each an ask back on this socket
//!   under its verb's own key, which the shell words (ready-rows.md's
//!   `{key, ask}`), the ask carrying which to-do.
//!
//! A time is `{at, as}` for the shell to word against its own clock, so a
//! record changes only when its to-do does.

use serde_json::{Value, json};

use crate::todo::as_javascript;
use crate::words::Words;

/// What For You marks a to-do with, a glyph role in the theme.
const GLYPH: &str = "waitsOnJoel";

/// The acts on a to-do this daemon answers, by their ids, Return's first.
const ACTS: [&str; 2] = ["done", "notNow"];

/// A to-do's words, laid on its record: `key` is its identity, `now`
/// whether Joel can do it now.
pub fn worded(record: &mut Value, item: &Value, key: &str, now: bool, words: &Words) {
    let owner = owner(item, words);
    let waits = waits(item, words);
    let minutes = item
        .get("minutes")
        .filter(|minutes| minutes.is_number())
        .map(|minutes| as_javascript(minutes).to_string());
    let side = [
        owner.clone(),
        match (&waits, &minutes) {
            (Some(waits), _) if !now => Some(waits.clone()),
            (_, Some(minutes)) => Some(words.said("minutes", &[("minutes", minutes)])),
            _ => None,
        },
    ]
    .into_iter()
    .flatten()
    .map(|said| json!({"said": said}))
    .collect::<Vec<_>>();
    record["name"] = item["title"].clone();
    record["glyph"] = json!(GLYPH);
    record["hue"] = json!(if now { "strong" } else { "muted" });
    record["side"] = json!(side);
    record["parts"] = parts(item, words);
    record["drawer"] = json!({
        "title": item["title"],
        "facts": facts(item, key, minutes.as_deref(), words),
        "body": body(item),
    });
    record["acts"] = ACTS
        .iter()
        .map(|act| {
            json!({
                "key": format!("verb:on:waiting:{act}"),
                "ask": {"ask": "act", "act": act, "item": key},
            })
        })
        .collect();
}

/// A string field, none where it's absent, empty or not a string.
fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// When a field's time is, in milliseconds, as ForYou.js's `when` reads
/// it; none where it isn't one.
fn time(value: Option<&Value>) -> Option<i64> {
    text(value)
        .and_then(|at| at.parse::<jiff::Timestamp>().ok())
        .map(jiff::Timestamp::as_millisecond)
}

/// That the session or quest that raised it is gone, as its record says
/// (the harness's `owner.state`), as Now.qml's `ownerSaid` says it.
fn owner(item: &Value, words: &Words) -> Option<String> {
    let state = text(item.get("owner").and_then(|owner| owner.get("state")))?;
    ["ended", "concluded"]
        .contains(&state)
        .then(|| words.said(state, &[]))
}

/// A signal or a check, as Waiting.js's `waitsFor` and `doneBy` read one
/// and Now.qml says it: a signal by its name, its value after the first
/// colon filling the phrase's blank.
fn known(said: Option<&Value>, words: &Words) -> Option<String> {
    let said = said?;
    if let Some(signal) = said.get("signal").and_then(Value::as_str) {
        let (signal, value) = signal.split_once(':').unwrap_or((signal, ""));
        return Some(words.said(signal, &[("value", value)]));
    }
    said.get("check")
        .is_some_and(Value::is_string)
        .then(|| words.said("check", &[("value", "")]))
}

/// What one not ready yet waits for, in words, or its owner's own words
/// in a file older than that.
fn waits(item: &Value, words: &Words) -> Option<String> {
    known(item.get("readyWhen"), words).or_else(|| text(item.get("when")).map(str::to_owned))
}

/// How it'll be known done, as a sentence of the drawer's own: by its
/// check or signal, or by Joel alone.
fn clears(item: &Value, words: &Words) -> Option<String> {
    let done = item.get("doneWhen");
    if let Some(how) = known(done, words) {
        return Some(words.tells("known", &[("how", &how)])).filter(|said| !said.is_empty());
    }
    let person = done.and_then(|done| done.get("by")).and_then(Value::as_str) == Some("person");
    person
        .then(|| words.tells("person", &[]))
        .filter(|said| !said.is_empty())
}

/// What its note is made of, NeedsYou.js's `note` finished by whoever
/// knows its side and its session.
fn parts(item: &Value, words: &Words) -> Value {
    let done = item.get("doneWhen");
    let seen = ["check", "signal"].iter().any(|how| {
        done.and_then(|done| done.get(*how))
            .is_some_and(Value::is_string)
    });
    let mut parts = json!({"nobody": words.said("nobody", &[]), "seen": seen});
    if let Some(when) = text(item.get("when"))
        .map(str::trim)
        .filter(|when| !when.is_empty())
    {
        parts["when"] = json!(when);
    }
    if item.get("v") != Some(&json!(2))
        && let Some(why) = item.get("why").and_then(Value::as_str)
    {
        parts["why"] = json!(why);
    }
    if let Some(at) = time(item.get("raisedAt")) {
        parts["waited"] = json!({"at": at, "as": "ago", "said": words.phrase("waitedFor")});
    }
    parts
}

/// The drawer's facts this daemon says, in Now.qml's `drawerOf` order,
/// each only where it says something.
fn facts(item: &Value, key: &str, minutes: Option<&str>, words: &Words) -> Vec<Value> {
    let fact = |id: &str, said: Option<String>| {
        said.map(|said| json!({"id": id, "name": words.fact(id), "said": said}))
    };
    let moment = |id: &str, field: &str| {
        time(item.get(field))
            .map(|at| json!({"id": id, "name": words.fact(id), "at": at, "as": "moment"}))
    };
    let identifies = |mut fact: Value| {
        fact["identifies"] = json!(true);
        fact
    };
    let owned = |field: &str, inner: &str| {
        text(item.get(field).and_then(|field| field.get(inner))).map(str::to_owned)
    };
    [
        moment("due", "deadline"),
        moment("set-for", "at"),
        fact(
            "takes",
            minutes.map(|minutes| words.said("takes", &[("minutes", minutes)])),
        ),
        moment("since", "raisedAt"),
        // When the session that raised it last found it still true, so
        // Joel can tell an ask that's gone stale (Joel, 2026-11-08).
        moment("checked", "checkedAt"),
        // A sentence, so it's set in Words, under its name.
        fact("clears", clears(item, words)).map(|mut fact| {
            fact["words"] = json!(true);
            fact
        }),
        fact("machine", owned("where", "machine")),
        fact("address", owned("owner", "address")).map(identifies),
        // A command, under its name, wrapping at its own punctuation.
        fact("check", owned("doneWhen", "check")).map(|fact| {
            let mut fact = identifies(fact);
            fact["words"] = json!(true);
            fact
        }),
        fact("item", Some(key.to_owned())).map(identifies),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Its steps, numbered, then why only Joel can do it, each only when it
/// says it.
fn body(item: &Value) -> String {
    let steps = item
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|step| text(Some(step)))
        .enumerate()
        .map(|(at, step)| format!("{}. {step}", at + 1));
    let why = text(item.get("why")).map(|why| ["", why].map(str::to_owned));
    steps
        .chain(why.into_iter().flatten())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocabulary() -> Value {
        serde_json::from_str(include_str!("../tests/words.json")).expect("For You's words parse")
    }

    fn words() -> Words {
        Words::read(&vocabulary()).expect("every word a to-do needs is there")
    }

    fn said(item: &Value, now: bool) -> Value {
        let mut record = json!({});
        worded(&mut record, item, "Q:sign-in", now, &words());
        record
    }

    #[test]
    fn one_ready_says_how_long_it_takes_and_one_waiting_what_for() {
        let item = json!({"title": "Sign in", "minutes": 2,
            "readyWhen": {"signal": "signin:Slack"}});
        let ready = said(&item, true);
        assert_eq!(ready["name"], "Sign in", "its title");
        assert_eq!(ready["hue"], "strong", "one Joel can do now is strong");
        assert_eq!(
            ready["side"],
            json!([{"said": "2 min"}]),
            "how long it takes"
        );
        let waiting = said(&item, false);
        assert_eq!(waiting["hue"], "muted", "one for later is muted");
        assert_eq!(
            waiting["side"],
            json!([{"said": "once signed in to Slack"}]),
            "what it waits for, its value after the colon"
        );
        let ended = said(
            &json!({"title": "T", "owner": {"state": "ended"}, "when": "after lunch"}),
            false,
        );
        assert_eq!(
            ended["side"],
            json!([{"said": "session ended"}, {"said": "after lunch"}]),
            "its owner gone leads, and an older file's when stands for what it waits for"
        );
    }

    #[test]
    fn its_note_is_made_of_parts_only_the_agents_daemon_can_finish() {
        let parts = said(
            &json!({"title": "T", "when": " tonight ", "why": "Only you can",
                "raisedAt": "2026-11-10T12:00:00Z", "doneWhen": {"by": "person"}}),
            true,
        )["parts"]
            .clone();
        assert_eq!(
            parts,
            json!({"nobody": "nobody's watching this", "seen": false, "when": "tonight",
                "why": "Only you can",
                "waited": {"at": 1_794_312_000_000_i64, "as": "ago", "said": "waiting {span}"}}),
            "every part, its why only from a file older than mesh_wait's"
        );
        let checked = said(
            &json!({"v": 2, "title": "T", "why": "W", "doneWhen": {"check": "true"}}),
            true,
        )["parts"]
            .clone();
        assert_eq!(
            checked,
            json!({"nobody": "nobody's watching this", "seen": true}),
            "one this daemon checks is seen, and mesh_wait's own file says no why"
        );
    }

    #[test]
    fn its_drawer_holds_its_facts_in_order_and_its_steps_numbered() {
        let drawer = said(
            &json!({"title": "Plug in the dock", "minutes": 1.0, "why": "It's in your hands",
                "steps": ["Find it", "", "Plug it in"], "deadline": "2026-11-12T23:00:00Z",
                "raisedAt": "2026-11-10T12:00:00Z", "where": {"machine": "laptop"},
                "owner": {"address": "ABC123"}, "doneWhen": {"signal": "device:dock"}}),
            true,
        )["drawer"]
            .clone();
        assert_eq!(drawer["title"], "Plug in the dock");
        assert_eq!(
            drawer["body"], "1. Find it\n2. Plug it in\n\nIt's in your hands",
            "its steps numbered, an empty one left out, then why"
        );
        let ids: Vec<&str> = drawer["facts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|fact| fact["id"].as_str().unwrap())
            .collect();
        assert_eq!(
            ids,
            [
                "due", "takes", "since", "clears", "machine", "address", "item"
            ],
            "only the facts it says, in the drawer's order"
        );
        let facts = &drawer["facts"];
        assert_eq!(
            facts[0],
            json!({"id": "due", "name": "Due", "at": 1_794_524_400_000_i64, "as": "moment"}),
            "a time for the shell to word"
        );
        assert_eq!(
            facts[1]["said"], "1 min",
            "a whole number said as JavaScript says it"
        );
        assert_eq!(
            facts[3],
            json!({"id": "clears", "name": "Clears When",
                "said": "Known done once the dock is seen.", "words": true}),
            "how it's known done, a sentence"
        );
        assert_eq!(
            facts[6],
            json!({"id": "item", "name": "Item", "said": "Q:sign-in", "identifies": true}),
            "who it is"
        );
    }

    #[test]
    fn its_acts_ask_this_daemon() {
        let acts = said(&json!({"title": "T"}), true)["acts"].clone();
        assert_eq!(
            acts[1],
            json!({"key": "verb:on:waiting:notNow",
                "ask": {"ask": "act", "act": "notNow", "item": "Q:sign-in"}}),
            "put off, by an ask on the socket, its words the shell's by its key"
        );
        assert_eq!(acts[0]["ask"]["act"], "done", "done first");
    }

    #[test]
    fn each_acts_key_is_a_verb_the_launcher_words() {
        let things = vocabulary()["things"].clone();
        let acts = said(&json!({"title": "T"}), true)["acts"].clone();
        for act in acts.as_array().expect("its acts are a list") {
            assert!(
                things
                    .as_array()
                    .is_some_and(|things| things.iter().any(|thing| thing["id"] == act["key"])),
                "an act's key names no word of the launcher's: {act}"
            );
        }
    }
}
