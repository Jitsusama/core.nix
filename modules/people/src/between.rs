//! What a source serves between Joel and someone, made ready as the daemon
//! takes it in: each thing once by its key, the oldest first, and the
//! newest they met at said apart as `met`. So the shell picks no first of
//! its own but the oldest thing that waits now, which turns with the clock
//! (People.qml's servedFirsts).

use std::collections::BTreeSet;

use serde_json::{Value, json};

/// A source's entry with what's between them made ready; one that says
/// nothing between them is as it came. A thing with no key is left out,
/// as the shell left it, since nothing could say it's the one said again.
#[must_use]
pub fn ready(mut entry: Value) -> Value {
    let Some(said) = entry.get("between").and_then(Value::as_array) else {
        return entry;
    };
    let mut keys = BTreeSet::new();
    let mut things: Vec<Value> = said
        .iter()
        .filter(|one| {
            one["key"]
                .as_str()
                .is_some_and(|key| !key.is_empty() && keys.insert(key.to_owned()))
        })
        .cloned()
        .collect();
    // Stable, so of two at once the one said first stays first.
    things.sort_by(|one, other| at(one).total_cmp(&at(other)));
    let met = things
        .iter()
        .filter(|one| one["kind"] == "met")
        .fold(None, |newest: Option<&Value>, one| match newest {
            Some(newest) if at(newest) >= at(one) => Some(newest),
            _ => Some(one),
        })
        .map_or(Value::Null, |met| {
            json!({
                "at": met["at"], "name": met["row"]["name"].as_str().unwrap_or_default(),
                "one": met["one"] == true,
            })
        });
    entry["met"] = met;
    entry["between"] = Value::Array(things);
    entry
}

/// When a thing was, in milliseconds since the epoch, as sources say it.
fn at(one: &Value) -> f64 {
    one["at"].as_f64().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thing(key: &str, at: u64, kind: &str) -> Value {
        json!({"key": key, "at": at, "kind": kind, "row": {"name": key}})
    }

    #[test]
    fn each_thing_is_kept_once_by_its_key_the_oldest_first() {
        let entry = json!({"between": [
            thing("b", 20, "wrote"), thing("a", 10, "wrote"), thing("b", 30, "wrote"),
            {"at": 5, "kind": "wrote"},
        ]});
        let keys: Vec<Value> = ready(entry)["between"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| json!([one["key"], one["at"]]))
            .collect();
        assert_eq!(
            keys,
            [json!(["a", 10]), json!(["b", 20])],
            "each said again by its key, or with none, is left out, and the oldest leads"
        );
    }

    #[test]
    fn the_newest_they_met_at_is_said_apart() {
        let mut one = thing("standup", 40, "met");
        one["one"] = json!(true);
        let entry = json!({"between": [
            thing("review", 30, "met"), one, thing("note", 50, "wrote"),
            thing("planning", 40, "met"),
        ]});
        assert_eq!(
            ready(entry)["met"],
            json!({"at": 40, "name": "standup", "one": true}),
            "the newest meeting, the first of two at once, is their last met"
        );
        let entry = json!({"between": [thing("note", 50, "wrote")]});
        assert_eq!(
            ready(entry)["met"],
            Value::Null,
            "met nobody, so no last met"
        );
    }

    #[test]
    fn the_book_keeps_what_a_source_said_between_them_ready() {
        let mut book = crate::Book::default();
        let line = json!({"email": "ada@example.com", "between": [
            thing("b", 20, "met"), thing("a", 10, "wrote"),
        ]});
        book.take("slack", crate::heard(&line.to_string()));
        let entry = &book.record("person:ada@example.com").unwrap().sources["slack"];
        assert_eq!(
            (&entry["between"][0]["key"], &entry["met"]["name"]),
            (&json!("a"), &json!("b")),
            "kept as it came: {entry}"
        );
    }

    #[test]
    fn an_entry_saying_nothing_between_them_is_as_it_came() {
        let entry = json!({"email": "ada@example.com", "known": false});
        assert_eq!(ready(entry.clone()), entry, "nothing was made of nothing");
    }
}
