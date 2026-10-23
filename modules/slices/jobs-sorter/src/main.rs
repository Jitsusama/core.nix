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
//! Usage: jobs-sorter [--under app.slice] [--home kitty-]... [--keep pi]...
//!        [--jobs jobs.slice/jobs.service] [--busy 500] [--ticks 2]
//!        [--every 1000]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
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
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--under" => settings.under = value()?,
            "--home" => settings.homes.push(value()?),
            "--keep" => settings.keep.push(value()?),
            "--jobs" => settings.jobs = value()?,
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

    #[test]
    fn a_group_whose_leader_left_can_still_move() {
        let frame = |cpu: u64| vec![proc(10, 1, 10, "zsh", 0), proc(31, 1, 30, "cc1", cpu)];
        let mut sorter = Sorter::default();
        let moves = looks(&mut sorter, &[frame(0), frame(90), frame(180)]);
        assert_eq!(moves[2].len(), 1);
        assert_eq!(moves[2][0].pids, vec![31]);
    }
}
