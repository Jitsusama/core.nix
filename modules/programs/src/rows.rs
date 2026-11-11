//! The Programs collection's rows as this daemon serves them
//! (`program_rows.rs`), and what the windows daemon says each program has
//! open, heard for as long as this runs: one watch of its `window`
//! records naming only each one's program and when it was looked at, so a
//! retitle or a window moving wakes nothing here. The rows are made again
//! as that changes or an entry does, and only while a watch wants them.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use programs::notify::Notify;
use programs::program_rows;
use serde_json::Value;

use super::{Daemon, Shared, keep, lock, programs};

/// How long to wait after the windows daemon hung up before asking it
/// again, doubled each time it doesn't answer up to the longest, so one
/// that keeps failing isn't asked in a loop.
const AGAIN: Duration = Duration::from_secs(1);
const LONGEST: Duration = Duration::from_secs(300);

/// The one watch held of the windows daemon.
const WATCH: &str = r#"{"id":"programs","ask":"watch","kind":"window","fields":["owner","order"]}"#;

/// Where the windows daemon answers: `PROGRAMS_WINDOWS`, else beside this
/// daemon's own socket, as systemd puts both.
pub fn socket(own: &Path) -> PathBuf {
    std::env::var_os("PROGRAMS_WINDOWS")
        .map_or_else(|| own.with_file_name("windows.sock"), PathBuf::from)
}

/// Hears the windows daemon for as long as this runs, asking again once
/// it hangs up and, where its socket isn't there, once inotify says it's
/// made, so nothing is asked while nothing changed. Each loss is said once
/// in the journal, and Programs counts the windows last heard meanwhile.
pub fn hear(shared: &Arc<Shared>, socket: &Path) {
    let mut lost = false;
    let mut again = AGAIN;
    loop {
        match follow(shared, socket) {
            Ok(()) => {
                lost = false;
                again = AGAIN;
            }
            Err(why) if !lost => {
                eprintln!(
                    "<4>the windows daemon at {} isn't answering, so Programs counts the windows \
                     last heard: {why}",
                    socket.display()
                );
                lost = true;
            }
            Err(_) => again = (again * 2).min(LONGEST),
        }
        if socket.exists() {
            std::thread::sleep(again);
        } else if let Err(why) = made(socket) {
            eprintln!("<4>Programs can't hear the windows daemon begin, so counts no more: {why}");
            return;
        }
    }
}

/// Waits until the socket's folder says it was made.
fn made(socket: &Path) -> std::io::Result<()> {
    let notify = Notify::new()?;
    let folder = socket.parent().unwrap_or(Path::new("/"));
    let name = socket
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    notify
        .watch(folder)
        .ok_or_else(|| std::io::Error::other(format!("{} can't be watched", folder.display())))?;
    while !socket.exists() {
        if notify
            .changes()?
            .iter()
            .any(|(_, changed)| Some(changed) == name.as_ref())
        {
            break;
        }
    }
    Ok(())
}

/// Watches the windows daemon until it hangs up.
fn follow(shared: &Arc<Shared>, socket: &Path) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{WATCH}")?;
    for line in BufReader::new(stream).lines() {
        let Ok(said) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        if said["id"] == "programs" {
            windows_changed(shared, &said);
        }
    }
    Ok(())
}

/// What the windows daemon told: its whole answer, or what changed and
/// went since; the rows made again where a watch wants them.
fn windows_changed(shared: &Shared, said: &Value) {
    let mut daemon = lock(&shared.daemon);
    let keyed = |records: &Value| -> Vec<(String, Value)> {
        records
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record.clone())))
            .collect()
    };
    if said.get("records").is_some() {
        daemon.windows = keyed(&said["records"])
            .into_iter()
            .collect::<BTreeMap<_, _>>();
    }
    daemon.windows.extend(keyed(&said["changed"]));
    for key in said["gone"].as_array().into_iter().flatten() {
        if let Some(key) = key.as_str() {
            daemon.windows.remove(key);
        }
    }
    if watched(&daemon) {
        make(shared, &mut daemon, &[]);
    }
}

/// Whether a watch wants Programs' rows: one of the `row` kind whole, of
/// their place or none, or naming one of them.
pub fn watched(daemon: &Daemon) -> bool {
    daemon.watches.iter().any(|watch| match &watch.keys {
        Some(keys) => keys.iter().any(|key| key.starts_with("row:programs:")),
        None => {
            watch
                .place
                .as_deref()
                .is_none_or(|place| place == "programs")
                && watch.kinds.iter().any(|kind| kind == "row")
        }
    })
}

/// Makes the rows again from the entries, read once until a folder of them
/// changes, and the windows held, and keeps what changed, telling each
/// watch only the rows that read differently and those gone. `also` names
/// what a watch about to be held shows.
pub fn make(shared: &Shared, daemon: &mut Daemon, also: &[String]) {
    if daemon.catalogue.is_none() {
        daemon.catalogue = Some(programs(shared, daemon, None));
    }
    let shown: Vec<String> = daemon
        .watches
        .iter()
        .flat_map(|watch| watch.shown.iter().cloned())
        .chain(also.iter().cloned())
        .collect();
    let opened = program_rows::opened(daemon.windows.values());
    let rows = program_rows::rows(
        daemon.catalogue.as_deref().unwrap_or_default(),
        &opened,
        &shown,
    );
    let moved = daemon.rows.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}
