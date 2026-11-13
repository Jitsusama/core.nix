//! The Programs collection's rows as this daemon serves them
//! (`program_rows.rs`), and what the windows daemon says each program has
//! open, heard for as long as this runs: one watch of its `window`
//! records naming only each one's program, when it was looked at, and its
//! process and app id, which pair kitty's OS windows with niri's
//! (`kitty_tabs.rs`) and never change, so a retitle or a window moving
//! wakes nothing here. The rows are made again as that changes or an
//! entry does, and only while a watch wants them, the tabs' too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use programs::program_rows;
use serde_json::Value;

use super::{Daemon, Shared, agents, follow, keep, lock, programs, rows_watched, tabs};

/// The one watch held of the windows daemon.
const WATCH: &str = concat!(
    r#"{"id":"programs","ask":"watch","kind":"window","#,
    r#""fields":["owner","order","pid","opened"]}"#
);

/// Where the windows daemon answers: `PROGRAMS_WINDOWS`, else beside this
/// daemon's own socket, as systemd puts both.
pub fn socket(own: &Path) -> PathBuf {
    std::env::var_os("PROGRAMS_WINDOWS")
        .map_or_else(|| own.with_file_name("windows.sock"), PathBuf::from)
}

/// Hears the windows daemon for as long as this runs (follow.rs), and
/// Programs counts the windows last heard while it isn't answering.
pub fn hear(shared: &Arc<Shared>, socket: &Path) {
    let said = |said: &Value| windows_changed(shared, said);
    follow::follow(
        socket,
        WATCH,
        "the windows daemon",
        "Programs counts the windows last heard",
        &said,
    );
}

/// What the windows daemon told: its whole answer, or what changed and
/// went since; the rows made again where a watch wants them.
fn windows_changed(shared: &Shared, said: &Value) {
    let mut daemon = lock(&shared.daemon);
    follow::records(daemon.windows.get_or_insert_default(), said);
    if watched(&daemon) {
        make(shared, &mut daemon);
    }
    if tabs::watched(&daemon) {
        tabs::make(shared, &mut daemon);
    }
}

/// Whether a watch wants Programs' rows: one of the `row` kind whole, of
/// their place or none, or naming one of them.
pub fn watched(daemon: &Daemon) -> bool {
    rows_watched(daemon, "programs")
}

/// Makes the rows again from the entries, read once until a folder of them
/// changes, and the windows held, and keeps what changed, telling each
/// watch only the rows that read differently and those gone, the hidden
/// programs the settings name shown too.
pub fn make(shared: &Shared, daemon: &mut Daemon) {
    if daemon.catalogue.is_none() {
        daemon.catalogue = Some(programs(shared, daemon, None));
    }
    let shown = daemon.shown.clone();
    let windows = daemon.windows.iter().flat_map(BTreeMap::values);
    let opened = program_rows::opened(windows, &agents::windows(daemon));
    let rows = program_rows::rows(
        daemon.catalogue.as_deref().unwrap_or_default(),
        &opened,
        &shown,
    );
    let moved = daemon.rows.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}
