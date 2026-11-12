//! Chrome's open tabs as this daemon serves them, each a row of their
//! own place (`tab_rows.rs`), made from what each program reading
//! Chrome's session has open: read as a watch of them begins and again as
//! the session folder changes for as long as one wants them (heard.rs),
//! so home's search finds a tab Joel opened a moment ago.

use programs::tab_rows::{self, PLACE};

use super::{Daemon, Run, Shared, bookmarks, keep, lock, start};

/// Whether a watch wants the tabs' rows, as a whole place.
pub fn watched(daemon: &Daemon) -> bool {
    daemon.watches.iter().any(|watch| {
        watch.keys.is_none()
            && watch.place.as_deref() == Some(PLACE)
            && watch.kinds.iter().any(|kind| kind == "row")
    })
}

/// Whether what Chrome has open is wanted for a place's rows: the tabs',
/// or the bookmarks', which leave out a page open now.
pub fn wanted(daemon: &Daemon) -> bool {
    watched(daemon) || bookmarks::watched(daemon)
}

/// Reads what each program reading Chrome's session has open, as a watch
/// begins while the tabs or the bookmarks are watched, each on a thread
/// of its own: only
/// what isn't held yet, since what is was read again at every change of
/// the session while they were watched, so a watch beginning reads
/// nothing more.
pub fn read(shared: &std::sync::Arc<Shared>) {
    let unread: Vec<usize> = {
        let daemon = lock(&shared.daemon);
        if !wanted(&daemon) {
            return;
        }
        shared
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| source.session.is_some())
            .filter(|(_, source)| {
                source
                    .opened_key()
                    .is_some_and(|key| !daemon.held.contains_key(&key))
            })
            .map(|(at, _)| at)
            .collect()
    };
    for at in unread {
        start(shared, at, Run::Opened);
    }
}

/// Makes the rows again from what's held open, keeping what changed and
/// telling each watch only the rows that read differently and those gone.
pub fn make(shared: &Shared, daemon: &mut Daemon) {
    let rows = tab_rows::rows(
        shared
            .sources
            .iter()
            .filter(|source| source.session.is_some())
            .filter_map(|source| {
                let app = source.app.as_deref()?;
                Some((app, daemon.held.get(&source.opened_key()?)?))
            }),
    );
    let moved = daemon.tabs.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}
