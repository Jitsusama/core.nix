//! When a source's command runs: one read of each source at a time,
//! whether a change, Tabs or a get asked for it, and once more after it
//! where a change came meanwhile, so the last read to end is the newest;
//! and a list refused while it's watched read again until it answers.

use std::sync::{Arc, PoisonError};
use std::time::Duration;

use serde_json::Value;

use super::{Daemon, Run, Shared, due, lock, read_source, tabs, watched};

/// How long a list refused while it's watched waits before each read
/// again, longer each time. Neovim's page in the looks-programs VM test
/// stood empty about two runs in five on 2026-11-16: the machine stalled
/// some 17 s as the launcher opened, open-now's read was refused, and
/// nothing read it again while the page watched. These outlast such a
/// stall and a command's own 10 s wait, and then leave it to a change.
const PAUSES: [Duration; 5] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
    Duration::from_secs(8),
    Duration::from_secs(16),
];

/// Runs a source's command on a thread of its own, or, while it runs
/// already, once more after it, so the last change always counts. A list
/// is read once more only if it's still due as the read ends: one asked
/// for by a change the running read already took, as a watch beginning
/// does a moment after the change that's read, would read it twice. Nor
/// is it read once more if nothing watches it now: its file changing
/// after its watch went would have it read for nobody, and a get would
/// read it again anyway. What Tabs has open reads kitty's list too, so it
/// counts as watching.
pub fn start(shared: &Arc<Shared>, at: usize, what: Run) {
    {
        let mut daemon = lock(&shared.daemon);
        if !daemon.running.insert((at, what)) {
            daemon.again.insert((at, what));
            return;
        }
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        read_claimed(&shared, at, what);
        again_while_refused(&shared, at, what);
    });
}

/// Reads a list again after each of `PAUSES` while its last read was
/// refused and it's still watched, so a page doesn't stand without what
/// it holds until its next opening; a read already under way stands for
/// the next.
fn again_while_refused(shared: &Shared, at: usize, what: Run) {
    if what != Run::List {
        return;
    }
    let key = shared.sources[at].list_key();
    for pause in PAUSES {
        if !refused_and_watched(&lock(&shared.daemon), &key) {
            return;
        }
        std::thread::sleep(pause);
        {
            let mut daemon = lock(&shared.daemon);
            if !refused_and_watched(&daemon, &key) {
                return;
            }
            if !daemon.running.insert((at, what)) {
                daemon.again.insert((at, what));
                continue;
            }
        }
        read_claimed(shared, at, what);
    }
}

/// Whether the last read of the list with this key was refused, and a
/// watch still waits on it.
fn refused_and_watched(daemon: &Daemon, key: &str) -> bool {
    let state = daemon.commands.get(key).and_then(|said| said.get("state"));
    state.and_then(Value::as_str) == Some("refused") && watched(daemon, key)
}

/// Reads a source whose read this thread holds in `running`, and once
/// more while one was asked for meanwhile, as `start` says; then lets it
/// go, waking a get waiting on it.
pub fn read_claimed(shared: &Shared, at: usize, what: Run) {
    loop {
        read_source(shared, at, what);
        let mut daemon = lock(&shared.daemon);
        let asked = daemon.again.remove(&(at, what));
        let again = asked
            && (what == Run::Opened
                || (due(shared, &daemon, at)
                    && (watched(&daemon, &shared.sources[at].list_key())
                        || tabs::wanted(&daemon))));
        if !again {
            daemon.running.remove(&(at, what));
            shared.read.notify_all();
            return;
        }
    }
}

/// Waits for a read of this source under way to end, then holds the next
/// in `running` for this thread where `wanted` says it's to be read, so
/// a read a change starts meanwhile waits to follow it, rather than
/// running beside it and ending first, its list then kept over by this
/// older one.
pub fn claim(shared: &Shared, at: usize, what: Run, wanted: impl Fn(&Daemon) -> bool) -> bool {
    let mut daemon = lock(&shared.daemon);
    while daemon.running.contains(&(at, what)) {
        daemon = shared
            .read
            .wait(daemon)
            .unwrap_or_else(PoisonError::into_inner);
    }
    let claimed = wanted(&daemon);
    if claimed {
        daemon.running.insert((at, what));
    }
    claimed
}
