//! The net's own guard against hanging: a thread that, once the net has
//! said nothing for twenty minutes, runs nothing and has every other thread
//! waiting on a lock, keeps a finding and aborts, so systemd-coredump puts
//! each thread's stack in the journal and the unit's Restart starts it again.
//!
//! On 12 November the watcher said nothing from 07:02 to 08:15, mid-search
//! of vm-desktop-launcher-moves-windows, with all eleven threads waiting on
//! a futex and no process running: no merge in that hour was tested after
//! merging, and it never ended between rounds to hand over to the build a
//! switch installed. It was found by hand and restarted, with no stack to
//! say which lock.
//!
//! A net at rest sleeps between rounds or looks, a thread in a timed sleep
//! rather than a futex, and a net at work runs nix-offload or git as a
//! child, so neither reads as hung however long it is quiet.
//!
//! A thread's wait is read by the system call it is blocked in, never by
//! the kernel function `wchan` names: that name changes between kernels
//! (`__futex_wait`, `futex_wait_queue`, and on 7.3.4 `futex_do_wait`), so
//! a guard reading it went blind on each new kernel, while a system call's
//! number is fixed for an architecture.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::state::{State, now};

/// How long the net may say nothing while hung before it aborts.
pub const QUIET_FOR: Duration = Duration::from_mins(20);

/// How often the guard looks.
const EVERY: Duration = Duration::from_secs(60);

/// The system calls a thread blocked on a lock, a join or a channel waits
/// in: futex, and futex_waitv, which has one number on every architecture.
#[cfg(target_arch = "x86_64")]
const FUTEXES: &[i64] = &[202, 449];
#[cfg(target_arch = "aarch64")]
const FUTEXES: &[i64] = &[98, 449];
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const FUTEXES: &[i64] = &[];

/// The system calls a thread sleeping for a while waits in: nanosleep and
/// clock_nanosleep.
#[cfg(target_arch = "x86_64")]
const SLEEPS: &[i64] = &[35, 230];
#[cfg(target_arch = "aarch64")]
const SLEEPS: &[i64] = &[101, 115];
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const SLEEPS: &[i64] = &[];

/// What a thread waits in, named from its `/proc/<pid>/task/<tid>/syscall`
/// line, whose first word is the number of the call it is blocked in,
/// `-1` when it is blocked outside one, or `running`. A line that can't be
/// read names nothing the guard counts as a lock, so it never aborts on a
/// wait it couldn't read.
pub fn wait_in(syscall: &str) -> String {
    let call = syscall.split_whitespace().next().unwrap_or_default();
    match call.parse::<i64>() {
        Ok(number) if FUTEXES.contains(&number) => "futex".to_owned(),
        Ok(number) if SLEEPS.contains(&number) => "sleep".to_owned(),
        Ok(-1) => "outside a call".to_owned(),
        Ok(number) => format!("call {number}"),
        Err(_) if call == "running" => "running".to_owned(),
        Err(_) => "unread".to_owned(),
    }
}

/// Whether a thread waits in a futex, as one blocked on a lock, a join or
/// a channel does. A timed wait is a futex wait too; the round's own sleep
/// between rounds isn't one, so a net at rest still never reads as hung.
pub fn on_a_lock(wait: &str) -> bool {
    wait == "futex"
}

/// Whether a net quiet for `quiet`, running `children` processes, whose
/// other threads wait in `waits`, is hung.
pub fn hung(quiet: Duration, children: usize, waits: &[String]) -> bool {
    quiet >= QUIET_FOR
        && children == 0
        && !waits.is_empty()
        && waits.iter().all(|wait| on_a_lock(wait))
}

/// Guard the net for as long as it runs, from a thread of its own.
pub fn guard(state: &State) {
    loop {
        std::thread::sleep(EVERY);
        let quiet = quiet_for(&state.root.join("logs/net.log"));
        if hung(quiet, children(), &waits()) {
            let said = format!(
                "the net has said nothing for {} min, runs nothing and every thread waits on a \
                 lock, so it aborts for its threads' stacks to go to the journal and starts again",
                quiet.as_secs() / 60
            );
            state.say(&said);
            let _ = state.find(&serde_json::json!({
                "kind": "stalled", "test": "net", "commit": state.heads().1.unwrap_or_default(),
                "quiet_minutes": quiet.as_secs() / 60, "at": now(),
            }));
            std::process::abort();
        }
    }
}

/// How long since `log` was last written, or none when it can't be read.
fn quiet_for(log: &Path) -> Duration {
    fs::metadata(log)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .unwrap_or_default()
}

/// What the net's threads other than this one wait in.
fn waits() -> Vec<String> {
    let own = fs::read_link("/proc/thread-self").ok().and_then(|link| {
        link.file_name()
            .map(|tid| tid.to_string_lossy().into_owned())
    });
    fs::read_dir("/proc/self/task")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|task| own.as_deref() != Some(&*task.file_name().to_string_lossy()))
        .filter_map(|task| fs::read_to_string(task.path().join("syscall")).ok())
        .map(|syscall| wait_in(&syscall))
        .collect()
}

/// How many processes the net runs now.
fn children() -> usize {
    let own = std::process::id().to_string();
    fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_to_string(entry.path().join("stat")).ok())
        .filter(|stat| parent(stat) == Some(own.as_str()))
        .count()
}

/// The parent's pid in a `/proc/<pid>/stat` line, read past the name,
/// which may hold spaces and parentheses of its own.
fn parent(stat: &str) -> Option<&str> {
    stat.rsplit_once(") ")?.1.split(' ').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn waiting(on: &[&str]) -> Vec<String> {
        on.iter().map(|wait| (*wait).to_owned()).collect()
    }

    /// A `syscall` line for a thread blocked in call `number`.
    fn blocked_in(number: i64) -> String {
        format!("{number} 0x7f3a 0x80 0x0 0x0 0x0 0x0 0x7ffd 0x7f3b\n")
    }

    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    #[test]
    fn a_wait_is_named_by_its_call_whatever_the_kernel_calls_it() {
        for &futex in FUTEXES {
            assert_eq!(
                wait_in(&blocked_in(futex)),
                "futex",
                "a thread blocked in call {futex} isn't read as on a lock"
            );
        }
        for &sleep in SLEEPS {
            assert_eq!(
                wait_in(&blocked_in(sleep)),
                "sleep",
                "a thread in call {sleep} isn't read as asleep"
            );
        }
        assert_eq!(
            wait_in("running\n"),
            "running",
            "a running thread was misread"
        );
        assert_eq!(
            wait_in("-1 0x7ffd 0x7f3b\n"),
            "outside a call",
            "a thread blocked outside a call was misread"
        );
        assert_eq!(
            wait_in(&blocked_in(7)),
            "call 7",
            "another call was misread"
        );
        assert!(
            !on_a_lock(&wait_in("")),
            "a wait that couldn't be read was taken as a lock"
        );
    }

    #[test]
    fn a_quiet_net_running_nothing_with_every_thread_on_a_lock_is_hung() {
        let locked = waiting(&["futex", "futex", "futex"]);
        assert!(
            hung(QUIET_FOR, 0, &locked),
            "the hang of 12 November wasn't seen"
        );
        assert!(
            !hung(QUIET_FOR.saturating_sub(Duration::from_secs(1)), 0, &locked),
            "a net quiet for less than its while was taken as hung"
        );
        assert!(
            !hung(QUIET_FOR, 1, &locked),
            "a net running a test was taken as hung"
        );
        assert!(
            !hung(QUIET_FOR, 0, &waiting(&["futex", "sleep"])),
            "a net resting between rounds was taken as hung"
        );
    }

    #[test]
    fn a_parent_is_read_past_a_name_with_spaces() {
        assert_eq!(
            parent("42 (a (b) c) S 7 42 42 0"),
            Some("7"),
            "the parent wasn't read"
        );
        assert_eq!(parent("garbled"), None, "a line with no name gave a parent");
    }
}
