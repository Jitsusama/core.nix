//! Whether the card's red lines fall: the measures past their targets this
//! hour against the same hour a week before. On 26 November Joel asked
//! that "we have various self improvement loops built in"; a loop that
//! only counts can show the same red lines every hour for a week, so the
//! loop itself is judged by its red lines falling, and its keeper is asked
//! when they haven't.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use super::measures::{Class, Measure};
use crate::DAY;

/// The measure's key, from one hour to the next.
pub const KEY: &str = "red-lines-falling";

/// Who answers for the loop's own cadence.
const KEEPER: &str = "1JQPPC";

/// How far back the hour it is held against lies.
const WEEK: i64 = 7 * DAY;

/// How far from a week back a kept hour may lie and still stand for it,
/// so a timer firing late or an hour missed doesn't leave it unjudged.
const NEAR: i64 = 90 * 60;

/// How long a kept hour is kept: a week, and the slack beside it.
const KEPT_FOR: i64 = WEEK + NEAR;

/// One hour's count, kept a week so a later hour is held against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hour {
    /// When, in seconds.
    pub at: i64,
    /// How many measures were past their targets.
    pub red: usize,
    /// How many were unjudged, as one whose figure couldn't be read is,
    /// so a line gone unread isn't taken for a line fixed.
    pub unjudged: usize,
    /// How many measures there were at all, so a source that made none
    /// this hour, as code health's file unread, isn't taken for its red
    /// lines fixed. An hour kept before this was counted reads nought.
    #[serde(default)]
    pub seen: usize,
}

/// This hour's count at `now`, in seconds, of `measures`, this one left
/// out, since it can't count itself.
#[must_use]
pub fn hour(measures: &[Measure], now: i64) -> Hour {
    let others = || measures.iter().filter(|m| m.key != KEY);
    Hour {
        at: now,
        red: others().filter(|m| m.past().is_some()).count(),
        unjudged: others().filter(|m| m.target.is_none()).count(),
        seen: others().count(),
    }
}

/// The measure for `this` hour, against the hours `kept`. It must hold
/// fewer red lines than the hour nearest a week back, or none where none
/// were. With no hour kept near enough a week back it's shown unjudged
/// until one is, and so it is while more measures go unjudged than then,
/// or fewer are made at all, since a line that couldn't be read, or whose
/// source made none, leaves the red ones and would read as falling while
/// nothing was looked at.
#[must_use]
pub fn falling(kept: &[Hour], this: Hour) -> Measure {
    let now = this.at;
    let then = kept
        .iter()
        .filter(|h| (h.at - (now - WEEK)).abs() <= NEAR)
        .min_by_key(|h| (h.at - (now - WEEK)).abs());
    let since = kept.iter().map(|h| h.at).min();
    let unread = |then: &Hour| this.unjudged + then.seen.saturating_sub(this.seen);
    let (target, behind) = match (then, since) {
        (Some(then), _) if unread(then) > then.unjudged => (
            None,
            vec![format!(
                "unjudged while more measures go unjudged or unmade than a week back, at {}: \
                 {} then, {} this hour",
                when(then.at),
                then.unjudged,
                unread(then)
            )],
        ),
        (Some(then), _) => (
            Some(count(then.red.saturating_sub(1))),
            vec![format!(
                "a week back, at {}: {}; this hour: {}",
                when(then.at),
                then.red,
                this.red
            )],
        ),
        (None, Some(since)) => (
            None,
            vec![format!(
                "unjudged until a week is kept; kept since {}",
                when(since)
            )],
        ),
        (None, None) => (None, vec!["unjudged until a week is kept".to_owned()]),
    };
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: "measures past their targets this hour, which must be fewer than a week back"
            .to_owned(),
        now: count(this.red),
        target,
        owner: KEEPER.to_owned(),
        behind,
    }
}

/// When `at`, in seconds, was, as a moment in UTC, since it lies a week
/// back, where the card's local clock alone would leave the day unsaid.
fn when(at: i64) -> String {
    Timestamp::from_second(at).map_or_else(|_| at.to_string(), |t| t.to_string())
}

#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
const fn count(n: usize) -> f64 {
    n as f64
}

/// The hours to keep after `this` one: those `kept` within a week and its
/// slack before it, and this one.
#[must_use]
pub fn kept(kept: &[Hour], this: Hour) -> Vec<Hour> {
    let mut next: Vec<Hour> = kept
        .iter()
        .copied()
        .filter(|h| h.at > this.at - KEPT_FOR && h.at < this.at)
        .collect();
    next.push(this);
    next
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn past(key: &str) -> Measure {
        Measure {
            key: key.to_owned(),
            class: Class::Ship,
            says: String::new(),
            now: 2.0,
            target: Some(0.0),
            owner: "OWNER".to_owned(),
            behind: Vec::new(),
        }
    }

    const fn at(at: i64, red: usize) -> Hour {
        Hour {
            at,
            red,
            unjudged: 0,
            seen: 20,
        }
    }

    #[test]
    fn the_lines_must_fall_below_the_hour_a_week_back_or_their_keeper_is_asked() {
        let week = [at(NOW - WEEK - 600, 5), at(NOW - DAY, 9)];
        let same = falling(&week, at(NOW, 5));
        assert!(
            same.past().is_some() && same.owner == KEEPER,
            "as many red lines as a week back read as falling: {same:?}"
        );
        let fewer = falling(&week, at(NOW, 4));
        assert_eq!(
            fewer.past(),
            None,
            "fewer red lines than a week back read as not falling"
        );
        let none = falling(&[at(NOW - WEEK, 0)], at(NOW, 0));
        assert_eq!(
            none.past(),
            None,
            "no red lines a week back and none now read as past"
        );
    }

    #[test]
    fn the_hour_nearest_a_week_back_stands_for_it_within_its_slack() {
        let hours = [
            at(NOW - WEEK - NEAR, 1),
            at(NOW - WEEK + 300, 9),
            at(NOW - WEEK + NEAR, 1),
        ];
        let judged = falling(&hours, at(NOW, 5));
        assert_eq!(
            judged.target,
            Some(8.0),
            "an hour further from a week back stood for it: {judged:?}"
        );
        let edge = falling(&[at(NOW - WEEK - NEAR, 2)], at(NOW, 5));
        assert_eq!(
            edge.target,
            Some(1.0),
            "an hour at the edge of the slack didn't stand for a week back"
        );
    }

    #[test]
    fn with_no_hour_kept_near_a_week_back_it_is_shown_unjudged() {
        let young = falling(&[at(NOW - DAY, 3)], at(NOW, 7));
        assert!(
            young.target.is_none() && young.behind[0].contains("kept since"),
            "a measure with no week kept was judged: {young:?}"
        );
        let far = falling(&[at(NOW - WEEK - 3 * 3600, 3)], at(NOW, 7));
        assert!(
            far.target.is_none(),
            "an hour three hours off a week back stood for it"
        );
    }

    #[test]
    fn lines_gone_unread_since_a_week_back_leave_it_unjudged_not_falling() {
        let then = Hour {
            at: NOW - WEEK,
            red: 9,
            unjudged: 2,
            seen: 20,
        };
        let unread = Hour {
            at: NOW,
            red: 3,
            unjudged: 8,
            seen: 20,
        };
        let judged = falling(&[then], unread);
        assert!(
            judged.target.is_none() && judged.behind[0].contains("more measures go unjudged"),
            "red lines gone unread read as falling: {judged:?}"
        );
        let read = Hour {
            unjudged: 2,
            ..unread
        };
        assert_eq!(
            falling(&[then], read).target,
            Some(8.0),
            "as many unjudged as a week back left it unjudged"
        );
        let unmade = Hour {
            unjudged: 2,
            seen: 14,
            ..unread
        };
        let judged = falling(&[then], unmade);
        assert!(
            judged.target.is_none() && judged.behind[0].contains("2 then, 8 this hour"),
            "red lines whose source made no measures read as falling: {judged:?}"
        );
        let before = Hour { seen: 0, ..then };
        assert_eq!(
            falling(&[before], unmade).target,
            Some(8.0),
            "an hour kept before measures were counted left it unjudged"
        );
    }

    #[test]
    fn it_counts_every_measure_but_itself_and_keeps_a_week_of_hours() {
        let mut unjudged = past("c");
        unjudged.target = None;
        let measures = [past("a"), past("b"), past(KEY), unjudged];
        let this = hour(&measures, NOW);
        assert_eq!(
            (this.red, this.unjudged, this.seen),
            (2, 1, 3),
            "the measure counted itself, or missed one past, unjudged or made"
        );
        let old = [at(NOW - KEPT_FOR - 1, 1), at(NOW - DAY, 2), at(NOW + 60, 3)];
        assert_eq!(
            kept(&old, at(NOW, 4)),
            [at(NOW - DAY, 2), at(NOW, 4)],
            "an hour past a week and its slack, or one after now, was kept"
        );
    }
}
