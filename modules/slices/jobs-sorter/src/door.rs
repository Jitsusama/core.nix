//! Holds a new Nix evaluation at the door while the machine is short of
//! memory, so evaluations come in as there's room for them rather than
//! all at once.
//!
//! On the evening of 8 November agents' evaluations here, a few at a time,
//! took swap from 1 to 29 GiB and available memory down to 12 GiB within
//! 40 minutes of a boot: a whole-zone derivation's evaluation at 11 GB,
//! four nix-unit runs at about 11 GB together, and more before them. Each
//! started while there was room and grew past it beside the others. So
//! while the kernel says less than [`ROOM`] is available, an evaluation
//! the sorter hasn't seen before, under a `--door` cgroup, is frozen in
//! its scope as it starts, small, and says so on its standard error. Once
//! there's [`MARGIN`] more, the oldest held is thawed, one each
//! [`SPACED`], so they grow one after another. None is held past
//! [`LONGEST`], so one never waits forever on memory others keep.
//!
//! Only a scope of its own is frozen, a `work-` scope the sorter split it
//! into, never a service's cgroup that holds other work too, and only an
//! evaluation seen within [`YOUNG`] of its start, so a sorter started
//! again freezes nothing already running.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Available memory under which a new evaluation waits, in bytes: an
/// evaluation here grows to 4 to 11 GB.
pub const ROOM: u64 = 16 << 30;

/// How much more than [`ROOM`] lets a held one in, so the door doesn't
/// swing on every reading.
pub const MARGIN: u64 = 4 << 30;

/// The time between two evaluations let in.
pub const SPACED: Duration = Duration::from_secs(30);

/// The longest an evaluation is held.
pub const LONGEST: Duration = Duration::from_mins(30);

/// How soon after its start an evaluation must be seen to be held, in
/// clock ticks.
pub const YOUNG: u64 = 500;

/// Whether a command line, its words, is a Nix evaluation that grows: a
/// `nix eval` or `nix flake check`, or nix-instantiate, nix-unit or
/// nix-eval-jobs.
pub fn evaluation(words: &[&str]) -> bool {
    let Some(program) = words.first() else {
        return false;
    };
    let name = program.rsplit('/').next().unwrap_or(program);
    match name {
        "nix-instantiate" | "nix-unit" | "nix-eval-jobs" => true,
        "nix" => {
            let mut verbs = words[1..].iter().filter(|word| !word.starts_with('-'));
            match verbs.next() {
                Some(&"eval") => true,
                Some(&"flake") => verbs.next() == Some(&"check"),
                _ => false,
            }
        }
        _ => false,
    }
}

/// The kernel's available memory, from /proc/meminfo's text, in bytes.
pub fn available(meminfo: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let kib: u64 = line
            .strip_prefix("MemAvailable:")?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse()
            .ok()?;
        kib.checked_mul(1024)
    })
}

/// An evaluation held at the door.
struct Held {
    pid: u32,
    scope: PathBuf,
    since: Instant,
}

/// What the door has seen and holds.
#[derive(Default)]
pub struct Door {
    seen: HashSet<(u32, u64)>,
    held: Vec<Held>,
    let_in: Option<Instant>,
}

/// One process under the door, as the sorter reads it.
pub struct Coming<'a> {
    pub pid: u32,
    /// When it started, in clock ticks since boot.
    pub start: u64,
    pub words: Vec<&'a str>,
    /// The cgroup it's in, a folder under /sys/fs/cgroup.
    pub scope: PathBuf,
}

/// What the door did to a process.
#[derive(Debug, PartialEq, Eq)]
pub enum Done {
    Held(u32),
    LetIn(u32),
}

impl Door {
    /// Looks at what's under the door `now`, `uptime` ticks since boot, with
    /// `free` bytes available: holds each new young evaluation in a scope of
    /// its own while memory is short, and lets the oldest held in once
    /// there's room, or once it's waited [`LONGEST`]. `freeze` freezes or
    /// thaws a scope and says whether it could.
    pub fn look(
        &mut self,
        coming: &[Coming],
        free: u64,
        now: Instant,
        uptime: u64,
        mut freeze: impl FnMut(&Path, bool) -> bool,
    ) -> Vec<Done> {
        let mut done = Vec::new();
        let here: HashSet<(u32, u64)> = coming.iter().map(|it| (it.pid, it.start)).collect();
        let pids: HashSet<u32> = coming.iter().map(|it| it.pid).collect();
        self.seen.retain(|it| here.contains(it));
        self.held.retain(|held| pids.contains(&held.pid));
        for it in coming {
            if self.seen.contains(&(it.pid, it.start)) {
                continue;
            }
            let young = uptime.saturating_sub(it.start) <= YOUNG;
            // Work runs a second where it was written before the sorter
            // splits it into a scope of its own, so a young evaluation is
            // looked at again until it's there.
            if young && evaluation(&it.words) && !own_scope(&it.scope) {
                continue;
            }
            self.seen.insert((it.pid, it.start));
            let holdable = free < ROOM
                && young
                && evaluation(&it.words)
                && !self.held.iter().any(|held| held.scope == it.scope);
            if holdable && freeze(&it.scope, true) {
                self.held.push(Held {
                    pid: it.pid,
                    scope: it.scope.clone(),
                    since: now,
                });
                done.push(Done::Held(it.pid));
            }
        }
        // Those that have waited long enough go, whatever the room.
        let mut kept = Vec::new();
        for held in self.held.drain(..) {
            let long = now.saturating_duration_since(held.since) >= LONGEST;
            if long && freeze(&held.scope, false) {
                done.push(Done::LetIn(held.pid));
            } else {
                kept.push(held);
            }
        }
        self.held = kept;
        let spaced = self
            .let_in
            .is_none_or(|at| now.saturating_duration_since(at) >= SPACED);
        if free >= ROOM + MARGIN
            && spaced
            && let Some(first) = self.held.first()
            && freeze(&first.scope, false)
        {
            done.push(Done::LetIn(first.pid));
            self.held.remove(0);
            self.let_in = Some(now);
        }
        done
    }
}

/// Whether a cgroup is a scope the sorter split work into, which holds one
/// piece of work alone.
fn own_scope(scope: &Path) -> bool {
    scope
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with("work-")
                && Path::new(name)
                    .extension()
                    .is_some_and(|kind| kind == "scope")
        })
}

/// Freezes or thaws the cgroup at `scope`, saying whether it could.
pub fn freeze(scope: &Path, frozen: bool) -> bool {
    fs::write(scope.join("cgroup.freeze"), if frozen { "1" } else { "0" }).is_ok()
}

/// Tells the process `pid`, on its standard error, that it waits at the
/// door with `free` bytes available; a process with nowhere to say it
/// hears nothing.
pub fn tell(pid: u32, free: u64) {
    if let Ok(mut said) = fs::OpenOptions::new()
        .append(true)
        .open(format!("/proc/{pid}/fd/2"))
    {
        let _ = writeln!(
            said,
            "jobs-sorter: this machine has {} GB of memory free, so this evaluation waits \
             for room, at most 30 minutes",
            free / 1_000_000_000
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluations_that_grow_are_known_by_their_words() {
        for words in [
            "nix eval --json .#checks",
            "/nix/store/x-nix/bin/nix --option eval .#x",
            "nix flake check",
            "nix-instantiate --eval",
            "nix-unit --flake .#tests",
            "nix-eval-jobs --flake .#hydraJobs",
        ] {
            assert!(evaluation(&words.split(' ').collect::<Vec<_>>()), "{words}");
        }
        for words in [
            "nix build .#x",
            "nix flake show",
            "nix-store -q",
            "cargo build",
            "",
        ] {
            assert!(
                !evaluation(&words.split(' ').collect::<Vec<_>>()),
                "{words}"
            );
        }
    }

    #[test]
    fn available_memory_is_read_in_bytes() {
        let meminfo = "MemTotal:       65000000 kB\nMemAvailable:   12000000 kB\n";
        assert_eq!(available(meminfo), Some(12_000_000 * 1024));
        assert_eq!(available("MemTotal: 1 kB\n"), None);
    }

    fn coming(pid: u32, start: u64, words: &'static str, scope: &str) -> Coming<'static> {
        Coming {
            pid,
            start,
            words: words.split(' ').collect(),
            scope: PathBuf::from(scope),
        }
    }

    // On 8 November evaluations started a few at a time while there was
    // room and grew past it together, taking swap to 29 GiB.
    #[test]
    fn a_new_evaluation_waits_while_memory_is_short_and_comes_in_once_there_is_room() {
        let mut door = Door::default();
        let mut frozen: Vec<(PathBuf, bool)> = Vec::new();
        let mut freezing = |scope: &Path, now: bool| {
            frozen.push((scope.to_path_buf(), now));
            true
        };
        let at = Instant::now();
        let short = 10 << 30;
        let work = [
            coming(10, 1000, "nix eval .#x", "/c/work-10-1.scope"),
            coming(11, 1000, "cargo build", "/c/work-11-1.scope"),
            coming(12, 1000, "nix eval .#y", "/c/jobs.service"),
            coming(13, 1, "nix eval .#z", "/c/work-13-1.scope"),
        ];
        let done = door.look(&work, short, at, 1100, &mut freezing);
        assert_eq!(
            done,
            [Done::Held(10)],
            "only a young evaluation in a scope of its own"
        );
        assert_eq!(
            door.look(&work, short, at, 1101, &mut freezing),
            [],
            "seen again, it isn't held twice"
        );
        let roomy = ROOM + MARGIN;
        let done = door.look(&work, roomy, at + SPACED, 1102, &mut freezing);
        assert_eq!(done, [Done::LetIn(10)]);
        assert_eq!(
            frozen,
            [
                (PathBuf::from("/c/work-10-1.scope"), true),
                (PathBuf::from("/c/work-10-1.scope"), false)
            ]
        );
    }

    #[test]
    fn held_evaluations_come_in_one_at_a_time_spaced_and_none_waits_forever() {
        let mut door = Door::default();
        let mut freezing = |_: &Path, _: bool| true;
        let at = Instant::now();
        let second = Duration::from_secs(1);
        let work = [
            coming(20, 1000, "nix eval .#a", "/c/work-20-1.scope"),
            coming(21, 1000, "nix-unit .#b", "/c/work-21-1.scope"),
        ];
        door.look(&work, 1 << 30, at, 1000, &mut freezing);
        let roomy = ROOM + MARGIN;
        let later =
            |door: &mut Door, after: Duration, freezing: &mut dyn FnMut(&Path, bool) -> bool| {
                door.look(&work, roomy, at + after, 1001, freezing)
            };
        assert_eq!(later(&mut door, second, &mut freezing), [Done::LetIn(20)]);
        assert!(
            later(&mut door, second * 2, &mut freezing).is_empty(),
            "the next waits its spacing"
        );
        assert_eq!(
            later(&mut door, second + SPACED, &mut freezing),
            [Done::LetIn(21)]
        );

        let mut door = Door::default();
        door.look(&work, 1 << 30, at, 1000, &mut freezing);
        assert_eq!(
            door.look(&work, 1 << 30, at + LONGEST, 1001, &mut freezing),
            [Done::LetIn(20), Done::LetIn(21)],
            "short of memory still, but they've waited long enough"
        );
    }

    #[test]
    fn an_evaluation_is_held_once_the_sorter_has_split_it_into_its_own_scope() {
        let mut door = Door::default();
        let mut freezing = |_: &Path, _: bool| true;
        let at = Instant::now();
        let before = [coming(40, 1000, "nix eval .#a", "/c/jobs.service")];
        assert_eq!(
            door.look(&before, 1 << 30, at, 1050, &mut freezing),
            [],
            "not yet in a scope of its own"
        );
        let after = [coming(40, 1000, "nix eval .#a", "/c/work-40-1.scope")];
        assert_eq!(
            door.look(&after, 1 << 30, at, 1150, &mut freezing),
            [Done::Held(40)]
        );
    }

    #[test]
    fn one_that_ends_while_held_is_forgotten() {
        let mut door = Door::default();
        let mut freezing = |_: &Path, _: bool| true;
        let at = Instant::now();
        let work = [coming(30, 1000, "nix eval .#a", "/c/work-30-1.scope")];
        door.look(&work, 1 << 30, at, 1000, &mut freezing);
        door.look(&[], 1 << 30, at, 1001, &mut freezing);
        assert_eq!(door.held.len(), 0, "nothing is held");
    }
}
