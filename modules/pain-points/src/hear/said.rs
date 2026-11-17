//! What Joel himself said in a pi session log, each message with what the
//! agent had just done before it: the tools it ran and the ones that
//! failed. A message is his when he typed it into the session, or sent it
//! from outside its window (the talk line, the launcher's answer or a
//! steer), which pi opens with a bracketed line naming him; anything else
//! opening with a bracket, or as words.rs knows the harness's and the
//! mesh's text, is not his. What he had attached, the window he was
//! looking at, is kept apart from his words.

use crate::pi::iso_ms;
use crate::words::automated;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How many of the agent's last tools are kept as a message's context.
const TOOLS_KEPT: usize = 4;

/// How many of his own last messages a session keeps, to hear one said
/// again.
const EARLIER_KEPT: usize = 4;

/// The openings of a message pi delivered from outside its window, when
/// the one sending it was Joel.
const HIS_OPENINGS: [&str; 4] = [
    "[Joel ",
    "[The person is asking",
    "[The person answered",
    "[Steered by joel",
];

/// One message Joel said, with its context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    /// The log entry's id, which names the moment in the session.
    pub entry: String,
    /// When pi took it, in milliseconds since 1970.
    pub at: i64,
    /// His words, with pi's opening and any attachment cut away.
    pub words: String,
    /// The window he attached, which says what he was looking at.
    pub looking_at: Option<String>,
    /// The agent's last tools before it, oldest first.
    pub tools: Vec<String>,
    /// Those of them whose results were errors.
    pub failed: Vec<String>,
    /// His last few messages in the session before it, oldest first.
    pub before: Vec<(i64, String)>,
}

/// What a session's reading carries from one run to the next: the tools
/// run since his last message, and his last few messages.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Carried {
    pub tools: Vec<String>,
    pub failed: Vec<String>,
    pub earlier: Vec<(i64, String)>,
}

/// His words in a user message's text, and the window attached, or none
/// when someone else sent it.
pub fn his(text: &str) -> Option<(String, Option<String>)> {
    let text = text.trim();
    let body = if text.starts_with('[') {
        if !HIS_OPENINGS.iter().any(|opening| text.starts_with(opening)) {
            return None;
        }
        &text[text.find(']')? + 1..]
    } else if automated(text) {
        return None;
    } else {
        text
    };
    let (words, attached) = match body.find("\n\nAttached:") {
        Some(at) => (&body[..at], Some(&body[at..])),
        None => (body, None),
    };
    let words = words.trim();
    if words.is_empty() {
        return None;
    }
    let window = attached.and_then(|a| {
        a.lines()
            .find_map(|l| l.trim().strip_prefix("- window: "))
            .map(|w| w.trim().to_string())
    });
    Some((words.to_string(), window))
}

/// A log line's parts that say whose it is, read without its content.
#[derive(Deserialize)]
struct Head {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    timestamp: String,
    message: Option<HeadMessage>,
}

#[derive(Deserialize)]
struct HeadMessage {
    role: String,
    #[serde(rename = "toolName", default)]
    tool_name: Option<String>,
    #[serde(rename = "isError", default)]
    is_error: bool,
}

/// A message's text blocks, joined, or its text when it is one string.
fn text_of(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn keep_last<T>(list: &mut Vec<T>, most: usize) {
    if list.len() > most {
        list.drain(..list.len() - most);
    }
}

/// Reads one log line into `carried`, giving the message it holds when it
/// is his.
pub fn line(text: &str, carried: &mut Carried) -> Option<Said> {
    // A tool's result is read for its name alone, so its output, often
    // the most of a log, is never decoded.
    let head: Head = serde_json::from_str(text).ok()?;
    let message = head.message.filter(|_| head.kind == "message")?;
    match message.role.as_str() {
        "assistant" => {
            let full: Value = serde_json::from_str(text).ok()?;
            let calls = full.pointer("/message/content").and_then(Value::as_array)?;
            carried.tools.extend(
                calls
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("toolCall"))
                    .filter_map(|b| b.get("name").and_then(Value::as_str))
                    .map(str::to_string),
            );
            keep_last(&mut carried.tools, TOOLS_KEPT);
            None
        }
        "toolResult" => {
            if message.is_error
                && let Some(name) = message.tool_name
            {
                carried.failed.push(name);
                keep_last(&mut carried.failed, TOOLS_KEPT);
            }
            None
        }
        "user" => {
            let full: Value = serde_json::from_str(text).ok()?;
            let (words, looking_at) = his(&text_of(full.pointer("/message/content")?))?;
            let at = iso_ms(&head.timestamp)?;
            let said = Said {
                entry: head.id,
                at,
                words: words.clone(),
                looking_at,
                tools: std::mem::take(&mut carried.tools),
                failed: std::mem::take(&mut carried.failed),
                before: carried.earlier.clone(),
            };
            carried.earlier.push((at, words));
            keep_last(&mut carried.earlier, EARLIER_KEPT);
            Some(said)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(id: &str, stamp: &str, text: &str) -> String {
        serde_json::json!({
            "type": "message", "id": id, "parentId": "p", "timestamp": stamp,
            "message": {"role": "user", "content": [{"type": "text", "text": text}],
                        "timestamp": 0}
        })
        .to_string()
    }

    fn called(names: &[&str]) -> String {
        let calls: Vec<Value> = names
            .iter()
            .map(|n| {
                serde_json::json!({"type": "toolCall", "id": "c", "name": n,
                                         "arguments": {}})
            })
            .collect();
        serde_json::json!({
            "type": "message", "id": "a", "parentId": "p",
            "timestamp": "2026-11-16T18:00:00.000Z",
            "message": {"role": "assistant", "content": calls, "timestamp": 0}
        })
        .to_string()
    }

    fn result(name: &str, error: bool) -> String {
        serde_json::json!({
            "type": "message", "id": "r", "parentId": "a",
            "timestamp": "2026-11-16T18:00:01.000Z",
            "message": {"role": "toolResult", "toolCallId": "c", "toolName": name,
                        "content": [{"type": "text", "text": "out"}], "isError": error,
                        "timestamp": 0}
        })
        .to_string()
    }

    #[test]
    fn what_he_typed_is_his_and_what_the_harness_sent_is_not() {
        assert_eq!(
            his("why is this so slow"),
            Some(("why is this so slow".into(), None))
        );
        assert_eq!(his("[Automated message, no human input: carry on]"), None);
        assert_eq!(his("Mesh message [a] (fyi) from X"), None);
        assert_eq!(
            his("[Qcxzeb is asking from outside this window: answer first.]\n\nhi"),
            None,
            "an agent asking from outside isn't him"
        );
        assert_eq!(his("   "), None, "nothing said is nothing of his");
    }

    #[test]
    fn a_peers_note_starting_a_session_is_not_his() {
        for note in [
            "Woken by 1JQPPC on WCJQSG's ruling da5a35fb (the quest sweep): no, it's stale",
            "Started by 1JQPPC for WCJQSG pivot 6b6ae191: this is broken",
            "Restarted by P9R1KI for the quest's plan, which still fails",
        ] {
            assert_eq!(his(note), None, "{note:?} is a peer's");
        }
        for his_own in [
            "Stopped by the launcher again, why",
            "Fixed by ABC123? no, still broken",
        ] {
            assert!(his(his_own).is_some(), "{his_own:?} is his");
        }
    }

    #[test]
    fn what_he_sent_from_outside_the_window_is_his_without_its_attachments() {
        let text = "[Joel is asking from outside this window: answer first, in a line or \
                    two, then do what is asked.]\n\nthe forage command does not play well \
                    on small terminals\n\nAttached:\n- window: kitty · mycelium forage\n- tab: x";
        assert_eq!(
            his(text),
            Some((
                "the forage command does not play well on small terminals".into(),
                Some("kitty · mycelium forage".into())
            ))
        );
        assert_eq!(
            his("[Steered by joel from outside this window]\n\nstop that"),
            Some(("stop that".into(), None))
        );
    }

    #[test]
    fn a_message_carries_the_tools_run_since_his_last() {
        let mut carried = Carried::default();
        assert_eq!(line(&called(&["bash", "edit"]), &mut carried), None);
        assert_eq!(line(&result("bash", false), &mut carried), None);
        assert_eq!(line(&result("edit", true), &mut carried), None);
        let said = line(
            &user("e1", "2026-11-16T18:00:05.000Z", "no, that's broken again"),
            &mut carried,
        )
        .expect("his message is said");
        assert_eq!(said.entry, "e1");
        assert_eq!(said.at, iso_ms("2026-11-16T18:00:05.000Z").expect("a time"));
        assert_eq!(said.words, "no, that's broken again");
        assert_eq!(said.tools, vec!["bash", "edit"]);
        assert_eq!(said.failed, vec!["edit"]);
        assert!(carried.tools.is_empty(), "his message starts a new turn");
        assert_eq!(carried.earlier, vec![(said.at, said.words.clone())]);
    }

    #[test]
    fn only_the_last_few_tools_and_messages_are_carried() {
        let mut carried = Carried::default();
        line(&called(&["a", "b", "c", "d", "e", "f"]), &mut carried);
        assert_eq!(carried.tools, vec!["c", "d", "e", "f"]);
        for n in 0..6 {
            line(
                &user(
                    &format!("e{n}"),
                    "2026-11-16T18:00:05.000Z",
                    &format!("ask {n}"),
                ),
                &mut carried,
            );
        }
        assert_eq!(carried.earlier.len(), EARLIER_KEPT);
        assert_eq!(carried.earlier[3].1, "ask 5");
    }

    #[test]
    fn a_line_that_is_not_a_message_or_not_json_says_nothing() {
        let mut carried = Carried::default();
        assert_eq!(line("not json", &mut carried), None);
        assert_eq!(
            line(
                r#"{"type":"custom_message","content":"Mesh message"}"#,
                &mut carried
            ),
            None
        );
        assert_eq!(carried, Carried::default());
    }
}
