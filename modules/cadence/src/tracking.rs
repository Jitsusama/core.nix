//! Whose an ask in `mission/asks.jsonl` is. From 19 November J7XHQN keeps
//! that file as Joel's work-system wish list, and marks each record's
//! `tracking`: empty for an ask Joel made himself, `derived` for one the
//! list drew from his, `quarantine` for a quarantined test to mend by a
//! date, and `owner-tracked` for an ask outside the work system it carries
//! for another owner. Only his own say how fast his asks ship and slip;
//! the list's own are told apart, and owner-tracked ones are another
//! system's to measure.

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
