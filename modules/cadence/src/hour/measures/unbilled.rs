//! The budgets billed under half what pi's logs priced their sessions at,
//! as spend-watch finds them, so a team's work spent on another key shows
//! on the card each hour rather than when someone thinks to compare. From
//! 23:00 on 17 November to 19:00 on the 18th, two team budgets' sessions
//! logged $170.55 and $375.60 and their keys were billed nothing and $2.67,
//! because the pi host sent its own key for every conversation. Every
//! budget should be billed what it spends, so the target is none.

use serde_json::Value;

use super::{Class, Measure, count};

/// The measure's key, from one hour to the next.
pub const KEY: &str = "budgets-billed-under-logged";

/// What begins the key of each finding spend-watch's collection holds of
/// a budget billed under what it logged: its detector's key, `unbilled:`,
/// behind the collection's `finding:`.
const FINDING: &str = "finding:unbilled:";

/// The key of the one finding a spend-watch on hold writes, which says
/// nothing of any budget.
const HELD: &str = "held";

/// What the measure says.
const SAYS: &str = "budgets billed under half what their sessions logged (by spend-watch)";

/// What the measure says when there is no collection to read.
const UNREAD: &str = " (spend-watch unread, on hold, or not written today)";

/// Each budget spend-watch found billed under half what it logged, by the
/// line its collection names it with, such as `Teal billed $27 of $250
/// logged`, leaving out those Joel marked as expected. `collection` is
/// spend-watch's `collection.json`, or `None` where none is fresh; that,
/// or a spend-watch on hold, leaves the measure unjudged rather than in.
#[must_use]
pub fn unbilled(collection: Option<&str>, owner: &str) -> Measure {
    let read = collection.and_then(|text| serde_json::from_str::<Value>(text).ok());
    let findings = read
        .as_ref()
        .and_then(|r| r.get("findings")?.as_array())
        .filter(|findings| !findings.iter().any(|f| key(f) == Some(HELD)));
    let Some(findings) = findings else {
        return Measure {
            key: KEY.to_owned(),
            class: Class::Functional,
            says: format!("{SAYS}{UNREAD}"),
            now: 0.0,
            target: None,
            owner: owner.to_owned(),
            behind: Vec::new(),
        };
    };
    let behind: Vec<String> = findings
        .iter()
        .filter(|f| key(f).is_some_and(|key| key.starts_with(FINDING)))
        .filter(|f| f.get("role").and_then(Value::as_str) != Some("expected"))
        .filter_map(|f| f.get("name")?.as_str().map(str::to_owned))
        .collect();
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: SAYS.to_owned(),
        now: count(behind.len()),
        target: Some(0.0),
        owner: owner.to_owned(),
        behind,
    }
}

fn key(finding: &Value) -> Option<&str> {
    finding.get("key")?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As spend-watch wrote it for 17 to 18 November, the teams renamed:
    /// two budgets billed under what they logged, one more Joel marked
    /// expected, beside findings of other kinds.
    const UNDER: &str = r#"{"findings": [
        {"key": "finding:said:6c9d9cf995971746", "name": "The provider's meter says: over"},
        {"key": "finding:unbilled:ochre", "name": "Ochre billed $3 of $376 logged",
         "role": "warning"},
        {"key": "finding:unbilled:teal", "name": "Teal billed $0 of $171 logged",
         "role": "warning"},
        {"key": "finding:unbilled:slate", "name": "Slate billed $1 of $90 logged",
         "role": "expected"},
        {"key": "finding:burn:2026-11-18", "name": "A burn in s1"}
    ]}"#;

    #[test]
    fn a_budget_billed_under_what_it_logged_is_past_its_target() {
        let m = unbilled(Some(UNDER), "WCJQSG");
        assert_eq!(m.now, 2.0);
        assert_eq!(m.target, Some(0.0));
        assert!(m.past().is_some(), "two budgets unbilled is past none");
        assert_eq!(
            m.behind,
            [
                "Ochre billed $3 of $376 logged",
                "Teal billed $0 of $171 logged"
            ]
        );
    }

    #[test]
    fn every_budget_billed_is_in() {
        let burn = r#"{"findings": [{"key": "finding:burn:x", "name": "A burn"}]}"#;
        let m = unbilled(Some(burn), "WCJQSG");
        assert_eq!(m.now, 0.0);
        assert_eq!(m.past(), None);
        assert_eq!(m.behind, [] as [String; 0]);
    }

    #[test]
    fn no_collection_leaves_it_unjudged() {
        let held = r#"{"findings": [{"key": "held", "name": "Watch on hold"}]}"#;
        for collection in [None, Some("not json"), Some("{}"), Some(held)] {
            let m = unbilled(collection, "WCJQSG");
            assert_eq!(m.target, None, "{collection:?}");
            assert!(m.says.ends_with(UNREAD), "{collection:?}");
        }
    }
}
