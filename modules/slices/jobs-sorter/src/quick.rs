//! Has the threads of Joel's own programs named by `--quick-name`, under a
//! `--quick` cgroup, ask the scheduler for a short slice. The kernel's
//! EEVDF scheduler gives a thread woken behind others a deadline one slice
//! after it becomes eligible, so a shorter slice puts a woken drawing
//! thread ahead sooner. On 20 November, with every thread of his Chrome
//! and Slack asking for half a millisecond against the default two, in an
//! hour of alternating quarters, their worst 1% of waits for a processor
//! fell from 1.6 to 0.45 ms (Chrome) and 0.74 to 0.26 ms (Slack) under a
//! load of 30, and from 4.6 to 1.1 ms and 17.7 to 0.59 ms at 60 and over;
//! between, Slack's was about even, 0.62 against 0.52 ms.
//!
//! A slice is inherited by the threads and processes a thread starts, so
//! once a process's threads ask, what it starts after asks too. A thread
//! the program puts on another policy loses its slice, so each is read
//! again every few passes, and asked again if it lost it. Only ordinary
//! and batch threads ask: a real-time thread has no slice, and an idle one
//! runs only when nothing else would. Each keeps its nice, which Chrome
//! sets to lower a hidden tab, and asks without privilege, since every
//! process here is the account's own.
//!
//! Only Joel's own ask. An agent's Chrome moves itself into his programs'
//! slice as his does, and the sorter sends back only one started from
//! jobs, so a program is his only when what started its first process
//! there is his user manager, his session, or another of his programs,
//! never a terminal or a pi session, where agents work. A program of the
//! name that isn't his is given the kernel's default back, so a slice it
//! inherited goes. A terminal's own programs don't ask either: the sorter
//! moves a terminal's busy work to jobs, where a short slice would run
//! beside his. A program whose starter has ended, or that is a service's
//! own process, is the user manager's child, as his are, so it counts as
//! his, an agent's too: nothing about its parent tells them apart, and
//! agents' browser tools keep their launcher. This keeps a short slice
//! from reaching agents' work by accident; any program of the account's
//! could ask for one itself. What his programs start, a link's handler or
//! a terminal opened from one, inherits the slice, as his own work. Taking
//! the flags away leaves the slices asked until each program restarts.

use std::ffi::{c_int, c_long, c_uint};
use std::fs;
use std::io;
use std::path::Path;

use crate::real_time;

/// Looked for on every fifth pass, as real time is: a new thread asks at
/// once from the one that started it, so only a thread the program moved
/// to another policy and back waits these few seconds.
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

/// How far up a program's own processes its starter is looked for.
const DEEPEST: usize = 64;

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

// The kernel's second version of the struct is 56 bytes; one laid out
// otherwise would be read as other fields, or refused.
const _: () = assert!(size_of::<Attr>() == 56, "sched_attr is 56 bytes");

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
}

/// What a thread holding `got` is set to so it asks for `slice`, zero
/// being the kernel's default, or none when it already does or isn't an
/// ordinary or batch thread. Its policy, nice and reset-on-fork are kept;
/// the rest is the kernel's.
fn wanted(got: &Attr, slice: u64) -> Option<Attr> {
    let default = slice == 0 && got.runtime != SLICE_NS;
    if !matches!(got.policy, SCHED_OTHER | SCHED_BATCH) || got.runtime == slice || default {
        return None;
    }
    Some(Attr {
        policy: got.policy,
        flags: got.flags & SCHED_FLAG_RESET_ON_FORK,
        nice: got.nice,
        runtime: slice,
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

/// What asking one thread came to.
#[derive(Debug, PartialEq)]
enum Asked {
    /// It already asked for that slice, or can't ask.
    No,
    /// It asks now.
    Yes,
    /// The kernel took the call but holds no slice for it, as one older
    /// than fair slices does, reading every ordinary thread's as 0.
    Unheard,
}

/// What a thread set to `attr` read back as `again` says: a kernel that
/// holds slices reads one back, the default or another, so only 0 with
/// nothing else changed is one that holds none. A thread that changed its
/// own policy or nice between is asked again later.
fn heard(attr: &Attr, again: &Attr) -> Asked {
    let untouched = again.policy == attr.policy && again.nice == attr.nice;
    if attr.runtime == 0 || again.runtime == attr.runtime || !untouched {
        Asked::Yes
    } else if again.runtime == 0 {
        Asked::Unheard
    } else {
        Asked::Yes
    }
}

/// Has one thread ask for `slice`.
fn ask_one(tid: c_int, slice: u64) -> io::Result<Asked> {
    let Some(attr) = wanted(&get(tid)?, slice) else {
        return Ok(Asked::No);
    };
    set(tid, &attr)?;
    Ok(heard(&attr, &get(tid)?))
}

/// Whether a process's name, as the kernel gives it, is one of `names`.
fn named(comm: &str, names: &[String]) -> bool {
    names.iter().any(|name| name == comm.trim_end_matches('\n'))
}

/// Whether a scope or service, the last part of a cgroup, is one of the
/// windows' own, a terminal's or a pi session's, by its `windows` prefix.
fn a_window(cgroup: &str, windows: &[String]) -> bool {
    let unit = cgroup.rsplit('/').next().unwrap_or(cgroup);
    windows
        .iter()
        .any(|prefix| unit.starts_with(prefix.as_str()))
}

/// Whether a program is Joel's to ask, from its own cgroup and the one of
/// what started its first process outside its own, both as
/// /proc/<pid>/cgroup gives them below the user manager's.
fn his(quick: &str, own: &str, starter: &str, windows: &[String]) -> bool {
    let programs = format!("/{}/", quick.trim_matches('/'));
    let from_his = starter == "/init.scope"
        || starter == "/session.slice"
        || starter.starts_with("/session.slice/")
        || starter.starts_with(&programs);
    from_his && !a_window(own, windows) && !a_window(starter, windows)
}

/// A process as the starter's walk reads it: its parent, its name and its
/// cgroup below the user manager's.
struct Seen {
    parent: u32,
    comm: String,
    cgroup: Option<String>,
}

/// A process as /proc has it.
fn seen(pid: u32, manager: &str) -> Option<Seen> {
    let stat = crate::parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)?;
    Some(Seen {
        parent: stat.ppid,
        comm: fs::read_to_string(format!("/proc/{pid}/comm")).ok()?,
        cgroup: crate::cgroup_of(pid, manager),
    })
}

/// The cgroup of what started a program's first process: up through its
/// parents of the same name, to the first of another, each read by
/// `read`. None when one can't be read, the walk reaches no other, or
/// what started it is outside the user manager.
fn starter(pid: u32, names: &[String], read: impl Fn(u32) -> Option<Seen>) -> Option<String> {
    let mut at = pid;
    for _ in 0..DEEPEST {
        let parent = read(at)?.parent;
        if parent == 0 {
            return None;
        }
        let above = read(parent)?;
        if !named(&above.comm, names) {
            return above.cgroup;
        }
        at = parent;
    }
    None
}

/// Where and whom to ask.
pub struct Asking<'a> {
    /// The cgroup asked in, below the user manager's, as the flag names it.
    pub quick: &'a str,
    /// Its directory.
    pub dir: &'a Path,
    /// The user manager's cgroup.
    pub manager: &'a str,
    /// The programs that ask.
    pub names: &'a [String],
    /// The windows' scope prefixes, whose programs never ask.
    pub windows: &'a [String],
}

/// Has every thread of Joel's programs named in `asking` ask for the short
/// slice, and gives any other of the name the default back, saying how
/// many had to. Whether the kernel held every slice asked. A process or
/// thread that left meanwhile is no loss; anything else is said once.
pub fn ask(asking: &Asking, warned: &mut bool) -> bool {
    let mut heard = true;
    for pid in real_time::processes_under(asking.dir) {
        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        if !named(&comm, asking.names) {
            continue;
        }
        // One whose parents can't be read is left as it is this pass.
        let (Some(own), Some(from)) = (
            crate::cgroup_of(pid, asking.manager),
            starter(pid, asking.names, |pid| seen(pid, asking.manager)),
        ) else {
            continue;
        };
        let slice = if his(asking.quick, &own, &from, asking.windows) {
            SLICE_NS
        } else {
            0
        };
        let Ok(entries) = fs::read_dir(format!("/proc/{pid}/task")) else {
            continue;
        };
        let mut asked = 0;
        let mut failed = None;
        for tid in entries
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<c_int>().ok())
        {
            match ask_one(tid, slice) {
                Ok(Asked::Yes) => asked += 1,
                Ok(Asked::No) => {}
                Ok(Asked::Unheard) => heard = false,
                Err(error) if error.raw_os_error() == Some(ESRCH) => {}
                Err(error) => failed = Some(error),
            }
        }
        // A name is the program's own to choose, so one that holds a
        // newline can't start a journal line of its own.
        let comm = comm.trim().escape_debug();
        if asked > 0 && slice == 0 {
            eprintln!("Gave {asked} threads of {comm} ({pid}), not Joel's, the default slice.");
        } else if asked > 0 {
            eprintln!(
                "Asked {asked} threads of {comm} ({pid}) for a {} µs slice.",
                SLICE_NS / 1000
            );
        }
        if let Some(error) = failed
            && !*warned
        {
            eprintln!("Couldn't set the slice of {comm} ({pid})'s threads: {error}");
            *warned = true;
        }
        if !heard {
            break;
        }
    }
    heard
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

    fn windows() -> Vec<String> {
        vec!["kitty-".to_owned(), "mycelium-pi-".to_owned()]
    }

    #[test]
    fn an_ordinary_thread_asks_and_keeps_its_nice() {
        let got = wanted(&ordinary(5, 2_000_000), SLICE_NS).expect("an ordinary thread asks");
        assert_eq!(got.runtime, SLICE_NS, "it asks for the short slice");
        assert_eq!(got.nice, 5, "a hidden tab's nice is kept");
        assert_eq!(got.policy, SCHED_OTHER, "its policy is kept");
    }

    #[test]
    fn a_batch_thread_asks_and_stays_batch() {
        let got = Attr {
            policy: SCHED_BATCH,
            ..ordinary(-8, 2_000_000)
        };
        let got = wanted(&got, SLICE_NS).expect("a batch thread asks");
        assert_eq!(
            (got.policy, got.nice),
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
        let got = wanted(&got, SLICE_NS).expect("it asks");
        assert_eq!(
            got.flags, SCHED_FLAG_RESET_ON_FORK,
            "only reset-on-fork is kept"
        );
    }

    #[test]
    fn one_already_asking_asks_nothing() {
        assert_eq!(
            wanted(&ordinary(0, SLICE_NS), SLICE_NS),
            None,
            "already short"
        );
    }

    #[test]
    fn real_time_deadline_and_idle_threads_ask_nothing() {
        for policy in [1, 2, 5, 6] {
            let got = Attr {
                policy,
                ..ordinary(0, 2_000_000)
            };
            assert_eq!(wanted(&got, SLICE_NS), None, "policy {policy} asks nothing");
        }
    }

    #[test]
    fn only_the_short_slice_is_given_back() {
        let back = wanted(&ordinary(4, SLICE_NS), 0).expect("the short slice goes");
        assert_eq!(
            (back.runtime, back.nice),
            (0, 4),
            "the default, its nice kept"
        );
        assert_eq!(
            wanted(&ordinary(0, 3_000_000), 0),
            None,
            "a slice the program chose is its own"
        );
    }

    #[test]
    fn a_name_is_matched_whole_without_its_newline() {
        let names = ["chrome".to_owned(), "slack".to_owned()];
        assert!(
            named("chrome\n", &names),
            "the kernel's newline is no part of it"
        );
        assert!(
            !named("chromedriver\n", &names),
            "a longer name is another program"
        );
        assert!(!named("Slack\n", &names), "names are matched as given");
    }

    #[test]
    fn a_program_started_by_his_manager_session_or_programs_is_his() {
        let own = "/app.slice/app-com.google.Chrome-20682.scope";
        for starter in [
            "/init.scope",
            "/session.slice/niri.service",
            "/app.slice/app-launcher.scope",
        ] {
            assert!(his("app.slice", own, starter, &windows()), "from {starter}");
        }
    }

    #[test]
    fn a_program_an_agent_or_a_window_started_is_not_his() {
        let own = "/app.slice/app-com.google.Chrome-7.scope";
        for starter in [
            "/agents.slice/pi-host-work-attended.service",
            "/jobs.slice/jobs-work.slice/work-3.scope",
            "/app.slice/kitty-12-0.scope",
            "/app.slice/mycelium-pi-4.scope",
            "/app.slicex/app-a.scope",
        ] {
            assert!(
                !his("app.slice", own, starter, &windows()),
                "from {starter}"
            );
        }
    }

    #[test]
    fn a_terminals_own_program_is_not_his_to_ask() {
        let own = "/app.slice/kitty-12-0.scope";
        assert!(
            !his("app.slice", own, "/init.scope", &windows()),
            "a terminal's"
        );
    }

    fn process(parent: u32, comm: &str, cgroup: &str) -> Seen {
        Seen {
            parent,
            comm: format!("{comm}\n"),
            cgroup: Some(cgroup.to_owned()),
        }
    }

    /// A renderer, its zygote and its browser, each a chrome, started by
    /// `starter` in `at`.
    fn browser(parent: u32, comm: &str, at: &str) -> impl Fn(u32) -> Option<Seen> {
        let comm = comm.to_owned();
        let at = at.to_owned();
        move |pid| match pid {
            30 => Some(process(20, "chrome", "/app.slice/app-niri-a-10.scope")),
            20 => Some(process(
                10,
                "chrome",
                "/app.slice/app-com.google.Chrome-10.scope",
            )),
            10 => Some(process(
                parent,
                "chrome",
                "/app.slice/app-com.google.Chrome-10.scope",
            )),
            9 => Some(process(1, &comm, &at)),
            _ => None,
        }
    }

    #[test]
    fn a_program_is_judged_by_what_started_its_first_process() {
        let names = ["chrome".to_owned()];
        let from = |parent, comm, at| starter(30, &names, browser(parent, comm, at));
        assert_eq!(
            from(9, "systemd", "/init.scope").as_deref(),
            Some("/init.scope"),
            "up past its own to his user manager"
        );
        assert_eq!(
            from(9, "pi", "/app.slice/mycelium-pi-4.scope").as_deref(),
            Some("/app.slice/mycelium-pi-4.scope"),
            "a pi session that started it is found, however deep its renderer"
        );
        assert_eq!(
            from(0, "", ""),
            None,
            "one with no starter is left as it is"
        );
        assert_eq!(from(8, "", ""), None, "a starter it can't read is left too");
    }

    #[test]
    fn a_kernel_that_reads_back_no_slice_is_unheard_and_a_race_is_not() {
        let asked = ordinary(3, SLICE_NS);
        assert_eq!(heard(&asked, &asked), Asked::Yes, "held");
        assert_eq!(heard(&asked, &ordinary(3, 0)), Asked::Unheard, "none held");
        let moved = Attr {
            policy: SCHED_BATCH,
            ..ordinary(3, 0)
        };
        assert_eq!(
            heard(&asked, &moved),
            Asked::Yes,
            "a policy changed between"
        );
        assert_eq!(
            heard(&asked, &ordinary(5, 0)),
            Asked::Yes,
            "a nice changed between"
        );
        assert_eq!(
            heard(&ordinary(3, 0), &ordinary(3, 0)),
            Asked::Yes,
            "a default given back"
        );
    }

    /// A thread of its own, niced as a hidden tab is, asks for the short
    /// slice through the kernel, keeps its nice, and a thread it starts
    /// afterwards inherits the slice. On a kernel that holds no fair
    /// slice, or in a build run at idle policy, it says so instead.
    #[test]
    fn a_thread_asks_through_the_kernel_and_what_it_starts_inherits() {
        let (tell, heard) = mpsc::channel();
        let (done, finish) = mpsc::channel::<()>();
        let worker = thread::spawn(move || {
            // SAFETY: gettid takes nothing and can't fail.
            let tid = unsafe { gettid() };
            let mut got = get(tid).expect("its own attributes read");
            got.nice = 3;
            got.runtime = 0;
            set(tid, &got).expect("it lowers its own nice");
            let outcome = ask_one(tid, SLICE_NS).expect("it asks");
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
            tell.send((outcome, get(tid).expect("read again"), child_attr))
                .expect("heard");
            finish.recv().ok();
        });
        let (outcome, own, child) = heard.recv().expect("the thread answers");
        done.send(()).ok();
        worker.join().expect("the thread ends");
        assert_eq!(own.nice, 3, "it keeps its nice");
        match outcome {
            Asked::Yes => {
                assert_eq!(own.runtime, SLICE_NS, "it holds the short slice");
                assert_eq!(
                    child.expect("its child read").runtime,
                    SLICE_NS,
                    "its child inherits"
                );
            }
            Asked::Unheard => assert_eq!(own.runtime, 0, "unheard reads no slice at all"),
            Asked::No => assert!(
                !matches!(own.policy, SCHED_OTHER | SCHED_BATCH),
                "only a thread that can't ask asks nothing"
            ),
        }
    }
}
