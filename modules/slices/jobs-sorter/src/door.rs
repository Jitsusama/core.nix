//! Holds a new Nix evaluation or build at the door while the machine is
//! short of memory, so heavy work comes in as there's room for it rather
//! than all at once.
//!
//! On the evening of 8 November agents' evaluations here, a few at a time,
//! took swap from 1 to 29 GiB and available memory down to 12 GiB within
//! 40 minutes of a boot: a whole-zone derivation's evaluation at 11 GB,
//! four nix-unit runs at about 11 GB together, and more before them. Each
//! started while there was room and grew past it beside the others. On 9
//! November a switch's hold ended and about 13 sessions started their
//! builds within a minute: the jobs slice went from 4 to 11 GB of swap in
//! four minutes, and a call's microphone went unread for 59 s.
//!
//! So a heavy start, one [`heavy`] knows by its words, under a `--door`
//! cgroup and seen by the sorter within [`YOUNG`] of its start, is let in
//! only while what's available, less what those let in lately haven't
//! grown to yet, leaves [`Machine::room`] once it has grown by
//! [`Machine::grows`]. Otherwise
//! it's frozen in its scope as it starts, small, and says so on its
//! standard error, and the oldest held comes in first once there's room.
//! Each let in counts as that growth, less what its scope already holds, for
//! [`RAMP`] after, so starts that come together are let in only as many
//! as there's room for, then one at a time as memory shows how much they
//! took. Both are shares of the machine's memory, so the door works the
//! same on a laptop, a desktop and a virtual machine's test. While a call
//! is on, more room is kept. None is
//! held past [`LONGEST`] once no call is on, so one never waits forever on
//! memory others keep, and those that waited that long come in
//! [`SPACED`] apart.
//!
//! Only a scope of its own is frozen, a `work-` scope the sorter split it
//! into, never a service's cgroup that holds other work too, and never a
//! scope already let in, so a run's own later evaluations aren't held
//! halfway through it. A sorter started again freezes nothing already
//! running.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a start is counted to be growing.
pub const RAMP: Duration = Duration::from_mins(2);

/// The longest a start is held while no call is on.
pub const LONGEST: Duration = Duration::from_mins(30);

/// The time between two let in because they waited [`LONGEST`].
pub const SPACED: Duration = Duration::from_secs(30);

/// How soon after its start a process must be seen to be held, in clock
/// ticks.
pub const YOUNG: u64 = 500;

/// Whether a command line, its words, is Nix work that grows: an
/// evaluation, a `nix flake check`, a build, a shell or a run, and
/// `nix-offload there`, which evaluates here before it sends.
pub fn heavy(words: &[&str]) -> bool {
    let Some(program) = words.first() else {
        return false;
    };
    let name = program.rsplit('/').next().unwrap_or(program);
    let mut verbs = words[1..].iter().filter(|word| !word.starts_with('-'));
    match name {
        "nix-instantiate" | "nix-unit" | "nix-eval-jobs" | "nix-build" | "nix-shell" => true,
        "nix-store" => words[1..]
            .iter()
            .any(|word| *word == "-r" || *word == "--realise"),
        "nix-offload" => verbs.next() == Some(&"there"),
        "nix" => match verbs.next() {
            Some(&("eval" | "build" | "develop" | "shell" | "run")) => true,
            Some(&"flake") => verbs.next() == Some(&"check"),
            _ => false,
        },
        _ => false,
    }
}

/// The kernel's available memory, from /proc/meminfo's text, in bytes.
pub fn available(meminfo: &str) -> Option<u64> {
    field(meminfo, "MemAvailable:")
}

/// The machine's memory, from /proc/meminfo's text, in bytes.
pub fn total(meminfo: &str) -> Option<u64> {
    field(meminfo, "MemTotal:")
}

/// The figure /proc/meminfo's text gives after `name`, in bytes.
fn field(meminfo: &str, name: &str) -> Option<u64> {
    meminfo.lines().find_map(|line| {
        let kib: u64 = line
            .strip_prefix(name)?
            .trim()
            .strip_suffix("kB")?
            .trim()
            .parse()
            .ok()?;
        kib.checked_mul(1024)
    })
}

/// A start held at the door.
struct Held {
    pid: u32,
    scope: PathBuf,
    since: Instant,
}

/// What the door has seen, holds and has let in.
#[derive(Default)]
pub struct Door {
    seen: HashSet<(u32, u64)>,
    held: Vec<Held>,
    /// Each scope let in, with when.
    admitted: HashMap<PathBuf, Instant>,
    waited_in: Option<Instant>,
}

/// One process under the door, as the sorter reads it.
pub struct Coming<'a> {
    pub pid: u32,
    /// When it started, in clock ticks since boot.
    pub start: u64,
    pub words: Vec<&'a str>,
    /// The cgroup it's in, a folder under /sys/fs/cgroup.
    pub scope: PathBuf,
    /// What its cgroup holds, in bytes.
    pub memory: u64,
}

/// What the machine has, as the door weighs it.
#[derive(Clone, Copy)]
pub struct Machine {
    /// Its memory, in bytes.
    pub total: u64,
    /// Available memory, in bytes.
    pub free: u64,
    /// Whether a call is on.
    pub calling: bool,
}

impl Machine {
    /// Available memory a start must leave once it has grown: a quarter of
    /// the machine's, 16 GiB of 64, where 9 November's thirteen starts
    /// would have come in two at a time. Through a call it's three
    /// eighths, so the call's programs and their camera never wait on
    /// memory.
    pub fn room(self) -> u64 {
        self.total / 16 * if self.calling { 6 } else { 4 }
    }

    /// How much a start is counted to grow by until [`RAMP`] has passed or
    /// its scope holds that much: a sixteenth of the machine's memory, 4
    /// GiB of 64, where an evaluation grows to 4 to 11 GB.
    pub fn grows(self) -> u64 {
        self.total / 16
    }
}

/// What the door did to a process.
#[derive(Debug, PartialEq, Eq)]
pub enum Done {
    Held(u32),
    LetIn(u32),
}

impl Door {
    /// Looks at what's under the door `now`, `uptime` ticks since boot, on
    /// `machine`: lets each new young heavy start in a scope of its own in
    /// while there's room for it and nothing waits before it, holds it
    /// otherwise, and lets the oldest held in as room comes, or once it's
    /// waited [`LONGEST`] with no call on. `freeze` freezes or thaws a
    /// scope and says whether it could.
    pub fn look(
        &mut self,
        coming: &[Coming],
        machine: Machine,
        now: Instant,
        uptime: u64,
        mut freeze: impl FnMut(&Path, bool) -> bool,
    ) -> Vec<Done> {
        let mut done = Vec::new();
        let here: HashSet<(u32, u64)> = coming.iter().map(|it| (it.pid, it.start)).collect();
        let pids: HashSet<u32> = coming.iter().map(|it| it.pid).collect();
        let memory: HashMap<&Path, u64> = coming
            .iter()
            .map(|it| (it.scope.as_path(), it.memory))
            .collect();
        self.seen.retain(|it| here.contains(it));
        self.held.retain(|held| pids.contains(&held.pid));
        self.admitted
            .retain(|scope, _| memory.contains_key(scope.as_path()));
        let (need, grows) = (machine.room(), machine.grows());
        let growing: u64 = self
            .admitted
            .iter()
            .filter(|(_, at)| now.saturating_duration_since(**at) < RAMP)
            .map(|(scope, _)| grows.saturating_sub(memory[scope.as_path()]))
            .sum();
        let mut room = machine.free.saturating_sub(growing);
        let fits = |room: u64| room >= need + grows;
        // The oldest held come in first while there's room.
        while fits(room) && !self.held.is_empty() {
            let first = self.held.remove(0);
            if freeze(&first.scope, false) {
                done.push(Done::LetIn(first.pid));
                self.admitted.insert(first.scope, now);
                room -= grows;
            }
        }
        for it in coming {
            if self.seen.contains(&(it.pid, it.start)) {
                continue;
            }
            let young = uptime.saturating_sub(it.start) <= YOUNG;
            let weighed = young && heavy(&it.words);
            // Work runs a second where it was written before the sorter
            // splits it into a scope of its own, so a young heavy start is
            // looked at again until it's there.
            if weighed && !own_scope(&it.scope) {
                continue;
            }
            self.seen.insert((it.pid, it.start));
            let known = self.admitted.contains_key(&it.scope)
                || self.held.iter().any(|held| held.scope == it.scope);
            if !weighed || known {
                continue;
            }
            if fits(room) && self.held.is_empty() {
                self.admitted.insert(it.scope.clone(), now);
                room -= grows;
            } else if freeze(&it.scope, true) {
                self.held.push(Held {
                    pid: it.pid,
                    scope: it.scope.clone(),
                    since: now,
                });
                done.push(Done::Held(it.pid));
            }
        }
        // One that has waited long enough goes, whatever the room, one at a
        // time, but never into a call.
        let spaced = self
            .waited_in
            .is_none_or(|at| now.saturating_duration_since(at) >= SPACED);
        if !machine.calling
            && spaced
            && self
                .held
                .first()
                .is_some_and(|held| now.saturating_duration_since(held.since) >= LONGEST)
        {
            let first = self.held.remove(0);
            if freeze(&first.scope, false) {
                done.push(Done::LetIn(first.pid));
                self.admitted.insert(first.scope, now);
                self.waited_in = Some(now);
            }
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

/// What the cgroup at `scope` holds, in bytes; none when it can't say.
pub fn memory(scope: &Path) -> u64 {
    fs::read_to_string(scope.join("memory.current"))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Tells the process `pid`, on its standard error, that it waits at the
/// door on `machine`; a process with nowhere to say it hears nothing.
pub fn tell(pid: u32, machine: Machine) {
    if let Ok(mut said) = fs::OpenOptions::new()
        .append(true)
        .open(format!("/proc/{pid}/fd/2"))
    {
        let why = if machine.calling {
            "a call is on and this machine has"
        } else {
            "this machine has"
        };
        let _ = writeln!(
            said,
            "jobs-sorter: {why} {} GB of memory free, so this waits for room{}",
            machine.free / 1_000_000_000,
            if machine.calling {
                ", until the call ends"
            } else {
                ", at most 30 minutes"
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_work_that_grows_is_known_by_its_words() {
        for words in [
            "nix eval --json .#checks",
            "/nix/store/x-nix/bin/nix --option eval .#x",
            "nix flake check",
            "nix build .#x",
            "nix develop",
            "nix shell nixpkgs#ruff -c ruff",
            "nix-instantiate --eval",
            "nix-unit --flake .#tests",
            "nix-eval-jobs --flake .#hydraJobs",
            "nix-build -A x",
            "nix-store -r /nix/store/x.drv",
            "nix-store --realise /nix/store/x.drv",
            "/nix/store/y-nix-offload/bin/nix-offload there -L . checks.x",
        ] {
            assert!(heavy(&words.split(' ').collect::<Vec<_>>()), "{words}");
        }
        for words in [
            "nix flake show",
            "nix path-info .#x",
            "nix-store -q",
            "nix-offload progress host",
            "cargo build",
            "",
        ] {
            assert!(!heavy(&words.split(' ').collect::<Vec<_>>()), "{words}");
        }
    }

    #[test]
    fn available_memory_is_read_in_bytes() {
        let meminfo = "MemTotal:       65000000 kB\nMemAvailable:   12000000 kB\n";
        assert_eq!(available(meminfo), Some(12_000_000 * 1024));
        assert_eq!(available("MemTotal: 1 kB\n"), None);
        assert_eq!(total(meminfo), Some(65_000_000 * 1024));
    }

    #[test]
    fn the_room_kept_and_growth_counted_are_shares_of_memory() {
        let small = Machine {
            total: 2 << 30,
            free: 0,
            calling: false,
        };
        assert_eq!((small.room(), small.grows()), (512 << 20, 128 << 20));
        let calling = Machine {
            calling: true,
            ..free(0)
        };
        assert_eq!(calling.room(), 24 << 30);
    }

    /// What 64 GiB of memory keeps and counts, as on the machine that
    /// met 9 November's thirteen starts.
    const TOTAL: u64 = 64 << 30;
    const ROOM: u64 = 16 << 30;
    const GROWS: u64 = 4 << 30;

    fn coming(pid: u32, start: u64, words: &'static str, scope: &str) -> Coming<'static> {
        Coming {
            pid,
            start,
            words: words.split(' ').collect(),
            scope: PathBuf::from(scope),
            memory: 0,
        }
    }

    fn free(free: u64) -> Machine {
        Machine {
            total: TOTAL,
            free,
            calling: false,
        }
    }

    /// Thirteen builds starting at once, as after the 9 November hold.
    fn thirteen() -> Vec<Coming<'static>> {
        (0..13)
            .map(|n| {
                let scope: &'static str = Box::leak(format!("/c/work-{n}-1.scope").into());
                coming(100 + n, 1000, "nix build .#x", scope)
            })
            .collect()
    }

    fn held(done: &[Done]) -> usize {
        done.iter()
            .filter(|done| matches!(done, Done::Held(_)))
            .count()
    }

    // On 8 November evaluations started a few at a time while there was
    // room and grew past it together, taking swap to 29 GiB.
    #[test]
    fn a_new_heavy_start_waits_while_memory_is_short_and_comes_in_once_there_is_room() {
        let mut door = Door::default();
        let mut frozen: Vec<(PathBuf, bool)> = Vec::new();
        let mut freezing = |scope: &Path, now: bool| {
            frozen.push((scope.to_path_buf(), now));
            true
        };
        let at = Instant::now();
        let short = free(10 << 30);
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
            "only a young heavy start in a scope of its own"
        );
        assert_eq!(
            door.look(&work, short, at, 1101, &mut freezing),
            [],
            "seen again, it isn't held twice"
        );
        let roomy = free(ROOM + GROWS);
        let done = door.look(&work, roomy, at, 1102, &mut freezing);
        assert_eq!(done, [Done::LetIn(10)]);
        assert_eq!(
            frozen,
            [
                (PathBuf::from("/c/work-10-1.scope"), true),
                (PathBuf::from("/c/work-10-1.scope"), false)
            ]
        );
    }

    // On 9 November about 13 sessions' builds started within a minute of
    // a hold ending, and a call's microphone went unread for 59 s.
    #[test]
    fn thirteen_starts_at_once_come_in_only_as_many_as_there_is_room_for() {
        let mut door = Door::default();
        let at = Instant::now();
        let mut work = thirteen();
        let done = door.look(&work, free(26_000_000_000), at, 1000, |_, _| true);
        assert_eq!(held(&done), 11, "two fit in 26 GB: {done:?}");
        // Grown to what they were counted at, memory shows it: none more.
        for it in &mut work[..2] {
            it.memory = GROWS;
        }
        let later = at + Duration::from_secs(10);
        let done = door.look(&work, free(18_000_000_000), later, 1001, |_, _| true);
        assert!(done.is_empty(), "{done:?}");
        // One ends and gives its memory back: one more comes in, the oldest.
        work.remove(0);
        let done = door.look(&work, free(22_000_000_000), later, 1002, |_, _| true);
        assert_eq!(done, [Done::LetIn(102)]);
    }

    #[test]
    fn a_start_let_in_counts_as_growing_until_its_ramp_has_passed() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = thirteen();
        let plenty = free(ROOM + 2 * GROWS);
        assert_eq!(
            held(&door.look(&work[..1], plenty, at, 1000, |_, _| true)),
            0
        );
        let second = &work[..2];
        let done = door.look(second, plenty, at, 1001, |_, _| true);
        assert_eq!(held(&done), 0, "room for two: {done:?}");
        let third = &work[..3];
        let done = door.look(third, plenty, at, 1002, |_, _| true);
        assert_eq!(done, [Done::Held(102)], "the first two still count");
        let done = door.look(third, plenty, at + RAMP, 1003, |_, _| true);
        assert_eq!(done, [Done::LetIn(102)], "their ramp has passed");
    }

    #[test]
    fn a_call_keeps_more_room_and_holds_its_starts_till_it_ends() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..1];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(
            door.look(work, calling, at, 1000, |_, _| true),
            [Done::Held(100)]
        );
        assert!(
            door.look(work, calling, at + LONGEST, 1001, |_, _| true)
                .is_empty(),
            "not let into a call, however long it has waited"
        );
        let ended = free(ROOM + GROWS);
        assert_eq!(
            door.look(work, ended, at + LONGEST, 1002, |_, _| true),
            [Done::LetIn(100)]
        );
    }

    #[test]
    fn those_that_waited_longest_come_in_spaced_and_none_waits_forever() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..2];
        let short = free(1 << 30);
        assert_eq!(held(&door.look(work, short, at, 1000, |_, _| true)), 2);
        assert_eq!(
            door.look(work, short, at + LONGEST, 1001, |_, _| true),
            [Done::LetIn(100)],
            "short of memory still, but it has waited long enough"
        );
        assert!(
            door.look(work, short, at + LONGEST, 1002, |_, _| true)
                .is_empty(),
            "the next waits its spacing"
        );
        assert_eq!(
            door.look(work, short, at + LONGEST + SPACED, 1003, |_, _| true),
            [Done::LetIn(101)]
        );
    }

    #[test]
    fn a_runs_own_later_starts_are_never_held() {
        let mut door = Door::default();
        let at = Instant::now();
        let first = [coming(
            50,
            1000,
            "nix-offload there .#x",
            "/c/work-50-1.scope",
        )];
        assert_eq!(door.look(&first, free(1 << 40), at, 1000, |_, _| true), []);
        let both = [
            coming(50, 1000, "nix-offload there .#x", "/c/work-50-1.scope"),
            coming(51, 1010, "nix eval .#x", "/c/work-50-1.scope"),
        ];
        assert!(
            door.look(&both, free(1 << 30), at, 1011, |_, _| true)
                .is_empty(),
            "its evaluation is the run's, let in with it"
        );
    }

    #[test]
    fn a_start_is_held_once_the_sorter_has_split_it_into_its_own_scope() {
        let mut door = Door::default();
        let at = Instant::now();
        let before = [coming(40, 1000, "nix eval .#a", "/c/jobs.service")];
        assert_eq!(
            door.look(&before, free(1 << 30), at, 1050, |_, _| true),
            [],
            "not yet in a scope of its own"
        );
        let after = [coming(40, 1000, "nix eval .#a", "/c/work-40-1.scope")];
        assert_eq!(
            door.look(&after, free(1 << 30), at, 1150, |_, _| true),
            [Done::Held(40)]
        );
    }

    #[test]
    fn one_that_ends_while_held_is_forgotten() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = [coming(30, 1000, "nix eval .#a", "/c/work-30-1.scope")];
        door.look(&work, free(1 << 30), at, 1000, |_, _| true);
        door.look(&[], free(1 << 30), at, 1001, |_, _| true);
        assert_eq!(door.held.len(), 0, "nothing is held");
    }
}
