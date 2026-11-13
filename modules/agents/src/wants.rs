//! What each session asks Joel, as For You's Needs You lists it: one
//! `want` record per session running or parked whose want is truly his
//! (`session::asks_joel`), never one whose want is an action, since that's
//! one of its own items, which For You lists itself (Holding.wanting).
//! Each says what it asked, since when, whether it's parked, and the
//! answers it offers to a single question (harness #1394), the one it
//! recommends first, so the shell reads them rather than parsing every
//! session's presence (KRINS0 item 2). Answering is asked of the daemon
//! too (`answer_of`), which finds the want by the session's address, so
//! the shell never names a want's id.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use serde_json::{Value, json};

use crate::acts;
use crate::clock;
use crate::session::{self, JoelCan, Lives, Session};

/// The kind of a session's want record.
pub const KIND: &str = "want";

/// The place its keys are under, Agents'.
const PLACE: &str = "agents";

/// The want records served last, for answering one: shared between the
/// daemon's loop, which holds them, and its asks.
#[derive(Clone, Default)]
pub struct Wants(Arc<Mutex<Vec<Value>>>);

impl Wants {
    /// Holds the wants among `records` from now on.
    pub fn hold<'a>(&self, records: impl Iterator<Item = &'a Value>) {
        let held = records
            .filter(|record| record["kind"] == KIND)
            .cloned()
            .collect();
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = held;
    }

    /// Answers a want, steered through the node at `node`, refused where
    /// there's none; None for any ask but an answer.
    pub fn answer(&self, said: &Value, node: Option<&Path>, now_ms: u128) -> Option<Value> {
        let held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let steer = answer_of(said, &held, now_ms)?;
        drop(held);
        Some(match (steer, node) {
            (Err(why), _) => json!({"error": {"code": "not-an-ask", "message": why}}),
            (Ok(steer), Some(node)) => acts::done(steer, node),
            (Ok(_), None) => json!({"error": {"code": "unsteered",
                "message": "this machine runs no node to steer it through"}}),
        })
    }
}

/// A want record per session asking Joel, the oldest ask first.
pub fn records(sessions: &[Session], joel_can: &JoelCan) -> Vec<Value> {
    let mut wanting: Vec<(String, Value)> = sessions
        .iter()
        .filter(|one| matches!(one.lives(), Lives::Running | Lives::Parked))
        .filter(|one| session::asks_joel(one.presence(), joel_can))
        .filter_map(|one| {
            let wants = one.presence()?.get("wants")?;
            if wants["kind"] == "action" {
                return None;
            }
            let title = one.named().0;
            let asked = wants["said"]
                .as_str()
                .filter(|said| !said.is_empty())
                .map_or_else(|| title.clone(), str::to_owned);
            let since = wants["since"].as_str().unwrap_or("").to_owned();
            let record = json!({
                "key": format!("{KIND}:{PLACE}:{}", one.address()),
                "kind": KIND,
                "address": one.address(),
                "title": title,
                "asked": asked,
                "since": since,
                "parked": one.lives() == Lives::Parked,
                "answers": answers(wants),
            });
            Some((since, record))
        })
        .collect();
    wanting.sort_by(|a, b| a.0.cmp(&b.0));
    wanting.into_iter().map(|(_, record)| record).collect()
}

/// What a want offers (Choices.js's `of`, which this replaces): null when
/// it offers no answers, else the want's id as the session knows it,
/// `<kind>@<since>`, its answers with the one it recommends first and the
/// rest in its order, each `{ value, label, description, recommended }`,
/// and whether it takes Joel's own words.
fn answers(wants: &Value) -> Value {
    let ask = &wants["ask"];
    let (Some(given), Some(since)) = (ask["choices"].as_array(), clock::ms_of(&wants["since"]))
    else {
        return Value::Null;
    };
    let recommended = ask["recommended"].as_str().unwrap_or("");
    let choices: Vec<Value> = given
        .iter()
        .filter_map(|one| {
            let value = one["value"].as_str().filter(|value| !value.is_empty())?;
            let label = one["label"]
                .as_str()
                .filter(|label| !label.is_empty())
                .unwrap_or(value);
            Some(json!({
                "value": value,
                "label": label,
                "description": one["description"].as_str().unwrap_or(""),
                "recommended": !recommended.is_empty() && value == recommended,
            }))
        })
        .collect();
    if choices.is_empty() {
        return Value::Null;
    }
    let (first, rest): (Vec<Value>, Vec<Value>) = choices
        .into_iter()
        .partition(|one| one["recommended"] == true);
    json!({
        "want": format!("{}@{}", wants["kind"].as_str().unwrap_or(""), clock::text_of(since)),
        "choices": first.into_iter().chain(rest).collect::<Vec<_>>(),
        "own": ask["allowOther"] == true,
    })
}

/// The node's ask that answers a session's want, as `acts::done` takes
/// it, from `{ "ask": "answer", "address", "value" }` or `"other"` for
/// Joel's own words: the want found among `wants` by its address, so a
/// want gone or answered since is refused rather than answered. None for
/// any other ask.
pub fn answer_of(said: &Value, wants: &[Value], now_ms: u128) -> Option<Result<Value, String>> {
    if said["ask"] != "answer" {
        return None;
    }
    let address = said["address"].as_str().unwrap_or("").to_uppercase();
    let Some(want) = wants
        .iter()
        .find(|one| one["address"].as_str().unwrap_or("").to_uppercase() == address)
        .and_then(|one| one["answers"]["want"].as_str())
    else {
        return Some(Err(format!(
            "{address} asks Joel nothing he can answer here"
        )));
    };
    let mut steer = json!({
        "act": "answer",
        "want": want,
        "from": "joel",
        "at": clock::text_of(i64::try_from(now_ms).unwrap_or(0)),
    });
    if let Some(value) = said["value"].as_str().filter(|value| !value.is_empty()) {
        steer["value"] = json!(value);
    } else if let Some(other) = said["other"]
        .as_str()
        .map(str::trim)
        .filter(|o| !o.is_empty())
    {
        steer["other"] = json!(other);
    } else {
        return Some(Err("an answer needs a value or Joel's own words".to_owned()));
    }
    Some(Ok(
        json!({"ask": "steer", "address": address, "steer": steer}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asking(address: &str, standing: &str, wants: &Value) -> Value {
        json!({"standing": standing, "presence": {"addresses": [address],
            "questInfo": {"title": format!("{address} Quest")}, "wants": wants}})
    }

    fn served(sessions: &[Value]) -> Vec<Value> {
        let sessions: Vec<Session> = sessions.iter().map(Session).collect();
        records(&sessions, &JoelCan::default())
    }

    #[test]
    fn each_session_asking_joel_is_a_want_the_oldest_first() {
        let question = json!({"kind": "question", "said": "which tide?",
            "since": "2026-11-13T05:00:00Z", "ask": {"recommended": "high",
                "allowOther": true, "choices": [
                    {"value": "low", "label": "Low Tide"},
                    {"value": "high", "label": "High Tide", "description": "at noon"},
                    {"value": ""}]}});
        let approval = json!({"kind": "approval", "since": "2026-11-13T04:00:00Z"});
        let wants = served(&[
            asking("SKIFF", "running", &question),
            asking("BUOY", "parked", &approval),
            asking(
                "KELP",
                "running",
                &json!({"kind": "action", "since": "2026-11-13T03:00:00Z"}),
            ),
            asking("REEF", "closed", &question),
            asking("TIDE", "running", &Value::Null),
        ]);
        let keys: Vec<&str> = wants.iter().filter_map(|one| one["key"].as_str()).collect();
        assert_eq!(
            keys,
            ["want:agents:BUOY", "want:agents:SKIFF"],
            "only those asking Joel, never an action nor one closed, the oldest first"
        );
        assert_eq!(
            wants[0],
            json!({"key": "want:agents:BUOY", "kind": "want", "address": "BUOY",
                "title": "BUOY Quest", "asked": "BUOY Quest", "since": "2026-11-13T04:00:00Z",
                "parked": true, "answers": null}),
            "one asking nothing in words is named by its session, and offers no answers"
        );
        assert_eq!(
            wants[1]["answers"],
            json!({"want": "question@2026-11-13T05:00:00.000Z", "own": true, "choices": [
                {"value": "high", "label": "High Tide", "description": "at noon",
                    "recommended": true},
                {"value": "low", "label": "Low Tide", "description": "", "recommended": false}]}),
            "its answers the recommended first, each labelled, its id as the session knows it"
        );
        assert_eq!(
            wants[1]["asked"], "which tide?",
            "what it asked, in its words"
        );
    }

    #[test]
    fn an_answer_is_steered_to_the_want_its_session_asks() {
        let wants = served(&[asking(
            "SKIFF",
            "running",
            &json!({"kind": "question", "since": "2026-11-13T05:00:00Z",
                "ask": {"choices": [{"value": "high"}]}}),
        )]);
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "skiff", "value": "high"}),
                &wants,
                0
            ),
            Some(Ok(
                json!({"ask": "steer", "address": "SKIFF", "steer": {"act": "answer",
                "want": "question@2026-11-13T05:00:00.000Z", "from": "joel",
                "at": "1970-01-01T00:00:00.000Z", "value": "high"}})
            )),
            "by its address, whatever its case, the want's id found here"
        );
        assert_eq!(
            answer_of(
                &json!({"ask": "answer", "address": "SKIFF", "other": " both "}),
                &wants,
                0
            )
            .and_then(Result::ok)
            .map(|ask| ask["steer"]["other"].clone()),
            Some(json!("both")),
            "Joel's own words, trimmed"
        );
        assert!(
            matches!(
                answer_of(
                    &json!({"ask": "answer", "address": "BUOY", "value": "x"}),
                    &wants,
                    0
                ),
                Some(Err(_))
            ),
            "a session asking nothing is refused"
        );
        assert!(
            matches!(
                answer_of(&json!({"ask": "answer", "address": "SKIFF"}), &wants, 0),
                Some(Err(_))
            ),
            "an answer with no words is refused"
        );
        assert_eq!(
            answer_of(&json!({"ask": "talk"}), &wants, 0),
            None,
            "another ask is none of this"
        );
    }
}
