//! Moves a terminal's busy work into the jobs slice, so the shell, pi and
//! editors in that terminal keep the CPU they need while it runs.
//!
//! kitty starts what each window runs in a scope of its own,
//! kitty-<pid>-<n>.scope, at the weight of any other program, so a shell,
//! pi, an editor or a pager answers as quickly as Chrome does. A build
//! started there would share the CPU with Chrome scope by scope, so once
//! a process group has used more than `--busy` milliseconds of CPU a
//! second for `--ticks` seconds running, this moves the whole group into
//! the jobs cgroup, which is idle: it gets every CPU nobody else wants, and
//! what it starts afterwards stays there.
//!
//! Two kinds of group are never moved whole: the one the terminal started,
//! the shell, whose parent is outside the scope, and one whose leader is
//! named by `--keep`, pi say. A heavy process inside one of those, a
//! language server under an editor, is moved alone, with what it started.
//!
//! A scope that `systemd-run --user --scope` makes without a slice lands in
//! the apps' slice too, at an app's CPU, memory floor and disk, even when an
//! agent's shell in jobs started it: one such evaluation held memory
//! pressure near half while Joel's keys came 440 ms late. So a scope there
//! whose first process was started from inside jobs goes back at once to
//! where its starter is, unless it sets limits of its own, which moving it
//! out of the scope would drop. A service made the same way is only said
//! of, since who asked for it can't be seen, as `src/strays.rs` says.
//!
//! Work lands in a few services' cgroups, pi's in jobs.service and a
//! terminal's in awaited.service, each one cgroup for every command written
//! there. systemd-oomd, when swap runs out, ends the whole leaf cgroup that
//! holds the most of it, so it would end every session's builds at once
//! rather than the one that filled swap. Each `--split` cgroup's work, a
//! process tree whose root was written there, goes into a scope of its own,
//! work-<pid>-<start>.scope, once it has run a second, as `SETTLE` says
//! why. systemd drops the scope once it empties. The scopes
//! go in the slice named after `=`, or else the one the unit sits in. Its
//! scopes share that slice's weight, as the work shared the unit's, so a
//! cgroup that sits beside slices of other weights names a slice of its own.
//! A process forked as its tree moved goes to its parent's scope. The
//! unit's own holder stays, so the cgroup is there for the next write.
//!
//! Work under a `--no-real-time` cgroup runs without real time, which
//! `src/real_time.rs` says why and how, and goes first when memory runs
//! out, as `src/oom.rs` says.
//!
//! Work split into a `--by-quest` slice goes in its quest's slice there,
//! weighted as the `--quests` file says, which `src/quests.rs` says why and
//! how.
//!
//! With `--background`, a window, a `--home` or `--weigh` scope, whose
//! every pi is marked there as one nobody is at runs at `--low-weight`
//! until a mark goes, as `src/low.rs` says why and how.
//!
//! With `--door`, a new Nix evaluation or build under a cgroup waits while
//! memory is short, and through a call `--calls`' pw-dump hears, as
//! `src/door.rs` and `src/calls.rs` say why and how.
//!
//! With `--revive`, a cgroup gives memory back when the kernel's reclaim
//! thread gives up, as `src/revive.rs` says why and how.
//!
//! With `--orchestrators`, a window holding a session others report to,
//! as the mesh's presence folder there says, keeps a memory floor and is
//! the last systemd-oomd ends, and every other pi gives way first to the
//! kernel's OOM killer, as `src/keep.rs` says why and how.
//!
//! With `--niri` and `--background`, the session window Joel is at, as
//! niri's event stream says, and those he was at in the last minutes, rank
//! as his own programs do, as `src/focus.rs` says why and how.
//!
//! With `--hosts` and `--background`, each pi host's unit under that
//! prefix ranks as the sessions it holds would in windows of their own, as
//! `src/hosts.rs` says why and how.
//!
//! Terminals sit in each `--under` slice: Joel's apps' and, apart from
//! them, the agents' sessions'.
//!
//! Usage: jobs-sorter [--under app.slice]... [--home kitty-]... [--keep pi]...
//!        [--jobs jobs.slice/jobs.service] [--busy 500] [--ticks 2]
//!        [--every 1000] [--split jobs.slice/jobs.service[=SLICE]]...
//!        [--no-real-time jobs.slice/jobs-work.slice]...
//!        [--by-quest jobs-work.slice]... [--quests FILE]
//!        [--background DIR] [--low-weight 10] [--weigh PREFIX]...
//!        [--revive jobs.slice] [--door jobs.slice] [--calls pw-dump]
//!        [--orchestrators DIR] [--niri niri] [--hosts PREFIX]

mod calls;
mod door;
mod focus;
mod hosts;
mod keep;
mod low;
mod oom;
mod quests;
mod real_time;
mod revive;
mod strays;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::Duration;

/// /proc's CPU times are in `USER_HZ`, which is 100 on every Linux ABI.
const MS_PER_TICK: u64 = 10;

const CGROUPS: &str = "/sys/fs/cgroup";

struct Settings {
    under: Vec<String>,
    homes: Vec<String>,
    keep: Vec<String>,
    jobs: String,
    busy_ms: u64,
    ticks: u32,
    every: Duration,
    split: Vec<String>,
    no_real_time: Vec<String>,
    by_quest: Vec<String>,
    quests: Option<PathBuf>,
    background: Option<PathBuf>,
    low_weight: u64,
    weigh: Vec<String>,
    revive: Option<String>,
    orchestrators: Option<PathBuf>,
    door: Option<String>,
    calls: Option<PathBuf>,
    niri: Option<PathBuf>,
    hosts: Option<String>,
}

/// One process, as /proc/<pid>/stat describes it.
#[derive(Clone, Debug, PartialEq)]
struct Sample {
    pid: u32,
    ppid: u32,
    pgrp: u32,
    comm: String,
    /// Its own CPU time and its waited-for children's, in ticks.
    cpu: u64,
    /// When it started, in ticks since boot.
    start: u64,
}

/// Reads one /proc/<pid>/stat line. The name sits in parentheses and may
/// hold anything, parentheses included, so the fields after it are found
/// from the last ')'.
fn parse_stat(line: &str) -> Option<Sample> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let pid = line[..open].trim().parse().ok()?;
    let comm = line[open + 1..close].to_string();
    let rest: Vec<&str> = line[close + 1..].split_whitespace().collect();
    // rest[0] is the state (field 3), so field n is rest[n - 3].
    let field = |n: usize| -> Option<u64> { rest.get(n - 3)?.parse().ok() };
    Some(Sample {
        pid,
        ppid: u32::try_from(field(4)?).ok()?,
        pgrp: u32::try_from(field(5)?).ok()?,
        comm,
        cpu: field(14)? + field(15)? + field(16)? + field(17)?,
        start: field(22)?,
    })
}

/// The user manager's own cgroup, from this process's: everything up to
/// and including its user@<uid>.service.
fn manager_of(own: &str) -> Option<String> {
    let path = own.lines().find_map(|line| line.strip_prefix("0::"))?;
    let mut root = String::new();
    for part in path.split('/').filter(|part| !part.is_empty()) {
        root.push('/');
        root.push_str(part);
        if part.starts_with("user@") && part.ends_with(".service") {
            return Some(root);
        }
    }
    None
}

/// Some processes to move, and why, for the journal.
#[derive(Debug, PartialEq)]
struct Move {
    pids: Vec<u32>,
    says: String,
}

/// What it remembers between looks.
#[derive(Default)]
struct Sorter {
    /// Each process's CPU time at the last look.
    seen: HashMap<u32, u64>,
    /// Seconds running each group, or protected process, has been busy.
    busy: HashMap<(String, u32), u32>,
    /// When the last look was, in ticks since boot.
    last: u64,
    /// systemd-run's services already said of.
    told: strays::Told,
}

impl Sorter {
    /// Looks at one scope's processes and says what to move. `now` is in
    /// ticks since boot; `span` is the milliseconds since the last look.
    fn look(&mut self, scope: &str, procs: &[Sample], now: u64, settings: &Settings) -> Vec<Move> {
        let span_ms = (now.saturating_sub(self.last) * MS_PER_TICK).max(1);
        let busy_after = settings.busy_ms * span_ms / 1000;
        let used: HashMap<u32, u64> = procs
            .iter()
            .map(|proc| (proc.pid, self.used(proc)))
            .collect();
        let here: HashSet<u32> = procs.iter().map(|proc| proc.pid).collect();

        let mut groups: HashMap<u32, Vec<&Sample>> = HashMap::new();
        for proc in procs {
            groups.entry(proc.pgrp).or_default().push(proc);
        }

        let mut moves = Vec::new();
        let mut still: HashSet<(String, u32)> = HashSet::new();
        for (pgrp, members) in &groups {
            let leader = members.iter().find(|proc| proc.pid == *pgrp);
            let kept = leader.is_some_and(|leader| {
                !here.contains(&leader.ppid) || settings.keep.contains(&leader.comm)
            });
            if !kept {
                let total: u64 = members.iter().map(|proc| used[&proc.pid]).sum();
                let key = (scope.to_string(), *pgrp);
                if total * MS_PER_TICK >= busy_after && self.streak(&key, settings) {
                    let name = leader.map_or("a group", |leader| leader.comm.as_str());
                    moves.push(Move {
                        pids: members.iter().map(|proc| proc.pid).collect(),
                        says: format!(
                            "Moved {name} (process group {pgrp}, {} processes) from {scope} \
                             to jobs: {} ms of CPU in {span_ms} ms.",
                            members.len(),
                            total * MS_PER_TICK
                        ),
                    });
                } else if total * MS_PER_TICK >= busy_after {
                    still.insert(key);
                }
                continue;
            }
            for proc in members.iter().filter(|proc| proc.pid != *pgrp) {
                let key = (scope.to_string(), proc.pid);
                if used[&proc.pid] * MS_PER_TICK < busy_after {
                    continue;
                }
                if self.streak(&key, settings) {
                    moves.push(Move {
                        pids: descendants(proc.pid, procs),
                        says: format!(
                            "Moved {} ({}) and what it started from {scope} to jobs: \
                             {} ms of CPU in {span_ms} ms.",
                            proc.comm,
                            proc.pid,
                            used[&proc.pid] * MS_PER_TICK
                        ),
                    });
                } else {
                    still.insert(key);
                }
            }
        }
        // A streak breaks the moment its group or process rests.
        self.busy
            .retain(|key, _| key.0 != scope || still.contains(key));
        for proc in procs {
            self.seen.insert(proc.pid, proc.cpu);
        }
        moves
    }

    /// The CPU a process has used since the last look, in ticks. One that
    /// started since counts all of its own; one that was there unseen
    /// counts nothing yet, since what it used before is unknown.
    fn used(&self, proc: &Sample) -> u64 {
        match self.seen.get(&proc.pid) {
            Some(before) => proc.cpu.saturating_sub(*before),
            None if proc.start >= self.last => proc.cpu,
            None => 0,
        }
    }

    /// Counts one more busy look for a key, saying whether that's enough.
    fn streak(&mut self, key: &(String, u32), settings: &Settings) -> bool {
        let count = self.busy.entry(key.clone()).or_insert(0);
        *count += 1;
        if *count >= settings.ticks {
            self.busy.remove(key);
            return true;
        }
        false
    }

    /// Forgets processes that have gone, and notes when this look was.
    fn finish(&mut self, alive: &HashSet<u32>, now: u64) {
        self.seen.retain(|pid, _| alive.contains(pid));
        self.last = now;
    }
}

/// A process and every process under it in the scope.
fn descendants(top: u32, procs: &[Sample]) -> Vec<u32> {
    let mut found = vec![top];
    let mut index = 0;
    while index < found.len() {
        let parent = found[index];
        found.extend(
            procs
                .iter()
                .filter(|proc| proc.ppid == parent && !found.contains(&proc.pid))
                .map(|proc| proc.pid)
                .collect::<Vec<_>>(),
        );
        index += 1;
    }
    found
}

/// A process tree to give a scope of its own: its root, the first process
/// written into the cgroup, and every process under it there.
#[derive(Debug, PartialEq)]
struct Piece {
    root: u32,
    ppid: u32,
    comm: String,
    start: u64,
    pids: Vec<u32>,
}

/// How long a tree runs in its unit's cgroup before it's given a scope of
/// its own. Most commands an agent writes, a git call say, are done within
/// it, and one that ends in the moment between this reading it and systemd
/// making its scope leaves systemd logging that the scope failed to start:
/// 7 did in the minutes after a boot on 7 November, as 57 sessions came
/// back at once. A tree that runs on gets its scope at the next look,
/// which is all the scope is for: oomd ending one tree's work rather than
/// every session's.
const SETTLE: Duration = Duration::from_secs(1);

/// Whether a tree that started at `start` has run for `SETTLE` by `now`,
/// both in ticks since boot. Without a clock, every tree has.
fn settled(start: u64, now: Option<u64>) -> bool {
    let ticks = u64::try_from(SETTLE.as_millis()).unwrap_or(u64::MAX) / MS_PER_TICK;
    now.is_none_or(|now| now.saturating_sub(start) >= ticks)
}

/// The trees in a cgroup, one for each process whose parent is elsewhere,
/// but the unit's own holder.
fn pieces(procs: &[Sample], holder: Option<u32>) -> Vec<Piece> {
    let here: HashSet<u32> = procs.iter().map(|proc| proc.pid).collect();
    procs
        .iter()
        .filter(|proc| Some(proc.pid) != holder && !here.contains(&proc.ppid))
        .map(|root| Piece {
            root: root.pid,
            ppid: root.ppid,
            comm: root.comm.clone(),
            start: root.start,
            pids: descendants(root.pid, procs),
        })
        .collect()
}

/// Those of `pids` still in `cgroup`, as `cgroup_of` reads each again,
/// just before they're moved. A process can leave in the second since the
/// sample, as `systemd-run --scope` does when it puts itself in a scope of
/// its own, and moving it then would take it out of that scope, which
/// systemd ends once it's empty; on 29 October that left a test's
/// `kept.scope` gone and its starter waiting on it for good. This narrows
/// the window to the moment between reading and moving, which the user
/// manager's call can't close.
fn still_in(pids: &[u32], cgroup: &str, cgroup_of: impl Fn(u32) -> Option<String>) -> Vec<u32> {
    let here = format!("/{}", cgroup.trim_start_matches('/'));
    pids.iter()
        .copied()
        .filter(|pid| cgroup_of(*pid).is_some_and(|path| path == here))
        .collect()
}

/// A `--split` value's cgroup, the slice its scopes go in, and its unit.
/// The slice is the one named after `=`, or else the one the unit sits in:
/// jobs-awaited.slice for jobs.slice/jobs-awaited.slice/awaited.service.
/// Its unit is the cgroup's last part.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "a systemd unit's suffix is case-sensitive"
)]
fn slice_and_unit(split: &str) -> Option<(&str, &str, &str)> {
    let (cgroup, named) = match split.split_once('=') {
        Some((cgroup, slice)) => (cgroup, Some(slice)),
        None => (split, None),
    };
    let (path, unit) = cgroup.rsplit_once('/')?;
    let slice = named.or_else(|| path.rsplit('/').next())?;
    slice
        .ends_with(".slice")
        .then_some((cgroup, slice, unit))
        .filter(|_| !unit.is_empty())
}

/// Whether a cgroup path is a scope this made, so a straggler joins it.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "a systemd unit's suffix is case-sensitive"
)]
fn made_here(cgroup: &str) -> bool {
    cgroup
        .rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("work-") && name.ends_with(".scope"))
}

/// How long a work scope's processes have to end once asked, before
/// systemd kills what's left. They're asked with a hangup as well as
/// SIGTERM, since an interactive shell ignores SIGTERM and ends on a
/// hangup. Without either, a scope took the user manager's 90 s, longer
/// than a reboot waits: on 7 November one bash kept the reboot waiting
/// its whole 60 s, which systemd then forced.
const WORK_STOP: Duration = Duration::from_secs(5);

/// What busctl is given to start a scope named `name` in `slice` around
/// `pids`: the user manager's `StartTransientUnit`, with its description.
/// A command can exit between being read and being moved, and systemd
/// then fails its scope, which would stay in the failed list for good; so
/// a failed scope is unloaded as a finished one is. When only some have
/// exited, systemd moves the rest and logs "PID ... vanished before we could
/// move it" for each one gone: that is this race, and no fault.
///
/// A scope is stopped as `WORK_STOP` says.
fn scope_call(name: &str, slice: &str, description: &str, pids: &[u32]) -> Vec<String> {
    let mut args: Vec<String> = [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "StartTransientUnit",
        "ssa(sv)a(sa(sv))",
        name,
        "fail",
        "6",
        "Slice",
        "s",
        slice,
        "Description",
        "s",
        description,
        "CollectMode",
        "s",
        "inactive-or-failed",
        "SendSIGHUP",
        "b",
        "true",
        "TimeoutStopUSec",
        "t",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect();
    args.push(WORK_STOP.as_micros().to_string());
    args.extend(["PIDs".into(), "au".into()]);
    args.push(pids.len().to_string());
    args.extend(pids.iter().map(u32::to_string));
    args.push("0".into());
    args
}

/// The main process systemd knows a unit by, its holder.
fn main_pid(unit: &str) -> Option<u32> {
    let output = Command::new("systemctl")
        .args(["--user", "show", "--property=MainPID", "--value", unit])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let pid: u32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;
    (pid != 0).then_some(pid)
}

/// Sets a quest slice's weight, unless it already has it. A failure is
/// said once, apart from the sorter's other failures, and tried again on
/// the next look.
fn weigh(weights: &mut quests::Weights, slice: &str, weight: u64) {
    if weights.unweighed(&[(slice.to_string(), weight)]).is_empty() {
        return;
    }
    match Command::new("busctl")
        .args(quests::weigh_call(slice, weight))
        .stdout(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {
            weights.weighed(slice, weight);
            eprintln!("Weighed {slice} at {weight}.");
        }
        Ok(output) => {
            if weights.first_failure() {
                eprintln!(
                    "Couldn't weigh {slice} at {weight}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
        }
        Err(error) => {
            if weights.first_failure() {
                eprintln!("Couldn't run busctl to weigh {slice}: {error}");
            }
        }
    }
}

/// The windows weighed low: the scopes in the `under` slices whose names
/// start with one of `prefixes`, while every pi in one is marked in `dir`,
/// at `weight`.
struct Windows<'a> {
    under: &'a [PathBuf],
    prefixes: &'a [String],
    dir: &'a Path,
    weight: u64,
    /// niri's focus, when it's followed.
    focus: Option<&'a focus::Following>,
    /// The prefix of the pi hosts' units, when they're ranked.
    hosts: Option<&'a str>,
}

/// How each window ranks, as last set: its weight, whether it's kept as
/// an orchestrator's, and whether it's held as the one Joel is at.
#[derive(Default)]
struct Ranks {
    weighing: low::Weighing,
    keeping: keep::Keeping,
    holding: focus::Holding,
    /// Each held window's scope, by niri's id for it, or when it was last
    /// looked for and not found.
    found: HashMap<u64, Result<PathBuf, std::time::Instant>>,
    /// Each pi host's rank, as last set.
    hosts: hosts::Hosts,
    /// The orchestrators each pid holds, as the presences last said.
    orchestrating: HashMap<u32, usize>,
}

/// The windows a sorter before this one held, to give back those not
/// wanted now; none while focus isn't followed, as nothing would give
/// them back ([`focus::Holding::left`]).
fn left_held(
    following: Option<&focus::Following>,
    under: &[PathBuf],
    prefixes: &[String],
) -> focus::Holding {
    if following.is_none() {
        return focus::Holding::default();
    }
    let scopes = under.iter().flat_map(|slice| homes(slice, prefixes));
    focus::Holding::left(scopes, |scope| {
        fs::read_to_string(scope.join("memory.low"))
            .ok()
            .and_then(|low| low.trim().parse().ok())
    })
}

/// How long a window whose scope wasn't found waits to be looked for
/// again, as one just opened may have started nothing yet.
const LOOK_AGAIN: Duration = Duration::from_secs(5);

/// Every window's scope, by its unit, with the app ids its processes were
/// started with.
fn started_with(scopes: &[PathBuf]) -> Vec<(String, Vec<String>)> {
    scopes
        .iter()
        .filter_map(|scope| {
            let unit = scope.file_name()?.to_str()?.to_owned();
            let procs = fs::read_to_string(scope.join("cgroup.procs")).unwrap_or_default();
            let apps = procs
                .lines()
                .filter_map(|pid| fs::read(format!("/proc/{pid}/environ")).ok())
                .filter_map(|environ| focus::app_id(&environ))
                .collect();
            Some((unit, apps))
        })
        .collect()
}

/// Holds the scope of each window Joel is at or was at a moment ago, as
/// niri's focus says, gives back each that aged out, and says which units
/// are held. A failure is said once, and tried again on the next look.
fn hold_windows(windows: &Windows, ranks: &mut Ranks) -> HashSet<String> {
    let Some(following) = windows.focus else {
        return HashSet::new();
    };
    let now = std::time::Instant::now();
    let wanted = following.held(now);
    ranks
        .found
        .retain(|id, _| wanted.iter().any(|window| window.id == *id));
    let mut scopes: Option<Vec<PathBuf>> = None;
    let mut held = Vec::new();
    for window in &wanted {
        let known = match ranks.found.get(&window.id) {
            Some(Ok(scope)) if scope.exists() => Some(scope.clone()),
            Some(Err(when)) if now.saturating_duration_since(*when) < LOOK_AGAIN => None,
            _ => {
                let all = scopes.get_or_insert_with(|| {
                    windows
                        .under
                        .iter()
                        .flat_map(|slice| homes(slice, windows.prefixes))
                        .collect()
                });
                let unit = focus::scope_of(window, &started_with(all)).map(str::to_owned);
                let scope = all
                    .iter()
                    .find(|scope| {
                        scope.file_name().and_then(|name| name.to_str()) == unit.as_deref()
                    })
                    .cloned();
                ranks.found.insert(window.id, scope.clone().ok_or(now));
                scope
            }
        };
        held.extend(known);
    }
    ranks.holding.keep_only(Path::exists);
    for (scope, hold) in ranks.holding.changes(&held) {
        let unit = scope
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let result = Command::new("busctl")
            .args(focus::hold_call(&unit, hold))
            .stdout(Stdio::null())
            .output();
        match result {
            Ok(output) if output.status.success() => {
                ranks.holding.done(&scope, hold);
                // An orchestrator's window gave its floor up to this one,
                // so it's kept again from nothing once given back.
                ranks.keeping.done(&unit, false);
                let says = if hold { "Held" } else { "Gave back" };
                eprintln!("{says} {unit}, a window Joel is or was just at.");
            }
            Ok(output) => {
                if !std::mem::replace(&mut ranks.holding.failed, true) {
                    eprintln!(
                        "Couldn't hold {unit}: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
            }
            Err(error) => {
                if !std::mem::replace(&mut ranks.holding.failed, true) {
                    eprintln!("Couldn't run busctl to hold {unit}: {error}");
                }
            }
        }
    }
    held.iter()
        .filter_map(|scope| Some(scope.file_name()?.to_str()?.to_owned()))
        .collect()
}

/// Watches the folder of marks, made if it isn't there yet. Without a
/// watch, which is said, windows keep their weight.
fn watch_marks(dir: &Path) -> Option<low::Watch> {
    fs::create_dir_all(dir)
        .and_then(|()| low::Watch::new(dir))
        .map_err(|error| eprintln!("Can't watch {}: {error}", dir.display()))
        .ok()
}

/// Waits out `every` between passes, weighing the windows each time a mark
/// comes or goes and once at its end. A mark that goes is a key on its way,
/// so its window is weighed back at once, and the pass still comes on time.
fn rest(watch: &low::Watch, every: Duration, windows: &Windows, ranks: &mut Ranks) {
    let at = std::time::Instant::now() + every;
    loop {
        let left = at.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() || !watch.wait(left) {
            break;
        }
        weigh_windows(windows, ranks);
    }
    weigh_windows(windows, ranks);
}

/// Whether one of `pis` started within `low::STARTING_FOR` of `now`, in
/// ticks since boot.
fn starting(pis: &[Sample], now: u64) -> bool {
    let ticks = u64::try_from(low::STARTING_FOR.as_millis()).unwrap_or(u64::MAX) / MS_PER_TICK;
    pis.iter()
        .any(|proc| now.saturating_sub(proc.start) < ticks)
}

/// Weighs each window high while Joel is at it or a pi in it starts, low
/// while every pi in it is marked, and back once none of those holds. A
/// failure is said once, and tried again on the next look.
fn weigh_windows(windows: &Windows, ranks: &mut Ranks) {
    let held = hold_windows(windows, ranks);
    let Windows {
        under,
        prefixes,
        dir,
        weight,
        focus: _,
        hosts: _,
    } = *windows;
    if let Some(prefix) = windows.hosts {
        rank_hosts(windows, prefix, &held, ranks);
    }
    let weighing = &mut ranks.weighing;
    let now = uptime_ticks();
    let windows: Vec<(String, u64)> = under
        .iter()
        .flat_map(|slice| homes(slice, prefixes))
        .filter_map(|scope| {
            let unit = scope.file_name()?.to_str()?.to_owned();
            let pis: Vec<Sample> = samples(&scope)
                .into_iter()
                .filter(|proc| proc.comm == "pi")
                .collect();
            let starting = now.is_some_and(|now| starting(&pis, now));
            let running: Vec<u32> = pis.iter().map(|proc| proc.pid).collect();
            let marked = |pid: u32| dir.join(pid.to_string()).exists();
            let wanted = if held.contains(&unit) {
                focus::WEIGHT
            } else {
                low::wanted(&running, starting, marked, weight)
            };
            Some((unit, wanted))
        })
        .collect();
    weighing.keep_only(
        &windows
            .iter()
            .map(|(unit, _)| unit.clone())
            .collect::<Vec<_>>(),
    );
    for (unit, wanted) in weighing.changes(&windows) {
        match Command::new("busctl")
            .args(low::weigh_call(&unit, wanted))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                weighing.weighed(&unit, wanted);
                eprintln!("Weighed {unit} at {wanted}.");
            }
            Ok(output) => {
                if !std::mem::replace(&mut weighing.failed, true) {
                    eprintln!(
                        "Couldn't weigh {unit} at {wanted}: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
            }
            Err(error) => {
                if !std::mem::replace(&mut weighing.failed, true) {
                    eprintln!("Couldn't run busctl to weigh {unit}: {error}");
                }
            }
        }
    }
}

/// Ranks each pi host's unit under `prefix` as its conversations would
/// rank in windows, from the windows `held` as ones Joel is at and the
/// orchestrators last counted. A failure is said once, and tried again on
/// the next look.
fn rank_hosts(windows: &Windows, prefix: &str, held: &HashSet<String>, ranks: &mut Ranks) {
    let viewers: Vec<String> = windows
        .under
        .iter()
        .flat_map(|slice| held.iter().map(|unit| slice.join(unit)))
        .flat_map(|scope| samples(&scope))
        .filter_map(|proc| {
            let words = fs::read(format!("/proc/{}/cmdline", proc.pid)).ok()?;
            let words: Vec<String> = words
                .split(|byte| *byte == 0)
                .map(|word| String::from_utf8_lossy(word).into_owned())
                .collect();
            hosts::viewed(&words, prefix)
        })
        .collect();
    let now = std::time::Instant::now();
    let total = memory_total();
    let units: Vec<PathBuf> = windows
        .under
        .iter()
        .flat_map(|slice| hosts::units(slice, prefix))
        .collect();
    let mut seen = Vec::new();
    for path in units {
        let Some(unit) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let procs: Vec<(u32, u32)> = samples(&path)
            .iter()
            .map(|proc| (proc.pid, proc.ppid))
            .collect();
        let Some(pid) = hosts::host_of(&procs) else {
            continue;
        };
        seen.push(unit.to_owned());
        let own = ranks.hosts.own(unit, pid, now, || {
            let output = Command::new("systemctl")
                .args(["--user", "cat", "--", unit])
                .stderr(Stdio::null())
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| hosts::own(&String::from_utf8_lossy(&output.stdout), total))
        });
        let signs = hosts::Signs {
            viewed: viewers.iter().filter(|viewed| *viewed == unit).count(),
            orchestrators: ranks.orchestrating.get(&pid).copied().unwrap_or(0),
            marked: windows.dir.join(pid.to_string()).exists(),
        };
        let rank = hosts::wanted(&own, signs, windows.weight);
        if !ranks.hosts.changed(unit, &rank) {
            continue;
        }
        match Command::new("busctl")
            .args(hosts::rank_call(unit, &rank))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                eprintln!(
                    "Ranked {unit} at weight {}, floor {}, for {} windows Joel is at and {} orchestrators.",
                    rank.weight, rank.floor, signs.viewed, signs.orchestrators
                );
                ranks.hosts.done(unit, rank);
            }
            Ok(output) => {
                if !std::mem::replace(&mut ranks.hosts.failed, true) {
                    eprintln!(
                        "Couldn't rank {unit}: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
            }
            Err(error) => {
                if !std::mem::replace(&mut ranks.hosts.failed, true) {
                    eprintln!("Couldn't run busctl to rank {unit}: {error}");
                }
            }
        }
    }
    ranks.hosts.keep_only(&seen);
}

/// The machine's memory in bytes, as /proc/meminfo says, which a floor
/// given as a percentage is of; none when it can't be read.
fn memory_total() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|text| {
            let line = text.lines().find(|line| line.starts_with("MemTotal:"))?;
            let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
            kib.checked_mul(1024)
        })
        .unwrap_or(0)
}

/// Keeps each window holding an orchestrator, as `dir`'s presences name
/// them, gives back each that no longer holds one, and puts each pi at
/// the adjustment it wants. A failure is said once, and tried again on
/// the next look.
fn keep_windows(under: &[PathBuf], prefixes: &[String], dir: &Path, ranks: &mut Ranks) {
    let Ranks {
        keeping,
        holding,
        orchestrating,
        ..
    } = ranks;
    let Ok(boot) = fs::read_to_string("/proc/sys/kernel/random/boot_id") else {
        return;
    };
    let proc = Path::new("/proc");
    let runs = |pid: u32| proc.join(pid.to_string()).exists();
    let presences = keep::read(dir);
    let orchestrators = keep::orchestrators(&presences, boot.trim(), runs);
    // A host holds many sessions under its one pid, so it's counted by
    // each, for `rank_hosts`.
    *orchestrating = hosts::orchestrating(&presences, boot.trim(), runs);
    let mut warned = keeping.failed;
    let windows: Vec<(String, bool)> = under
        .iter()
        .flat_map(|slice| homes(slice, prefixes))
        .filter_map(|scope| {
            let unit = scope.file_name()?.to_str()?.to_owned();
            let pis: Vec<u32> = samples(&scope)
                .into_iter()
                .filter(|sample| sample.comm == "pi")
                .map(|sample| sample.pid)
                .collect();
            for &pid in &pis {
                keep::adjust(
                    proc,
                    pid,
                    keep::adjustment(pid, &orchestrators),
                    &mut warned,
                );
            }
            Some((unit, pis.iter().any(|pid| orchestrators.contains(pid))))
        })
        .collect();
    keeping.failed = warned;
    keeping.keep_only(
        &windows
            .iter()
            .map(|(unit, _)| unit.clone())
            .collect::<Vec<_>>(),
    );
    // A window held as one Joel is at has a floor over an orchestrator's,
    // and is kept again once it's given back.
    for (unit, keep) in keeping
        .changes(&windows)
        .into_iter()
        .filter(|(unit, _)| !holding.holds(unit))
    {
        match Command::new("busctl")
            .args(keep::keep_call(&unit, keep))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                keeping.done(&unit, keep);
                let says = if keep { "Kept" } else { "Gave back" };
                eprintln!("{says} {unit}, an orchestrator's window.");
            }
            Ok(output) => {
                if !std::mem::replace(&mut keeping.failed, true) {
                    eprintln!(
                        "Couldn't keep {unit}: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
            }
            Err(error) => {
                if !std::mem::replace(&mut keeping.failed, true) {
                    eprintln!("Couldn't run busctl to keep {unit}: {error}");
                }
            }
        }
    }
}

/// Gives each piece of work in one split cgroup a scope of its own, in
/// its quest's slice when its slice is sorted by quest. A tree that left
/// before its scope began is no loss; anything else is said once.
fn split(
    root: &Path,
    manager: &str,
    value: &str,
    holder: &mut Option<u32>,
    by_quest: Option<&mut quests::Weights>,
    warned: &mut bool,
) {
    let Some((cgroup, slice, unit)) = slice_and_unit(value) else {
        return;
    };
    let mut by_quest = by_quest;
    let procs = samples(&root.join(cgroup));
    if holder.is_none_or(|pid| !procs.iter().any(|proc| proc.pid == pid)) {
        *holder = main_pid(unit);
    }
    let now = uptime_ticks();
    for mut piece in pieces(&procs, *holder) {
        if !settled(piece.start, now) {
            continue;
        }
        piece.pids = still_in(&piece.pids, cgroup, |pid| cgroup_of(pid, manager));
        if piece.pids.is_empty() {
            continue;
        }
        if let Some(parent) = cgroup_of(piece.ppid, manager).filter(|path| made_here(path)) {
            send(
                &root.join(parent.trim_start_matches('/')),
                &piece.pids,
                warned,
            );
            continue;
        }
        let placed = by_quest.as_deref().and_then(|weights| {
            let environ = fs::read(format!("/proc/{}/environ", piece.root)).ok()?;
            weights.place(slice, &environ)
        });
        let into = placed.as_ref().map_or(slice, |(quest, _)| quest.as_str());
        let name = format!("work-{}-{}.scope", piece.root, piece.start);
        let description = format!("{} ({}), work from {unit}", piece.comm, piece.root);
        let result = Command::new("busctl")
            .args(scope_call(&name, into, &description, &piece.pids))
            .stdout(Stdio::null())
            .output();
        match result {
            Ok(output) if output.status.success() => {
                // Once the scope has brought the slice into being.
                if let (Some(weights), Some((quest, weight))) = (by_quest.as_deref_mut(), &placed) {
                    weigh(weights, quest, *weight);
                }
            }
            // A tree whose processes have all gone has nothing to move.
            Ok(_)
                if piece
                    .pids
                    .iter()
                    .all(|pid| !Path::new(&format!("/proc/{pid}")).exists()) => {}
            Ok(output) if !*warned => {
                eprintln!(
                    "Couldn't give {} ({}) a scope of its own: {}",
                    piece.comm,
                    piece.root,
                    String::from_utf8_lossy(&output.stderr).trim()
                );
                *warned = true;
            }
            Err(error) if !*warned => {
                eprintln!("Couldn't run busctl: {error}");
                *warned = true;
            }
            _ => {}
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "uptime is never negative, and far inside a u64 in milliseconds"
)]
fn uptime_ticks() -> Option<u64> {
    let text = fs::read_to_string("/proc/uptime").ok()?;
    let seconds: f64 = text.split_whitespace().next()?.parse().ok()?;
    Some((seconds * 1000.0) as u64 / MS_PER_TICK)
}

fn samples(scope: &Path) -> Vec<Sample> {
    let Ok(text) = fs::read_to_string(scope.join("cgroup.procs")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|pid| fs::read_to_string(format!("/proc/{pid}/stat")).ok())
        .filter_map(|line| parse_stat(&line))
        .collect()
}

#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "a systemd unit's suffix is case-sensitive"
)]
fn homes(under: &Path, prefixes: &[String]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(under) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.ends_with(".scope") && prefixes.iter().any(|home| name.starts_with(home))
                })
        })
        .collect()
}

/// Where a scope's work goes back to: the cgroup of the process that
/// started it, outside the scope, when that sits in `jobs` or under it.
/// Paths are the user manager's, as /proc/<pid>/cgroup gives them below it.
fn returns_to(
    procs: &[Sample],
    cgroup_of: impl Fn(u32) -> Option<String>,
    jobs: &str,
) -> Option<String> {
    let here: HashSet<u32> = procs.iter().map(|proc| proc.pid).collect();
    let inside = format!("{jobs}/");
    procs
        .iter()
        .filter(|proc| !here.contains(&proc.ppid))
        .filter_map(|proc| cgroup_of(proc.ppid))
        .find(|cgroup| cgroup == jobs || cgroup.starts_with(&inside))
}

/// Whether a cgroup holds only systemd's defaults for what a scope may set:
/// no memory ceiling, quota, weight of its own or disk limit. Its task
/// limit isn't asked, since systemd gives every scope one.
fn unlimited(read: impl Fn(&str) -> Option<String>) -> bool {
    let reads = |file: &str, unset: &str| read(file).is_none_or(|value| value.trim() == unset);
    reads("memory.max", "max")
        && reads("memory.high", "max")
        && reads("memory.swap.max", "max")
        && reads("cpu.weight", "100")
        && reads("io.max", "")
        && read("cpu.max").is_none_or(|value| value.split_whitespace().next() == Some("max"))
}

/// Every scope under `under` but the terminals'.
#[expect(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "a systemd unit's suffix is case-sensitive"
)]
fn strays(under: &Path, homes: &[String]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(under) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.ends_with(".scope") && !homes.iter().any(|home| name.starts_with(home))
                })
        })
        .collect()
}

/// A process's cgroup below the user manager's, or `None` outside it.
fn cgroup_of(pid: u32, manager: &str) -> Option<String> {
    let text = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    let path = text.lines().find_map(|line| line.strip_prefix("0::"))?;
    path.strip_prefix(manager).map(str::to_owned)
}

/// Writes each process into the jobs cgroup. One that has exited since is
/// no loss; anything else is said once.
fn send(jobs: &Path, pids: &[u32], warned: &mut bool) {
    for pid in pids {
        if let Err(error) = fs::write(jobs.join("cgroup.procs"), format!("{pid}\n"))
            && error.raw_os_error() != Some(3)
            && !*warned
        {
            eprintln!("Couldn't move {pid} into {}: {error}", jobs.display());
            *warned = true;
        }
    }
}

fn settings(mut args: impl Iterator<Item = String>) -> Result<Settings, String> {
    let mut settings = Settings {
        under: Vec::new(),
        homes: Vec::new(),
        keep: Vec::new(),
        jobs: "jobs.slice/jobs.service".into(),
        busy_ms: 500,
        ticks: 2,
        every: Duration::from_millis(1000),
        split: Vec::new(),
        no_real_time: Vec::new(),
        by_quest: Vec::new(),
        quests: None,
        background: None,
        low_weight: 10,
        weigh: Vec::new(),
        revive: None,
        orchestrators: None,
        door: None,
        calls: None,
        niri: None,
        hosts: None,
    };
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--under" => settings.under.push(value()?),
            "--home" => settings.homes.push(value()?),
            "--keep" => settings.keep.push(value()?),
            "--jobs" => settings.jobs = value()?,
            "--split" => settings.split.push(value()?),
            "--no-real-time" => settings.no_real_time.push(value()?),
            "--by-quest" => settings.by_quest.push(value()?),
            "--quests" => settings.quests = Some(PathBuf::from(value()?)),
            "--background" => settings.background = Some(PathBuf::from(value()?)),
            "--weigh" => settings.weigh.push(value()?),
            "--revive" => settings.revive = Some(value()?),
            "--door" => settings.door = Some(value()?),
            "--calls" => settings.calls = Some(PathBuf::from(value()?)),
            "--niri" => settings.niri = Some(PathBuf::from(value()?)),
            "--hosts" => settings.hosts = Some(value()?),
            "--orchestrators" => settings.orchestrators = Some(PathBuf::from(value()?)),
            "--low-weight" => {
                settings.low_weight = value()?.parse().map_err(|_| "--low-weight is a weight")?;
            }
            "--busy" => settings.busy_ms = value()?.parse().map_err(|_| "--busy is ms")?,
            "--ticks" => settings.ticks = value()?.parse().map_err(|_| "--ticks is a count")?,
            "--every" => {
                let ms = value()?.parse().map_err(|_| "--every is ms")?;
                settings.every = Duration::from_millis(ms);
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if settings.under.is_empty() {
        settings.under.push("app.slice".into());
    }
    if settings.homes.is_empty() {
        settings.homes.push("kitty-".into());
    }
    Ok(settings)
}

#[cfg(test)]
mod settings_tests {
    use super::settings;

    fn given(words: &str) -> Vec<String> {
        settings(words.split_whitespace().map(str::to_owned))
            .unwrap()
            .under
    }

    #[test]
    fn the_apps_slice_is_watched_when_none_is_named() {
        assert_eq!(given(""), ["app.slice"]);
    }

    #[test]
    fn every_slice_named_is_watched() {
        assert_eq!(
            given("--under app.slice --under agents.slice"),
            ["app.slice", "agents.slice"]
        );
    }
}

/// Moves a scope that work in `slice` started, with no memory ceiling of
/// its own, back to the cgroup of what started it.
fn return_stray(scope: &Path, root: &Path, manager: &str, slice: &str, warned: &mut bool) {
    let procs = samples(scope);
    if procs.is_empty() || !unlimited(|file| fs::read_to_string(scope.join(file)).ok()) {
        return;
    }
    let Some(back) = returns_to(&procs, |pid| cgroup_of(pid, manager), slice) else {
        return;
    };
    let pids: Vec<u32> = procs.iter().map(|proc| proc.pid).collect();
    send(&root.join(back.trim_start_matches('/')), &pids, warned);
    let name = scope.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "Moved {name} ({} processes) back to {back}, which started it.",
        pids.len()
    );
}

/// Follows niri's focus with `niri`, waking `watch` as it moves. Windows
/// are weighed and held as the marks' watch wakes, so niri's focus is
/// followed only beside it.
fn follow_focus(niri: Option<&Path>, watch: Option<&mut low::Watch>) -> Option<focus::Following> {
    match (niri, watch) {
        (Some(niri), Some(watch)) => {
            let waker = watch
                .waker()
                .map_err(|error| eprintln!("The focus can't wake the sorter: {error}"))
                .ok();
            Some(focus::Following::start(niri, waker))
        }
        (Some(_), None) => {
            eprintln!("--niri needs --background, so no window is held for focus.");
            None
        }
        _ => None,
    }
}

fn run() -> Result<(), String> {
    let settings = settings(std::env::args().skip(1))?;
    let own = fs::read_to_string("/proc/self/cgroup").map_err(|e| e.to_string())?;
    let manager = manager_of(&own).ok_or("not running under a user manager")?;
    let root = Path::new(CGROUPS).join(manager.trim_start_matches('/'));
    let under: Vec<PathBuf> = settings
        .under
        .iter()
        .map(|slice| root.join(slice))
        .collect();
    let jobs = root.join(&settings.jobs);
    let slice = format!(
        "/{}",
        settings.jobs.split('/').next().unwrap_or(&settings.jobs)
    );
    if !jobs.join("cgroup.procs").exists() {
        return Err(format!("{} doesn't exist", jobs.display()));
    }
    eprintln!(
        "Sorting busy work from {} ({}*) into {}.",
        settings.under.join(", "),
        settings.homes.join("*, "),
        jobs.display()
    );
    if let Some(cgroup) = &settings.revive {
        revive::watch(&root.join(cgroup));
    }
    let mut sorter = Sorter {
        last: uptime_ticks().unwrap_or(0),
        ..Sorter::default()
    };
    let mut warned = false;
    let mut holders: Vec<Option<u32>> = vec![None; settings.split.len()];
    let mut weights = quests::Weights::new(settings.quests.clone().unwrap_or_default());
    let mut pass: u32 = 0;
    let windows = [settings.homes.as_slice(), &settings.weigh].concat();
    let mut watch = settings.background.as_deref().and_then(watch_marks);
    let following = follow_focus(settings.niri.as_deref(), watch.as_mut());
    let mut ranks = Ranks {
        holding: left_held(following.as_ref(), &under, &windows),
        ..Ranks::default()
    };
    let mut doorway = Doorway::new(settings.calls.as_deref());
    loop {
        match (&watch, &settings.background) {
            (Some(watch), Some(dir)) => {
                let windows = Windows {
                    under: &under,
                    prefixes: &windows,
                    dir,
                    weight: settings.low_weight,
                    focus: following.as_ref(),
                    hosts: settings.hosts.as_deref(),
                };
                rest(watch, settings.every, &windows, &mut ranks);
            }
            _ => thread::sleep(settings.every),
        }
        let now = uptime_ticks().ok_or("can't read /proc/uptime")?;
        let mut alive = HashSet::new();
        for scope in under.iter().flat_map(|slice| homes(slice, &settings.homes)) {
            let procs = samples(&scope);
            alive.extend(procs.iter().map(|proc| proc.pid));
            let name = scope.file_name().unwrap_or_default().to_string_lossy();
            for found in sorter.look(&name, &procs, now, &settings) {
                send(&jobs, &found.pids, &mut warned);
                eprintln!("{}", found.says);
            }
        }
        for scope in under
            .iter()
            .flat_map(|slice| strays(slice, &settings.homes))
        {
            return_stray(&scope, &root, &manager, &slice, &mut warned);
        }
        sorter.told.say(&under);
        // After the terminals' work is moved, so what moved is split too.
        weights.refresh();
        for base in &settings.by_quest {
            for (slice, weight) in weights.unweighed(&weights.current(base)) {
                weigh(&mut weights, &slice, weight);
            }
        }
        for (cgroup, holder) in settings.split.iter().zip(holders.iter_mut()) {
            let quest_slices = slice_and_unit(cgroup)
                .is_some_and(|(_, slice, _)| settings.by_quest.iter().any(|base| base == slice));
            let by_quest = quest_slices.then_some(&mut weights);
            split(&root, &manager, cgroup, holder, by_quest, &mut warned);
        }
        if pass.is_multiple_of(real_time::EVERY) {
            if let Some(dir) = &settings.orchestrators {
                keep_windows(&under, &windows, dir, &mut ranks);
            }
            for cgroup in &settings.no_real_time {
                real_time::take(&root.join(cgroup), &mut warned);
                let work = real_time::processes_under(&root.join(cgroup));
                oom::raise(Path::new("/proc"), &work, &mut warned);
            }
        }
        watch_door(&mut doorway, &root, settings.door.as_deref(), now);
        pass = pass.wrapping_add(1);
        sorter.finish(&alive, now);
    }
}

/// The door, and the calls it keeps more room through when it hears them.
struct Doorway {
    door: door::Door,
    calls: Option<calls::Calls>,
}

impl Doorway {
    /// A door that hears calls through `pw_dump`, when there's one.
    fn new(pw_dump: Option<&Path>) -> Doorway {
        Doorway {
            door: door::Door::default(),
            calls: pw_dump.map(calls::Calls::follow),
        }
    }
}

/// Ranks new heavy starts under `cgroup` in `root` behind other work, when
/// there's a door, while memory is short, and holds them while it's
/// shorter still or a call is on, as src/door.rs says, `uptime` ticks since
/// boot.
fn watch_door(doorway: &mut Doorway, root: &Path, cgroup: Option<&str>, uptime: u64) {
    let Some(cgroup) = cgroup.map(|cgroup| root.join(cgroup)) else {
        return;
    };
    let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let (Some(total), Some(free)) = (door::total(&meminfo), door::available(&meminfo)) else {
        return;
    };
    let now = std::time::Instant::now();
    let machine = door::Machine {
        total,
        free,
        calling: doorway.calls.as_mut().is_some_and(|calls| calls.on(now)),
    };
    let read: Vec<(u32, u64, String, PathBuf)> = real_time::processes_under(&cgroup)
        .into_iter()
        .filter_map(|pid| {
            let stat = parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)?;
            let words = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
            let words = String::from_utf8_lossy(&words).replace('\0', " ");
            let own = fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
            let path = own.lines().find_map(|line| line.strip_prefix("0::"))?;
            let scope = Path::new(CGROUPS).join(path.trim_start_matches('/'));
            Some((pid, stat.start, words, scope))
        })
        .collect();
    let blocking = door::blocking(&fs::read_to_string("/proc/locks").unwrap_or_default());
    let coming: Vec<door::Coming> = read
        .iter()
        .map(|(pid, start, words, scope)| {
            let heavy = door::heavy(&words.split_whitespace().collect::<Vec<_>>());
            door::Coming {
                pid: *pid,
                start: *start,
                words: words.split_whitespace().collect(),
                scope: scope.clone(),
                // Only heavy work is weighed by what its scope holds, and
                // asked whether it lands a switch.
                memory: if heavy { door::memory(scope) } else { 0 },
                first: heavy
                    && fs::read(format!("/proc/{pid}/environ"))
                        .is_ok_and(|environ| door::marked_first(&environ)),
                frozen: heavy
                    && fs::read(scope.join("cgroup.freeze"))
                        .is_ok_and(|state| state.starts_with(b"1")),
                blocking: blocking.contains(pid),
            }
        })
        .collect();
    for done in doorway.door.look(&coming, machine, now, uptime, door::act) {
        match done {
            door::Done::Held(pid) => {
                door::tell(pid, machine);
                let call = if machine.calling { ", a call on" } else { "" };
                eprintln!("Held {pid} at the door, with {free} bytes available{call}.");
            }
            door::Done::LetIn(pid) => eprintln!("Let {pid} in."),
            door::Done::Ranked(pid, high) => eprintln!(
                "Let {pid} in behind other work, its memory held under {high} bytes, with \
                 {free} bytes available."
            ),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("jobs-sorter: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings {
            under: vec!["app.slice".into()],
            homes: vec!["kitty-".into()],
            keep: vec!["pi".into()],
            jobs: "jobs.slice/jobs.service".into(),
            busy_ms: 500,
            ticks: 2,
            every: Duration::from_millis(1000),
            split: vec!["jobs.slice/jobs.service".into()],
            no_real_time: vec!["jobs.slice/jobs-work.slice".into()],
            by_quest: vec!["jobs-work.slice".into()],
            quests: None,
            background: None,
            low_weight: 10,
            weigh: Vec::new(),
            revive: None,
            orchestrators: None,
            door: None,
            calls: None,
            niri: None,
            hosts: None,
        }
    }

    #[expect(clippy::similar_names, reason = "named as /proc names them")]
    fn proc(pid: u32, ppid: u32, pgrp: u32, comm: &str, cpu: u64) -> Sample {
        Sample {
            pid,
            ppid,
            pgrp,
            comm: comm.into(),
            cpu,
            start: 0,
        }
    }

    /// A terminal: zsh (started by kitty, pid 1), pi under it, and a build.
    fn window(build_cpu: u64, pi_cpu: u64) -> Vec<Sample> {
        vec![
            proc(10, 1, 10, "zsh", 5),
            proc(20, 10, 20, "pi", pi_cpu),
            proc(30, 10, 30, "make", 1),
            proc(31, 30, 30, "cc1", build_cpu),
        ]
    }

    /// Runs looks a second apart, each with the processes it's given.
    fn looks(sorter: &mut Sorter, frames: &[Vec<Sample>]) -> Vec<Vec<Move>> {
        let settings = settings();
        frames
            .iter()
            .map(|procs| {
                let now = sorter.last + 100;
                let moves = sorter.look("kitty-1-1.scope", procs, now, &settings);
                sorter.finish(&procs.iter().map(|proc| proc.pid).collect(), now);
                moves
            })
            .collect()
    }

    #[test]
    fn a_pi_counts_as_starting_for_thirty_seconds() {
        let mut pi = proc(20, 1, 20, "pi", 0);
        pi.start = 10_000;
        assert!(starting(&[pi.clone()], 10_000));
        assert!(starting(&[pi.clone()], 12_999));
        assert!(!starting(&[pi.clone()], 13_000));
        assert!(!starting(&[], 10_000));
    }

    #[test]
    fn a_stat_line_is_read_past_a_name_with_parentheses() {
        let line = "1234 (we (ird) name) S 1 1234 1234 0 -1 4194560 100 0 0 0 \
                    70 30 5 2 20 0 1 0 98765 1000 100";
        let sample = parse_stat(line).unwrap();
        assert_eq!(sample.pid, 1234);
        assert_eq!(sample.comm, "we (ird) name");
        assert_eq!((sample.ppid, sample.pgrp), (1, 1234));
        assert_eq!(sample.cpu, 107);
        assert_eq!(sample.start, 98765);
    }

    #[test]
    fn the_manager_is_found_in_the_cgroup_path() {
        let own = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/x.service\n";
        assert_eq!(
            manager_of(own).as_deref(),
            Some("/user.slice/user-1000.slice/user@1000.service")
        );
        assert_eq!(manager_of("0::/system.slice/x.service\n"), None);
    }

    #[test]
    fn a_build_busy_for_two_seconds_moves_whole() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        assert!(moves[0].is_empty() && moves[1].is_empty());
        assert_eq!(moves[2].len(), 1);
        let mut pids = moves[2][0].pids.clone();
        pids.sort_unstable();
        assert_eq!(pids, vec![30, 31]);
    }

    #[test]
    fn a_build_that_rests_starts_its_count_again() {
        let mut sorter = Sorter::default();
        let moves = looks(
            &mut sorter,
            &[window(0, 0), window(90, 0), window(90, 0), window(180, 0)],
        );
        assert!(moves.iter().all(Vec::is_empty));
    }

    #[test]
    fn a_short_burst_stays() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 0), window(95, 0)]);
        assert!(moves.iter().all(Vec::is_empty));
    }

    #[test]
    fn the_shell_and_pi_never_move() {
        let busy = |cpu: u64| vec![proc(10, 1, 10, "zsh", cpu), proc(20, 10, 20, "pi", cpu)];
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[busy(0), busy(90), busy(180), busy(270)]);
        assert!(moves.iter().all(Vec::is_empty));
    }

    #[test]
    fn a_heavy_process_under_pi_moves_alone_with_its_children() {
        let frame = |cpu: u64| {
            vec![
                proc(10, 1, 10, "zsh", 0),
                proc(20, 10, 20, "pi", 0),
                proc(21, 20, 20, "tsserver", cpu),
                proc(22, 21, 20, "typings", 0),
                proc(23, 20, 20, "git", 0),
            ]
        };
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[frame(0), frame(90), frame(180)]);
        assert_eq!(moves[2].len(), 1);
        assert_eq!(moves[2][0].pids, vec![21, 22]);
    }

    #[test]
    fn a_process_first_seen_counts_only_after_a_look() {
        // pid 31 was running before this look and is seen for the first
        // time with a lot of CPU behind it, all of which may be old.
        let mut sorter = Sorter {
            last: 1000,
            ..Sorter::default()
        };
        let mut old = window(5000, 0);
        for proc in &mut old {
            proc.start = 10;
        }
        let moves = looks(&mut sorter, &[old.clone()]);
        assert_eq!(moves[0], [] as [Move; 0]);
        assert_eq!(sorter.busy.len(), 0);
    }

    /// An agent's shell in jobs (pid 5) ran `systemd-run --scope`, whose
    /// command (40) started a child (41) inside the scope.
    fn scope() -> Vec<Sample> {
        vec![proc(40, 5, 40, "nix", 0), proc(41, 40, 40, "nix", 0)]
    }

    #[test]
    fn a_scope_started_from_jobs_goes_back_to_its_starter() {
        let back = returns_to(
            &scope(),
            |pid| (pid == 5).then(|| "/jobs.slice/jobs.service".to_owned()),
            "/jobs.slice",
        );
        assert_eq!(back.as_deref(), Some("/jobs.slice/jobs.service"));
    }

    #[test]
    fn a_scope_started_from_a_terminal_stays() {
        let starter = |pid: u32| (pid == 5).then(|| "/app.slice/kitty-1-1.scope".to_owned());
        assert_eq!(returns_to(&scope(), starter, "/jobs.slice"), None);
        let lookalike = |pid: u32| (pid == 5).then(|| "/jobs.slicer/x.service".to_owned());
        assert_eq!(returns_to(&scope(), lookalike, "/jobs.slice"), None);
    }

    #[test]
    fn a_scope_with_limits_of_its_own_is_left_alone() {
        let defaults = |file: &str| match file {
            "cpu.max" => Some("max 100000\n".to_owned()),
            "cpu.weight" => Some("100\n".to_owned()),
            "io.max" => Some(String::new()),
            "pids.max" => Some("76479\n".to_owned()),
            _ => Some("max\n".to_owned()),
        };
        assert!(unlimited(defaults));
        assert!(unlimited(|_| None));
        let capped = |file: &str| (file == "memory.max").then(|| "1073741824\n".to_owned());
        assert!(!unlimited(capped));
        let quota = |file: &str| (file == "cpu.max").then(|| "50000 100000\n".to_owned());
        assert!(!unlimited(quota));
    }

    #[test]
    fn only_what_is_still_in_the_cgroup_is_moved() {
        // 41 put itself in a scope of its own since the sample; 42 exited.
        let now = |pid: u32| match pid {
            40 | 43 => Some("/jobs.slice/jobs.service".to_owned()),
            41 => Some("/app.slice/kept.scope".to_owned()),
            _ => None,
        };
        assert_eq!(
            still_in(&[40, 41, 42, 43], "jobs.slice/jobs.service", now),
            [40, 43]
        );
        let lookalike = |_| Some("/jobs.slice/jobs.service-x".to_owned());
        assert_eq!(
            still_in(&[40], "jobs.slice/jobs.service", lookalike),
            [] as [u32; 0]
        );
    }

    #[test]
    fn a_group_whose_leader_left_can_still_move() {
        let frame = |cpu: u64| vec![proc(10, 1, 10, "zsh", 0), proc(31, 1, 30, "cc1", cpu)];
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[frame(0), frame(90), frame(180)]);
        assert_eq!(moves[2].len(), 1);
        assert_eq!(moves[2][0].pids, vec![31]);
    }

    // jobs.service as two sessions' commands leave it: the holder, a shell
    // pi 900 wrote there with the build it started, a subagent's pi with
    // its own shell, and an orphan the user manager took in.
    fn jobs_service() -> Vec<Sample> {
        vec![
            proc(5, 2, 5, "sleep", 0),
            proc(40, 900, 40, "bash", 1),
            proc(41, 40, 40, "cargo", 90),
            proc(42, 41, 40, "rustc", 300),
            proc(50, 901, 50, "pi", 20),
            proc(51, 50, 51, "bash", 1),
            proc(60, 2, 60, "git", 5),
        ]
    }

    #[test]
    fn each_command_written_into_jobs_is_a_piece_with_what_it_started() {
        let pieces = pieces(&jobs_service(), Some(5));
        let trees: Vec<(u32, Vec<u32>)> = pieces
            .iter()
            .map(|piece| (piece.root, piece.pids.clone()))
            .collect();
        assert_eq!(
            trees,
            vec![(40, vec![40, 41, 42]), (50, vec![50, 51]), (60, vec![60])]
        );
        assert_eq!(pieces[0].comm, "bash");
        assert_eq!(pieces[0].ppid, 900);
    }

    #[test]
    fn a_tree_gets_its_scope_once_it_has_run_a_second() {
        assert!(!settled(1000, Some(1000)));
        assert!(!settled(1000, Some(1099)));
        assert!(settled(1000, Some(1100)));
        assert!(settled(1000, None));
    }

    #[test]
    fn the_holder_stays_however_it_looks() {
        let roots: Vec<u32> = pieces(&jobs_service(), Some(5))
            .iter()
            .map(|piece| piece.root)
            .collect();
        assert!(!roots.contains(&5), "{roots:?}");
        // An orphan shares the holder's parent, so only the unit's word
        // tells them apart.
        assert!(roots.contains(&60), "{roots:?}");
    }

    #[test]
    fn a_work_scope_goes_in_the_slice_its_unit_is_in() {
        assert_eq!(
            slice_and_unit("jobs.slice/jobs.service"),
            Some(("jobs.slice/jobs.service", "jobs.slice", "jobs.service"))
        );
        assert_eq!(
            slice_and_unit("jobs.slice/jobs-awaited.slice/awaited.service"),
            Some((
                "jobs.slice/jobs-awaited.slice/awaited.service",
                "jobs-awaited.slice",
                "awaited.service"
            ))
        );
        assert_eq!(slice_and_unit("jobs.service"), None);
        assert_eq!(slice_and_unit("jobs.slice/"), None);
    }

    #[test]
    fn a_work_scope_goes_in_the_slice_named_for_it() {
        assert_eq!(
            slice_and_unit("jobs.slice/jobs.service=jobs-work.slice"),
            Some(("jobs.slice/jobs.service", "jobs-work.slice", "jobs.service"))
        );
        assert_eq!(slice_and_unit("jobs.slice/jobs.service=work"), None);
    }

    #[test]
    fn only_its_own_scopes_take_a_straggler() {
        assert!(made_here("/jobs.slice/work-40-1234.scope"));
        assert!(made_here("/jobs.slice/jobs-awaited.slice/work-7-1.scope"));
        assert!(!made_here("/jobs.slice/jobs.service"));
        assert!(!made_here("/app.slice/kitty-1-2.scope"));
        assert!(!made_here("/jobs.slice/work-40-1234.scope/inner"));
    }

    #[test]
    fn a_scope_is_asked_for_with_its_slice_every_process_a_short_stop_and_no_trace_if_it_fails() {
        let args = scope_call("work-40-7.scope", "jobs.slice", "bash (40)", &[40, 41, 42]);
        let tail: Vec<&str> = args[6..].iter().map(String::as_str).collect();
        assert_eq!(
            tail,
            vec![
                "StartTransientUnit",
                "ssa(sv)a(sa(sv))",
                "work-40-7.scope",
                "fail",
                "6",
                "Slice",
                "s",
                "jobs.slice",
                "Description",
                "s",
                "bash (40)",
                "CollectMode",
                "s",
                "inactive-or-failed",
                "SendSIGHUP",
                "b",
                "true",
                "TimeoutStopUSec",
                "t",
                "5000000",
                "PIDs",
                "au",
                "3",
                "40",
                "41",
                "42",
                "0",
            ]
        );
        assert_eq!(&args[..3], ["--user", "--quiet", "call"]);
    }
}
