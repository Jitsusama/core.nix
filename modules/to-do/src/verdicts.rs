//! What the daemon found of each to-do, as `mission/verdicts.json` says
//! it: `done`, `ready`, or `waiting` with every figure of what it last
//! found. The harness and the shell act on the first two and skip any
//! other state, so `waiting` only adds what For You's drawer says.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::{Map, Value, json};

use crate::judge::Heard;
use crate::pace::Pace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Done,
    Ready,
    Waiting,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Done => "done",
            State::Ready => "ready",
            State::Waiting => "waiting",
        }
    }
}

/// Which of a to-do's two the figures are of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum For {
    Done,
    Ready,
}

/// One to-do's verdict: its state, since when, and how it was found.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub state: State,
    pub at: i64,
    pub how: Value,
}

/// An ISO time, as the harness and the shell read times.
pub fn iso(at: i64) -> String {
    jiff::Timestamp::from_second(at).map_or_else(|_| String::new(), |t| t.to_string())
}

fn time(at: Option<i64>) -> Value {
    at.map_or(Value::Null, |at| Value::String(iso(at)))
}

/// How a check stands, every figure of its runs.
pub fn by_check(of: For, pace: &Pace) -> Value {
    let last = pace.last.as_ref();
    json!({
        "by": "check",
        "for": of.name(),
        "check": pace.check,
        "ran": pace.ran,
        "first_at": time(pace.first_at),
        "last_at": time(pace.last_at),
        "exit": last.and_then(|run| run.exit),
        "took_ms": last.map(|run| run.took_ms),
        "said": last.map(|run| run.said.clone()),
        "next_at": time(Some(pace.next_at)),
    })
}

/// A check not run, since its owner is in the strict sandbox and here it
/// would run outside it, as Joel with all he can reach.
pub fn refused(of: For, check: &str) -> Value {
    json!({
        "by": "check",
        "for": of.name(),
        "check": check,
        "refused": "strict sandbox",
    })
}

/// The to-dos `text` says are done, as it was last written, so a restart
/// keeps what was found until each to-do leaves its file.
pub fn done_in(text: &str) -> BTreeMap<String, Verdict> {
    let Ok(file) = serde_json::from_str::<Value>(text) else {
        return BTreeMap::new();
    };
    let Some(items) = file.get("items").and_then(Value::as_object) else {
        return BTreeMap::new();
    };
    items
        .iter()
        .filter(|(_, item)| item.get("state").and_then(Value::as_str) == Some("done"))
        .filter_map(|(key, item)| {
            let at = item.get("at")?.as_str()?.parse::<jiff::Timestamp>().ok()?;
            Some((
                key.clone(),
                Verdict {
                    state: State::Done,
                    at: at.as_second(),
                    how: item.get("how").cloned().unwrap_or(Value::Null),
                },
            ))
        })
        .collect()
}

/// How a signal stands: whether it was heard, and its figure.
pub fn by_signal(of: For, signal: &str, found: &Result<Heard, Option<(&str, String)>>) -> Value {
    let mut how = Map::new();
    how.insert("by".into(), "signal".into());
    how.insert("for".into(), of.name().into());
    how.insert("signal".into(), signal.into());
    let (heard_at, figure) = match found {
        Ok(heard) => (time(heard.at), Some(heard.figure.clone())),
        Err(figure) => (Value::Null, figure.as_ref().map(|(k, v)| (*k, v.clone()))),
    };
    how.insert("heard_at".into(), heard_at);
    if let Some((name, value)) = figure.filter(|(_, v)| !v.is_empty()) {
        // A time the machine gave in seconds is said as the others are.
        let value = match (name.ends_with("_at"), value.parse::<i64>()) {
            (true, Ok(at)) => Value::String(iso(at)),
            _ => Value::String(value),
        };
        how.insert(name.into(), value);
    }
    Value::Object(how)
}

impl For {
    pub fn name(self) -> &'static str {
        match self {
            For::Done => "done",
            For::Ready => "ready",
        }
    }
}

/// The file whole, by key, in key order so the same verdicts are the same
/// text.
pub fn file(verdicts: &BTreeMap<String, Verdict>) -> String {
    let items: Map<String, Value> = verdicts
        .iter()
        .map(|(key, verdict)| {
            (
                key.clone(),
                json!({ "state": verdict.state.name(), "at": iso(verdict.at), "how": verdict.how }),
            )
        })
        .collect();
    json!({ "v": 1, "items": items }).to_string()
}

/// Writes `text` to `path` through a rename, so no reader sees half.
pub fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let writing = path.with_extension("json.writing");
    fs::write(&writing, text)?;
    fs::rename(&writing, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pace::Run;

    #[test]
    fn a_checks_figures_are_each_said() {
        let mut pace = Pace::new("test -e /x", 1_794_200_000);
        pace.ran(
            Run {
                exit: Some(1),
                took_ms: 1200,
                said: "no /x".into(),
            },
            1_794_200_000,
        );
        let how = by_check(For::Done, &pace);
        assert_eq!(how["by"], "check");
        assert_eq!(how["for"], "done");
        assert_eq!(how["ran"], 1);
        assert_eq!(how["exit"], 1);
        assert_eq!(how["took_ms"], 1200);
        assert_eq!(how["said"], "no /x");
        assert_eq!(how["first_at"], "2026-11-09T04:53:20Z");
        assert_eq!(how["next_at"], "2026-11-09T04:53:50Z");
    }

    #[test]
    fn a_signal_not_heard_says_where_it_stands() {
        let found = Err(Some(("core", "b4986f5".to_owned())));
        let how = by_signal(For::Ready, "switch:1d839c4", &found);
        assert_eq!(how["signal"], "switch:1d839c4");
        assert_eq!(how["heard_at"], Value::Null);
        assert_eq!(how["core"], "b4986f5");
        let boot = Ok(Heard {
            at: Some(1_794_200_000),
            figure: ("boot_at", "1794200000".into()),
        });
        let how = by_signal(For::Done, "boot", &boot);
        assert_eq!(how["heard_at"], "2026-11-09T04:53:20Z");
        assert_eq!(how["boot_at"], "2026-11-09T04:53:20Z");
    }

    #[test]
    fn the_file_is_the_same_text_for_the_same_verdicts() {
        let mut verdicts = BTreeMap::new();
        verdicts.insert(
            "Q:b".to_owned(),
            Verdict {
                state: State::Waiting,
                at: 0,
                how: json!({}),
            },
        );
        verdicts.insert(
            "Q:a".to_owned(),
            Verdict {
                state: State::Done,
                at: 1_794_200_000,
                how: json!({"by": "check"}),
            },
        );
        let text = file(&verdicts);
        assert_eq!(text, file(&verdicts.clone()));
        let read: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(read["items"]["Q:a"]["state"], "done");
        assert_eq!(read["items"]["Q:a"]["at"], "2026-11-09T04:53:20Z");
        assert_eq!(read["items"]["Q:b"]["state"], "waiting");
        let kept = done_in(&text);
        assert_eq!(kept.keys().collect::<Vec<_>>(), ["Q:a"]);
        assert_eq!(kept["Q:a"], verdicts["Q:a"]);
        assert_eq!(done_in("not json"), BTreeMap::new());
    }
}
