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

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::state::{State, now};

/// How long the net may say nothing while hung before it aborts.
pub const QUIET_FOR: Duration = Duration::from_secs(20 * 60);

/// How often the guard looks.
const EVERY: Duration = Duration::from_secs(60);

/// What the kernel says a thread blocked on a lock, a join or a channel
/// waits in.
const ON_A_LOCK: &str = "__futex_wait";

/// Whether a net quiet for `quiet`, running `children` processes, whose
/// other threads wait in `waits`, is hung.
pub fn hung(quiet: Duration, children: usize, waits: &[String]) -> bool {
    quiet >= QUIET_FOR
        && children == 0
        && !waits.is_empty()
        && waits.iter().all(|wait| wait == ON_A_LOCK)
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
        .filter_map(|task| fs::read_to_string(task.path().join("wchan")).ok())
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

    #[test]
    fn a_quiet_net_running_nothing_with_every_thread_on_a_lock_is_hung() {
        let locked = waiting(&[ON_A_LOCK, ON_A_LOCK, ON_A_LOCK]);
        assert!(
            hung(QUIET_FOR, 0, &locked),
            "the hang of 12 November wasn't seen"
        );
        assert!(
            !hung(QUIET_FOR - Duration::from_secs(1), 0, &locked),
            "a net quiet for less than its while was taken as hung"
        );
        assert!(
            !hung(QUIET_FOR, 1, &locked),
            "a net running a test was taken as hung"
        );
        assert!(
            !hung(QUIET_FOR, 0, &waiting(&[ON_A_LOCK, "hrtimer_nanosleep"])),
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
