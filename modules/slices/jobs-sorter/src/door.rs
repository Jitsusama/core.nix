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
//! Room alone let too much in together once a wait ended: at 10:40 on 24
//! November a call's hold ended, the work held for it started at once,
//! and within fourteen minutes the machine was at a load of 99 with
//! memory full 10.7% over a minute, through Joel's next call. So for
//! [`PACED`] after a call ends, or after [`QUIET`] with no new heavy start,
//! starts come in [`PER_MINUTE`] a minute, oldest first, and none while
//! memory stalls past [`STALLED`]; the rest wait their turn, frozen as
//! though short of room, and are told so. The pace lasts past [`PACED`]
//! until what it holds has come in, so its backlog isn't let in together
//! as it ends. A switch never waits its turn, and the turn is no more
//! than a wait for room: those that have waited [`LONGEST`] come in,
//! taking none of the pace's turns, all at once while there's room for
//! each and memory doesn't stall, and otherwise [`SPACED`] apart, so a
//! stream faster than the pace never makes waits grow while memory
//! allows. One held through a call counts that wait from the call's end,
//! so a long call's backlog comes in at the pace rather than all at once
//! as the call ends.
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

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
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

/// How long starts come in paced after a call ends, or after a quiet
/// stretch.
pub const PACED: Duration = Duration::from_mins(10);

/// How long the door must see no new heavy start for the next to begin a
/// paced stretch.
pub const QUIET: Duration = Duration::from_mins(10);

/// How many heavy starts come in each minute while paced.
pub const PER_MINUTE: usize = 2;

/// The memory full pressure over a minute, in percent, past which no
/// paced start comes in.
pub const STALLED: f64 = 5.0;

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
        // A World tree's evaluator, whose options take values, so its verb
        // is found among the words rather than first: on 24 November ten
        // map chains ran it beside the World walks.
        "tec" => verbs.any(|word| matches!(*word, "eval" | "instantiate" | "build")),
        "nix" => match verbs.next() {
            Some(&("eval" | "build" | "develop" | "shell" | "run")) => true,
            Some(&"flake") => verbs.next() == Some(&"check"),
            _ => false,
        },
        _ => false,
    }
}

/// The share of the last minute every task waited on memory, in percent,
/// from /proc/pressure/memory's text.
pub fn stalled(pressure: &str) -> Option<f64> {
    let full = pressure
        .lines()
        .find_map(|line| line.strip_prefix("full "))?;
    full.split_whitespace()
        .find_map(|word| word.strip_prefix("avg60="))?
        .parse()
        .ok()
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
    /// Whether a call was on at the last look.
    calling: bool,
    /// When the last call ended, from which a start it held counts its
    /// wait: a call holds what starts through it however long it lasts.
    call_ended: Option<Instant>,
    /// When the last new heavy start was seen.
    last_start: Option<Instant>,
    /// When starts began to come in paced.
    paced_from: Option<Instant>,
    /// When each start came in while paced, over the last minute.
    turns: VecDeque<Instant>,
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
    /// The share of the last minute every task waited on memory, in
    /// percent.
    pub stalled: f64,
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
    /// Held for its turn, as starts come in paced.
    Paced(u32),
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
        self.follow_call(machine, now);
        self.settle_pace(now);
        // Let in before the room is counted, so it counts as growing.
        let blocking = self.let_in_blocking(coming, now, &mut act);
        let mut done: Vec<Done> = blocking.into_iter().map(Done::LetIn).collect();
        let (need, grows) = (machine.room(), machine.grows());
        let mut room = machine
            .free
            .saturating_sub(self.growing(&memory, grows, now));
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
        let oldest = oldest(coming);
        self.take_over(coming, now, uptime);
        // Switches, then the oldest held, come in while there's room, or
        // ranked while there's their floor.
        while let Some(held) = self.held.first() {
            let (plain, switch) = (fits(room, held.first), held.first);
            if !plain && !ranks(room) || !switch && !self.turn(machine, now) {
                break;
            }
            let ranked = (!plain).then(|| room - floor);
            if let Some(came) = self.come_in(ranked, now, &mut act) {
                done.push(came);
                room -= grows;
                self.took_turn(now, !switch);
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
            // A later step comes with its run, so neither ends a quiet
            // stretch nor begins a pace.
            if !midway {
                self.follow_start(now);
            }
            let switches_ahead = self.held.iter().filter(|held| held.first).count();
            let ahead = if it.first {
                switches_ahead > 0
            } else {
                !self.held.is_empty()
            };
            // A switch never waits its turn, and a run's later step was let
            // in with the run.
            let turn = midway || it.first || self.turn(machine, now);
            // Held while paced with room for it, it waits only its turn.
            let would = self.pacing(now) && (fits(room, it.first) || ranks(room));
            if midway || (fits(room, it.first) && !ahead && turn) {
                self.admitted.insert(it.scope.clone(), now);
                room = room.saturating_sub(grows);
                self.took_turn(now, !midway && !it.first);
            } else if !ahead && ranks(room) && turn {
                let high = room - floor;
                if act(&it.scope, Act::Rank { high }) {
                    done.push(Done::Ranked(it.pid, high));
                }
                self.admitted.insert(it.scope.clone(), now);
                room = room.saturating_sub(grows);
                self.took_turn(now, !it.first);
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
                done.push(if would {
                    Done::Paced(it.pid)
                } else {
                    Done::Held(it.pid)
                });
            }
        }
        done.extend(self.let_in_longest(machine, room, now, &mut act));
        done
    }

    /// Lets in each held that has waited [`LONGEST`], oldest first, but
    /// never into a call: all of them while `room` fits each and memory
    /// doesn't stall, and otherwise one each [`SPACED`], whatever the room.
    fn let_in_longest(
        &mut self,
        machine: Machine,
        mut room: u64,
        now: Instant,
        act: &mut impl FnMut(&Path, Act) -> bool,
    ) -> Vec<Done> {
        let mut done = Vec::new();
        if machine.calling {
            return done;
        }
        let (need, grows) = (machine.room(), machine.grows());
        // Counted from the call's end for one held through it, or the
        // whole backlog of a long call would have waited LONGEST as it
        // ended, and come in past the pace and the stall alike.
        while let Some(held) = self.held.first() {
            let from = self
                .call_ended
                .map_or(held.since, |ended| ended.max(held.since));
            if now.saturating_duration_since(from) < LONGEST {
                break;
            }
            // Its turn isn't asked, nor does it take one: a start waits
            // LONGEST at most, paced or not, so with room for it and no
            // stall each comes in, or a stream faster than any spacing would
            // make waits grow. Short of room or in a stall, main's one each
            // SPACED, ranked and capped at its own growth, so what it takes
            // past that it reclaims from itself.
            let plain = room >= grows + need && machine.stalled <= STALLED;
            let spaced = self
                .waited_in
                .is_none_or(|at| now.saturating_duration_since(at) >= SPACED);
            if !plain && !spaced {
                break;
            }
            let Some(came) = self.come_in((!plain).then_some(grows), now, act) else {
                break;
            };
            done.push(came);
            room = room.saturating_sub(grows);
            if !plain {
                self.waited_in = Some(now);
                break;
            }
        }
        done
    }

    /// What the starts let in within [`RAMP`] of `now` are yet to grow to,
    /// each counted to grow by `grows`, less what its scope holds by
    /// `memory`.
    fn growing(&self, memory: &HashMap<&Path, u64>, grows: u64, now: Instant) -> u64 {
        self.admitted
            .iter()
            .filter(|(_, at)| now.saturating_duration_since(**at) < RAMP)
            .map(|(scope, _)| grows.saturating_sub(memory[scope.as_path()]))
            .sum()
    }

    /// Begins a paced stretch `now` as a call ends on `machine`, since
    /// what it held comes in together.
    fn follow_call(&mut self, machine: Machine, now: Instant) {
        if self.calling && !machine.calling {
            self.paced_from = Some(now);
            self.call_ended = Some(now);
        }
        self.calling = machine.calling;
    }

    /// Notes a new heavy start seen `now`, beginning a paced stretch when
    /// it ends a quiet one: starts that come together after a quiet
    /// stretch came together after a hold, as at 10:40 on 24 November.
    fn follow_start(&mut self, now: Instant) {
        let quiet = self
            .last_start
            .is_some_and(|at| now.saturating_duration_since(at) >= QUIET);
        if quiet && !self.pacing(now) {
            self.paced_from = Some(now);
        }
        self.last_start = Some(now);
    }

    /// Whether starts come in paced `now`: for [`PACED`] after a call ends
    /// or a quiet stretch, and past it while a start other than a switch
    /// is still held.
    fn pacing(&self, now: Instant) -> bool {
        self.paced_from.is_some_and(|from| {
            now.saturating_duration_since(from) < PACED || self.held.iter().any(|it| !it.first)
        })
    }

    /// Ends a paced stretch `now` once [`PACED`] has passed and nothing it
    /// held still waits, so a later hold for room alone isn't paced.
    fn settle_pace(&mut self, now: Instant) {
        if !self.pacing(now) {
            self.paced_from = None;
        }
    }

    /// Whether a start other than a switch may come in `now` on `machine`:
    /// always unless paced, and while paced only [`PER_MINUTE`] a minute
    /// and while memory stalls no more than [`STALLED`].
    fn turn(&mut self, machine: Machine, now: Instant) -> bool {
        if !self.pacing(now) {
            return true;
        }
        self.turns
            .retain(|at| now.saturating_duration_since(*at) < Duration::from_mins(1));
        self.turns.len() < PER_MINUTE && machine.stalled <= STALLED
    }

    /// Counts a start come in `now` against the pace, while paced, when it
    /// `counts`: a switch and a run's later step don't.
    fn took_turn(&mut self, now: Instant, counts: bool) {
        if counts && self.pacing(now) {
            self.turns.push_back(now);
        }
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

/// When each scope's oldest process started, so a run already going is
/// known by it.
fn oldest<'a>(coming: &'a [Coming]) -> HashMap<&'a Path, u64> {
    let mut oldest: HashMap<&Path, u64> = HashMap::new();
    for it in coming {
        let start = oldest.entry(it.scope.as_path()).or_insert(it.start);
        *start = (*start).min(it.start);
    }
    oldest
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

/// Opens the standard error of the process `pid` to tell it something:
/// without waiting, so a full pipe or a slow terminal never holds the
/// sorter's one loop, which a paced stretch tells often, and never as the
/// sorter's own terminal. A process with nowhere to say it hears nothing.
fn stderr_of(pid: u32) -> Option<fs::File> {
    fs::OpenOptions::new()
        .append(true)
        .custom_flags(crate::joins::O_NONBLOCK | crate::joins::O_NOCTTY)
        .open(format!("/proc/{pid}/fd/2"))
        .ok()
}

/// Tells the process `pid`, on its standard error, that it waits at the
/// door on `machine`.
pub fn tell(pid: u32, machine: Machine) {
    if let Some(mut said) = stderr_of(pid) {
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

/// Tells the process `pid`, on its standard error, that it waits its turn
/// at the door while starts come in paced.
pub fn tell_paced(pid: u32) {
    if let Some(mut said) = stderr_of(pid) {
        let _ = writeln!(
            said,
            "jobs-sorter: heavy starts come in {PER_MINUTE} a minute for {} minutes after a call \
             or a quiet stretch, and none while memory stalls past {STALLED}%, so this waits its \
             turn, at most {} minutes while memory has room for it",
            PACED.as_secs() / 60,
            LONGEST.as_secs() / 60
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
            "tec --git-dir /w/.git --ref main instantiate //areas/x:y",
            "/nix/store/t-tec/bin/tec eval --json //areas/x",
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
            "tec --git-dir /w/.git status",
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
            stalled: 0.0,
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
            stalled: 0.0,
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

    fn let_in(done: &[Done]) -> usize {
        done.iter()
            .filter(|done| matches!(done, Done::LetIn(_) | Done::Ranked(..)))
            .count()
    }

    const MINUTE: Duration = Duration::from_mins(1);

    // At 10:40 on 24 November a call's hold ended and the work held for it
    // started together: within fourteen minutes the machine was at a load of
    // 99, memory full 10.7% over a minute, through Joel's next call.
    #[test]
    fn a_call_ending_lets_what_it_held_in_two_a_minute() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..8];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(work, calling, at, 1000, |_, _| true)), 8);
        let ended = free(1 << 40);
        let came = door.look(work, ended, at + MINUTE, 1001, |_, _| true);
        assert_eq!(
            came,
            [Done::LetIn(100), Done::LetIn(101)],
            "the two held longest come in first, and only two"
        );
        assert!(
            door.look(work, ended, at + MINUTE * 3 / 2, 1002, |_, _| true)
                .is_empty(),
            "two came in this last minute"
        );
        assert_eq!(
            door.look(work, ended, at + MINUTE * 2, 1003, |_, _| true),
            [Done::LetIn(102), Done::LetIn(103)]
        );
        let last = at + MINUTE + PACED;
        assert_eq!(
            let_in(&door.look(work, ended, last, 1004, |_, _| true)),
            PER_MINUTE + 2,
            "the two the pace allows, and the two that have waited LONGEST since the call \
             ended, with room for them"
        );
        assert!(!door.pacing(last), "with nothing held the pace is over");
    }

    /// Looks every ten seconds for `minutes` from `from`, at what `work`
    /// holds, adding one new start to it each `every` looks, and says how
    /// many came in each minute and the longest any waited, those still
    /// held counted to the last look, and those held before it began
    /// counted from `from`.
    fn looked_on(
        door: &mut Door,
        work: &mut Vec<Coming<'static>>,
        from: Instant,
        minutes: u32,
        every: Option<u32>,
    ) -> (Vec<usize>, Duration) {
        let mut held_at: HashMap<u32, Instant> = HashMap::new();
        let (mut each, mut longest) = (vec![0; minutes as usize], Duration::ZERO);
        let mut now = from;
        for look in 0..minutes * 6 {
            now = from + Duration::from_secs(10 * u64::from(look));
            let uptime = 2000 + 1000 * u64::from(look);
            if every.is_some_and(|every| look % every == 0) {
                let pid = 5000 + look;
                let scope: &'static str = Box::leak(format!("/c/work-{pid}-1.scope").into());
                work.push(coming(pid, uptime, "nix build .#x", scope));
            }
            for done in door.look(work, free(1 << 40), now, uptime, |_, _| true) {
                match done {
                    Done::Held(pid) | Done::Paced(pid) => {
                        held_at.insert(pid, now);
                    }
                    Done::LetIn(pid) | Done::Ranked(pid, _) => {
                        each[(look / 6) as usize] += 1;
                        let since = held_at.remove(&pid).unwrap_or(from);
                        longest = longest.max(now - since);
                    }
                }
            }
        }
        let still = door
            .held
            .iter()
            .map(|held| now - held_at.get(&held.pid).copied().unwrap_or(from))
            .max();
        (each, longest.max(still.unwrap_or_default()))
    }

    // Joel, 9 November: "we need to not pause anything". Starts coming
    // faster than the pace for good would otherwise wait longer and longer.
    #[test]
    fn a_steady_stream_faster_than_the_pace_still_waits_longest_at_most() {
        let mut door = Door::default();
        let at = Instant::now();
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(door.look(&[], calling, at, 1000, |_, _| true), []);
        // Six a minute for half an hour, from the call's end: past the pace
        // and one each SPACED together.
        let (_, longest) = looked_on(&mut door, &mut Vec::new(), at + SPACED, 30, Some(1));
        assert!(
            longest <= LONGEST,
            "a start waited {longest:?}, past LONGEST, behind a stream the pace can't keep up with"
        );
    }

    // A call holds what starts through it however long it lasts, so its
    // backlog has all waited LONGEST as it ends: counted from the start,
    // it would all come in together, as at 10:40 on 24 November.
    #[test]
    fn a_long_calls_backlog_comes_in_at_the_pace_then_once_it_has_waited_longest() {
        let mut door = Door::default();
        let at = Instant::now();
        let mut work: Vec<Coming<'static>> = (100..120)
            .map(|pid| {
                let scope: &'static str = Box::leak(format!("/c/work-{pid}-1.scope").into());
                coming(pid, 1000, "nix build .#x", scope)
            })
            .collect();
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(&work, calling, at, 1000, |_, _| true)), 20);
        let end = at + LONGEST * 4;
        let before = at + (LONGEST * 4).saturating_sub(SPACED);
        assert_eq!(door.look(&work, calling, before, 1001, |_, _| true), []);
        let (each, longest) = looked_on(&mut door, &mut work, end, 7, None);
        assert_eq!(
            each,
            [2, 2, 2, 2, 2, 10, 0],
            "two a minute from the call's end, and the ten left once they've waited LONGEST \
             since it, with room for them all"
        );
        assert!(longest <= LONGEST, "one waited {longest:?} past the call");
    }

    // Without settling, the pace would come back for any start held short
    // of room, so room coming back would let those in two a minute.
    #[test]
    fn once_the_pace_is_over_a_hold_for_room_alone_is_not_paced() {
        let mut door = Door::default();
        let at = Instant::now();
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(door.look(&[], calling, at, 1000, |_, _| true), []);
        let roomy = free(1 << 40);
        assert_eq!(door.look(&[], roomy, at + MINUTE, 1001, |_, _| true), []);
        let after = at + MINUTE + PACED;
        assert_eq!(door.look(&[], roomy, after, 1002, |_, _| true), []);
        let work = &thirteen()[..4];
        assert_eq!(
            held(&door.look(work, free(1 << 30), after, 1003, |_, _| true)),
            4
        );
        assert_eq!(
            let_in(&door.look(work, roomy, after + SPACED, 1004, |_, _| true)),
            4,
            "with the pace over, room coming back lets all four in"
        );
    }

    #[test]
    fn a_runs_later_step_never_begins_a_pace() {
        let mut door = Door::default();
        let at = Instant::now();
        let one = [coming(90, 1000, "nix eval .#x", "/c/work-90-1.scope")];
        assert_eq!(door.look(&one, free(1 << 40), at, 1000, |_, _| true), []);
        let later = at + QUIET;
        let uptime = 1000 + 100 * QUIET.as_secs();
        let run = [
            coming(60, 900, "bash ./build.sh", "/c/work-60-1.scope"),
            coming(61, uptime, "nix build .#x", "/c/work-60-1.scope"),
        ];
        assert_eq!(
            door.look(&run, free(1 << 40), later, uptime, |_, _| true),
            []
        );
        assert!(
            !door.pacing(later),
            "a run's later step, let in with its run, began a pace after a quiet stretch"
        );
    }

    // Joel, 9 November: "we need to not pause anything". A start that waits
    // its turn still waits no longer than LONGEST, as one short of room does.
    #[test]
    fn a_paced_start_comes_in_once_it_has_waited_longest() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..4];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(work, calling, at, 1000, |_, _| true)), 4);
        let stalled = Machine {
            stalled: STALLED * 4.0,
            ..free(1 << 40)
        };
        assert_eq!(
            let_in(&door.look(work, stalled, at + MINUTE, 1001, |_, _| true)),
            0,
            "while memory stalls none takes its turn"
        );
        assert_eq!(
            let_in(&door.look(work, stalled, at + MINUTE + LONGEST, 1002, |_, _| true)),
            1,
            "the one held longest comes in once it has waited LONGEST, its turn or not"
        );
        assert_eq!(
            let_in(&door.look(
                work,
                stalled,
                at + MINUTE + LONGEST + SPACED,
                1003,
                |_, _| true
            )),
            1,
            "and the next one SPACED later, as a wait for room is let go"
        );
    }

    // A backlog held through a stall isn't let in together once PACED has
    // passed: the pace lasts while the stall keeps it held.
    #[test]
    fn the_pace_lasts_until_what_it_held_has_come_in() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(work, calling, at, 1000, |_, _| true)), 13);
        let stalled = Machine {
            stalled: STALLED * 4.0,
            ..free(1 << 40)
        };
        let end = at + MINUTE;
        assert_eq!(let_in(&door.look(work, stalled, end, 1001, |_, _| true)), 0);
        let came = let_in(&door.look(work, stalled, end + PACED, 1002, |_, _| true));
        assert_eq!(
            came, 1,
            "past PACED in a stall, one that waited LONGEST comes in, not all at once"
        );
        assert!(
            door.pacing(end + PACED),
            "the pace holds while anything it held still waits"
        );
    }

    #[test]
    fn starts_after_a_quiet_stretch_come_in_two_a_minute() {
        let mut door = Door::default();
        let at = Instant::now();
        let roomy = free(1 << 40);
        let one = [coming(90, 1000, "nix eval .#x", "/c/work-90-1.scope")];
        assert_eq!(
            door.look(&one, roomy, at, 1000, |_, _| true),
            [],
            "one start alone comes straight in"
        );
        let later = at + QUIET;
        let uptime = 1000 + 100 * QUIET.as_secs();
        let together: Vec<Coming> = (0..6)
            .map(|n| {
                let scope: &'static str = Box::leak(format!("/c/work-{n}-2.scope").into());
                coming(200 + n, uptime, "nix build .#x", scope)
            })
            .collect();
        let done = door.look(&together, roomy, later, uptime, |_, _| true);
        assert_eq!(
            done.iter()
                .filter(|done| matches!(done, Done::Paced(_)))
                .count(),
            4,
            "two come in and the rest wait their turn: {done:?}"
        );
        assert_eq!(
            let_in(&door.look(&together, roomy, later + MINUTE, uptime + 1, |_, _| true)),
            2
        );
    }

    #[test]
    fn a_start_waits_its_turn_while_memory_stalls() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..2];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(work, calling, at, 1000, |_, _| true)), 2);
        let stalled = Machine {
            stalled: STALLED + 1.0,
            ..free(1 << 40)
        };
        assert!(
            door.look(work, stalled, at + MINUTE, 1001, |_, _| true)
                .is_empty(),
            "nothing comes in while memory stalls past its bar"
        );
        let eased = Machine {
            stalled: STALLED,
            ..free(1 << 40)
        };
        assert_eq!(
            let_in(&door.look(work, eased, at + MINUTE * 2, 1002, |_, _| true)),
            2
        );
    }

    #[test]
    fn a_switch_never_waits_its_turn() {
        let mut door = Door::default();
        let at = Instant::now();
        let work = &thirteen()[..3];
        let calling = Machine {
            calling: true,
            ..free(ROOM + GROWS)
        };
        assert_eq!(held(&door.look(work, calling, at, 1000, |_, _| true)), 3);
        let ended = free(1 << 40);
        assert_eq!(
            let_in(&door.look(work, ended, at + MINUTE, 1001, |_, _| true)),
            2
        );
        let switch = [first(60, 1100, "nix eval .#unit", "/c/work-60-1.scope")];
        let with: Vec<Coming> = work.iter().cloned().chain(switch).collect();
        assert_eq!(
            door.look(&with, ended, at + MINUTE, 1101, |_, _| true),
            [],
            "a switch comes in beside the two, unheld"
        );
    }

    #[test]
    fn the_memory_stalled_is_read_from_pressure() {
        let pressure = "some avg10=3.20 avg60=7.50 avg300=2.00 total=1\n\
                        full avg10=1.40 avg60=6.25 avg300=1.72 total=2\n";
        assert_eq!(stalled(pressure), Some(6.25));
        assert_eq!(stalled("some avg10=1 avg60=2 avg300=3 total=4\n"), None);
    }
}
