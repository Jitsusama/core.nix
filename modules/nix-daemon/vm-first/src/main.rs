//! Puts the checks with deadlines that Nix runs, VM tests first among them,
//! ahead of the builds they share the daemon with.
//!
//! Every build runs as `SCHED_BATCH`, so a VM test's qemu shares each CPU
//! with whatever compilers run beside it, and a batch thread never
//! preempts another as it wakes. Its guest's timers then fire late, and a
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
//! As each build goes, this says in its journal what the build read and
//! wrote on the disks themselves, from its cgroup's io.stat, with its
//! client's command and quest, so the builds that wear the disk most can
//! be found. A build's own name sits in its environment, which only its
//! user may read, so it's known by what asked for it.
//!
//! Usage: vm-first [--cgroup /sys/fs/cgroup/build.slice/nix-daemon.service]
//!        [--name nixos-test-driver\*]... [--nice -10] [--weight 1000] [--every 1000]

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

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

/// The client a daemon worker serves, from its command line: Nix writes
/// the client's pid over the worker's `--daemon`, padding it with NULs.
fn client_of(cmdline: &[u8]) -> Option<u32> {
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
        if let Some(client) = fs::read(format!("/proc/{pid}/cmdline"))
            .ok()
            .and_then(|cmdline| client_of(&cmdline))
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

/// Who asked for a build: the client's command, and its quest if it sits
/// in one's slice.
struct Asker {
    command: String,
    quest: Option<String>,
}

/// What one build has read and written so far. Nix names a build's cgroup
/// by its build user, and the next build of that user makes it again, so a
/// build is known by its cgroup's inode, which the kernel never reuses
/// while vm-first runs.
///
/// A disk counts a write only once writeback sends it, which waits until
/// enough is dirty or 30 s have passed, so a build often ends with some of
/// its writes still in memory. `unwritten` keeps what was dirty or being
/// written at the last look; a file the build deleted before then never
/// reaches the disk, so it's the most still to come.
struct Tally {
    cgroup: u64,
    read: u64,
    written: u64,
    unwritten: u64,
    asked: Option<Asker>,
}

/// The bytes of a cgroup's file pages dirty or being written back, from its
/// `memory.stat`.
fn parse_unwritten(stat: &str) -> u64 {
    stat.lines()
        .filter_map(|line| line.split_once(' '))
        .filter(|(key, _)| matches!(*key, "file_dirty" | "file_writeback"))
        .filter_map(|(_, value)| value.trim().parse::<u64>().ok())
        .sum()
}

/// The bytes a cgroup has read and written, from its `io.stat`, on the
/// devices `counts` takes by their `major:minor`. `io.stat` lists a write
/// on every layer it passes through, so a disk under LUKS shows each byte
/// twice, on the mapper and on the disk beneath it, and zram shows swap
/// that never leaves memory; counting only the disks themselves counts
/// each byte the hardware takes once.
fn parse_io_stat(stat: &str, counts: impl Fn(&str) -> bool) -> (u64, u64) {
    let lines: Vec<&str> = stat
        .lines()
        .filter(|line| line.split_whitespace().next().is_some_and(&counts))
        .collect();
    let count = |key: &str| -> u64 {
        lines
            .iter()
            .flat_map(|line| line.split_whitespace())
            .filter_map(|field| field.strip_prefix(key))
            .filter_map(|value| value.parse::<u64>().ok())
            .sum()
    };
    (count("rbytes="), count("wbytes="))
}

/// Whether a block device, by its `major:minor`, is hardware: a disk the
/// kernel found on a bus, rather than a layer such as device-mapper's or a
/// device kept in memory such as zram, which have no device behind them.
fn is_a_disk(device: &str) -> bool {
    Path::new("/sys/dev/block")
        .join(device)
        .join("device")
        .exists()
}

/// A count of bytes as a person reads it, in binary units.
fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a tenth of a unit is all it says"
    )]
    let mut scaled = bytes as f64 / 1024.0;
    let mut unit = 0;
    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }
    format!("{scaled:.1} {}", UNITS[unit])
}

/// The longest a client's command is said at, in characters.
const COMMAND_LENGTH: usize = 120;

/// A client's command as a person would type it, from its command line:
/// the program by its name alone, and cut short past `COMMAND_LENGTH`.
fn command_of(cmdline: &[u8]) -> String {
    let mut args = cmdline
        .split(|&byte| byte == 0)
        .filter(|arg| !arg.is_empty())
        .map(String::from_utf8_lossy);
    let Some(program) = args.next() else {
        return String::new();
    };
    let name = program.rsplit('/').next().unwrap_or(&program).to_string();
    let said = std::iter::once(name)
        .chain(args.map(std::borrow::Cow::into_owned))
        .collect::<Vec<_>>()
        .join(" ");
    if said.chars().count() <= COMMAND_LENGTH {
        return said;
    }
    let mut cut: String = said.chars().take(COMMAND_LENGTH - 3).collect();
    cut.push_str("...");
    cut
}

/// The line vm-first says when a build has gone.
fn told(tally: &Tally) -> String {
    let left = if tally.unwritten == 0 {
        String::new()
    } else {
        format!(
            ", leaving up to {} more in memory to write",
            size(tally.unwritten)
        )
    };
    let did = format!(
        "wrote {} and read {}{left}.",
        size(tally.written),
        size(tally.read)
    );
    match &tally.asked {
        Some(asker) => {
            let quest = asker
                .quest
                .as_ref()
                .map_or("no quest".to_string(), |quest| format!("quest {quest}"));
            format!("A build for `{}`, for {quest}, {did}", asker.command)
        }
        None => format!("A build whose client had gone {did}"),
    }
}

/// The quest a quest's slice is for, from the slice's path.
fn quest_of(slice: &str) -> Option<&str> {
    slice
        .rsplit('/')
        .next()?
        .strip_prefix(QUEST_SLICE)?
        .strip_suffix(".slice")
        .filter(|quest| !quest.is_empty())
}

/// Who asked for a build, while its client is still there to ask.
fn asker_of(build: &Path) -> Option<Asker> {
    let client = client_of_build(build)?;
    let command = command_of(&fs::read(format!("/proc/{client}/cmdline")).ok()?);
    let cgroup = fs::read_to_string(format!("/proc/{client}/cgroup")).ok()?;
    let quest = quest_slice(&cgroup).and_then(quest_of).map(str::to_string);
    Some(Asker { command, quest })
}

/// Brings each build's tally up to date, and says what each build that has
/// gone since the last look read and wrote.
fn tally(tallies: &mut HashMap<PathBuf, Tally>, builds: &[PathBuf]) {
    let mut seen = Vec::with_capacity(builds.len());
    for build in builds {
        let Ok(cgroup) = fs::metadata(build).map(|meta| meta.ino()) else {
            continue;
        };
        if tallies
            .get(build)
            .is_some_and(|known| known.cgroup != cgroup)
            && let Some(gone) = tallies.remove(build)
        {
            eprintln!("{}", told(&gone));
        }
        let known = tallies.entry(build.clone()).or_insert_with(|| Tally {
            cgroup,
            read: 0,
            written: 0,
            unwritten: 0,
            asked: None,
        });
        if known.asked.is_none() {
            known.asked = asker_of(build);
        }
        if let Ok(stat) = fs::read_to_string(build.join("io.stat")) {
            (known.read, known.written) = parse_io_stat(&stat, is_a_disk);
        }
        if let Ok(stat) = fs::read_to_string(build.join("memory.stat")) {
            known.unwritten = parse_unwritten(&stat);
        }
        seen.push(build);
    }
    tallies.retain(|build, gone| {
        let here = seen.contains(&build);
        if !here {
            eprintln!("{}", told(gone));
        }
        here
    });
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

fn settings() -> Result<Settings, String> {
    let mut settings = Settings {
        cgroup: PathBuf::from("/sys/fs/cgroup/build.slice/nix-daemon.service"),
        names: Vec::new(),
        nice: -10,
        weight: 1000,
        every: Duration::from_millis(1000),
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--cgroup" => settings.cgroup = PathBuf::from(value()?),
            "--name" => settings.names.push(value()?),
            "--nice" => settings.nice = value()?.parse().map_err(|_| "--nice is a number")?,
            "--weight" => {
                settings.weight = value()?
                    .parse()
                    .ok()
                    .filter(|weight| (1..=10_000).contains(weight))
                    .ok_or("--weight is a number from 1 to 10000")?;
            }
            "--every" => {
                let ms = value()?.parse().map_err(|_| "--every is ms")?;
                settings.every = Duration::from_millis(ms);
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if settings.names.is_empty() {
        settings.names = TIMED.iter().map(|&name| name.into()).collect();
    }
    Ok(settings)
}

fn run() -> Result<(), String> {
    let settings = settings()?;
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

fn main() {
    if let Err(error) = run() {
        eprintln!("vm-first: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Settings {
        Settings {
            cgroup: PathBuf::new(),
            names: TIMED.iter().map(|&name| name.into()).collect(),
            nice: -10,
            weight: 1000,
            every: Duration::from_millis(1000),
        }
    }

    /// A stat line with the given name, nice and policy, the other fields
    /// as a sleeping process has them.
    fn stat(comm: &str, nice: i32, policy: i32) -> String {
        let mut fields = vec!["0".to_string(); 52 - 2];
        fields[0] = "S".into();
        fields[19 - 3] = nice.to_string();
        fields[41 - 3] = policy.to_string();
        format!("4242 ({comm}) {}", fields.join(" "))
    }

    #[test]
    fn hands_builds_the_controllers_the_daemon_has_and_has_not_handed() {
        assert_eq!(
            to_enable("cpuset cpu io memory hugetlb pids\n", "\n").as_deref(),
            Some("+cpu +io +memory +pids")
        );
        // As a daemon whose unit delegates no controllers is given them, without cpu.
        assert_eq!(
            to_enable("io memory pids\n", "memory\n").as_deref(),
            Some("+io +pids")
        );
        assert_eq!(
            to_enable("cpu io memory pids\n", "cpu io memory pids\n"),
            None
        );
        assert_eq!(to_enable("", ""), None);
    }

    #[test]
    fn knows_the_daemon_from_its_workers() {
        assert!(is_daemon(
            b"/nix/store/abc-nix/bin/nix-daemon\x00--daemon\x00"
        ));
        assert!(is_daemon(b"nix-daemon\x00--daemon\x00"));
        // A worker, which writes its client's pid over --daemon.
        assert!(!is_daemon(b"nix-daemon\x00307605\x00"));
        assert!(!is_daemon(b"bash\x00--daemon\x00"));
        assert!(!is_daemon(b""));
    }

    #[test]
    fn the_daemon_is_aside_only_once_it_has_moved_itself() {
        let workers_left = |pid: &str| match pid {
            "696" | "702" => b"nix-daemon\x00307605\x00".to_vec(),
            _ => Vec::new(),
        };
        // A restarted daemon's workers stay behind in its cgroup, the new
        // daemon not yet among them.
        assert!(!daemon_aside("696\n702\n", workers_left));
        assert!(!daemon_aside("", workers_left));

        let moved = |pid: &str| match pid {
            "966" => b"nix-daemon\x00--daemon\x00".to_vec(),
            other => workers_left(other),
        };
        assert!(daemon_aside("696\n702\n966\n", moved));
    }

    #[test]
    fn finds_the_builds_cgroups_and_nothing_else() {
        let root = std::env::temp_dir().join(format!("vm-first-{}", process::id()));
        for dir in ["nix-daemon", "nix-build-uid-872415232", "nix-build-pid-7-0"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("nix-build-file"), "").unwrap();

        let mut found = build_cgroups(&root);
        found.sort();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            found,
            vec![
                root.join("nix-build-pid-7-0"),
                root.join("nix-build-uid-872415232")
            ]
        );
    }

    #[test]
    fn a_vms_build_is_weighed_for_cpu_and_disk() {
        assert_eq!(
            weight_files(1000),
            [
                ("cpu.weight", "1000".to_string()),
                ("io.weight", "default 1000".to_string())
            ]
        );
    }

    #[test]
    fn a_daemon_worker_names_its_client() {
        // Nix writes the client's pid over "--daemon", padding with NULs.
        assert_eq!(
            client_of(b"nix-daemon\x00307605\x00\x00\x00"),
            Some(307_605)
        );
        assert_eq!(
            client_of(b"/nix/store/x-nix/bin/nix-daemon\x0042\x00"),
            Some(42)
        );
        assert_eq!(client_of(b"nix-daemon\x00--daemon\x00"), None);
        assert_eq!(client_of(b"bash\x00123\x00"), None);
        assert_eq!(client_of(b""), None);
    }

    #[test]
    fn reads_a_parent() {
        let mut line = stat("cc1", 0, 3);
        line = line.replacen(" S 0 ", " S 307960 ", 1);
        assert_eq!(parse_ppid(&line), Some(307_960));
        assert_eq!(parse_ppid("4242 (cc1)"), None);
    }

    #[test]
    fn reads_what_a_build_wrote_and_read_on_every_disk() {
        let stat = "259:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n\
                    8:0 rbytes=5 wbytes=10 rios=1 wios=1 dbytes=0 dios=0\n";
        assert_eq!(parse_io_stat(stat, |_| true), (105, 2058));
        assert_eq!(parse_io_stat("", |_| true), (0, 0));
    }

    #[test]
    fn counts_a_write_once_however_many_layers_it_passes() {
        // As a laptop's build.slice reads: zram, then the disk, then the
        // LUKS mapper on it, holding the disk's bytes again.
        let stat = "253:0 rbytes=7 wbytes=70 rios=1 wios=1 dbytes=0 dios=0\n\
                    259:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n\
                    254:0 rbytes=100 wbytes=2048 rios=1 wios=2 dbytes=0 dios=0\n";
        assert_eq!(parse_io_stat(stat, |device| device == "259:0"), (100, 2048));
    }

    #[test]
    fn says_sizes_as_a_person_reads_them() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(1023), "1023 B");
        assert_eq!(size(1536), "1.5 KiB");
        assert_eq!(size(64 << 20), "64.0 MiB");
        assert_eq!(size(3 << 30), "3.0 GiB");
        assert_eq!(size(5 << 40), "5.0 TiB");
    }

    #[test]
    fn names_a_client_by_its_command() {
        assert_eq!(
            command_of(
                b"/run/current-system/sw/bin/nix-build\x00--store\x00daemon\x00/etc/a.nix\x00"
            ),
            "nix-build --store daemon /etc/a.nix"
        );
        let long = [b"nix\x00build\x00".as_slice(), &[b'x'; 400]].concat();
        let said = command_of(&long);
        assert!(said.starts_with("nix build xxx"), "{said}");
        assert!(said.ends_with("..."), "{said}");
        assert!(said.chars().count() <= 120, "{said}");
    }

    #[test]
    fn says_which_quest_asked_for_a_build_and_what_it_wrote() {
        let mut tally = Tally {
            cgroup: 1,
            read: 0,
            written: 1 << 30,
            unwritten: 0,
            asked: Some(Asker {
                command: "nix build .#checks".into(),
                quest: Some("qestabc".into()),
            }),
        };
        assert_eq!(
            told(&tally),
            "A build for `nix build .#checks`, for quest qestabc, wrote 1.0 GiB and read 0 B."
        );
        tally.asked = Some(Asker {
            command: "nix-build x".into(),
            quest: None,
        });
        assert_eq!(
            told(&tally),
            "A build for `nix-build x`, for no quest, wrote 1.0 GiB and read 0 B."
        );
        tally.asked = None;
        assert_eq!(
            told(&tally),
            "A build whose client had gone wrote 1.0 GiB and read 0 B."
        );
        tally.unwritten = 64 << 20;
        assert_eq!(
            told(&tally),
            "A build whose client had gone wrote 1.0 GiB and read 0 B, \
             leaving up to 64.0 MiB more in memory to write."
        );
    }

    #[test]
    fn counts_what_a_build_left_to_write() {
        let stat = "anon 4096\nfile 9000\nfile_dirty 3000\nfile_writeback 500\n\
                    file_mapped 0\n";
        assert_eq!(parse_unwritten(stat), 3500);
        assert_eq!(parse_unwritten(""), 0);
    }

    #[test]
    fn names_the_quest_of_its_slice() {
        assert_eq!(
            quest_of("/user.slice/jobs.slice/jobs-work.slice/jobs-work-qestabc.slice"),
            Some("qestabc")
        );
        assert_eq!(quest_of("/user.slice/jobs.slice/jobs-work.slice"), None);
    }

    #[test]
    fn finds_the_quest_a_client_works_for() {
        let quest = "/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/jobs-work-qest20261024abc123.slice";
        assert_eq!(
            quest_slice(&format!("0::{quest}/work-307590-4656305.scope\n")),
            Some(quest)
        );
        assert_eq!(
            quest_slice(
                "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs-work.slice/work-1-2.scope\n"
            ),
            None
        );
        assert_eq!(
            quest_slice("0::/system.slice/compiler-build.service\n"),
            None
        );
    }

    #[test]
    fn a_quests_build_stays_below_a_vms() {
        assert_eq!(quest_weight(400, 1000), 400);
        assert_eq!(quest_weight(10_000, 1000), 999);
        assert_eq!(quest_weight(0, 1000), 1);
    }

    #[test]
    fn reads_nice_and_policy() {
        let line = stat("qemu-system-x86", 0, 3);
        assert_eq!(parse_stat(&line), Some(Scheduling { nice: 0, policy: 3 }));
    }

    #[test]
    fn reads_past_a_name_holding_parentheses() {
        let line = stat("a) 1 2 (b", -10, 0);
        assert_eq!(
            parse_stat(&line),
            Some(Scheduling {
                nice: -10,
                policy: 0
            })
        );
    }

    #[test]
    fn a_short_line_reads_as_nothing() {
        assert_eq!(parse_stat("4242 (qemu) S 1 2"), None);
    }

    #[test]
    fn knows_a_timed_test_by_what_runs_in_it_wrapped_or_not() {
        let names = defaults().names;
        for comm in [
            "qemu-system-x86\n",
            ".qemu-system-x8\n",
            "qemu-kvm\n",
            "nixos-test-driv\n",
            ".nixos-test-dri\n",
            "systemd-nspawn\n",
            "niri\n",
            ".niri-wrapped\n",
            "sway\n",
            ".sway-wrapped\n",
        ] {
            assert!(is_timed(comm, &names), "{comm}");
        }
        // A niri build's own test programs, which cargo names after the
        // crate and a hash, are a compiler's work, not a compositor.
        for comm in [
            "cc1\n",
            "vde_switch\n",
            "rustc\n",
            "python3.14\n",
            "bash\n",
            "niri-3f2a1b9c0d\n",
            "niri_config-ab1\n",
            "niri_ipc-9e8d7c\n",
            "swaybg\n",
        ] {
            assert!(!is_timed(comm, &names), "{comm}");
        }
    }

    #[test]
    fn finds_every_process_under_a_build_its_containers_too() {
        let root = std::env::temp_dir().join(format!("vm-first-under-{}", process::id()));
        let payload = root.join("payload");
        fs::create_dir_all(payload.join("init.scope")).unwrap();
        fs::write(root.join("cgroup.procs"), "10\n11\n").unwrap();
        fs::write(payload.join("cgroup.procs"), "").unwrap();
        fs::write(payload.join("init.scope/cgroup.procs"), "30\n").unwrap();

        let mut found = pids_under(&root);
        found.sort();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(found, vec!["10", "11", "30"]);
    }

    #[test]
    fn a_batch_thread_or_a_nice_one_is_behind() {
        let settings = defaults();
        assert!(behind(&Scheduling { nice: 0, policy: 3 }, &settings));
        assert!(behind(&Scheduling { nice: 0, policy: 0 }, &settings));
        assert!(behind(
            &Scheduling {
                nice: -10,
                policy: 3
            },
            &settings
        ));
        assert!(!behind(
            &Scheduling {
                nice: -10,
                policy: 0
            },
            &settings
        ));
    }
}
