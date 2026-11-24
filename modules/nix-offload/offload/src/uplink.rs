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
    /// The bytes they carried, as sized before each went: one this
    /// machine couldn't size adds none.
    pub bytes: u64,
    /// How many copies went up, whether they ended well or not.
    pub copies: u32,
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

/// Runs `copy`, a copy up of `bytes` where they're known, counting it.
pub fn sent<T>(bytes: Option<u64>, copy: impl FnOnce() -> T) -> T {
    let began = Instant::now();
    let ended = copy();
    let took = began.elapsed();
    SO_FAR.with(|so_far| {
        let mut spent = so_far.get();
        spent.sending += took;
        spent.bytes += bytes.unwrap_or(0);
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
        let ended = sent(Some(300), || 7);
        sent(None, || ());
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
        sent(Some(5), || ());
        begun();
        assert!(!taken().any(), "nothing was spent since it began");
    }

    #[test]
    fn runs_on_threads_of_their_own_count_apart() {
        begun();
        sent(Some(1), || ());
        let other = std::thread::spawn(|| {
            begun();
            sent(Some(40), || ());
            taken()
        })
        .join()
        .expect("the other run ended");
        assert_eq!(other.bytes, 40, "the other thread's run counts its own");
        assert_eq!(taken().bytes, 1, "this thread's run counts its own");
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
