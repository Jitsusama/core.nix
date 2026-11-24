//! Fits the floors jobs-sorter gives pi hosts' and conversations' units
//! into what their slice's own floor has left once the windows beside
//! them are protected, so the windows Joel types in keep theirs whole
//! while the others beside them, the strays, fit in what's left.
//!
//! Under the kernel's `memory_recursiveprot`, when a slice's children
//! claim more protection than the slice's own floor, each gets the same
//! share of its claim. At 13:35 on 21 November agents.slice's children
//! claimed 7.8 GB of its 6.7 GB floor, and at 19:3x 8.3 GB, so every
//! window kept only 81 to 86% of its floor, however much of the claims
//! were orchestrators' units, which Joel doesn't type in. Raising
//! agents.slice's floor instead would take it from the account's, which
//! is spent on Joel's own programs and his session.
//!
//! A claim is what a cgroup holds up to its floor. The windows beside the
//! units, and the units' own `MemoryLow`, which their unit files ask for
//! and the kernel keeps whatever this says, claim what they hold. Only
//! the floor the sorter adds is fitted, in the order given, and it counts
//! whole: a unit growing into it before the next look then can't push
//! the claims past the slice's floor. A floor trimmed short is rounded
//! down to a [`STEP`], and kept while the room wavers within one
//! ([`steady`]), so it isn't set again on every look.
//!
//! A stray, any other child of the slice, claims what it holds up to its
//! own floor, which the sorter never sets, so fitting gives it nothing
//! and trims nothing of it. Its claim is fitted after the units Joel is
//! at and before the rest, as one any agent can start with a floor of max
//! would otherwise leave the units Joel is at none. So when the strays
//! claim more than the units Joel is at leave, the slice is claimed past
//! its floor again and the kernel shares it among every child, the
//! windows too: the units Joel is at keep their floors ahead of the
//! windows keeping theirs whole. The sorter says so once a slice, naming
//! the strays (`Fitting::overran` in `src/main.rs`).
//!
//! The room is the slice's floor as set, not the protection the kernel
//! leaves it, which no file says: it holds while the account's own
//! children claim no more than the account's floor.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::hosts::Signs;

/// What a floor trimmed short is rounded down to, and how far the room
/// must clear it before it rises ([`steady`]).
pub const STEP: u64 = 64 << 20;

/// A unit's floor as the sorter would give it, beside what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wanted {
    /// The floor its unit file sets, kept whole.
    pub own: u64,
    /// What the sorter adds to it for the windows and orchestrators.
    pub added: u64,
    /// What it holds now, as its memory.current says.
    pub current: u64,
}

/// What a slice's floor, `low`, leaves for the units in it, once the
/// `others` in it, each as what it holds and its floor, are protected.
pub fn room(low: u64, others: impl IntoIterator<Item = (u64, u64)>) -> u64 {
    others.into_iter().fold(low, |room, (current, floor)| {
        room.saturating_sub(current.min(floor))
    })
}

/// How much of what each of `wanted` would add it's given, in the order
/// given, out of `room` less what they hold under their own floors: whole
/// while it fits, then what's left rounded down to a [`STEP`], then none.
pub fn fit(wanted: &[Wanted], room: u64) -> Vec<u64> {
    let mut left = wanted.iter().fold(room, |left, unit| {
        left.saturating_sub(unit.current.min(unit.own))
    });
    wanted
        .iter()
        .map(|unit| {
            let given = if unit.added <= left {
                unit.added
            } else {
                left / STEP * STEP
            };
            left = left.saturating_sub(given);
            given
        })
        .collect()
}

/// The floor to add to a unit, from what's `fitted` now, what fits a
/// [`STEP`] less room, `margin`, and what was added `last`: the last
/// while it's no more than fits now, raised only to what fits with a
/// step to spare, so a room wavering by a page doesn't set it on every
/// look. Never more than fits now, so the claims stay within the room.
pub fn steady(fitted: u64, margin: u64, last: Option<u64>) -> u64 {
    match last {
        Some(last) if last <= fitted => last.max(margin).min(fitted),
        _ => fitted,
    }
}

/// The order units with these `signs` are given their floors in: those a
/// window Joel is at shows first, then those running more orchestrators,
/// then those a window shows, each as it came.
pub fn order(signs: &[Signs]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..signs.len()).collect();
    order.sort_by_key(|&at| {
        let unit = signs[at];
        std::cmp::Reverse((unit.viewed > 0, unit.orchestrators, unit.shown > 0))
    });
    order
}

/// How long a window's claim is held at its peak before it may fall
/// ([`Peaks`]).
pub const PEAK_FOR: Duration = Duration::from_secs(60);

/// Each window's claims within the last [`PEAK_FOR`], each with when it
/// was made, so a claim that dips is held at the highest of them: a
/// window's claim falls as soon as what it holds does, and the room the
/// units are fitted into rose and fell with every page reclaimed, so
/// their floors hunted. Every claim is kept, not only the first peak, so
/// when that one ages out the claim falls to the next highest of the
/// minute rather than to the look's own.
#[derive(Debug, Default)]
pub struct Peaks(HashMap<String, Vec<(u64, Instant)>>);

impl Peaks {
    /// What `window` claims at `now`, as it claims `claim`: the highest it
    /// claimed within [`PEAK_FOR`], this look's included.
    pub fn hold(&mut self, window: &str, claim: u64, now: Instant) -> u64 {
        let claims = self.0.entry(window.to_owned()).or_default();
        claims.retain(|&(_, at)| now.saturating_duration_since(at) < PEAK_FOR);
        // A claim no higher than this one can never be the peak again, as this one outlives it.
        claims.retain(|&(was, _)| was > claim);
        claims.push((claim, now));
        claims.iter().map(|&(was, _)| was).max().unwrap_or(claim)
    }

    /// Forgets the claims made [`PEAK_FOR`] or more before `now`, and the
    /// windows left with none, so a window gone is forgotten.
    pub fn forget_before(&mut self, now: Instant) {
        self.0.retain(|_, claims| {
            claims.retain(|&(_, at)| now.saturating_duration_since(at) < PEAK_FOR);
            !claims.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;

    fn unit(added: u64, current: u64) -> Wanted {
        Wanted {
            own: 0,
            added,
            current,
        }
    }

    #[test]
    fn the_room_is_the_slices_floor_less_what_its_windows_hold_under_theirs() {
        let others = [(220 * MIB, 512 * MIB), (3 * GIB, 2 * GIB), (GIB, 0)];
        assert_eq!(
            room(6 * GIB, others),
            6 * GIB - 220 * MIB - 2 * GIB,
            "a window counts what it holds up to its floor, and one with none counts nothing"
        );
        assert_eq!(
            room(GIB, [(2 * GIB, 2 * GIB)]),
            0,
            "windows claiming past the slice's floor leave no room, not less than none"
        );
    }

    #[test]
    fn the_units_joel_is_at_come_first_then_orchestrators_then_the_shown() {
        let shown = Signs {
            shown: 1,
            ..Signs::default()
        };
        let orchestrating = |orchestrators| Signs {
            orchestrators,
            ..Signs::default()
        };
        let viewed = Signs {
            viewed: 1,
            shown: 1,
            ..Signs::default()
        };
        let signs = [
            Signs::default(),
            shown,
            orchestrating(1),
            viewed,
            orchestrating(2),
            shown,
        ];
        assert_eq!(
            order(&signs),
            [3, 4, 2, 1, 5, 0],
            "the one Joel is at, then two orchestrators, then one, then the shown as they came, \
             then the rest"
        );
    }

    #[test]
    fn floors_that_fit_are_given_whole() {
        let wanted = [unit(2 * GIB, 3 * GIB), unit(GIB, GIB)];
        assert_eq!(
            fit(&wanted, 4 * GIB),
            [2 * GIB, GIB],
            "with room for both, each unit keeps all it would add"
        );
    }

    #[test]
    fn past_the_room_the_first_keep_theirs_and_the_last_go_without() {
        let wanted = [
            unit(2 * GIB, 3 * GIB),
            unit(GIB, 2 * GIB),
            unit(GIB, 2 * GIB),
        ];
        assert_eq!(
            fit(&wanted, 2 * GIB + 700 * MIB),
            [2 * GIB, 640 * MIB, 0],
            "the first keeps its whole floor, the second what's left in whole steps, the third none"
        );
        assert_eq!(
            fit(&wanted, 3 * GIB + 63 * MIB),
            [2 * GIB, GIB, 0],
            "a floor filling the room to the byte is given whole, and less than a step left gives \
             none"
        );
    }

    #[test]
    fn a_floor_given_counts_whole_though_its_unit_holds_less() {
        let wanted = [unit(GIB, 100 * MIB), unit(GIB, 2 * GIB)];
        assert_eq!(
            fit(&wanted, GIB + 100 * MIB),
            [GIB, 64 * MIB],
            "the first may grow into its whole floor before the next look, so the second gets \
             what's left of the room in whole steps"
        );
    }

    #[test]
    fn a_later_units_own_floor_keeps_an_earlier_ones_added_floor_within_the_room() {
        let wanted = [
            unit(2 * GIB, 2 * GIB),
            Wanted {
                own: GIB,
                added: 0,
                current: 3 * GIB,
            },
        ];
        assert_eq!(
            fit(&wanted, 2 * GIB),
            [GIB, 0],
            "the second's own floor is kept first, so the first's added floor stops at what's \
             left, not at the whole room"
        );
    }

    #[test]
    fn a_floor_trimmed_short_rises_only_once_the_room_clears_a_step() {
        assert_eq!(
            steady(2 * GIB, GIB + 960 * MIB, Some(GIB + 960 * MIB)),
            GIB + 960 * MIB,
            "a room a page past the whole floor keeps the floor a step under it"
        );
        assert_eq!(
            steady(2 * GIB, 2 * GIB, Some(GIB + 960 * MIB)),
            2 * GIB,
            "a room a step past it gives it whole"
        );
        assert_eq!(
            steady(GIB, 960 * MIB, Some(2 * GIB)),
            GIB,
            "a smaller room takes it down at once"
        );
        assert_eq!(
            steady(640 * MIB, 576 * MIB, None),
            640 * MIB,
            "one never set gets what fits"
        );
    }

    #[test]
    fn a_windows_claim_falls_only_to_the_highest_of_the_last_minute() {
        let mut peaks = Peaks::default();
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        peaks.hold("kitty-1-1.scope", 2 * GIB, at(0));
        peaks.hold("kitty-1-1.scope", GIB, at(30));
        assert_eq!(
            peaks.hold("kitty-1-1.scope", 512 * MIB, at(70)),
            GIB,
            "the first peak's end let the claim fall past the one after it"
        );
        assert_eq!(
            peaks.hold("kitty-1-1.scope", 512 * MIB, at(90)),
            512 * MIB,
            "a minute with no higher claim kept one"
        );
    }

    #[test]
    fn a_windows_claim_is_held_at_its_peak_for_a_minute() {
        let mut peaks = Peaks::default();
        let start = Instant::now();
        let at = |secs: u64| start + Duration::from_secs(secs);
        assert_eq!(
            peaks.hold("kitty-1-1.scope", GIB, at(0)),
            GIB,
            "a first claim is its peak"
        );
        assert_eq!(
            peaks.hold("kitty-1-1.scope", 512 * MIB, at(30)),
            GIB,
            "a dip inside the minute keeps the peak"
        );
        assert_eq!(
            peaks.hold("kitty-1-2.scope", 512 * MIB, at(30)),
            512 * MIB,
            "each window keeps its own"
        );
        assert_eq!(
            peaks.hold("kitty-1-1.scope", 512 * MIB, at(60)),
            512 * MIB,
            "one after it lets it fall"
        );
        assert_eq!(
            peaks.hold("kitty-1-1.scope", 2 * GIB, at(70)),
            2 * GIB,
            "a rise is taken at once"
        );
        assert_eq!(
            peaks.hold("kitty-1-1.scope", GIB, at(129)),
            2 * GIB,
            "and held a minute from when it came"
        );
        peaks.forget_before(at(129));
        assert_eq!(
            peaks.0.keys().collect::<Vec<_>>(),
            ["kitty-1-1.scope"],
            "a window whose peak is past the minute is forgotten, as it would fall anyway"
        );
    }
}
