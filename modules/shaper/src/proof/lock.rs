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
/// open, writing this proof's pid in it. The lock follows the open file,
/// which a child forked meanwhile shares until it execs, and std opens
/// every file close-on-exec, so it goes at the exec. A child a proof
/// starts must therefore exec at once: a `pre_exec` that waited, as
/// swayidle's must not, or a fork that never execs, would hold the lock
/// past the proof.
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
    use std::time::{Duration, Instant};

    #[test]
    fn only_one_proof_runs_at_once() {
        let dir = std::env::temp_dir().join(format!("shaper-alone-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        // A proof gone before left its pid, longer than this one's.
        fs::write(dir.join("shaper-proof.lock"), "99999999\n").expect("a stale lock");
        let first = alone(&marker).expect("the first proof");
        // SAFETY: fcntl(2) reading the flags of a file this holds open.
        let flags = unsafe { libc::fcntl(first.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0, "fcntl: {}", std::io::Error::last_os_error());
        assert!(
            flags & libc::FD_CLOEXEC != 0,
            "a child that execs would keep the lock"
        );
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
        let freed = Instant::now() + Duration::from_secs(2);
        let next = loop {
            match alone(&marker) {
                Err(NotAlone::Another(_)) if Instant::now() < freed => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                next => break next,
            }
        };
        assert!(
            next.is_ok(),
            "a proof gone still held the next 2 s after the drop: {next:?}"
        );
        assert!(
            matches!(alone(&dir.join("no/such")), Err(NotAlone::Unopened(_))),
            "a lock it couldn't open taken for another proof's"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    // What the wait above outlasts: a child forked while a proof holds
    // the lock keeps it past the proof's drop, until the child execs.
    #[test]
    fn a_child_forked_while_held_keeps_the_lock_until_it_execs() {
        use std::os::unix::process::CommandExt;
        let dir = std::env::temp_dir().join(format!("shaper-forked-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder");
        let marker = dir.join("was-on");
        let first = alone(&marker).expect("the first proof");
        let mut ends = [0; 2];
        // SAFETY: pipe(2) filling two descriptors this test owns.
        assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0, "a pipe");
        let [heard, told] = ends;
        let child = std::thread::spawn(move || {
            let mut command = std::process::Command::new("true");
            // SAFETY: the child only writes a byte and sleeps, both
            // async-signal-safe, between its fork and its exec.
            unsafe {
                command.pre_exec(move || {
                    let _ = libc::write(told, [1_u8].as_ptr().cast(), 1);
                    std::thread::sleep(Duration::from_millis(300));
                    Ok(())
                });
            }
            command.status()
        });
        let mut forked = [0_u8; 1];
        // SAFETY: read(2) into a byte this test owns, from its pipe.
        let read = unsafe { libc::read(heard, forked.as_mut_ptr().cast(), 1) };
        assert_eq!(read, 1, "the child never forked");
        drop(first);
        assert!(
            matches!(alone(&marker), Err(NotAlone::Another(_))),
            "a forked child let go of the lock before it execed"
        );
        let ran = child.join().expect("the child's thread");
        assert!(ran.is_ok_and(|status| status.success()), "the child ran");
        assert!(alone(&marker).is_ok(), "a child gone still held the lock");
        // SAFETY: closing the pipe this test opened.
        unsafe {
            libc::close(heard);
            libc::close(told);
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
