//! A session's acts, done as the shell asks for one by its row's `ask`
//! (served.rs), so the shell steers no session itself and the rows and
//! what their acts do sit in one place. Each act is a steer the realm's
//! node puts in the session's inbox, here or carried to the machine
//! running it, or a wake of one parked here. Go To stays the shell's,
//! since it moves niri's focus, which only the shell holds.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

use crate::clock;

/// The acts that steer a session by their own name, as the node takes
/// them; Wake only wakes one parked, putting nothing in its inbox.
const STEERED: [&str; 5] = ["pause", "resume", "stop", "close", "wake"];

/// The act that parks a session, freeing what it holds until it's woken
/// (Joel, 24 November: a cell he can't act on from where he is "is shit").
/// It says when he asked, as the harness drops a Park dated before the
/// session started, so one asked of a session since restarted parks
/// nothing (agentic-harness.pi #2159).
const PARK: &str = "park";

/// How long the node may take to answer a steer before the act is said
/// to have failed, since the shell waits on its answer to say what
/// happened, and the node answers one in milliseconds.
const PATIENCE: Duration = Duration::from_secs(5);

/// The node's ask for an act the shell asked for, or none for an ask
/// that isn't a session's act, so the daemon's other asks are answered as
/// they were, and why for an act it can't do. `now_ms` names a talk, so
/// its answer can be followed as the harness takes it, and dates a Park,
/// which the harness drops when it's older than the session's start.
pub fn steer_of(said: &Value, now_ms: u128) -> Option<Result<Value, String>> {
    let ask = said["ask"].as_str()?;
    let steer = if STEERED.contains(&ask) {
        json!({"act": ask, "from": "joel"})
    } else if ask == PARK {
        let at = clock::text_of(i64::try_from(now_ms).unwrap_or(i64::MAX));
        json!({"act": PARK, "at": at, "from": "joel"})
    } else if ask == "talk" {
        match said["text"].as_str() {
            Some(text) if !text.trim().is_empty() => {
                let mut talk =
                    json!({"text": text, "id": format!("talk-{now_ms}"), "from": "joel"});
                // Said now, the words go into the run under way after its
                // tool, as Super+S says them to the void, whose run only
                // routes his last line; any other talk waits for the turn
                // to end, as Joel asked (12 November, lib/mesh/steer.ts).
                if said["now"] == Value::Bool(true) {
                    talk["now"] = Value::Bool(true);
                }
                talk
            }
            _ => return Some(Err("a talk has words to say".to_owned())),
        }
    } else {
        return None;
    };
    let Some(address) = said["address"]
        .as_str()
        .filter(|address| !address.is_empty())
    else {
        return Some(Err(format!(
            "{ask} names the session it's for by its address"
        )));
    };
    Some(Ok(
        json!({"ask": "steer", "address": address.to_uppercase(), "steer": steer}),
    ))
}

/// The answer to an act: the node's own, `steered` with the machine it
/// was put on or the node's error, or an error saying why it wasn't
/// asked, in the form the shell reads either (Sessions.qml's steer).
pub fn done(steer: Result<Value, String>, node: &Path) -> Value {
    let asked = steer.map(|steer| asked(node, &steer).map_err(|why| why.to_string()));
    match asked {
        Ok(Ok(answer)) => answer,
        Ok(Err(why)) => refused(&format!("the node didn't answer: {why}")),
        Err(why) => refused(&why),
    }
}

fn refused(why: &str) -> Value {
    json!({"error": {"code": "unsteered", "message": why}})
}

/// Asks the node at `socket` for `steer`, after the hello every client of
/// the interface opens with, and gives its answer without its id.
fn asked(socket: &Path, steer: &Value) -> io::Result<Value> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(PATIENCE))?;
    let mut ask = steer.clone();
    ask["id"] = json!(2);
    writeln!(stream, "{}\n{ask}", json!({"id": 1, "ask": "hello"}))?;
    for line in BufReader::new(stream).lines() {
        let Ok(mut answer) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        if answer.get("id") != Some(&json!(2)) {
            continue;
        }
        if let Some(fields) = answer.as_object_mut() {
            fields.remove("id");
        }
        return Ok(answer);
    }
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "it hung up before answering",
    ))
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    #[test]
    fn each_act_is_the_steer_the_node_takes() {
        for act in STEERED {
            let asked = steer_of(&json!({"ask": act, "address": "p9r1ki"}), 7);
            assert_eq!(
                asked,
                Some(Ok(json!({"ask": "steer", "address": "P9R1KI",
                    "steer": {"act": act, "from": "joel"}}))),
                "{act} steers the session by its address, as Joel"
            );
        }
        assert_eq!(
            steer_of(
                &json!({"ask": "talk", "address": "P9R1KI", "text": "what now?"}),
                7
            ),
            Some(Ok(json!({"ask": "steer", "address": "P9R1KI",
                "steer": {"text": "what now?", "id": "talk-7", "from": "joel"}}))),
            "a talk says its words under an id of its own"
        );
    }

    #[test]
    fn a_park_says_when_joel_asked_it() {
        assert_eq!(
            steer_of(
                &json!({"ask": "park", "address": "p9r1ki"}),
                1_764_000_000_007
            ),
            Some(Ok(json!({"ask": "steer", "address": "P9R1KI",
                "steer": {"act": "park", "at": "2025-11-24T16:00:00.007Z", "from": "joel"}}))),
            "a park steers the session as Joel, at the moment he asked"
        );
        assert!(
            matches!(steer_of(&json!({"ask": "park"}), 7), Some(Err(_))),
            "a park for no session isn't sent"
        );
    }

    #[test]
    fn a_talk_said_now_goes_into_the_turn_and_nothing_else_does() {
        let talk = |now: Value| {
            let mut said = json!({"ask": "talk", "address": "VOID01", "text": "and this"});
            if !now.is_null() {
                said["now"] = now;
            }
            steer_of(&said, 7)
                .and_then(Result::ok)
                .map(|asked| asked["steer"].clone())
        };
        assert_eq!(
            talk(json!(true)),
            Some(json!({"text": "and this", "id": "talk-7", "from": "joel", "now": true})),
            "a talk said now goes into the run under way"
        );
        for unsaid in [json!(false), json!("true"), json!(1), Value::Null] {
            assert_eq!(
                talk(unsaid.clone()),
                Some(json!({"text": "and this", "id": "talk-7", "from": "joel"})),
                "a talk with now as {unsaid} waits for the turn to end"
            );
        }
        assert_eq!(
            steer_of(&json!({"ask": "pause", "address": "P", "now": true}), 7),
            Some(Ok(json!({"ask": "steer", "address": "P",
                "steer": {"act": "pause", "from": "joel"}}))),
            "only words are said now, never an act"
        );
    }

    #[test]
    fn an_act_it_cant_do_says_why_and_any_other_ask_isnt_its() {
        assert!(
            matches!(
                steer_of(&json!({"ask": "talk", "address": "P", "text": " "}), 7),
                Some(Err(_))
            ),
            "a talk of nothing isn't sent"
        );
        assert!(
            matches!(steer_of(&json!({"ask": "pause"}), 7), Some(Err(_))),
            "an act for no session isn't sent"
        );
        assert_eq!(
            steer_of(&json!({"ask": "sharing", "sharing": true}), 7),
            None,
            "the daemon's other asks are answered as they were"
        );
    }

    #[test]
    fn an_act_is_answered_as_the_node_answers_it() {
        let socket = std::env::temp_dir().join(format!("agents-acts-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let node = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the act asks the node");
            let mut answering = stream.try_clone().expect("a stream to answer on");
            let mut asks = BufReader::new(stream).lines();
            let hello = asks.next().expect("a hello").expect("a line");
            writeln!(answering, r#"{{"id": 1, "hello": {{}}}}"#).expect("the hello answered");
            let ask = asks.next().expect("a steer").expect("a line");
            writeln!(
                answering,
                r#"{{"id": 2, "steered": {{"machine": "skiff"}}}}"#
            )
            .expect("the steer answered");
            (hello, ask)
        });
        let steer =
            steer_of(&json!({"ask": "wake", "address": "NAPPER"}), 7).expect("wake is an act");
        let answer = done(steer, &socket);
        let (hello, ask) = node.join().expect("the node stood in");
        let _ = std::fs::remove_file(&socket);
        assert_eq!(
            answer,
            json!({"steered": {"machine": "skiff"}}),
            "the node's answer, without its id"
        );
        assert!(hello.contains(r#""hello""#), "it opens with hello: {hello}");
        let ask: Value = serde_json::from_str(&ask).expect("the steer is JSON");
        assert_eq!(
            (&ask["ask"], &ask["address"], &ask["steer"]["act"]),
            (&json!("steer"), &json!("NAPPER"), &json!("wake")),
            "it asks the node to wake the session: {ask}"
        );
    }

    #[test]
    fn an_act_with_no_node_says_it_wasnt_done() {
        let gone = std::env::temp_dir().join("agents-acts-no-node.sock");
        let answer = done(Ok(json!({"ask": "steer"})), &gone);
        assert!(
            answer["error"]["message"]
                .as_str()
                .is_some_and(|why| why.starts_with("the node didn't answer")),
            "it says the node didn't answer: {answer}"
        );
    }
}
