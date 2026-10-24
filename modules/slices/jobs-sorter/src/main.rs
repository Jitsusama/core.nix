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
//! out of the scope would drop.
//!
//! Work lands in a few services' cgroups, pi's in jobs.service and a
//! terminal's in awaited.service, each one cgroup for every command written
//! there. systemd-oomd, when swap runs out, ends the whole leaf cgroup that
//! holds the most of it, so it would end every session's builds at once
//! rather than the one that filled swap. Each `--split` cgroup's work, a
//! process tree whose root was written there, goes into a scope of its own,
//! work-<pid>-<start>.scope, which systemd drops once it empties. The scopes
//! go in the slice named after `=`, or else the one the unit sits in. Its
//! scopes share that slice's weight, as the work shared the unit's, so a
//! cgroup that sits beside slices of other weights names a slice of its own.
//! A process forked as its tree moved goes to its parent's scope. The
//! unit's own holder stays, so the cgroup is there for the next write.
//!
//! Work under a `--no-real-time` cgroup runs without real time, which
//! `src/real_time.rs` says why and how.
//!
//! Usage: jobs-sorter [--under app.slice] [--home kitty-]... [--keep pi]...
//!        [--jobs jobs.slice/jobs.service] [--busy 500] [--ticks 2]
//!        [--every 1000] [--split jobs.slice/jobs.service[=SLICE]]...
//!        [--no-real-time jobs.slice/jobs-work.slice]...

mod real_time;

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
    under: String,
    homes: Vec<String>,
    keep: Vec<String>,
    jobs: String,
    busy_ms: u64,
    ticks: u32,
    every: Duration,
    split: Vec<String>,
    no_real_time: Vec<String>,
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

/// What busctl is given to start a scope named `name` in `slice` around
/// `pids`: the user manager's `StartTransientUnit`, with its description.
/// A command can exit between being read and being moved, and systemd
/// then fails its scope, which would stay in the failed list for good; so
/// a failed scope is unloaded as a finished one is.
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
        "4",
        "Slice",
        "s",
        slice,
        "Description",
        "s",
        description,
        "CollectMode",
        "s",
        "inactive-or-failed",
        "PIDs",
        "au",
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect();
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

/// Gives each piece of work in one split cgroup a scope of its own. A
/// tree that left before its scope began is no loss; anything else is
/// said once.
fn split(root: &Path, manager: &str, value: &str, holder: &mut Option<u32>, warned: &mut bool) {
    let Some((cgroup, slice, unit)) = slice_and_unit(value) else {
        return;
    };
    let procs = samples(&root.join(cgroup));
    if holder.is_none_or(|pid| !procs.iter().any(|proc| proc.pid == pid)) {
        *holder = main_pid(unit);
    }
    for piece in pieces(&procs, *holder) {
        if let Some(parent) = cgroup_of(piece.ppid, manager).filter(|path| made_here(path)) {
            send(
                &root.join(parent.trim_start_matches('/')),
                &piece.pids,
                warned,
            );
            continue;
        }
        let name = format!("work-{}-{}.scope", piece.root, piece.start);
        let description = format!("{} ({}), work from {unit}", piece.comm, piece.root);
        let result = Command::new("busctl")
            .args(scope_call(&name, slice, &description, &piece.pids))
            .stdout(Stdio::null())
            .output();
        match result {
            Ok(output) if output.status.success() => {}
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

fn settings() -> Result<Settings, String> {
    let mut settings = Settings {
        under: "app.slice".into(),
        homes: Vec::new(),
        keep: Vec::new(),
        jobs: "jobs.slice/jobs.service".into(),
        busy_ms: 500,
        ticks: 2,
        every: Duration::from_millis(1000),
        split: Vec::new(),
        no_real_time: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--under" => settings.under = value()?,
            "--home" => settings.homes.push(value()?),
            "--keep" => settings.keep.push(value()?),
            "--jobs" => settings.jobs = value()?,
            "--split" => settings.split.push(value()?),
            "--no-real-time" => settings.no_real_time.push(value()?),
            "--busy" => settings.busy_ms = value()?.parse().map_err(|_| "--busy is ms")?,
            "--ticks" => settings.ticks = value()?.parse().map_err(|_| "--ticks is a count")?,
            "--every" => {
                let ms = value()?.parse().map_err(|_| "--every is ms")?;
                settings.every = Duration::from_millis(ms);
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if settings.homes.is_empty() {
        settings.homes.push("kitty-".into());
    }
    Ok(settings)
}

fn run() -> Result<(), String> {
    let settings = settings()?;
    let own = fs::read_to_string("/proc/self/cgroup").map_err(|e| e.to_string())?;
    let manager = manager_of(&own).ok_or("not running under a user manager")?;
    let root = Path::new(CGROUPS).join(manager.trim_start_matches('/'));
    let under = root.join(&settings.under);
    let jobs = root.join(&settings.jobs);
    let slice = format!(
        "/{}",
        settings.jobs.split('/').next().unwrap_or(&settings.jobs)
    );
    if !jobs.join("cgroup.procs").exists() {
        return Err(format!("{} doesn't exist", jobs.display()));
    }
    eprintln!(
        "Sorting busy work from {}/{}* into {}.",
        under.display(),
        settings.homes.join("*, "),
        jobs.display()
    );
    let mut sorter = Sorter {
        last: uptime_ticks().unwrap_or(0),
        ..Sorter::default()
    };
    let mut warned = false;
    let mut holders: Vec<Option<u32>> = vec![None; settings.split.len()];
    let mut pass: u32 = 0;
    loop {
        thread::sleep(settings.every);
        let now = uptime_ticks().ok_or("can't read /proc/uptime")?;
        let mut alive = HashSet::new();
        for scope in homes(&under, &settings.homes) {
            let procs = samples(&scope);
            alive.extend(procs.iter().map(|proc| proc.pid));
            let name = scope.file_name().unwrap_or_default().to_string_lossy();
            for found in sorter.look(&name, &procs, now, &settings) {
                send(&jobs, &found.pids, &mut warned);
                eprintln!("{}", found.says);
            }
        }
        for scope in strays(&under, &settings.homes) {
            let procs = samples(&scope);
            if procs.is_empty() || !unlimited(|file| fs::read_to_string(scope.join(file)).ok()) {
                continue;
            }
            let Some(back) = returns_to(&procs, |pid| cgroup_of(pid, &manager), &slice) else {
                continue;
            };
            let pids: Vec<u32> = procs.iter().map(|proc| proc.pid).collect();
            send(&root.join(back.trim_start_matches('/')), &pids, &mut warned);
            let name = scope.file_name().unwrap_or_default().to_string_lossy();
            eprintln!(
                "Moved {name} ({} processes) back to {back}, which started it.",
                pids.len()
            );
        }
        // After the terminals' work is moved, so what moved is split too.
        for (cgroup, holder) in settings.split.iter().zip(holders.iter_mut()) {
            split(&root, &manager, cgroup, holder, &mut warned);
        }
        if pass.is_multiple_of(real_time::EVERY) {
            for cgroup in &settings.no_real_time {
                real_time::take(&root.join(cgroup), &mut warned);
            }
        }
        pass = pass.wrapping_add(1);
        sorter.finish(&alive, now);
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
            under: "app.slice".into(),
            homes: vec!["kitty-".into()],
            keep: vec!["pi".into()],
            jobs: "jobs.slice/jobs.service".into(),
            busy_ms: 500,
            ticks: 2,
            every: Duration::from_millis(1000),
            split: vec!["jobs.slice/jobs.service".into()],
            no_real_time: vec!["jobs.slice/jobs-work.slice".into()],
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
        assert!(moves[0].is_empty());
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
    fn a_scope_is_asked_for_with_its_slice_every_process_and_no_trace_if_it_fails() {
        let args = scope_call("work-40-7.scope", "jobs.slice", "bash (40)", &[40, 41, 42]);
        let tail: Vec<&str> = args[6..].iter().map(String::as_str).collect();
        assert_eq!(
            tail,
            vec![
                "StartTransientUnit",
                "ssa(sv)a(sa(sv))",
                "work-40-7.scope",
                "fail",
                "4",
                "Slice",
                "s",
                "jobs.slice",
                "Description",
                "s",
                "bash (40)",
                "CollectMode",
                "s",
                "inactive-or-failed",
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
