//! Another daemon's watch, held for as long as this runs: asked once, and
//! asked again only once that daemon hangs up and, where its socket isn't
//! there, once inotify says it's made, so nothing is asked while nothing
//! changed. The windows daemon's (rows.rs) and the agents daemon's
//! (agents.rs) are held this way.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use programs::notify::Notify;
use serde_json::Value;

/// How long to wait after a daemon hung up before asking it again,
/// doubled each time it doesn't answer up to the longest, so one that
/// keeps failing isn't asked in a loop.
const AGAIN: Duration = Duration::from_secs(1);
const LONGEST: Duration = Duration::from_secs(300);

/// Holds `watch`, one line whose `id` is "programs", on the daemon at
/// `socket`, giving `said` each line it answers with. Each loss is said
/// once in the journal, naming `who` and what's done `meanwhile`.
pub fn follow(socket: &Path, watch: &str, who: &str, meanwhile: &str, said: &dyn Fn(&Value)) {
    let mut lost = false;
    let mut again = AGAIN;
    loop {
        match held(socket, watch, said) {
            Ok(()) => {
                lost = false;
                again = AGAIN;
            }
            Err(why) if !lost => {
                eprintln!(
                    "<4>{who} at {} isn't answering, so {meanwhile}: {why}",
                    socket.display()
                );
                lost = true;
            }
            Err(_) => again = (again * 2).min(LONGEST),
        }
        if socket.exists() {
            std::thread::sleep(again);
        } else if let Err(why) = made(socket) {
            eprintln!("<4>Programs can't hear {who} begin, so hears no more of it: {why}");
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

/// Holds the watch until the daemon hangs up.
fn held(socket: &Path, watch: &str, said: &dyn Fn(&Value)) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    writeln!(stream, "{watch}")?;
    for line in BufReader::new(stream).lines() {
        let Ok(answer) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        if answer["id"] == "programs" {
            said(&answer);
        }
    }
    Ok(())
}

/// What a watch's answer says of its records, each by its key: the whole
/// of them where it answers whole, and what changed and went since.
pub fn records(held: &mut std::collections::BTreeMap<String, Value>, said: &Value) {
    let keyed = |records: &Value| -> Vec<(String, Value)> {
        records
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|record| Some((record["key"].as_str()?.to_owned(), record.clone())))
            .collect()
    };
    if said.get("records").is_some() {
        *held = keyed(&said["records"]).into_iter().collect();
    }
    held.extend(keyed(&said["changed"]));
    for key in said["gone"].as_array().into_iter().flatten() {
        if let Some(key) = key.as_str() {
            held.remove(key);
        }
    }
}
