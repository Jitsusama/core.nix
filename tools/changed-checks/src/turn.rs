//! One run evaluating on this machine at a time.
//!
//! Runs are sent elsewhere, so the machine queue lets several start at
//! once, each light while it waits on a builder. When the lent builder
//! goes, they all fall back to evaluating here together: at 00:11 on 6
//! November six whole-flake evaluations began within one second, each
//! several gigabytes across its chunks, faster than the memory stall each
//! reads before an evaluation (`pressure.rs`) could rise, and a session
//! was killed for memory at 00:26. So a run evaluating here takes a turn,
//! a lock every run on the machine shares, and the rest wait for it.

use std::fs::{File, TryLockError};
use std::path::Path;

/// Take the turn the lock at `path` gives, calling `waiting` once first
/// when another run holds it. The turn lasts until what's given is
/// dropped, or the run ends.
pub fn taken(path: &Path, waiting: impl FnOnce()) -> Result<File, String> {
    let unreadable = |error: &dyn std::fmt::Display| format!("{}: {error}", path.display());
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| unreadable(&error))?;
    }
    let lock = File::create(path).map_err(|error| unreadable(&error))?;
    match lock.try_lock() {
        Ok(()) => return Ok(lock),
        Err(TryLockError::WouldBlock) => waiting(),
        Err(TryLockError::Error(error)) => return Err(unreadable(&error)),
    }
    lock.lock().map_err(|error| unreadable(&error))?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scratch::Scratch;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn a_turn_free_is_taken_at_once_without_waiting() {
        let scratch = Scratch::new("turn-free");
        let path = scratch.join("turns/here.lock");
        let mut waited = false;
        let _turn = taken(&path, || waited = true).unwrap();
        assert!(!waited);
    }

    #[test]
    fn a_turn_held_is_waited_for_and_taken_once_given_back() {
        let scratch = Scratch::new("turn-held");
        let path = scratch.join("here.lock");
        let held = taken(&path, || panic!("free at first")).unwrap();
        let (told, heard) = mpsc::channel();
        let other = path.clone();
        let waiter = std::thread::spawn(move || {
            let waiting = told.clone();
            let _turn = taken(&other, move || waiting.send("waiting").unwrap()).unwrap();
            told.send("taken").unwrap();
        });
        assert_eq!(heard.recv_timeout(Duration::from_secs(5)), Ok("waiting"));
        assert!(
            heard.recv_timeout(Duration::from_millis(300)).is_err(),
            "not taken while held"
        );
        drop(held);
        assert_eq!(heard.recv_timeout(Duration::from_secs(5)), Ok("taken"));
        waiter.join().unwrap();
    }
}
