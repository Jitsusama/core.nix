//! Which access point of its network a card should be on, from what each
//! carried from here (measure.rs), changed rarely and never during a call.
//!
//! The best access point changes with the hour and the room (Joel, 14 Nov
//! 2026: "fixating on one could hurt me"), so none is fixed: the one the
//! card is on is kept while it carries well, and another is tried only
//! when it carries badly two minutes running, or fails, or when one
//! measured lately carried far better.
//! A wider band, 5 or 6 GHz, is kept to while it carries well and tried
//! from 2.4 GHz whenever one is heard well enough: 2.4 GHz can be quicker
//! to the gateway but carries far less.
//! wpa_supplicant still roams on its own when the signal fades; this
//! moves the card only between access points it can already hear, and
//! has one carrying badly that hears no other look, as often as it moves.
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

/// How much fainter than the one it's on another access point may be
/// heard and still be tried: about a quarter of the power. It holds for
/// one known to carry quickly too, since an echo's figure says nothing of
/// room: on 15 Nov 2026 the card moved twice from -47 dBm to another eero
/// heard at -65, and its link rate fell from about 900 Mbit/s to between
/// 245 and 432 as each carried worse than what it left.
const FAINTER: i16 = 6;

/// How busy an untried access point's channel may say it is and still be
/// tried. On 15 Nov 2026 every eero's ch149 radio averaged 42 to 46% busy,
/// and the one the card tried four times that afternoon carried badly in
/// about 20 of its 35 minutes; their ch52 radios averaged 15 to 21%, and
/// were past 40% in 1% of the eero's samples.
const BUSIEST: u8 = 40;

/// From this frequency in MHz a radio is on 5 or 6 GHz, whose channels are
/// wider than 2.4 GHz's: on 15 Nov 2026 the card's link rate fell from
/// about 1200 to 258 Mbit/s moving to the same access point's 2.4 GHz
/// radio, below what the internet brings down.
const WIDE: u32 = 5000;

/// How faint a radio on a wider band may be heard and still be tried for
/// it: the same access point is heard about 10 dB fainter on 5 GHz than
/// on 2.4, so the margin of `FAINTER` would never let one be tried.
const FAINTEST: i16 = -67;

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
    /// Its channel, by frequency in MHz; 0 where wpa_supplicant doesn't say.
    pub frequency: u32,
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
    /// Look for another access point of the network, none being heard.
    Look,
    /// Look for an access point of the network on a wider band, none being
    /// heard there.
    LookWider,
}

/// What's been chosen lately, to keep the card from hunting.
#[derive(Default)]
pub struct Chooser {
    moved: Option<Instant>,
    /// When it last had the card look for another access point, which it
    /// does no more often than it moves.
    looked: Option<Instant>,
    /// The access point the card carried badly on at the last choice. One
    /// minute's figure alone moves nothing: on 15 Nov 2026 one point read
    /// 45 ms at p95 for a minute between 11 and 18, and the one tried for
    /// it carried 32 to 64 for the next 14.
    badly_on: Option<String>,
    trial: Option<Trial>,
    spurned: HashMap<String, Instant>,
}

struct Trial {
    from: String,
    before: Measured,
    /// Whether the move was for a wider band, kept unless it carries badly
    /// rather than unless it carries worse than the narrower one left.
    banded: bool,
}

impl Chooser {
    /// Chooses, from what the access point the card is on carried over the
    /// span, the others it hears, and whether a call is on.
    pub fn choose(
        &mut self,
        now: Instant,
        on: (&str, &Measured, i16, u32),
        others: &[Other],
        calling: bool,
    ) -> Choice {
        let (point, measured, signal, frequency) = on;
        let failing = measured.lost > FAILING;
        let carried_badly = badly(measured);
        // A failing link is left on its first minute, a merely bad one on
        // its second.
        let lately = std::mem::replace(&mut self.badly_on, carried_badly.then(|| point.to_owned()));
        let bad = carried_badly && (failing || lately.as_deref() == Some(point));
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
            let back = if trial.banded {
                badly(measured)
            } else {
                worse(measured, &trial.before)
            };
            if back {
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
        // A card on a wider band carrying well keeps off a narrower one,
        // however quick, since quickness isn't room to bring things down.
        let untried = others.iter().filter(|other| {
            other.point != point
                && !self.spurned.contains_key(&other.point)
                && (bad || wide(other.frequency) || !wide(frequency))
        });
        let narrow = frequency != 0 && !wide(frequency);
        let banded = if narrow && !bad {
            for_the_band(untried.clone())
        } else {
            None
        };
        let reached = untried.filter(|other| within_reach(other, signal, frequency));
        let faster = reached.clone().filter_map(|other| {
            let theirs = other.measured.as_ref()?;
            quicker(theirs, measured).then_some((slowness(theirs), other))
        });
        let near = reached.filter(|other| {
            bad && other.measured.is_none() && other.busy.is_none_or(|busy| busy <= BUSIEST)
        });
        let chosen = banded.or_else(|| {
            faster
                .min_by_key(|(slowness, _)| *slowness)
                .map(|(_, other)| other)
                .or_else(|| near.max_by_key(|other| (wide(other.frequency), other.signal)))
        });
        let Some(chosen) = chosen else {
            return self.look(now, bad && others.iter().all(|other| other.point == point));
        };
        self.moved = Some(now);
        self.trial = Some(Trial {
            from: point.to_owned(),
            before: measured.clone(),
            banded: banded.is_some(),
        });
        Choice::Try(chosen.point.clone())
    }

    /// Has a card carrying badly that hears no other access point of its
    /// network look for one, once a settle at most, so the next choice has
    /// others to choose from.
    fn look(&mut self, now: Instant, alone: bool) -> Choice {
        let rested = self
            .looked
            .is_none_or(|looked| now.duration_since(looked) >= SETTLE);
        if !alone || !rested {
            return Choice::Stay;
        }
        self.looked = Some(now);
        Choice::Look
    }
}

/// Where a card on a narrower band carrying well goes for a wider one:
/// the quickest known to carry well from here, else the loudest untried
/// heard well enough and not too busy.
fn for_the_band<'a>(untried: impl Iterator<Item = &'a Other> + Clone) -> Option<&'a Other> {
    let on_it = untried.filter(|other| wide(other.frequency));
    let known = on_it.clone().filter_map(|other| {
        let theirs = other.measured.as_ref()?;
        (!badly(theirs)).then_some((slowness(theirs), other))
    });
    let heard = on_it.filter(|other| {
        other.measured.is_none()
            && other.signal >= FAINTEST
            && other.busy.is_none_or(|busy| busy <= BUSIEST)
    });
    known
        .min_by_key(|(slowness, _)| *slowness)
        .map(|(_, other)| other)
        .or_else(|| heard.max_by_key(|other| other.signal))
}

/// Whether another access point is heard loud enough, from one heard this
/// loud on this channel, to be tried: within `FAINTER`, or on a wider band
/// and no fainter than `FAINTEST`, since the same access point is heard
/// fainter there.
fn within_reach(other: &Other, signal: i16, frequency: u32) -> bool {
    other.signal >= signal - FAINTER
        || (wider(other.frequency, frequency) && other.signal >= FAINTEST)
}

/// Whether a channel is on 5 or 6 GHz.
fn wide(frequency: u32) -> bool {
    frequency >= WIDE
}

/// Whether a channel is on a wider band than another's, where both are
/// known.
pub fn wider(to: u32, from: u32) -> bool {
    from != 0 && wide(to) && !wide(from)
}

/// Whether an access point carries badly enough to try another.
fn badly(measured: &Measured) -> bool {
    slowness(measured) > SLOW || measured.lost > LOSSY
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
pub(crate) mod tests {
    use super::*;

    /// A chooser that has seen this access point carry badly a minute
    /// before, so a second bad minute is acted on.
    pub(crate) fn after_a_bad_minute(point: &str) -> Chooser {
        Chooser {
            badly_on: Some(point.to_owned()),
            ..Chooser::default()
        }
    }

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";
    /// The desk's access point's own 2.4 GHz radio.
    const NEAR: &str = "6c:ae:f6:c1:84:27";
    const CH52: u32 = 5260;
    const CH6: u32 = 2437;

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
            frequency: CH52,
            measured,
        }
    }

    #[test]
    fn an_access_point_carrying_well_is_kept() {
        let mut chooser = Chooser::default();
        let well = carried(15, 0.0);
        let louder = hall(-40, Some(10), None);

        let choice = chooser.choose(Instant::now(), (DESK, &well, -50, CH52), &[louder], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "a louder one untried is no reason to move"
        );
    }

    #[test]
    fn one_minute_carrying_badly_moves_nothing_but_two_running_do() {
        let mut chooser = Chooser::default();
        let (badly, well) = (carried(45, 0.0), carried(15, 0.0));
        let near = [hall(-41, Some(20), None)];
        let (start, minute) = (Instant::now(), Duration::from_secs(60));

        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &near, false);
        let eased = chooser.choose(start + minute, (DESK, &well, -50, CH52), &near, false);
        let again = chooser.choose(start + minute * 2, (DESK, &badly, -50, CH52), &near, false);
        let still = chooser.choose(start + minute * 3, (DESK, &badly, -50, CH52), &near, false);

        assert_eq!(first, Choice::Stay, "one minute at 45 ms moves nothing");
        assert_eq!(eased, Choice::Stay, "15 ms carries well");
        assert_eq!(again, Choice::Stay, "a bad minute after a good one is new");
        assert_eq!(still, Choice::Try(HALL.into()), "two running are");
    }

    #[test]
    fn one_minute_carrying_badly_asks_no_look_but_two_running_do() {
        let mut chooser = Chooser::default();
        let badly = carried(52, 0.0);
        let start = Instant::now();

        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &[], false);
        let minute = start + Duration::from_secs(60);
        let second = chooser.choose(minute, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(first, Choice::Stay, "one minute at 52 ms asks no look");
        assert_eq!(second, Choice::Look, "two running do");
    }

    #[test]
    fn a_failing_link_is_left_on_its_first_minute() {
        let mut chooser = Chooser::default();
        let failing = carried(400, 15.0);
        let near = hall(-53, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &failing, -50, CH52), &[near], false);

        assert_eq!(choice, Choice::Try(HALL.into()), "15% lost is left at once");
    }

    #[test]
    fn one_carrying_badly_tries_another_heard_about_as_well() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(119, 0.5);
        let near = hall(-53, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50, CH52), &[near], false);

        assert_eq!(
            choice,
            Choice::Try(HALL.into()),
            "119 ms at p95 is worth a try"
        );
    }

    #[test]
    fn one_carrying_badly_and_hearing_no_other_looks_for_one() {
        let mut chooser = Chooser::default();
        let (badly, well) = (carried(52, 0.0), carried(15, 0.0));
        let now = Instant::now();

        let carrying_well = chooser.choose(now, (DESK, &well, -50, CH52), &[], false);
        let calling = chooser.choose(now, (DESK, &badly, -50, CH52), &[], true);
        let looking = chooser.choose(now, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(
            carrying_well,
            Choice::Stay,
            "one carrying well needs no other"
        );
        assert_eq!(calling, Choice::Stay, "no look in a call, as no move");
        assert_eq!(looking, Choice::Look, "52 ms at p95 is worth a look");
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_and_hearing_no_5_ghz_looks_for_it() {
        let mut chooser = Chooser::default();
        let well = carried(17, 0.0);
        let now = Instant::now();

        let calling = chooser.choose(now, (NEAR, &well, -43, CH6), &[], true);
        let looking = chooser.choose(now, (NEAR, &well, -43, CH6), &[], false);
        let soon = now + Duration::from_secs(60);
        let soon = chooser.choose(soon, (NEAR, &well, -43, CH6), &[], false);
        let later = chooser.choose(now + SETTLE, (NEAR, &well, -43, CH6), &[], false);

        assert_eq!(calling, Choice::Stay, "no look in a call");
        assert_eq!(looking, Choice::LookWider, "5 GHz is worth a look");
        assert_eq!(soon, Choice::Stay, "a minute on, it isn't asked again");
        assert_eq!(later, Choice::LookWider, "once settled, it is");
    }

    #[test]
    fn a_card_on_2_4_ghz_hearing_only_2_4_ghz_looks_for_5_ghz() {
        let mut chooser = Chooser::default();
        let well = carried(17, 0.0);
        let other = Other {
            point: HALL.into(),
            ..near_at(-45, None)
        };

        let choice = chooser.choose(Instant::now(), (NEAR, &well, -43, CH6), &[other], false);

        assert_eq!(choice, Choice::LookWider, "2.4 GHz is no wider");
    }

    #[test]
    fn a_look_for_another_is_asked_again_only_once_settled() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(52, 0.0);
        let start = Instant::now();
        let first = chooser.choose(start, (DESK, &badly, -50, CH52), &[], false);

        let soon = start + Duration::from_secs(60);
        let soon = chooser.choose(soon, (DESK, &badly, -50, CH52), &[], false);
        let later = chooser.choose(start + SETTLE, (DESK, &badly, -50, CH52), &[], false);

        assert_eq!(first, Choice::Look, "the first look is asked");
        assert_eq!(soon, Choice::Stay, "a minute on, it isn't asked again");
        assert_eq!(later, Choice::Look, "once settled, it is");
    }

    #[test]
    fn one_carrying_badly_keeps_off_another_heard_much_more_faintly() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let faint = hall(-70, Some(20), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -50, CH52), &[faint], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "20 dB fainter would carry worse still"
        );
    }

    #[test]
    fn one_carrying_badly_keeps_off_an_untried_channel_45_percent_busy() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(72, 0.0);
        let busy = hall(-45, Some(45), None);

        let choice = chooser.choose(Instant::now(), (DESK, &badly, -47, CH52), &[busy], false);

        assert_eq!(choice, Choice::Stay, "45% busy carried badly all afternoon");
    }

    #[test]
    fn one_measured_lately_and_carrying_far_better_is_moved_to() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let better = hall(-55, None, Some(carried(12, 0.0)));

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &middling, -50, CH52),
            &[better],
            false,
        );

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

        let choice = chooser.choose(
            Instant::now(),
            (DESK, &middling, -50, CH52),
            &[little],
            false,
        );

        assert_eq!(choice, Choice::Stay, "6 ms better is within the margin");
    }

    #[test]
    fn nothing_moves_during_a_call_unless_the_link_is_failing() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.5);
        let failing = carried(400, 15.0);
        let near = hall(-53, Some(20), None);
        let now = Instant::now();

        let on = (DESK, &badly, -50, CH52);
        let held = chooser.choose(now, on, std::slice::from_ref(&near), true);
        let saved = chooser.choose(now, (DESK, &failing, -50, CH52), &[near], true);

        assert_eq!(held, Choice::Stay, "a slow link is borne through a call");
        assert_eq!(saved, Choice::Try(HALL.into()), "a failing one isn't");
    }

    #[test]
    fn a_card_just_moved_stays_put_a_while() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(119, 0.5);
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50, CH52),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            ..near
        };

        let soon = start + TRIAL + Duration::from_secs(1);
        let choice = chooser.choose(soon, (HALL, &badly, -53, CH52), &[desk], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "no second move before the card settles"
        );
    }

    #[test]
    fn a_move_that_carries_worse_goes_back_and_is_not_tried_again_soon() {
        let mut chooser = after_a_bad_minute(DESK);
        let start = Instant::now();
        let near = hall(-53, Some(20), None);
        let badly = carried(119, 0.5);
        let tried = chooser.choose(
            start,
            (DESK, &badly, -50, CH52),
            std::slice::from_ref(&near),
            false,
        );
        assert_eq!(tried, Choice::Try(HALL.into()), "the first move is made");
        let desk = Other {
            point: DESK.into(),
            signal: -50,
            busy: None,
            frequency: CH52,
            measured: Some(badly.clone()),
        };

        let worse = carried(200, 1.0);
        let back = chooser.choose(start + TRIAL, (HALL, &worse, -53, CH52), &[desk], false);
        let later = start + SETTLE + TRIAL + Duration::from_secs(1);
        let again = chooser.choose(later, (DESK, &badly, -50, CH52), &[near], false);

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

    /// The desk's 5 GHz radio, heard this loud.
    fn desk_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: DESK.into(),
            signal,
            busy: Some(20),
            frequency: CH52,
            measured,
        }
    }

    /// The desk's 2.4 GHz radio, heard this loud.
    fn near_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: NEAR.into(),
            signal,
            busy: Some(20),
            frequency: CH6,
            measured,
        }
    }

    #[test]
    fn a_card_on_2_4_ghz_carrying_well_tries_5_ghz_heard_well_enough() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-45, None)], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "2.4 GHz carries a quarter of what 5 GHz does, however quick"
        );
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_heard_too_faintly() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-75, None)], false);

        assert_eq!(choice, Choice::Stay, "at -75 dBm 5 GHz would carry worse");
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_45_percent_busy() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let busy = Other {
            busy: Some(45),
            ..desk_at(-45, None)
        };

        let choice = chooser.choose(Instant::now(), (NEAR, &well, -34, CH6), &[busy], false);

        assert_eq!(choice, Choice::Stay, "a busy channel is no more room");
    }

    #[test]
    fn a_card_on_2_4_ghz_keeps_off_5_ghz_known_to_carry_badly_from_here() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let known = desk_at(-45, Some(carried(119, 0.0)));

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(choice, Choice::Stay, "119 ms at p95 is no better band");
    }

    #[test]
    fn a_card_on_2_4_ghz_moves_to_5_ghz_known_to_carry_well_though_slower() {
        let mut chooser = Chooser::default();
        let well = carried(9, 0.0);
        let known = desk_at(-45, Some(carried(20, 0.0)));

        let on = (NEAR, &well, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "20 ms at p95 carries well, and on the wider band"
        );
    }

    #[test]
    fn a_card_on_5_ghz_carrying_well_keeps_off_2_4_ghz_however_quick() {
        let mut chooser = Chooser::default();
        let middling = carried(30, 0.0);
        let quick = near_at(-34, Some(carried(9, 0.0)));

        let on = (DESK, &middling, -45, CH52);
        let choice = chooser.choose(Instant::now(), on, &[quick], false);

        assert_eq!(
            choice,
            Choice::Stay,
            "30 ms carries well, on the wider band"
        );
    }

    #[test]
    fn one_carrying_badly_tries_5_ghz_before_a_louder_2_4_ghz() {
        let mut chooser = after_a_bad_minute(HALL);
        let badly = carried(119, 0.0);
        let others = [near_at(-34, None), desk_at(-44, None)];

        let on = (HALL, &badly, -46, 5745);
        let choice = chooser.choose(Instant::now(), on, &others, false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "the 5 GHz radio, heard well enough, comes before the louder 2.4"
        );
    }

    #[test]
    fn one_on_2_4_ghz_carrying_badly_tries_5_ghz_heard_well_enough_though_fainter() {
        let mut chooser = Chooser::default();
        let badly = carried(119, 0.0);

        let on = (NEAR, &badly, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[desk_at(-45, None)], false);

        assert_eq!(
            choice,
            Choice::Try(DESK.into()),
            "5 GHz is heard fainter than 2.4 from the same radio, and -45 is loud"
        );
    }

    /// Another eero's 5 GHz radio, heard from the desk far more faintly.
    fn far_at(signal: i16, measured: Option<Measured>) -> Other {
        Other {
            point: "6c:ae:f6:ba:85:88".into(),
            signal,
            busy: Some(20),
            frequency: 5745,
            measured,
        }
    }

    #[test]
    fn one_on_5_ghz_carrying_badly_keeps_off_5_ghz_heard_much_more_faintly() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(56, 0.0);

        let on = (DESK, &badly, -47, CH52);
        let choice = chooser.choose(Instant::now(), on, &[far_at(-65, None)], false);

        assert_eq!(choice, Choice::Stay, "18 dB fainter carries less");
    }

    #[test]
    fn one_carrying_badly_keeps_off_one_known_quicker_but_heard_much_more_faintly() {
        let mut chooser = after_a_bad_minute(DESK);
        let badly = carried(61, 0.0);
        let known = far_at(-65, Some(carried(26, 0.0)));

        let on = (DESK, &badly, -48, CH52);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        assert_eq!(choice, Choice::Stay, "17 dB fainter isn't room");
    }

    #[test]
    fn one_on_2_4_ghz_carrying_badly_moves_to_5_ghz_known_quicker_though_fainter() {
        let mut chooser = after_a_bad_minute(NEAR);
        let badly = carried(119, 0.0);
        let known = desk_at(-45, Some(carried(20, 0.0)));

        let on = (NEAR, &badly, -34, CH6);
        let choice = chooser.choose(Instant::now(), on, &[known], false);

        let to_the_desk = Choice::Try(DESK.into());
        assert_eq!(choice, to_the_desk, "5 GHz is heard fainter, and wider");
    }

    #[test]
    fn a_move_to_a_wider_band_goes_back_only_if_it_carries_badly() {
        let start = Instant::now();
        let well = carried(9, 0.0);
        let on = (NEAR, &well, -34, CH6);
        let (mut kept, mut left) = (Chooser::default(), Chooser::default());
        for chooser in [&mut kept, &mut left] {
            let tried = chooser.choose(start, on, &[desk_at(-45, None)], false);
            assert_eq!(tried, Choice::Try(DESK.into()), "the first move is made");
        }
        let near = near_at(-34, Some(well.clone()));
        let (slower, slow) = (carried(25, 0.0), carried(60, 0.0));

        let stayed = kept.choose(
            start + TRIAL,
            (DESK, &slower, -45, CH52),
            std::slice::from_ref(&near),
            false,
        );
        let back = left.choose(start + TRIAL, (DESK, &slow, -45, CH52), &[near], false);

        assert_eq!(stayed, Choice::Stay, "25 ms on 5 GHz still carries well");
        assert_eq!(back, Choice::Back(NEAR.into()), "60 ms doesn't");
    }
}
