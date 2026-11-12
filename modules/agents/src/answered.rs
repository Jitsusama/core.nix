//! The talks a session has answered and Joel hasn't read, so he knows
//! one came without looking for it (Joel, 2026-11-11: "how do i know when
//! a sessions answers a talk?"). An answer is said calmly, as a count
//! with the rest of how the sessions stand and on its session's row,
//! never as a notice, and its drawer leads with what he asked and what
//! came back, where he replies in place.
//!
//! A talk is the node's `talk` record of the session's steer inbox, its
//! lines in the order they were written: what Joel sent (`delivered`),
//! how the session read it, and what it said (`replying` as it writes,
//! `replied` once it's done). Only the newest talk to a session counts,
//! so one he's since sent, or one the machine sent it on its own, has
//! moved it on. Its sign and count go once he's read it (`read`), but
//! the drawer keeps it, so it stays as he reads it there, until the talk
//! is over (`over`) or its session is gone. The harness keeps each
//! talk's lines (lib/mesh/talk.ts): a reader tags on the stage, passes
//! over stages and fields it doesn't know, and finds a field absent
//! rather than null; `read` is the daemon's alone to write.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use crate::clock;
use crate::said::Said;

/// What the machine sends a session on its own, rather than Joel, as the
/// harness starts each of its own messages.
const AUTOMATED: &str = "[Automated message";

/// A talk its session answered that Joel hasn't read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    /// The session's address.
    pub address: String,
    /// The talk's id, which names its file in the session's inbox.
    pub talk: String,
    /// What Joel asked.
    pub asked: String,
    /// What the session answered.
    pub answer: String,
    /// When it answered, in milliseconds since 1970, where it said.
    pub at: Option<i64>,
    /// Whether Joel has read it.
    pub read: bool,
}

/// When a talk was sent, in milliseconds, from its id, `talk-<ms>` or
/// `talk-<ms>-<n>`, so the newest is known without reading its lines.
fn sent(id: &str) -> (i64, i64) {
    let mut parts = id.strip_prefix("talk-").unwrap_or("").split('-');
    let mut number = || parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    (number(), number())
}

/// The answer of one talk, if it's one: Joel's, answered, and not over,
/// read or not.
fn answer_of(talk: &Value) -> Option<Answer> {
    let lines = talk["lines"].as_array()?;
    let delivered = lines.iter().find(|line| line["stage"] == "delivered")?;
    let asked = delivered["text"].as_str()?;
    if delivered["from"] != "joel" || asked.starts_with(AUTOMATED) {
        return None;
    }
    let (answered, line) = lines
        .iter()
        .enumerate()
        .rfind(|(_, line)| line["stage"] == "replied")?;
    let answer = line["text"]
        .as_str()
        .filter(|text| !text.trim().is_empty())?;
    let after = &lines[answered..];
    if after.iter().any(|line| line["stage"] == "over") {
        return None;
    }
    Some(Answer {
        address: talk["address"].as_str()?.to_owned(),
        talk: talk["id"].as_str()?.to_owned(),
        asked: asked.to_owned(),
        answer: answer.to_owned(),
        at: clock::ms_of(&line["at"]),
        read: after.iter().any(|line| line["stage"] == "read"),
    })
}

/// The answers among the node's `talks`, read or not, one at most a session,
/// from the newest talk to each whose session is `live`, in the order of
/// their addresses.
pub fn answers<'a>(
    talks: impl Iterator<Item = &'a Value>,
    live: &dyn Fn(&str) -> bool,
) -> Vec<Answer> {
    let mut newest: std::collections::BTreeMap<&str, &Value> = std::collections::BTreeMap::new();
    for talk in talks {
        let (Some(address), Some(id)) = (talk["address"].as_str(), talk["id"].as_str()) else {
            continue;
        };
        let newer = newest
            .get(address)
            .is_none_or(|kept| sent(kept["id"].as_str().unwrap_or("")) < sent(id));
        if newer {
            newest.insert(address, talk);
        }
    }
    newest
        .into_iter()
        .filter(|(address, _)| live(address))
        .filter_map(|(_, talk)| answer_of(talk))
        .collect()
}

/// Says in the talk's own file that Joel read its answer, as the harness
/// writes its lines, one appended in one write, so the node tells every
/// reader of it as it tells the rest of the talk. Its address and id are
/// a folder's and a file's names alone, never a path.
pub fn read(steer: &Path, answer: &Answer, now: i64) -> std::io::Result<()> {
    let plain = |name: &str| {
        !name.is_empty()
            && name
                .chars()
                .all(|one| one.is_ascii_alphanumeric() || one == '-' || one == '_')
    };
    if !plain(&answer.address) || !plain(&answer.talk) {
        return Err(std::io::Error::other("a talk named by more than a name"));
    }
    let path = steer
        .join(&answer.address)
        .join("replies")
        .join(format!("{}.jsonl", answer.talk));
    let line = json!({"stage": "read", "by": "joel", "at": clock::text_of(now)});
    OpenOptions::new()
        .append(true)
        .open(path)?
        .write_all(format!("{line}\n").as_bytes())
}

/// What a drawer says first of a session that answered: what Joel asked,
/// then its answer and when it came, each a sentence or more, so he
/// reads it in place (`names`, the drawer's names for its facts). An
/// answer that only passes his words on, as the void's "→ WCJQSG" does,
/// names where they went as Agents does (`named`, a session's quest
/// title, else its name, by an address it goes by), never an address.
pub fn facts(answer: &Answer, names: &Said, named: &dyn Fn(&str) -> Option<String>) -> Vec<Value> {
    let mut answered = json!({"label": names.word("answer"),
        "value": passed_on(&answer.answer, named), "words": true});
    if let Some(at) = answer.at {
        answered["at"] = json!({"at": at, "as": "clock"});
    }
    vec![
        json!({"label": names.word("asked"), "value": answer.asked, "words": true}),
        answered,
    ]
}

/// An answer as Joel reads it: as it came, unless it only passes his
/// words on to one address the sessions know.
fn passed_on(answer: &str, named: &dyn Fn(&str) -> Option<String>) -> String {
    answer
        .trim()
        .strip_prefix('→')
        .map(str::trim)
        .filter(|to| !to.is_empty() && !to.contains(char::is_whitespace))
        .and_then(named)
        .map_or_else(|| answer.to_owned(), |name| format!("→ {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawer_says_what_joel_asked_then_what_it_answered_and_when() {
        let names =
            Said::names_under(crate::said::LAUNCHERS, "agents").expect("the launcher's words read");
        let answer = Answer {
            address: "BEACON".to_owned(),
            talk: "talk-1".to_owned(),
            asked: "is the beacon lit?".to_owned(),
            answer: "Yes, since nine.".to_owned(),
            at: Some(1_794_414_593_809),
            read: true,
        };
        assert_eq!(
            facts(&answer, &names, &|_| None),
            [
                json!({"label": "You Asked", "value": "is the beacon lit?", "words": true}),
                json!({"label": "Its Answer", "value": "Yes, since nine.", "words": true,
                    "at": {"at": 1_794_414_593_809_i64, "as": "clock"}}),
            ],
            "what was asked, then the answer with its time"
        );
    }

    #[test]
    fn an_answer_passing_joels_words_on_names_where_they_went() {
        let named = |address: &str| (address == "LAMPLT").then(|| "Light the Lamps".to_owned());
        let cases = [
            (
                "→ LAMPLT",
                "→ Light the Lamps",
                "one it knows reads by its name",
            ),
            (" →  LAMPLT ", "→ Light the Lamps", "however it's spaced"),
            (
                "→ GONEAW",
                "→ GONEAW",
                "one it doesn't know stays as it came",
            ),
            (
                "→ LAMPLT and BEACON",
                "→ LAMPLT and BEACON",
                "more than an address is words",
            ),
            (
                "Yes, LAMPLT",
                "Yes, LAMPLT",
                "an answer naming one isn't passing on",
            ),
        ];
        for (said, read, why) in cases {
            assert_eq!(passed_on(said, &named), read, "{why}");
        }
    }

    fn talk(address: &str, id: &str, lines: &[Value]) -> Value {
        json!({"key": format!("talk:{address}/{id}"), "kind": "talk", "address": address,
            "id": id, "lines": lines})
    }

    fn delivered(text: &str) -> Value {
        json!({"stage": "delivered", "from": "joel", "at": "2026-11-11T21:00:00Z", "text": text})
    }

    fn replied(text: &str) -> Value {
        json!({"stage": "replied", "at": "2026-11-11T21:01:00Z", "text": text})
    }

    fn answered(talks: &[Value]) -> Vec<(String, String)> {
        answers(talks.iter(), &|address| address != "GONE01")
            .into_iter()
            .filter(|one| !one.read)
            .map(|one| (one.address, one.answer))
            .collect()
    }

    #[test]
    fn a_talk_answered_is_an_answer_with_what_joel_asked() {
        let talks = [talk(
            "BEACON",
            "talk-1794414593809",
            &[
                delivered("is the beacon lit?"),
                json!({"stage": "reading", "at": "2026-11-11T21:00:10Z"}),
                json!({"stage": "replying", "at": "2026-11-11T21:00:50Z", "text": "Yes"}),
                replied("Yes, since nine."),
            ],
        )];
        assert_eq!(
            answers(talks.iter(), &|_| true),
            [Answer {
                address: "BEACON".to_owned(),
                talk: "talk-1794414593809".to_owned(),
                asked: "is the beacon lit?".to_owned(),
                answer: "Yes, since nine.".to_owned(),
                at: clock::ms_of(&json!("2026-11-11T21:01:00Z")),
                read: false,
            }],
            "the answer, what was asked, and when it came"
        );
    }

    #[test]
    fn a_talk_still_being_answered_or_failed_is_none() {
        let talks = [
            talk(
                "BEACON",
                "talk-1",
                &[
                    delivered("still there?"),
                    json!({"stage": "replying", "text": "Ye"}),
                ],
            ),
            talk(
                "LAMPLT",
                "talk-1",
                &[delivered("lit?"), json!({"stage": "failed"})],
            ),
        ];
        assert_eq!(answered(&talks), [], "only what's been answered whole is");
    }

    #[test]
    fn an_answer_is_still_to_read_until_read_over_moved_on_or_its_session_gone() {
        let talks = [
            talk(
                "BEACON",
                "talk-1",
                &[
                    delivered("lit?"),
                    replied("Yes."),
                    json!({"stage": "read", "by": "joel", "at": 1}),
                ],
            ),
            talk(
                "LAMPLT",
                "talk-1",
                &[delivered("lit?"), replied("Yes."), json!({"stage": "over"})],
            ),
            talk("HARBOR", "talk-5", &[delivered("lit?"), replied("Yes.")]),
            talk("HARBOR", "talk-10", &[delivered("and now?")]),
            talk("GONE01", "talk-1", &[delivered("lit?"), replied("Yes.")]),
        ];
        assert_eq!(answered(&talks), [], "none of these is still to read");
        let kept: Vec<(String, bool)> = answers(talks.iter(), &|address| address != "GONE01")
            .into_iter()
            .map(|one| (one.address, one.read))
            .collect();
        assert_eq!(
            kept,
            [("BEACON".to_owned(), true)],
            "one read stays for its drawer, but none over or moved on"
        );
    }

    #[test]
    fn what_the_machine_sent_on_its_own_is_no_answer_but_moves_it_on() {
        let reopened = json!({"stage": "delivered", "from": "joel",
            "text": "[Automated message, no human input: the machine restarted.] Carry on."});
        let talks = [
            talk(
                "BEACON",
                "talk-1",
                &[reopened.clone(), replied("Carrying on.")],
            ),
            talk("LAMPLT", "talk-1-0", &[delivered("lit?"), replied("Yes.")]),
            talk("LAMPLT", "talk-1-1", &[reopened, replied("Carrying on.")]),
        ];
        assert_eq!(
            answered(&talks),
            [],
            "an automated talk answers nothing Joel asked"
        );
    }

    #[test]
    fn the_newest_talk_to_each_is_the_one_heard() {
        let talks = [
            talk(
                "BEACON",
                "talk-20",
                &[delivered("and now?"), replied("Still.")],
            ),
            talk("BEACON", "talk-3", &[delivered("lit?"), replied("Yes.")]),
        ];
        assert_eq!(
            answered(&talks),
            [("BEACON".to_owned(), "Still.".to_owned())],
            "talk-20 came after talk-3, by its number, not its name"
        );
    }

    #[test]
    fn reading_an_answer_says_so_in_its_talks_file() {
        let steer = std::env::temp_dir().join(format!("agents-answered-{}", std::process::id()));
        let replies = steer.join("BEACON").join("replies");
        std::fs::create_dir_all(&replies).expect("its replies folder");
        let file = replies.join("talk-1.jsonl");
        std::fs::write(&file, "{\"stage\":\"replied\",\"text\":\"Yes.\"}\n").expect("written");
        let answer = Answer {
            address: "BEACON".to_owned(),
            talk: "talk-1".to_owned(),
            asked: String::new(),
            answer: String::new(),
            at: None,
            read: false,
        };
        read(&steer, &answer, 1_794_414_593_809).expect("read");
        let lines = std::fs::read_to_string(&file).expect("its lines");
        let elsewhere = Answer {
            talk: "../talk-1".to_owned(),
            ..answer
        };
        let refused = read(&steer, &elsewhere, 0).is_err();
        std::fs::remove_dir_all(&steer).expect("cleaned");
        assert_eq!(
            lines,
            "{\"stage\":\"replied\",\"text\":\"Yes.\"}\n\
             {\"at\":\"2026-11-11T16:29:53.809Z\",\"by\":\"joel\",\"stage\":\"read\"}\n",
            "the read line follows the talk's own"
        );
        assert!(refused, "a talk named by a path is refused");
    }
}
