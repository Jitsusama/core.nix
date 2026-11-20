//! The budgets sessions asked for in the last day that the harness
//! couldn't draw on, as spend-watch finds them in pi's logs, so a team's
//! work going on the default key shows on the card the hour it starts
//! rather than a day later, when the warehouse bills it. On 17 and 18
//! November two teams' work went on Joel's key that way and nothing said
//! so. Every budget asked for should be drawn on, so the target is none.

use super::unbilled::told;
use super::{Class, Measure, count};

/// The measure's key, from one hour to the next.
pub const KEY: &str = "budgets-left-unresolved";

/// What begins the key of each finding spend-watch's collection holds of
/// a budget its sessions couldn't draw on: its detector's key,
/// `unresolved:`, behind the collection's `finding:`.
const FINDING: &str = "finding:unresolved:";

/// The key of the one finding a spend-watch on hold writes, which says
/// nothing of any budget. A warehouse that stopped settling doesn't stop
/// this count, which reads the logs alone.
const UNJUDGED: [&str; 1] = ["held"];

/// What the measure says.
const SAYS: &str = "budgets sessions asked for and couldn't draw on (by spend-watch)";

/// What the measure says when there is no collection to read.
const UNREAD: &str = " (spend-watch unread, stopped, or on hold)";

/// Each budget spend-watch found its sessions couldn't draw on in the last
/// day, by the line its collection names it with, such as `2 sessions
/// couldn't draw on Teal`, leaving out those Joel marked as expected.
/// `collection` and `status` are spend-watch's `collection.json` and
/// `status.json`, read at `now`, in seconds; either unread, a stale
/// heartbeat or a hold leaves the measure unjudged rather than in.
#[must_use]
pub fn unresolved(
    collection: Option<&str>,
    status: Option<&str>,
    now: i64,
    owner: &str,
) -> Measure {
    let Some(behind) = told(collection, status, now, FINDING, &UNJUDGED) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The moment the tests read at, 07:00 on 19 November.
    const NOW: i64 = 1_795_089_600;

    /// A status whose heartbeat is `ago` seconds before `NOW`.
    fn beat(ago: i64) -> String {
        format!(r#"{{"v": 1, "heartbeat": {}.5}}"#, NOW - ago)
    }

    /// Two budgets sessions couldn't draw on, one more Joel marked
    /// expected, beside a budget billed under and a warehouse that stopped
    /// settling, which are other measures' business.
    const MISSED: &str = r#"{"findings": [
        {"key": "finding:unresolved:ochre", "name": "1 session couldn't draw on Ochre",
         "role": "warning"},
        {"key": "finding:unresolved:teal", "name": "2 sessions couldn't draw on Teal",
         "role": "warning"},
        {"key": "finding:unresolved:slate", "name": "1 session couldn't draw on Slate",
         "role": "expected"},
        {"key": "finding:unbilled:umber", "name": "Umber billed $0 of $90 logged"},
        {"key": "finding:unbilled-unsettled", "name": "the warehouse's bill stopped settling"}
    ]}"#;

    #[test]
    fn a_budget_sessions_could_not_draw_on_is_past_its_target() {
        let m = unresolved(Some(MISSED), Some(&beat(30)), NOW, "WCJQSG");
        assert_eq!(m.now, 2.0);
        assert_eq!(m.target, Some(0.0));
        assert!(m.past().is_some(), "two budgets missed is past none");
        assert_eq!(
            m.behind,
            [
                "1 session couldn't draw on Ochre",
                "2 sessions couldn't draw on Teal"
            ]
        );
    }

    #[test]
    fn every_budget_drawn_on_is_in() {
        let burn = r#"{"findings": [{"key": "finding:burn:x", "name": "A burn"}]}"#;
        let m = unresolved(Some(burn), Some(&beat(30)), NOW, "WCJQSG");
        assert_eq!(m.now, 0.0);
        assert_eq!(m.past(), None);
    }

    #[test]
    fn no_word_from_spend_watch_leaves_it_unjudged() {
        let held = r#"{"findings": [{"key": "held", "name": "Watch on hold"}]}"#;
        for collection in [None, Some("not json"), Some("{}"), Some(held)] {
            let m = unresolved(collection, Some(&beat(30)), NOW, "WCJQSG");
            assert_eq!(m.target, None, "{collection:?}");
            assert!(m.says.ends_with(UNREAD), "{collection:?}");
        }
        let m = unresolved(Some(MISSED), Some(&beat(16 * 60)), NOW, "WCJQSG");
        assert_eq!(m.target, None, "a stale heartbeat");
    }
}
