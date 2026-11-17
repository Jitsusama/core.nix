//! What the listener has heard, kept as a line a pain in heard.jsonl, and
//! how each stands against what others already carry: the asks and misses
//! ledgers his orchestrator keeps, which it only reads. A pain the ledgers
//! already hold is carried, and goes to no one again; one alike to others
//! heard this fortnight counts each of them as a hit, so what he hits most
//! ranks first.

use crate::words::alike;
use serde::{Deserialize, Serialize};

/// How alike two pains' words must be to be the same friction hit again.
pub const SAME: f64 = 0.5;

/// A ledger's words shorter than this are too short to find inside his.
const QUOTED_LEAST: usize = 16;

/// How long a pain is kept, and hits are counted over.
pub const KEPT_MS: i64 = 14 * 86_400_000;

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
}

/// The words of each line of a ledger, and whether that line says its ask
/// landed or its miss has a check.
pub fn ledger(text: &str) -> Vec<(String, bool)> {
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|line| {
            let words = line.get("words")?.as_str()?.to_string();
            let closed = ["landed", "check"]
                .iter()
                .any(|key| line.get(key).is_some_and(|v| !v.is_null()));
            Some((words, closed))
        })
        .collect()
}

/// Whether a ledger line's words are these words: alike, or quoted
/// inside them, a part at a time where the ledger cut him with "...".
pub fn carried(words: &str, ledgers: &[(String, bool)]) -> bool {
    let low = words.to_lowercase();
    ledgers.iter().any(|(said, _)| {
        alike(said, words) > SAME
            || said
                .to_lowercase()
                .split("...")
                .map(str::trim)
                .filter(|part| part.len() >= QUOTED_LEAST)
                .any(|part| low.contains(part))
    })
}

/// How many of `kept` say the same as `words`, counting this one.
pub fn hits(words: &str, kept: &[Heard]) -> usize {
    1 + kept
        .iter()
        .filter(|heard| alike(&heard.words, words) > SAME)
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
        }
    }

    #[test]
    fn a_ledger_gives_each_lines_words_and_whether_it_closed() {
        let text = concat!(
            r#"{"at":"x","words":"the bar is too tight","owner":"Y","landed":"2026"}"#,
            "\n",
            r#"{"at":"x","words":"no window opened","what":"w","owner":"Z","check":"c"}"#,
            "\n",
            r#"{"at":"x","words":"still open ask","owner":"Z"}"#,
            "\nnot json\n"
        );
        assert_eq!(
            ledger(text),
            vec![
                ("the bar is too tight".to_string(), true),
                ("no window opened".to_string(), true),
                ("still open ask".to_string(), false),
            ]
        );
    }

    #[test]
    fn a_pain_the_ledgers_quote_is_carried() {
        let ledgers = vec![(
            "i had sent a number of things to the void ... now going in shows me nothing"
                .to_string(),
            false,
        )];
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
            !carried("the launcher is slow to open", &ledgers),
            "another pain"
        );
        let short = vec![("no".to_string(), false)];
        assert!(!carried("no, the other one", &short), "too short to find");
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
