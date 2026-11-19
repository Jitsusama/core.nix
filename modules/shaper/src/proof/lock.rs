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
        let dir = a_folder("shaper-alone");
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
        let next = taken_soon(&marker);
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
    // the lock keeps it past the proof's drop, until the child execs,
    // and no longer once it has, though it runs on.
    #[test]
    fn a_child_forked_while_held_keeps_the_lock_until_it_execs() {
        use std::io::Read;
        use std::os::unix::process::CommandExt;
        let dir = a_folder("shaper-forked");
        let marker = dir.join("was-on");
        let first = alone(&marker).expect("the first proof");
        // std's pipes are close-on-exec, so the child holds its ends only
        // until its exec, as does a child another test forks meanwhile.
        let (mut forked, told) = std::io::pipe().expect("a pipe for the fork");
        let (held, mut go) = std::io::pipe().expect("a pipe for the exec");
        let (told_fd, held_fd) = (told.as_raw_fd(), held.as_raw_fd());
        // The child is forked with a copy of every end, close-on-exec or
        // not, so it closes its copy of `go` before it waits: with the
        // parent's the only writer left, `go` dropping is the end of the
        // pipe, and a check that panics ends the child before its exec.
        let go_fd = go.as_raw_fd();
        let child = std::thread::spawn(move || {
            let mut command = std::process::Command::new("sleep");
            // It sleeps far past the 2 s the taking below may take, so
            // the lock was free while it ran, and a test killed before it
            // kills the child leaves it a minute at most.
            command
                .arg("60")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            // SAFETY: between its fork and its exec the child only closes
            // a descriptor, writes a byte, reads one and makes an error of
            // a number, all async-signal-safe.
            unsafe {
                command.pre_exec(move || {
                    libc::close(go_fd);
                    // A child that can't tell the parent ends here, so the
                    // parent hears `told` end rather than wait on it.
                    if libc::write(told_fd, [1_u8].as_ptr().cast(), 1) != 1 {
                        return Err(std::io::Error::from_raw_os_error(libc::EIO));
                    }
                    // Only the byte the parent writes once its check holds
                    // lets it exec: the end of the pipe, a check failing,
                    // ends the child here, and spawn gives the error.
                    let mut byte = 0_u8;
                    if libc::read(held_fd, (&raw mut byte).cast(), 1) == 1 {
                        Ok(())
                    } else {
                        Err(std::io::Error::from_raw_os_error(libc::ECANCELED))
                    }
                });
            }
            // A spawn returns once the child has execed, and one that
            // never forked lets go of `told` here, which the read hears.
            let spawned = command.spawn();
            drop((told, held));
            spawned
        });
        let mut byte = [0_u8; 1];
        let heard = forked.read(&mut byte).expect("the fork's pipe");
        assert_eq!(heard, 1, "the child never forked");
        drop(first);
        // The child waits for this check before it execs, however slow
        // the machine, and a check that panics ends it as `go` drops.
        assert!(
            matches!(alone(&marker), Err(NotAlone::Another(_))),
            "a forked child let go of the lock before it execed"
        );
        go.write_all(&[1]).expect("the child let go");
        let mut sleeping = child
            .join()
            .expect("the child's thread")
            .expect("the child ran");
        let next = taken_soon(&marker);
        let running = sleeping.try_wait();
        let _ = sleeping.kill();
        let _ = sleeping.wait();
        assert!(
            matches!(running, Ok(None)),
            "the child ended before the lock was taken: {running:?}"
        );
        assert!(
            next.is_ok(),
            "a child that execed still held the lock: {next:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A folder of this test's own in the temp folder, made here and
    /// private: never one already there, which another user sharing the
    /// temp folder could have made, with a link where the lock goes.
    fn a_folder(name: &str) -> std::path::PathBuf {
        use std::os::unix::fs::DirBuilderExt;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        let dir = std::env::temp_dir().join(format!("{name}-{}-{nanos}", std::process::id()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .expect("a folder of its own");
        dir
    }

    /// Takes the lock beside `marker`, for up to 2 s while another holds
    /// it: flock(2) follows the open file, which a child another test
    /// forks shares until it execs, so a lock may outlive its drop by
    /// that moment.
    fn taken_soon(marker: &Path) -> Result<fs::File, NotAlone> {
        let freed = Instant::now() + Duration::from_secs(2);
        loop {
            match alone(marker) {
                Err(NotAlone::Another(_)) if Instant::now() < freed => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                next => return next,
            }
        }
    }
}
