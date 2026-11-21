//! Has the threads of Joel's own apps named by `--quick-name`, under a
//! `--quick` cgroup, ask the scheduler for a short slice. The kernel's
//! EEVDF scheduler gives a thread woken behind others a deadline one slice
//! after it becomes eligible, so a shorter slice puts a woken drawing
//! thread ahead sooner. Under full load on 20 November, with his Chrome's
//! and Slack's threads asking for half a millisecond against the default
//! two, their worst 1% of waits for a processor fell, at every load, from
//! 2.6 to 0.5 ms (Chrome) and 1.1 to 0.5 ms (Slack) under 30 loaded, and
//! from 8.8 to 4.1 ms and 28.2 to 0.25 ms at 60 and over.
//!
//! A slice is inherited by the threads and processes a thread starts, so
//! once a process's threads ask, what it starts after asks too. A thread
//! the app puts on another policy loses its slice, so each is read again
//! every few passes, and asked again if it lost it. Only ordinary and
//! batch threads ask: a real-time thread has no slice, and an idle one
//! runs only when nothing else would. Each keeps its nice, which the app
//! sets to lower a hidden tab, and asks without privilege, since every
//! process here is the account's own.

use std::ffi::{c_int, c_long, c_uint};
use std::fs;
use std::io;
use std::path::Path;

use crate::real_time;

/// Looked for on every fifth pass, as real time is: a new thread asks at
/// once from the one that started it, so only a thread the app moved to
/// another policy and back waits these few seconds.
pub const EVERY: u32 = 5;

/// The slice asked for, in nanoseconds: half a millisecond, the one
/// measured. The kernel takes from 0.1 to 100 ms.
pub const SLICE_NS: u64 = 500_000;

const SCHED_OTHER: c_uint = 0;
const SCHED_BATCH: c_uint = 3;
const SCHED_FLAG_RESET_ON_FORK: u64 = 0x01;
const ESRCH: i32 = 3;

#[cfg(target_arch = "x86_64")]
const SYS_SCHED_SETATTR: c_long = 314;
#[cfg(target_arch = "x86_64")]
const SYS_SCHED_GETATTR: c_long = 315;
#[cfg(target_arch = "aarch64")]
const SYS_SCHED_SETATTR: c_long = 274;
#[cfg(target_arch = "aarch64")]
const SYS_SCHED_GETATTR: c_long = 275;

/// A thread's scheduling attributes, as the kernel's `struct sched_attr`
/// lays them out.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Attr {
    size: u32,
    policy: c_uint,
    flags: u64,
    nice: i32,
    priority: u32,
    runtime: u64,
    deadline: u64,
    period: u64,
    util_min: u32,
    util_max: u32,
}

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
}

/// What a thread holding `got` asks for instead, or none when it already
/// asks for the short slice or isn't an ordinary or batch thread. Its
/// policy, nice and reset-on-fork are kept; the rest is the kernel's.
pub fn asked(got: &Attr) -> Option<Attr> {
    if !matches!(got.policy, SCHED_OTHER | SCHED_BATCH) || got.runtime == SLICE_NS {
        return None;
    }
    Some(Attr {
        policy: got.policy,
        flags: got.flags & SCHED_FLAG_RESET_ON_FORK,
        nice: got.nice,
        runtime: SLICE_NS,
        ..Attr::default()
    })
}

/// A thread's attributes, read.
fn get(tid: c_int) -> io::Result<Attr> {
    let mut attr = Attr::default();
    let size = c_uint::try_from(size_of::<Attr>()).unwrap_or(c_uint::MAX);
    // SAFETY: `attr` is a `sched_attr` of `size` bytes that outlives the call.
    let done = unsafe { syscall(SYS_SCHED_GETATTR, tid, &raw mut attr, size, 0 as c_uint) };
    if done == 0 {
        Ok(attr)
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Sets a thread's attributes.
fn set(tid: c_int, attr: &Attr) -> io::Result<()> {
    let mut attr = *attr;
    attr.size = u32::try_from(size_of::<Attr>()).unwrap_or(u32::MAX);
    // SAFETY: `attr` is a whole `sched_attr` that outlives the call.
    let done = unsafe { syscall(SYS_SCHED_SETATTR, tid, &raw const attr, 0 as c_uint) };
    if done == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Has one thread ask for the short slice; whether it had to.
fn ask_one(tid: c_int) -> io::Result<bool> {
    let Some(wanted) = asked(&get(tid)?) else {
        return Ok(false);
    };
    set(tid, &wanted)?;
    Ok(true)
}

/// Has every thread of the processes named `names` under `dir` ask for
/// the short slice, saying how many had to. A process or thread that left
/// meanwhile is no loss; anything else is said once.
pub fn ask(dir: &Path, names: &[String], warned: &mut bool) {
    for pid in real_time::processes_under(dir) {
        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        if !names.iter().any(|name| name == comm.trim()) {
            continue;
        }
        let Ok(entries) = fs::read_dir(format!("/proc/{pid}/task")) else {
            continue;
        };
        let mut asked = 0;
        let mut failed = None;
        for tid in entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<c_int>().ok())
        {
            match ask_one(tid) {
                Ok(true) => asked += 1,
                Ok(false) => {}
                Err(error) if error.raw_os_error() == Some(ESRCH) => {}
                Err(error) => failed = Some(error),
            }
        }
        // A name is the program's own to choose, so one that holds a
        // newline can't start a journal line of its own.
        let comm = comm.trim().escape_debug();
        if asked > 0 {
            eprintln!(
                "Asked {asked} threads of {comm} ({pid}) for a {} µs slice.",
                SLICE_NS / 1000
            );
        }
        if let Some(error) = failed
            && !*warned
        {
            eprintln!("Couldn't have {comm} ({pid}) ask for a short slice: {error}");
            *warned = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    unsafe extern "C" {
        fn gettid() -> c_int;
    }

    fn ordinary(nice: i32, runtime: u64) -> Attr {
        Attr {
            policy: SCHED_OTHER,
            nice,
            runtime,
            ..Attr::default()
        }
    }

    #[test]
    fn an_ordinary_thread_asks_and_keeps_its_nice() {
        let wanted = asked(&ordinary(5, 2_000_000)).expect("an ordinary thread asks");
        assert_eq!(wanted.runtime, SLICE_NS, "it asks for the short slice");
        assert_eq!(wanted.nice, 5, "a hidden tab's nice is kept");
        assert_eq!(wanted.policy, SCHED_OTHER, "its policy is kept");
    }

    #[test]
    fn a_batch_thread_asks_and_stays_batch() {
        let got = Attr {
            policy: SCHED_BATCH,
            ..ordinary(-8, 2_000_000)
        };
        let wanted = asked(&got).expect("a batch thread asks");
        assert_eq!(
            (wanted.policy, wanted.nice),
            (SCHED_BATCH, -8),
            "batch, nice -8 kept"
        );
    }

    #[test]
    fn reset_on_fork_is_kept_and_other_flags_are_not() {
        let got = Attr {
            flags: SCHED_FLAG_RESET_ON_FORK | 0x60,
            ..ordinary(0, 2_000_000)
        };
        let wanted = asked(&got).expect("it asks");
        assert_eq!(
            wanted.flags, SCHED_FLAG_RESET_ON_FORK,
            "only reset-on-fork is kept"
        );
    }

    #[test]
    fn one_already_asking_asks_nothing() {
        assert_eq!(asked(&ordinary(0, SLICE_NS)), None, "already short");
    }

    #[test]
    fn real_time_deadline_and_idle_threads_ask_nothing() {
        for policy in [1, 2, 5, 6] {
            let got = Attr {
                policy,
                ..ordinary(0, 2_000_000)
            };
            assert_eq!(asked(&got), None, "policy {policy} asks nothing");
        }
    }

    /// A thread of its own, niced as a hidden tab is, asks for the short
    /// slice through the kernel, keeps its nice, and a thread it starts
    /// afterwards inherits the slice.
    #[test]
    fn a_thread_asks_through_the_kernel_and_what_it_starts_inherits() {
        let (tell, heard) = mpsc::channel();
        let (done, finish) = mpsc::channel::<()>();
        let worker = thread::spawn(move || {
            // SAFETY: gettid takes nothing and can't fail.
            let tid = unsafe { gettid() };
            let mut got = get(tid).expect("its own attributes read");
            got.nice = 3;
            set(tid, &got).expect("it lowers its own nice");
            assert!(ask_one(tid).expect("it asks"), "it had to ask");
            let (child_tell, child_heard) = mpsc::channel();
            let (child_done, child_finish) = mpsc::channel::<()>();
            let child = thread::spawn(move || {
                // SAFETY: as above.
                child_tell.send(unsafe { gettid() }).expect("heard");
                // Kept alive until it's read.
                child_finish.recv().ok();
            });
            let child_tid = child_heard.recv().expect("its child's id");
            let child_attr = get(child_tid);
            child_done.send(()).ok();
            child.join().expect("its child ends");
            tell.send((get(tid).expect("read again"), child_attr))
                .expect("heard");
            finish.recv().ok();
        });
        let (own, child) = heard.recv().expect("the thread answers");
        done.send(()).ok();
        worker.join().expect("the thread ends");
        assert_eq!(own.runtime, SLICE_NS, "it holds the short slice");
        assert_eq!(own.nice, 3, "it keeps its nice");
        assert_eq!(
            child.expect("its child read").runtime,
            SLICE_NS,
            "its child inherits"
        );
    }
}
