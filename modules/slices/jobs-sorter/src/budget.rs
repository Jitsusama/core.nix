//! Fits the floors jobs-sorter gives pi hosts' and conversations' units
//! into what their slice's own floor has left once the windows beside
//! them are protected, so the windows Joel types in keep theirs whole.
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
//! A claim is what a cgroup holds up to its floor, so a unit holding less
//! than its floor leaves the rest to the next. A unit's own `MemoryLow`
//! comes first, as its unit file asks; only the floor the sorter adds is
//! fitted, in the order given, rounded down to a [`STEP`] so the floors
//! don't follow every page a sibling takes.

use crate::hosts::Signs;

/// What a fitted floor is rounded down to, so it changes, and is set
/// again, only when the room moves by this much.
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
/// given, out of `room`: whole while what they hold under their floors
/// fits, then what's left rounded down to a [`STEP`], then none.
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
            let past_own = unit.current.saturating_sub(unit.own);
            left = left.saturating_sub(past_own.min(given));
            given
        })
        .collect()
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
    }

    #[test]
    fn a_unit_holding_less_than_its_floor_leaves_the_rest_to_the_next() {
        let wanted = [unit(GIB, 100 * MIB), unit(GIB, 2 * GIB)];
        assert_eq!(
            fit(&wanted, GIB + 100 * MIB),
            [GIB, GIB],
            "the first claims only the 100 MiB it holds, so the second's floor fits too"
        );
    }

    #[test]
    fn a_units_own_floor_is_counted_first_and_never_fitted() {
        let wanted = [
            unit(GIB, 2 * GIB),
            Wanted {
                own: GIB,
                added: GIB,
                current: 3 * GIB,
            },
        ];
        assert_eq!(
            fit(&wanted, 2 * GIB),
            [GIB, 0],
            "the second's own 1 GiB is taken from the room before anything added, which then \
             leaves room for the first's alone"
        );
    }
}
