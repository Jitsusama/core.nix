//! What Joel himself said in a pi session log, each message with what the
//! agent had just done before it: the tools it ran and the ones that
//! failed. A message is his when he typed it into the session, or sent it
//! from outside its window (the talk line, the launcher's answer or a
//! steer), which pi opens with a bracketed line naming him; anything else
//! opening with a bracket, or as words.rs knows the harness's and the
//! mesh's text, is not his. What he had attached, the window he was
//! looking at, is kept apart from his words. It also says when the agent
//! answered him: a turn he opened, nobody else's message between, that ran
//! a tool and ended speaking, with when he opened it, which is how a
//! session's own cell takes up a pain he said to it.

use crate::pi::iso_ms;
use crate::words::automated;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How many of the agent's last tools are kept as a message's context.
const TOOLS_KEPT: usize = 4;

/// How many of his own last messages a session keeps, to hear one said
/// again.
const EARLIER_KEPT: usize = 4;

/// How many answers a session keeps while no cell is known to take them.
const ANSWERS_KEPT: usize = 32;

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
    /// Whether the agent ran a tool since his last message with nobody
    /// else's between, and no turn of it errored or was cut short: a turn
    /// answering a peer or the harness, or one after a failed turn, isn't
    /// an answer to him.
    #[serde(default)]
    pub acting: bool,
    /// An answer a reading kept before answers held what they answered,
    /// in milliseconds since 1970, taken as it was then: every pain said
    /// in the session before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered: Option<i64>,
    /// When he said the message the kept `answered` answered, as a reading
    /// kept it before `answers`: its pains from then on are taken.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asked: Option<i64>,
    /// When he opened the turn under way, by his first message since the
    /// last turn ended: none once it ends, or once someone else's message
    /// makes the turn theirs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<i64>,
    /// Whether the turn under way last stopped on an error, which pi tries
    /// again within it unless he speaks first, opening a turn of his own.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub errored: bool,
    /// The log entry of the message that errored, which pi's retry edits
    /// out of its context: none when a reading kept before it was named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Whether a peer's or the harness's message came after that error
    /// with no word yet that pi tries again, so the answer after it, if pi
    /// never does, is theirs.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub crossed: bool,
    /// Each answer to him not yet taken, as when his turn opened and when
    /// it was answered, kept until the session's cell is known, so every
    /// answer in one reading takes what it answered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<(i64, i64)>,
}

/// His words in a user message's text, and the window attached, or none
/// when someone else sent it.
pub fn his(text: &str) -> Option<(String, Option<String>)> {
    let text = text.trim();
    if others(text) {
        return None;
    }
    let body = if text.starts_with('[') {
        &text[text.find(']')? + 1..]
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

/// His message at `at` in `carried`'s turn: it opens the turn where none is
/// under way, or where the last stopped on an error he spoke after rather
/// than waiting for pi to try it again.
fn opened(carried: &mut Carried, at: i64) {
    if carried.errored {
        carried.from = Some(at);
    } else {
        carried.from.get_or_insert(at);
    }
}

/// Whether a user message's trimmed `text` is someone else's: a peer's or
/// the harness's, by its opening or as automated text.
fn others(text: &str) -> bool {
    if text.starts_with('[') {
        !HIS_OPENINGS.iter().any(|opening| text.starts_with(opening))
    } else {
        automated(text)
    }
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
    /// The entry a context edit changes, and what it puts in its place.
    #[serde(rename = "targetId", default)]
    target: Option<String>,
    #[serde(default)]
    replacement: Option<Value>,
}

/// Whether `head` is pi's retry of `carried`'s errored turn: an edit that
/// drops the message that errored from its context, with nothing in its
/// place. Any other edit, such as an extension's, isn't one.
fn retried(head: &Head, carried: &Carried) -> bool {
    head.replacement.is_none() && (carried.error.is_none() || head.target == carried.error)
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
    // pi tries an errored turn again by first editing the error out of its
    // context, then waiting, so a peer's or the harness's message before
    // that edit can still land inside his retry, and one after it does.
    // Only one with no retry after it starts a turn of its own, answering
    // them, not him. pi writes no edit when it gives up.
    if carried.errored {
        match head.kind.as_str() {
            "custom_message" => carried.crossed = true,
            "context_edit" if retried(&head, carried) => {
                (carried.errored, carried.crossed) = (false, false);
            }
            _ => {}
        }
    }
    let message = head.message.filter(|_| head.kind == "message")?;
    match message.role.as_str() {
        "assistant" => {
            if std::mem::take(&mut carried.crossed) {
                carried.from = None;
            }
            let full: Value = serde_json::from_str(text).ok()?;
            let content = full.pointer("/message/content")?;
            carried.tools.extend(
                content
                    .as_array()?
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("toolCall"))
                    .filter_map(|b| b.get("name").and_then(Value::as_str))
                    .map(str::to_string),
            );
            keep_last(&mut carried.tools, TOOLS_KEPT);
            let acted = content
                .as_array()?
                .iter()
                .any(|b| b.get("type").and_then(Value::as_str) == Some("toolCall"));
            // A turn that errored, or that he or the harness stopped, said
            // nothing to him, and one that only talked did nothing about
            // what hurt; either leaves the pain open.
            let stopped = full.pointer("/message/stopReason").and_then(Value::as_str);
            match stopped {
                Some("error" | "aborted") => carried.acting = false,
                _ => carried.acting |= acted,
            }
            if stopped == Some("stop")
                && carried.acting
                && !text_of(content).trim().is_empty()
                && let (Some(from), Some(at)) = (carried.from, iso_ms(&head.timestamp))
            {
                carried.answers.push((from, at));
                keep_last(&mut carried.answers, ANSWERS_KEPT);
            }
            // A turn ends on any stop but a call for tools, answered or not,
            // or an error, which pi tries again within the same turn.
            carried.errored = stopped == Some("error");
            carried.error = carried.errored.then(|| head.id.clone());
            if stopped.is_some_and(|stop| !matches!(stop, "toolUse" | "error")) {
                carried.from = None;
            }
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
            // His message starts a new turn, and anyone else's is a turn
            // that answers them, not him.
            carried.acting = false;
            let full: Value = serde_json::from_str(text).ok()?;
            let content = full
                .pointer("/message/content")
                .map(text_of)
                .unwrap_or_default();
            let at = iso_ms(&head.timestamp);
            let Some((words, looking_at)) = his(&content) else {
                // Only someone else's message makes the turn theirs: one of
                // his with no words, such as a screenshot alone, opens his.
                if others(content.trim()) {
                    carried.from = None;
                } else if let Some(at) = at {
                    opened(carried, at);
                }
                (carried.errored, carried.crossed) = (false, false);
                return None;
            };
            let at = at?;
            opened(carried, at);
            (carried.errored, carried.crossed) = (false, false);
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

    /// The agent's message ending its turn as `stop`, saying `text`.
    fn ended(stop: &str, text: &str) -> String {
        serde_json::json!({
            "type": "message", "id": "s", "parentId": "r",
            "timestamp": "2026-11-16T18:00:09.000Z",
            "message": {"role": "assistant", "stopReason": stop,
                        "content": [{"type": "text", "text": text}], "timestamp": 0}
        })
        .to_string()
    }

    #[test]
    fn a_turn_that_acted_and_ended_speaking_answered_him() {
        let at = iso_ms("2026-11-16T18:00:09.000Z");
        let answered = |lines: &[String]| {
            let mut carried = Carried::default();
            for text in lines {
                line(text, &mut carried);
            }
            carried.answers.last().map(|&(_, at)| at)
        };
        let his = user("e1", "2026-11-16T18:00:05.000Z", "this is broken again");
        let ran = called(&["bash"]);
        let done = ended("stop", "Fixed: the launcher opens again.");
        let peer = user(
            "m1",
            "2026-11-16T18:00:07.000Z",
            "[Mesh message from a peer] a question",
        );
        assert_eq!(answered(&[his.clone(), ran.clone(), done.clone()]), at);
        assert_eq!(
            answered(&[his.clone(), ran.clone(), ran.clone(), done.clone()]),
            at,
            "a turn of several tool calls answered him"
        );
        for (lines, why) in [
            (
                vec![his.clone(), done.clone()],
                "a turn that only talked did nothing about it",
            ),
            (
                vec![ran.clone(), his.clone(), done.clone()],
                "tools run before he spoke aren't an answer to him",
            ),
            (
                vec![his.clone(), ran.clone(), ended("error", "overloaded")],
                "a turn that errored said nothing to him",
            ),
            (
                vec![his.clone(), ran.clone(), peer.clone(), done.clone()],
                "a turn answering a peer's message didn't answer him",
            ),
            (
                vec![
                    his.clone(),
                    ran.clone(),
                    ended("error", "overloaded"),
                    done.clone(),
                ],
                "a turn that only talked after a failed one did nothing about it",
            ),
            (
                vec![his.clone(), ran.clone(), ended("aborted", "")],
                "a turn stopped partway said nothing to him",
            ),
            (
                vec![his.clone(), ran.clone(), ended("stop", "  ")],
                "a turn that ended saying nothing said nothing",
            ),
        ] {
            assert_eq!(answered(&lines), None, "{why}");
        }
    }

    #[test]
    fn each_answer_is_kept_with_when_he_opened_its_turn() {
        let his = user("e1", "2026-11-16T18:00:05.000Z", "this is broken again");
        let ran = called(&["bash"]);
        let done = ended("stop", "Fixed: the launcher opens again.");
        let peer = user(
            "m1",
            "2026-11-16T18:00:07.000Z",
            "[Mesh message from a peer] a question",
        );
        let mut carried = Carried::default();
        for text in [&his, &ran, &done] {
            line(text, &mut carried);
        }
        let kept = serde_json::to_string(&carried).expect("carried writes");
        let back: Carried = serde_json::from_str(&kept).expect("carried reads");
        let opened = iso_ms("2026-11-16T18:00:05.000Z").expect("a time");
        let at = iso_ms("2026-11-16T18:00:09.000Z").expect("a time");
        assert_eq!(
            (back.answers, back.from),
            (vec![(opened, at)], None),
            "an answer is kept with when his turn opened until the session's cell is known"
        );
        let windows = |lines: &[&String]| {
            let mut carried = Carried::default();
            for text in lines {
                line(text, &mut carried);
            }
            carried.answers
        };
        let steer = user("e2", "2026-11-16T18:00:06.000Z", "and the bar too");
        let again = user("e3", "2026-11-16T18:00:08.000Z", "now the dock");
        assert_eq!(
            windows(&[&his, &steer, &ran, &done, &again, &ran, &done]),
            vec![
                (opened, at),
                (iso_ms("2026-11-16T18:00:08.000Z").expect("a time"), at),
            ],
            "a turn he steered wasn't answered from its opening, or one answer hid another"
        );
        assert_eq!(
            windows(&[&his, &peer, &ran, &done]),
            Vec::new(),
            "a turn a peer's message made theirs answered him"
        );
        let old: Carried = serde_json::from_str(r#"{"tools": [], "failed": [], "earlier": []}"#)
            .expect("a reading kept before answers reads");
        assert_eq!(
            old,
            Carried::default(),
            "a reading kept before answers reads as none"
        );
    }

    #[test]
    fn his_turn_lasts_through_calls_for_tools_retries_and_his_wordless_messages() {
        let his = user("e1", "2026-11-16T18:00:05.000Z", "this is broken again");
        let opened = iso_ms("2026-11-16T18:00:05.000Z").expect("a time");
        let at = iso_ms("2026-11-16T18:00:09.000Z").expect("a time");
        let asking = serde_json::json!({
            "type": "message", "id": "a", "parentId": "p",
            "timestamp": "2026-11-16T18:00:06.000Z",
            "message": {"role": "assistant", "stopReason": "toolUse", "timestamp": 0,
                        "content": [{"type": "toolCall", "id": "c", "name": "bash",
                                     "arguments": {}}]}
        })
        .to_string();
        let failed = ended("error", "");
        let shot = serde_json::json!({
            "type": "message", "id": "e2", "parentId": "p",
            "timestamp": "2026-11-16T18:00:07.000Z",
            "message": {"role": "user", "content": [{"type": "image", "data": "x"}],
                        "timestamp": 0}
        })
        .to_string();
        let done = ended("stop", "Fixed: the launcher opens again.");
        for (lines, what) in [
            (vec![&his, &asking, &done], "a call for tools"),
            (
                vec![&his, &asking, &failed, &asking, &done],
                "pi's retry after an error",
            ),
            (
                vec![&his, &shot, &asking, &done],
                "a screenshot of his alone",
            ),
        ] {
            let mut carried = Carried::default();
            for text in lines {
                line(text, &mut carried);
            }
            assert_eq!(
                carried.answers,
                vec![(opened, at)],
                "{what} ended his turn, or gave it away"
            );
        }
        // Read across two runs, parted after a call for tools or an error.
        for (before, after) in [
            (vec![&his, &asking], vec![&done]),
            (vec![&his, &asking, &failed], vec![&asking, &done]),
        ] {
            let mut midway = Carried::default();
            for text in before {
                line(text, &mut midway);
            }
            let kept = serde_json::to_string(&midway).expect("carried writes");
            let mut back: Carried = serde_json::from_str(&kept).expect("carried reads");
            for text in after {
                line(text, &mut back);
            }
            assert_eq!(
                back.answers,
                vec![(opened, at)],
                "a turn read across two runs lost when he opened it"
            );
        }
    }

    #[test]
    fn a_peers_message_after_an_error_takes_the_turn_unless_pi_tries_again() {
        let his = user("e1", "2026-11-16T18:00:05.000Z", "this is broken again");
        let opened = iso_ms("2026-11-16T18:00:05.000Z").expect("a time");
        let at = iso_ms("2026-11-16T18:00:09.000Z").expect("a time");
        let asking = called(&["bash"]);
        // The answer that errored has an id of its own, "f", beside the
        // others' "s", so an edit of the wrong one can't pass as the retry.
        let failed = ended("error", "").replacen(r#""id":"s""#, r#""id":"f""#, 1);
        let done = ended("stop", "Fixed: the launcher opens again.");
        let peer = r#"{"type":"custom_message","content":"Mesh message"}"#.to_owned();
        // pi's retry edits the errored answer out of its context first.
        let retry = r#"{"type":"context_edit","targetId":"f","replacement":null}"#.to_owned();
        // An extension may edit another entry, or put text in its place.
        let other = r#"{"type":"context_edit","targetId":"s","replacement":null}"#.to_owned();
        let rewrite =
            r#"{"type":"context_edit","targetId":"f","replacement":"x"}"#.to_owned();
        assert!(
            failed.contains(r#""id":"f""#),
            "the errored answer kept the others' id"
        );
        let his_answer = vec![(opened, at)];
        for (lines, answers, what) in [
            (
                vec![&his, &asking, &failed, &peer, &asking, &done],
                Vec::new(),
                "a peer's turn after an error pi never tried again",
            ),
            (
                vec![
                    &his, &asking, &failed, &peer, &other, &rewrite, &asking, &done,
                ],
                Vec::new(),
                "a peer's turn after edits that aren't pi's retry",
            ),
            (
                vec![&his, &asking, &failed, &peer, &retry, &asking, &done],
                his_answer.clone(),
                "a peer's message before pi's retry",
            ),
            (
                vec![&his, &asking, &failed, &retry, &peer, &asking, &done],
                his_answer.clone(),
                "a peer's message during pi's retry",
            ),
            (
                vec![&his, &asking, &peer, &asking, &done],
                his_answer,
                "a peer's message mid-turn with no error",
            ),
        ] {
            let mut carried = Carried::default();
            for text in lines {
                line(text, &mut carried);
            }
            assert_eq!(carried.answers, answers, "{what} was read wrongly");
        }
        // Read across two runs, parted after the error or the peer's message.
        for (before, after, answers) in [
            (
                vec![&his, &asking, &failed],
                vec![&peer, &asking, &done],
                Vec::new(),
            ),
            (
                vec![&his, &asking, &failed, &peer],
                vec![&asking, &done],
                Vec::new(),
            ),
            (
                vec![&his, &asking, &failed, &peer],
                vec![&retry, &asking, &done],
                vec![(opened, at)],
            ),
            (
                vec![&his, &asking, &failed],
                vec![&retry, &peer, &asking, &done],
                vec![(opened, at)],
            ),
        ] {
            let mut midway = Carried::default();
            for text in before {
                line(text, &mut midway);
            }
            let kept = serde_json::to_string(&midway).expect("carried writes");
            let mut back: Carried = serde_json::from_str(&kept).expect("carried reads");
            for text in after {
                line(text, &mut back);
            }
            assert_eq!(
                back.answers, answers,
                "a run lost that the turn had errored, that a peer came after, or which \
                 error pi tried again"
            );
        }
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
