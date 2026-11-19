//! One proof at a time: a lock beside the shaper's word on disk, held
//! for as long as a proof runs, however it ends, since two at once would
//! load the link twice, and each would take the other's shaper for its
//! own.

use std::fs;
use std::io::{Seek, Write};
use std::os::fd::AsRawFd;
use std::path::Path;

/// Why a proof can't run alone.
#[derive(Debug)]
pub enum NotAlone {
    /// Another proof holds the lock: its pid, as it wrote it.
    Another(String),
    /// The lock couldn't be opened, so it can't be told.
    Unopened(String),
}

/// Holds the lock beside `marker` for as long as the file it gives is
/// open, writing this proof's pid in it.
pub fn alone(marker: &Path) -> Result<fs::File, NotAlone> {
    let at = marker.with_file_name("shaper-proof.lock");
    let mut lock = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&at)
        .map_err(|e| NotAlone::Unopened(format!("{}: {e}", at.display())))?;
    // SAFETY: flock(2) on a file this holds open.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let who = fs::read_to_string(&at).unwrap_or_default();
        return Err(NotAlone::Another(who.trim().to_string()));
    }
    let _ = lock.set_len(0);
    let _ = lock.rewind();
    let _ = writeln!(lock, "{}", std::process::id());
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_proof_runs_at_once() {
        let dir = std::env::temp_dir().join(format!("shaper-alone-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let first = alone(&marker).expect("the first proof");
        match alone(&marker) {
            Err(NotAlone::Another(who)) => {
                assert_eq!(who, std::process::id().to_string(), "the holder unnamed");
            }
            other => panic!("a second proof ran beside the first: {other:?}"),
        }
        drop(first);
        // flock(2) follows the open file, which a child another test forks
        // shares until it execs and close-on-exec lets go of it, so the
        // lock may outlive the drop by that moment.
        let freed = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let next = loop {
            match alone(&marker) {
                Err(NotAlone::Another(_)) if std::time::Instant::now() < freed => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                next => break next,
            }
        };
        assert!(next.is_ok(), "a proof gone still held the next: {next:?}");
        assert!(
            matches!(alone(&dir.join("no/such")), Err(NotAlone::Unopened(_))),
            "a lock it couldn't open taken for another proof's"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
