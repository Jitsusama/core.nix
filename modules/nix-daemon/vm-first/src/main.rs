//! Puts the virtual machines Nix's checks start ahead of the builds they
//! share the daemon with.
//!
//! Every build runs as `SCHED_BATCH`, so a VM test's qemu shares each CPU
//! with whatever compilers run beside it, and a batch thread never
//! preempts another as it wakes. Its guest's timers then fire late, and a
//! test's timeouts fail on a busy machine. So once a second this finds
//! every process whose name starts with a `--name` in `--cgroup`, the
//! daemon's, and in each build's cgroup Nix makes under it, and puts each
//! of its threads back to `SCHED_OTHER` at `--nice`. Threads a VM starts
//! afterwards take that from the thread that starts them.
//!
//! Inside one cgroup the CPU is split by nice, but between cgroups by
//! weight, so a build in a cgroup of its own holding a VM is weighed at
//! `--weight` for CPU and disk, against the 100 of every other build.
//! Nix 3.21 makes the builds' cgroups without handing them the
//! controllers to weigh them by, so this hands them on too. Everything
//! outside the daemon's cgroup, what Joel uses, keeps its lead.
//!
//! Usage: vm-first [--cgroup /sys/fs/cgroup/build.slice/nix-daemon.service]
//!        [--name qemu-system]... [--nice -10] [--weight 1000] [--every 1000]

use std::fs;
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

#[repr(C)]
struct SchedParam {
    sched_priority: i32,
}

unsafe extern "C" {
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

/// Whether a process's name marks it as a VM. A name is at most 15
/// characters, and Nix's wrapped programs start with a dot.
fn is_vm(comm: &str, names: &[String]) -> bool {
    let comm = comm.trim_end();
    names.iter().any(|name| {
        comm.starts_with(name.as_str())
            || comm
                .strip_prefix('.')
                .is_some_and(|bare| bare.starts_with(name.as_str()))
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

/// Hands the daemon's builds the controllers, once Nix has moved the daemon
/// into a cgroup of its own beside them, which it does with `use-cgroups`;
/// until then the daemon's cgroup holds processes, and the kernel takes no
/// controllers for a cgroup that does. Nix 2.35 does this itself, and finds
/// it done.
fn hand_controllers(root: &Path, warned: &mut bool) {
    if !root.join("nix-daemon").is_dir() {
        return;
    }
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

/// Weighs a build holding a VM above the builds beside it. The weight
/// ranks it only among the daemon's builds, all in build.slice, so it
/// never comes before what Joel uses.
fn weigh(build: &Path, weight: u64, warned: &mut bool) {
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
            Ok(()) => eprintln!("Weighed {} at {value} for its VM.", path.display()),
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

fn look(settings: &Settings, warned: &mut bool) {
    hand_controllers(&settings.cgroup, warned);
    sort_cgroup(&settings.cgroup, settings, warned);
    for build in build_cgroups(&settings.cgroup) {
        if sort_cgroup(&build, settings, warned) {
            weigh(&build, settings.weight, warned);
        }
    }
}

/// Brings each VM in one cgroup ahead, saying whether it holds one.
fn sort_cgroup(cgroup: &Path, settings: &Settings, warned: &mut bool) -> bool {
    let Ok(procs) = fs::read_to_string(cgroup.join("cgroup.procs")) else {
        return false;
    };
    let mut found = false;
    for pid in procs.lines() {
        let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) else {
            continue;
        };
        if !is_vm(&comm, &settings.names) {
            continue;
        }
        found = true;
        let moved = sort_vm(pid, settings, warned);
        if moved > 0 {
            eprintln!(
                "Brought {} ({pid}) ahead of the builds: {moved} threads to nice {}.",
                comm.trim_end(),
                settings.nice
            );
        }
    }
    found
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
        settings.names = vec!["qemu-system".into(), "qemu-kvm".into()];
    }
    Ok(settings)
}

fn run() -> Result<(), String> {
    let settings = settings()?;
    eprintln!(
        "Bringing {}* in {} ahead at nice {}.",
        settings.names.join("*, "),
        settings.cgroup.display(),
        settings.nice
    );
    let mut warned = false;
    loop {
        if Path::new(&settings.cgroup).exists() {
            look(&settings, &mut warned);
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
            names: vec!["qemu-system".into(), "qemu-kvm".into()],
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
    fn knows_qemu_by_its_name_wrapped_or_not() {
        let names = defaults().names;
        assert!(is_vm("qemu-system-x86\n", &names));
        assert!(is_vm(".qemu-system-x8\n", &names));
        assert!(is_vm("qemu-kvm\n", &names));
        assert!(!is_vm("cc1\n", &names));
        assert!(!is_vm("nixos-test-driv\n", &names));
        assert!(!is_vm("vde_switch\n", &names));
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
