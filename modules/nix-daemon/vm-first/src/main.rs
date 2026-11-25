//! Puts the checks with deadlines that Nix runs, VM tests first among them,
//! ahead of the builds they share the daemon with.
//!
//! Every build runs as `SCHED_IDLE`, so a VM test's qemu gives way to every
//! program and every other piece of work on the machine, and at the batch
//! policy builds had before, it shared each CPU with the compilers beside it
//! and never preempted one as it woke. Its guest's timers then fire late, and a
//! test's timeouts fail on a busy machine. A container's test and a nested
//! compositor's keep time the same way, with no qemu. So once a second
//! this looks for a process with a `--name`: by default qemu, NixOS's test
//! driver or systemd-nspawn, by what their names start with, or niri or
//! sway by their whole names, since cargo names a niri build's test
//! programs `niri-<hash>`. A name ending in `*` is matched as a prefix.
//! In each build's cgroup Nix makes under `--cgroup`, the daemon's,
//! finding one puts every thread of the build, in the cgroups its containers make too, back to
//! `SCHED_OTHER` at `--nice`. In the daemon's own cgroup, where builds from
//! before it moved aside run, only the processes named move. Threads
//! started afterwards take that from the thread that starts them.
//!
//! Inside one cgroup the CPU is split by nice, but between cgroups by
//! weight, so a build in a cgroup of its own holding such a check is
//! weighed at `--weight` for CPU and disk, against the 100 of every other
//! build.
//! Nix 3.21 makes the builds' cgroups without handing them the
//! controllers to weigh them by, so this hands them on too. Everything
//! outside the daemon's cgroup, what Joel uses, keeps its lead.
//!
//! Any other build is weighed by the quest that asked for it. Each of the
//! daemon's workers serves one client and writes the client's pid in its
//! own command line, which anyone may read, and a build's processes are
//! the worker's children. A client an agent ran sits in its quest's slice,
//! jobs-work-<quest>.slice, which jobs-sorter weighs by the quest's
//! priority, so the build takes that slice's CPU weight, kept below a VM's.
//! A build whose client sits in no quest's slice, or has gone, stays as
//! Nix made it.
//!
//! As each build that wrote anything goes, this says in its journal what
//! the build read and wrote on the disks themselves, from its cgroup's io.stat, with its
//! client's command and quest, so the builds that wear the disk most can
//! be found. A build's own name sits in its environment, which only its
//! user may read, so it's known by what asked for it.
//!
//! `vm-first --help` says its flags; it takes the command kit's shape
//! (docs/commands.md), and what it does goes to its journal on stderr.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use terminal_kit::{Context, Exit, Failure, Help};

mod tally;

use tally::{Tally, tally};

const SCHED_OTHER: i32 = 0;
const PRIO_PROCESS: i32 = 0;
/// errno for a thread that has gone.
const ESRCH: i32 = 3;
/// errno for a cgroup that has gone with its build.
const ENOENT: i32 = 2;
/// errno for a cgroup that still holds processes of its own.
const EBUSY: i32 = 16;
/// Where the cgroup file system is, which a process's cgroup is under.
const CGROUP_ROOT: &str = "/sys/fs/cgroup";
/// How jobs-sorter names a quest's slice, before the quest's folded id.
const QUEST_SLICE: &str = "jobs-work-";
/// How far up from a build's process its worker may be.
const WORKER_DEPTH: usize = 8;

#[repr(C)]
struct SchedParam {
    sched_priority: i32,
}

/// The mark systemd-oomd reads to pass over a cgroup while another can go.
const OOMD_AVOID: &CStr = c"user.oomd_avoid";

unsafe extern "C" {
    fn getxattr(path: *const c_char, name: *const c_char, value: *mut u8, size: usize) -> isize;
    fn setxattr(
        path: *const c_char,
        name: *const c_char,
        value: *const u8,
        size: usize,
        flags: i32,
    ) -> i32;
    fn setpriority(which: i32, who: u32, prio: i32) -> i32;
    fn sched_setscheduler(pid: i32, policy: i32, param: *const SchedParam) -> i32;
}

struct Settings {
    cgroup: PathBuf,
    names: Vec<String>,
    nice: i32,
    weight: u64,
    every: Duration,
}

/// How one thread is scheduled, as /proc/<pid>/task/<tid>/stat says.
#[derive(Debug, PartialEq)]
struct Scheduling {
    nice: i32,
    policy: i32,
}

/// Reads a thread's nice (field 19) and policy (field 41). The name sits
/// in parentheses and may hold anything, so fields are counted from the
/// last ')'.
fn parse_stat(line: &str) -> Option<Scheduling> {
    let close = line.rfind(')')?;
    let rest: Vec<&str> = line[close + 1..].split_whitespace().collect();
    // rest[0] is the state (field 3), so field n is rest[n - 3].
    Some(Scheduling {
        nice: rest.get(19 - 3)?.parse().ok()?,
        policy: rest.get(41 - 3)?.parse().ok()?,
    })
}

/// Reads a process's parent (field 4) from its stat line.
fn parse_ppid(line: &str) -> Option<u32> {
    let close = line.rfind(')')?;
    line[close + 1..]
        .split_whitespace()
        .nth(4 - 3)?
        .parse()
        .ok()
}

/// The client a daemon worker serves, from its command line and the user
/// `owner` it runs as: Nix writes the client's pid over the worker's
/// `--daemon`, padding it with NULs. A worker runs as root and a build's
/// processes never do, so a build naming itself nix-daemon with another
/// client's pid isn't taken for a worker, nor given that client's weight.
fn client_of(cmdline: &[u8], owner: u32) -> Option<u32> {
    if owner != 0 {
        return None;
    }
    let mut args = cmdline.split(|byte| *byte == 0);
    let program = args.next()?;
    if !program
        .rsplit(|byte| *byte == b'/')
        .next()?
        .eq(b"nix-daemon")
    {
        return None;
    }
    std::str::from_utf8(args.next()?).ok()?.parse().ok()
}

/// The quest's slice a process's cgroup sits in, as a path under the
/// cgroup file system, from its `/proc/<pid>/cgroup`.
fn quest_slice(cgroup: &str) -> Option<&str> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let mut end = 0;
    for part in path.split('/') {
        end += part.len();
        let quest = part
            .strip_prefix(QUEST_SLICE)
            .and_then(|rest| rest.strip_suffix(".slice"));
        if quest.is_some_and(|id| !id.is_empty()) {
            return Some(&path[..end]);
        }
        end += 1;
    }
    None
}

/// A quest's build weight: the quest's own, kept below a VM's so a check's
/// VM still goes ahead of every build.
fn quest_weight(quest: u64, vm: u64) -> u64 {
    quest.min(vm.saturating_sub(1)).max(1)
}

/// The client that asked for a build, found through the daemon's worker
/// its processes descend from.
fn client_of_build(build: &Path) -> Option<u32> {
    let procs = fs::read_to_string(build.join("cgroup.procs")).ok()?;
    let mut pid: u32 = procs.lines().next()?.parse().ok()?;
    for _ in 0..WORKER_DEPTH {
        // A process's folder is owned by the user it runs as.
        let owner = fs::metadata(format!("/proc/{pid}")).ok()?.uid();
        if let Some(client) = fs::read(format!("/proc/{pid}/cmdline"))
            .ok()
            .and_then(|cmdline| client_of(&cmdline, owner))
        {
            return Some(client);
        }
        pid = parse_ppid(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)?;
    }
    None
}

/// The CPU weight of the quest that asked for a build, if its client sits
/// in a quest's slice.
fn weight_of_quest(build: &Path) -> Option<u64> {
    let client = client_of_build(build)?;
    let cgroup = fs::read_to_string(format!("/proc/{client}/cgroup")).ok()?;
    let slice = quest_slice(&cgroup)?;
    let weight = format!("{CGROUP_ROOT}{slice}/cpu.weight");
    fs::read_to_string(weight).ok()?.trim().parse().ok()
}

/// What runs in a check with deadlines, whatever it runs its machine on: a
/// VM's qemu, NixOS's test driver, which runs a container's test as it runs
/// a VM's, a container itself, and a nested compositor's niri or sway. Only
/// the daemon's builds are looked in, so Joel's own niri never counts.
/// A name ending in `*` matches as a prefix; niri and sway match whole,
/// since a niri build's own test programs are named `niri-<hash>`.
const TIMED: [&str; 6] = [
    "qemu-system*",
    "qemu-kvm*",
    "nixos-test-driver*",
    "systemd-nspawn*",
    "niri",
    "sway",
];

/// Every process in a cgroup and the cgroups under it, as a container in a
/// build makes its own.
fn pids_under(cgroup: &Path) -> Vec<String> {
    let mut pids: Vec<String> = fs::read_to_string(cgroup.join("cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect();
    if let Ok(entries) = fs::read_dir(cgroup) {
        for entry in entries.filter_map(Result::ok) {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pids.extend(pids_under(&entry.path()));
            }
        }
    }
    pids
}

/// The longest name the kernel keeps for a process.
const COMM_LENGTH: usize = 15;

/// Whether a process's name marks a check with deadlines. Nix's wrapped
/// programs start with a dot, and the kernel cuts a name at 15 characters,
/// so `.nixos-test-dri` is the test driver: a cut name is matched by every
/// prefix that starts with what is left of it. A whole name matches only
/// itself, or Nix's wrapper of it, as `.niri-wrapped` is.
fn is_timed(comm: &str, names: &[String]) -> bool {
    let comm = comm.trim_end();
    let cut = comm.len() == COMM_LENGTH;
    let bare = comm.strip_prefix('.').unwrap_or(comm);
    names.iter().any(|name| match name.strip_suffix('*') {
        Some(prefix) => bare.starts_with(prefix) || (cut && prefix.starts_with(bare)),
        None => bare == name || bare.strip_suffix("-wrapped") == Some(name.as_str()),
    })
}

/// The controllers a build's cgroup is given, as Nix itself does from
/// 2.35 on: CPU and disk to weigh it by, memory and pids to count it by.
const CONTROLLERS: [&str; 4] = ["cpu", "io", "memory", "pids"];

/// What to write to the daemon's `cgroup.subtree_control` to hand its
/// builds the controllers it has and hasn't handed on, or `None` when
/// there is nothing to hand.
fn to_enable(available: &str, enabled: &str) -> Option<String> {
    let available: Vec<&str> = available.split_whitespace().collect();
    let enabled: Vec<&str> = enabled.split_whitespace().collect();
    let missing: Vec<String> = CONTROLLERS
        .iter()
        .filter(|name| available.contains(name) && !enabled.contains(name))
        .map(|name| format!("+{name}"))
        .collect();
    (!missing.is_empty()).then(|| missing.join(" "))
}

/// The cgroups Nix made for its builds, directly under the daemon's.
fn build_cgroups(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("nix-build-")
        })
        .map(|entry| entry.path())
        .collect()
}

/// What a build's cgroup files read once it holds a VM's weight.
fn weight_files(weight: u64) -> [(&'static str, String); 2] {
    [
        ("cpu.weight", weight.to_string()),
        ("io.weight", format!("default {weight}")),
    ]
}

/// Whether a command line is the daemon's own, `nix-daemon --daemon`. Each
/// per-connection worker writes its client's pid over `--daemon`.
fn is_daemon(cmdline: &[u8]) -> bool {
    let mut args = cmdline.split(|&byte| byte == 0);
    let named = args
        .next()
        .and_then(|argv0| argv0.rsplit(|&byte| byte == b'/').next())
        .is_some_and(|name| name == b"nix-daemon");
    named && args.next() == Some(b"--daemon".as_slice())
}

/// Whether the daemon has moved itself into the `nix-daemon` cgroup, given
/// the pids that cgroup holds. Its workers outlive a restart there, so the
/// cgroup is there, and holds processes, before the new daemon is.
fn daemon_aside(pids: &str, cmdline: impl Fn(&str) -> Vec<u8>) -> bool {
    pids.lines().any(|pid| is_daemon(&cmdline(pid)))
}

/// Hands the daemon's builds the controllers, once Nix has moved the daemon
/// into a cgroup of its own beside them, which it does with `use-cgroups`.
/// Until then the service's cgroup holds the daemon, or is about to: systemd
/// starts each new daemon in it, and can't once it has controllers to hand
/// on, so a restart that found them handed would fail to start. Nix 2.35
/// does this itself, and finds it done.
fn hand_controllers(root: &Path, warned: &mut bool) {
    let read = |file: &str| fs::read_to_string(root.join(file)).unwrap_or_default();
    let Some(enable) = to_enable(&read("cgroup.controllers"), &read("cgroup.subtree_control"))
    else {
        return;
    };
    match fs::write(root.join("cgroup.subtree_control"), &enable) {
        Ok(()) => eprintln!("Handed the builds {enable}."),
        // Builds the daemon started before it moved still run in its
        // cgroup: they finish, and a later look hands them on.
        Err(error) if error.raw_os_error() == Some(EBUSY) => {}
        Err(error) if !*warned => {
            eprintln!("Couldn't hand the builds {enable}: {error}");
            *warned = true;
        }
        Err(_) => {}
    }
}

/// Weighs a build, for the VM it holds or the quest that asked for it.
/// The weight ranks it only among the daemon's builds, all in build.slice,
/// so it never comes before what Joel uses.
fn weigh(build: &Path, weight: u64, reason: &str, warned: &mut bool) {
    for (file, value) in weight_files(weight) {
        let path = build.join(file);
        // A controller the daemon hasn't handed on has no file to write.
        let Ok(now) = fs::read_to_string(&path) else {
            continue;
        };
        if now.trim() == value {
            continue;
        }
        match fs::write(&path, &value) {
            Ok(()) => eprintln!("Weighed {} at {value} for its {reason}.", path.display()),
            Err(error) if error.raw_os_error() == Some(ENOENT) => {}
            Err(error) if !*warned => {
                eprintln!("Couldn't weigh {}: {error}", path.display());
                *warned = true;
            }
            Err(_) => {}
        }
    }
}

/// Whether a thread still needs moving ahead.
fn behind(now: &Scheduling, settings: &Settings) -> bool {
    now.policy != SCHED_OTHER || now.nice != settings.nice
}

/// Puts one thread at `SCHED_OTHER` and the nice asked for. Changing policy
/// keeps the nice, so the nice is set after.
fn bring_ahead(tid: u32, nice: i32) -> Result<(), i32> {
    let param = SchedParam { sched_priority: 0 };
    // SAFETY: both calls take plain integers and a pointer to a live
    // struct of the layout the kernel reads.
    let failed = unsafe {
        // A thread id is at most the kernel's pid_max, 2^22, so it reads the
        // same as the i32 the call takes.
        sched_setscheduler(tid.cast_signed(), SCHED_OTHER, &raw const param) != 0
            || setpriority(PRIO_PROCESS, tid, nice) != 0
    };
    if failed {
        return Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0));
    }
    Ok(())
}

/// Brings every thread of one VM ahead, saying how many moved.
fn sort_vm(pid: &str, settings: &Settings, warned: &mut bool) -> usize {
    let Ok(tasks) = fs::read_dir(format!("/proc/{pid}/task")) else {
        return 0;
    };
    let mut moved = 0;
    for task in tasks.filter_map(Result::ok) {
        let Some(tid) = task
            .file_name()
            .to_str()
            .and_then(|tid| tid.parse::<u32>().ok())
        else {
            continue;
        };
        let Some(now) = fs::read_to_string(task.path().join("stat"))
            .ok()
            .and_then(|line| parse_stat(&line))
        else {
            continue;
        };
        if !behind(&now, settings) {
            continue;
        }
        match bring_ahead(tid, settings.nice) {
            Ok(()) => moved += 1,
            Err(ESRCH) => {}
            Err(errno) if !*warned => {
                eprintln!("Couldn't bring thread {tid} of {pid} ahead: errno {errno}");
                *warned = true;
            }
            Err(_) => {}
        }
    }
    moved
}

/// Marks the daemon's cgroup for systemd-oomd to spare while a build can go
/// instead. oomd kills whichever of the service's cgroups reclaims most, and
/// the daemon's going takes every connection with it; it still goes when
/// nothing else can.
fn spare_the_daemon(daemon: &Path, warned: &mut bool) {
    let Ok(path) = CString::new(daemon.as_os_str().as_bytes()) else {
        return;
    };
    let mut value = [0u8; 8];
    // SAFETY: both names are NUL-terminated, and the buffer is as long as
    // said.
    let read = unsafe {
        getxattr(
            path.as_ptr(),
            OOMD_AVOID.as_ptr(),
            value.as_mut_ptr(),
            value.len(),
        )
    };
    if read == 1 && value[0] == b'1' {
        return;
    }
    // SAFETY: as above, the value being one byte long.
    let set = unsafe { setxattr(path.as_ptr(), OOMD_AVOID.as_ptr(), b"1".as_ptr(), 1, 0) };
    if set == 0 {
        eprintln!("Asked systemd-oomd to stop a build before the daemon.");
    } else if !*warned {
        eprintln!(
            "Couldn't ask systemd-oomd to spare the daemon: {}",
            std::io::Error::last_os_error()
        );
        *warned = true;
    }
}

fn look(settings: &Settings, tallies: &mut HashMap<PathBuf, Tally>, warned: &mut bool) {
    let daemon = settings.cgroup.join("nix-daemon");
    let pids = fs::read_to_string(daemon.join("cgroup.procs")).unwrap_or_default();
    if daemon_aside(&pids, |pid| {
        fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default()
    }) {
        hand_controllers(&settings.cgroup, warned);
        spare_the_daemon(&daemon, warned);
    }
    sort_cgroup(&settings.cgroup, settings, warned);
    let builds = build_cgroups(&settings.cgroup);
    tally(tallies, &builds);
    for build in builds {
        if let Some(reason) = sort_build(&build, settings, warned) {
            weigh(&build, settings.weight, reason, warned);
        } else if let Some(quest) = weight_of_quest(&build) {
            weigh(
                &build,
                quest_weight(quest, settings.weight),
                "quest",
                warned,
            );
        }
    }
}

/// Brings each process named a check with deadlines in one cgroup ahead, as
/// in the daemon's own, where builds from before it moved aside still run.
fn sort_cgroup(cgroup: &Path, settings: &Settings, warned: &mut bool) {
    let Ok(procs) = fs::read_to_string(cgroup.join("cgroup.procs")) else {
        return;
    };
    for pid in procs.lines() {
        let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) else {
            continue;
        };
        if is_timed(&comm, &settings.names) {
            bring_process_ahead(pid, &comm, settings, warned);
        }
    }
}

/// Brings every process of a build ahead, its containers' too, once one of
/// them marks a check with deadlines: a container's services and a nested
/// compositor keep time as a VM's qemu does. Says what it's weighed for, a
/// VM or a test, or nothing for a build without one.
fn sort_build(build: &Path, settings: &Settings, warned: &mut bool) -> Option<&'static str> {
    let pids = pids_under(build);
    let comms: Vec<(String, String)> = pids
        .into_iter()
        .filter_map(|pid| {
            let comm = fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            Some((pid, comm))
        })
        .collect();
    let timed: Vec<&str> = comms
        .iter()
        .filter(|(_, comm)| is_timed(comm, &settings.names))
        .map(|(_, comm)| comm.trim_start_matches('.'))
        .collect();
    if timed.is_empty() {
        return None;
    }
    let reason = if timed.iter().any(|comm| comm.starts_with("qemu")) {
        "VM"
    } else {
        "test"
    };
    for (pid, comm) in &comms {
        bring_process_ahead(pid, comm, settings, warned);
    }
    Some(reason)
}

/// Brings every thread of one process ahead, saying so when any moved.
fn bring_process_ahead(pid: &str, comm: &str, settings: &Settings, warned: &mut bool) {
    let moved = sort_vm(pid, settings, warned);
    if moved > 0 {
        eprintln!(
            "Brought {} ({pid}) ahead of the builds: {moved} threads to nice {}.",
            comm.trim_end(),
            settings.nice
        );
    }
}

fn settings(args: &[String]) -> Result<Settings, Failure> {
    let mut settings = Settings {
        cgroup: PathBuf::from("/sys/fs/cgroup/build.slice/nix-daemon.service"),
        names: Vec::new(),
        nice: -10,
        weight: 1000,
        every: Duration::from_millis(1000),
    };
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("vm-first --help");
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        let mut value = || {
            args.next()
                .ok_or_else(|| wrongly(format!("{flag} needs a value")))
        };
        match flag.as_str() {
            "--cgroup" => settings.cgroup = PathBuf::from(value()?),
            "--name" => settings.names.push(value()?.clone()),
            "--nice" => {
                settings.nice = value()?
                    .parse()
                    .ok()
                    .filter(|nice| (-20..=19).contains(nice))
                    .ok_or_else(|| wrongly("--nice is a number from -20 to 19".into()))?;
            }
            "--weight" => {
                settings.weight = value()?
                    .parse()
                    .ok()
                    .filter(|weight| (1..=10_000).contains(weight))
                    .ok_or_else(|| wrongly("--weight is a number from 1 to 10000".into()))?;
            }
            "--every" => {
                let ms = value()?
                    .parse()
                    .map_err(|_| wrongly("--every is a number of ms".into()))?;
                settings.every = Duration::from_millis(ms);
            }
            _ => {
                return Err(terminal_kit::flags::unknown(
                    "flag",
                    flag,
                    &["--cgroup", "--name", "--nice", "--weight", "--every"],
                ));
            }
        }
    }
    if settings.names.is_empty() {
        settings.names = TIMED.iter().map(|&name| name.into()).collect();
    }
    Ok(settings)
}

fn help() -> Help {
    Help::new("vm-first", "puts Nix's timed checks ahead of its builds")
        .example(
            "vm-first",
            "watches the daemon's builds, as its service runs",
        )
        .example(
            "vm-first --name 'niri-*'",
            "brings only niri's test programs ahead",
        )
        .option("--cgroup PATH", "the daemon's cgroup, its builds under it")
        .option(
            "--name NAME",
            "a process to bring ahead, in the timed ones' place; * ends a prefix",
        )
        .option("--nice N", "the nice it brings them to; -10")
        .option("--weight N", "their build's CPU and disk weight; 1000")
        .option("--every MS", "how often it looks; 1000")
}

fn run(context: &Context) -> Result<(), Failure> {
    let settings = settings(&context.args)?;
    eprintln!(
        "Bringing {} in {} ahead at nice {}.",
        settings.names.join(", "),
        settings.cgroup.display(),
        settings.nice
    );
    let mut warned = false;
    let mut tallies = HashMap::new();
    loop {
        if Path::new(&settings.cgroup).exists() {
            look(&settings, &mut tallies, &mut warned);
        }
        thread::sleep(settings.every);
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
