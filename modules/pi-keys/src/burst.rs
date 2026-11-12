//! A thread's bursts of work, from what the kernel counts for it.
//!
//! Every tick the sampler reads a thread's schedstat (nanoseconds on a
//! CPU, and waiting for one) and stat (its state, and its major faults,
//! which on this machine are mostly swap-ins). A burst is a run of ticks in
//! which the thread ran, waited for a CPU, or sat in D; it ends at the
//! first tick it did none of those.

/// What the kernel counts for a thread at one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    /// Nanoseconds on a CPU, from schedstat.
    pub run: u64,
    /// Nanoseconds waiting for a CPU, from schedstat.
    pub wait: u64,
    /// Its state letter from stat: R, S, D and so on.
    pub state: char,
    /// Its major faults so far, from stat.
    pub majflt: u64,
}

/// A burst that ended, in milliseconds on the monotonic clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Burst {
    pub at: f64,
    pub ms: f64,
    pub cpu: f64,
    pub wait: f64,
    pub d: f64,
    pub maj: u64,
}

/// How much of a burst has to be the thread's own time, on a CPU, waiting
/// for one or in D, for a key arriving in it to wait it out. Sampling
/// strings many wakes of a few microseconds into one long run, as a kitty
/// drawing many windows or niri does, and none of those holds a key.
pub const SOLID: f64 = 0.7;

impl Burst {
    /// Whether the thread was busy for most of it.
    pub fn solid(&self) -> bool {
        self.cpu + self.wait + self.d >= SOLID * self.ms
    }

    /// Whether it's worth keeping: long enough to hold a key up, or
    /// showing a wait or a fault. The bursts measured before 12 Nov 2026
    /// kept only these, so the same rule keeps the numbers comparable.
    pub fn worth(&self) -> bool {
        self.ms >= 8.0 || self.wait >= 1.0 || self.maj > 0
    }
}

/// Reads a schedstat file's first two numbers.
pub fn schedstat(text: &str) -> Option<(u64, u64)> {
    let mut fields = text.split_whitespace();
    Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
}

/// Reads a stat file's state and major faults. The command's name comes in
/// parentheses and may hold spaces and parentheses itself, so the fields
/// are counted from the last closing one.
pub fn stat(text: &str) -> Option<(char, u64)> {
    let rest = text.get(text.rfind(')')? + 2..)?;
    let fields: Vec<&str> = rest.split_whitespace().collect();
    Some((
        fields.first()?.chars().next()?,
        fields.get(9)?.parse().ok()?,
    ))
}

/// One thread's bursts as its readings come.
#[derive(Debug, Default)]
pub struct Bursts {
    last: Option<Reading>,
    open: Option<Open>,
}

#[derive(Debug)]
struct Open {
    at: f64,
    run: u64,
    wait: u64,
    d: f64,
    maj: u64,
}

impl Bursts {
    /// Takes the reading at `now`, `interval` milliseconds after the last,
    /// and gives the burst it ends, if it ends one.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a burst's nanoseconds are far under 2^52"
    )]
    pub fn take(&mut self, now: f64, interval: f64, reading: Reading) -> Option<Burst> {
        let last = self.last.replace(reading)?;
        let ran = reading.run.saturating_sub(last.run);
        let waited = reading.wait.saturating_sub(last.wait);
        let busy = ran > 0 || waited > 0 || matches!(reading.state, 'R' | 'D');
        if busy {
            let open = self.open.get_or_insert(Open {
                at: now - interval,
                run: 0,
                wait: 0,
                d: 0.0,
                maj: 0,
            });
            open.run += ran;
            open.wait += waited;
            open.maj += reading.majflt.saturating_sub(last.majflt);
            if reading.state == 'D' {
                open.d += interval;
            }
            return None;
        }
        let open = self.open.take()?;
        Some(Burst {
            at: open.at,
            ms: now - open.at,
            cpu: open.run as f64 / 1e6,
            wait: open.wait as f64 / 1e6,
            d: open.d,
            maj: open.maj,
        })
    }

    /// Forgets what came before, as after a gap in sampling: a burst
    /// spanning it would be the sampler's, not the thread's.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(run: u64, wait: u64, state: char) -> Reading {
        Reading {
            run,
            wait,
            state,
            majflt: 0,
        }
    }

    #[test]
    fn a_thread_that_ran_for_three_ticks_ends_one_burst() {
        let mut bursts = Bursts::default();
        assert_eq!(bursts.take(0.0, 2.0, at(0, 0, 'S')), None);
        assert_eq!(bursts.take(2.0, 2.0, at(2_000_000, 0, 'S')), None);
        assert_eq!(bursts.take(4.0, 2.0, at(4_000_000, 100_000, 'S')), None);
        assert_eq!(bursts.take(6.0, 2.0, at(5_000_000, 100_000, 'S')), None);
        let burst = bursts.take(8.0, 2.0, at(5_000_000, 100_000, 'S')).unwrap();
        assert_eq!(burst.at, 0.0);
        assert_eq!(burst.ms, 8.0);
        assert!((burst.cpu - 5.0).abs() < 1e-9);
        assert!((burst.wait - 0.1).abs() < 1e-9);
        assert_eq!(burst.d, 0.0);
    }

    #[test]
    fn a_thread_in_d_is_busy_though_it_runs_nowhere() {
        let mut bursts = Bursts::default();
        bursts.take(0.0, 2.0, at(0, 0, 'S'));
        bursts.take(2.0, 2.0, at(0, 0, 'D'));
        bursts.take(4.0, 2.0, at(0, 0, 'D'));
        let burst = bursts.take(6.0, 2.0, at(0, 0, 'S')).unwrap();
        assert_eq!(burst.d, 4.0);
        assert_eq!(burst.ms, 6.0);
        assert!(!burst.solid());
    }

    #[test]
    fn its_faults_are_counted_by_burst() {
        let mut bursts = Bursts::default();
        bursts.take(
            0.0,
            2.0,
            Reading {
                majflt: 10,
                ..at(0, 0, 'S')
            },
        );
        bursts.take(
            2.0,
            2.0,
            Reading {
                majflt: 13,
                ..at(1_000_000, 0, 'R')
            },
        );
        let burst = bursts
            .take(
                4.0,
                2.0,
                Reading {
                    majflt: 13,
                    ..at(1_000_000, 0, 'S')
                },
            )
            .unwrap();
        assert_eq!(burst.maj, 3);
    }

    #[test]
    fn after_a_reset_the_next_reading_only_sets_where_it_starts() {
        let mut bursts = Bursts::default();
        bursts.take(0.0, 2.0, at(0, 0, 'S'));
        bursts.take(2.0, 2.0, at(2_000_000, 0, 'S'));
        bursts.reset();
        assert_eq!(bursts.take(900.0, 2.0, at(9_000_000, 0, 'S')), None);
        assert_eq!(bursts.take(902.0, 2.0, at(9_000_000, 0, 'S')), None);
    }

    #[test]
    fn a_burst_of_many_tiny_wakes_is_not_solid() {
        let burst = Burst {
            at: 0.0,
            ms: 40.0,
            cpu: 2.0,
            wait: 1.0,
            d: 0.0,
            maj: 0,
        };
        assert!(!burst.solid());
        assert!(Burst { cpu: 30.0, ..burst }.solid());
    }

    #[test]
    fn a_short_quiet_burst_is_not_worth_keeping() {
        let burst = Burst {
            at: 0.0,
            ms: 4.0,
            cpu: 4.0,
            wait: 0.2,
            d: 0.0,
            maj: 0,
        };
        assert!(!burst.worth());
        assert!(Burst { ms: 8.0, ..burst }.worth());
        assert!(Burst { wait: 1.0, ..burst }.worth());
        assert!(Burst { maj: 1, ..burst }.worth());
    }

    #[test]
    fn schedstat_gives_time_on_a_cpu_and_waiting() {
        assert_eq!(schedstat("123 456 7\n"), Some((123, 456)));
        assert_eq!(schedstat(""), None);
    }

    #[test]
    fn stat_counts_from_the_name_s_last_parenthesis() {
        let line = "42 (pi (x) y) D 1 42 42 0 -1 4194560 100 0 77 0 5 6 0 0 20 0 1";
        assert_eq!(stat(line), Some(('D', 77)));
    }
}
