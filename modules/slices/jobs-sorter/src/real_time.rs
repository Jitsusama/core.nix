//! Takes real time from work. A thread on `SCHED_FIFO` or `SCHED_RR` runs
//! ahead of every ordinary thread whatever its cgroup's weight, since the
//! kernel has no real-time group scheduling. The account may take real
//! time, for call audio, and so may what an agent starts: a headless
//! compositor asks for it as it starts. So each real-time thread under a
//! `--no-real-time` cgroup goes back to the ordinary policy, and its
//! process's real-time limit drops to none, hard limit too, so neither it
//! nor anything it starts afterwards can take real time again.

use std::ffi::{c_int, c_ulong};
use std::fs;
use std::io;
use std::path::Path;

/// Looked for on every fifth pass. Reading each thread's policy took 7 ms
/// of CPU for 523 threads, 0.7% of a core at a pass a second; a fifth of
/// that leaves a thread real time for at most five passes, and its own
/// limit stops it running more than 200 ms at a time meanwhile.
pub const EVERY: u32 = 5;

const SCHED_FIFO: u64 = 1;
const SCHED_RR: u64 = 2;
const SCHED_OTHER: c_int = 0;
/// Kept when a thread holds it, since a process without privilege may
/// not clear it.
const SCHED_RESET_ON_FORK: c_int = 0x4000_0000;
const RLIMIT_RTPRIO: c_int = 14;
const ESRCH: i32 = 3;
const EPERM: i32 = 1;

#[repr(C)]
struct Limit {
    soft: c_ulong,
    hard: c_ulong,
}

#[repr(C)]
struct Param {
    priority: c_int,
}

unsafe extern "C" {
    fn prlimit(pid: c_int, resource: c_int, new: *const Limit, old: *mut Limit) -> c_int;
    fn sched_setscheduler(pid: c_int, policy: c_int, param: *const Param) -> c_int;
}

/// A thread's scheduling policy, field 41 of its stat line, found from
/// the last ')' as the name may hold anything.
pub fn policy_of(line: &str) -> Option<u64> {
    let close = line.rfind(')')?;
    // The first field after the name is the state, field 3.
    line[close + 1..]
        .split_whitespace()
        .nth(41 - 3)?
        .parse()
        .ok()
}

/// Whether a policy runs a thread ahead of every ordinary one.
pub fn is_real_time(policy: u64) -> bool {
    policy == SCHED_FIFO || policy == SCHED_RR
}

/// Every process in a cgroup and in the cgroups below it.
pub fn processes_under(dir: &Path) -> Vec<u32> {
    let mut found: Vec<u32> = fs::read_to_string(dir.join("cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .filter_map(|pid| pid.parse().ok())
        .collect();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                found.extend(processes_under(&entry.path()));
            }
        }
    }
    found
}

/// A process's threads that run in real time.
fn real_time_threads(pid: u32) -> Vec<u32> {
    let Ok(entries) = fs::read_dir(format!("/proc/{pid}/task")) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter(|tid| {
            fs::read_to_string(format!("/proc/{pid}/task/{tid}/stat"))
                .ok()
                .as_deref()
                .and_then(policy_of)
                .is_some_and(is_real_time)
        })
        .collect()
}

/// Lowers a process's real-time limit to none, hard limit too.
fn forbid(pid: c_int) -> io::Result<()> {
    let none = Limit { soft: 0, hard: 0 };
    // SAFETY: `none` outlives the call, and no old limit is asked for.
    let done = unsafe { prlimit(pid, RLIMIT_RTPRIO, &raw const none, std::ptr::null_mut()) };
    if done == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Puts one thread back on the ordinary policy, keeping its nice value and
/// any reset-on-fork it holds.
fn demote(tid: c_int) -> io::Result<()> {
    let param = Param { priority: 0 };
    for policy in [SCHED_OTHER, SCHED_OTHER | SCHED_RESET_ON_FORK] {
        // SAFETY: `param` outlives the call.
        if unsafe { sched_setscheduler(tid, policy, &raw const param) } == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(EPERM) {
            return Err(error);
        }
    }
    Err(io::Error::from_raw_os_error(EPERM))
}

/// Takes real time from every thread under `dir`, saying what it took. A
/// process that left meanwhile is no loss; anything else is said once.
pub fn take(dir: &Path, warned: &mut bool) {
    for pid in processes_under(dir) {
        let threads = real_time_threads(pid);
        let Ok(id) = c_int::try_from(pid) else {
            continue;
        };
        if threads.is_empty() {
            continue;
        }
        // The limit first, so the process can't take real time back.
        let mut result = forbid(id);
        let mut taken = 0;
        for tid in &threads {
            let Ok(tid) = c_int::try_from(*tid) else {
                continue;
            };
            match demote(tid) {
                Ok(()) => taken += 1,
                Err(error) if result.is_ok() => result = Err(error),
                Err(_) => {}
            }
        }
        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        match result {
            Ok(()) => eprintln!(
                "Took real time from {} ({pid}), work under {}: {taken} of {} threads.",
                comm.trim(),
                dir.display(),
                threads.len()
            ),
            Err(error) if error.raw_os_error() == Some(ESRCH) => {}
            Err(error) if !*warned => {
                eprintln!(
                    "Couldn't take real time from {} ({pid}): {error}",
                    comm.trim()
                );
                *warned = true;
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thread's stat line as Linux 7.2 writes it, with `policy` in field
    /// 41 and a name that holds parentheses and spaces.
    fn stat(policy: u64) -> String {
        let mut fields: Vec<String> = (4..=52).map(|n| n.to_string()).collect();
        fields[41 - 4] = policy.to_string();
        format!("4321 (sway (x) 2) S {}", fields.join(" "))
    }

    #[test]
    fn the_policy_is_field_41_past_a_name_with_parentheses() {
        assert_eq!(policy_of(&stat(2)), Some(2));
        assert_eq!(policy_of(&stat(0)), Some(0));
    }

    #[test]
    fn a_line_cut_short_has_no_policy() {
        assert_eq!(policy_of("4321 (sway) S 1 2 3"), None);
    }

    #[test]
    fn fifo_and_round_robin_are_real_time_and_nothing_else_is() {
        let real: Vec<u64> = (0..8).filter(|policy| is_real_time(*policy)).collect();
        assert_eq!(real, [1, 2]);
    }

    #[test]
    fn its_own_policy_reads_back_from_proc() {
        let line = fs::read_to_string("/proc/thread-self/stat").unwrap();
        assert_eq!(policy_of(&line).map(is_real_time), Some(false));
    }
}
