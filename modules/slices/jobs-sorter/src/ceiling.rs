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
//! running can't bring it down. Each look reads the ceiling back from the
//! slice's `memory.high`, so one set elsewhere, or a set that failed but
//! landed late, is followed from where it stands, and one cleared or put
//! past [`TOP`] is put back.

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

/// How long a ceiling that couldn't be set waits to be tried again, unless
/// it was to come down: a fall waits only [`FALL_AGAIN`], since a spared
/// slice is waiting on it.
pub const AGAIN: Duration = Duration::from_secs(30);

/// How long a fall that couldn't be set waits to be tried again: as long
/// as a set may take before it gives up, so a bus that answers nothing
/// holds the sorter's look, its door and its call brake with it, half the
/// time at most rather than all of it.
pub const FALL_AGAIN: Duration = Duration::from_secs(5);

/// Why a ceiling is set, as the sorter's journal says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The first set, to the declared ceiling.
    Declared,
    /// Put back at the declared ceiling after a set that failed, with
    /// nothing read back to say where it stands.
    Again,
    /// Put back inside its bounds after something else set it outside them.
    Elsewhere,
    /// Raised, nothing spared having waited.
    Rise,
    /// Brought down for a spared slice waiting on memory.
    Waiting,
    /// Brought down for a call.
    Call,
    /// Brought down for memory available falling short of [`ROOM`].
    Short,
}

impl Why {
    /// The reason as the journal's line ends.
    pub fn said(self) -> &'static str {
        match self {
            Why::Declared => "as it declares",
            Why::Again => "as it declares, after a set that failed",
            Why::Elsewhere => "back inside its bounds after something else set it",
            Why::Rise => "nothing it spares waiting on memory",
            Why::Waiting => "a slice it spares waiting on memory",
            Why::Call => "a call on",
            Why::Short => "memory short",
        }
    }
}

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
    /// The slice's ceiling as its `memory.high` reads, or none when that
    /// can't be read.
    pub high: Option<u64>,
}

/// Two ceilings within a MiB of each other are one: the kernel keeps a
/// ceiling in whole pages, so one set to a share of memory reads back a
/// little under it.
fn near(one: u64, other: u64) -> bool {
    one.abs_diff(other) < MIB
}

/// The ceiling `memory.high` says, `max` being none at all.
pub fn high(text: &str) -> Option<u64> {
    match text.trim() {
        "max" => Some(u64::MAX),
        high => high.parse().ok(),
    }
}

/// Where the ceiling stands: as last set, or unknown before the first set
/// and after a failed one; since when everything spared has been quiet;
/// and when a set last failed; and whether any set has landed.
#[derive(Default)]
pub struct Ceiling {
    set: Option<u64>,
    quiet_from: Option<Instant>,
    failed: Option<Instant>,
    landed: bool,
}

/// The ceiling the slice declares, `percent` of `total`.
pub fn least(total: u64, percent: u64) -> u64 {
    total / 100 * percent
}

impl Ceiling {
    /// The ceiling to set `now` and why, given what `look` found and the
    /// slice's declared ceiling `least`, or none when it stands as it
    /// should.
    pub fn wanted(&mut self, look: &Look, least: u64, now: Instant) -> Option<(u64, Why)> {
        // What memory.high reads is where the ceiling stands, whoever set
        // it; within a MiB of the last set, it's that set. A sorter just
        // started takes it only as the declared ceiling, so one left by an
        // earlier sorter, or declared before a change, is set afresh, and
        // so is one read after its first set failed, which never stood.
        // Only a failed set leaves a ceiling unknown once one has stood,
        // and the declared one read back has stood, as much as a set has.
        match (self.set, look.high) {
            (Some(set), Some(high)) if !near(set, high) => self.set = Some(high),
            (None, Some(high)) if self.landed || near(high, least) => {
                self.set = Some(high);
                self.landed = true;
            }
            _ => {}
        }
        let short = look.available < look.total / 100 * ROOM;
        let down = if look.waiting {
            Some(Why::Waiting)
        } else if look.calling {
            Some(Why::Call)
        } else if short {
            Some(Why::Short)
        } else {
            None
        };
        if down.is_some() {
            self.quiet_from = None;
        } else if self.quiet_from.is_none() {
            self.quiet_from = Some(now);
        }
        let again = self
            .failed
            .is_some_and(|at| now.saturating_duration_since(at) < AGAIN);
        let fall_again = self
            .failed
            .is_some_and(|at| now.saturating_duration_since(at) < FALL_AGAIN);
        // In whole MiB, so a ceiling risen to it reads back as at it.
        let top = (self::least(look.total, TOP) / MIB * MIB).max(least);
        // One read under the declared ceiling stays there until nothing
        // waits, as a known one does, since raising it would take from
        // what waits.
        let under = look.high.is_some_and(|high| high < least);
        let Some(set) = self.set else {
            return match down {
                Some(_) if fall_again || under => None,
                Some(why) => Some((least, why)),
                None if again => None,
                None if self.failed.is_some() => Some((least, Why::Again)),
                None => Some((least, Why::Declared)),
            };
        };
        if let Some(why) = down {
            if fall_again {
                return None;
            }
            // Down a fall at most, from the top at most, and never below
            // the declared ceiling; one something else set under it stays
            // there until nothing waits, since raising it now would take
            // from what waits.
            let fallen = (set.min(top).saturating_sub(fall(look.total)) / MIB * MIB).max(least);
            return (set > least && !near(fallen, set)).then_some((fallen, why));
        }
        if again {
            return None;
        }
        let bounded = set.clamp(least, top);
        if !near(bounded, set) {
            return Some((bounded, Why::Elsewhere));
        }
        let quiet = self
            .quiet_from
            .is_some_and(|from| now.saturating_duration_since(from) >= QUIET_FOR);
        // Jobs within a step of its ceiling is using it; more given to a
        // slice that isn't is room handed out untested.
        let step = step(look.total);
        let filled = look.current.saturating_add(step) >= set;
        let raised = set.saturating_add(step).min(top) / MIB * MIB;
        if quiet && filled && raised > set && !near(raised, set) {
            self.quiet_from = Some(now);
            return Some((raised, Why::Rise));
        }
        None
    }

    /// The ceiling now stands at `high`.
    pub fn set(&mut self, high: u64) {
        self.set = Some(high);
        self.failed = None;
        self.landed = true;
    }

    /// Setting it failed `now`; it may have landed late, so it's unknown
    /// until `memory.high` is read again.
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
/// given up after [`FALL_AGAIN`], since the sorter's loop would wait on
/// either, and a fall that failed is tried again after as long.
pub fn high_call(slice: &str, high: u64) -> Vec<String> {
    let high = high.to_string();
    let timeout = format!("--timeout={}", FALL_AGAIN.as_secs());
    [
        "--user",
        "--quiet",
        "--allow-interactive-authorization=no",
        &timeout,
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

    /// Jobs holding `current`, two fifths of memory available, nothing
    /// waiting, and its ceiling not read back.
    fn roomy(current: u64) -> Look {
        Look {
            total: TOTAL,
            available: TOTAL / 100 * 40,
            current,
            waiting: false,
            calling: false,
            high: None,
        }
    }

    /// What a look at a jobs holding `current` finds when a spared slice
    /// waits.
    fn waiting(current: u64) -> Look {
        Look {
            waiting: true,
            ..roomy(current)
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
            Some((base(), Why::Declared)),
            "a ceiling of unknown height wasn't set to the declared one"
        );
    }

    #[test]
    fn a_ceiling_read_back_as_declared_needs_no_first_set() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        // The kernel keeps it in whole pages, a little under the share.
        let read = Look {
            high: Some(base() / 4096 * 4096),
            ..roomy(base())
        };
        assert_eq!(
            ceiling.wanted(&read, base(), now),
            None,
            "a ceiling already standing as declared was set again"
        );
        assert_eq!(
            ceiling.wanted(
                &Look {
                    waiting: true,
                    ..read
                },
                base(),
                now
            ),
            None,
            "a ceiling read back a page under the declared one was set to it"
        );
        let mut ceiling = Ceiling::default();
        let left = Look {
            high: Some(base() + STEP),
            ..roomy(base())
        };
        assert_eq!(
            ceiling.wanted(&left, base(), now),
            Some((base(), Why::Declared)),
            "a ceiling an earlier sorter left lifted was taken as standing"
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
        let raised = (base() + STEP) / MIB * MIB;
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + QUIET_FOR),
            Some((raised, Why::Rise))
        );
        ceiling.set(raised);
        assert_eq!(
            ceiling.wanted(&roomy(raised), base(), now + QUIET_FOR * 3 / 2),
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
    fn jobs_within_a_step_of_its_ceiling_fills_it() {
        let now = Instant::now();
        let mut ceiling = standing(base(), now);
        assert_eq!(
            ceiling.wanted(&roomy(base() - STEP - 1), base(), now + QUIET_FOR),
            None,
            "it rose for a slice more than a step short of its ceiling"
        );
        assert!(
            ceiling
                .wanted(&roomy(base() - STEP), base(), now + QUIET_FOR * 2)
                .is_some(),
            "it didn't rise for a slice a step short of its ceiling, where reclaim keeps it"
        );
    }

    #[test]
    fn it_never_rises_past_the_top() {
        let now = Instant::now();
        // Half of 64 GiB, as a share, is 18 bytes short of a whole MiB.
        let top = least(TOTAL, TOP) / MIB * MIB;
        let mut ceiling = standing(top - GIB, now);
        assert_eq!(
            ceiling.wanted(&roomy(top - GIB), base(), now + QUIET_FOR),
            Some((top, Why::Rise))
        );
        ceiling.set(top);
        assert_eq!(
            ceiling.wanted(&roomy(top), base(), now + QUIET_FOR * 2),
            None,
            "it rose past the top, or set the top again"
        );
    }

    #[test]
    fn a_spared_slice_waiting_brings_it_down_a_fall_each_look_to_the_declared() {
        let now = Instant::now();
        let high = base() + 3 * FALL / 2;
        let mut ceiling = standing(high, now);
        let lower = (high - FALL) / MIB * MIB;
        assert_eq!(
            ceiling.wanted(&waiting(high), base(), now + Duration::from_secs(1)),
            Some((lower, Why::Waiting))
        );
        ceiling.set(lower);
        assert_eq!(
            ceiling.wanted(&waiting(high), base(), now + Duration::from_secs(2)),
            Some((base(), Why::Waiting)),
            "it fell below the declared ceiling, or not to it"
        );
        ceiling.set(base());
        assert_eq!(
            ceiling.wanted(&waiting(high), base(), now + Duration::from_secs(3)),
            None
        );
    }

    #[test]
    fn a_wait_starts_the_quiet_spell_again() {
        let now = Instant::now();
        let mut ceiling = standing(base(), now);
        assert_eq!(
            ceiling.wanted(&waiting(base()), base(), now + QUIET_FOR / 2),
            None
        );
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
                .is_some(),
            "it didn't rise a quiet spell after the first quiet look"
        );
    }

    #[test]
    fn a_call_or_memory_short_brings_it_down_and_holds_it() {
        let now = Instant::now();
        let high = base() + STEP;
        for (look, why) in [
            (
                Look {
                    calling: true,
                    ..roomy(high)
                },
                Why::Call,
            ),
            (
                Look {
                    available: TOTAL / 100 * (ROOM - 1),
                    ..roomy(high)
                },
                Why::Short,
            ),
        ] {
            let mut ceiling = standing(high, now);
            assert_eq!(
                ceiling.wanted(&look, base(), now + QUIET_FOR),
                Some((base(), why))
            );
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
        assert!(
            ceiling.wanted(&roomy(base()), base(), now).is_some(),
            "a ceiling never set didn't want setting"
        );
        ceiling.failed(now);
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + AGAIN / 2),
            None,
            "a refused set was tried again at once"
        );
        assert_eq!(
            ceiling.wanted(&roomy(base()), base(), now + AGAIN),
            Some((base(), Why::Again))
        );
    }

    #[test]
    fn a_first_set_that_failed_is_tried_again_unless_memory_high_reads_as_declared() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        assert!(
            ceiling.wanted(&roomy(base()), base(), now).is_some(),
            "a ceiling never set didn't want setting"
        );
        ceiling.failed(now);
        // As an earlier sorter's stop that didn't finish could leave it.
        let left = Look {
            high: Some(least(TOTAL, 38)),
            ..roomy(base())
        };
        assert_eq!(
            ceiling.wanted(&left, base(), now + AGAIN),
            Some((base(), Why::Again)),
            "a ceiling read after the first set failed was kept as standing"
        );
        // A spared slice waiting tries it again after a fall's wait.
        ceiling.failed(now + AGAIN);
        let waited = Look {
            waiting: true,
            ..left
        };
        assert_eq!(
            ceiling.wanted(&waited, base(), now + AGAIN + FALL_AGAIN / 2),
            None,
            "a failed set with its ceiling unknown was tried again before FALL_AGAIN"
        );
        assert_eq!(
            ceiling.wanted(&waited, base(), now + AGAIN + FALL_AGAIN),
            Some((base(), Why::Waiting))
        );
    }

    #[test]
    fn a_rise_from_the_declared_ceiling_read_at_start_that_lands_late_stands() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        let read = Look {
            high: Some(base()),
            ..roomy(base())
        };
        assert_eq!(ceiling.wanted(&read, base(), now), None);
        // Its rise timed out, and the manager set it after.
        ceiling.failed(now + QUIET_FOR);
        let risen = Look {
            high: Some(base() + STEP),
            ..roomy(0)
        };
        assert_eq!(
            ceiling.wanted(&risen, base(), now + QUIET_FOR + AGAIN),
            None,
            "a rise that landed late, from the declared ceiling read at start, was undone"
        );
    }

    #[test]
    fn a_ceiling_of_unknown_height_read_under_the_declared_stays_while_a_slice_waits() {
        let now = Instant::now();
        let mut ceiling = Ceiling::default();
        let under = Look {
            high: Some(least(TOTAL, 20)),
            ..waiting(0)
        };
        assert_eq!(
            ceiling.wanted(&under, base(), now),
            None,
            "a ceiling read under the declared one was raised while a spared slice waited"
        );
        assert_eq!(
            ceiling.wanted(
                &Look {
                    waiting: false,
                    ..under
                },
                base(),
                now
            ),
            Some((base(), Why::Declared))
        );
    }

    #[test]
    fn a_failed_set_back_inside_its_bounds_waits_before_it_is_tried_again() {
        let now = Instant::now();
        let top = least(TOTAL, TOP) / MIB * MIB;
        let mut ceiling = standing(base(), now);
        let cleared = Look {
            high: high("max\n"),
            ..roomy(base())
        };
        let tried = now + Duration::from_secs(1);
        assert_eq!(
            ceiling.wanted(&cleared, base(), tried),
            Some((top, Why::Elsewhere))
        );
        ceiling.failed(tried);
        assert_eq!(
            ceiling.wanted(&cleared, base(), tried + AGAIN / 2),
            None,
            "a set back inside its bounds that failed was tried again before AGAIN"
        );
        assert_eq!(
            ceiling.wanted(&cleared, base(), tried + AGAIN),
            Some((top, Why::Elsewhere))
        );
    }

    #[test]
    fn a_failed_fall_is_tried_again_soon_from_where_it_stands() {
        let now = Instant::now();
        let high = base() + 2 * FALL;
        let mut ceiling = standing(high, now);
        let lower = (high - FALL) / MIB * MIB;
        assert_eq!(
            ceiling.wanted(&waiting(high), base(), now),
            Some((lower, Why::Waiting))
        );
        ceiling.failed(now);
        // It didn't land, so memory.high still says where it stood.
        let unlanded = Look {
            high: Some(high),
            ..waiting(high)
        };
        assert_eq!(
            ceiling.wanted(&unlanded, base(), now + Duration::from_secs(1)),
            None,
            "a fall that failed was tried again before FALL_AGAIN"
        );
        let tried = now + FALL_AGAIN;
        assert_eq!(
            ceiling.wanted(&unlanded, base(), tried),
            Some((lower, Why::Waiting)),
            "a fall that failed wasn't tried again at FALL_AGAIN, or wasn't a fall"
        );
        ceiling.failed(tried);
        // This time it landed late, so the next fall goes on from there.
        let landed = Look {
            high: Some(lower),
            ..waiting(high)
        };
        assert_eq!(
            ceiling.wanted(&landed, base(), tried + FALL_AGAIN),
            Some((base(), Why::Waiting)),
            "a fall that landed late wasn't followed from where it landed"
        );
    }

    #[test]
    fn a_ceiling_set_elsewhere_is_followed_and_one_past_its_bounds_put_back() {
        let now = Instant::now();
        let top = least(TOTAL, TOP) / MIB * MIB;
        let mut ceiling = standing(base(), now);
        let cleared = Look {
            high: high("max\n"),
            ..roomy(base())
        };
        assert_eq!(
            ceiling.wanted(&cleared, base(), now + Duration::from_secs(1)),
            Some((top, Why::Elsewhere)),
            "a ceiling cleared elsewhere wasn't put back under the top"
        );
        assert_eq!(
            ceiling.wanted(
                &Look {
                    waiting: true,
                    ..cleared
                },
                base(),
                now + Duration::from_secs(2)
            ),
            Some((top - FALL, Why::Waiting)),
            "a ceiling cleared elsewhere didn't fall from the top while a slice waited"
        );
        let mut ceiling = standing(base(), now);
        let raised = Look {
            high: Some(base() + STEP),
            ..waiting(base())
        };
        assert_eq!(
            ceiling.wanted(&raised, base(), now + Duration::from_secs(1)),
            Some((base(), Why::Waiting)),
            "a ceiling raised elsewhere wasn't brought down from where it stood"
        );
        let mut ceiling = standing(base(), now);
        let lowered = Look {
            high: Some(base() - STEP),
            ..waiting(base())
        };
        assert_eq!(
            ceiling.wanted(&lowered, base(), now + Duration::from_secs(1)),
            None,
            "a ceiling lowered elsewhere was raised while a slice waited"
        );
        assert_eq!(
            ceiling.wanted(
                &Look {
                    waiting: false,
                    ..lowered
                },
                base(),
                now + Duration::from_secs(2)
            ),
            Some((base(), Why::Elsewhere)),
            "a ceiling lowered elsewhere wasn't put back once nothing waited"
        );
    }

    #[test]
    fn a_ceiling_is_read_from_its_memory_high() {
        assert_eq!(high("17393516544\n"), Some(17_393_516_544));
        assert_eq!(high("max\n"), Some(u64::MAX));
        assert_eq!(high(""), None);
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
        assert_eq!(
            call[..5],
            [
                "--user",
                "--quiet",
                "--allow-interactive-authorization=no",
                "--timeout=5",
                "call"
            ],
            "a set could ask for a password, or wait past FALL_AGAIN"
        );
        assert_eq!(
            call[10..],
            ["jobs.slice", "true", "1", "MemoryHigh", "t", "17000000000"]
        );
    }
}
