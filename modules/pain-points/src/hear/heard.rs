//! What the listener has heard, kept as a line a pain in heard.jsonl, and
//! how each stands against what others already carry: the asks and misses
//! ledgers his orchestrator keeps, which it only reads. A pain the ledgers
//! already hold is carried, and goes to no one again; one alike to others
//! heard this fortnight counts each of them as a hit, so what he hits most
//! ranks first.

use crate::words::{alike, same};
use serde::{Deserialize, Serialize};

/// How alike a pain must be to a ledger line to be the one it carries:
/// strict, as two pains about one tool ("too tight on the
/// right", "slow to open") share half their words yet aren't one ask.
const CARRIED_ALIKE: f64 = 0.75;

/// A ledger's words shorter than this are too short to find inside his.
const QUOTED_LEAST: usize = 16;

/// How long a pain is kept, and hits are counted over; one sent and not
/// yet closed is kept past it, as only its fix running, or its owner's
/// word that it was none, closes it.
pub const KEPT_MS: i64 = 14 * 86_400_000;

/// A landing its question's thread led to, or one naming its entry: the
/// commit it merged at, the first of what its note said, which names the
/// change, and whether it named the entry outright.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landing {
    pub commit: String,
    pub said: String,
    #[serde(default)]
    pub by_entry: bool,
}

/// One pain heard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heard {
    /// The session's id and the log entry's, which name it once.
    pub key: String,
    pub session: String,
    pub entry: String,
    pub at: i64,
    /// Whether he said it to the void.
    #[serde(default)]
    pub void: bool,
    pub words: String,
    pub kinds: Vec<String>,
    #[serde(default)]
    pub looking_at: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub failed: Vec<String>,
    /// How many pains heard this fortnight, this one with them, say the
    /// same.
    pub hits: usize,
    /// Whether a ledger line already carries it.
    pub carried: bool,
    /// The cell it was sent to, and when.
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub routed_at: Option<i64>,
    /// How many sends of it the node never took.
    #[serde(default)]
    pub tries: u32,
    /// When a note to the ledgers' keeper of it, sent to its owner, was
    /// first held for a node that wasn't answering: it goes in the next
    /// run's note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unnoted_since: Option<i64>,
    /// The peer that took it up, by answering or passing on its question,
    /// or the session's own cell, by answering him where he said it.
    #[serde(default)]
    pub taken_by: Option<String>,
    /// When the session's own cell answered him where he said it, when
    /// that is how it was taken rather than by a peer on its question's
    /// thread, so a take that proves wrong can be told apart, and a peer's
    /// reply named in its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answered_at: Option<i64>,
    /// The commit that closed it: a landing in its question's thread that
    /// a switch carried.
    #[serde(default)]
    pub closed_by: Option<String>,
    /// When a run first saw it closed, so one closed today counts with the
    /// day's however old, and stays kept a day for the ledger's check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<i64>,
    /// The short ids of its question's thread so far: each question sent
    /// of it, and each reply to one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread: Vec<String>,
    /// Whether a question of it held other pains too, which no one landing
    /// closes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shared: bool,
    /// Each landing its thread led to, kept until a switch carries one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub landed: Vec<Landing>,
    /// Why its owner said it was no pain, through `pain-points test
    /// --none` on its line of the misses ledger, which closes it as a
    /// landing the machine runs would.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub none: Option<String>,
}

impl Heard {
    /// Whether it is closed: by a landing the machine runs, or by its
    /// owner's word that it was no pain.
    #[must_use]
    pub const fn closed(&self) -> bool {
        self.closed_by.is_some() || self.none.is_some()
    }

    /// Whether it is a miss of ours for his ledger: sent to a cell, said
    /// where he wasn't in the void, carried by no ledger, and no stall.
    #[must_use]
    pub fn is_miss(&self) -> bool {
        self.to.is_some() && !self.carried && !self.void && !super::stalled::is(self)
    }

    /// What closed it, as its landing's note said it.
    #[must_use]
    pub fn fixed_by(&self) -> Option<&Landing> {
        let commit = self.closed_by.as_ref()?;
        self.landed.iter().find(|l| &l.commit == commit)
    }
}

/// A ledger line as the listener reads it: the words it carries, and the
/// cell it names as their owner, empty when it names none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub words: String,
    pub owner: String,
}

/// Each line of a ledger. A line carries its pain whether or not its ask
/// has landed: either way the keeper has it. A line this listener wrote
/// of a pain it sent carries nothing, or its own pains would read as
/// carried and leave the count.
pub fn ledger(text: &str) -> Vec<Line> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| line.get(super::missed::PAIN).is_none())
        .filter_map(|line| {
            let owner = line.get("owner").and_then(serde_json::Value::as_str);
            Some(Line {
                words: line.get("words")?.as_str()?.to_string(),
                owner: owner.unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// Whether a ledger line carries these words.
pub fn carried(words: &str, ledgers: &[Line]) -> bool {
    carrier(words, ledgers).is_some()
}

/// The first ledger line whose words are these words: alike, or quoted
/// inside them, a part at a time where the ledger cut him with "...".
pub fn carrier<'a>(words: &str, ledgers: &'a [Line]) -> Option<&'a Line> {
    let low = words.to_lowercase();
    ledgers.iter().find(|line| {
        alike(&line.words, words) > CARRIED_ALIKE
            || line
                .words
                .to_lowercase()
                .split("...")
                .map(str::trim)
                .filter(|part| part.len() >= QUOTED_LEAST)
                .any(|part| low.contains(part))
    })
}

/// Why each pain the misses ledger `text` holds was said to be none, by
/// its key: a line pain-points wrote of it whose owner gave why, with the
/// time they said it, as cadence reads a miss said none. A line giving
/// why with no time stays open there, so it does here.
pub fn said_none(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|line| {
            line.get("none_at_ms")
                .is_some_and(serde_json::Value::is_i64)
        })
        .filter_map(|line| {
            let key = line.get(super::missed::PAIN)?.as_str()?;
            let why = line.get("none")?.as_str()?.trim();
            (!why.is_empty()).then(|| (key.to_owned(), why.to_owned()))
        })
        .collect()
}

/// How many of `kept` say the same as `words`, counting this one.
pub fn hits(words: &str, kept: &[Heard]) -> usize {
    1 + kept
        .iter()
        .filter(|heard| same(&heard.words, words))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn heard(key: &str, words: &str) -> Heard {
        Heard {
            key: key.into(),
            session: "s".into(),
            entry: key.into(),
            at: 0,
            void: false,
            words: words.into(),
            kinds: vec!["corrected".into()],
            looking_at: None,
            tools: Vec::new(),
            failed: Vec::new(),
            hits: 1,
            carried: false,
            to: None,
            routed_at: None,
            tries: 0,
            unnoted_since: None,
            taken_by: None,
            answered_at: None,
            closed_by: None,
            closed_at: None,
            none: None,
            thread: Vec::new(),
            shared: false,
            landed: Vec::new(),
        }
    }

    #[test]
    fn a_ledger_gives_each_lines_words_and_owner() {
        let text = concat!(
            r#"{"at":"x","words":"the bar is too tight","owner":"Y","landed":"2026"}"#,
            "\n",
            r#"{"at":"x","words":"no window opened","what":"w","owner":"Z","check":"c"}"#,
            "\n",
            r#"{"at":"x","words":"still open ask","owner":"Z"}"#,
            "\n",
            r#"{"at":"x","words":"one this listener sent","owner":"Z","pain":"s/e1"}"#,
            "\nnot json\n"
        );
        assert_eq!(
            ledger(text),
            [
                ("the bar is too tight", "Y"),
                ("no window opened", "Z"),
                ("still open ask", "Z")
            ]
            .map(|(words, owner)| Line {
                words: words.into(),
                owner: owner.into()
            }),
            "a line's words or owner went astray, or a line it sent was read"
        );
        let ownerless = ledger(r#"{"at":"x","words":"no one named"}"#);
        assert_eq!(ownerless[0].owner, "", "a line naming no owner gave one");
    }

    fn said(words: &str) -> Line {
        Line {
            words: words.into(),
            owner: "ABC123".into(),
        }
    }

    #[test]
    fn a_pain_the_ledgers_quote_is_carried() {
        let ledgers = [
            said("i had sent a number of things to the void ... now going in shows me nothing"),
            said("the launcher is slow to open"),
        ];
        assert!(
            carried(
                "so i had sent a number of things to the void earlier, and now going in shows \
             me nothing at all",
                &ledgers
            ),
            "the whole of a ledger's words"
        );
        assert!(
            carried("I had sent a number of things to the void", &ledgers),
            "a part"
        );
        assert!(
            !carried("the bar is the wrong colour", &ledgers),
            "another pain"
        );
        assert_eq!(
            carrier("the launcher is slow to open", &ledgers),
            ledgers.get(1),
            "the line carrying a pain wasn't the one found"
        );
        let short = [said("no")];
        assert!(!carried("no, the other one", &short), "too short to find");
        let tight = [said("the launcher is too tight on the right")];
        assert!(
            !carried("the launcher is slow to open", &tight),
            "another pain about the same tool"
        );
    }

    #[test]
    fn what_he_hits_again_counts_each_time() {
        let kept = vec![
            heard("a", "the launcher is slow to open"),
            heard("b", "launcher slow to open again"),
            heard("c", "make the bar green"),
        ];
        assert_eq!(hits("the launcher is so slow to open", &kept), 3);
        assert_eq!(hits("nothing like the others at all", &kept), 1);
    }
}
