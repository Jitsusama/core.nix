//! What a run spent on this machine's uplink: the time it waited for a
//! turn to send, the time its copies up ran, and the bytes they carried,
//! for its line in `there.jsonl`. A copy's time said nowhere else: on 23
//! November a run's send to a builder waited 146 minutes for a turn and
//! ended unsent, and only its own log said so. With it on each line, the
//! journal shows how much of a day's runs went to sending, and when.
//!
//! Each run is counted on its own thread, as runs sent at once each have
//! one, so the count is kept per thread and taken when the run ends.

use std::cell::Cell;
use std::time::{Duration, Instant};

/// What a run spent on the uplink.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    /// How long it waited for turns to send.
    pub waited: Duration,
    /// How long its copies up ran.
    pub sending: Duration,
    /// The bytes they carried, as sized before each went, of the copies
    /// that went up whole: one that failed or was stopped may have sent
    /// any part of its size, so it adds none, nor does one this machine
    /// couldn't size.
    pub bytes: u64,
    /// How many copies ran, whether they ended well or not, one sent
    /// again for a scope refused counting with it, as one. One whose
    /// `nix` never started sent nothing, so it isn't one.
    pub copies: u32,
}

/// How a copy up went, as [`sent`] counts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Went {
    /// It went up whole, so its bytes count.
    Whole,
    /// It ran and failed or was stopped, having sent any part of itself.
    Short,
    /// Its `nix` never started, so it sent nothing and took no time.
    Unrun,
}

impl Spent {
    const NONE: Self = Self {
        waited: Duration::ZERO,
        sending: Duration::ZERO,
        bytes: 0,
        copies: 0,
    };

    /// Whether it sent or waited at all.
    pub fn any(&self) -> bool {
        self.copies > 0 || !self.waited.is_zero()
    }

    /// Its line in the journal: whole seconds, as the line's own are.
    pub fn said(&self) -> serde_json::Value {
        serde_json::json!({
            "waited": self.waited.as_secs(),
            "seconds": self.sending.as_secs(),
            "bytes": self.bytes,
            "copies": self.copies,
        })
    }
}

thread_local! {
    static SO_FAR: Cell<Spent> = const { Cell::new(Spent::NONE) };
}

/// Begins a run's count on this thread, dropping what came before.
pub fn begun() {
    SO_FAR.with(|so_far| so_far.set(Spent::NONE));
}

/// What this thread's run has spent since it began, leaving none.
pub fn taken() -> Spent {
    SO_FAR.with(|so_far| so_far.replace(Spent::NONE))
}

/// Runs `wait` for a turn to send, counting how long it took.
pub fn waited<T>(wait: impl FnOnce() -> T) -> T {
    let began = Instant::now();
    let turn = wait();
    let took = began.elapsed();
    SO_FAR.with(|so_far| {
        let mut spent = so_far.get();
        spent.waited += took;
        so_far.set(spent);
    });
    turn
}

/// Runs `copy`, a copy up of `bytes` where they're known, counting it
/// where `went` says it ran, and its bytes only where it went up whole.
pub fn sent<T>(bytes: Option<u64>, copy: impl FnOnce() -> T, went: impl FnOnce(&T) -> Went) -> T {
    let began = Instant::now();
    let ended = copy();
    let took = began.elapsed();
    let carried = match went(&ended) {
        Went::Unrun => return ended,
        Went::Short => 0,
        Went::Whole => bytes.unwrap_or(0),
    };
    SO_FAR.with(|so_far| {
        let mut spent = so_far.get();
        spent.sending += took;
        spent.bytes += carried;
        spent.copies += 1;
        so_far.set(spent);
    });
    ended
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_counts_its_waits_and_copies_and_is_taken_whole() {
        begun();
        waited(|| std::thread::sleep(Duration::from_millis(20)));
        let ended = sent(Some(300), || 7, |_| Went::Whole);
        sent(None, || (), |()| Went::Whole);
        let spent = taken();
        assert_eq!(ended, 7, "a copy's own answer comes back");
        assert!(
            spent.waited >= Duration::from_millis(20),
            "the wait counted: {spent:?}"
        );
        assert_eq!(spent.bytes, 300, "a copy unsized adds no bytes");
        assert_eq!(spent.copies, 2, "both copies counted");
        assert_eq!(taken(), Spent::default(), "taking leaves none");
    }

    #[test]
    fn a_run_begun_drops_what_came_before_on_its_thread() {
        sent(Some(5), || (), |()| Went::Whole);
        begun();
        assert!(!taken().any(), "nothing was spent since it began");
    }

    #[test]
    fn runs_on_threads_of_their_own_count_apart() {
        begun();
        sent(Some(1), || (), |()| Went::Whole);
        let other = std::thread::spawn(|| {
            begun();
            sent(Some(40), || (), |()| Went::Whole);
            taken()
        })
        .join()
        .expect("the other run ended");
        assert_eq!(other.bytes, 40, "the other thread's run counts its own");
        assert_eq!(taken().bytes, 1, "this thread's run counts its own");
    }

    #[test]
    fn a_copy_that_failed_counts_its_time_but_none_of_its_bytes() {
        begun();
        let ended = sent(
            Some(900),
            || {
                std::thread::sleep(Duration::from_millis(10));
                false
            },
            |_| Went::Short,
        );
        let spent = taken();
        assert!(!ended, "a copy's own answer comes back");
        assert_eq!(
            (spent.bytes, spent.copies),
            (0, 1),
            "a failed copy's bytes counted, or the copy didn't"
        );
        assert!(
            spent.sending >= Duration::from_millis(10),
            "a failed copy's time didn't count: {spent:?}"
        );
    }

    // A copy whose nix couldn't be run sent nothing, so a run whose every
    // try failed so would otherwise say it went to the uplink.
    #[test]
    fn a_copy_whose_nix_never_started_isnt_counted() {
        begun();
        let ended = sent(Some(900), || "couldn't run nix", |_| Went::Unrun);
        let spent = taken();
        assert_eq!(ended, "couldn't run nix", "a copy's own answer comes back");
        assert!(!spent.any(), "a copy that never ran was counted: {spent:?}");
    }

    #[test]
    fn its_line_says_whole_seconds() {
        let spent = Spent {
            waited: Duration::from_millis(146_900),
            sending: Duration::from_millis(3_200),
            bytes: 194_704_264,
            copies: 2,
        };
        assert_eq!(
            spent.said(),
            serde_json::json!({"waited": 146, "seconds": 3, "bytes": 194_704_264, "copies": 2}),
            "its line"
        );
    }
}
