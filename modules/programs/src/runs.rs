//! When a source's command runs: one read of each source at a time,
//! whether a change, Tabs or a get asked for it, and once more after it
//! where a change came meanwhile, so the last read to end is the newest.

use std::sync::{Arc, PoisonError};

use super::{Daemon, Run, Shared, due, lock, read_source, tabs, watched};

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
    std::thread::spawn(move || read_claimed(&shared, at, what));
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
