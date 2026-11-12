//! Each ask as the launcher draws it: a ready row in Asks, its drawer
//! holding everything the line shows, whole, and its two acts carrying
//! the digest of all of it, so a yes is for exactly what was read.

use std::fmt::Write;

use ready_rows::time::{self, Form};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ask::{Ask, VERSION};
use crate::tree::Tree;
use crate::words::Words;

/// The place the rows are in.
pub const PLACE: &str = "asks";
/// The summary's key.
pub const SUMMARY: &str = "summary:asks";
/// How long a row stays once its ask is answered, in milliseconds.
pub const STAYS_MS: i64 = 300_000;

/// Where an ask stands.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Waiting,
    /// Said yes to, the value being read and the command run.
    Releasing,
    Ran {
        exit: Option<i32>,
    },
    /// Said yes to, but the value couldn't be read or the command run.
    Failed {
        why: String,
    },
    Refused,
    /// Left unanswered past its ten minutes.
    Ended,
}

impl State {
    /// Its name, as a row and an answer say it.
    pub fn name(&self) -> &'static str {
        match self {
            State::Waiting => "waiting",
            State::Releasing => "releasing",
            State::Ran { .. } => "ran",
            State::Failed { .. } => "failed",
            State::Refused => "refused",
            State::Ended => "ended",
        }
    }
}

/// An ask as it's held: what its file said, its tree as read, where it
/// stands and since when.
#[derive(Clone, Debug)]
pub struct Held {
    pub ask: Ask,
    pub tree: Tree,
    pub state: State,
    pub since_ms: i64,
}

/// The digest of everything the line shows of an ask, and the tree as it
/// was read: SHA-256 of one JSON array, its objects' keys sorted and
/// every number whole, so it reads the same however it's made.
pub fn digest(ask: &Ask, tree: &Tree) -> String {
    let quest = ask
        .quest
        .as_ref()
        .map(|(id, title)| json!({"id": id, "title": title}));
    let said = json!([
        VERSION,
        ask.id,
        ask.asked_ms,
        ask.session,
        quest,
        ask.item,
        ask.why,
        ask.argv,
        ask.tree,
        tree.commit,
        tree.uncommitted,
        ask.given,
        ask.minutes
    ]);
    let hash = Sha256::digest(said.to_string().as_bytes());
    hash.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// A word of the command, quoted as a POSIX shell would need it, so a
/// space inside one argument reads as one.
fn quoted(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_./=:,+@%".contains(&byte));
    if plain {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', r"'\''"))
    }
}

/// The tree's path as Joel reads it, his home folder as `~`.
fn homed(path: &str, home: &str) -> String {
    match path.strip_prefix(home) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => {
            format!("~{rest}")
        }
        _ => path.to_owned(),
    }
}

/// The fact saying what came of an answered ask, or none while it waits.
fn outcome(held: &Held, words: &Words) -> Option<Value> {
    let name = words.fill("outcome", &[]);
    let said = match &held.state {
        State::Waiting => return None,
        State::Releasing => words.fill("releasing", &[]),
        State::Ran { exit: Some(code) } => words.fill("ran", &[("exit", &code.to_string())]),
        State::Ran { exit: None } => words.fill("stopped", &[]),
        State::Failed { why } => words.fill("failed", &[("why", why)]),
        State::Refused => words.fill("refused", &[]),
        State::Ended => words.fill("ended", &[]),
    };
    let warns = matches!(held.state, State::Failed { .. } | State::Ran { exit: None })
        || matches!(held.state, State::Ran { exit: Some(code) } if code != 0);
    let mut fact = json!({"name": name, "said": said});
    if warns {
        fact["hue"] = json!("warning");
    }
    Some(fact)
}

/// The row for an ask, without its order, which the kept rows give it.
pub fn row(held: &Held, home: &str, words: &Words) -> Value {
    let ask = &held.ask;
    let digest = digest(ask, &held.tree);
    let key = format!("row:{PLACE}:{}", ask.id);
    let eyebrow = match &ask.quest {
        Some((_, title)) => words.fill("from", &[("session", &ask.session), ("quest", title)]),
        None => ask.session.clone(),
    };
    let command: Vec<String> = ask.argv.iter().map(|word| quoted(word)).collect();
    let path = homed(&ask.tree, home);
    let short = held
        .tree
        .commit
        .as_deref()
        .map(|commit| &commit[..commit.len().min(12)]);
    let place = match short {
        Some(commit) => words.fill("at", &[("path", &path), ("commit", commit)]),
        None => words.fill("notGit", &[("path", &path)]),
    };
    let mut facts = vec![
        json!({"name": words.fill("runs", &[]), "said": command.join(" "), "whole": true}),
        json!({"name": words.fill("in", &[]), "said": place, "whole": true}),
    ];
    let changes = match held.tree.uncommitted {
        Some(0) => None,
        Some(count) => Some(words.fill("uncommitted", &[("count", &count.to_string())])),
        None => Some(words.fill("unknown", &[])),
    };
    if let Some(said) = changes.filter(|_| held.tree.commit.is_some()) {
        facts.push(json!({"name": words.fill("changes", &[]), "said": said, "hue": "warning"}));
    }
    let minutes = ask.minutes.to_string();
    facts.push(json!({
        "name": words.fill("givenAs", &[]),
        "said": words.fill("given", &[("name", &ask.given), ("minutes", &minutes)]),
    }));
    if held.state == State::Waiting {
        facts.push(json!({
            "name": words.fill("ends", &[]),
            "at": ask.ends_ms(), "as": Form::Moment.name(),
        }));
    }
    facts.extend(outcome(held, words));
    let acts = if held.state == State::Waiting {
        let act = |verb: &str| {
            json!({"key": format!("verb:{verb}"),
                   "ask": {"ask": verb, "key": key, "digest": digest}})
        };
        json!([act("release"), act("refuse")])
    } else {
        json!([])
    };
    json!({
        "key": key,
        "thing": format!("ask:{}", ask.id),
        "place": PLACE,
        "kind": "ask",
        "state": held.state.name(),
        "name": ask.item,
        "glyph": "passwords",
        "hue": if held.state == State::Waiting { "action" } else { "machine" },
        "note": eyebrow,
        "side": [time::at(ask.asked_ms, Form::Came)],
        "attention": (held.state == State::Waiting).then_some(ask.asked_ms),
        "digest": digest,
        "acts": acts,
        "drawer": {"eyebrow": eyebrow, "title": ask.item, "body": ask.why, "facts": facts},
    })
}

/// The summary beside the rows: how many wait, or that none does.
pub fn summary(held: &[&Held], words: &Words) -> Value {
    let waiting = held
        .iter()
        .filter(|one| one.state == State::Waiting)
        .count();
    let said = match waiting {
        0 => words.fill("none", &[]),
        1 => words.fill("one", &[]),
        count => words.fill("waiting", &[("count", &count.to_string())]),
    };
    json!({"key": SUMMARY, "kind": "summary", "summary": {"place": PLACE, "said": said}})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(state: State, uncommitted: Option<u64>) -> Held {
        Held {
            ask: Ask {
                id: "3f0c9e2a7b5d41e8a6c2f9d01b7e4a35".to_owned(),
                asked_ms: 1_794_458_400_000,
                session: "S1".to_owned(),
                quest: Some(("Q-1".to_owned(), "Try the Line".to_owned())),
                item: "op://Private/Release Test/password".to_owned(),
                why: "To try the line.".to_owned(),
                argv: vec!["secret-asks".to_owned(), "received".to_owned()],
                tree: "/home/joel/src/tree".to_owned(),
                given: "RELEASED".to_owned(),
                minutes: 1,
            },
            tree: Tree {
                commit: Some("4f2a91c03b1d7e6a5b4c3d2e1f0a9b8c7d6e5f4a".to_owned()),
                uncommitted,
            },
            state,
            since_ms: 1_794_458_400_000,
        }
    }

    fn words() -> Words {
        Words::of(&[
            ("from", "{session} \u{b7} {quest}"),
            ("runs", "Runs"),
            ("in", "In"),
            ("at", "{path} at {commit}"),
            ("changes", "Changes"),
            ("uncommitted", "{count} not committed"),
            ("givenAs", "Given as"),
            ("given", "{name}, for {minutes} min"),
            ("outcome", "Outcome"),
            ("ran", "ran, exit {exit}"),
        ])
    }

    #[test]
    fn a_waiting_ask_offers_both_acts_with_its_digest() {
        let row = row(&held(State::Waiting, Some(0)), "/home/joel", &words());
        let digest = row["digest"].as_str().expect("a digest");
        assert_eq!(digest.len(), 64, "the digest isn't SHA-256 in hex");
        assert_eq!(
            row["acts"][0]["ask"]["ask"], "release",
            "release isn't first"
        );
        assert_eq!(
            row["acts"][1]["ask"]["digest"], digest,
            "refuse lacks the digest"
        );
        assert_eq!(
            row["drawer"]["facts"][1]["said"],
            "~/src/tree at 4f2a91c03b1d"
        );
    }

    #[test]
    fn uncommitted_changes_are_warned_of_and_change_the_digest() {
        let clean = row(&held(State::Waiting, Some(0)), "", &words());
        let dirty = row(&held(State::Waiting, Some(2)), "", &words());
        assert_ne!(
            clean["digest"], dirty["digest"],
            "the digest missed the changes"
        );
        let facts = dirty["drawer"]["facts"].as_array().expect("facts");
        let warned = facts
            .iter()
            .find(|fact| fact["name"] == "Changes")
            .expect("a fact");
        assert_eq!(warned["said"], "2 not committed", "the count wasn't said");
        assert_eq!(warned["hue"], "warning", "the changes weren't warned of");
        let calm = clean["drawer"]["facts"].as_array().expect("facts");
        assert!(
            calm.iter().all(|fact| fact["name"] != "Changes"),
            "a clean tree warned"
        );
    }

    #[test]
    fn an_answered_ask_offers_nothing_and_says_what_came_of_it() {
        let row = row(&held(State::Ran { exit: Some(0) }, Some(0)), "", &words());
        assert_eq!(row["acts"], json!([]), "an answered ask still offered acts");
        let facts = row["drawer"]["facts"].as_array().expect("facts");
        assert_eq!(facts.last().expect("a fact")["said"], "ran, exit 0");
    }

    #[test]
    fn a_word_with_a_space_is_quoted_as_one() {
        assert_eq!(
            quoted("plain-word"),
            "plain-word",
            "a plain word was quoted"
        );
        assert_eq!(
            quoted("two words"),
            "'two words'",
            "a spaced word wasn't quoted"
        );
        assert_eq!(
            quoted("it's"),
            r"'it'\''s'",
            "a quote inside wasn't escaped"
        );
    }
}
