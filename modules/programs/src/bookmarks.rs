//! Chrome's bookmarks as this daemon serves them, each a row of their own
//! place (`bookmark_rows.rs`), read from each profile a source reads
//! Chrome's session from: as a watch of them begins, and again as Chrome
//! writes a bookmark file or its open pages change while one wants them
//! (heard.rs, tabs.rs), so home finds a bookmark made a moment ago and
//! never lists a page twice.

use std::collections::HashSet;
use std::path::Path;

use programs::bookmark_rows::{self, PLACE};

use super::{Daemon, Shared, keep};

/// Whether a watch wants the bookmarks' rows, as a whole place.
pub fn watched(daemon: &Daemon) -> bool {
    daemon.watches.iter().any(|watch| {
        watch.keys.is_none()
            && watch.place.as_deref() == Some(PLACE)
            && watch.kinds.iter().any(|kind| kind == "row")
    })
}

/// Makes the rows again from each profile's bookmark files, leaving out
/// every page held open now, keeping what changed and telling each watch
/// only the rows that read differently and those gone.
pub fn make(shared: &Shared, daemon: &mut Daemon) {
    let open: HashSet<&str> = shared
        .sources
        .iter()
        .filter_map(|source| daemon.held.get(&source.opened_key()?))
        .flat_map(|record| record["sites"].as_array().into_iter().flatten())
        .filter_map(|site| site["address"].as_str())
        .collect();
    let rows = shared
        .sources
        .iter()
        .filter_map(|source| Some((source.app.as_deref()?, source.session.as_ref()?)))
        .flat_map(|(app, session)| {
            bookmark_rows::rows(
                app,
                &bookmark_rows::read(Path::new(&session.profile)),
                &open,
            )
        })
        .collect();
    let moved = daemon.bookmarks.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}
