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
//! The threads of each `--quick-name` program under a `--quick` cgroup,
//! Joel's Chrome and Slack in his programs' slice, ask for a short slice, so a
//! woken drawing thread runs sooner under load, as `src/quick.rs` says why
//! and how.
//!
//! Work split into a `--by-quest` slice goes in its quest's slice there,
//! weighted as the `--quests` file says, which `src/quests.rs` says why and
//! how. With `--tiers` and `--parents`, each quest's slice sits inside its
//! tier's, as `src/tiers.rs` says why and how.
//!
//! With `--background`, a window, a `--home` or `--weigh` scope, whose
//! every pi is marked there as one nobody is at runs at `--low-weight`
//! until a mark goes, as `src/low.rs` says why and how.
//!
//! With `--door`, a new Nix evaluation or build under a cgroup waits while
//! memory is short, and through a call `--calls`' pw-dump hears, as
//! `src/door.rs` and `src/calls.rs` say why and how. Through such a call,
//! build.slice and jobs.slice are held to a quarter of the processors, as
//! `src/brake.rs` says why and how, and through a call on another of the
//! person's machines too, which `--calls-elsewhere`' mycelium hears, as
//! `src/elsewhere.rs` says.
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
//! `src/hosts.rs` says why and how, and with `--conversations` and
//! `--slots` too, so does each unit a host runs one conversation in. With
//! `--hosts` alone, each one's busy
//! work goes to jobs as a terminal's does, as `looked_at` says why.
//!
//! With `--sessions` and `--weigh`, a session's holder found in jobs goes
//! back to a scope of its own in that slice, with the pi it runs, as
//! `src/sessions.rs` says why and how.
//!
//! Terminals sit in each `--under` slice: Joel's apps' and, apart from
//! them, the agents' sessions'. A terminal's busy work goes to `--jobs`,
//! or to the cgroup named after `=` in its slice's `--under`, as the
//! agents' sessions' does: their busy work is agents' work, and on 12
//! November, sent where Joel's own goes, about 1,700 groups of it took his
//! polkit answers and his weight, sessions' mycelium holders and pis among
//! them.
//!
//! Its help, refusals and exits are the terminal kit's, and `jobs-sorter
//! --help` names every flag (src/cli.rs): asked wrongly ends 64, and a
//! sorter that can't read or sort the cgroups 1.

mod brake;
mod budget;
mod calls;
mod cli;
mod door;
mod elsewhere;
mod focus;
mod holds;
mod hosts;
mod joins;
mod keep;
mod kitty;
mod low;
mod oom;
mod own;
mod probe;
mod quests;
mod quick;
mod real_time;
mod revive;
mod sessions;
mod strays;
mod swap_in;
mod tiers;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

use terminal_kit::{Exit, Failure, flags};

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
    /// Cgroups whose `quick_names` programs ask for a short slice.
    quick: Vec<String>,
    quick_names: Vec<String>,
    by_quest: Vec<String>,
    quests: Option<PathBuf>,
    tiers: Option<PathBuf>,
    parents: Option<PathBuf>,
    tier_order: tiers::Order,
    background: Option<PathBuf>,
    low_weight: u64,
    weigh: Vec<String>,
    revive: Option<String>,
    orchestrators: Option<PathBuf>,
    door: Option<String>,
    calls: Option<PathBuf>,
    /// The folder of mycelium nodes' sockets, and the mycelium to ask them.
    calls_elsewhere: Option<(PathBuf, PathBuf)>,
    niri: Option<PathBuf>,
    hosts: Option<String>,
    /// The prefix of the units a host runs each conversation in, and the
    /// folder of the hosts' slots that names each one's session.
    conversations: Option<String>,
    slots: Option<PathBuf>,
    /// The slice a session found in jobs goes back to.
    sessions: Option<String>,
    /// Where each `--under` slice's terminals send their busy work, when
    /// not to `jobs`, in the order the slices were named.
    sends: Vec<Option<String>>,
}

impl Settings {
    /// Each watched slice, with the cgroup its terminals' busy work goes to.
    fn places(&self) -> Vec<(&str, &str)> {
        self.under
            .iter()
            .zip(&self.sends)
            .map(|(slice, sends)| (slice.as_str(), sends.as_deref().unwrap_or(&self.jobs)))
            .collect()
    }

    /// The prefixes of every window's scope: the terminals' (`--home`) and
    /// the sessions' held apart from them (`--weigh`).
    fn windows(&self) -> Vec<String> {
        [self.homes.as_slice(), &self.weigh].concat()
    }
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
    /// The scope and the group or process it was found as, which a
    /// refusal is said once for ([`Sorter::said`]).
    key: (String, u32),
    /// What it is and where from, as the journal names it.
    what: String,
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
    /// Groups and processes whose move to jobs failed, each said once
    /// while it runs: a setuid program's, with cores held.
    refused: HashSet<(String, u32)>,
    /// What's been said of scopes moved back to where they belong.
    returns: Returns,
}

/// What's been said of scopes moved back to where they belong, by name,
/// each forgotten once it's no longer found: a stray to what started it,
/// and a session out of jobs to a scope of its own.
#[derive(Debug, Default)]
struct Returns {
    /// Strays whose move back failed, said once while they stay.
    strays: HashSet<String>,
    /// Sessions whose move into their scope failed, said once while
    /// they stay.
    sessions: HashSet<String>,
    /// Sessions given a scope through systemd, which fills it after it
    /// answers, so one found in jobs again wasn't filled.
    asked: HashSet<String>,
    /// Those asked for and found in jobs again, said once while they stay.
    stuck: HashSet<String>,
    /// Sessions systemd gave no scope, with none there to move into, said
    /// once while they stay.
    unmade: HashSet<String>,
}

impl Returns {
    /// Forgets what was said of strays no longer found, so one that comes
    /// back is new.
    fn strays_found(&mut self, names: &HashSet<String>) {
        self.strays.retain(|name| names.contains(name));
    }

    /// Forgets what was said of sessions no longer found in jobs.
    fn sessions_found(&mut self, names: &HashSet<String>) {
        for said in [
            &mut self.sessions,
            &mut self.asked,
            &mut self.stuck,
            &mut self.unmade,
        ] {
            said.retain(|name| names.contains(name));
        }
    }

    /// What to say of asking systemd for `name`'s scope: `given` the first
    /// time; `still` once, when it's still in jobs on a later look; and
    /// nothing after, while it stays.
    fn asked(
        &mut self,
        name: &str,
        given: impl FnOnce() -> String,
        still: impl FnOnce() -> String,
    ) -> Option<String> {
        if self.asked.insert(name.to_owned()) {
            return Some(given());
        }
        self.stuck.insert(name.to_owned()).then(still)
    }

    /// What to say of `name` when systemd made it no scope and none was
    /// there: once while it stays, with a mark of its own, so a move into
    /// a scope found later still says why it failed.
    fn unmade(&mut self, name: &str) -> Option<String> {
        self.unmade.insert(name.to_owned()).then(|| {
            format!(
                "Couldn't give {name} its scope back from jobs, as systemd made it none and \
                 none was there. It stays in jobs, and isn't said again while it does."
            )
        })
    }
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
                    // A name is the program's own to choose, so one that
                    // holds a newline can't start a journal line of its own.
                    let name = leader.map_or("a group".to_owned(), |leader| {
                        leader.comm.escape_debug().to_string()
                    });
                    let pids = whole(members, procs);
                    let what = format!(
                        "{name} (process group {pgrp}, {} processes) from {scope}",
                        pids.len()
                    );
                    moves.push(Move {
                        pids,
                        key,
                        says: format!(
                            "Moved {what} to jobs: {} ms of CPU in {span_ms} ms.",
                            total * MS_PER_TICK
                        ),
                        what,
                    });
                } else if total * MS_PER_TICK >= busy_after {
                    still.insert(key);
                }
                continue;
            }
            // One named by `--keep` stays alone too, as a host's git
            // engine does, which pi keeps beside it for its status line.
            for proc in members
                .iter()
                .filter(|proc| proc.pid != *pgrp && !settings.keep.contains(&proc.comm))
            {
                let key = (scope.to_string(), proc.pid);
                if used[&proc.pid] * MS_PER_TICK < busy_after {
                    continue;
                }
                if self.streak(&key, settings) {
                    let what = format!(
                        "{} ({}) and what it started from {scope}",
                        proc.comm.escape_debug(),
                        proc.pid
                    );
                    moves.push(Move {
                        pids: descendants(proc.pid, procs),
                        key,
                        says: format!(
                            "Moved {what} to jobs: {} ms of CPU in {span_ms} ms.",
                            used[&proc.pid] * MS_PER_TICK
                        ),
                        what,
                    });
                } else {
                    still.insert(key);
                }
            }
        }
        // A streak breaks the moment its group or process rests.
        self.busy
            .retain(|key, _| key.0 != scope || still.contains(key));
        // A refusal is forgotten once its group or process has gone from
        // the scope, which a group's key outlives its leader for.
        self.refused
            .retain(|key| key.0 != scope || groups.contains_key(&key.1) || here.contains(&key.1));
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

    /// What to say of `found` once its processes were written as `sent`
    /// says: the move, when any went; when none did, that it failed, once
    /// for each group or process while it runs, since it's tried again
    /// each time it has been busy long enough; and nothing when every one
    /// had ended.
    fn said(&mut self, found: &Move, sent: &Sent) -> Option<String> {
        if sent.moved > 0 {
            self.refused.remove(&found.key);
        }
        let Some((pid, error)) = &sent.failed else {
            return (sent.moved > 0).then(|| found.says.clone());
        };
        if sent.moved > 0 {
            return Some(format!(
                "{} Only {} went: {} stayed where they were, as moving {pid} failed: {error}.",
                found.says, sent.moved, sent.stayed
            ));
        }
        self.refused.insert(found.key.clone()).then(|| {
            format!(
                "Couldn't move {} to jobs, as moving {pid} failed: {error}. It stays where \
                 it is, and isn't said again while it runs, unless some of it moves.",
                found.what
            )
        })
    }

    /// Looks at each scope under each of `under`, moving what's busy into
    /// the `sends` beside it, and says which processes it saw.
    fn sort(
        &mut self,
        under: &[PathBuf],
        sends: &[PathBuf],
        now: u64,
        settings: &Settings,
    ) -> HashSet<u32> {
        let mut alive = HashSet::new();
        let mut scopes = HashSet::new();
        for (slice, jobs) in under.iter().zip(sends) {
            for scope in looked_at(slice, &settings.homes, settings.hosts.as_deref()) {
                let procs = samples(&scope);
                alive.extend(procs.iter().map(|proc| proc.pid));
                let name = scope.file_name().unwrap_or_default().to_string_lossy();
                self.move_busy(&name, &procs, jobs, now, settings);
                scopes.insert(name.into_owned());
            }
        }
        self.forget_unlooked(&scopes);
        alive
    }

    /// Forgets the refusals of scopes no longer looked at, whose names
    /// a later scope could take.
    fn forget_unlooked(&mut self, scopes: &HashSet<String>) {
        self.refused.retain(|key| scopes.contains(&key.0));
    }

    /// Moves what one scope's look finds busy into `jobs`, saying what
    /// came of each.
    fn move_busy(
        &mut self,
        scope: &str,
        procs: &[Sample],
        jobs: &Path,
        now: u64,
        settings: &Settings,
    ) {
        for says in self.busy_moved(scope, procs, jobs, now, settings) {
            eprintln!("{says}");
        }
    }

    /// What moving what one scope's look finds busy into `jobs` came to,
    /// as the journal says it.
    fn busy_moved(
        &mut self,
        scope: &str,
        procs: &[Sample],
        jobs: &Path,
        now: u64,
        settings: &Settings,
    ) -> Vec<String> {
        self.look(scope, procs, now, settings)
            .iter()
            .filter_map(|found| self.said(found, &send(jobs, &found.pids)))
            .collect()
    }

    /// Forgets processes that have gone, and notes when this look was.
    fn finish(&mut self, alive: &HashSet<u32>, now: u64) {
        self.seen.retain(|pid, _| alive.contains(pid));
        self.last = now;
    }
}

/// A group's processes and every process under them in the scope, so
/// what one started in a group of its own, as Nix's ssh to a builder is,
/// goes with it.
fn whole(members: &[&Sample], procs: &[Sample]) -> Vec<u32> {
    let mut found: Vec<u32> = Vec::new();
    for member in members {
        for pid in descendants(member.pid, procs) {
            if !found.contains(&pid) {
                found.push(pid);
            }
        }
    }
    found
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
    /// The prefix of the units a host runs each conversation in, and the
    /// folder of its slots, when they're ranked beside it.
    conversations: Option<(&'a str, &'a Path)>,
    /// The prefixes of the scopes a session window's pi runs in beside it
    /// (`--weigh`), held with the window ([`focus::attached_to`]).
    pis: &'a [String],
}

/// How each window ranks, as last set: its weight, whether it's kept as
/// an orchestrator's, and whether it's held as the one Joel is at.
#[derive(Default)]
struct Ranks {
    weighing: low::Weighing,
    keeping: keep::Keeping,
    holding: holds::Holding,
    /// The windows kept in memory beside those held ([`focus::KEEP`]).
    in_memory: holds::Holding,
    /// Each held or kept window's scope, by niri's id for it, or when it was last
    /// looked for and not found.
    found: HashMap<u64, Result<PathBuf, std::time::Instant>>,
    /// The scope of each held or kept window's pi, by niri's id for it,
    /// with when it was last looked for and how many looks in a row found
    /// none ([`pi_of`]).
    pis: HashMap<u64, (Option<PathBuf>, std::time::Instant, u8)>,
    /// The window with the focus whose memory was last asked back, so
    /// it's asked once each time the focus comes to it ([`swap_in`]).
    reading: Option<u64>,
    /// Each pi host's rank, as last set.
    hosts: hosts::Hosts,
    /// What fitting the units' floors into their slices has said.
    fitting: Fitting,
    /// The sessions viewers attach to that no slot holds, as the looks
    /// before found them, so each is said once while it lasts.
    unslotted: Unslotted,
    /// The orchestrators each pid holds, as the presences last said.
    orchestrating: HashMap<u32, usize>,
    /// The windows Joel was at within [`keep::SPARED_FOR`], by unit, which
    /// keep the account's adjustment for the OOM killer.
    spared: HashSet<String>,
}

/// The windows a sorter before this one held, to give back those not
/// wanted now, and which of them it held with a hard floor, as their
/// memory.min says; none while focus isn't followed, as nothing would
/// give them back ([`holds::Holding::left`]).
fn left_held(
    following: Option<&focus::Following>,
    under: &[PathBuf],
    prefixes: &[String],
    pis: &[String],
) -> holds::Holding {
    if following.is_none() {
        return holds::Holding::default();
    }
    held_before(under, prefixes, pis)
}

/// The holds a sorter before this one left on the window scopes under
/// `under`, as their memory.low and memory.min read, said once, so a
/// count of hard floors after a restart can be checked against the
/// journal.
fn held_before(under: &[PathBuf], prefixes: &[String], pis: &[String]) -> holds::Holding {
    let scopes = under.iter().flat_map(|slice| homes(slice, prefixes));
    let min = |scope: &Path| -> Option<u64> {
        fs::read_to_string(scope.join("memory.min"))
            .ok()
            .and_then(|value| value.trim().parse().ok())
    };
    // memory.low is read as fitting the floors reads it, so the file has one meaning here.
    let holding = holds::Holding::left(scopes, memory_low, min, pis);
    if holding.held() > 0 {
        eprintln!(
            // A held window holds two scopes, its own and its pi's, so these
            // count scopes, and a hard one may be one whose floor can't be read.
            "Took up {} scopes' holds from the sorter before, {} of them hard.",
            holding.held(),
            holding.hard()
        );
    }
    holding
}

/// The windows a sorter before this one kept in memory, as their
/// writeback off says, to give back those not wanted now
/// ([`focus::KEEP`]); a held window's is off too, and is wanted while it's
/// held.
fn left_in_memory(
    following: Option<&focus::Following>,
    under: &[PathBuf],
    prefixes: &[String],
) -> holds::Holding {
    if following.is_none() {
        return holds::Holding::default();
    }
    let scopes = under.iter().flat_map(|slice| homes(slice, prefixes));
    holds::Holding::taken_up(scopes, |scope| {
        fs::read_to_string(scope.join("memory.zswap.writeback")).is_ok_and(|on| on.trim() == "0")
    })
}

/// Whether windows Joel left are kept in memory, unless the file at
/// [`focus::KEEP_OFF`] under the account's config folder says not.
fn keeping_in_memory() -> bool {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    config.is_none_or(|config| !config.join(focus::KEEP_OFF).exists())
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
/// niri's focus says, keeps in memory each he was at within
/// [`focus::KEEP`], gives back each that aged out, and says which units
/// are held. A failure is said once, and tried again on the next look.
fn hold_windows(windows: &Windows, ranks: &mut Ranks) -> Vec<String> {
    let Some(following) = windows.focus else {
        return Vec::new();
    };
    let now = std::time::Instant::now();
    let wanted = following.held(now);
    let in_memory = if keeping_in_memory() {
        following.kept(now)
    } else {
        Vec::new()
    };
    let looked_for = |id: &u64| {
        wanted
            .iter()
            .chain(&in_memory)
            .any(|window| window.id == *id)
    };
    ranks.found.retain(|id, _| looked_for(id));
    ranks.pis.retain(|id, _| looked_for(id));
    let mut scopes: Option<Vec<PathBuf>> = None;
    // What each kitty drawing a window not found by its app id says it
    // shows, asked once a look, as its answer runs to megabytes.
    let mut kitties: HashMap<u32, Vec<kitty::Shown>> = HashMap::new();
    let at = following.at().map(|window| window.id);
    let mut looked = Vec::new();
    let each = wanted
        .iter()
        .map(|window| (window, true))
        .chain(in_memory.iter().map(|window| (window, false)));
    for (window, holds) in each {
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
                let unit = focus::scope_of(window, &started_with(all), windows.pis)
                    .map(str::to_owned)
                    .or_else(|| {
                        let shown = kitties.entry(window.pid).or_insert_with(|| {
                            kitty::me()
                                .and_then(|me| kitty::ask(window, me).ok())
                                .unwrap_or_default()
                        });
                        kitty::named(window, at == Some(window.id), shown)
                    });
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
        let pi = pi_of(window, known.as_deref(), windows, &mut ranks.pis, now);
        looked.push((holds, known, pi));
    }
    // A window's pi ran in its window's own scope when keeping was
    // measured, on 11 November, so keeping its pi keeps what was.
    let (held, kept) = focus::held_and_kept(looked);
    let own = |id: u64| match ranks.found.get(&id) {
        Some(Ok(scope)) => Some(scope.as_path()),
        _ => None,
    };
    let pi = |id: u64| ranks.pis.get(&id).and_then(|(pi, _, _)| pi.as_deref());
    let spared = focus::spared(&following.within(now, keep::SPARED_FOR), own, pi)
        .into_iter()
        .filter_map(|scope| unit_of(scope).map(str::to_owned))
        .collect();
    ranks.spared = spared;
    set_holds(&held, &kept, windows.pis, ranks);
    set_in_memory(&kept, ranks);
    read_back(following.at(), ranks);
    held.iter()
        .filter_map(|scope| Some(scope.file_name()?.to_str()?.to_owned()))
        .collect()
}

/// The scope of the pi of `window`, whose own scope is `own`, as last
/// found while it stands ([`focus::stands`]), or looked for again. A
/// session's window with none is said once ([`focus::says_alone`]), as a
/// mycelium spelling its command lines anew would leave every pi unheld.
fn pi_of(
    window: &focus::Window,
    own: Option<&Path>,
    windows: &Windows,
    found: &mut HashMap<u64, (Option<PathBuf>, std::time::Instant, u8)>,
    now: std::time::Instant,
) -> Option<PathBuf> {
    let Some(own) = own else {
        found.remove(&window.id);
        return None;
    };
    let last = found.get(&window.id);
    if let Some((pi, when, _)) = last
        && focus::stands(pi.as_deref(), *when, now, Path::exists)
    {
        return pi.clone();
    }
    let before = last.map_or(0, |(_, _, misses)| *misses);
    let pi = beside(own, window.pid, windows);
    let misses = focus::misses(pi.is_some(), before);
    if window.session() && focus::says_alone(misses) {
        eprintln!(
            "Holding {} alone: no pi found beside it.",
            unit_of(own).unwrap_or_default()
        );
    }
    found.insert(window.id, (pi.clone(), now, misses));
    pi
}

/// The scope, in any watched slice, of the pi whose held session a process
/// the window's kitty, `kitty`, started in its scope `own` attaches to or
/// holds ([`focus::kittys_own`]).
fn beside(own: &Path, kitty: u32, windows: &Windows) -> Option<PathBuf> {
    let procs = samples(own);
    let named: Vec<(u32, u32, &str)> = procs
        .iter()
        .map(|proc| (proc.pid, proc.ppid, proc.comm.as_str()))
        .collect();
    let cmdline = |pid: u32| fs::read(format!("/proc/{pid}/cmdline")).ok();
    let ids = focus::window_ids(&named, kitty, cmdline, children);
    focus::pi_scope(&ids, windows.under, windows.pis, Path::exists)
}

/// The processes `pid` started, as each of its threads' children says.
fn children(pid: u32) -> Vec<u32> {
    fs::read_dir(format!("/proc/{pid}/task"))
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|task| fs::read_to_string(task.path().join("children")).ok())
        .flat_map(|children| {
            children
                .split_whitespace()
                .filter_map(|pid| pid.parse().ok())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Holds each of `held` not held yet and gives back each held no longer,
/// keeping writeback off on one given back that's among `kept`.
fn set_holds(held: &[PathBuf], kept: &[PathBuf], pis: &[String], ranks: &mut Ranks) {
    ranks.holding.keep_only(Path::exists);
    set_holds_by(held, pis, ranks, |unit, hard, scope| {
        let what = if hard.is_some() { "hold" } else { "give back" };
        match Command::new("busctl")
            .args(holds::hold_call(
                unit,
                hard,
                kept.iter().any(|kept| kept == scope),
            ))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(format!(
                "Couldn't {what} {unit}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => Err(format!("Couldn't run busctl to {what} {unit}: {error}")),
        }
    });
}

/// Holds each of `held` not held yet and gives back each held no longer
/// through `call`, given each one's unit, the hard floor it's held with or
/// none to give it back, and its scope, and saying why one failed.
fn set_holds_by(
    held: &[PathBuf],
    pis: &[String],
    ranks: &mut Ranks,
    mut call: impl FnMut(&str, Option<u64>, &Path) -> Result<(), String>,
) {
    // A host's give-back that failed leaves its hard floors with the kernel, so they count
    // against the pis' as a pi's own failed give-back does.
    ranks.holding.elsewhere = ranks.hosts.hard_left();
    for (scope, hold) in ranks.holding.changes(held, pis) {
        let unit = scope
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let hard = hold.then(|| ranks.holding.hard_floor(&scope, pis));
        match call(&unit, hard, &scope) {
            Ok(()) => {
                // A failure after this one is said again, so a give-back that keeps failing, as
                // it would leave a hard floor behind, isn't silent.
                ranks.holding.failed = false;
                ranks.holding.done(&scope, hard);
                // An orchestrator's window gave its floor up to this one,
                // so it's kept again from nothing once given back.
                ranks.keeping.done(&unit, keep::Floor::Nothing);
                match hard {
                    Some(0) if holds::hard_for(&scope, pis) > 0 => eprintln!(
                        "Held {unit}, a window Joel is or was just at, with no hard floor while {} \
                         others keep theirs.",
                        focus::MOST
                    ),
                    Some(0) => eprintln!("Held {unit}, a window Joel is or was just at."),
                    Some(hard) => eprintln!(
                        "Held {unit}, a window Joel is or was just at, {} MiB of it hard.",
                        hard >> 20
                    ),
                    None => eprintln!("Gave back {unit}, a window Joel was just at."),
                }
            }
            Err(why) => {
                if !std::mem::replace(&mut ranks.holding.failed, true) {
                    eprintln!("{why}");
                }
            }
        }
    }
}

/// Turns zswap writeback off on each of `kept` that hasn't it off yet,
/// and back on for each kept no longer ([`focus::KEEP`]).
fn set_in_memory(kept: &[PathBuf], ranks: &mut Ranks) {
    ranks.in_memory.keep_only(Path::exists);
    // Keeping a scope in memory holds no floor, so no pi counts here.
    for (scope, keep) in ranks.in_memory.changes(kept, &[]) {
        let unit = scope
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let result = Command::new("busctl")
            .args(holds::keep_call(&unit, keep))
            .stdout(Stdio::null())
            .output();
        match result {
            Ok(output) if output.status.success() => {
                ranks.in_memory.done(&scope, keep.then_some(0));
                let says = if keep {
                    "Keeping in memory"
                } else {
                    "Let go to disk"
                };
                eprintln!("{says} {unit}, a window Joel was at within two hours.");
            }
            Ok(output) => {
                if !std::mem::replace(&mut ranks.in_memory.failed, true) {
                    eprintln!(
                        "Couldn't keep {unit} in memory: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    );
                }
            }
            Err(error) => {
                if !std::mem::replace(&mut ranks.in_memory.failed, true) {
                    eprintln!("Couldn't run busctl to keep {unit} in memory: {error}");
                }
            }
        }
    }
}

/// Asks, once each time the focus comes to a session window whose scope is
/// found, for the swapped memory of the window and its pis to be read back,
/// or to an app's window, such as Slack's or Chrome's, for its own
/// process's and those it started, through the root unit that may
/// (`src/swap_in.rs`), which checks it all again itself.
fn read_back(at: Option<focus::Window>, ranks: &mut Ranks) {
    let Some(window) = at else {
        ranks.reading = None;
        return;
    };
    if ranks.reading == Some(window.id) {
        return;
    }
    // niri names an app window's own process, where a session's runs in a
    // kitty niri doesn't know it by. On 24 November Slack had 857 MiB in
    // swap against 797 resident, and Chrome 703 against 743.
    if !window.session() {
        ranks.reading = Some(window.id);
        start_reading_back(&window.pid.to_string());
        return;
    }
    let Some(Ok(scope)) = ranks.found.get(&window.id) else {
        return;
    };
    ranks.reading = Some(window.id);
    let first = fs::read_to_string(scope.join("cgroup.procs"))
        .ok()
        .and_then(|procs| procs.lines().next().map(str::to_owned));
    // The pi's scope is read back by its holder, which carries the app id
    // of the window that started it, as a launcher's window did, where the
    // pi doesn't, so the reader knows it as the window's; a session
    // attached again in another window carries its first one's, and the
    // reader refuses it. The pi goes first, as it's what Joel waits on.
    let pi = ranks
        .pis
        .get(&window.id)
        .and_then(|(pi, _, _)| pi.as_deref())
        .and_then(|pi| started_by(pi, &window.app_id));
    for pid in pi.into_iter().chain(first) {
        start_reading_back(&pid);
    }
}

/// The first process in `scope` started with `app_id`.
fn started_by(scope: &Path, app_id: &str) -> Option<String> {
    fs::read_to_string(scope.join("cgroup.procs"))
        .ok()?
        .lines()
        .find(|pid| {
            fs::read(format!("/proc/{pid}/environ"))
                .ok()
                .and_then(|environ| focus::app_id(&environ))
                .is_some_and(|id| id == app_id)
        })
        .map(str::to_owned)
}

/// A scope's unit, from its path.
fn unit_of(scope: &Path) -> Option<&str> {
    scope.file_name()?.to_str()
}

/// Starts reading back the swapped memory of `pid`'s scope.
fn start_reading_back(pid: &str) {
    let unit = format!("memory-swap-in@{pid}.service");
    match Command::new("systemctl")
        .args(["start", "--no-block", &unit])
        .stdout(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(output) => eprintln!(
            "Couldn't start {unit}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => eprintln!("Couldn't run systemctl to start {unit}: {error}"),
    }
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
        conversations: _,
        pis: _,
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

/// Ranks each pi host's unit under `prefix`, and each conversation's unit
/// beside it, as its conversations would rank in windows, from the
/// windows `held` as ones Joel is at, the windows showing each, and the
/// orchestrators last counted. A failure is said once, and tried again on
/// the next look.
fn rank_hosts(windows: &Windows, prefix: &str, held: &[String], ranks: &mut Ranks) {
    let units: Vec<PathBuf> = windows
        .under
        .iter()
        .flat_map(|slice| {
            let mut found = hosts::units(slice, prefix);
            if let Some((conversations, _)) = windows.conversations {
                found.extend(hosts::units(slice, conversations));
            }
            found
        })
        .collect();
    let Joined {
        viewers,
        shown,
        unslotted,
    } = viewers_and_shown(windows, prefix, held, &units, &command_lines);
    say_unslotted(&mut ranks.unslotted, unslotted);
    let shares = shares(ranks.holding.hard(), &ranks.hosts, &units, &viewers);
    let now = std::time::Instant::now();
    let total = memory_total();
    let mut seen = Vec::new();
    let mut ranked = Vec::new();
    for path in units {
        let Some(unit) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let procs: Vec<(u32, u32)> = samples(&path)
            .iter()
            .map(|proc| (proc.pid, proc.ppid))
            .collect();
        // An empty unit is a host stopping, so it's left until it starts again.
        if procs.is_empty() {
            continue;
        }
        // One with two roots, as a command its started process left behind makes, still holds
        // what a window shows and the orchestrators its roots run, so every root is counted, and
        // the doubt said once.
        let roots = hosts::roots(&procs);
        let pid = hosts::host_of(&procs);
        if pid.is_none() && ranks.hosts.unclear.insert(unit.to_owned()) {
            eprintln!("Ranking {unit} by all its roots: no one process in it started it.");
        }
        seen.push(unit.to_owned());
        let own = own_of(unit, pid, now, total, ranks);
        let (orchestrators, marked) = hosts::counted(&roots, &ranks.orchestrating, |root| {
            windows.dir.join(root.to_string()).exists()
        });
        let signs = hosts::Signs {
            viewed: viewers.iter().filter(|viewed| *viewed == unit).count(),
            held: shares.get(unit).copied().unwrap_or(0),
            shown: shown.iter().filter(|viewed| *viewed == unit).count(),
            orchestrators,
            marked,
            current: holds::current(&path),
            keeps_its_work: joins::keeps_its_work(
                unit,
                windows
                    .conversations
                    .map(|(conversations, _)| conversations),
            ),
        };
        let rank = hosts::wanted(&own, signs, windows.weight);
        ranked.push(Ranked {
            path: path.clone(),
            asked: rank.floor,
            rank,
            signs,
            own: own.floor,
            own_hard: own.hard,
            writeback: own.writeback,
        });
    }
    fit_ranked(&mut ranked, windows.prefixes, ranks, now);
    let control =
        kitty::me().map(|uid| PathBuf::from(format!("/run/user/{uid}/systemd/user.control")));
    give_ranks(ranked, &mut ranks.hosts, control.as_deref());
    let conversations = windows
        .conversations
        .map(|(conversations, _)| conversations);
    take_back_left_settings(
        control.as_deref(),
        prefix,
        conversations,
        &seen,
        &mut ranks.hosts,
    );
    ranks.hosts.keep_only(&seen);
}

/// Takes back the settings the sorter gave units under `prefix` or
/// `conversations` that no longer run, in the runtime control folder,
/// `control` ([`sweep`]), asking systemctl.
fn take_back_left_settings(
    control: Option<&Path>,
    prefix: &str,
    conversations: Option<&str>,
    seen: &[String],
    hosts: &mut hosts::Hosts,
) {
    let prefixes: Vec<&str> = std::iter::once(prefix).chain(conversations).collect();
    // systemctl answers for every unit even as it leaves with a failure for those not active,
    // so what it says is read whatever its status; an answer short of a unit is refused below.
    let ask = |units: &[String]| {
        let mut asked = Command::new("systemctl")
            .args(own::is_active_call(units))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("couldn't run systemctl: {error}"))?;
        let mut out = asked
            .stdout
            .take()
            .ok_or_else(|| "systemctl gave no output to read".to_owned())?;
        let reading = thread::spawn(move || {
            let mut answer = Vec::new();
            io::Read::read_to_end(&mut out, &mut answer).map(|_| answer)
        });
        // It runs on the look that ranks the units Joel is at, so it's stopped past its time.
        let deadline = std::time::Instant::now() + own::ASK_WITHIN;
        loop {
            match asked.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = asked.kill();
                    let _ = asked.wait();
                    return Err(format!(
                        "systemctl didn't answer within {:?}",
                        own::ASK_WITHIN
                    ));
                }
            }
        }
        let answer = reading
            .join()
            .map_err(|_| "reading systemctl's answer failed".to_owned())?
            .map_err(|error| format!("couldn't read systemctl's answer: {error}"))?;
        Ok(String::from_utf8_lossy(&answer).into_owned())
    };
    let now = std::time::Instant::now();
    sweep(control, &prefixes, seen, hosts, now, ask);
}

/// Takes back the settings in `control` the sorter gave units under
/// `prefixes` that no longer run, which systemd keeps after a unit stops
/// ([`own::left_behind`]): on the first look, those of every unit not
/// `seen` running, and after it, those of the units ranked before that
/// have gone ([`hosts::Hosts::leave`]), as `ask` says of them all in one
/// call. A unit seen has processes, so it runs and isn't asked after.
/// One systemctl says hasn't stopped is asked after again a while later,
/// until it has. Each unit's settings taken back are forgotten, so it's
/// ranked afresh should it start again. A sweep that can't ask is said
/// once and tried again a while later.
fn sweep(
    control: Option<&Path>,
    prefixes: &[&str],
    seen: &[String],
    hosts: &mut hosts::Hosts,
    now: std::time::Instant,
    ask: impl FnOnce(&[String]) -> Result<String, String>,
) {
    hosts.leave(seen, now);
    let due = |later: &HashMap<String, std::time::Instant>, unit: &String| {
        later.get(unit).is_some_and(|due| now >= *due)
    };
    if hosts.sweep.done
        && hosts.sweep.pending.is_empty()
        && !hosts.sweep.later.values().any(|due| now >= *due)
    {
        return;
    }
    if hosts.sweep.held_off.is_some_and(|until| now < until) {
        return;
    }
    let mut say = |why: String| {
        if !std::mem::replace(&mut hosts.sweep.failed, true) {
            eprintln!("Couldn't take back the settings stopped units left: {why}.");
        }
    };
    let Some(control) = control else {
        say("no user id to find the runtime control folder by".to_owned());
        return;
    };
    let left: Vec<String> = match own::left_behind(control, prefixes) {
        Ok(units) => units
            .into_iter()
            .filter(|unit| !seen.contains(unit))
            .collect(),
        Err(error) => {
            say(format!("couldn't read {}: {error}", control.display()));
            return;
        }
    };
    let asked: Vec<String> = left
        .iter()
        .filter(|unit| {
            !hosts.sweep.done
                || hosts.sweep.pending.contains_key(*unit)
                || due(&hosts.sweep.later, unit)
        })
        .cloned()
        .collect();
    // A unit gone with no settings of the sorter's left has nothing to take back.
    hosts.sweep.pending.retain(|unit, _| asked.contains(unit));
    hosts.sweep.later.retain(|unit, _| left.contains(unit));
    if !asked.is_empty() {
        let answer = ask(&asked);
        let Some(states) = answer
            .as_deref()
            .ok()
            .and_then(|stdout| own::states(stdout, asked.len()))
        else {
            say(answer.err().unwrap_or_else(|| {
                format!(
                    "systemctl didn't say whether each of {} units runs",
                    asked.len()
                )
            }));
            hosts.sweep.held_off = Some(now + own::ASK_AGAIN_AFTER);
            return;
        };
        hosts.sweep.held_off = None;
        // Each name was read from a folder anyone in the account can make, so it's said quoted,
        // and a line or a quote in it can't pass for a line of the sorter's own.
        for (unit, state) in asked.into_iter().zip(states) {
            match state {
                own::Active::Stopped => {
                    let (taken, error) = own::forget(control, &unit);
                    hosts.forget(&unit);
                    hosts.sweep.later.remove(&unit);
                    match error {
                        None => {
                            eprintln!(
                                "Took back the {taken} settings {unit:?} left when it stopped."
                            );
                        }
                        Some(error) => {
                            if hosts.sweep.unswept.insert(unit.clone()) {
                                eprintln!(
                                    "Couldn't take back all the settings {unit:?} left, {taken} \
                                     taken: {error}."
                                );
                            }
                        }
                    }
                }
                // systemd keeps a failed unit loaded, its settings' files listed, until it's
                // reset, so they stay.
                own::Active::Failed => {
                    hosts.sweep.pending.remove(&unit);
                    hosts.sweep.later.insert(unit, now + own::ASK_AGAIN_AFTER);
                }
                own::Active::Runs => {
                    hosts.sweep.later.insert(unit, now + own::ASK_AGAIN_AFTER);
                }
            }
        }
    }
    hosts.sweep.done = true;
    hosts.sweep.failed = false;
}

/// Gives each of the `ranked` units the rank it was fitted to, when it
/// differs from the one `hosts` last gave it, through busctl, saying once
/// when it can't ([`give_ranks_by`]). Each unit whose rank lands has its
/// folder in `control`, the runtime control folder, marked as holding
/// settings the sorter set ([`own::mark`]), so they're taken back once it
/// stops, and a mark that can't be left is said once.
fn give_ranks(ranked: Vec<Ranked>, hosts: &mut hosts::Hosts, control: Option<&Path>) {
    give_ranks_marking(ranked, hosts, control, |unit, rank| {
        match Command::new("busctl")
            .args(hosts::rank_call(unit, rank))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => Ok(()),
            Ok(output) => Err(format!(
                "Couldn't rank {unit}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => Err(format!("Couldn't run busctl to rank {unit}: {error}")),
        }
    });
}

/// What [`give_ranks`] does with the ranks `call` gives: each that lands has its unit's folder in
/// `control` marked, so its settings are taken back once the sorter stops, and one that fails
/// leaves it as it was.
fn give_ranks_marking(
    ranked: Vec<Ranked>,
    hosts: &mut hosts::Hosts,
    control: Option<&Path>,
    mut call: impl FnMut(&str, &hosts::Rank) -> Result<(), String>,
) {
    let mut unmarked = None;
    let mut marked = false;
    give_ranks_by(ranked, hosts, |unit, rank| {
        call(unit, rank)?;
        match control.map(|control| own::mark(control, unit)) {
            Some(Ok(())) => marked = true,
            Some(Err(error)) => {
                unmarked.get_or_insert(format!("{unit:?}: {error}"));
            }
            None => {}
        }
        Ok(())
    });
    // A failure after a mark that was left is said again.
    let still = unmarked.is_some() || (hosts.sweep.unmarked && !marked);
    let said = std::mem::replace(&mut hosts.sweep.unmarked, still);
    if let Some(why) = unmarked
        && !said
    {
        eprintln!("Couldn't mark the settings the sorter set on {why}, so they'll stay.");
    }
}

/// Gives each of the `ranked` units the rank it was fitted to, when it
/// differs from the one `hosts` last gave it, with `call`, which says why
/// when it can't; that is said once. Those giving back hard floors go
/// first, and once one not failing already fails, none after it is given
/// more than it keeps: its floors stay with the kernel, and the shares
/// were given as if they had gone, so a fourth would hold until the next
/// look counted them ([`shares`]).
fn give_ranks_by(
    mut ranked: Vec<Ranked>,
    hosts: &mut hosts::Hosts,
    mut call: impl FnMut(&str, &hosts::Rank) -> Result<(), String>,
) {
    let name = |ranked: &Ranked| {
        ranked
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
    };
    ranked.sort_by_key(|unit| {
        name(unit).is_some_and(|name| unit.signs.held > hosts.last_held(&name))
    });
    let mut kept_back = false;
    for mut unit in ranked {
        let Some(name) = name(&unit) else {
            continue;
        };
        let before = hosts.last_held(&name);
        if kept_back && unit.signs.held > before {
            unit.signs.held = before;
            unit.rank.hard = unit
                .own_hard
                .saturating_add(u64::try_from(before).unwrap_or(u64::MAX) * holds::HARD);
        }
        // A failing unit is asked again each look though its rank is the one it was set: capped
        // at what it keeps while failing, it may never differ, and would never land.
        let failing = hosts.kept(&name).is_some();
        if !failing && !hosts.changed(&name, &unit.rank) {
            continue;
        }
        // A unit this sorter never set may keep what a call before left it, which its memory.min
        // says, should its first call here fail.
        let kept = || {
            let min = fs::read_to_string(unit.path.join("memory.min"))
                .ok()
                .and_then(|min| min.trim().parse().ok());
            if holds::in_agents(&unit.path) {
                hosts::kept_hard(min, unit.own_hard)
            } else {
                0
            }
        };
        match call(&name, &unit.rank) {
            Ok(()) => {
                let Ranked {
                    rank, signs, asked, ..
                } = unit;
                // A floor fitted short says what it would have been, so a unit Joel is at
                // left without one shows in the journal.
                let short = if rank.floor < asked {
                    format!(" of the {asked} it would have")
                } else {
                    String::new()
                };
                eprintln!(
                    "Ranked {name} at weight {}, floor {}{short}, hard {}: {} windows Joel's at \
                     of {} showing it, {} orchestrators.",
                    rank.weight,
                    rank.floor,
                    rank.hard,
                    signs.viewed,
                    signs.shown,
                    signs.orchestrators
                );
                // A failure after this one is said again, as a hard floor that can't be given
                // back would otherwise stay silent.
                hosts.failed = false;
                hosts.done(&name, rank, signs.held);
            }
            Err(why) => {
                // Only a unit not failing already: one that was is counted in the shares at what
                // it keeps. What it keeps is read once `fail` has noted it, which for a unit this
                // sorter never set is what the kernel says, not the none it was known to hold.
                hosts.fail(&name, kept);
                kept_back |= !failing && unit.signs.held < hosts.last_held(&name);
                if !std::mem::replace(&mut hosts.failed, true) {
                    eprintln!("{why}");
                }
            }
        }
    }
}

/// A host's or conversation's unit as ranked, before its floor is fitted
/// into what its slice has left.
struct Ranked {
    path: PathBuf,
    rank: hosts::Rank,
    /// The floor it would have before it's fitted.
    asked: u64,
    signs: hosts::Signs,
    /// The floor its unit file sets.
    own: u64,
    /// The hard floor its unit file sets.
    own_hard: u64,
    /// Whether its unit file leaves writeback to the disk's swap on.
    writeback: bool,
}

/// How many of the others an overrun line names ([`Fitting::overran`]),
/// so a slice with many children still says it in one line.
const SAID: usize = 5;

/// What fitting the units' floors into their slices ([`fit_floors`]) has
/// said, so each is said once while it lasts.
#[derive(Default)]
struct Fitting {
    /// The slices whose floor couldn't be read to fit their units' floors
    /// into.
    unfitted: HashSet<String>,
    /// The slices whose others claimed past the room the units Joel is at
    /// left them ([`Fitting::overran`]).
    overrun: HashSet<String>,
    /// Each window's claim at its peak within the minute.
    peaks: budget::Peaks,
    /// The scopes the sorter floors itself, as of this look.
    floored: Floored,
}

/// The scopes the sorter floors itself: the windows it holds, each with its
/// pi, and those keep.rs keeps. Both claim first within the windows' joint
/// cap, those it holds ahead of those it keeps ([`claims`]): those it holds
/// are up to twice [`focus::MOST`] scopes, and more while a give-back
/// fails, and those it keeps are every window running a pi, so neither is
/// bounded by its count.
#[derive(Debug, Default)]
struct Floored {
    held: HashSet<PathBuf>,
    /// Those keep.rs keeps, by unit.
    kept: HashSet<String>,
}

impl Floored {
    /// Takes this look's floors from what `holding` holds and `keeping`
    /// keeps.
    fn take(&mut self, holding: &holds::Holding, keeping: &keep::Keeping) {
        self.held.clone_from(holding.scopes());
        self.kept = keeping.units().map(str::to_owned).collect();
    }

    /// Whether keep.rs keeps `scope`'s floor.
    fn keeps(&self, scope: &Path) -> bool {
        scope
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| self.kept.contains(name))
    }
}

impl Fitting {
    /// What to say of `slice` when the others beside its units, `strays`,
    /// each by name with its claim, claim more in all, `claimed`, than the
    /// room the units Joel is at `left` them: the sorter can't trim what
    /// they claim, so the slice is claimed past its floor and the kernel
    /// shares it among every child again, the windows too. Said once, and
    /// again only once they fit by a [`budget::STEP`] and overrun again, so
    /// others hovering at the edge don't say it on every crossing. Each is
    /// named quoted, the first [`SAID`] of them, since a child's name is
    /// whatever started it chose.
    fn overran(
        &mut self,
        slice: &str,
        strays: &[(String, u64)],
        claimed: u64,
        left: u64,
    ) -> Option<String> {
        if claimed.saturating_add(budget::STEP) <= left {
            self.overrun.remove(slice);
            return None;
        }
        if claimed <= left || !self.overrun.insert(slice.to_owned()) {
            return None;
        }
        let named: Vec<&(String, u64)> = strays.iter().filter(|(_, claim)| *claim > 0).collect();
        let mut which: Vec<String> = named
            .iter()
            .take(SAID)
            .map(|(name, claim)| format!("{name:?} {} MiB", claim >> 20))
            .collect();
        if named.len() > SAID {
            which.push(format!("and {} more", named.len() - SAID));
        }
        Some(format!(
            "The others in {slice} claim {} MiB, past the {} MiB the units Joel is at leave \
             them, so its floor is shared among every child again, the windows too: {}.",
            claimed >> 20,
            left >> 20,
            which.join(", ")
        ))
    }
}

/// What the children of `slice` beside the units at `mine` claim of its
/// floor `low`: the room the windows, those under `prefixes`, leave the
/// units, and each other's claim by name, fitted after the units Joel is
/// at ([`fit_floors`]).
fn claims(
    slice: &Path,
    low: u64,
    prefixes: &[String],
    mine: &[&Path],
    floored: &Floored,
    peaks: &mut budget::Peaks,
    now: std::time::Instant,
) -> (u64, Vec<(String, u64)>) {
    // Only the windows' floors are taken first: any agent can start a scope here with a floor
    // of max, which would leave the units Joel is at none.
    let scopes = homes(slice, prefixes);
    let (mut windows, others): (Vec<PathBuf>, Vec<PathBuf>) = fs::read_dir(slice)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && !mine.contains(&path.as_path()))
        .partition(|path| scopes.contains(path));
    // A child that can't be read has most likely just gone, so it claims nothing, while a unit
    // that can't be read counts as holding its whole floor (`fit_floors`).
    let claim = |path: &PathBuf| {
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some((name, holds::current(path)?, memory_low(path)?))
    };
    // Every scope counts as a window by its name, and together they claim no more than
    // focus::MOST windows at focus::FLOOR, the share the slice's floor is sized for: first
    // those the sorter holds, a window and its pi each, so up to twice focus::MOST and more
    // while a give-back fails; then those keep.rs keeps, every window running a pi; then those
    // named alone; each by name so a look's order is the next one's. Past that they claim as
    // strays do, after the units Joel is at, and aren't held: an agent naming scopes as
    // windows and asking for max would otherwise take the units' floors, or push the sorter's
    // own floors out. Each window claims what it held at
    // its peak within the minute, so a window paging out doesn't hand the units room it takes
    // back a look later, but never past its floor now: one the sorter has let go claims its
    // new floor at once.
    windows.sort_by_cached_key(|path| {
        (
            !floored.held.contains(path),
            !floored.keeps(path),
            path.clone(),
        )
    });
    let mut left = focus::FLOOR.saturating_mul(u64::try_from(focus::MOST).unwrap_or(u64::MAX));
    let mut kept = Vec::new();
    let mut strays: Vec<(String, u64)> = Vec::new();
    for path in &windows {
        let Some((name, current, floor)) = claim(path) else {
            continue;
        };
        let claims = current.min(floor);
        let peak = peaks
            .hold(&path.display().to_string(), claims.min(focus::FLOOR), now)
            .min(floor);
        let held = peak.min(left);
        left -= held;
        kept.push(held);
        strays.push((name, claims.saturating_sub(held)));
    }
    let room = budget::room(low, kept.into_iter().map(|held| (held, held)));
    strays.splice(
        0..0,
        others
            .iter()
            .filter_map(claim)
            .map(|(name, current, floor)| (name, current.min(floor))),
    );
    (room, strays)
}

/// Fits the `ranked` units' floors ([`fit_floors`]) beside the scopes the
/// sorter floors itself as `ranks` holds and keeps them this look.
fn fit_ranked(
    ranked: &mut [Ranked],
    prefixes: &[String],
    ranks: &mut Ranks,
    now: std::time::Instant,
) {
    ranks.fitting.floored.take(&ranks.holding, &ranks.keeping);
    fit_floors(ranked, prefixes, &ranks.hosts, &mut ranks.fitting, now);
}

/// Fits the floors the `ranked` units would be given into what each one's
/// slice has left once the windows beside them keep theirs ([`budget`]),
/// each against the floor `hosts` last set on it: first the units Joel is
/// at, then what the others beside them claim, then the rest. So the
/// windows Joel types in, those under `prefixes`, keep theirs whole while
/// those others fit in what the units Joel is at leave; past it the
/// kernel shares the slice's floor among every child again, which is said
/// once ([`Fitting::overran`]). Each window claims its peak of the last
/// minute at `now` ([`budget::Peaks`]). A slice whose floor can't be read is left
/// as ranked, and said once, as `fitting` remembers.
fn fit_floors(
    ranked: &mut [Ranked],
    prefixes: &[String],
    hosts: &hosts::Hosts,
    fitting: &mut Fitting,
    now: std::time::Instant,
) {
    fitting.peaks.forget_before(now);
    let mut slices: Vec<PathBuf> = ranked
        .iter()
        .filter_map(|unit| unit.path.parent().map(Path::to_path_buf))
        .collect();
    slices.sort();
    slices.dedup();
    // A slice this look doesn't fit forgets it overran, so its next overrun is said.
    let fitted: HashSet<String> = slices.iter().map(|s| s.display().to_string()).collect();
    fitting.overrun.retain(|slice| fitted.contains(slice));
    for slice in slices {
        let name = slice.display().to_string();
        let Some(low) = memory_low(&slice) else {
            if fitting.unfitted.insert(name) {
                eprintln!(
                    "Ranking the units in {} unfitted: its memory.low can't be read.",
                    slice.display()
                );
            }
            continue;
        };
        fitting.unfitted.remove(&name);
        let mine: Vec<usize> = (0..ranked.len())
            .filter(|&at| ranked[at].path.parent() == Some(slice.as_path()))
            .collect();
        let paths: Vec<&Path> = mine.iter().map(|&at| ranked[at].path.as_path()).collect();
        let (room, strays) = claims(
            &slice,
            low,
            prefixes,
            &paths,
            &fitting.floored,
            &mut fitting.peaks,
            now,
        );
        let claimed = strays
            .iter()
            .fold(0u64, |sum, (_, claim)| sum.saturating_add(*claim));
        let signs: Vec<hosts::Signs> = mine.iter().map(|&at| ranked[at].signs).collect();
        let order: Vec<usize> = budget::order(&signs)
            .into_iter()
            .map(|at| mine[at])
            .collect();
        // A unit whose memory can't be read counts as holding its whole floor.
        let mut wanted: Vec<budget::Wanted> = order
            .iter()
            .map(|&at| budget::Wanted {
                own: ranked[at].own,
                added: ranked[at].rank.floor.saturating_sub(ranked[at].own),
                current: ranked[at].signs.current.unwrap_or(u64::MAX),
            })
            .collect();
        // What the others claim is fitted after the units Joel is at and before the rest.
        let viewed = order
            .iter()
            .take_while(|&&at| ranked[at].signs.viewed > 0)
            .count();
        wanted.insert(
            viewed,
            budget::Wanted {
                own: 0,
                added: claimed,
                current: 0,
            },
        );
        let mut fitted = budget::fit(&wanted, room);
        let mut margin = budget::fit(&wanted, room.saturating_sub(budget::STEP));
        // Weighed against the room the units Joel is at leave them, not what fit gave them,
        // which is never more than they claim, so the mark could never clear.
        let left = budget::left_after(&wanted, room, &fitted[..viewed]);
        fitted.remove(viewed);
        margin.remove(viewed);
        if let Some(line) = fitting.overran(&name, &strays, claimed, left) {
            eprintln!("{line}");
        }
        for ((&at, fitted), margin) in order.iter().zip(fitted).zip(margin) {
            let unit = &mut ranked[at];
            let last = unit
                .path
                .file_name()
                .and_then(|name| hosts.floor_set(&name.to_string_lossy()))
                .map(|floor| floor.saturating_sub(unit.own));
            unit.rank.floor = unit
                .own
                .saturating_add(budget::steady(fitted, margin, last));
            unit.rank.writeback = hosts::writeback(unit.writeback, &unit.signs, unit.rank.floor);
        }
    }
}

/// The soft floor the cgroup at `path` keeps, as its memory.low says, with
/// `max` read as the most there is; none when it can't be read.
fn memory_low(path: &Path) -> Option<u64> {
    let text = fs::read_to_string(path.join("memory.low")).ok()?;
    match text.trim() {
        "max" => Some(u64::MAX),
        number => number.parse().ok(),
    }
}

/// The settings `unit`, started by `pid` when one process did, holds of
/// its own, read with `systemctl cat` once a minute and again when its
/// host restarts. An instance's are its template's, so they're read once
/// for them all and kept while any instance runs (`Hosts::keep_only`),
/// rather than read for each every minute.
fn own_of(
    unit: &str,
    pid: Option<u32>,
    now: std::time::Instant,
    total: u64,
    ranks: &mut Ranks,
) -> own::Own {
    let template = own::template(unit);
    let source = template.as_deref().unwrap_or(unit);
    let reader = if template.is_some() {
        0
    } else {
        pid.unwrap_or(0)
    };
    ranks.hosts.own(source, reader, now, || {
        let output = Command::new("systemctl")
            .args(["--user", "cat", "--", source])
            .stderr(Stdio::null())
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| own::own(&String::from_utf8_lossy(&output.stdout), total))
    })
}

/// How many hard floors each of `units` keeps for the `viewers` in the
/// windows Joel is at: the held windows' are shared out across every
/// unit, after the `pis` held hard (`set_holds`), to units in agents.slice
/// alone, whose own hard floor backs them. A unit whose last rank call
/// failed keeps what it was set with until one lands ([`hosts::Hosts::kept`]),
/// so those count against every other unit's, and it's given no more
/// than it keeps: whether its next call lands or fails, the kernel then
/// keeps at most [`focus::MOST`] across them all.
fn shares(
    pis: usize,
    hosts: &hosts::Hosts,
    units: &[PathBuf],
    viewers: &[String],
) -> HashMap<String, usize> {
    let in_agents: Vec<String> = viewers
        .iter()
        .filter(|unit| {
            units.iter().any(|path| {
                path.file_name().and_then(|name| name.to_str()) == Some(unit.as_str())
                    && holds::in_agents(path)
            })
        })
        .cloned()
        .collect();
    let (failing, others): (Vec<String>, Vec<String>) = in_agents
        .into_iter()
        .partition(|unit| hosts.kept(unit).is_some());
    let mut given = holds::shares(pis + hosts.hard_left(), &others);
    for unit in failing {
        let kept = hosts.kept(&unit).unwrap_or(0);
        let share = given.entry(unit).or_insert(0);
        if *share < kept {
            *share += 1;
        }
    }
    given
}

/// What windows' viewers join ([`viewers_and_shown`]).
struct Joined {
    /// The unit for each viewer in a window held, in the order they're held.
    viewers: Vec<String>,
    /// The unit for each viewer in any window.
    shown: Vec<String>,
    /// Each host's key and session a viewer attaches to that no slot holds,
    /// though the host runs conversations apart, so its host is joined in
    /// its place ([`unslotted_session`]).
    unslotted: Vec<(String, String)>,
}

/// The units among `units` each window `held` shows, and each window
/// shows at all, one for each viewer in it, in the order the windows are
/// held, as `cmdlines` gives each scope's processes' words. A window
/// showing a conversation that runs in a unit of its own gives that unit
/// what it gives, and its host nothing.
fn viewers_and_shown(
    windows: &Windows,
    prefix: &str,
    held: &[String],
    units: &[PathBuf],
    cmdlines: &impl Fn(&Path) -> Vec<Vec<String>>,
) -> Joined {
    let names: HashSet<&str> = units
        .iter()
        .filter_map(|path| path.file_name()?.to_str())
        .collect();
    let joins = |words: &[String]| {
        joins::unit_of(words, prefix, windows.conversations, |unit| {
            names.contains(unit)
        })
    };
    let viewing = focus::viewing(held.iter().map(String::as_str), windows.pis);
    let viewers = viewing
        .iter()
        .flat_map(|unit| windows.under.iter().map(move |slice| slice.join(unit)))
        .flat_map(|scope| cmdlines(&scope))
        .filter_map(|words| joins(&words))
        .collect();
    // Every window's viewers, a held one's among them.
    let showing: Vec<(Vec<String>, String)> = windows
        .under
        .iter()
        .flat_map(|slice| homes(slice, windows.prefixes))
        .filter(|scope| {
            scope
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|unit| !focus::runs_a_pi(unit, windows.pis))
        })
        .flat_map(|scope| cmdlines(&scope))
        .filter_map(|words| {
            let unit = joins(&words)?;
            Some((words, unit))
        })
        .collect();
    let mut unslotted: Vec<(String, String)> = showing
        .iter()
        .filter_map(|(words, unit)| unslotted_session(words, unit, windows.conversations, &names))
        .collect();
    let shown = showing.into_iter().map(|(_, unit)| unit).collect();
    unslotted.sort();
    unslotted.dedup();
    Joined {
        viewers,
        shown,
        unslotted,
    }
}

/// The host's key and session a viewer's `words` attach to, as the journal
/// may say them, when it joined its host's `unit` though some of `names`
/// are that host's live conversation units and no slot of the host's holds
/// the session: the conversation Joel views goes unranked.
fn unslotted_session(
    words: &[String],
    unit: &str,
    conversations: Option<(&str, &Path)>,
    names: &HashSet<&str>,
) -> Option<(String, String)> {
    let (conversations, slots) = conversations?;
    if joins::keeps_its_work(unit, Some(conversations)) {
        return None;
    }
    let (key, session) = joins::session(words)?;
    let theirs = format!("{conversations}{key}@");
    if !names.iter().any(|name| name.starts_with(&theirs)) {
        return None;
    }
    // A slot that holds the session though its unit isn't live is a
    // conversation parked, or starting, since the host writes the slot
    // before its unit: that's its life, not drift.
    if joins::conversation(words, conversations, slots, |_| true).is_some() {
        return None;
    }
    // The words are the process's own to choose, so neither starts a
    // journal line of its own nor fills it.
    let said = |word: &str| {
        word.chars()
            .take(SAID_MOST)
            .collect::<String>()
            .escape_debug()
            .to_string()
    };
    Some((said(&key), said(&session)))
}

/// The most of a viewer's host key or session said in the journal, in
/// characters: a session is a 36-character UUID.
const SAID_MOST: usize = 64;

/// The unslotted sessions the looks before found, each a host's key and a
/// session.
#[derive(Default)]
struct Unslotted {
    /// Those the last look found.
    last: HashSet<(String, String)>,
    /// Those said, and found by every look since.
    said: HashSet<(String, String)>,
}

impl Unslotted {
    /// From this look's `now`, the sessions to say are unslotted and those
    /// said before that no longer are. A session is said at its second
    /// look in a row, as [`focus::says_alone`] says a window, so a slot
    /// read as the host rewrites it isn't said; and once, until it clears.
    fn look(&mut self, now: Vec<(String, String)>) -> UnslottedSaid {
        let now: HashSet<(String, String)> = now.into_iter().collect();
        let mut missing: Vec<_> = now
            .iter()
            .filter(|session| self.last.contains(*session) && !self.said.contains(*session))
            .cloned()
            .collect();
        let mut found: Vec<_> = self.said.difference(&now).cloned().collect();
        missing.sort();
        found.sort();
        self.said.retain(|session| now.contains(session));
        self.said.extend(missing.iter().cloned());
        self.last = now;
        (missing, found)
    }
}

/// The sessions a look says are unslotted, and those it says no longer are.
type UnslottedSaid = (Vec<(String, String)>, Vec<(String, String)>);

/// Says what this look found of the unslotted sessions `said` holds.
fn say_unslotted(said: &mut Unslotted, now: Vec<(String, String)>) {
    let (missing, found) = said.look(now);
    for (key, session) in missing {
        eprintln!(
            "Ranking host {key} in place of session {session}'s conversation: no slot holds \
             the session."
        );
    }
    // A session drops out once a slot holds it, no window shows it, or its host runs no
    // conversation unit at all, where the host is still ranked for it, as a host running its
    // conversations itself always is; so the line names all three rather than a rank changed.
    for (key, session) in found {
        eprintln!(
            "Session {session} of host {key} no longer goes unranked: a slot holds it, no window \
             shows it, or its host runs no conversation units."
        );
    }
}

/// The words of each process in `scope`, from its command line.
fn command_lines(scope: &Path) -> Vec<Vec<String>> {
    samples(scope)
        .into_iter()
        .filter_map(|proc| fs::read(format!("/proc/{}/cmdline", proc.pid)).ok())
        .map(|words| {
            words
                .split(|byte| *byte == 0)
                .map(|word| String::from_utf8_lossy(word).into_owned())
                .collect()
        })
        .collect()
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

/// What ranks a pi's quest: the tier each quest is in, and the tiers'
/// order.
struct Ranking<'a> {
    weights: &'a quests::Weights,
    order: &'a tiers::Order,
}

impl Ranking<'_> {
    /// How many tiers below the first `quest` sits; a session on no quest
    /// is unranked.
    fn rank(&self, quest: Option<&str>) -> usize {
        let tier = quest.and_then(|quest| self.weights.tier_of(quest));
        self.order.rank(tier.unwrap_or(tiers::UNRANKED))
    }
}

/// Keeps each window holding an orchestrator, as `dir`'s presences name
/// them, gives back each that no longer holds one, and puts each pi at
/// the adjustment it wants, by its quest's tier as `ranking` says. A
/// failure is said once, and tried again on the next look.
fn keep_windows(
    under: &[PathBuf],
    prefixes: &[String],
    dir: &Path,
    ranking: &Ranking,
    ranks: &mut Ranks,
) {
    let Ranks {
        keeping,
        holding,
        orchestrating,
        spared,
        ..
    } = ranks;
    let Ok(boot) = fs::read_to_string("/proc/sys/kernel/random/boot_id") else {
        return;
    };
    let proc = Path::new("/proc");
    let runs = |pid: u32| proc.join(pid.to_string()).exists();
    let presences = keep::read(dir);
    let orchestrators = keep::orchestrators(&presences, boot.trim(), runs);
    let quests = keep::quests(&presences, boot.trim(), runs);
    // A host holds many sessions under its one pid, so it's counted by
    // each, for `rank_hosts`.
    *orchestrating = hosts::orchestrating(&presences, boot.trim(), runs);
    let mut warned = keeping.failed;
    let windows: Vec<(String, keep::Floor)> = under
        .iter()
        .flat_map(|slice| homes(slice, prefixes))
        .filter_map(|scope| {
            let unit = scope.file_name()?.to_str()?.to_owned();
            let pis: Vec<u32> = samples(&scope)
                .into_iter()
                .filter(|sample| sample.comm == "pi")
                .map(|sample| sample.pid)
                .collect();
            let joels = spared.contains(&unit);
            for &pid in &pis {
                let wanted = keep::adjustment(
                    joels || orchestrators.contains(&pid),
                    ranking.rank(quests.get(&pid).map(String::as_str)),
                );
                keep::adjust(proc, pid, wanted, &mut warned);
            }
            let orchestrates = pis.iter().any(|pid| orchestrators.contains(pid));
            Some((unit, keep::floor_of(pis.len(), orchestrates)))
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
    for (unit, floor) in keeping
        .changes(&windows)
        .into_iter()
        .filter(|(unit, _)| !holding.holds(unit))
    {
        match Command::new("busctl")
            .args(keep::keep_call(&unit, floor))
            .stdout(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => {
                keeping.done(&unit, floor);
                let says = match floor {
                    keep::Floor::Orchestrator => "an orchestrator's window",
                    keep::Floor::Pi => "a pi's window, softly",
                    keep::Floor::Nothing => "which runs no pi",
                };
                let did = if floor == keep::Floor::Nothing {
                    "Gave back"
                } else {
                    "Kept"
                };
                eprintln!("{did} {unit}, {says}.");
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
            sent_warned(
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
        let unlisted = by_quest
            .as_deref()
            .and_then(|weights| weights.unlisted(slice));
        let into = placed
            .as_ref()
            .map(|(quest, _)| quest.as_str())
            .or(unlisted.as_deref())
            .unwrap_or(slice);
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
                    piece.comm.escape_debug(),
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

/// What's looked at for busy work under `under`: each terminal's scope
/// under a `homes` prefix, and each pi host's unit under the `hosts` one.
/// A host runs every command its conversations start in its own unit,
/// which oomd avoids, until something moves it: from 00:19 on 15 November
/// three evaluations there held 22 GB, agents.slice stalled at its
/// ceiling, Joel's typing into his sessions with it, and oomd ended 38
/// small units beside them, holders and mycelium's node, freeing nothing.
fn looked_at(under: &Path, prefixes: &[String], hosts: Option<&str>) -> Vec<PathBuf> {
    let mut found = homes(under, prefixes);
    if let Some(prefix) = hosts {
        found.extend(hosts::units(under, prefix));
    }
    found
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

/// Every scope under `under` but the windows': the terminals' and the
/// sessions' held apart from them. A session's scope is Joel's window
/// whoever opened it: after the 16:28 reboot on 13 November, agents
/// reopened the windows from jobs, and 65 holders were moved back there,
/// leaving 46 window pis at jobs' weight and the OOM killer's first pick.
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

/// What writing processes into a cgroup came to.
#[derive(Debug, Default)]
struct Sent {
    /// How many went.
    moved: usize,
    /// How many stayed, failing rather than having ended.
    stayed: usize,
    /// The first that failed, and why. With cores held, one holding
    /// capabilities its mover lacks, a setuid program's, is refused a
    /// move onto other cores; a cgroup that has gone fails every one.
    failed: Option<(u32, io::Error)>,
}

/// Writes each process into the cgroup `into`. One that has exited since
/// is no loss, so it counts as neither moved nor stayed.
fn send(into: &Path, pids: &[u32]) -> Sent {
    sent_by(pids, |pid| {
        fs::write(into.join("cgroup.procs"), format!("{pid}\n"))
    })
}

/// What writing each process with `write` came to, as [`send`] counts it.
fn sent_by(pids: &[u32], mut write: impl FnMut(u32) -> io::Result<()>) -> Sent {
    let mut sent = Sent::default();
    for pid in pids {
        match write(*pid) {
            Ok(()) => sent.moved += 1,
            Err(error) if error.raw_os_error() == Some(3) => {}
            Err(error) => {
                sent.stayed += 1;
                sent.failed.get_or_insert((*pid, error));
            }
        }
    }
    sent
}

/// Writes `pids` into `into`, saying the first failure in the sorter's
/// life of the moves that split a cgroup, which say nothing of what went.
fn sent_warned(into: &Path, pids: &[u32], warned: &mut bool) {
    let sent = send(into, pids);
    if let Some((pid, error)) = &sent.failed
        && !*warned
    {
        eprintln!("Couldn't move {pid} into {}: {error}", into.display());
        *warned = true;
    }
}

/// What to say of moving `name`'s processes back out of jobs, as `sent`
/// says they went: `moved` of how many went, when any did, and why some
/// stayed; when none went, `refused` of why, once while `name` stays in
/// `failed`; and nothing when every one had ended.
fn returned(
    failed: &mut HashSet<String>,
    name: &str,
    sent: &Sent,
    moved: impl FnOnce(String) -> String,
    refused: impl FnOnce(String) -> String,
) -> Option<String> {
    if sent.moved > 0 {
        failed.remove(name);
    }
    let Some((pid, error)) = &sent.failed else {
        return (sent.moved > 0).then(|| moved(format!("{} processes", sent.moved)));
    };
    let why = format!("moving {pid} failed: {error}");
    if sent.moved > 0 {
        return Some(moved(format!(
            "{} of {} processes, as {why}",
            sent.moved,
            sent.moved + sent.stayed
        )));
    }
    failed.insert(name.to_owned()).then(|| refused(why))
}

fn settings(mut args: impl Iterator<Item = String>) -> Result<Settings, Failure> {
    let wrongly = |why: &str| cli::wrongly("jobs-sorter", why);
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
        quick: Vec::new(),
        quick_names: Vec::new(),
        by_quest: Vec::new(),
        quests: None,
        tiers: None,
        parents: None,
        tier_order: tiers::Order::default(),
        background: None,
        low_weight: 10,
        weigh: Vec::new(),
        revive: None,
        orchestrators: None,
        door: None,
        calls: None,
        calls_elsewhere: None,
        niri: None,
        hosts: None,
        conversations: None,
        slots: None,
        sessions: None,
        sends: Vec::new(),
    };
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| wrongly(&format!("{flag} needs a value")))
        };
        match flag.as_str() {
            "--under" => {
                let given = value()?;
                let (slice, sends) = match given.split_once('=') {
                    Some((slice, sends)) => (slice.to_owned(), Some(sends.to_owned())),
                    None => (given, None),
                };
                settings.under.push(slice);
                settings.sends.push(sends);
            }
            "--home" => settings.homes.push(value()?),
            "--keep" => settings.keep.push(value()?),
            "--jobs" => settings.jobs = value()?,
            "--split" => settings.split.push(value()?),
            "--no-real-time" => settings.no_real_time.push(value()?),
            "--quick" => settings.quick.push(value()?),
            "--quick-name" => settings.quick_names.push(value()?),
            "--by-quest" => settings.by_quest.push(value()?),
            "--quests" => settings.quests = Some(PathBuf::from(value()?)),
            "--tiers" => settings.tiers = Some(PathBuf::from(value()?)),
            "--parents" => settings.parents = Some(PathBuf::from(value()?)),
            "--tier-order" => {
                settings.tier_order = tiers::Order::new(&value()?).map_err(|why| wrongly(&why))?;
            }
            "--background" => settings.background = Some(PathBuf::from(value()?)),
            "--weigh" => settings.weigh.push(value()?),
            "--revive" => settings.revive = Some(value()?),
            "--door" => settings.door = Some(value()?),
            "--calls" => settings.calls = Some(PathBuf::from(value()?)),
            "--calls-elsewhere" => settings.calls_elsewhere = Some(elsewhere_from(&value()?)?),
            "--niri" => settings.niri = Some(PathBuf::from(value()?)),
            "--hosts" => settings.hosts = Some(value()?),
            "--conversations" => settings.conversations = Some(value()?),
            "--slots" => settings.slots = Some(PathBuf::from(value()?)),
            "--sessions" => settings.sessions = Some(value()?),
            "--orchestrators" => settings.orchestrators = Some(PathBuf::from(value()?)),
            "--low-weight" => settings.low_weight = number(&value()?, "--low-weight is a weight")?,
            "--busy" => settings.busy_ms = number(&value()?, "--busy is ms")?,
            "--ticks" => settings.ticks = number(&value()?, "--ticks is a count")?,
            "--every" => {
                settings.every = Duration::from_millis(number(&value()?, "--every is ms")?);
            }
            _ => {
                return Err(
                    flags::unknown("flag", &flag, &cli::flags()).try_this("jobs-sorter --help")
                );
            }
        }
    }
    if settings.under.is_empty() {
        settings.under.push("app.slice".into());
        settings.sends.push(None);
    }
    if settings.homes.is_empty() {
        settings.homes.push("kitty-".into());
    }
    paired(settings)
}

/// The settings, refused when a flag is given without the one it needs.
fn paired(settings: Settings) -> Result<Settings, Failure> {
    let wrongly = |why: &str| cli::wrongly("jobs-sorter", why);
    if settings.quick.is_empty() != settings.quick_names.is_empty() {
        return Err(wrongly(
            "--quick names where programs ask for a short slice, and --quick-name which",
        ));
    }
    // The kernel keeps a process's name to 15 bytes, so a longer one could
    // never be matched and would leave that program unasked unseen.
    if settings.quick_names.iter().any(|name| name.len() > 15) {
        return Err(wrongly(
            "--quick-name is a program's name as the kernel keeps it, at most 15 bytes",
        ));
    }
    if settings.sessions.is_some() && settings.weigh.is_empty() {
        return Err(wrongly(
            "--sessions names a session's scope by a --weigh prefix, and none is given",
        ));
    }
    // A conversation's unit is found from its session's slot, and ranked beside its host's.
    if settings.conversations.is_some() != settings.slots.is_some()
        || (settings.conversations.is_some() && settings.hosts.is_none())
    {
        return Err(wrongly(
            "--conversations ranks a host's conversations' units beside --hosts, by --slots",
        ));
    }
    Ok(settings)
}

/// `--calls-elsewhere`'s value, the nodes' sockets' folder and the
/// mycelium to ask them, or the refusal saying what it is.
fn elsewhere_from(given: &str) -> Result<(PathBuf, PathBuf), Failure> {
    let (sockets, mycelium) = given.split_once('=').ok_or_else(|| {
        cli::wrongly(
            "jobs-sorter",
            "--calls-elsewhere is the sockets' folder, =, then mycelium",
        )
    })?;
    Ok((PathBuf::from(sockets), PathBuf::from(mycelium)))
}

/// A flag's value read as a number, or the refusal saying what it is.
fn number<T: std::str::FromStr>(value: &str, what: &str) -> Result<T, Failure> {
    value.parse().map_err(|_| cli::wrongly("jobs-sorter", what))
}

#[cfg(test)]
mod settings_tests {
    use super::settings;

    fn given(words: &str) -> Vec<String> {
        settings(words.split_whitespace().map(str::to_owned))
            .unwrap()
            .under
    }

    // The help's flags and the flags settings parses are two lists, so a
    // flag added to the help alone would be offered and then refused.
    #[test]
    fn every_flag_its_help_names_is_one_it_takes() {
        for flag in crate::cli::flags() {
            if let Err(refused) = settings([flag.to_owned(), "1".to_owned()].into_iter()) {
                assert!(
                    !refused.what.starts_with("no flag called"),
                    "jobs-sorter's help names {flag}, but it refuses it as unknown"
                );
            }
        }
    }

    #[test]
    fn quick_takes_its_cgroups_and_names_and_neither_alone() {
        let got = settings(
            "--quick app.slice --quick-name chrome --quick-name slack"
                .split_whitespace()
                .map(str::to_owned),
        )
        .expect("both are given");
        assert_eq!(got.quick, ["app.slice"], "where they ask");
        assert_eq!(got.quick_names, ["chrome", "slack"], "who asks");
        for alone in ["--quick app.slice", "--quick-name chrome"] {
            assert!(
                settings(alone.split_whitespace().map(str::to_owned)).is_err(),
                "{alone} alone is refused"
            );
        }
        assert!(
            settings(
                "--quick app.slice --quick-name google-chrome-st"
                    .split_whitespace()
                    .map(str::to_owned)
            )
            .is_err(),
            "a name longer than the kernel keeps, which nothing could match, is refused"
        );
    }

    #[test]
    fn conversations_take_their_slots_beside_the_hosts_and_neither_alone() {
        let got = settings(
            "--hosts pi-host- --conversations pi-conversation- --slots /run/pi-host"
                .split_whitespace()
                .map(str::to_owned),
        )
        .expect("all three are given");
        assert_eq!(got.conversations.as_deref(), Some("pi-conversation-"));
        assert_eq!(got.slots, Some(std::path::PathBuf::from("/run/pi-host")));
        for alone in [
            "--hosts pi-host- --conversations pi-conversation-",
            "--hosts pi-host- --slots /run/pi-host",
            "--conversations pi-conversation- --slots /run/pi-host",
        ] {
            assert!(
                settings(alone.split_whitespace().map(str::to_owned)).is_err(),
                "{alone} is refused"
            );
        }
    }

    #[test]
    fn the_apps_slice_is_watched_when_none_is_named() {
        assert_eq!(given(""), ["app.slice"]);
    }

    #[test]
    fn every_slice_named_is_watched() {
        assert_eq!(
            given("--under app.slice --under agents.slice"),
            ["app.slice", "agents.slice"],
            "a slice named with --under isn't watched"
        );
    }

    #[test]
    fn a_session_goes_back_only_where_a_prefix_can_name_its_scope() {
        let words = |line: &str| {
            line.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let named = settings(words("--sessions agents.slice --weigh mycelium-pi-").into_iter());
        assert_eq!(
            named.map(|given| given.sessions),
            Ok(Some("agents.slice".to_owned()))
        );
        assert!(
            settings(words("--sessions agents.slice").into_iter()).is_err(),
            "a session's scope was to be named with no prefix to name it by"
        );
    }

    #[test]
    fn an_agents_terminal_sends_its_work_where_agents_work_goes() {
        let sorted = settings(
            "--under app.slice --under agents.slice=jobs.slice/jobs.service \
             --jobs jobs.slice/jobs-awaited.slice/awaited.service"
                .split_whitespace()
                .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(
            sorted.places(),
            [
                ("app.slice", "jobs.slice/jobs-awaited.slice/awaited.service"),
                ("agents.slice", "jobs.slice/jobs.service"),
            ],
            "an agent's terminal's busy work went where Joel's own goes, or his went elsewhere"
        );
    }
}

/// Moves each stray scope under `under` back to what started it,
/// forgetting the failures said of those no longer found.
fn return_strays(
    under: &[PathBuf],
    windows: &[String],
    root: &Path,
    manager: &str,
    slice: &str,
    returns: &mut Returns,
) {
    let found: Vec<PathBuf> = under
        .iter()
        .flat_map(|under| strays(under, windows))
        .collect();
    let names = found
        .iter()
        .map(|scope| {
            scope
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    returns.strays_found(&names);
    for scope in &found {
        return_stray(scope, root, manager, slice, &mut returns.strays);
    }
}

/// Moves a scope that work in `slice` started, with no memory ceiling of
/// its own, back to the cgroup of what started it.
fn return_stray(
    scope: &Path,
    root: &Path,
    manager: &str,
    slice: &str,
    failed: &mut HashSet<String>,
) {
    let procs = samples(scope);
    if procs.is_empty() || !unlimited(|file| fs::read_to_string(scope.join(file)).ok()) {
        return;
    }
    let Some(back) = returns_to(&procs, |pid| cgroup_of(pid, manager), slice) else {
        return;
    };
    let pids: Vec<u32> = procs.iter().map(|proc| proc.pid).collect();
    let sent = send(&root.join(back.trim_start_matches('/')), &pids);
    let name = scope.file_name().unwrap_or_default().to_string_lossy();
    let says = returned(
        failed,
        &name,
        &sent,
        |went| format!("Moved {name} ({went}) back to {back}, which started it."),
        |why| {
            format!(
                "Couldn't move {name} back to {back}, which started it, as {why}. It stays \
                 where it is, and isn't said again while it does."
            )
        },
    );
    if let Some(says) = says {
        eprintln!("{says}");
    }
}

/// Gives each session found under the jobs slice a scope of its own in
/// `--sessions`' slice, named by the first `--weigh` prefix, or moves it
/// into the one it has there already.
fn return_sessions(root: &Path, settings: &Settings, returns: &mut Returns) {
    let (Some(slice), Some(prefix)) = (&settings.sessions, settings.weigh.first()) else {
        return;
    };
    let jobs = settings.jobs.split('/').next().unwrap_or(&settings.jobs);
    let procs: Vec<Sample> = real_time::processes_under(&root.join(jobs))
        .into_iter()
        .filter_map(|pid| parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?))
        .collect();
    let cmdline = |pid: u32| fs::read(format!("/proc/{pid}/cmdline")).ok();
    let found = sessions::found(&procs, &settings.keep, cmdline);
    let names: HashSet<String> = found
        .iter()
        .map(|(id, _)| format!("{prefix}{id}.scope"))
        .collect();
    returns.sessions_found(&names);
    for (id, pids) in found {
        let name = format!("{prefix}{id}.scope");
        let scope = root.join(slice).join(&name);
        // A holder in jobs naming a session whose scope holds its holder
        // already is an agent's, which would be held as Joel's pi is.
        let there = samples(&scope)
            .into_iter()
            .filter_map(|proc| cmdline(proc.pid));
        if sessions::held_already(there, &id) {
            continue;
        }
        let made = Command::new("busctl")
            .args(sessions::scope_call(&name, slice, &pids))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        let says = if made {
            returns.asked(
                &name,
                || {
                    format!(
                        "Asked systemd to give {name} ({} processes) its scope back in \
                         {slice}, from jobs.",
                        pids.len()
                    )
                },
                || {
                    format!(
                        "{name} is still in jobs after systemd was asked for its scope in \
                         {slice}, so some of it didn't go; systemd's journal of the scope says \
                         why. It's tried again each time, and isn't said again while it stays."
                    )
                },
            )
        } else if scope.join("cgroup.procs").exists() {
            returned(
                &mut returns.sessions,
                &name,
                &send(&scope, &pids),
                |went| format!("Gave {name} ({went}) its scope back in {slice}, from jobs."),
                |why| {
                    format!(
                        "Couldn't give {name} its scope back in {slice}, as {why}. It stays in \
                         jobs, and isn't said again while it does."
                    )
                },
            )
        } else {
            returns.unmade(&name)
        };
        if let Some(says) = says {
            eprintln!("{says}");
        }
    }
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

/// Weighs each quest's slice under each of the `bases` that isn't yet weighed as
/// its quest's priority says.
fn weigh_quests(weights: &mut quests::Weights, bases: &[String]) {
    weights.refresh_all();
    // Said in the journal, so each move between tiers can be found later.
    for (quest, before, now) in weights.moves() {
        eprintln!("{quest} moved from the {before} tier to {now}.");
    }
    for base in bases {
        for (slice, weight) in weights.unweighed(&weights.current(base)) {
            weigh(weights, &slice, weight);
        }
    }
}

/// The quests' weights, ranked into tiers when both files are given.
fn weights_from(settings: &Settings) -> quests::Weights {
    let mut weights = quests::Weights::new(settings.quests.clone().unwrap_or_default());
    match (&settings.tiers, &settings.parents) {
        (Some(heads), Some(parents)) => {
            weights.rank_by(tiers::Tiers::new(heads.clone(), parents.clone()));
        }
        (None, None) => {}
        _ => eprintln!("--tiers needs --parents, and --parents --tiers, so no quest is ranked."),
    }
    weights
}

/// What [`rank_for_memory`] looks over.
struct Looking<'a> {
    settings: &'a Settings,
    root: &'a Path,
    under: &'a [PathBuf],
    windows: &'a [String],
    weights: &'a quests::Weights,
}

/// Whether the kernel holds the slices asked, and whether a failure to
/// ask was said, apart from the sorter's other failures so one can't hide
/// another.
struct Quick {
    heard: bool,
    warned: bool,
}

/// Has Joel's `--quick-name` programs ask for a short slice, until the
/// kernel is found to hold none.
fn ask_quick(
    settings: &Settings,
    root: &Path,
    manager: &str,
    windows: &[String],
    quick: &mut Quick,
) {
    if !quick.heard {
        return;
    }
    for cgroup in &settings.quick {
        let asking = quick::Asking {
            quick: cgroup,
            dir: &root.join(cgroup),
            manager,
            names: &settings.quick_names,
            windows,
        };
        quick.heard &= quick::ask(&asking, &mut quick.warned);
    }
    if !quick.heard {
        eprintln!("This kernel holds no slice a thread asks for, so none is asked again.");
    }
}

/// Puts each pi and each piece of work where it goes for the kernel's
/// OOM killer, by tier, keeping orchestrators' windows, and takes real
/// time from work.
fn rank_for_memory(looking: &Looking, ranks: &mut Ranks, warned: &mut bool) {
    let settings = looking.settings;
    if let Some(dir) = &settings.orchestrators {
        let ranking = Ranking {
            weights: looking.weights,
            order: &settings.tier_order,
        };
        keep_windows(looking.under, looking.windows, dir, &ranking, ranks);
    }
    for cgroup in &settings.no_real_time {
        real_time::take(&looking.root.join(cgroup), warned);
        let work = real_time::processes_under(&looking.root.join(cgroup));
        oom::raise(
            Path::new("/proc"),
            &work,
            &settings.tier_order,
            &settings.by_quest,
            warned,
        );
    }
}

/// Where each watched slice's terminals send their busy work, under the
/// account's cgroup, each said once and refused when it doesn't exist.
fn sends_from(root: &Path, settings: &Settings) -> Result<Vec<PathBuf>, String> {
    let places = settings.places();
    let sends: Vec<PathBuf> = places.iter().map(|(_, sends)| root.join(sends)).collect();
    if let Some(gone) = sends
        .iter()
        .find(|jobs| !jobs.join("cgroup.procs").exists())
    {
        return Err(format!("{} doesn't exist", gone.display()));
    }
    for (under, sends) in places {
        eprintln!(
            "Sorting busy work from {under} ({}*) into {sends}.",
            settings.homes.join("*, ")
        );
    }
    Ok(sends)
}

fn run(settings: &Settings) -> Result<(), String> {
    let own = fs::read_to_string("/proc/self/cgroup").map_err(|e| e.to_string())?;
    let manager = manager_of(&own).ok_or("not running under a user manager")?;
    let root = Path::new(CGROUPS).join(manager.trim_start_matches('/'));
    let under: Vec<PathBuf> = settings
        .under
        .iter()
        .map(|slice| root.join(slice))
        .collect();
    let slice = format!(
        "/{}",
        settings.jobs.split('/').next().unwrap_or(&settings.jobs)
    );
    let sends = sends_from(&root, settings)?;
    if let Some(cgroup) = &settings.revive {
        revive::watch(&root.join(cgroup));
    }
    let mut sorter = Sorter {
        last: uptime_ticks().unwrap_or(0),
        ..Sorter::default()
    };
    let mut warned = false;
    let mut quick = Quick {
        heard: true,
        warned: false,
    };
    let mut holders: Vec<Option<u32>> = vec![None; settings.split.len()];
    let mut weights = weights_from(settings);
    let mut pass: u32 = 0;
    let windows = settings.windows();
    let mut watch = settings.background.as_deref().and_then(watch_marks);
    let following = follow_focus(settings.niri.as_deref(), watch.as_mut());
    let mut ranks = Ranks {
        holding: left_held(following.as_ref(), &under, &windows, &settings.weigh),
        in_memory: left_in_memory(following.as_ref(), &under, &windows),
        ..Ranks::default()
    };
    let mut doorway = Doorway::new(settings.calls.as_deref());
    doorway.elsewhere = settings
        .calls_elsewhere
        .as_ref()
        .map(|(sockets, mycelium)| elsewhere::Elsewhere::follow(sockets, mycelium));
    let processors = thread::available_parallelism().map_or(1, usize::from);
    doorway.brake = brake::Brake::found(
        |place| braked_cpu_max(place, &root),
        brake::quota(processors),
    );
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
                    conversations: settings
                        .conversations
                        .as_deref()
                        .zip(settings.slots.as_deref()),
                    pis: &settings.weigh,
                };
                rest(watch, settings.every, &windows, &mut ranks);
            }
            _ => thread::sleep(settings.every),
        }
        let now = uptime_ticks().ok_or("can't read /proc/uptime")?;
        let alive = sorter.sort(&under, &sends, now, settings);
        let returns = &mut sorter.returns;
        return_strays(&under, &windows, &root, &manager, &slice, returns);
        if pass.is_multiple_of(sessions::EVERY) {
            return_sessions(&root, settings, returns);
        }
        sorter.told.say(&under);
        // After the terminals' work is moved, so what moved is split too.
        weigh_quests(&mut weights, &settings.by_quest);
        for (cgroup, holder) in settings.split.iter().zip(holders.iter_mut()) {
            let quest_slices = slice_and_unit(cgroup)
                .is_some_and(|(_, slice, _)| settings.by_quest.iter().any(|base| base == slice));
            let by_quest = quest_slices.then_some(&mut weights);
            split(&root, &manager, cgroup, holder, by_quest, &mut warned);
        }
        if pass.is_multiple_of(real_time::EVERY) {
            let looking = Looking {
                settings,
                root: &root,
                under: &under,
                windows: &windows,
                weights: &weights,
            };
            rank_for_memory(&looking, &mut ranks, &mut warned);
        }
        if pass.is_multiple_of(quick::EVERY) {
            ask_quick(settings, &root, &manager, &windows, &mut quick);
        }
        watch_door(&mut doorway, &root, settings.door.as_deref(), now);
        brake_for_calls(&mut doorway, &root);
        pass = pass.wrapping_add(1);
        sorter.finish(&alive, now);
    }
}

/// The door, and the calls it keeps more room through when it hears them.
struct Doorway {
    door: door::Door,
    calls: Option<calls::Calls>,
    /// The calls on the person's other machines, which brake builds and
    /// jobs here too, though the door keeps no room for them.
    elsewhere: Option<elsewhere::Elsewhere>,
    /// Where builds and jobs stand braked for a call (src/brake.rs).
    brake: brake::Brake,
    /// Whether memory's pressure couldn't be read when last looked at, so
    /// the pace's stall gate going off is said once, not every look.
    unstalled: bool,
}

impl Doorway {
    /// A door that hears calls through `pw_dump`, when there's one.
    fn new(pw_dump: Option<&Path>) -> Doorway {
        Doorway {
            door: door::Door::default(),
            calls: pw_dump.map(calls::Calls::follow),
            elsewhere: None,
            brake: brake::Brake::default(),
            unstalled: false,
        }
    }
}

/// Holds builds and jobs to a quarter of the processors while a call is
/// on, here or on another of the person's machines (src/elsewhere.rs), and
/// lets them go once it ends, as src/brake.rs says. A slice that
/// couldn't be set is tried again once it has waited, and one found let go
/// mid-call is set again; `account` is the account's manager's cgroup.
fn brake_for_calls(doorway: &mut Doorway, account: &Path) {
    let now = std::time::Instant::now();
    let here = doorway.calls.as_mut().is_some_and(|calls| calls.on(now));
    let there = doorway
        .elsewhere
        .as_ref()
        .is_some_and(|elsewhere| elsewhere.on(now));
    let calling = here || there;
    let processors = std::thread::available_parallelism().map_or(1, usize::from);
    let lost = if calling {
        let read = |place| braked_cpu_max(place, account);
        doorway.brake.check(read, brake::quota(processors), now)
    } else {
        Vec::new()
    };
    let due = doorway.brake.due(calling, now);
    for place in lost.iter().filter(|place| !due.contains(place)) {
        eprintln!(
            "Found {} let go again mid-call, so it's braked again once {} s have passed.",
            brake::BRAKED[*place].1,
            brake::AGAIN.as_secs()
        );
    }
    if due.is_empty() {
        return;
    }
    let quota = brake::quota_for(calling, processors);
    for place in due {
        let (manager, slice) = brake::BRAKED[place];
        let set = Command::new("busctl")
            .args(brake::quota_call(manager, slice, quota))
            .stdout(Stdio::null())
            .output();
        match set {
            Ok(output) if output.status.success() => {
                doorway.brake.set(place, calling);
                if lost.contains(&place) {
                    eprintln!("Found {slice} let go mid-call, and braked it again.");
                }
            }
            Ok(output) => {
                doorway.brake.failed(place, calling, now);
                eprintln!(
                    "Couldn't set {slice}'s quota for a call: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
            Err(error) => {
                doorway.brake.failed(place, calling, now);
                eprintln!("Couldn't run busctl to set {slice}'s quota for a call: {error}");
            }
        }
    }
    if doorway.brake.stands(calling) {
        eprintln!(
            "{}.",
            if here {
                "Braked builds and jobs, a call on"
            } else if there {
                "Braked builds and jobs, a call on another machine"
            } else {
                "Let go of builds and jobs, no call on"
            }
        );
    }
}

/// What the cpu.max of the slice at `place` in brake.rs's `BRAKED` reads,
/// `account` being the account's manager's cgroup.
fn braked_cpu_max(place: usize, account: &Path) -> Option<String> {
    fs::read_to_string(brake::cgroup(place, Path::new(CGROUPS), account).join("cpu.max")).ok()
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
    let stalled = door::stalled(&fs::read_to_string("/proc/pressure/memory").unwrap_or_default());
    if stalled.is_none() && !doorway.unstalled {
        eprintln!(
            "Can't read memory's pressure from /proc/pressure/memory, so paced starts aren't \
             held for a stall."
        );
    }
    doorway.unstalled = stalled.is_none();
    let machine = door::Machine {
        total,
        free,
        calling: doorway.calls.as_mut().is_some_and(|calls| calls.on(now)),
        stalled: stalled.unwrap_or(0.0),
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
            door::Done::Paced(pid) => {
                door::tell_paced(pid);
                eprintln!(
                    "Held {pid} at the door for its turn, with starts paced and memory full {}% \
                     over a minute.",
                    machine.stalled
                );
            }
            door::Done::LetIn(pid) => eprintln!("Let {pid} in."),
            door::Done::Ranked(pid, high) => eprintln!(
                "Let {pid} in behind other work, its memory held under {high} bytes, with \
                 {free} bytes available."
            ),
        }
    }
}

fn main() -> ExitCode {
    terminal_kit::main(
        &cli::help(),
        env!("CARGO_PKG_VERSION"),
        |context| match context.args.split_first() {
            Some((first, rest)) if first == "probe" => {
                let beat =
                    probe::every(rest).map_err(|why| cli::wrongly("jobs-sorter probe", why))?;
                probe::run(beat)
            }
            Some((first, rest)) if first == "swap-in" => {
                eprintln!("{}", swap_in::main(rest)?);
                Ok(())
            }
            _ => {
                let settings = settings(context.args.iter().cloned())?;
                run(&settings).map_err(|why| {
                    Failure::new(Exit::Failed, "jobs-sorter couldn't sort the cgroups").because(why)
                })
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewers_join_in_the_order_their_windows_are_held_and_only_a_session_no_slot_holds_is_found()
    {
        let root = std::env::temp_dir().join(format!("viewers-{}", std::process::id()));
        let slice = root.join("agents.slice");
        for scope in [
            "kitty-1-1.scope",
            "kitty-1-2.scope",
            "kitty-1-3.scope",
            "kitty-1-4.scope",
            "kitty-1-5.scope",
            "mycelium-pi-x.scope",
        ] {
            fs::create_dir_all(slice.join(scope)).unwrap();
        }
        let slots = root.join("slots");
        let host = slots.join("a.d");
        fs::create_dir_all(&host).unwrap();
        // s1 runs in its slot's unit; s7's slot outlives its unit, parked or starting.
        fs::write(host.join("s1.json"), r#"{"session": {"sessionId": "s1"}}"#).unwrap();
        fs::write(host.join("s9.json"), r#"{"session": {"sessionId": "s7"}}"#).unwrap();
        let attach = |host: &str, session: &str| {
            ["mycelium", "attach", "--host", host, "--session", session]
                .map(str::to_owned)
                .to_vec()
        };
        // A pi's scope holds its agents' commands, which may spell themselves as a viewer.
        let cmdlines = |scope: &Path| match scope.file_name().and_then(|name| name.to_str()) {
            Some("kitty-1-1.scope") => vec![attach("a", "s2")],
            Some("kitty-1-2.scope") => vec![attach("c", "s3")],
            Some("kitty-1-3.scope") => vec![attach("b", "s4")],
            Some("kitty-1-4.scope") => vec![attach("a", "s1"), attach("a", "s7")],
            // A second window on s2, and a session spelled to start a journal line.
            Some("kitty-1-5.scope") => vec![attach("a", "s2"), attach("a", "x\nRanked")],
            Some("mycelium-pi-x.scope") => vec![attach("d", "s5")],
            _ => Vec::new(),
        };
        let units: Vec<PathBuf> = [
            "pi-host-a.service",
            "pi-host-b.service",
            "pi-host-c.service",
            "pi-host-d.service",
            "pi-conversation-a@s1.service",
        ]
        .map(|unit| slice.join(unit))
        .to_vec();
        let under = [slice.clone()];
        let prefixes = ["kitty-", "mycelium-pi-"].map(str::to_owned);
        let pis = ["mycelium-pi-".to_owned()];
        let windows = Windows {
            under: &under,
            prefixes: &prefixes,
            dir: &root,
            weight: 100,
            focus: None,
            hosts: Some("pi-host-"),
            conversations: Some(("pi-conversation-", &slots)),
            pis: &pis,
        };
        // Neither the scopes' names nor the order they were made in, so only `held` gives it.
        let held = [
            "kitty-1-2.scope",
            "mycelium-pi-x.scope",
            "kitty-1-1.scope",
            "kitty-1-3.scope",
        ]
        .map(str::to_owned);
        let joined = viewers_and_shown(&windows, "pi-host-", &held, &units, &cmdlines);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            joined.viewers,
            [
                "pi-host-c.service",
                "pi-host-a.service",
                "pi-host-b.service"
            ],
            "the held windows' viewers join in the order they're held, the pi's scope passed over"
        );
        assert!(
            joined
                .shown
                .contains(&"pi-conversation-a@s1.service".to_owned()),
            "a viewer of a conversation in its slot's unit shows that unit, got {:?}",
            joined.shown
        );
        assert_eq!(
            joined.unslotted,
            [
                ("a".to_owned(), "s2".to_owned()),
                ("a".to_owned(), "x\\nRanked".to_owned())
            ],
            "only sessions no slot holds are found, s2 once for its two windows, and a session's \
             newline can't start a journal line; s1 runs in its unit, and s7's slot holds it"
        );
    }

    #[test]
    fn an_unslotted_session_is_said_at_its_second_look_once_and_again_when_it_clears() {
        let x = ("a".to_owned(), "x".to_owned());
        let y = ("a".to_owned(), "y".to_owned());
        let mut unslotted = Unslotted::default();
        let looks: Vec<_> = [
            vec![x.clone()],
            vec![x.clone(), y.clone()],
            vec![x.clone()],
            vec![],
            vec![x.clone()],
            vec![x.clone()],
        ]
        .into_iter()
        .map(|now| unslotted.look(now))
        .collect();
        let none = (vec![], vec![]);
        assert_eq!(
            looks,
            [
                none.clone(),
                (vec![x.clone()], vec![]),
                none.clone(),
                (vec![], vec![x.clone()]),
                none,
                (vec![x.clone()], vec![]),
            ],
            "a session is said at its second look in a row and once, said cleared when it \
             goes, and said again when it comes back; y, for one look, is never said"
        );
    }

    /// A runtime control folder under the temporary folder named for `name`, holding each of
    /// `units`' folders with the five files systemd writes for the sorter's settings, and
    /// the sorter's mark.
    fn control_with(name: &str, units: &[&str]) -> PathBuf {
        let control = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&control);
        for unit in units {
            let folder = control.join(format!("{unit}.d"));
            fs::create_dir_all(&folder).expect("a unit's folder");
            for file in [
                "50-CPUWeight.conf",
                "50-MemoryLow.conf",
                "50-MemoryZSwapWriteback.conf",
                "50-ManagedOOMPreference.conf",
                "50-MemoryMin.conf",
            ] {
                fs::write(folder.join(file), "[Service]\n").expect("a setting's file");
            }
            own::mark(&control, unit).expect("the sorter's mark");
        }
        control
    }

    #[test]
    fn a_sweep_asks_once_after_the_units_not_running_and_again_after_those_still_stopping() {
        let unit = |name: &str| format!("pi-conversation-w@{name}.service");
        let control = control_with(
            "sweep-asks-once",
            &[
                &unit("runs"),
                &unit("gone"),
                &unit("stopping"),
                &unit("failed"),
                &unit("old"),
            ],
        );
        let mut hosts = hosts::Hosts::default();
        let rank = hosts::wanted(&own::Own::default(), hosts::Signs::default(), 10);
        for name in ["runs", "gone", "stopping", "failed"] {
            hosts.done(&unit(name), rank.clone(), 0);
        }
        let seen = [unit("runs")];
        let mut asked: Vec<Vec<String>> = Vec::new();
        let mut states = HashMap::from([
            (unit("gone"), "inactive"),
            (unit("stopping"), "deactivating"),
            (unit("failed"), "failed"),
            (unit("old"), "inactive"),
        ]);
        let look = |hosts: &mut hosts::Hosts,
                    states: &HashMap<String, &str>,
                    asked: &mut Vec<Vec<String>>| {
            let ask = |units: &[String]| {
                asked.push(units.to_vec());
                let lines: Vec<&str> = units
                    .iter()
                    .map(|unit| *states.get(unit).unwrap_or(&"active"))
                    .collect();
                Ok(lines.join("\n"))
            };
            let prefixes = ["pi-conversation-", "pi-host-"];
            sweep(
                Some(&control),
                &prefixes,
                &seen,
                hosts,
                std::time::Instant::now(),
                ask,
            );
            // As `rank_hosts` does after it.
            hosts.keep_only(&seen);
        };
        look(&mut hosts, &states, &mut asked);
        let there = |name: &str| control.join(format!("{}.d", unit(name))).exists();
        assert_eq!(
            asked,
            [[unit("failed"), unit("gone"), unit("old"), unit("stopping")]],
            "one call asks after every unit with the sorter's settings that isn't seen running"
        );
        assert_eq!(
            ["runs", "gone", "stopping", "failed", "old"].map(there),
            [true, false, true, true, false],
            "only the stopped units' settings go: a failed one's stay for systemd, which keeps \
             it loaded"
        );
        assert!(
            hosts.changed(&unit("gone"), &rank) && !hosts.changed(&unit("runs"), &rank),
            "a unit whose settings went is forgotten, so its return is ranked afresh"
        );
        states.insert(unit("stopping"), "inactive");
        look(&mut hosts, &states, &mut asked);
        assert_eq!(
            asked[1..],
            [[unit("stopping")]],
            "a unit that left still stopping is asked after again, alone"
        );
        assert!(!there("stopping"), "and once stopped its settings go");
        look(&mut hosts, &states, &mut asked);
        assert_eq!(
            asked.len(),
            2,
            "with none left to ask after, nothing is asked"
        );
        let _ = fs::remove_dir_all(&control);
    }

    #[test]
    fn a_unit_not_yet_stopped_when_first_asked_is_asked_again_until_it_is() {
        // A unit still stopping when the sorter starts, or failed and kept loaded until it's
        // reset, isn't one the sorter ranked, so nothing else would ever ask after it again.
        for first in ["deactivating\n", "failed\n"] {
            let gone = "pi-host-work-gone.service".to_owned();
            let control = control_with("sweep-not-yet-stopped", &[&gone]);
            let mut hosts = hosts::Hosts::default();
            let start = std::time::Instant::now();
            let again = own::ASK_AGAIN_AFTER;
            let mut asks = 0;
            let looks = [
                (Duration::ZERO, first, true),
                (again / 2, "inactive\n", false),
                (again, first, true),
                (again * 2, "inactive\n", true),
                (again * 3, "inactive\n", false),
            ];
            for (after, answer, asked) in looks {
                let before = asks;
                let ask = |_: &[String]| {
                    asks += 1;
                    Ok(answer.to_owned())
                };
                sweep(
                    Some(&control),
                    &["pi-host-"],
                    &[],
                    &mut hosts,
                    start + after,
                    ask,
                );
                assert_eq!(
                    asks > before,
                    asked,
                    "asked at {after:?} after a first look that found it {first:?}"
                );
            }
            let left = control.join(format!("{gone}.d")).exists();
            let _ = fs::remove_dir_all(&control);
            assert!(
                !left,
                "its settings went once it stopped, after a first look found it {first:?}"
            );
        }
    }

    #[test]
    fn a_sweep_that_cant_ask_says_so_once_and_is_tried_again() {
        let gone = "pi-host-work-gone.service".to_owned();
        let control = control_with("sweep-cant-ask", &[&gone]);
        let mut hosts = hosts::Hosts::default();
        let start = std::time::Instant::now();
        let mut asks = 0;
        // Each look: how long after the first it comes, what systemctl says, and whether it's
        // asked at all: after a failed ask, not again until ASK_AGAIN_AFTER has passed.
        let again = own::ASK_AGAIN_AFTER;
        let looks = [
            (Duration::ZERO, Err("no bus".to_owned()), true),
            (again / 2, Ok("inactive\n".to_owned()), false),
            (again, Ok(String::new()), true),
            (again * 2, Ok("inactive\n".to_owned()), true),
        ];
        for (after, answer, asked) in looks {
            let before = asks;
            let ask = |_: &[String]| {
                asks += 1;
                answer.clone()
            };
            let swept = hosts.sweep.done;
            let said = hosts.sweep.failed;
            sweep(
                Some(&control),
                &["pi-host-"],
                &[],
                &mut hosts,
                start + after,
                ask,
            );
            assert_eq!(
                asks > before,
                asked,
                "asked at {after:?} after a first look whose ask failed"
            );
            assert!(
                swept
                    || hosts.sweep.done
                        == (asked && answer.as_deref().is_ok_and(|answer| !answer.is_empty())),
                "a sweep counts as done only once systemctl answers for every unit: {answer:?}"
            );
            assert!(
                !said || hosts.sweep.failed || hosts.sweep.done,
                "a failure said stays said until an ask works, so it's said once: {answer:?}"
            );
        }
        assert_eq!(
            asks, 3,
            "a sweep that couldn't ask is tried again, held off a while"
        );
        assert!(
            !hosts.sweep.failed,
            "and the failure is said again once one works"
        );
        assert!(
            !control.join(format!("{gone}.d")).exists(),
            "the settings went once it could ask"
        );
        let _ = fs::remove_dir_all(&control);
    }

    #[test]
    fn only_units_in_agents_slice_take_a_hard_floors_share() {
        let units = ["/cg/agents.slice/a.service", "/cg/app.slice/b.service"].map(PathBuf::from);
        let viewers = ["b.service", "a.service"].map(str::to_owned);
        assert_eq!(
            shares(2, &hosts::Hosts::default(), &units, &viewers),
            HashMap::from([("a.service".to_owned(), 1)]),
            "the unit in app.slice takes no share, nor the one floor left"
        );
    }

    /// What the kernel keeps hard, by unit, after one of `rank_hosts`'s looks over `units`: each
    /// unit is given its share from `shares` and its rank through `give_ranks_by`, as
    /// `rank_hosts` gives them, and a call `lands` says fails leaves the kernel with what it
    /// kept before.
    fn hard_look(
        holding: &holds::Holding,
        hosts: &mut hosts::Hosts,
        kernel: &mut HashMap<String, usize>,
        units: &[PathBuf],
        viewers: &[&str],
        lands: impl Fn(&str) -> bool,
    ) {
        let viewers: Vec<String> = viewers.iter().map(|unit| (*unit).to_owned()).collect();
        let given = shares(holding.hard(), hosts, units, &viewers);
        let ranked = units
            .iter()
            .filter_map(|path| {
                let unit = path.file_name()?.to_str()?;
                let signs = hosts::Signs {
                    viewed: viewers.iter().filter(|viewer| *viewer == unit).count(),
                    held: given.get(unit).copied().unwrap_or(0),
                    ..hosts::Signs::default()
                };
                let rank = hosts::wanted(&own::Own::default(), signs, 10);
                Some(Ranked {
                    path: path.clone(),
                    asked: rank.floor,
                    rank,
                    signs,
                    own: 0,
                    own_hard: 0,
                    writeback: true,
                })
            })
            .collect();
        give_ranks_by(ranked, hosts, |unit, rank| {
            if !lands(unit) {
                return Err(format!("{unit}'s call fails"));
            }
            let hard = usize::try_from(rank.hard / holds::HARD).expect("a count of floors");
            kernel.insert(unit.to_owned(), hard);
            Ok(())
        });
    }

    #[test]
    fn at_most_three_hard_floors_hold_across_pis_and_hosts_while_a_hosts_call_fails() {
        let units = ["a.service", "b.service"].map(|unit| Path::new("/cg/agents.slice").join(unit));
        let pis = ["mycelium-pi-".to_owned()];
        let pi = PathBuf::from("/cg/agents.slice/mycelium-pi-1.scope");
        let all = |_: &str| true;
        let not_a = |unit: &str| unit != "a.service";
        // Each case: whether a pi holds hard, the viewers while every call lands, the viewers
        // after, while a's calls fail, and the floors that hold on the second look of those,
        // once a is counted as failing: no fewer than what a keeps leaves, so b isn't kept from
        // its share for as long as a's calls fail.
        let cases: [(bool, &[&str], &[&str], usize); 6] = [
            (
                false,
                &["a.service", "a.service"],
                &["b.service", "b.service", "a.service"],
                3,
            ),
            (
                false,
                &["a.service", "a.service"],
                &["a.service", "b.service", "b.service"],
                3,
            ),
            (
                false,
                &["a.service"; 3],
                &["b.service", "a.service", "a.service"],
                3,
            ),
            (false, &["a.service"; 3], &["b.service", "b.service"], 3),
            (
                false,
                &["a.service", "b.service", "b.service"],
                &["a.service", "a.service", "a.service", "b.service"],
                2,
            ),
            (
                true,
                &["a.service", "a.service"],
                &["a.service", "b.service"],
                3,
            ),
        ];
        for (pi_held, before, after, full) in cases {
            let mut holding = holds::Holding::default();
            if pi_held {
                holding.done(&pi, Some(holds::HARD));
            }
            let mut hosts = hosts::Hosts::default();
            let mut kernel = HashMap::new();
            hard_look(&holding, &mut hosts, &mut kernel, &units, before, all);
            // Three looks with a's calls failing, a pi held beside them from the third as
            // `set_holds` gives it its floor, counting what a's failed calls left, then two with
            // a's calls landing again.
            for look in 1..=5 {
                if look == 3 {
                    holding.elsewhere = hosts.hard_left();
                    let another = PathBuf::from("/cg/agents.slice/mycelium-pi-2.scope");
                    let hard = holding.hard_floor(&another, &pis);
                    holding.done(&another, Some(hard));
                }
                let lands = |unit: &str| look > 3 || not_a(unit);
                hard_look(&holding, &mut hosts, &mut kernel, &units, after, lands);
                let kept = holding.hard() + kernel.values().sum::<usize>();
                assert!(
                    look != 2 || kept == full,
                    "{kept} hard floors hold at the second look a's calls fail, not {full}, a \
                     pi held hard {pi_held}, viewers {before:?} then {after:?}: {kernel:?}"
                );
                assert!(
                    kept <= focus::MOST,
                    "{kept} hard floors hold at look {look}, a's calls failing up to the third, a \
                     pi held hard {pi_held} and another from the third, viewers {before:?} then \
                     {after:?}: {kernel:?}"
                );
            }
        }
    }

    #[test]
    fn a_pi_is_held_with_no_hard_floor_while_a_hosts_failed_give_back_keeps_them_all() {
        let pis = ["mycelium-pi-".to_owned()];
        let pi = PathBuf::from("/cg/agents.slice/mycelium-pi-1.scope");
        let mut ranks = Ranks::default();
        // A host this sorter never set, whose give-back fails, keeps every hard floor.
        ranks.hosts.fail("a.service", || focus::MOST);
        let mut given = Vec::new();
        set_holds_by(
            std::slice::from_ref(&pi),
            &pis,
            &mut ranks,
            |unit, hard, _| {
                given.push((unit.to_owned(), hard));
                Ok(())
            },
        );
        assert_eq!(
            given,
            [("mycelium-pi-1.scope".to_owned(), Some(0))],
            "a pi Joel is at is held, but with none of the floors the failing host keeps"
        );
        assert_eq!(ranks.holding.hard(), 0, "so no pi counts as holding one");
    }

    #[test]
    fn a_rank_that_lands_leaves_the_mark_and_one_that_fails_none() {
        let control = std::env::temp_dir().join(format!("ranks-mark-{}", std::process::id()));
        let _ = fs::remove_dir_all(&control);
        for unit in ["a.service", "b.service"] {
            fs::create_dir_all(control.join(format!("{unit}.d"))).unwrap();
        }
        let ranked = ["a.service", "b.service"]
            .map(|unit| {
                let rank = hosts::wanted(&own::Own::default(), hosts::Signs::default(), 10);
                Ranked {
                    path: Path::new("/cg/agents.slice").join(unit),
                    asked: rank.floor,
                    rank,
                    signs: hosts::Signs::default(),
                    own: 0,
                    own_hard: 0,
                    writeback: true,
                }
            })
            .into();
        let mut hosts = hosts::Hosts::default();
        give_ranks_marking(ranked, &mut hosts, Some(&control), |unit, _| {
            if unit == "b.service" {
                Err(format!("{unit}'s call fails"))
            } else {
                Ok(())
            }
        });
        let marked = ["a.service", "b.service"]
            .map(|unit| control.join(format!("{unit}.d")).join(own::MARK).exists());
        let _ = fs::remove_dir_all(&control);
        assert_eq!(
            marked,
            [true, false],
            "a landed rank marks its unit's settings to be taken back, and a failed one doesn't"
        );
    }

    #[test]
    fn a_give_back_is_asked_before_a_gain_whichever_unit_comes_first() {
        // b, which gains every floor, comes before a, which gives them back, as systemd may
        // list them: were b asked first, it would take them while a's give-back fails.
        let units = ["b.service", "a.service"].map(|unit| Path::new("/cg/agents.slice").join(unit));
        let holding = holds::Holding::default();
        let mut hosts = hosts::Hosts::default();
        let mut kernel = HashMap::new();
        let viewers = ["a.service"; 3];
        hard_look(&holding, &mut hosts, &mut kernel, &units, &viewers, |_| {
            true
        });
        let viewers = ["b.service"; 3];
        hard_look(
            &holding,
            &mut hosts,
            &mut kernel,
            &units,
            &viewers,
            |unit| unit != "a.service",
        );
        let kept: usize = kernel.values().sum();
        assert!(
            kept <= focus::MOST,
            "{kept} hard floors hold once b, listed first, gains while a's give-back fails: \
             {kernel:?}"
        );
    }

    #[test]
    fn a_host_whose_gain_failed_is_asked_again_until_it_lands() {
        let units = [Path::new("/cg/agents.slice/a.service").to_path_buf()];
        // Three pis hold every hard floor while Joel is at a, so a is ranked with none.
        let mut full = holds::Holding::default();
        for pi in 1..=focus::MOST {
            full.done(
                Path::new(&format!("/cg/agents.slice/mycelium-pi-{pi}.scope")),
                Some(holds::HARD),
            );
        }
        let mut hosts = hosts::Hosts::default();
        let mut kernel = HashMap::new();
        let viewers = ["a.service"];
        hard_look(&full, &mut hosts, &mut kernel, &units, &viewers, |_| true);
        // The pis give theirs back and a's call to gain one fails once; capped at what it keeps
        // while failing, its rank no longer differs from the one it was set, yet it must be
        // asked again, or it never leaves failing and never gains its floor.
        let empty = holds::Holding::default();
        hard_look(&empty, &mut hosts, &mut kernel, &units, &viewers, |_| false);
        for _ in 0..2 {
            hard_look(&empty, &mut hosts, &mut kernel, &units, &viewers, |_| true);
        }
        assert_eq!(
            kernel.get("a.service"),
            Some(&1),
            "a's floor never landed once its calls did: {kernel:?}"
        );
    }

    #[test]
    fn a_restarted_sorters_failed_give_back_holds_back_those_after_it() {
        let units = ["a.service", "b.service"].map(|unit| Path::new("/cg/agents.slice").join(unit));
        let holding = holds::Holding::default();
        // A sorter just started never set a, whose memory.min can't be read here, so it counts
        // as keeping all three the kernel may hold, as one before it may have left them.
        let mut hosts = hosts::Hosts::default();
        let mut kernel = HashMap::from([("a.service".to_owned(), focus::MOST)]);
        let viewers = ["b.service"; 3];
        hard_look(
            &holding,
            &mut hosts,
            &mut kernel,
            &units,
            &viewers,
            |unit| unit != "a.service",
        );
        let kept: usize = kernel.values().sum();
        assert!(
            kept <= focus::MOST,
            "{kept} hard floors hold once a restarted sorter's give-back from a failed: {kernel:?}"
        );
    }

    fn settings() -> Settings {
        Settings {
            under: vec!["app.slice".into()],
            homes: vec!["kitty-".into()],
            keep: vec!["pi".into(), "git-engine".into()],
            jobs: "jobs.slice/jobs.service".into(),
            busy_ms: 500,
            ticks: 2,
            every: Duration::from_millis(1000),
            split: vec!["jobs.slice/jobs.service".into()],
            no_real_time: vec!["jobs.slice/jobs-work.slice".into()],
            quick: Vec::new(),
            quick_names: Vec::new(),
            by_quest: vec!["jobs-work.slice".into()],
            quests: None,
            tiers: None,
            parents: None,
            tier_order: tiers::Order::default(),
            background: None,
            low_weight: 10,
            weigh: Vec::new(),
            revive: None,
            orchestrators: None,
            door: None,
            calls: None,
            calls_elsewhere: None,
            niri: None,
            hosts: None,
            conversations: None,
            slots: None,
            sessions: None,
            sends: vec![None],
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

    /// What writing `moved` processes and refusing `stayed` came to.
    fn sent(moved: usize, stayed: usize) -> Sent {
        Sent {
            moved,
            stayed,
            failed: (stayed > 0).then(|| (31, io::Error::from_raw_os_error(13))),
        }
    }

    /// A folder of a test's own, removed when it ends, however it ends.
    struct OwnDir(PathBuf);

    impl Drop for OwnDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A folder of this test's own, refused rather than reused if its
    /// name is there already.
    fn own_dir(name: &str) -> OwnDir {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{}-{nanos}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        OwnDir(dir)
    }

    #[test]
    fn sending_counts_what_went_and_names_the_first_that_failed() {
        let own = own_dir("sent");
        let into = &own.0;
        let went = send(into, &[30, 31]);
        assert_eq!((went.moved, went.stayed), (2, 0));
        assert!(
            went.failed.is_none(),
            "a move that went names no failure: {:?}",
            went.failed
        );
        let gone = send(&into.join("gone"), &[30, 31]);
        assert_eq!((gone.moved, gone.stayed), (0, 2));
        assert_eq!(gone.failed.as_ref().map(|(pid, _)| *pid), Some(30));
    }

    #[test]
    fn a_process_that_ended_counts_as_neither_moved_nor_stayed() {
        let sent = sent_by(&[30, 31, 32], |pid| match pid {
            30 => Err(io::Error::from_raw_os_error(3)),
            31 => Err(io::Error::from_raw_os_error(13)),
            _ => Ok(()),
        });
        assert_eq!((sent.moved, sent.stayed), (1, 1));
        assert_eq!(sent.failed.as_ref().map(|(pid, _)| *pid), Some(31));
    }

    #[test]
    fn a_busy_move_is_said_once_it_went_and_its_failure_once() {
        let mut sorter = Sorter::default();
        let settings = settings();
        let own = own_dir("jobs");
        let jobs = own.0.clone();
        let gone = jobs.join("gone");
        let busy = |sorter: &mut Sorter, frame: &[Sample], into: &Path| {
            let now = sorter.last + 100;
            let says = sorter.busy_moved("kitty-1-1.scope", frame, into, now, &settings);
            sorter.finish(&frame.iter().map(|proc| proc.pid).collect(), now);
            says
        };
        busy(&mut sorter, &window(0, 0), &jobs);
        busy(&mut sorter, &window(90, 1), &gone);
        let refused = busy(&mut sorter, &window(180, 2), &gone);
        assert!(
            refused.len() == 1 && refused[0].starts_with("Couldn't move make"),
            "a failed move says it failed: {refused:?}"
        );
        busy(&mut sorter, &window(270, 3), &gone);
        let again = busy(&mut sorter, &window(360, 4), &gone);
        assert!(again.is_empty(), "a failure said again: {again:?}");
        busy(&mut sorter, &window(450, 5), &jobs);
        let moved = busy(&mut sorter, &window(540, 6), &jobs);
        assert!(
            moved.len() == 1 && moved[0].starts_with("Moved make"),
            "a move that went says it moved: {moved:?}"
        );
    }

    #[test]
    fn a_refusal_is_forgotten_once_its_scope_is_no_longer_looked_at() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        let found = &moves[2][0];
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a first refusal is said"
        );
        let settings = settings();
        let now = sorter.last + 100;
        sorter.look(
            "kitty-2-1.scope",
            &[proc(40, 1, 40, "zsh", 1)],
            now,
            &settings,
        );
        assert_eq!(
            sorter.said(found, &sent(0, 2)),
            None,
            "another scope's look forgot it"
        );
        sorter.forget_unlooked(&HashSet::from([
            "kitty-1-1.scope".to_owned(),
            "kitty-2-1.scope".to_owned(),
        ]));
        assert_eq!(
            sorter.said(found, &sent(0, 2)),
            None,
            "a scope still looked at lost its refusal"
        );
        sorter.forget_unlooked(&HashSet::from(["kitty-2-1.scope".to_owned()]));
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a scope no longer looked at kept its refusal"
        );
    }

    #[test]
    fn a_return_says_how_many_went_and_a_failure_once_while_it_stays() {
        let mut failed = HashSet::new();
        let says = |went: String| format!("Moved ({went})");
        let refused = |why: String| format!("Refused, as {why}");
        assert_eq!(
            returned(&mut failed, "a", &sent(3, 0), says, refused).as_deref(),
            Some("Moved (3 processes)")
        );
        assert_eq!(returned(&mut failed, "a", &sent(0, 0), says, refused), None);
        let first = returned(&mut failed, "a", &sent(0, 2), says, refused);
        assert_eq!(
            first.as_deref(),
            Some("Refused, as moving 31 failed: Permission denied (os error 13)")
        );
        assert_eq!(
            returned(&mut failed, "a", &sent(0, 2), says, refused),
            None,
            "a failure said twice"
        );
        let some = returned(&mut failed, "a", &sent(1, 1), says, refused).unwrap();
        assert!(
            some.starts_with("Moved (1 of 2 processes, as moving 31"),
            "{some}"
        );
        assert!(
            returned(&mut failed, "a", &sent(0, 2), says, refused).is_some(),
            "a failure after a move that went is said again"
        );
        returned(&mut failed, "a", &sent(2, 0), says, refused);
        assert!(
            returned(&mut failed, "a", &sent(0, 2), says, refused).is_some(),
            "a failure after a move that all went is said again"
        );
    }

    #[test]
    fn a_return_gone_from_jobs_is_forgotten() {
        let mut returns = Returns::default();
        let refused = |why: String| why;
        let none = HashSet::new();
        assert!(
            returned(&mut returns.strays, "s", &sent(0, 1), |w| w, refused).is_some(),
            "a stray's first failure is said"
        );
        returns.strays_found(&HashSet::from(["s".to_owned()]));
        assert_eq!(
            returned(&mut returns.strays, "s", &sent(0, 1), |w| w, refused),
            None,
            "a stray still found was said again"
        );
        returns.strays_found(&none);
        assert!(
            returned(&mut returns.strays, "s", &sent(0, 1), |w| w, refused).is_some(),
            "a stray that went kept its failure"
        );
        let ask = |returns: &mut Returns| {
            returns.asked("pi-1.scope", || "given".to_owned(), || "still".to_owned())
        };
        ask(&mut returns);
        ask(&mut returns);
        for set in [&mut returns.sessions, &mut returns.unmade] {
            set.insert("pi-1.scope".to_owned());
            set.insert("pi-2.scope".to_owned());
        }
        returns.asked.insert("pi-2.scope".to_owned());
        returns.stuck.insert("pi-2.scope".to_owned());
        returns.sessions_found(&HashSet::from(["pi-2.scope".to_owned()]));
        let kept: HashSet<String> = HashSet::from(["pi-2.scope".to_owned()]);
        for (which, set) in [
            ("sessions", &returns.sessions),
            ("asked", &returns.asked),
            ("stuck", &returns.stuck),
            ("unmade", &returns.unmade),
        ] {
            assert_eq!(set, &kept, "{which} kept the wrong sessions");
        }
        returns.sessions_found(&none);
        assert_eq!(
            ask(&mut returns).as_deref(),
            Some("given"),
            "a session that left jobs was remembered"
        );
    }

    #[test]
    fn a_move_counts_what_stayed_not_what_it_left() {
        let mut sorter = Sorter::default();
        let three = |cpu: u64| {
            let mut procs = window(cpu, 0);
            // What make started in a group of its own, as Nix's ssh is.
            procs.push(proc(32, 30, 32, "ssh", 0));
            procs
        };
        let moves = looks(&mut sorter, &[three(0), three(90), three(180)]);
        let found = &moves[2][0];
        assert!(
            found.what.contains("3 processes"),
            "a group names what it takes, its own group's two and what they started: {}",
            found.what
        );
        // One went, one failed, and one had ended.
        let some = sorter.said(found, &sent(1, 1)).unwrap();
        assert!(
            some.ends_with(
                "Only 1 went: 1 stayed where they were, as moving 31 failed: \
                 Permission denied (os error 13)."
            ),
            "{some}"
        );
    }

    #[test]
    fn a_session_with_no_scope_leaves_a_later_failure_to_be_said() {
        let mut returns = Returns::default();
        assert!(
            returns.unmade("pi-1.scope").is_some(),
            "a session with no scope is said"
        );
        assert_eq!(
            returns.unmade("pi-1.scope"),
            None,
            "said again while it stays"
        );
        assert!(
            returned(
                &mut returns.sessions,
                "pi-1.scope",
                &sent(0, 1),
                |w| w,
                |w| w
            )
            .is_some(),
            "a failure into a scope found later went unsaid"
        );
    }

    #[test]
    fn a_name_with_a_newline_stays_on_its_line() {
        let mut sorter = Sorter::default();
        let forged = |cpu: u64| {
            let mut procs = window(cpu, 0);
            procs[2].comm = "make\nMoved".into();
            procs
        };
        let moves = looks(&mut sorter, &[forged(0), forged(90), forged(180)]);
        let found = &moves[2][0];
        assert!(
            !found.says.contains('\n') && found.says.starts_with("Moved make\\nMoved ("),
            "a name broke its line: {:?}",
            found.says
        );
    }

    #[test]
    fn a_session_still_in_jobs_after_its_scope_is_asked_for_is_said_once() {
        let mut returns = Returns::default();
        let ask = |returns: &mut Returns| {
            returns.asked("pi-1.scope", || "given".to_owned(), || "still".to_owned())
        };
        assert_eq!(ask(&mut returns).as_deref(), Some("given"));
        assert_eq!(ask(&mut returns).as_deref(), Some("still"));
        assert_eq!(ask(&mut returns), None, "said again while it stays");
        assert!(
            returned(
                &mut returns.sessions,
                "pi-1.scope",
                &sent(0, 1),
                |w| w,
                |w| w
            )
            .is_some(),
            "a move's failure went unsaid after the session was said to be still in jobs"
        );
    }

    #[test]
    fn a_kept_groups_busy_process_is_refused_once_while_it_runs() {
        let mut sorter = Sorter::default();
        // node (21), under pi (20), whom the settings keep, with a name
        // that tries to start a line of its own.
        let busy = |cpu: u64| {
            let mut procs = window(0, 0);
            procs.push(proc(21, 20, 20, "node\nMoved", cpu));
            procs
        };
        let moves = looks(&mut sorter, &[busy(0), busy(90), busy(180)]);
        let found = &moves[2][0];
        assert_eq!(found.key.1, 21);
        assert!(
            !found.says.contains('\n') && found.what.starts_with("node\\nMoved (21)"),
            "a kept group's process broke its line: {:?}",
            found.says
        );
        assert!(
            sorter.said(found, &sent(0, 1)).is_some(),
            "a first refusal is said"
        );
        looks(&mut sorter, &[busy(270)]);
        assert_eq!(
            sorter.said(found, &sent(0, 1)),
            None,
            "said again while it runs"
        );
        looks(&mut sorter, &[window(0, 0)]);
        assert!(
            sorter.said(found, &sent(0, 1)).is_some(),
            "a refusal outlived its process"
        );
    }

    #[test]
    fn a_move_says_so_only_when_a_process_went() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        let found = &moves[2][0];
        assert_eq!(
            sorter.said(found, &sent(2, 0)).as_deref(),
            Some(found.says.as_str())
        );
        assert_eq!(
            sorter.said(found, &sent(0, 0)),
            None,
            "every process had ended"
        );
        let some = sorter.said(found, &sent(1, 1)).unwrap();
        assert!(
            some.starts_with(&found.says) && some.contains("1 stayed where they were"),
            "a move that took only some says how many stayed: {some}"
        );
    }

    #[test]
    fn a_move_that_goes_lets_a_later_refusal_be_said() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        let found = &moves[2][0];
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a first refusal is said"
        );
        let some = sorter.said(found, &sent(1, 1)).unwrap();
        assert!(some.contains("1 stayed where they were"), "{some}");
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a refusal after a move that went is said again"
        );
        sorter.said(found, &sent(2, 0));
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a refusal after a move that all went is said again"
        );
    }

    #[test]
    fn a_refusal_outlives_its_groups_leader() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        let found = &moves[2][0];
        assert!(
            sorter.said(found, &sent(0, 1)).is_some(),
            "a first refusal is said"
        );
        // make (30) has ended; cc1 (31) runs on in its group.
        let leaderless = vec![proc(10, 1, 10, "zsh", 5), proc(31, 1, 30, "cc1", 270)];
        looks(&mut sorter, &[leaderless]);
        assert_eq!(
            sorter.said(found, &sent(0, 1)),
            None,
            "said again without its leader"
        );
    }

    #[test]
    fn a_refused_move_is_said_once_while_its_group_runs() {
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[window(0, 0), window(90, 1), window(180, 2)]);
        let found = &moves[2][0];
        let first = sorter.said(found, &sent(0, 2)).unwrap();
        assert!(
            first.starts_with("Couldn't move make (process group 30, 2 processes)")
                && first.contains("moving 31 failed"),
            "a refusal names the group and why: {first}"
        );
        assert_eq!(
            sorter.said(found, &sent(0, 2)),
            None,
            "said again while it runs"
        );
        looks(&mut sorter, &[window(270, 3)]);
        assert_eq!(
            sorter.said(found, &sent(0, 2)),
            None,
            "forgotten while it runs"
        );
        looks(&mut sorter, &[vec![proc(10, 1, 10, "zsh", 5)]]);
        assert!(
            sorter.said(found, &sent(0, 2)).is_some(),
            "a group that ended and came back is said of again"
        );
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
    fn a_hosts_commands_are_looked_at_beside_the_terminals() {
        let slice = std::env::temp_dir().join(format!("jobs-sorter-hosts-{}", std::process::id()));
        for unit in [
            "kitty-1-1.scope",
            "pi-host-work-background.service",
            "sessions-kitty.service",
            "mycelium.service",
        ] {
            fs::create_dir_all(slice.join(unit)).expect("a unit's folder");
        }
        let mut looked: Vec<String> = looked_at(&slice, &["kitty-".into()], Some("pi-host-"))
            .iter()
            .filter_map(|path| Some(path.file_name()?.to_str()?.to_owned()))
            .collect();
        looked.sort_unstable();
        let _ = fs::remove_dir_all(&slice);
        assert_eq!(
            looked,
            ["kitty-1-1.scope", "pi-host-work-background.service"],
            "a host's unit is looked at as a terminal's scope is, and nothing else is"
        );
    }

    // From 00:19 on 15 November a host's command evaluated in the host's
    // own unit, its shell's parent outside it, as the harness's kept
    // command wrapper leaves it, while the host and its git engine idled.
    #[test]
    fn a_hosts_busy_command_moves_whole_and_the_host_stays() {
        let frame = |cpu: u64| {
            vec![
                proc(10, 1, 10, "node", 1),
                proc(11, 10, 10, "git-engine", cpu),
                proc(21, 20, 20, "tee", 0),
                proc(22, 20, 20, "bash", 0),
                proc(23, 22, 20, "nix", cpu),
                proc(24, 23, 24, "ssh", 0),
            ]
        };
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[frame(0), frame(90), frame(180)]);
        assert_eq!(moves[2].len(), 1, "one move: {:?}", moves[2]);
        let mut pids = moves[2][0].pids.clone();
        pids.sort_unstable();
        assert_eq!(
            pids,
            [21, 22, 23, 24],
            "the command goes whole, what it started in a group of its own with it, \
             and the host and its busy git engine stay"
        );
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
    fn a_restart_counts_the_hard_floors_memory_min_shows_on_pis_alone() {
        let slice = std::env::temp_dir()
            .join(format!("held-before-{}", std::process::id()))
            .join("agents.slice");
        let hard = (holds::HARD).to_string();
        // Each scope a sorter before held, with its memory.min, or none where it can't be read.
        for (scope, min) in [
            ("kitty-1-9.scope", Some("0")),
            ("mycelium-pi-1.scope", Some(hard.as_str())),
            ("kitty-2-9.scope", Some(hard.as_str())),
            ("mycelium-pi-2.scope", Some("0")),
            ("kitty-3-9.scope", Some(hard.as_str())),
            ("mycelium-pi-3.scope", None),
            ("kitty-4-9.scope", Some(hard.as_str())),
        ] {
            let dir = slice.join(scope);
            fs::create_dir_all(&dir).unwrap();
            // A floor of max was set by someone else, so it isn't a hold to take up.
            let low = if scope == "kitty-4-9.scope" {
                "max".to_owned()
            } else {
                focus::FLOOR.to_string()
            };
            fs::write(dir.join("memory.low"), low).unwrap();
            if let Some(min) = min {
                fs::write(dir.join("memory.min"), min).unwrap();
            }
        }
        let prefixes = ["kitty-".to_owned(), "mycelium-pi-".to_owned()];
        let holding = held_before(
            std::slice::from_ref(&slice),
            &prefixes,
            &["mycelium-pi-".to_owned()],
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            holding.held(),
            6,
            "every scope with the floor is taken up, and not the one whose floor is max"
        );
        assert_eq!(
            holding.hard(),
            2,
            "the pi whose memory.min shows a floor and the one whose can't be read, not the \
             windows' own scopes or the pi held soft"
        );
    }

    #[test]
    fn the_unit_joel_is_at_is_fitted_first_into_what_the_slices_floor_leaves() {
        const MIB: u64 = 1 << 20;
        let slice = std::env::temp_dir()
            .join(format!("fit-floors-{}", std::process::id()))
            .join("agents.slice");
        // Each child as what it holds and its floor: a window, a scope any agent could start
        // with a floor of max, and the two units, whose floors as set last look must not count
        // against them.
        for (child, current, low) in [
            ("kitty-1-1.scope", 1024 * MIB, "2147483648".to_owned()),
            ("run-u7.scope", 2048 * MIB, "max".to_owned()),
            ("pi-host-a.service", 2048 * MIB, (4096 * MIB).to_string()),
            (
                "pi-conversation-b@1.service",
                2048 * MIB,
                (4096 * MIB).to_string(),
            ),
        ] {
            let dir = slice.join(child);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("memory.current"), current.to_string()).unwrap();
            fs::write(dir.join("memory.low"), low).unwrap();
        }
        fs::write(slice.join("memory.low"), (4096 * MIB).to_string()).unwrap();
        let ranked_at = |unit: &str, own: u64, floor: u64, signs: hosts::Signs| Ranked {
            path: slice.join(unit),
            rank: hosts::Rank {
                weight: 100,
                floor,
                hard: 0,
                writeback: hosts::writeback(true, &signs, floor),
                preference: "none".to_owned(),
            },
            asked: floor,
            signs,
            own,
            own_hard: 0,
            writeback: true,
        };
        let shown = hosts::Signs {
            shown: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let viewed = hosts::Signs { viewed: 1, ..shown };
        let mut ranked = [
            ranked_at("pi-host-a.service", 0, 512 * MIB, shown),
            ranked_at("pi-conversation-b@1.service", 256 * MIB, 2304 * MIB, viewed),
        ];
        let mut fitting = Fitting::default();
        fit_floors(
            &mut ranked,
            &["kitty-".to_owned()],
            &hosts::Hosts::default(),
            &mut fitting,
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            [ranked[0].rank.floor, ranked[1].rank.floor],
            [0, 2304 * MIB],
            "of the 3 GiB the window leaves, the unit Joel is at keeps its whole floor, ahead of \
             the scope no window holds, and the unit only shown gets none"
        );
        assert!(
            fitting.overrun.contains(&slice.display().to_string()),
            "the scope claims its 2 GiB of the 768 MiB left, past the slice's floor, which is said"
        );
        assert!(
            !ranked[1].rank.writeback,
            "the unit Joel is at, fitting under its floor, has writeback off"
        );
    }

    #[test]
    fn others_claiming_past_the_room_left_them_are_said_once_each_time() {
        const MIB: u64 = 1 << 20;
        let mut fitting = Fitting::default();
        let strays = [
            ("run-u7.scope".to_owned(), 2048 * MIB),
            ("run-u8.scope".to_owned(), 0),
        ];
        let said = fitting.overran("agents.slice", &strays, 2048 * MIB, 768 * MIB);
        assert_eq!(
            said.as_deref(),
            Some(
                "The others in agents.slice claim 2048 MiB, past the 768 MiB the units Joel is at \
                 leave them, so its floor is shared among every child again, the windows too: \
                 \"run-u7.scope\" 2048 MiB."
            ),
            "it says which children overran, quoted, and by how much"
        );
        assert_eq!(
            fitting.overran("agents.slice", &strays, 2048 * MIB, 768 * MIB),
            None,
            "and once while it lasts"
        );
        assert_eq!(
            fitting.overran("agents.slice", &strays, 767 * MIB, 768 * MIB),
            None,
            "others fitting in what's left say nothing"
        );
        assert_eq!(
            fitting.overran("agents.slice", &strays, 2048 * MIB, 768 * MIB),
            None,
            "others hovering within a step of the edge don't say it again on each crossing"
        );
        fitting.overran("agents.slice", &strays, 704 * MIB, 768 * MIB);
        assert!(
            fitting
                .overran("agents.slice", &strays, 2048 * MIB, 768 * MIB)
                .is_some(),
            "once they fit by a step and overrun again, it's said again"
        );
        let many: Vec<(String, u64)> = (0..9)
            .map(|n| (format!("run-u{n}.scope"), 256 * MIB))
            .collect();
        let said = Fitting::default()
            .overran("agents.slice", &many, 2304 * MIB, 768 * MIB)
            .unwrap_or_default();
        assert!(
            said.ends_with("\"run-u4.scope\" 256 MiB, and 4 more."),
            "a long list of others was said whole: {said}"
        );
    }

    #[test]
    fn others_that_shrink_well_inside_the_room_left_them_clear_the_overrun_mark() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let stray = "run-u1.service";
        let slice = slice_with(
            "fit-clears",
            2048 * MIB,
            &[(unit, 512 * MIB, "0"), (stray, 2048 * MIB, "max")],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(512 * MIB),
            ..hosts::Signs::default()
        };
        let mut fitting = Fitting::default();
        let mut marked = Vec::new();
        for stray_mib in [2048, 256, 2048] {
            fs::write(
                slice.join(stray).join("memory.current"),
                (stray_mib * MIB).to_string(),
            )
            .unwrap();
            let mut ranked = [ranked_in(&slice, unit, 0, 512 * MIB, viewed)];
            fit_floors(
                &mut ranked,
                &["kitty-".to_owned()],
                &hosts::Hosts::default(),
                &mut fitting,
                std::time::Instant::now(),
            );
            marked.push(!fitting.overrun.is_empty());
        }
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            marked,
            [true, false, true],
            "others that fit with room to spare kept the overrun mark, so their next overrun \
             went unsaid"
        );
    }

    #[test]
    fn a_slice_left_out_of_a_look_forgets_it_overran() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let slice = slice_with("fit-forget", 2048 * MIB, &[(unit, 512 * MIB, "0")]);
        let mut fitting = Fitting::default();
        fitting.overrun.insert("/elsewhere/agents.slice".to_owned());
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(512 * MIB),
            ..hosts::Signs::default()
        };
        let mut ranked = [ranked_in(&slice, unit, 0, 512 * MIB, viewed)];
        fit_floors(
            &mut ranked,
            &["kitty-".to_owned()],
            &hosts::Hosts::default(),
            &mut fitting,
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert!(
            fitting.overrun.is_empty(),
            "a slice no look fits still counts as overrun, so its next overrun goes unsaid: {:?}",
            fitting.overrun
        );
    }

    /// The floors `fit_floors` gives the units in `slice`, each as ranked by `units`, at each of
    /// `looks`, after `before` changes the slice for that look, in MiB.
    fn floors_over(
        slice: &Path,
        units: &[(&str, u64, hosts::Signs)],
        looks: &[u64],
        before: impl Fn(u64),
    ) -> Vec<Vec<u64>> {
        let mut fitting = Fitting::default();
        let start = std::time::Instant::now();
        looks
            .iter()
            .map(|&secs| {
                before(secs);
                let mut ranked: Vec<Ranked> = units
                    .iter()
                    .map(|&(unit, floor, signs)| ranked_in(slice, unit, 0, floor, signs))
                    .collect();
                fit_floors(
                    &mut ranked,
                    &["kitty-".to_owned()],
                    &hosts::Hosts::default(),
                    &mut fitting,
                    start + Duration::from_secs(secs),
                );
                ranked.iter().map(|unit| unit.rank.floor >> 20).collect()
            })
            .collect()
    }

    #[test]
    fn a_window_given_back_its_floor_gives_the_units_its_room_at_once() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let slice = slice_with(
            "fit-lowered",
            3072 * MIB,
            &[
                ("kitty-1-1.scope", 2048 * MIB, "2147483648"),
                (unit, 2048 * MIB, "0"),
            ],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let floors = floors_over(&slice, &[(unit, 2560 * MIB, viewed)], &[0, 30], |secs| {
            if secs == 30 {
                fs::write(
                    slice.join("kitty-1-1.scope/memory.low"),
                    (512 * MIB).to_string(),
                )
                .unwrap();
            }
        });
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            floors,
            [[1024], [2560]],
            "a window whose floor the sorter lowered still claimed its old peak"
        );
    }

    #[test]
    fn what_a_window_claims_past_a_windows_floor_is_a_strays_claim_and_not_held() {
        const MIB: u64 = 1 << 20;
        let (viewed_unit, shown_unit) = ("pi-conversation-b@1.service", "pi-host-a.service");
        let slice = slice_with(
            "fit-past",
            6144 * MIB,
            &[
                ("kitty-9-9.scope", 3072 * MIB, "max"),
                (viewed_unit, 2048 * MIB, "0"),
                (shown_unit, 2048 * MIB, "0"),
            ],
        );
        let shown = hosts::Signs {
            shown: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let viewed = hosts::Signs { viewed: 1, ..shown };
        let units = [
            (viewed_unit, 2048 * MIB, viewed),
            (shown_unit, 2048 * MIB, shown),
        ];
        let floors = floors_over(&slice, &units, &[0, 30], |secs| {
            if secs == 30 {
                fs::write(
                    slice.join("kitty-9-9.scope/memory.current"),
                    (2048 * MIB).to_string(),
                )
                .unwrap();
            }
        });
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            floors[0],
            [2048, 1024],
            "the 1 GiB the window claims past a window's floor wasn't fitted after the unit Joel \
             is at and before the rest"
        );
        assert_eq!(
            floors[1],
            [2048, 2048],
            "what the window claimed past a window's floor was held for a minute, as a \
             window's own is, though a stray's never is"
        );
    }

    #[test]
    fn within_the_cap_held_floors_claim_first_then_kept_ones_then_scopes_only_named() {
        const MIB: u64 = 1 << 20;
        let held = ["kitty-1-1.scope", "mycelium-pi-1.scope"];
        let named = [
            "kitty-0-1.scope",
            "kitty-0-2.scope",
            "kitty-0-3.scope",
            "kitty-0-4.scope",
        ];
        let children: Vec<(&str, u64, &str)> = held
            .iter()
            .map(|&name| (name, 1024 * MIB, "max"))
            .chain(named.iter().map(|&name| (name, 2048 * MIB, "max")))
            .chain([("kitty-1-9.scope", 512 * MIB, "536870912")])
            .collect();
        let slice = slice_with("fit-floored-first", 32768 * MIB, &children);
        let floored = Floored {
            held: held.iter().map(|name| slice.join(name)).collect(),
            kept: ["kitty-1-9.scope".to_owned()].into(),
        };
        let (room, strays) = claims(
            &slice,
            32768 * MIB,
            &["kitty-".to_owned(), "mycelium-pi-".to_owned()],
            &[],
            &floored,
            &mut budget::Peaks::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        let past: Vec<&str> = strays
            .iter()
            .filter(|(_, claim)| *claim > 0)
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            room,
            32768 * MIB - focus::FLOOR * focus::MOST as u64,
            "the windows together took the units Joel is at more than the cap"
        );
        // 2 GiB held and 512 MiB kept leave the named 3.5 GiB of the cap: all
        // of the first, 1.5 GiB of the second, and none of the rest.
        assert_eq!(
            past,
            ["kitty-0-2.scope", "kitty-0-3.scope", "kitty-0-4.scope"],
            "a held window's floor or its pi's didn't come first within the cap, the one keep.rs \
             keeps didn't come next, or scopes only named as windows weren't capped by name"
        );
    }

    // Held scopes are a window and its pi, so up to twice focus::MOST, and
    // more while a give-back fails: outside the cap, six at their floor took
    // 12 GiB ahead of the units Joel is at, against a cap of 6 sized to fit
    // the slice's floor.
    #[test]
    fn held_floors_past_the_cap_claim_after_the_units_joel_is_at() {
        const MIB: u64 = 1 << 20;
        let held = [
            "kitty-1-1.scope",
            "mycelium-pi-1.scope",
            "kitty-1-2.scope",
            "mycelium-pi-2.scope",
            "kitty-1-3.scope",
            "mycelium-pi-3.scope",
        ];
        let children: Vec<(&str, u64, &str)> =
            held.iter().map(|&name| (name, 2048 * MIB, "max")).collect();
        let slice = slice_with("fit-held-capped", 32768 * MIB, &children);
        let floored = Floored {
            held: held.iter().map(|name| slice.join(name)).collect(),
            kept: HashSet::new(),
        };
        let (room, strays) = claims(
            &slice,
            32768 * MIB,
            &["kitty-".to_owned(), "mycelium-pi-".to_owned()],
            &[],
            &floored,
            &mut budget::Peaks::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            room,
            32768 * MIB - focus::FLOOR * focus::MOST as u64,
            "held floors took the units Joel is at more than the windows' cap"
        );
        let past: u64 = strays.iter().map(|(_, claim)| claim).sum();
        assert_eq!(
            past,
            focus::FLOOR * focus::MOST as u64,
            "what the held floors claim past the cap isn't claimed after the units"
        );
    }

    // keep.rs floors every window running a pi, however many there are, so
    // what it keeps can't claim ahead of the units Joel is at without a
    // bound: on 21 November 17 of his 20 windows ran a pi.
    #[test]
    fn the_floors_keep_rs_keeps_claim_first_within_the_cap_however_many() {
        const MIB: u64 = 1 << 20;
        let kept: Vec<String> = (10..30).map(|n| format!("kitty-2-{n}.scope")).collect();
        let children: Vec<(&str, u64, &str)> = kept
            .iter()
            .map(|name| (name.as_str(), 512 * MIB, "536870912"))
            .chain([("kitty-0-1.scope", 2048 * MIB, "max")])
            .collect();
        let slice = slice_with("fit-kept-capped", 32768 * MIB, &children);
        let floored = Floored {
            held: HashSet::new(),
            kept: kept.iter().cloned().collect(),
        };
        let (room, strays) = claims(
            &slice,
            32768 * MIB,
            &["kitty-".to_owned()],
            &[],
            &floored,
            &mut budget::Peaks::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        let past: Vec<&str> = strays
            .iter()
            .filter(|(_, claim)| *claim > 0)
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(
            room,
            32768 * MIB - focus::FLOOR * focus::MOST as u64,
            "the floors keep.rs keeps took the units Joel is at more than the windows' cap"
        );
        assert_eq!(
            past.len(),
            9,
            "twelve kept floors fit the cap, and the rest and the named scope claim after the \
             units: {past:?}"
        );
        assert!(
            past.contains(&"kitty-0-1.scope"),
            "a scope only named as a window came before the floors keep.rs keeps: {past:?}"
        );
    }

    #[test]
    fn a_look_fits_the_floors_beside_what_the_sorter_holds_and_keeps_now() {
        let mut ranks = Ranks::default();
        let held = Path::new("/sys/fs/cgroup/agents.slice/mycelium-pi-1.scope");
        let kept = Path::new("/sys/fs/cgroup/agents.slice/kitty-1-9.scope");
        let other = Path::new("/sys/fs/cgroup/agents.slice/kitty-0-1.scope");
        ranks.holding.done(held, Some(0));
        ranks.keeping.done("kitty-1-9.scope", keep::Floor::Pi);
        fit_ranked(&mut [], &[], &mut ranks, std::time::Instant::now());
        assert!(
            ranks.fitting.floored.held.contains(held) && ranks.fitting.floored.keeps(kept),
            "a look fitted the floors without the scopes the sorter holds and keeps"
        );
        assert!(
            !ranks.fitting.floored.held.contains(other) && !ranks.fitting.floored.keeps(other),
            "a scope the sorter neither holds nor keeps was taken as one it floors"
        );
    }

    #[test]
    fn scopes_named_as_windows_claim_no_more_than_the_windows_the_sorter_holds() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let slice = slice_with(
            "fit-windows",
            8192 * MIB,
            &[
                ("kitty-1-1.scope", 2048 * MIB, "max"),
                ("kitty-1-2.scope", 2048 * MIB, "max"),
                ("kitty-1-3.scope", 2048 * MIB, "max"),
                ("kitty-1-4.scope", 2048 * MIB, "max"),
                (unit, 2048 * MIB, "0"),
            ],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let floors = floors_over(&slice, &[(unit, 2048 * MIB, viewed)], &[0], |_| {});
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            floors,
            [[2048]],
            "four scopes named as windows took the unit's floor, though the sorter holds at \
             most {} windows",
            focus::MOST
        );
    }

    /// A slice under the temporary folder, named for `name`, with the floor `low` and each of
    /// `children` as its name, what it holds and its memory.low.
    fn slice_with(name: &str, low: u64, children: &[(&str, u64, &str)]) -> PathBuf {
        let slice = std::env::temp_dir()
            .join(format!("{name}-{}", std::process::id()))
            .join("agents.slice");
        let _ = fs::remove_dir_all(&slice);
        for (child, current, floor) in children {
            let dir = slice.join(child);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("memory.current"), current.to_string()).unwrap();
            fs::write(dir.join("memory.low"), floor).unwrap();
        }
        fs::write(slice.join("memory.low"), low.to_string()).unwrap();
        slice
    }

    /// `unit` in `slice` as ranked with its unit file's floor `own`, the floor `floor` before
    /// it's fitted, and `signs`, its writeback as that floor gives it.
    fn ranked_in(slice: &Path, unit: &str, own: u64, floor: u64, signs: hosts::Signs) -> Ranked {
        Ranked {
            path: slice.join(unit),
            rank: hosts::Rank {
                weight: 100,
                floor,
                hard: 0,
                writeback: hosts::writeback(true, &signs, floor),
                preference: "none".to_owned(),
            },
            asked: floor,
            signs,
            own,
            own_hard: 0,
            writeback: true,
        }
    }

    #[test]
    fn a_unit_joel_is_at_trimmed_under_what_it_holds_writes_back_again() {
        const MIB: u64 = 1 << 20;
        let slice = slice_with(
            "fit-writeback",
            2048 * MIB,
            &[
                ("kitty-1-1.scope", 1024 * MIB, "2147483648"),
                ("pi-host-a.service", 512 * MIB, "0"),
                ("pi-conversation-b@1.service", 2048 * MIB, "0"),
            ],
        );
        let shown = hosts::Signs {
            shown: 1,
            current: Some(512 * MIB),
            ..hosts::Signs::default()
        };
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..shown
        };
        let mut ranked = [
            ranked_in(&slice, "pi-host-a.service", 0, 512 * MIB, shown),
            ranked_in(&slice, "pi-conversation-b@1.service", 0, 2048 * MIB, viewed),
        ];
        assert!(
            !ranked[1].rank.writeback,
            "the unit Joel is at holds no more than the floor it asks, so starts with writeback off"
        );
        fit_floors(
            &mut ranked,
            &["kitty-".to_owned()],
            &hosts::Hosts::default(),
            &mut Fitting::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            ranked[1].rank.floor,
            1024 * MIB,
            "it's trimmed to the room the window leaves"
        );
        assert!(
            ranked[1].rank.writeback,
            "holding past the floor it's fitted to, it writes back again"
        );
    }

    #[test]
    fn a_floor_set_last_look_within_a_step_is_kept_less_the_units_own() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let slice = slice_with(
            "fit-last",
            4096 * MIB,
            &[
                ("kitty-1-1.scope", 1024 * MIB, "2147483648"),
                (unit, 2048 * MIB, "0"),
            ],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let own = 256 * MIB;
        // The 3 GiB the window leaves, less what the unit holds of its own floor, fits 2816 MiB
        // of the 4 GiB it would add; a step less, 2752.
        let mut hosts = hosts::Hosts::default();
        let last = own + 2752 * MIB;
        let mut set = ranked_in(&slice, unit, own, last, viewed).rank;
        set.floor = last;
        hosts.done(unit, set, 0);
        let mut ranked = [ranked_in(&slice, unit, own, own + 4096 * MIB, viewed)];
        fit_floors(
            &mut ranked,
            &["kitty-".to_owned()],
            &hosts,
            &mut Fitting::default(),
            std::time::Instant::now(),
        );
        let mut fresh = [ranked_in(&slice, unit, own, own + 4096 * MIB, viewed)];
        fit_floors(
            &mut fresh,
            &["kitty-".to_owned()],
            &hosts::Hosts::default(),
            &mut Fitting::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            fresh[0].rank.floor,
            own + 2816 * MIB,
            "with nothing set before, the unit is given all that fits"
        );
        assert_eq!(
            ranked[0].rank.floor, last,
            "a floor set within a step of what fits is kept, its own floor counted once"
        );
    }

    #[test]
    fn a_scope_named_like_a_window_claims_no_more_than_a_windows_floor() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        // A scope any agent could start, named as a window, holding 3 GiB under a floor of max.
        let slice = slice_with(
            "fit-window-max",
            4096 * MIB,
            &[
                ("kitty-9-9.scope", 3072 * MIB, "max"),
                (unit, 2048 * MIB, "0"),
            ],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let mut ranked = [ranked_in(&slice, unit, 0, focus::FLOOR, viewed)];
        fit_floors(
            &mut ranked,
            &["kitty-".to_owned()],
            &hosts::Hosts::default(),
            &mut Fitting::default(),
            std::time::Instant::now(),
        );
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            ranked[0].rank.floor,
            focus::FLOOR,
            "the unit Joel is at keeps its floor: past a window's own, the scope claims as a \
             stray does, after it"
        );
    }

    #[test]
    fn a_window_paging_out_gives_the_units_its_room_only_after_a_minute() {
        const MIB: u64 = 1 << 20;
        let unit = "pi-conversation-b@1.service";
        let slice = slice_with(
            "fit-peak",
            3072 * MIB,
            &[
                ("kitty-1-1.scope", 2048 * MIB, "2147483648"),
                (unit, 2048 * MIB, "0"),
            ],
        );
        let viewed = hosts::Signs {
            viewed: 1,
            current: Some(2048 * MIB),
            ..hosts::Signs::default()
        };
        let mut fitting = Fitting::default();
        let start = std::time::Instant::now();
        let mut floors = Vec::new();
        for (secs, window) in [(0, 2048), (30, 512), (61, 512)] {
            fs::write(
                slice.join("kitty-1-1.scope/memory.current"),
                (window * MIB).to_string(),
            )
            .unwrap();
            let mut ranked = [ranked_in(&slice, unit, 0, 2048 * MIB, viewed)];
            let now = start + Duration::from_secs(secs);
            fit_floors(
                &mut ranked,
                &["kitty-".to_owned()],
                &hosts::Hosts::default(),
                &mut fitting,
                now,
            );
            floors.push(ranked[0].rank.floor / MIB);
        }
        fs::remove_dir_all(slice.parent().unwrap()).unwrap();
        assert_eq!(
            floors,
            [1024, 1024, 2048],
            "the window's dip inside the minute keeps the unit's floor where it was, and once \
             the minute is out the unit grows into the room"
        );
    }

    #[test]
    fn a_sessions_scope_is_never_a_stray_whoever_started_it() {
        let under = std::env::temp_dir().join(format!("windows-stay-{}", std::process::id()));
        for scope in [
            "kitty-1-1.scope",
            "mycelium-pi-815d37ae.scope",
            "run-u7.scope",
        ] {
            fs::create_dir_all(under.join(scope)).unwrap();
        }
        let given = super::settings(
            "--under agents.slice --weigh mycelium-pi-"
                .split_whitespace()
                .map(str::to_owned),
        )
        .unwrap();
        let found = strays(&under, &given.windows());
        fs::remove_dir_all(&under).unwrap();
        assert_eq!(
            found,
            [under.join("run-u7.scope")],
            "a window's scope was taken for work that strayed from jobs, or a stray was missed"
        );
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
