//! Whose an ask in `mission/asks.jsonl` is. From 19 November J7XHQN keeps
//! that file as Joel's work-system wish list, and marks each record's
//! `tracking`: empty for an ask Joel made himself, `derived` for one the
//! list drew from his, `quarantine` for a quarantined test to mend by a
//! date, and `owner-tracked` for an ask outside the work system it carries
//! for another owner. Only his own say how fast his asks ship and slip;
//! the list's own are told apart, and owner-tracked ones are another
//! system's to measure.

use std::collections::BTreeSet;

use jiff::Timestamp;
use serde_json::Value;

/// Which of the list's kinds an ask is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracking {
    /// One Joel made himself.
    His,
    /// One the list set itself: drawn from his, or a quarantine's mend.
    Derived,
    /// One carried for an owner outside the work system.
    Carried,
}

impl Tracking {
    /// A measure's key for this kind, from its key for his own asks, so
    /// his keep the keys they had before the list told kinds apart.
    #[must_use]
    pub fn key(self, his: &str) -> String {
        match self {
            Self::His => his.to_owned(),
            Self::Derived => format!("{his}-derived"),
            Self::Carried => format!("{his}-carried"),
        }
    }
}

/// The kind `ask` is, or none when it was closed without landing, which
/// the list does to an ask it put down, so it neither waits nor slips. A
/// kind the list doesn't name is taken as carried, outside his own.
#[must_use]
pub fn of(ask: &Value) -> Option<Tracking> {
    if ask["closed"].as_str().is_some_and(|c| !c.is_empty()) && ask["landed"].is_null() {
        return None;
    }
    Some(match ask["tracking"].as_str().unwrap_or_default() {
        "" => Tracking::His,
        "derived" | "quarantine" => Tracking::Derived,
        _ => Tracking::Carried,
    })
}

/// The list's lines but its twins: a line the keeper folded into another
/// ask with the same moment and words, its id marked `~`. Every reader of
/// the list keeps an ask by its latest line, so a twin written after the
/// ask it copies would stand in its place. A twin whose ask isn't in the
/// list is the only line of that ask, so it is kept.
#[must_use]
pub fn untwinned(lines: Vec<Value>) -> Vec<Value> {
    let marked = |ask: &Value| ask["id"].as_str().is_some_and(|id| id.contains('~'));
    let copied: BTreeSet<[String; 2]> = lines
        .iter()
        .filter(|ask| !marked(ask))
        .map(|ask| asked(ask))
        .collect();
    lines
        .into_iter()
        .filter(|ask| !marked(ask) || !copied.contains(&asked(ask)))
        .collect()
}

/// Which ask a line of the list is, as every reader of the list keys it:
/// the moment it was asked, as seconds when it reads as a time, so one
/// moment written two ways is one ask, and its words. Later lines of the
/// same ask stand for it.
#[must_use]
pub fn asked(ask: &Value) -> [String; 2] {
    let at = ask["at"]
        .as_str()
        .and_then(|at| at.parse::<Timestamp>().ok())
        .map_or_else(|| ask["at"].to_string(), |at| at.as_second().to_string());
    let words = ask["words"]
        .as_str()
        .map_or_else(|| ask["words"].to_string(), str::to_owned);
    [at, words]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn one_moment_written_two_ways_is_one_ask_and_other_words_another() {
        let utc = json!({"at": "2026-11-13T17:00:00Z", "words": "fix it"});
        let local = json!({"at": "2026-11-13T12:00:00-05:00", "words": "fix it"});
        let other = json!({"at": "2026-11-13T12:00:00-05:00", "words": "fix that"});
        assert_eq!(asked(&utc), asked(&local), "one moment read as two asks");
        assert_ne!(
            asked(&local),
            asked(&other),
            "other words read as the same ask"
        );
    }

    #[test]
    fn a_twin_is_left_out_beside_its_ask_and_kept_where_it_is_the_only_line() {
        let ask = json!({"id": "0000000a", "at": "t1", "words": "fix it"});
        let twin = json!({"id": "0000000a~1", "at": "t1", "words": "fix it", "closed": "t2"});
        let alone = json!({"id": "0000000b~1", "at": "t3", "words": "fix that"});
        let other = json!({"id": "0000000c", "at": "t1", "words": "fix this"});
        assert_eq!(
            untwinned(vec![ask.clone(), twin, alone.clone(), other.clone()]),
            [ask, alone, other],
            "a twin beside its ask was read, or a lone one or another ask dropped"
        );
    }

    #[test]
    fn an_ask_is_his_unless_the_list_marks_it_otherwise() {
        assert_eq!(of(&json!({"words": "fix it"})), Some(Tracking::His));
        assert_eq!(of(&json!({"tracking": null})), Some(Tracking::His));
        assert_eq!(of(&json!({"tracking": ""})), Some(Tracking::His));
        assert_eq!(of(&json!({"tracking": "derived"})), Some(Tracking::Derived));
        assert_eq!(
            of(&json!({"tracking": "quarantine"})),
            Some(Tracking::Derived)
        );
        assert_eq!(
            of(&json!({"tracking": "owner-tracked"})),
            Some(Tracking::Carried)
        );
        assert_eq!(
            of(&json!({"tracking": "elsewhere"})),
            Some(Tracking::Carried),
            "a kind the list doesn't name is no ask of his"
        );
    }

    #[test]
    fn an_ask_closed_without_landing_is_none_and_one_landed_stays() {
        assert_eq!(of(&json!({"closed": "put down", "landed": null})), None);
        assert_eq!(
            of(&json!({"closed": "a standing rule", "landed": "2026-11-19T15:20:00-05:00"})),
            Some(Tracking::His)
        );
        assert_eq!(
            of(&json!({"closed": "", "landed": null})),
            Some(Tracking::His)
        );
    }
}
