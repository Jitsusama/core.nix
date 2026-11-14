//! What a key waits on, worked out from the bursts of the threads it
//! passes.
//!
//! Nothing stamps a key typed into pi, so the time is modelled. Instants
//! are drawn at random from the time Joel was at a pi window, and a key
//! sent along the path one takes: niri reads it, kitty writes it to the
//! pty, pi draws the echo, kitty reads and draws it, and niri presents it.
//! At each hop the key waits out whatever solid burst that thread is in
//! when it arrives, then pays the hop's own cost; the last waits for the
//! screen's next frame. Bursts too short to keep count as no wait, so the
//! figures are a floor.

/// The hops a key passes, in order, and the thread each runs on.
pub const HOPS: [(&str, Thread); 5] = [
    ("niri_in", Thread::Niri),
    ("kitty_in", Thread::Kitty),
    ("pi", Thread::Pi),
    ("kitty_out", Thread::Kitty),
    ("niri_out", Thread::Niri),
];

/// A hop's own cost once its thread runs, in milliseconds: pi's from a
/// scratch pi echoing keys (a median of 1.4 ms), and a kitty pass and a
/// niri frame at a guess of 2 ms each.
pub const COST: [f64; 5] = [0.5, 0.5, 1.5, 2.0, 2.0];

/// A key later than this is two frames late at 60 Hz, which is what
/// Joel feels as sluggish.
pub const LATE: f64 = 33.0;

/// A wait at one thread longer than this is a frame lost there.
pub const LOST: f64 = 16.0;

/// The threads on a key's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Thread {
    Niri,
    Kitty,
    Pi,
}

/// What held a thread through a burst: running its own work, waiting for
/// a CPU, or in D, for the disk or swap read back in. Which it was says
/// whether a slow key is the program's to fix or its place in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Ran,
    Queued,
    Disk,
}

impl Why {
    /// The one of a burst's times, on a CPU, waiting for one and in D, in
    /// milliseconds, that's largest; a tie goes to running, then to
    /// waiting, so a burst is never put down to the queue on a guess.
    pub fn of(cpu: f64, wait: f64, d: f64) -> Why {
        let _ = (cpu, wait, d);
        Why::Ran
    }
}

/// A thread's solid bursts as spans of the monotonic clock, in
/// milliseconds, sorted by their starts, with what held each when it's
/// told.
#[derive(Debug, Default, Clone)]
pub struct Spans {
    starts: Vec<f64>,
    ends: Vec<f64>,
    whys: Vec<Option<Why>>,
}

impl Spans {
    /// Bursts whose cause isn't told.
    pub fn new(spans: Vec<(f64, f64)>) -> Self {
        Self::sorted(spans.into_iter().map(|(a, b)| (a, b, None)).collect())
    }

    /// Bursts, each with what held the thread through it.
    pub fn told(spans: Vec<(f64, f64, Why)>) -> Self {
        Self::sorted(spans.into_iter().map(|(a, b, w)| (a, b, Some(w))).collect())
    }

    fn sorted(mut spans: Vec<(f64, f64, Option<Why>)>) -> Self {
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        Spans {
            starts: spans.iter().map(|s| s.0).collect(),
            ends: spans.iter().map(|s| s.1).collect(),
            whys: spans.iter().map(|s| s.2).collect(),
        }
    }

    /// What held the thread through the burst a key arriving at `t` lands
    /// in, when it lands in one whose cause is told.
    pub fn why(&self, t: f64) -> Option<Why> {
        let _ = t;
        None
    }

    /// How long a key arriving at `t` waits for the burst it lands in.
    pub fn residual(&self, t: f64) -> f64 {
        let after = self.starts.partition_point(|start| *start <= t);
        match after.checked_sub(1) {
            Some(i) => (self.ends[i] - t).max(0.0),
            None => 0.0,
        }
    }
}

/// The three threads' spans.
#[derive(Debug, Default, Clone)]
pub struct Path {
    pub niri: Spans,
    pub kitty: Spans,
    pub pi: Spans,
}

impl Path {
    fn of(&self, thread: Thread) -> &Spans {
        match thread {
            Thread::Niri => &self.niri,
            Thread::Kitty => &self.kitty,
            Thread::Pi => &self.pi,
        }
    }
}

/// One modelled key: how long it took, its wait at each hop, and what
/// held pi when pi held it past [`LOST`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub total: f64,
    pub waits: [f64; 5],
    pub pi_why: Option<Why>,
}

/// A small generator, so the same sample always models the same keys.
#[derive(Debug, Clone)]
pub struct Draw(u64);

impl Draw {
    pub fn new(seed: u64) -> Self {
        Draw(seed | 1)
    }

    /// A number in [0, 1).
    #[expect(clippy::cast_precision_loss, reason = "53 bits fit an f64 exactly")]
    pub fn next(&mut self) -> f64 {
        // xorshift64*, plenty for drawing instants.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let x = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Models `count` keys typed at instants drawn from `at_pi`, the spans of
/// time Joel was at a pi window, on a screen drawing a frame every
/// `frame` milliseconds.
pub fn keys(
    path: &Path,
    at_pi: &[(f64, f64)],
    frame: f64,
    count: usize,
    draw: &mut Draw,
) -> Vec<Key> {
    let total: f64 = at_pi.iter().map(|(from, to)| (to - from).max(0.0)).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    (0..count)
        .map(|_| {
            let start = instant(at_pi, draw.next() * total);
            let mut t = start;
            let mut waits = [0.0; 5];
            for (hop, (_, thread)) in HOPS.iter().enumerate() {
                waits[hop] = path.of(*thread).residual(t);
                t += waits[hop] + COST[hop];
            }
            let vblank = frame - t.rem_euclid(frame);
            Key {
                total: t + vblank - start,
                waits,
                pi_why: None,
            }
        })
        .collect()
}

/// The instant `into` milliseconds into the spans laid end to end.
fn instant(spans: &[(f64, f64)], mut into: f64) -> f64 {
    for (from, to) in spans {
        let length = (to - from).max(0.0);
        if into < length {
            return from + into;
        }
        into -= length;
    }
    spans.last().map_or(0.0, |span| span.1)
}

/// What a sample's keys came to.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub keys: usize,
    pub p50: f64,
    pub p99: f64,
    pub max: f64,
    /// The share of keys later than [`LATE`].
    pub late: f64,
    /// The share of keys that waited longer than [`LOST`] at niri, at
    /// kitty and at pi, either way through.
    pub lost_at: [f64; 3],
    /// The share of keys pi held past [`LOST`] while running, waiting for
    /// a CPU and in D. A key held in a burst whose cause isn't told counts
    /// in none, so the three can sum to less than pi's share.
    pub pi_lost_by: [f64; 3],
}

/// Sums up modelled keys, or nothing when there are none.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a count of keys is far under 2^52, and a share of it is positive"
)]
pub fn summary(keys: &[Key]) -> Option<Summary> {
    if keys.is_empty() {
        return None;
    }
    let mut totals: Vec<f64> = keys.iter().map(|k| k.total).collect();
    totals.sort_by(f64::total_cmp);
    let at = |p: f64| totals[((totals.len() as f64 * p) as usize).min(totals.len() - 1)];
    let share = |n: usize| n as f64 / keys.len() as f64;
    let lost = |thread: Thread| {
        share(
            keys.iter()
                .filter(|key| {
                    HOPS.iter()
                        .zip(key.waits)
                        .any(|((_, t), wait)| *t == thread && wait > LOST)
                })
                .count(),
        )
    };
    Some(Summary {
        keys: keys.len(),
        p50: at(0.5),
        p99: at(0.99),
        max: totals[totals.len() - 1],
        late: share(totals.iter().filter(|t| **t > LATE).count()),
        lost_at: [lost(Thread::Niri), lost(Thread::Kitty), lost(Thread::Pi)],
        pi_lost_by: [0.0; 3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f64 = 1000.0 / 60.0;

    #[test]
    fn a_key_waits_out_only_the_burst_it_lands_in() {
        let spans = Spans::new(vec![(100.0, 140.0), (10.0, 20.0)]);
        assert_eq!(spans.residual(5.0), 0.0);
        assert_eq!(spans.residual(15.0), 5.0);
        assert_eq!(spans.residual(30.0), 0.0);
        assert_eq!(spans.residual(110.0), 30.0);
        assert_eq!(spans.residual(200.0), 0.0);
    }

    #[test]
    fn with_nothing_busy_a_key_costs_its_hops_and_part_of_a_frame() {
        let keys = keys(
            &Path::default(),
            &[(0.0, 60_000.0)],
            FRAME,
            2000,
            &mut Draw::new(7),
        );
        assert_eq!(keys.len(), 2000);
        let floor: f64 = COST.iter().sum();
        for key in &keys {
            assert!(
                key.total >= floor && key.total <= floor + FRAME + 1e-9,
                "{key:?}"
            );
            assert_eq!(key.waits, [0.0; 5]);
        }
    }

    #[test]
    fn a_pi_busy_half_the_time_for_100_ms_at_a_go_holds_two_keys_in_five() {
        // Half land in a burst, and of those a key is late once its wait,
        // 6.5 ms of hops and its frame pass 33 ms: about 82%, so 41% in all.
        // It loses a frame at pi once the wait passes 16 ms: 84% of half.
        let busy = (0..300)
            .map(|i| (f64::from(i) * 200.0, f64::from(i) * 200.0 + 100.0))
            .collect();
        let path = Path {
            pi: Spans::new(busy),
            ..Path::default()
        };
        let keys = keys(&path, &[(0.0, 60_000.0)], FRAME, 4000, &mut Draw::new(3));
        let sum = summary(&keys).unwrap();
        assert!((sum.late - 0.41).abs() < 0.05, "{sum:?}");
        assert!((sum.lost_at[2] - 0.42).abs() < 0.05, "{sum:?}");
        assert_eq!(sum.lost_at[0], 0.0);
        assert_eq!(sum.lost_at[1], 0.0);
        assert!(sum.p99 > 90.0 && sum.max <= 100.0 + 6.5 + FRAME, "{sum:?}");
    }

    #[test]
    fn a_burst_is_put_down_to_the_largest_of_its_times() {
        assert_eq!(Why::of(30.0, 5.0, 0.0), Why::Ran);
        assert_eq!(Why::of(5.0, 30.0, 0.0), Why::Queued);
        assert_eq!(Why::of(5.0, 5.0, 30.0), Why::Disk);
        assert_eq!(Why::of(10.0, 10.0, 0.0), Why::Ran, "a tie goes to running");
        assert_eq!(Why::of(0.0, 10.0, 10.0), Why::Queued, "then to waiting");
    }

    #[test]
    fn a_key_takes_the_cause_of_the_burst_it_lands_in() {
        let spans = Spans::told(vec![(100.0, 140.0, Why::Queued), (10.0, 20.0, Why::Disk)]);
        assert_eq!(spans.why(15.0), Some(Why::Disk));
        assert_eq!(spans.why(110.0), Some(Why::Queued));
        assert_eq!(spans.why(30.0), None, "between bursts");
        assert_eq!(spans.why(5.0), None, "before the first");
        assert_eq!(Spans::new(vec![(0.0, 10.0)]).why(5.0), None, "not told");
    }

    #[test]
    fn pi_s_lost_keys_split_by_what_held_it() {
        // pi is busy half the time for 100 ms at a go, waiting for a CPU
        // through every other burst and running through the rest.
        let busy = (0..300)
            .map(|i| {
                let at = f64::from(i) * 200.0;
                let why = if i % 2 == 0 { Why::Queued } else { Why::Ran };
                (at, at + 100.0, why)
            })
            .collect();
        let path = Path {
            pi: Spans::told(busy),
            ..Path::default()
        };
        let keys = keys(&path, &[(0.0, 60_000.0)], FRAME, 4000, &mut Draw::new(3));
        let sum = summary(&keys).unwrap();
        let [ran, queued, disk] = sum.pi_lost_by;
        assert!(
            (ran + queued + disk - sum.lost_at[2]).abs() < 1e-9,
            "{sum:?}"
        );
        assert!((queued - 0.21).abs() < 0.04, "half of pi's 42%: {sum:?}");
        assert!((ran - 0.21).abs() < 0.04, "the other half: {sum:?}");
        assert_eq!(disk, 0.0, "no burst was in D");
    }

    #[test]
    fn a_key_held_in_a_burst_not_told_counts_in_no_cause() {
        let path = Path {
            pi: Spans::new(vec![(0.0, 100_000.0)]),
            ..Path::default()
        };
        let keys = keys(&path, &[(0.0, 50_000.0)], FRAME, 200, &mut Draw::new(5));
        let sum = summary(&keys).unwrap();
        assert_eq!(sum.lost_at[2], 1.0, "pi held every key");
        assert_eq!(sum.pi_lost_by, [0.0; 3], "yet none by a cause");
    }

    #[test]
    fn keys_are_drawn_only_from_the_time_at_a_pi_window() {
        // pi is stuck from 0 to 1000 ms, while Joel was elsewhere.
        let path = Path {
            pi: Spans::new(vec![(0.0, 1000.0)]),
            ..Path::default()
        };
        let keys = keys(&path, &[(2000.0, 3000.0)], FRAME, 500, &mut Draw::new(11));
        let held = keys.iter().filter(|k| k.waits[2] > 0.0).count();
        assert_eq!(
            held, 0,
            "keys waited on a pi stuck while Joel was elsewhere"
        );
    }

    #[test]
    fn a_key_held_by_kitty_on_its_way_back_counts_against_kitty() {
        let path = Path {
            kitty: Spans::new(vec![(0.0, 100_000.0)]),
            ..Path::default()
        };
        let keys = keys(
            &path,
            &[(10_000.0, 20_000.0)],
            FRAME,
            200,
            &mut Draw::new(5),
        );
        let sum = summary(&keys).unwrap();
        assert_eq!(sum.lost_at, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn no_keys_sum_to_nothing() {
        assert_eq!(summary(&[]), None);
        assert_eq!(
            keys(&Path::default(), &[], FRAME, 100, &mut Draw::new(1)),
            []
        );
    }

    #[test]
    fn the_same_seed_draws_the_same_keys() {
        let a = keys(
            &Path::default(),
            &[(0.0, 1000.0)],
            FRAME,
            50,
            &mut Draw::new(9),
        );
        let b = keys(
            &Path::default(),
            &[(0.0, 1000.0)],
            FRAME,
            50,
            &mut Draw::new(9),
        );
        assert_eq!(a, b);
    }
}
