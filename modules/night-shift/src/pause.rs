//! A night job pauses for a job Joel waits on, rather than holding the
//! turn its whole run: on 18 November World's nightly roll held it from
//! 01:31, and a switch's evaluation waited 17 minutes behind it and gave
//! up. A second heavy job beside the first is what the turn exists to
//! stop, so the night job doesn't run on beside the one Joel waits on;
//! it stops where it is, keeping what it holds, leaves the turn, and goes
//! on once the turn comes back to it.
//!
//! A job with a cgroup of its own is frozen by it, which holds every
//! process in it, those that left its process group too, and tells none
//! of them. A job without one is sent STOP in its process group, and CONT
//! to go on. Either way what it holds stays held while it waits, and the
//! kernel may reclaim it under pressure as from any job that sleeps.
//!
//! A pause holds only the job's own processes. What it handed elsewhere
//! runs on: a process that left its group when it has no cgroup apart,
//! and a build it asked nix-daemon for, which runs under the daemon.
//! World's night jobs are git, which does its work in its own processes.
//! A job stopped while paused goes on first, for at most its grace, so it
//! hears the TERM and gives back its locks.
//!
//! A job pauses once for each job Joel waits on, so one that is stopped
//! or stuck, and never takes the turn it was left, costs the night job
//! `turn::AHEAD_IS_LEFT_AT_MOST` once rather than every few seconds.

use crate::ahead::{self, Ahead};
use crate::holding;
use crate::turn::{self, Look};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How a night job is held still.
#[derive(Debug, PartialEq)]
pub enum Hold {
    /// By the `cgroup.freeze` of its own cgroup.
    Cgroup(PathBuf),
    /// By STOP and CONT to its process group.
    Group(i32),
}

// std links libc, so this comes from it.
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

const SIGCONT: i32 = 18;
const SIGSTOP: i32 = 19;

impl Hold {
    /// Holds the job still, or lets it go on.
    pub fn set(&self, still: bool) -> io::Result<()> {
        match self {
            Hold::Cgroup(cgroup) => {
                let file = cgroup.join("cgroup.freeze");
                fs::write(&file, if still { "1" } else { "0" }).map_err(|error| {
                    io::Error::new(error.kind(), format!("{}: {error}", file.display()))
                })
            }
            Hold::Group(group) => {
                let signal = if still { SIGSTOP } else { SIGCONT };
                // SAFETY: kill takes any pid and signal, and only reports
                // errors.
                if unsafe { kill(-group, signal) } == 0 {
                    Ok(())
                } else {
                    Err(io::Error::last_os_error())
                }
            }
        }
    }
}

/// A pause: since when, for whom, and the mark that it waits to take the
/// turn back, there until it does.
struct Paused {
    since: Instant,
    waiter: String,
    _mark: Ahead,
}

/// A running night job's share of the turn: it holds the turn until a job
/// Joel waits on waits for it, then pauses until the turn is back.
/// Dropped while paused, the job goes on, so a job stopped meanwhile
/// hears the TERM it is sent.
pub struct Sharing<'a> {
    lock: &'a File,
    runtime: &'a Path,
    name: &'a str,
    hold: Hold,
    paused: Option<Paused>,
    /// How long it was paused before this pause.
    before: Duration,
    /// Each job Joel waits on it has paused for, by process.
    paused_for: HashSet<String>,
    /// When the turn, free, was first left to another, as `turn::look`
    /// keeps it.
    left: Option<Instant>,
}

impl<'a> Sharing<'a> {
    /// The share of the job `name`, which holds the turn by `lock` in
    /// `runtime` and is held still by `hold`.
    pub fn new(lock: &'a File, runtime: &'a Path, name: &'a str, hold: Hold) -> Self {
        Sharing {
            lock,
            runtime,
            name,
            hold,
            paused: None,
            before: Duration::ZERO,
            paused_for: HashSet::new(),
            left: None,
        }
    }

    /// Whether the job is paused now.
    pub fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    /// How long the job has been paused in all.
    pub fn paused(&self) -> Duration {
        self.before
            + self
                .paused
                .as_ref()
                .map_or(Duration::ZERO, |now| now.since.elapsed())
    }

    /// Looks once: a job holding the turn pauses when one Joel waits on
    /// waits for it, and a paused one goes on once the turn is back.
    /// Gives what that changed, to say; fails when a paused job can't take
    /// the turn back or can't go on once it has, which stops it.
    pub fn look(&mut self) -> io::Result<Option<String>> {
        if let Some(paused) = &self.paused {
            let found =
                turn::look(self.lock, self.runtime, false, &mut self.left).map_err(|error| {
                    io::Error::new(
                        error.kind(),
                        format!("it can't take the turn back: {error}"),
                    )
                })?;
            if !matches!(found, Look::Had) {
                return Ok(None);
            }
            let said = format!(
                "goes on, having paused {} s for {}",
                paused.since.elapsed().as_secs(),
                paused.waiter
            );
            self.before += paused.since.elapsed();
            self.paused = None;
            holding::note(self.runtime, self.name);
            // A job still held would keep the turn doing nothing until its
            // most, which no bar stops, as a held job stalls nobody.
            self.hold.set(false).map_err(|error| {
                io::Error::new(error.kind(), format!("it can't be let go on: {error}"))
            })?;
            return Ok(Some(said));
        }
        // Those already paused for are passed over rather than ending the
        // look, so one that never takes its turn hides none behind it.
        let Some(waiter) =
            ahead::next_but(self.runtime, &self.paused_for).filter(|waiter| !waiter.paused)
        else {
            return Ok(None);
        };
        self.paused_for.insert(waiter.pid.clone());
        Ok(Some(match self.pause(&waiter.name) {
            Ok(()) => format!("pauses, leaving the turn to {}", waiter.why()),
            Err(error) => format!(
                "runs on with the turn, as it can't pause for {}: {error}",
                waiter.name
            ),
        }))
    }

    /// Holds the job still, marks it paused and lets go of the turn. A job
    /// it can't pause whole keeps the turn and runs on.
    fn pause(&mut self, waiter: &str) -> io::Result<()> {
        self.hold.set(true)?;
        let marked = Ahead::paused(self.runtime, self.name)
            .and_then(|mark| self.lock.unlock().map(|()| mark));
        match marked {
            Ok(mark) => {
                self.left = None;
                self.paused = Some(Paused {
                    since: Instant::now(),
                    waiter: waiter.to_owned(),
                    _mark: mark,
                });
                Ok(())
            }
            Err(error) => {
                let _ = self.hold.set(false);
                Err(error)
            }
        }
    }
}

impl Drop for Sharing<'_> {
    fn drop(&mut self) {
        if self.paused.is_some() {
            let _ = self.hold.set(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cgroup_is_frozen_and_thawed_by_its_freeze_file() {
        let cgroup = std::env::temp_dir().join(format!("night-shift-pause-{}", std::process::id()));
        fs::create_dir_all(&cgroup).unwrap();
        let hold = Hold::Cgroup(cgroup.clone());
        hold.set(true).unwrap();
        assert_eq!(
            fs::read_to_string(cgroup.join("cgroup.freeze")).unwrap(),
            "1"
        );
        hold.set(false).unwrap();
        assert_eq!(
            fs::read_to_string(cgroup.join("cgroup.freeze")).unwrap(),
            "0"
        );
        fs::remove_dir_all(&cgroup).unwrap();
    }
}
