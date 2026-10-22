//! Puts the virtual machines Nix's checks start ahead of the builds they
//! share the daemon with.
//!
//! Every build runs in the daemon's one cgroup, as SCHED_BATCH, so a VM
//! test's qemu shares each CPU evenly with whatever compilers run beside
//! it, and a batch thread never preempts another as it wakes. Its guest's
//! timers then fire late, and a test's timeouts fail on a busy machine.
//! Inside one cgroup the CPU is split by nice, so once a second this
//! finds every process in `--cgroup` whose name starts with a `--name`,
//! and puts each of its threads back to SCHED_OTHER at `--nice`. Threads
//! a VM starts afterwards take that from the thread that starts them.
//! Programs outside the cgroup, everything Joel uses, keep their weight.
//!
//! Usage: vm-first [--cgroup /sys/fs/cgroup/build.slice/nix-daemon.service]
//!        [--name qemu-system]... [--nice -10] [--every 1000]

use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::Duration;

const SCHED_OTHER: i32 = 0;
const PRIO_PROCESS: i32 = 0;
/// errno for a thread that has gone.
const ESRCH: i32 = 3;

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

/// Whether a thread still needs moving ahead.
fn behind(now: &Scheduling, settings: &Settings) -> bool {
    now.policy != SCHED_OTHER || now.nice != settings.nice
}

/// Puts one thread at SCHED_OTHER and the nice asked for. Changing policy
/// keeps the nice, so the nice is set after.
fn bring_ahead(tid: u32, nice: i32) -> Result<(), i32> {
    let param = SchedParam { sched_priority: 0 };
    // SAFETY: both calls take plain integers and a pointer to a live
    // struct of the layout the kernel reads.
    let failed = unsafe {
        sched_setscheduler(tid as i32, SCHED_OTHER, &param) != 0
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
    for task in tasks.filter_map(|task| task.ok()) {
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
    let Ok(procs) = fs::read_to_string(settings.cgroup.join("cgroup.procs")) else {
        return;
    };
    for pid in procs.lines() {
        let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) else {
            continue;
        };
        if !is_vm(&comm, &settings.names) {
            continue;
        }
        let moved = sort_vm(pid, settings, warned);
        if moved > 0 {
            eprintln!(
                "Brought {} ({pid}) ahead of the builds: {moved} threads to nice {}.",
                comm.trim_end(),
                settings.nice
            );
        }
    }
}

fn settings() -> Result<Settings, String> {
    let mut settings = Settings {
        cgroup: PathBuf::from("/sys/fs/cgroup/build.slice/nix-daemon.service"),
        names: Vec::new(),
        nice: -10,
        every: Duration::from_millis(1000),
    };
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--cgroup" => settings.cgroup = PathBuf::from(value()?),
            "--name" => settings.names.push(value()?),
            "--nice" => settings.nice = value()?.parse().map_err(|_| "--nice is a number")?,
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
