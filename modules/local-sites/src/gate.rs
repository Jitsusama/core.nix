//! What a page may ask a feed. A page reads and never acts: of the daemon
//! interface (docs/daemons.md) it may say `hello`, `get`, `list`, `watch`,
//! `unwatch` and `health`, and only of the kinds and fields its feed
//! declares. Anything else is answered with an error naming why, rather
//! than passed on, so no page can steer a session or update a machine
//! through a daemon that would let a local client do so.

use serde_json::{Value, json};

use crate::config::{Feed, Kind};

/// What one line from a page comes to.
#[derive(Debug, PartialEq)]
pub enum Passed {
    /// Said hello, answered here.
    Hello { id: Value },
    /// A watch of one declared kind, narrowed to these keys and fields.
    Watch {
        id: Value,
        kind: String,
        keys: Option<Vec<String>>,
        fields: Option<Vec<String>>,
    },
    /// The watch with this id ends.
    Unwatch { id: Value },
    /// An ask the daemon answers once, as it's to be sent.
    Once { id: Value, asked: Value },
    /// Refused, with the answer saying why.
    Refused(Value),
}

/// What a page's line `line` comes to on `feed`.
pub fn pass(feed: &Feed, line: &str) -> Passed {
    let Ok(said) = serde_json::from_str::<Value>(line) else {
        return Passed::Refused(error(
            &Value::Null,
            "not-an-ask",
            "a line is one JSON object",
        ));
    };
    let id = said.get("id").cloned().unwrap_or(Value::Null);
    let ask = said.get("ask").and_then(Value::as_str).unwrap_or("");
    for unsupported in ["place", "every", "when", "deep"] {
        if said.get(unsupported).is_some() {
            return Passed::Refused(error(
                &id,
                "refused",
                &format!("a page can't ask with {unsupported}; the feed asks what it declares"),
            ));
        }
    }
    match ask {
        "hello" => Passed::Hello { id },
        "unwatch" => Passed::Unwatch { id },
        "health" => Passed::Once {
            asked: json!({"ask": "health"}),
            id,
        },
        "watch" => match one_kind(feed, &said) {
            Err(why) => Passed::Refused(error(&id, "unknown-kind", &why)),
            Ok((kind, declared)) => match fields(declared, &said) {
                Err(why) => Passed::Refused(error(&id, "refused", &why)),
                Ok(fields) => Passed::Watch {
                    id,
                    kind,
                    keys: words(said.get("keys")),
                    fields,
                },
            },
        },
        "list" => match one_kind(feed, &said) {
            Err(why) => Passed::Refused(error(&id, "unknown-kind", &why)),
            Ok((kind, declared)) => match fields(declared, &said) {
                Err(why) => Passed::Refused(error(&id, "refused", &why)),
                Ok(fields) => {
                    let mut asked = json!({"ask": "list", "kind": kind});
                    if let Some(fields) = fields {
                        asked["fields"] = json!(fields);
                    }
                    if let Some(limit) = said.get("limit") {
                        asked["limit"] = limit.clone();
                    }
                    Passed::Once { id, asked }
                }
            },
        },
        "get" => {
            let keys = words(said.get("keys")).unwrap_or_default();
            match keys
                .iter()
                .find(|key| !feed.kinds.contains_key(kind_of(key)))
            {
                Some(key) => Passed::Refused(error(
                    &id,
                    "unknown-kind",
                    &format!("{key} is of a kind this feed doesn't show"),
                )),
                None => Passed::Once {
                    asked: json!({"ask": "get", "keys": keys}),
                    id,
                },
            }
        }
        "" => Passed::Refused(error(&id, "not-an-ask", "a line names its ask")),
        other => Passed::Refused(error(
            &id,
            "refused",
            &format!("a page reads and never acts, so it can't ask {other}"),
        )),
    }
}

/// A record trimmed to what its feed shows of its kind: nothing of a kind
/// it doesn't declare, and only the declared fields, besides the key,
/// where it names some.
pub fn shown(feed: &Feed, record: &Value) -> Option<Value> {
    let key = record.get("key").and_then(Value::as_str)?;
    let kind = feed.kinds.get(kind_of(key))?;
    Some(daemon_serve::asks::trimmed(record, kind.fields.as_deref()))
}

/// The kind a key is of: what comes before its first colon, or the key
/// itself where it has none, as a record kept alone is keyed by its kind.
pub fn kind_of(key: &str) -> &str {
    key.split_once(':').map_or(key, |(kind, _)| kind)
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    daemon_serve::asks::error(id, code, message)
}

fn one_kind<'a>(feed: &'a Feed, said: &Value) -> Result<(String, &'a Kind), String> {
    let kinds = words(said.get("kind")).ok_or("a watch or a list names its kind")?;
    let [kind] = kinds.as_slice() else {
        return Err("a page watches or lists one kind at a time".to_owned());
    };
    feed.kinds
        .get(kind)
        .map(|declared| (kind.clone(), declared))
        .ok_or_else(|| format!("this feed shows no {kind}"))
}

/// The fields a page asked, which must be among those its feed shows, or
/// those its feed shows where it asked none.
fn fields(declared: &Kind, said: &Value) -> Result<Option<Vec<String>>, String> {
    let asked = said
        .get("fields")
        .map(|fields| words(Some(fields)).ok_or("fields are names"))
        .transpose()?;
    match (asked, &declared.fields) {
        (None, shown) => Ok(shown.clone()),
        (Some(asked), None) => Ok(Some(asked)),
        (Some(asked), Some(shown)) => match asked.iter().find(|field| !shown.contains(field)) {
            Some(field) => Err(format!("this feed doesn't show {field}")),
            None => Ok(Some(asked)),
        },
    }
}

/// One word or a list of them.
fn words(said: Option<&Value>) -> Option<Vec<String>> {
    match said? {
        Value::String(one) => Some(vec![one.clone()]),
        Value::Array(many) => many
            .iter()
            .map(|word| word.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;

    fn node() -> Feed {
        Feed {
            socket: PathBuf::from("/run/node.sock"),
            kinds: BTreeMap::from([
                ("machine".to_owned(), Kind::default()),
                (
                    "session".to_owned(),
                    Kind {
                        fields: Some(vec!["title".to_owned(), "presence".to_owned()]),
                        deep: None,
                    },
                ),
            ]),
        }
    }

    fn code(passed: &Passed) -> &str {
        match passed {
            Passed::Refused(answer) => answer["error"]["code"].as_str().unwrap_or(""),
            _ => "",
        }
    }

    #[test]
    fn a_watch_of_a_declared_kind_takes_the_fields_it_shows() {
        assert_eq!(
            pass(&node(), r#"{"id": 3, "ask": "watch", "kind": "session"}"#),
            Passed::Watch {
                id: json!(3),
                kind: "session".to_owned(),
                keys: None,
                fields: Some(vec!["title".to_owned(), "presence".to_owned()]),
            }
        );
    }

    #[test]
    fn a_watch_may_narrow_to_keys_and_to_fields_it_shows() {
        let passed = pass(
            &node(),
            &json!({
                "id": 4, "ask": "watch", "kind": ["session"],
                "keys": ["session:a"], "fields": ["title"]
            })
            .to_string(),
        );
        assert_eq!(
            passed,
            Passed::Watch {
                id: json!(4),
                kind: "session".to_owned(),
                keys: Some(vec!["session:a".to_owned()]),
                fields: Some(vec!["title".to_owned()]),
            }
        );
    }

    #[test]
    fn a_field_or_kind_the_feed_doesnt_show_is_refused() {
        let wider = r#"{"id": 5, "ask": "watch", "kind": "session", "fields": ["text"]}"#;
        assert_eq!(code(&pass(&node(), wider)), "refused");
        let other = r#"{"id": 6, "ask": "watch", "kind": "message"}"#;
        assert_eq!(code(&pass(&node(), other)), "unknown-kind");
        let two = r#"{"id": 7, "ask": "watch", "kind": ["machine", "session"]}"#;
        assert_eq!(code(&pass(&node(), two)), "unknown-kind");
    }

    #[test]
    fn an_act_is_refused_and_never_passed_on() {
        for act in ["steer", "update", "settings", "charge-limit"] {
            let line = format!(r#"{{"id": 8, "ask": "{act}"}}"#);
            assert_eq!(code(&pass(&node(), &line)), "refused", "{act}");
        }
    }

    #[test]
    fn a_page_cant_ask_how_the_feed_asks() {
        let deep = r#"{"id": 9, "ask": "watch", "kind": "session", "deep": ["presence"]}"#;
        assert_eq!(code(&pass(&node(), deep)), "refused");
    }

    #[test]
    fn a_get_names_only_keys_of_kinds_it_shows() {
        assert_eq!(
            pass(
                &node(),
                r#"{"id": 10, "ask": "get", "keys": ["machine:kestrel"]}"#
            ),
            Passed::Once {
                id: json!(10),
                asked: json!({"ask": "get", "keys": ["machine:kestrel"]})
            }
        );
        let message = r#"{"id": 11, "ask": "get", "keys": ["machine:kestrel", "message:m1"]}"#;
        assert_eq!(code(&pass(&node(), message)), "unknown-kind");
    }

    #[test]
    fn a_list_asks_for_the_fields_shown() {
        assert_eq!(
            pass(
                &node(),
                r#"{"id": 12, "ask": "list", "kind": "session", "limit": 5}"#
            ),
            Passed::Once {
                id: json!(12),
                asked: json!({
                    "ask": "list", "kind": "session", "fields": ["title", "presence"], "limit": 5
                })
            }
        );
    }

    #[test]
    fn a_record_shows_only_its_kinds_declared_fields() {
        let session = json!({"key": "session:a", "title": "A", "cwd": "/home/joel/secret"});
        assert_eq!(
            shown(&node(), &session),
            Some(json!({"key": "session:a", "title": "A"}))
        );
        assert_eq!(
            shown(&node(), &json!({"key": "message:m1", "text": "hi"})),
            None
        );
        assert_eq!(kind_of("summary"), "summary", "a record kept alone");
    }

    #[test]
    fn a_line_that_isnt_an_ask_is_named_so() {
        assert_eq!(code(&pass(&node(), "not json")), "not-an-ask");
        assert_eq!(code(&pass(&node(), r#"{"id": 1}"#)), "not-an-ask");
    }
}
