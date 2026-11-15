//! Which access point of its network a card should be on, from what each
//! carried from here (measure.rs), changed rarely and never during a call.
//!
//! The best access point changes with the hour and the room (Joel, 14 Nov
//! 2026: "fixating on one could hurt me"), so none is fixed: the one the
//! card is on is kept while it carries well, and another is tried only
//! when it carries badly, or when one measured lately carried far better.
//! wpa_supplicant still roams on its own when the signal fades; this
//! moves the card only between access points it can already hear.
//!
//! The figures to beat are a desk's on 14 Nov 2026, one access point's
//! two radios at 4 dB apart: the gateway's p95 was 15 ms on one and
//! 119 ms on the other, with 2% and 31% of ten-second spells over 150 ms.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

use crate::measure::Measured;

/// How long after a move the card stays put, so it never hunts.
pub const SETTLE: Duration = Duration::from_secs(600);

/// How long a move is on trial: worse by then, and the card goes back.
pub const TRIAL: Duration = Duration::from_secs(300);

/// How long an access point found worse from here is left untried.
pub const SPURNED: Duration = Duration::from_secs(3600);

/// Past either, an access point carries badly enough to try another: a
/// p95 near three times the better radio's on 14 Nov, or a loss a call
/// hears.
const SLOW: Duration = Duration::from_millis(40);
const LOSSY: f64 = 2.0;

/// Past this loss the link is failing, and it's left even during a call.
const FAILING: f64 = 10.0;

/// How much fainter than the one it's on an untried access point may be
/// heard and still be tried: about a quarter of the power.
const FAINTER: i16 = 6;

/// How busy an untried access point's channel may say it is and still be
/// tried.
const BUSIEST: u8 = 70;

/// How much quicker one measured lately must have been to be moved to:
/// both, so neither a small figure's ratio nor a large one's few
/// milliseconds moves the card.
const QUICKER_BY: f64 = 0.7;
const QUICKER: Duration = Duration::from_millis(10);

/// How much worse a trial may carry before the card goes back.
const WORSE: Duration = Duration::from_millis(10);
const WORSE_LOSS: f64 = 2.0;

/// What another access point of the network is known by.
#[derive(Clone, Debug, PartialEq)]
pub struct Other {
    pub point: String,
    /// How loud it's heard, in dBm.
    pub signal: i16,
    /// How busy it says its channel is, in percent, where it says.
    pub busy: Option<u8>,
    /// What it carried while the card was last on it, within the span.
    pub measured: Option<Measured>,
}

/// What the card should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    Stay,
    /// Move to this access point, on trial.
    Try(String),
    /// Go back to the access point the trial left, which carried better.
    Back(String),
}

/// What's been chosen lately, to keep the card from hunting.
#[derive(Default)]
pub struct Chooser {
    moved: Option<Instant>,
    trial: Option<Trial>,
    spurned: HashMap<String, Instant>,
}

struct Trial {
    from: String,
    before: Measured,
}

impl Chooser {
    /// Chooses, from what the access point the card is on carried over the
    /// span, the others it hears, and whether a call is on.
    pub fn choose(
        &mut self,
        now: Instant,
        on: (&str, &Measured, i16),
        others: &[Other],
        calling: bool,
    ) -> Choice {
        let (point, measured, signal) = on;
        let failing = measured.lost > FAILING;
        if calling && !failing {
            return Choice::Stay;
        }
        if let Some(trial) = self.trial.take() {
            let over = self
                .moved
                .is_none_or(|moved| now.duration_since(moved) >= TRIAL);
            if !over && !failing {
                self.trial = Some(trial);
                return Choice::Stay;
            }
            if worse(measured, &trial.before) {
                self.spurned.insert(point.to_owned(), now);
                self.moved = Some(now);
                return Choice::Back(trial.from);
            }
        }
        let settled = self
            .moved
            .is_none_or(|moved| now.duration_since(moved) >= SETTLE);
        if !settled && !failing {
            return Choice::Stay;
        }
        self.spurned
            .retain(|_, spurned| now.duration_since(*spurned) < SPURNED);
        let bad = failing || slowness(measured) > SLOW || measured.lost > LOSSY;
        let untried = others
            .iter()
            .filter(|other| other.point != point && !self.spurned.contains_key(&other.point));
        let faster = untried.clone().filter_map(|other| {
            let theirs = other.measured.as_ref()?;
            quicker(theirs, measured).then_some((slowness(theirs), other))
        });
        let near = untried.filter(|other| {
            bad && other.measured.is_none()
                && other.signal >= signal - FAINTER
                && other.busy.is_none_or(|busy| busy <= BUSIEST)
        });
        let chosen = faster
            .min_by_key(|(slowness, _)| *slowness)
            .map(|(_, other)| other)
            .or_else(|| near.max_by_key(|other| other.signal));
        let Some(chosen) = chosen else {
            return Choice::Stay;
        };
        self.moved = Some(now);
        self.trial = Some(Trial {
            from: point.to_owned(),
            before: measured.clone(),
        });
        Choice::Try(chosen.point.clone())
    }
}

/// An access point's p95, where nothing answered as slow as can be.
fn slowness(measured: &Measured) -> Duration {
    measured.p95.unwrap_or(Duration::MAX)
}

/// Whether one access point carried far quicker than another, and lost no
/// more.
fn quicker(theirs: &Measured, ours: &Measured) -> bool {
    let (their_p95, our_p95) = (slowness(theirs), slowness(ours));
    theirs.lost <= ours.lost.max(LOSSY)
        && their_p95 <= our_p95.mul_f64(QUICKER_BY)
        && our_p95.saturating_sub(their_p95) >= QUICKER
}

/// Whether a trial carried worse than what it left.
fn worse(trial: &Measured, before: &Measured) -> bool {
    slowness(trial) > slowness(before).saturating_add(WORSE)
        || trial.lost > before.lost + WORSE_LOSS
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn carried(p95: u64, lost: f64) -> Measured {
        Measured {
            echoes: 120,
            p50: Some(Duration::from_millis(p95 / 3)),
            p95: Some(Duration::from_millis(p95)),
            lost,
            retried: 1.0,
            failed: 0.0,
            signal: Some(-50),
        }
    }

    fn hall(signal: i16, busy: Option<u8>, measured: Option<Measured>) -> Other {
        Other {
            point: HALL.into(),
            signal,
            busy,
            measured,
        }
    }

    #[test]
    fn an_access_point_carrying_well_is_kept() {
        let mut chooser = Chooser::default();
        let well = carried(15, 0.0);
        let louder = hall(-40, Some(10), None);

        let choice = chooser.choose(Instant::now(), (DESK, &well, -50), &[louder], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "a louder one untried is no reason to move"
        );
    }

    #[test]
    fn one_carrying_badly_tries_another_heard_about_as_well() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let near = hall(-53, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50), &[near], false);

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "119 ms at p95 is worth a try"
        );
    }

    #[test]
    fn one_carrying_badly_keeps_off_another_heard_much_more_faintly() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let faint = hall(-70, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50), &[faint], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "20 dB fainter would carry worse still"
        );
    }

    #[test]
    fn one_measured_lately_and_carrying_far_better_is_moved_to() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let better = hall(-55, None, Some(carried(12, 0.0)));

        let choice = chooser.choose(Instant::now(), (DESK, &middling, -50), &[better], false);

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "12 ms against 30 beats both margins"
        );
    }

    #[test]
    fn one_carrying_only_a_little_better_is_not() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let little = hall(-55, None, Some(carried(24, 0.0)));

        let choice = chooser.choose(Instant::now(), (DESK, &middling, -50), &[little], false);

        assert_eq!(choice, Choice::Stay, "6 ms better is within the margin");
    }

    #[test]
    fn nothing_moves_during_a_call_unless_the_link_is_failing() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let failing = carried(400, 15.0);
        let near = hall(-53, Some(20), None);
        let now = Instant::now();

        let held = chooser.choose(now, (DESK, &badly, -50), std::slice::from_ref(&near), true);
        let saved = chooser.choose(now, (DESK, &failing, -50), &[near], true);

        assert_eq!(held, Choice::Stay, "a slow link is borne through a call");
        assert_eq!(saved, Choice::Try(HALL.into()), "a failing one isn't");
    }

    #[test]
    fn a_card_just_moved_stays_put_a_while() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            ..near
        };

        let soon = start + TRIAL + Duration::from_secs(1);
        let choice = chooser.choose(soon, (HALL, &badly, -53), &[desk], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "no second move before the card settles"
        );
    }

    #[test]
    fn a_move_that_carries_worse_goes_back_and_is_not_tried_again_soon() {
        let mut chooser = Chooser::default();
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let badly = carried(119, 0.5);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            signal: -50,
            busy: None,
            measured: Some(badly.clone()),
        };

        let worse = carried(200, 1.0);
        let back = chooser.choose(start + TRIAL, (HALL, &worse, -53), &[desk], false);
        let later = start + SETTLE + TRIAL + Duration::from_secs(1);
        let again = chooser.choose(later, (DESK, &badly, -50), &[near], false);

        assert_eq!(
            back,
            Choice::Back(DESK.into()),
            "200 ms is worse than the 119 it left"
        );
        assert_eq!(
            again,
            Choice::Stay,
            "the hall carried worse from here an hour ago"
        );
    }
}
