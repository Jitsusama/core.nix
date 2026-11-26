//! Whether the card's red lines fall: the measures past their targets this
//! hour against the same hour a week before. On 26 November Joel asked
//! that "we have various self improvement loops built in"; a loop that
//! only counts can show the same red lines every hour for a week, so the
//! loop itself is judged by its red lines falling, and its keeper is asked
//! when they haven't.

use jiff::Timestamp;

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

/// How many measures are past their targets among `measures`, this one
/// left out, since it can't count itself.
#[must_use]
pub fn red(measures: &[Measure]) -> usize {
    measures
        .iter()
        .filter(|m| m.key != KEY && m.past().is_some())
        .count()
}

/// The measure at `now`, in seconds, with `red` lines this hour and the
/// hours `kept`, each as when and how many. It must be fewer than the hour
/// nearest a week back, or none where none were; with no hour kept near
/// enough a week back, it's shown unjudged until one is.
#[must_use]
pub fn falling(kept: &[(i64, usize)], red: usize, now: i64) -> Measure {
    let then = kept
        .iter()
        .filter(|&&(at, _)| (at - (now - WEEK)).abs() <= NEAR)
        .min_by_key(|&&(at, _)| (at - (now - WEEK)).abs());
    let since = kept.iter().map(|&(at, _)| at).min();
    let (target, behind) = match (then, since) {
        (Some(&(at, was)), _) => (
            Some(count(was.saturating_sub(1))),
            vec![format!("a week back, at {}: {was}; this hour: {red}", when(at))],
        ),
        (None, Some(since)) => (
            None,
            vec![format!("unjudged until a week is kept; kept since {}", when(since))],
        ),
        (None, None) => (None, vec!["unjudged until a week is kept".to_owned()]),
    };
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: "measures past their targets this hour, which must be fewer than a week back"
            .to_owned(),
        now: count(red),
        target,
        owner: KEEPER.to_owned(),
        behind,
    }
}

/// When `at`, in seconds, was, as the card says it.
fn when(at: i64) -> String {
    Timestamp::from_second(at).map_or_else(|_| at.to_string(), |t| t.to_string())
}

#[expect(clippy::cast_precision_loss, reason = "counts stay far below 2^52")]
const fn count(n: usize) -> f64 {
    n as f64
}

/// The hours to keep after `now`'s: those `kept` within a week and its
/// slack, and this one.
#[must_use]
pub fn kept(kept: &[(i64, usize)], red: usize, now: i64) -> Vec<(i64, usize)> {
    let mut next: Vec<(i64, usize)> = kept
        .iter()
        .copied()
        .filter(|&(at, _)| at > now - KEPT_FOR && at < now)
        .collect();
    next.push((now, red));
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

    #[test]
    fn the_lines_must_fall_below_the_hour_a_week_back_or_their_keeper_is_asked() {
        let week = [(NOW - WEEK - 600, 5), (NOW - DAY, 9)];
        let same = falling(&week, 5, NOW);
        assert!(
            same.past().is_some() && same.owner == KEEPER,
            "as many red lines as a week back read as falling: {same:?}"
        );
        let fewer = falling(&week, 4, NOW);
        assert_eq!(fewer.past(), None, "fewer red lines than a week back read as not falling");
        let none = falling(&[(NOW - WEEK, 0)], 0, NOW);
        assert_eq!(none.past(), None, "no red lines a week back and none now read as past");
    }

    #[test]
    fn with_no_hour_kept_near_a_week_back_it_is_shown_unjudged() {
        let young = falling(&[(NOW - DAY, 3)], 7, NOW);
        assert!(
            young.target.is_none() && young.behind[0].contains("kept since"),
            "a measure with no week kept was judged: {young:?}"
        );
        let far = falling(&[(NOW - WEEK - 3 * 3600, 3)], 7, NOW);
        assert!(far.target.is_none(), "an hour three hours off a week back stood for it");
    }

    #[test]
    fn it_counts_every_measure_past_but_itself_and_keeps_a_week_of_hours() {
        let measures = [past("a"), past("b"), past(KEY)];
        assert_eq!(red(&measures), 2, "the measure counted itself, or missed one past");
        let old = [(NOW - KEPT_FOR - 1, 1), (NOW - DAY, 2), (NOW + 60, 3)];
        assert_eq!(
            kept(&old, 4, NOW),
            [(NOW - DAY, 2), (NOW, 4)],
            "an hour past a week and its slack, or one after now, was kept"
        );
    }
}
