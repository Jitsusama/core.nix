//! Reading a source: which sources a get, a list or a watch names
//! (`sources_of`), running one's command or reading what it has open and
//! keeping what it said (`read_source`), and whether its list is due to be
//! read again (`due`, `changed_at`). Kept apart from main.rs so the
//! daemon's wiring and its reading each change for their own reasons.

use std::time::{Instant, SystemTime};

use programs::command;
use programs::source::{self, Source};
use serde_json::json;

use super::{
    Daemon, Run, Shared, TIMES, agents, bookmarks, fronts, hand_over, keep, lock, tabs, taken,
};

/// The sources whose `recent`, `listed` or `opened` key is one of these,
/// or every source with such a list when `keys` is none and its kind is
/// wanted.
pub(super) fn sources_of(
    shared: &Shared,
    run: Run,
    kinds: &[String],
    keys: Option<&[String]>,
) -> Vec<usize> {
    shared
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| match run {
            Run::List => source.lists(),
            Run::Opened => source.opened_key().is_some(),
        })
        .filter(|(_, source)| {
            let (key, kind) = match run {
                Run::List => (Some(source.list_key()), source.list_kind()),
                Run::Opened => (source.opened_key(), "opened"),
            };
            let app = source.app.as_deref().unwrap_or_default();
            let fronts = |keys: &[String]| run == Run::Opened && fronts::names(app, keys);
            match keys {
                Some(keys) => key.is_some_and(|key| keys.contains(&key)) || fronts(keys),
                None => kinds.iter().any(|k| k == kind),
            }
        })
        .map(|(at, _)| at)
        .collect()
}

/// Runs a source's command now, on this thread, and keeps its record. A
/// command that fails keeps the record from before, and says why.
pub(super) fn read_source(shared: &Shared, at: usize, what: Run) {
    let source = &shared.sources[at];
    let (argv, key) = match what {
        Run::List => (&source.list, source.list_key()),
        Run::Opened => (&source.opened, source.opened_key().unwrap_or_default()),
    };
    if what == Run::List {
        let mut daemon = lock(&shared.daemon);
        daemon.stale[at] = false;
        daemon.read_as_of[at] = changed_at(source);
    }
    let started = Instant::now();
    let read = (what == Run::Opened).then(|| source::open_now(source));
    let printed = if what == Run::List && source.list.is_empty() {
        Ok(source::files_in(&source.folders))
    } else if let Some(Some(read)) = read {
        read
    } else {
        command::run(argv)
    };
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut daemon = lock(&shared.daemon);
    daemon.took.push_back(took);
    while daemon.took.len() > TIMES {
        daemon.took.pop_front();
    }
    match printed {
        Ok(printed) => {
            let taking = key == taken::LIST;
            let answering = json!({"state": "answering"});
            let was = daemon.commands.insert(key, answering.clone());
            if what == Run::List && source.told {
                daemon.listers[at] = source::listers(&printed);
            }
            let record = match what {
                Run::List => Some(agents::listed(&mut daemon, at, source, &printed)),
                Run::Opened => source::opened(source, &printed),
            };
            keep(&mut daemon, record.into_iter().collect(), &[]);
            // Keeping told only what moved, so a command answering again
            // after it refused is said as it now stands.
            if was != Some(answering) {
                hand_over(&daemon);
            }
            if taking {
                taken::changed(shared, &mut daemon);
            }
            tabs::heard(shared, &mut daemon, at, what);
            // A page opened or shut leaves its bookmark's row or brings it
            // back, so home never lists one page twice.
            if what == Run::Opened && source.session.is_some() {
                bookmarks::changed(shared, &mut daemon);
            }
        }
        Err(why) => {
            // A list never read says it's empty and why, so a watch
            // waiting to hear every list once hears this one too and lets
            // go as the launcher closes; one read before keeps what it
            // last said, as the old reader kept its lines.
            if what == Run::List && !daemon.held.contains_key(&key) {
                let mut record = source::recent(source, "");
                record["refused"] = json!(why);
                keep(&mut daemon, vec![record], &[]);
            }
            daemon
                .commands
                .insert(key, json!({"state": "refused", "reason": why}));
            hand_over(&daemon);
            tabs::heard(shared, &mut daemon, at, what);
        }
    }
}

/// When a source's file last changed, or the latest of its folders, as a
/// file made, moved or removed in one changes it; none where it has none.
fn changed_at(source: &Source) -> Option<SystemTime> {
    let modified = |path: &str| {
        std::fs::metadata(path)
            .and_then(|file| file.modified())
            .ok()
    };
    match source.watch.as_deref() {
        Some(file) => modified(file),
        None => source
            .folders
            .iter()
            .filter_map(|folder| modified(folder))
            .max(),
    }
}

/// Whether a source's list is to be read again: it's asked and not told,
/// or told but naming a process that hasn't told yet, or its file changed
/// since it was last read, as inotify said or its time says, or it was told
/// it changed.
pub(super) fn due(shared: &Shared, daemon: &Daemon, at: usize) -> bool {
    let source = &shared.sources[at];
    let untold = || {
        daemon.listers[at]
            .iter()
            .any(|pid| !daemon.tellers.contains(pid))
    };
    (source.asked && (!source.told || untold()))
        || daemon.stale[at]
        || changed_at(source) != daemon.read_as_of[at]
}
