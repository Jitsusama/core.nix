//! Lifts jobs' memory ceiling while nothing else waits on memory, and
//! brings it back as soon as anything does.
//!
//! jobs.slice is held to 26% of memory (modules/slices/nixos.nix), which
//! cut how long apps waited on memory from 0.67% of the time to 0.024%.
//! But the ceiling counts the page cache too, and at 09:00 on 25 November,
//! just after a boot, jobs sat at it holding 2.4 GB of its own memory and
//! 14.3 GB of cache, 5 GB of that mapped. It was slowed 122,620 times in
//! ten minutes and read about 19 GB of its cache back from the disk, while
//! apps, sessions and agents waited on memory not at all and 26 GB was
//! available.
//!
//! So the ceiling follows the room. While the slices it spares (apps,
//! sessions, agents) have waited on memory less than [`WAITING`] for
//! [`QUIET_FOR`], no call is on, jobs fills what it has, and memory
//! available stays over [`ROOM`] of the whole, the ceiling rises by
//! a [`step`], never past [`TOP`] of memory. The first wait in a spared
//! slice, a call, or available memory falling short brings it down a
//! [`fall`] each look, to the ceiling the slice declares and never below it. So
//! jobs gets more only by testing that nothing else misses it, and gives it
//! back before the apps' wait reaches where it was without a ceiling.
//!
//! It's set as a runtime property, gone at the next boot, and the unit
//! puts back the declared ceiling as it stops, since a sorter that isn't
//! running can't bring it down.

use std::time::{Duration, Instant};

const MIB: u64 = 1 << 20;

/// How much a spared slice may wait on memory, as its `some avg10`, a
/// percentage, before the ceiling comes down: about four times what apps
/// waited with the ceiling in place, a sixth of what they waited without.
pub const WAITING: f64 = 0.1;

/// How long every spared slice must have stayed under [`WAITING`] before
/// the ceiling rises, and between one rise and the next.
pub const QUIET_FOR: Duration = Duration::from_secs(30);

/// How much the ceiling rises at once, of `total` memory: 2 GiB of 64, and
/// a share rather than a size, so a small machine's jobs must fill its
/// ceiling as nearly as a large one's to be given more.
pub fn step(total: u64) -> u64 {
    total / 32
}

/// How much it falls each look, of `total` memory: 4 GiB of 64, so the
/// reclaim a lower ceiling asks of systemd as it sets it stays small, while
/// a quarter of memory comes back in four seconds.
pub fn fall(total: u64) -> u64 {
    total / 16
}

/// The share of memory, as a percentage, that must stay available for the
/// ceiling to rise, and short of which it falls.
pub const ROOM: u64 = 30;

/// The most of memory, as a percentage, the ceiling ever gives jobs.
pub const TOP: u64 = 50;

/// How long a ceiling that couldn't be set waits to be tried again.
pub const AGAIN: Duration = Duration::from_secs(30);

/// What one look found.
#[derive(Clone, Copy)]
pub struct Look {
    /// The machine's memory.
    pub total: u64,
    /// The memory available, as the kernel counts it.
    pub available: u64,
    /// What the slice holds now.
    pub current: u64,
    /// Whether a spared slice waited on memory past [`WAITING`].
    pub waiting: bool,
    /// Whether a call is on.
    pub calling: bool,
}

/// Where the ceiling stands: as last set, or unknown before the first set
/// and after a failed one; since when everything spared has been quiet;
/// and when a set last failed.
#[derive(Default)]
pub struct Ceiling {
    set: Option<u64>,
    quiet_from: Option<Instant>,
    failed: Option<Instant>,
}

/// The ceiling the slice declares, `percent` of `total`.
pub fn least(total: u64, percent: u64) -> u64 {
    total / 100 * percent
}

impl Ceiling {
    /// The ceiling to set `now`, given what `look` found and the slice's
    /// declared ceiling `least`, or none when it stands as it should.
    pub fn wanted(&mut self, look: &Look, least: u64, now: Instant) -> Option<u64> {
        if self
            .failed
            .is_some_and(|at| now.saturating_duration_since(at) < AGAIN)
        {
            return None;
        }
        let short = look.available < look.total / 100 * ROOM;
        if look.waiting || look.calling || short {
            self.quiet_from = None;
        } else if self.quiet_from.is_none() {
            self.quiet_from = Some(now);
        }
        let Some(set) = self.set else {
            return Some(least);
        };
        if self.quiet_from.is_none() {
            // Down a fall at most, and never below the declared ceiling,
            // which one under it is brought up to.
            let fallen = (set.saturating_sub(fall(look.total)) / MIB * MIB).max(least);
            return (set != least).then_some(fallen);
        }
        let top = self::least(look.total, TOP).max(least);
        let quiet = self
            .quiet_from
            .is_some_and(|from| now.saturating_duration_since(from) >= QUIET_FOR);
        // Jobs within a step of its ceiling is using it; more given to a
        // slice that isn't is room handed out untested.
        let step = step(look.total);
        let filled = look.current.saturating_add(step) >= set;
        if quiet && filled && set < top {
            self.quiet_from = Some(now);
            return Some(set.saturating_add(step).min(top) / MIB * MIB);
        }
        None
    }

    /// Where the ceiling stands as last set, or none when it's unknown.
    pub fn stands(&self) -> Option<u64> {
        self.set
    }

    /// The ceiling now stands at `high`.
    pub fn set(&mut self, high: u64) {
        self.set = Some(high);
        self.failed = None;
    }

    /// Setting it failed `now`; it may have landed late, so it's unknown.
    pub fn failed(&mut self, now: Instant) {
        self.set = None;
        self.failed = Some(now);
    }
}

/// How much a slice waited on memory over ten seconds, as a percentage,
/// from its `memory.pressure`.
pub fn waited(pressure: &str) -> Option<f64> {
    pressure
        .lines()
        .find_map(|line| line.strip_prefix("some "))?
        .split_whitespace()
        .find_map(|word| word.strip_prefix("avg10="))?
        .parse()
        .ok()
}

/// What busctl is given to set `slice`'s memory ceiling to `high` bytes,
/// as a runtime property, refused rather than asking for a password and
/// given up after 5 s, since the sorter's loop would wait on either.
pub fn high_call(slice: &str, high: u64) -> Vec<String> {
    let high = high.to_string();
    [
        "--user",
        "--quiet",
        "--allow-interactive-authorization=no",
        "--timeout=5",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        slice,
        "true",
        "1",
        "MemoryHigh",
        "t",
        &high,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;
    const TOTAL: u64 = 64 * GIB;
    const STEP: u64 = 2 * GIB;
    const FALL: u64 = 4 * GIB;

    fn base() -> u64 {
        least(TOTAL, 26)
    }

    /// Jobs at its ceiling, a third of memory available, nothing waiting.
    fn roomy(current: u64) -> Look {
        Look {
            total: TOTAL,
            available: TOTAL / 100 * 40,
            current,
            waiting: false,
            calling: false,
        }
    }

    /// A ceiling already standing at `high`, quiet from `at`.
    fn standing(high: u64, at: Instant) -> Ceiling {
        let mut ceiling = Ceiling::default();
        ceiling.set(high);
        assert_eq!(ceiling.wanted(&roomy(high), base(), at), None);
        ceiling
    }

    #[test]
    fn the_first_look_sets_the_declared_ceiling() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now),
            Some(base()),
            "a ceiling of unknown height wasn't set to the declared one"
        );
    }

    #[test]
    fn it_rises_a_step_once_everything_spared_has_been_quiet() {
        let now = Instant::now();
        let mut ceiling = standing(base(), now);
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + QUIET_FOR / 2),
            None,
            "it rose before the spared slices had been quiet long enough"
        );
        let raised = ceiling.wanted(&roomy(base()), base(), now + QUIET_FOR);
        assert_eq!(raised, Some((base() + STEP) / MIB * MIB));
        ceiling.set(raised.unwrap());
        assert_eq!(
            ceiling.wanted(&roomy(raised.unwrap()), base(), now + QUIET_FOR * 3 / 2),
            None,
            "it rose again without waiting out another quiet spell"
        );
    }

    #[test]
    fn a_step_and_a_fall_are_shares_of_memory() {
        assert_eq!((step(TOTAL), fall(TOTAL)), (STEP, FALL));
        assert_eq!((step(2 * GIB), fall(2 * GIB)), (64 * MIB, 128 * MIB));
    }

    #[test]
    fn it_rises_only_while_jobs_fills_what_it_has() {
        let now = Instant::now();
        let mut ceiling = standing(base(), now);
        assert_eq!(
            ceiling.wanted(&roomy(base() / 2), base(), now + QUIET_FOR),
            None,
            "it rose for a slice using half its ceiling"
        );
    }

    #[test]
    fn it_never_rises_past_the_top() {
        let now = Instant::now();
        let top = least(TOTAL, TOP);
        let mut ceiling = standing(top - GIB, now);
        assert_eq!(
            ceiling.wanted(&roomy(top - GIB), base(), now + QUIET_FOR),
            Some(top / MIB * MIB)
        );
        ceiling.set(top);
        assert_eq!(
            ceiling.wanted(&roomy(top), base(), now + QUIET_FOR * 2),
            None,
            "it rose past the top"
        );
    }

    #[test]
    fn a_spared_slice_waiting_brings_it_down_a_fall_each_look_to_the_declared() {
        let now = Instant::now();
        let high = base() + 3 * FALL / 2;
        let mut ceiling = standing(high, now);
        let waiting = Look {
            waiting: true,
            ..roomy(high)
        };
        let lower = ceiling.wanted(&waiting, base(), now + Duration::from_secs(1));
        assert_eq!(lower, Some((high - FALL) / MIB * MIB));
        ceiling.set(lower.unwrap());
        assert_eq!(
            ceiling.wanted(&waiting, base(), now + Duration::from_secs(2)),
            Some(base()),
            "it fell below the declared ceiling, or not to it"
        );
        ceiling.set(base());
        assert_eq!(
            ceiling.wanted(&waiting, base(), now + Duration::from_secs(3)),
            None
        );
    }

    #[test]
    fn a_wait_starts_the_quiet_spell_again() {
        let now = Instant::now();
        let mut ceiling = standing(base(), now);
        let waiting = Look {
            waiting: true,
            ..roomy(base())
        };
        assert_eq!(ceiling.wanted(&waiting, base(), now + QUIET_FOR / 2), None);
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + QUIET_FOR),
            None,
            "it rose though a spared slice waited within the quiet spell"
        );
        // The spell began at the first quiet look, so it ends a spell later.
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + QUIET_FOR * 3 / 2),
            None
        );
        assert!(
            ceiling
                .wanted(&roomy(base()), base(), now + QUIET_FOR * 2)
                .is_some()
        );
    }

    #[test]
    fn a_call_or_memory_short_brings_it_down_and_holds_it() {
        let now = Instant::now();
        let high = base() + STEP;
        for look in [
            Look {
                calling: true,
                ..roomy(high)
            },
            Look {
                available: TOTAL / 100 * (ROOM - 1),
                ..roomy(high)
            },
        ] {
            let mut ceiling = standing(high, now);
            assert_eq!(ceiling.wanted(&look, base(), now + QUIET_FOR), Some(base()));
            ceiling.set(base());
            assert_eq!(
                ceiling.wanted(&look, base(), now + QUIET_FOR * 4),
                None,
                "it rose through a call, or with memory short"
            );
        }
    }

    #[test]
    fn a_failed_set_waits_before_it_is_tried_again() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        assert!(ceiling.wanted(&roomy(base()), base(), now).is_some());
        ceiling.failed(now);
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + AGAIN / 2),
            None,
            "a refused set was tried again at once"
        );
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + AGAIN),
            Some(base())
        );
    }

    #[test]
    fn a_slice_wait_is_read_from_its_pressure() {
        let pressure = "some avg10=0.25 avg60=0.10 avg300=0.02 total=1\n\
                        full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(waited(pressure), Some(0.25));
        assert_eq!(waited(""), None);
    }

    #[test]
    fn a_ceiling_is_set_on_the_account_manager_until_the_next_boot() {
        let call = high_call("jobs.slice", 17_000_000_000);
        assert_eq!(call[0], "--user");
        assert_eq!(
            call[10..],
            ["jobs.slice", "true", "1", "MemoryHigh", "t", "17000000000"]
        );
    }
}
