//! Joel's meetings, as the google daemon says them, and whether one is
//! near enough that no agent may open a camera.
//!
//! The meetings are kept, in memory and in the guard's state folder, from
//! the last answer the daemon gave, so a calendar that can't be read, as
//! when Google's names didn't resolve for five hours on 24 Nov, leaves the
//! guard up through a meeting it already knew of rather than down.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// How long before a meeting no agent may open a camera: Joel may be
/// setting up, and a camera in use then is one he sees and didn't start,
/// as at 09:51 on 24 Nov.
pub const AHEAD_MS: u64 = 15 * 60 * 1000;

/// How long the daemon is given to answer.
const ANSWER: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meeting {
    pub title: String,
    /// When it starts and ends, in ms since the epoch.
    pub start: u64,
    pub end: u64,
}

/// The meetings in the google daemon's answer to a list of them. One Joel
/// declined is left out, and so is a day-long one, which holds a day
/// rather than a call; one marked done ends then. These are the rules
/// call-under-load's calendar reads by (../call-under-load/src/gate.rs),
/// so the guard and the camera's check agree on what a meeting is.
///
/// # Errors
///
/// The daemon's refusal, or an answer without its records or the time
/// they were read, which it gives when it couldn't reach Google.
pub fn read(answer: &Value) -> Result<Vec<Meeting>, String> {
    if let Some(error) = answer.get("error") {
        return Err(format!("the google daemon refused: {error}"));
    }
    if answer["read"].as_u64().is_none() {
        return Err("the google daemon's answer has no read time".into());
    }
    let records = answer["records"]
        .as_array()
        .ok_or("the google daemon's answer has no records")?;
    Ok(records
        .iter()
        .map(|record| &record["meeting"])
        .filter(|meeting| meeting["allDay"] != true && meeting["answer"] != "no")
        .filter_map(|meeting| {
            let start = meeting["start"].as_u64()?;
            let end = meeting["end"].as_u64()?;
            let done = meeting["done"]["at"].as_u64().unwrap_or(end);
            Some(Meeting {
                // A title is anyone's text, and it goes to the journal.
                title: meeting["title"]
                    .as_str()
                    .unwrap_or("a meeting")
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect(),
                start,
                end: end.min(done),
            })
        })
        .collect())
}

/// The meeting that keeps the guard up at `now`, if any: one starting
/// within the quarter hour, or under way. Of several, the one ending last,
/// so the journal says how long the guard stays up.
pub fn near(meetings: &[Meeting], now: u64) -> Option<&Meeting> {
    meetings
        .iter()
        .filter(|meeting| meeting.start.saturating_sub(AHEAD_MS) <= now && now < meeting.end)
        .max_by_key(|meeting| meeting.end)
}

/// Asks the google daemon at `socket` for Joel's meetings.
///
/// # Errors
///
/// The socket can't be reached, or its answer isn't one line of JSON
/// within ten seconds.
pub fn ask(socket: &Path) -> Result<Value, String> {
    let seen = |error: std::io::Error| format!("{}: {error}", socket.display());
    let mut stream = UnixStream::connect(socket).map_err(seen)?;
    stream.set_read_timeout(Some(ANSWER)).map_err(seen)?;
    stream.set_write_timeout(Some(ANSWER)).map_err(seen)?;
    stream
        .write_all(b"{\"id\":1,\"ask\":\"list\",\"kind\":\"meeting\"}\n")
        .map_err(seen)?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).map_err(seen)?;
    serde_json::from_str(&line).map_err(|error| format!("{}: {error}", socket.display()))
}

/// The meetings kept at `file` by an earlier run, none if it has none.
///
/// # Errors
///
/// The file is there but can't be read as meetings.
pub fn kept(file: &Path) -> Result<Vec<Meeting>, String> {
    match std::fs::read(file) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", file.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps `meetings` at `file` for the next run, through a file beside it,
/// so a run stopped while writing leaves the last whole list.
///
/// # Errors
///
/// The file can't be written.
pub fn keep(file: &Path, meetings: &[Meeting]) -> Result<(), String> {
    let seen = |error: std::io::Error| format!("{}: {error}", file.display());
    let fresh = file.with_extension("new");
    let text = serde_json::to_vec(meetings).map_err(|error| error.to_string())?;
    std::fs::write(&fresh, text).map_err(seen)?;
    std::fs::rename(&fresh, file).map_err(seen)
}

#[cfg(test)]
#[path = "meetings_tests.rs"]
mod tests;
