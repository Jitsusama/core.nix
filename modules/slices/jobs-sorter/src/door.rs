//! Ranks a new Nix evaluation or build behind other work while the machine
//! is short of memory, and holds it at the door only when memory truly
//! can't fit it, so heavy work comes in as there's room for it rather than
//! all at once, and never waits long.
//!
//! Long because its cases, half of it, stay beside the door's private
//! pieces as every module in this crate keeps them.
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
//! [`Machine::grows`]. Otherwise, while what's available still leaves
//! [`Machine::floor`] once it has grown, it runs at once ranked behind
//! all other work: the lowest CPU and disk weight, and its memory capped
//! at what keeps that floor, so it slows rather than crowds what Joel
//! uses. Only below that, or through a call, is it frozen in its scope as
//! it starts, small, saying so on its standard error, and the oldest held
//! comes in first once there's room. Freezing every start short of the
//! room paused work Joel ruled must never pause ("we need to not pause
//! anything", 9 November): that night the door held 137 starts for 17
//! hours in all, a median of 3.7 minutes each and 15 the whole 30, among
//! them changed-checks' one-second evaluations of the machine's system,
//! while 16 to 21 GB stayed available.
//! Each let in counts as that growth, less what its scope already holds, for
//! [`RAMP`] after, so starts that come together are let in only as many
//! as there's room for, then one at a time as memory shows how much they
//! took. Both are shares of the machine's memory, so the door works the
//! same on a laptop, a desktop and a virtual machine's test. While a call
//! is on, more room is kept and nothing is ranked in. None is
//! held past [`LONGEST`] once no call is on, so one waits minutes at most
//! on memory others keep, and those that waited that long come in
//! [`SPACED`] apart.
//!
//! A start whose environment carries [`FIRST`] lands a switch, which
//! every merged fix waits on: it waits behind other switches alone, and
//! with no call on needs room only for its own growth. On 9 November a
//! switch's evaluation waited 30 minutes with 13 GB free, most of the rest
//! held by idle sessions that no waiting would free.
//!
//! Only a scope of its own is frozen, a `work-` scope the sorter split it
//! into, never a service's cgroup that holds other work too, and never a
//! scope already let in or whose run began over [`YOUNG`] before the
//! start, so a run is never held halfway through it: freezing a scope
//! freezes all of it, and on 9 November a stamp check's own timeout went
//! on counting through the freeze and killed it. A sorter started again
//! freezes nothing already running.
//!
//! A start can take a lock in the moment before it's held, so one held
//! while another process waits on a lock it holds comes in at once: on 9
//! November an evaluation held a flake input's fetch lock, frozen, and two
//! lanes' builds and a lint already let in waited on it for 16 minutes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What a start's environment carries when it lands a switch, which every
/// merged fix waits on: exported by whatever builds and switches the
/// machine, and inherited by the Nix work it starts.
pub const FIRST: &str = "JOBS_DOOR_FIRST=1";

/// How long a start is counted to be growing.
pub const RAMP: Duration = Duration::from_mins(2);

/// The longest a start is held while no call is on: minutes, since below
/// the floor most memory is held by idle sessions that no waiting frees.
pub const LONGEST: Duration = Duration::from_mins(5);

/// The time between two let in because they waited [`LONGEST`].
pub const SPACED: Duration = Duration::from_secs(30);

/// How soon after its start a process must be seen to be held, in clock
/// ticks.
pub const YOUNG: u64 = 500;

/// How long a clock tick is, in milliseconds.
const MS_PER_TICK: u64 = 10;

/// Whether a command line, its words, is Nix work that grows: an
/// evaluation, a `nix flake check`, a build, a shell or a run, and
/// `nix-offload there` told to run here.
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
        // `there` evaluates and builds on builders, never here unasked, so
        // it grows this machine only when told to run here.
        "nix-offload" => {
            verbs.next() == Some(&"there")
                && words[1..]
                    .iter()
                    .any(|word| matches!(*word, "--here" | "--here-when-idle"))
        }
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
    /// Whether it lands a switch, and so waits ahead of the rest.
    first: bool,
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
#[derive(Clone)]
pub struct Coming<'a> {
    pub pid: u32,
    /// When it started, in clock ticks since boot.
    pub start: u64,
    pub words: Vec<&'a str>,
    /// The cgroup it's in, a folder under /sys/fs/cgroup.
    pub scope: PathBuf,
    /// What its cgroup holds, in bytes.
    pub memory: u64,
    /// Whether it lands a switch, its environment carrying [`FIRST`].
    pub first: bool,
    /// Whether its cgroup is frozen.
    pub frozen: bool,
    /// Whether it holds a lock another process waits on.
    pub blocking: bool,
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

    /// Available memory a start ranked behind other work must leave once
    /// it has grown, and what its memory is capped to keep: a sixteenth of
    /// the machine's, 4 GiB of 64, so it takes the last of the room only
    /// when there's 8 GiB to give.
    pub fn floor(self) -> u64 {
        self.total / 16
    }
}

/// What the door does to a start's scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Freeze,
    Thaw,
    /// Ranks it behind all other work, its memory held under `high`
    /// bytes.
    Rank {
        high: u64,
    },
}

/// What the door did to a process.
#[derive(Debug, PartialEq, Eq)]
pub enum Done {
    Held(u32),
    LetIn(u32),
    /// Let in ranked behind other work, its memory held under the bytes
    /// given.
    Ranked(u32, u64),
}

impl Door {
    /// Looks at what's under the door `now`, `uptime` ticks since boot, on
    /// `machine`: lets each new young heavy start in a scope of its own in
    /// while there's room for it and nothing waits before it, ranks it in
    /// behind other work while there's its floor, holds it otherwise, and
    /// lets the oldest held in as room comes, or once it's waited
    /// [`LONGEST`] with no call on. `act` freezes, thaws or ranks a scope
    /// and says whether it could.
    pub fn look(
        &mut self,
        coming: &[Coming],
        machine: Machine,
        now: Instant,
        uptime: u64,
        mut act: impl FnMut(&Path, Act) -> bool,
    ) -> Vec<Done> {
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
        // Let in before the room is counted, so it counts as growing.
        let blocking = self.let_in_blocking(coming, now, &mut act);
        let mut done: Vec<Done> = blocking.into_iter().map(Done::LetIn).collect();
        let (need, grows) = (machine.room(), machine.grows());
        let growing: u64 = self
            .admitted
            .iter()
            .filter(|(_, at)| now.saturating_duration_since(**at) < RAMP)
            .map(|(scope, _)| grows.saturating_sub(memory[scope.as_path()]))
            .sum();
        let mut room = machine.free.saturating_sub(growing);
        // A switch lands every merged fix, so with no call on it needs
        // room only for its own growth: what the rest keep is mostly held
        // by sessions that no waiting frees.
        let fits = |room: u64, first: bool| {
            room >= grows + if first && !machine.calling { 0 } else { need }
        };
        // Short of the room, a start still runs at once, ranked behind the
        // rest, while there's its floor and no call is on; capped to keep
        // the floor, it slows rather than crowds what the person uses.
        let floor = machine.floor();
        let ranks = |room: u64| !machine.calling && room >= grows + floor;
        // When each scope's oldest process started, so a run already going
        // is known by it.
        let mut oldest: HashMap<&Path, u64> = HashMap::new();
        for it in coming {
            let start = oldest.entry(it.scope.as_path()).or_insert(it.start);
            *start = (*start).min(it.start);
        }
        self.take_over(coming, now, uptime);
        // Switches, then the oldest held, come in while there's room, or
        // ranked while there's their floor.
        while let Some(held) = self.held.first() {
            let plain = fits(room, held.first);
            if !plain && !ranks(room) {
                break;
            }
            let ranked = (!plain).then(|| room - floor);
            if let Some(came) = self.come_in(ranked, now, &mut act) {
                done.push(came);
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
            // A scope whose run began well before this start is that run's
            // later step, and freezing the scope would freeze the whole run,
            // its own clocks counting on.
            let midway = oldest
                .get(it.scope.as_path())
                .is_some_and(|began| began.saturating_add(YOUNG) < it.start);
            let switches_ahead = self.held.iter().filter(|held| held.first).count();
            let ahead = if it.first {
                switches_ahead > 0
            } else {
                !self.held.is_empty()
            };
            if midway || (fits(room, it.first) && !ahead) {
                self.admitted.insert(it.scope.clone(), now);
                room = room.saturating_sub(grows);
            } else if !ahead && ranks(room) {
                let high = room - floor;
                if act(&it.scope, Act::Rank { high }) {
                    done.push(Done::Ranked(it.pid, high));
                }
                self.admitted.insert(it.scope.clone(), now);
                room = room.saturating_sub(grows);
            } else if act(&it.scope, Act::Freeze) {
                let held = Held {
                    pid: it.pid,
                    scope: it.scope.clone(),
                    since: now,
                    first: it.first,
                };
                // A switch waits behind switches alone.
                let place = if it.first {
                    switches_ahead
                } else {
                    self.held.len()
                };
                self.held.insert(place, held);
                done.push(Done::Held(it.pid));
            }
        }
        done.extend(self.let_in_longest(machine, now, &mut act));
        done
    }

    /// Lets in the first held once it has waited [`LONGEST`], whatever the
    /// room, one at a time, but never into a call.
    fn let_in_longest(
        &mut self,
        machine: Machine,
        now: Instant,
        act: &mut impl FnMut(&Path, Act) -> bool,
    ) -> Option<Done> {
        let spaced = self
            .waited_in
            .is_none_or(|at| now.saturating_duration_since(at) >= SPACED);
        let waited = self
            .held
            .first()
            .is_some_and(|held| now.saturating_duration_since(held.since) >= LONGEST);
        if machine.calling || !spaced || !waited {
            return None;
        }
        // Short of its floor still, it comes in ranked and capped at its own
        // growth, so what it takes past that it reclaims from itself.
        let came = self.come_in(Some(machine.grows()), now, act)?;
        self.waited_in = Some(now);
        Some(came)
    }

    /// Lets the first held in, ranked under `ranked` bytes when given, and
    /// says how; none when it couldn't be thawed.
    fn come_in(
        &mut self,
        ranked: Option<u64>,
        now: Instant,
        act: &mut impl FnMut(&Path, Act) -> bool,
    ) -> Option<Done> {
        let first = self.held.remove(0);
        // Ranked before it thaws, so it never runs a moment unranked.
        let ranked = ranked.filter(|high| act(&first.scope, Act::Rank { high: *high }));
        if !act(&first.scope, Act::Thaw) {
            return None;
        }
        self.admitted.insert(first.scope, now);
        Some(ranked.map_or(Done::LetIn(first.pid), |high| Done::Ranked(first.pid, high)))
    }

    /// Lets in each held scope holding a lock others wait on, whatever the
    /// room and through a call: held, it holds them too, work already let
    /// in, and what they'd wait for is only its own little start. Says the
    /// pid of each let in.
    fn let_in_blocking(
        &mut self,
        coming: &[Coming],
        now: Instant,
        act: &mut impl FnMut(&Path, Act) -> bool,
    ) -> Vec<u32> {
        let blocking: HashSet<&Path> = coming
            .iter()
            .filter(|it| it.blocking)
            .map(|it| it.scope.as_path())
            .collect();
        let mut let_in = Vec::new();
        let mut kept = Vec::new();
        for held in std::mem::take(&mut self.held) {
            if blocking.contains(held.scope.as_path()) && act(&held.scope, Act::Thaw) {
                let_in.push(held.pid);
                self.admitted.insert(held.scope, now);
            } else {
                kept.push(held);
            }
        }
        self.held = kept;
        let_in
    }

    /// Holds again each scope frozen with heavy work in it that the door
    /// doesn't hold: a sorter that stopped held it, or a thaw failed, and no
    /// sorter would ever let it in. Its wait counts from its start, and the
    /// held keep their order. On 9 November a restarted sorter left ten such
    /// scopes frozen for over half an hour.
    fn take_over(&mut self, coming: &[Coming], now: Instant, uptime: u64) {
        for it in coming {
            let left = it.frozen
                && heavy(&it.words)
                && own_scope(&it.scope)
                && !self.admitted.contains_key(&it.scope)
                && !self.held.iter().any(|held| held.scope == it.scope);
            if left {
                let waited = Duration::from_millis(
                    uptime.saturating_sub(it.start).saturating_mul(MS_PER_TICK),
                );
                self.seen.insert((it.pid, it.start));
                self.held.push(Held {
                    pid: it.pid,
                    scope: it.scope.clone(),
                    since: now.checked_sub(waited).unwrap_or(now),
                    first: it.first,
                });
            }
        }
        self.held.sort_by_key(|held| (!held.first, held.since));
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

/// The processes holding a lock another waits on, from /proc/locks' text.
/// A waiter's line follows its holder's under the same number, with `->`
/// before its kind. Only waited-on locks count: nearly every Nix process
/// holds a read lock on a database's shared memory for as long as it runs.
pub fn blocking(locks: &str) -> HashSet<u32> {
    let mut holders: HashMap<&str, u32> = HashMap::new();
    let mut waited: HashSet<&str> = HashSet::new();
    for line in locks.lines() {
        let mut words = line.split_whitespace();
        let Some(number) = words.next() else {
            continue;
        };
        let mut rest = words.peekable();
        if rest.peek() == Some(&"->") {
            waited.insert(number);
        } else if let Some(pid) = rest.nth(3).and_then(|pid| pid.parse().ok()) {
            holders.insert(number, pid);
        }
    }
    waited
        .iter()
        .filter_map(|number| holders.get(number).copied())
        .collect()
}

/// Freezes, thaws or ranks the cgroup at `scope`, saying whether it could.
/// Ranking goes through the user's systemd, which owns the scope and
/// turns on the parent's CPU and disk controllers for it; written to the
/// cgroup straight, the weights have no file to go in.
pub fn act(scope: &Path, act: Act) -> bool {
    match act {
        Act::Freeze => fs::write(scope.join("cgroup.freeze"), "1").is_ok(),
        Act::Thaw => fs::write(scope.join("cgroup.freeze"), "0").is_ok(),
        Act::Rank { high } => scope
            .file_name()
            .and_then(|unit| unit.to_str())
            .is_some_and(|unit| {
                std::process::Command::new("busctl")
                    .args(rank_call(unit, high))
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|status| status.success())
            }),
    }
}

/// What busctl is given to rank the scope `unit` behind all other work:
/// the lowest CPU and disk weight, and its memory held under `high`
/// bytes, as runtime properties that go with the scope.
pub fn rank_call(unit: &str, high: u64) -> Vec<String> {
    let high = high.to_string();
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "3",
        "CPUWeight",
        "t",
        "1",
        "IOWeight",
        "t",
        "1",
        "MemoryHigh",
        "t",
        &high,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// What the cgroup at `scope` holds, in bytes; none when it can't say.
pub fn memory(scope: &Path) -> u64 {
    fs::read_to_string(scope.join("memory.current"))
        .ok()
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(0)
}

/// Whether a process's environment, as /proc/<pid>/environ holds it, carries
/// [`FIRST`].
pub fn marked_first(environ: &[u8]) -> bool {
    environ
        .split(|byte| *byte == 0)
        .any(|pair| pair == FIRST.as_bytes())
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
            "jobs-sorter: {why} {} GB of memory free, less what the starts just let in will grow \
             to, so this waits for room or its turn{}",
            machine.free / 1_000_000_000,
            if machine.calling {
                ", until the call ends".to_owned()
            } else {
                format!(", at most {} minutes", LONGEST.as_secs() / 60)
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_switch_is_known_by_its_environment() {
        assert!(
            marked_first(b"HOME=/home/a\0JOBS_DOOR_FIRST=1\0PATH=/bin\0"),
            "the mark among others is a switch's"
        );
        assert!(
            !marked_first(b"HOME=/home/a\0JOBS_DOOR_FIRST=0\0"),
            "a mark set to 0 is no switch's"
        );
        assert!(
            !marked_first(b"NOT_JOBS_DOOR_FIRST=1\0"),
            "a longer name holding the mark is no switch's"
        );
    }

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
            "/nix/store/y-nix-offload/bin/nix-offload there --here -L . checks.x",
            "nix-offload there --here-when-idle . checks.x",
        ] {
            assert!(heavy(&words.split(' ').collect::<Vec<_>>()), "{words}");
        }
        for words in [
            "nix flake show",
            "nix path-info .#x",
            "nix-store -q",
            "nix-offload progress host",
            // Evaluated and built on builders, never here unasked: on 9
            // November such runs held 30 to 370 MB here at most.
            "/nix/store/y-nix-offload/bin/nix-offload there -L . checks.x",
            "nix-offload there --changed",
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
        assert_eq!(
            (small.room(), small.grows(), small.floor()),
            (512 << 20, 128 << 20, 128 << 20)
        );
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
    const FLOOR: u64 = 4 << 30;

    fn coming(pid: u32, start: u64, words: &'static str, scope: &str) -> Coming<'static> {
        Coming {
            pid,
            start,
            words: words.split(' ').collect(),
            scope: PathBuf::from(scope),
            memory: 0,
            first: false,
            frozen: false,
            blocking: false,
        }
    }

    /// A start already frozen when the door first sees it.
    fn frozen(pid: u32, start: u64, words: &'static str, scope: &str) -> Coming<'static> {
        Coming {
            frozen: true,
            ..coming(pid, start, words, scope)
        }
    }

    #[test]
    fn a_hold_left_by_a_sorter_that_stopped_is_taken_over() {
        let mut door = Door::default();
        let at = Instant::now();
        // Frozen a minute before this door started, with room short.
        let left = [frozen(60, 1000, "nix eval .#x", "/c/work-60-1.scope")];
        let since = 1000 + 60 * 100;
        assert_eq!(door.look(&left, free(1 << 30), at, since, |_, _| true), []);
        assert_eq!(
            door.look(&left, free(1 << 30), at + LONGEST / 2, since + 1, |_, _| {
                true
            }),
            [],
            "it waits on until its wait, counted from its start, passes the longest"
        );
        assert_eq!(
            door.look(&left, free(1 << 30), at + LONGEST, since + 2, |_, _| {
                true
            }),
            [Done::Ranked(60, GROWS)]
        );
    }

    #[test]
    fn a_left_hold_comes_in_as_soon_as_there_is_room() {
        let mut door = Door::default();
        let at = Instant::now();
        let left = [frozen(60, 1000, "nix eval .#x", "/c/work-60-1.scope")];
        assert_eq!(
            door.look(&left, free(1 << 40), at, 2000, |_, _| true),
            [Done::LetIn(60)]
        );
    }

    #[test]
    fn a_scope_frozen_with_no_heavy_start_in_it_is_left_alone() {
        let mut door = Door::default();
        let at = Instant::now();
        let other = [frozen(60, 1000, "sleep 9", "/c/work-60-1.scope")];
        assert_eq!(door.look(&other, free(1 << 40), at, 2000, |_, _| true), []);
    }

    /// A start that lands a switch, as its environment says.
    fn first(pid: u32, start: u64, words: &'static str, scope: &str) -> Coming<'static> {
        Coming {
            first: true,
            ..coming(pid, start, words, scope)
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
        let mut frozen: Vec<(PathBuf, Act)> = Vec::new();
        let mut freezing = |scope: &Path, act: Act| {
            frozen.push((scope.to_path_buf(), act));
            true
        };
        let at = Instant::now();
        let short = free(GROWS + FLOOR - 1);
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
                (PathBuf::from("/c/work-10-1.scope"), Act::Freeze),
                (PathBuf::from("/c/work-10-1.scope"), Act::Thaw)
            ],
            "held, then let in with room for it, unranked"
        );
    }

    // On 9 November changed-checks' one-second evaluation of the machine's
    // system waited the whole 30 minutes at the door, with 16 GB free.
    #[test]
    fn a_start_short_of_room_runs_at_once_ranked_behind_other_work() {
        let mut door = Door::default();
        let mut acted: Vec<Act> = Vec::new();
        let at = Instant::now();
        let short = free(10 << 30);
        let work = [coming(10, 1000, "nix eval .#x", "/c/work-10-1.scope")];
        let done = door.look(&work, short, at, 1000, |_, act| {
            acted.push(act);
            true
        });
        assert_eq!(done, [Done::Ranked(10, (10 << 30) - FLOOR)]);
        assert_eq!(
            acted,
            [Act::Rank {
                high: (10 << 30) - FLOOR
            }],
            "never frozen, and capped to keep the floor"
        );
    }

    #[test]
    fn a_held_start_comes_in_ranked_once_there_is_its_floor() {
        let mut door = Door::default();
        let mut acted: Vec<Act> = Vec::new();
        let at = Instant::now();
        let work = [coming(10, 1000, "nix eval .#x", "/c/work-10-1.scope")];
        let mut acting = |_: &Path, act: Act| {
            acted.push(act);
            true
        };
        let done = door.look(&work, free(1 << 30), at, 1000, &mut acting);
        assert_eq!(done, [Done::Held(10)]);
        let done = door.look(&work, free(GROWS + FLOOR), at, 1001, &mut acting);
        assert_eq!(done, [Done::Ranked(10, GROWS)]);
        assert_eq!(
            acted,
            [Act::Freeze, Act::Rank { high: GROWS }, Act::Thaw],
            "ranked before it thaws"
        );
    }

    #[test]
    fn ranking_asks_systemd_for_the_lowest_weights_and_a_memory_cap() {
        let call = rank_call("work-10-1.scope", 6 << 30);
        let at = call.iter().position(|arg| arg == "work-10-1.scope");
        assert_eq!(
            at.map(|at| &call[at..]),
            Some(
                &[
                    "work-10-1.scope",
                    "true",
                    "3",
                    "CPUWeight",
                    "t",
                    "1",
                    "IOWeight",
                    "t",
                    "1",
                    "MemoryHigh",
                    "t",
                    "6442450944"
                ]
                .map(str::to_owned)[..]
            ),
            "{call:?}"
        );
    }

    // On 9 November an evaluation took a flake input's fetch lock in the
    // moment before it was held, and two lanes' builds and a lint waited
    // on that lock for 16 minutes, already let in and going nowhere.
    #[test]
    fn a_held_start_others_wait_on_comes_in_whatever_the_room() {
        let mut door = Door::default();
        let at = Instant::now();
        let mut work = [
            coming(10, 1000, "nix eval .#x", "/c/work-10-1.scope"),
            coming(11, 1000, "nix eval .#y", "/c/work-11-1.scope"),
        ];
        let short = free(1 << 30);
        let done = door.look(&work, short, at, 1000, |_, _| true);
        assert_eq!(done, [Done::Held(10), Done::Held(11)]);
        work[1].blocking = true;
        let calling = Machine {
            calling: true,
            ..short
        };
        let done = door.look(&work, calling, at, 1001, |_, _| true);
        assert_eq!(done, [Done::LetIn(11)], "only the one others wait on");
        assert_eq!(
            door.look(&work, calling, at, 1002, |_, _| true),
            [],
            "let in once"
        );
    }

    #[test]
    fn a_lock_others_wait_on_is_known_by_its_holder() {
        let locks = "\
1: POSIX  ADVISORY  READ 1353274 00:22:102258 1073741826 1073742335
2: FLOCK  ADVISORY  WRITE 964893 00:22:84096289 0 EOF
2: -> FLOCK  ADVISORY  WRITE 1200 00:22:84096289 0 EOF
2: -> FLOCK  ADVISORY  WRITE 1201 00:22:84096289 0 EOF
3: OFDLCK ADVISORY  WRITE -1 00:22:9 0 EOF
3: -> OFDLCK ADVISORY  WRITE -1 00:22:9 0 EOF
4: POSIX  ADVISORY  READ 1257727 00:22:83907755 128 128
";
        assert_eq!(blocking(locks), HashSet::from([964_893]));
        assert_eq!(blocking(""), HashSet::new());
    }

    // On 9 November about 13 sessions' builds started within a minute of
    // a hold ending, and a call's microphone went unread for 59 s.
    #[test]
    fn thirteen_starts_at_once_come_in_only_as_many_as_there_is_room_for() {
        let mut door = Door::default();
        let at = Instant::now();
        let mut work = thirteen();
        let done = door.look(&work, free(26_000_000_000), at, 1000, |_, _| true);
        let ranked = done
            .iter()
            .filter(|done| matches!(done, Done::Ranked(..)))
            .count();
        assert_eq!(
            (held(&done), ranked),
            (8, 3),
            "two fit in 26 GB, three more ranked behind them: {done:?}"
        );
        // Grown to what they were counted at, memory shows it: none more.
        for it in &mut work[..5] {
            it.memory = GROWS;
        }
        let later = at + Duration::from_secs(10);
        let done = door.look(&work, free(8_000_000_000), later, 1001, |_, _| true);
        assert!(done.is_empty(), "{done:?}");
        // One ends and gives its memory back: one more comes in, the oldest.
        work.remove(0);
        let done = door.look(&work, free(12_000_000_000), later, 1002, |_, _| true);
        assert_eq!(done, [Done::Ranked(105, 12_000_000_000 - FLOOR)]);
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
        assert_eq!(
            done,
            [Done::Ranked(102, ROOM - FLOOR)],
            "the first two still count, so it's ranked behind them"
        );
        let fourth = &work[..4];
        let done = door.look(fourth, plenty, at + RAMP, 1003, |_, _| true);
        assert!(done.is_empty(), "their ramp has passed: {done:?}");
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
            [Done::Ranked(100, GROWS)],
            "short of memory still, but it has waited long enough"
        );
        assert!(
            door.look(work, short, at + LONGEST, 1002, |_, _| true)
                .is_empty(),
            "the next waits its spacing"
        );
        assert_eq!(
            door.look(work, short, at + LONGEST + SPACED, 1003, |_, _| true),
            [Done::Ranked(101, GROWS)]
        );
    }

    #[test]
    fn a_runs_own_later_starts_are_never_held() {
        let mut door = Door::default();
        let at = Instant::now();
        let first = [coming(50, 1000, "nix build .#x", "/c/work-50-1.scope")];
        assert_eq!(door.look(&first, free(1 << 40), at, 1000, |_, _| true), []);
        let both = [
            coming(50, 1000, "nix build .#x", "/c/work-50-1.scope"),
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

    // On 9 November a switch's evaluation, carrying three merged fixes,
    // waited 30 minutes at the door with 13 GB free, most of the rest held
    // by idle sessions that no waiting would free.
    #[test]
    fn a_switch_needs_only_room_to_grow_and_comes_in_before_those_held_ahead() {
        let mut door = Door::default();
        let at = Instant::now();
        let short = free(GROWS + (1 << 30));
        let agents = [coming(40, 1000, "nix build .#a", "/c/work-40-1.scope")];
        assert_eq!(
            door.look(&agents, short, at, 1000, |_, _| true),
            [Done::Held(40)]
        );
        let switch = [
            agents[0].clone(),
            first(41, 1001, "nix eval .#unit", "/c/work-41-1.scope"),
        ];
        assert!(
            door.look(&switch, short, at, 1001, |_, _| true).is_empty(),
            "a switch's start comes in with room only for its own growth"
        );
        assert_eq!(door.held.len(), 1, "the agent's start still waits");
    }

    #[test]
    fn a_held_switch_comes_in_first_once_its_growth_fits() {
        let mut door = Door::default();
        let at = Instant::now();
        let none = free(1 << 30);
        let work = [
            coming(50, 1000, "nix build .#a", "/c/work-50-1.scope"),
            first(51, 1000, "nix eval .#unit", "/c/work-51-1.scope"),
        ];
        assert_eq!(held(&door.look(&work, none, at, 1000, |_, _| true)), 2);
        assert_eq!(
            door.look(&work, free(GROWS + (1 << 30)), at, 1001, |_, _| true),
            [Done::LetIn(51)],
            "the switch goes first, though the agent's start was held before it"
        );
    }

    // On 9 November a stamp check ran for minutes before its first Nix
    // start, and freezing its scope then froze the whole run, whose own
    // timeout went on counting and killed it.
    #[test]
    fn a_run_already_going_is_never_frozen_for_its_first_heavy_start() {
        let mut door = Door::default();
        let at = Instant::now();
        let running = [
            coming(70, 1000, "node tools/check.mjs", "/c/work-70-1.scope"),
            coming(
                71,
                1000 + YOUNG + 1,
                "nix build .#stamp",
                "/c/work-70-1.scope",
            ),
        ];
        assert!(
            door.look(&running, free(1 << 30), at, 1000 + YOUNG + 2, |_, _| true)
                .is_empty(),
            "a run's later start goes on, however short memory is"
        );
        assert!(door.held.is_empty(), "nothing is held");
    }

    #[test]
    fn a_call_holds_a_switch_as_it_holds_the_rest() {
        let mut door = Door::default();
        let at = Instant::now();
        let calling = Machine {
            calling: true,
            ..free(ROOM)
        };
        let switch = [first(60, 1000, "nix eval .#unit", "/c/work-60-1.scope")];
        assert_eq!(
            door.look(&switch, calling, at, 1000, |_, _| true),
            [Done::Held(60)],
            "a call keeps its room from a switch too"
        );
    }
}
