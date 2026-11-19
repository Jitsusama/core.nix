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

/// The keys of the findings that say no budget was judged: the one a
/// spend-watch on hold writes, and the one saying the warehouse stopped
/// settling its bill.
const UNJUDGED: [&str; 2] = ["held", "finding:unbilled-unsettled"];

/// How old spend-watch's heartbeat may be for its collection to be its
/// word now, in seconds. It beats every thirty seconds, and stops beating
/// eleven minutes after its last pass that worked, so a collection it
/// rewrites only when its text changes is current while the beat is.
const BEAT_WITHIN: f64 = 15.0 * 60.0;

/// What the measure says.
const SAYS: &str = "budgets billed under half what their sessions logged (by spend-watch)";

/// What the measure says when there is no collection to read.
const UNREAD: &str = " (spend-watch unread, stopped, on hold, or its bill unsettled)";

/// Each budget spend-watch found billed under half what it logged, by the
/// line its collection names it with, such as `Teal billed $27 of $250
/// logged`, leaving out those Joel marked as expected. `collection` is
/// spend-watch's `collection.json` and `status` its `status.json`, read
/// at `now`, in seconds. A collection unread, a heartbeat stale or
/// missing, a spend-watch on hold or a warehouse no longer settling
/// leaves the measure unjudged rather than in.
#[must_use]
pub fn unbilled(collection: Option<&str>, status: Option<&str>, now: i64, owner: &str) -> Measure {
    let told = told(collection, status, now, FINDING, &UNJUDGED);
    let Some(behind) = told else {
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

/// The name of each finding spend-watch's collection holds whose key
/// begins `finding`, leaving out those Joel marked as expected; none
/// where the collection can't be read as spend-watch's word now: unread,
/// its heartbeat stale or missing, or holding one of the `unjudged` keys.
pub(super) fn told(
    collection: Option<&str>,
    status: Option<&str>,
    now: i64,
    finding: &str,
    unjudged: &[&str],
) -> Option<Vec<String>> {
    let beating = status
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|status| status.get("heartbeat")?.as_f64())
        // A beat ahead of the clock by more than the quarter hour is no
        // more a sign of life than one behind it.
        .is_some_and(|beat| (beat - seconds(now)).abs() <= BEAT_WITHIN);
    let read = collection
        .filter(|_| beating)
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    let findings = read
        .as_ref()
        .and_then(|r| r.get("findings")?.as_array())
        .filter(|findings| {
            !findings
                .iter()
                .any(|f| key(f).is_some_and(|key| unjudged.contains(&key)))
        })?;
    Some(
        findings
            .iter()
            .filter(|f| key(f).is_some_and(|key| key.starts_with(finding)))
            .filter(|f| f.get("role").and_then(Value::as_str) != Some("expected"))
            .filter_map(|f| f.get("name")?.as_str().map(str::to_owned))
            .collect(),
    )
}

fn key(finding: &Value) -> Option<&str> {
    finding.get("key")?.as_str()
}

/// `now` as the heartbeat counts it; seconds since 1970 are exact in an
/// f64 for millennia.
fn seconds(now: i64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "exact for any time now")]
    let seconds = now as f64;
    seconds
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

    /// The moment the tests read at, 07:00 on 19 November.
    const NOW: i64 = 1_795_089_600;

    /// A status whose heartbeat is `ago` seconds before `NOW`.
    fn beat(ago: i64) -> String {
        format!(r#"{{"v": 1, "heartbeat": {}.5}}"#, NOW - ago)
    }

    #[test]
    fn a_budget_billed_under_what_it_logged_is_past_its_target() {
        let m = unbilled(Some(UNDER), Some(&beat(30)), NOW, "WCJQSG");
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
        let m = unbilled(Some(burn), Some(&beat(30)), NOW, "WCJQSG");
        assert_eq!(m.now, 0.0);
        assert_eq!(m.past(), None);
        assert_eq!(m.behind, [] as [String; 0]);
    }

    #[test]
    fn no_collection_leaves_it_unjudged() {
        let held = r#"{"findings": [{"key": "held", "name": "Watch on hold"}]}"#;
        let unsettled = r#"{"findings": [{"key": "finding:unbilled-unsettled",
            "name": "the warehouse's bill stopped settling"}]}"#;
        for collection in [
            None,
            Some("not json"),
            Some("{}"),
            Some(held),
            Some(unsettled),
        ] {
            let m = unbilled(collection, Some(&beat(30)), NOW, "WCJQSG");
            assert_eq!(m.target, None, "{collection:?}");
            assert!(m.says.ends_with(UNREAD), "{collection:?}");
        }
    }

    // A spend-watch that stopped, or whose passes stopped working, stops
    // beating, and its last collection isn't its word now.
    #[test]
    fn a_stopped_spend_watch_leaves_it_unjudged() {
        let stale = beat(16 * 60);
        let ahead = beat(-16 * 60);
        for status in [
            None,
            Some("{}"),
            Some("not json"),
            Some(stale.as_str()),
            Some(ahead.as_str()),
        ] {
            let m = unbilled(Some(UNDER), status, NOW, "WCJQSG");
            assert_eq!(m.target, None, "{status:?}");
            assert!(m.says.ends_with(UNREAD), "{status:?}");
        }
        let m = unbilled(Some(UNDER), Some(&beat(14 * 60)), NOW, "WCJQSG");
        assert_eq!(m.now, 2.0, "a beat within the quarter hour is current");
    }
}
