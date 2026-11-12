//! google-people SOCKET: everyone Joel's meetings and mail name, as a
//! People source on its line protocol, which the people daemon runs as a
//! child (modules/people). It says `{"roster": true}`, then a line for
//! each person google's daemon at SOCKET names (src/people.rs), and
//! another as each changes or goes, watching only the meetings and the
//! mail, so it wakes only when one of those does.
//!
//! While google's daemon isn't there, as before Joel signs in, it looks
//! again the moment its socket is made in its folder, which systemd does
//! as google.socket starts, and otherwise after FIRST, then twice as long
//! each time up to LOOKS, for a socket there that won't answer. Once a
//! connection it had ends, it looks again after FIRST and so on: the next
//! connection's answer says who's gone, as a restart leaves most where
//! they were, and if none comes within LOOKS everyone it said is gone,
//! since what the account held may not be Joel's to see any more.
//! What the people daemon writes it (step-ins, asks ahead) is read and
//! let go: a roster isn't asked, and it ends once the daemon does.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use google::people::{Met, changes, people};
use inotify::{Inotify, WatchMask};
use serde_json::{Value, json};

/// How long it first waits to look for google's daemon again, short as a
/// restart is, so a meeting stepped into just after still names its
/// guests.
const FIRST: Duration = Duration::from_secs(1);

/// The longest it waits to look again, and how long those it said stay
/// once google's daemon has gone.
const LOOKS: Duration = Duration::from_secs(15);

fn main() -> ExitCode {
    let Some(socket) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("<3>usage: google-people SOCKET");
        return ExitCode::from(2);
    };
    // A socket named from the runtime folder, as the people daemon's
    // sources can't say where that is.
    let socket = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(runtime) if socket.is_relative() => Path::new(&runtime).join(socket),
        _ => socket,
    };
    // Its input is read to its end, so the daemon never waits on writing
    // it, and its end is the daemon's.
    std::thread::spawn(|| {
        for line in std::io::stdin().lock().lines() {
            if line.is_err() {
                break;
            }
        }
        std::process::exit(0);
    });
    let mut told: BTreeMap<String, Met> = BTreeMap::new();
    if !say(&[json!({"roster": true})]) {
        return ExitCode::SUCCESS;
    }
    // Watched before it's first looked for, so a socket made since is
    // heard, waiting, at the first wait.
    let mut appearing = appearing(&socket);
    let mut lost = Instant::now();
    let mut wait = FIRST;
    loop {
        let ended = match UnixStream::connect(&socket) {
            Ok(stream) => {
                let said = follow(stream, &mut told);
                lost = Instant::now();
                wait = FIRST;
                said
            }
            Err(error) => Err(error),
        };
        if let Err(error) = ended {
            eprintln!(
                "<6>google's daemon at {} isn't answering: {error}",
                socket.display()
            );
        }
        if !told.is_empty() && lost.elapsed() >= LOOKS {
            let went = changes(&told, &BTreeMap::new());
            told.clear();
            if !say(&went) {
                return ExitCode::SUCCESS;
            }
        }
        let made;
        (appearing, made) = waited(appearing, wait);
        wait = next_wait(wait, made);
    }
}

/// How long to wait after this look: twice as long up to LOOKS, or FIRST
/// again where the socket was just made. systemd makes the file before
/// google's daemon listens on it, so the look it wakes is often refused,
/// and doubling then left people 15 s late, after a meeting's guests
/// were shown (RTOIMQ, 2026-11-12).
fn next_wait(wait: Duration, made: bool) -> Duration {
    if made { FIRST } else { (wait * 2).min(LOOKS) }
}

/// Tells each time a file of the socket's name is made in its folder, so
/// the socket systemd makes as google.socket starts is looked at once
/// (a 15 s wait for people after google started, XY9H5C, 2026-11-11).
/// None where the folder can't be watched, said once, and then it waits.
fn appearing(socket: &Path) -> Option<Receiver<()>> {
    let (folder, name) = (socket.parent()?, socket.file_name()?.to_owned());
    let inotify = Inotify::init().and_then(|inotify| {
        inotify
            .watches()
            .add(folder, WatchMask::CREATE | WatchMask::MOVED_TO)
            .map(|_| inotify)
    });
    let mut inotify = inotify
        .inspect_err(|error| {
            eprintln!(
                "<4>google-people can't watch {} for google's socket, so it looks after \
                 a wait: {error}",
                folder.display()
            );
        })
        .ok()?;
    let (tell, appeared) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(events) = inotify.read_events_blocking(&mut buffer) {
            let made = events
                .into_iter()
                .any(|event| event.name == Some(name.as_os_str()));
            if made && tell.send(()).is_err() {
                return;
            }
        }
    });
    Some(appeared)
}

/// Waits `wait`, or less where the socket is made meanwhile, and gives
/// back what tells of that while it still can, and whether it was made.
fn waited(appearing: Option<Receiver<()>>, wait: Duration) -> (Option<Receiver<()>>, bool) {
    let Some(appeared) = appearing else {
        std::thread::sleep(wait);
        return (None, false);
    };
    match appeared.recv_timeout(wait) {
        Ok(()) => {
            // A burst of events is one making.
            while appeared.try_recv().is_ok() {}
            (Some(appeared), true)
        }
        Err(RecvTimeoutError::Timeout) => (Some(appeared), false),
        Err(RecvTimeoutError::Disconnected) => {
            eprintln!(
                "<4>google-people stopped hearing its socket's folder, so it looks after a wait"
            );
            std::thread::sleep(wait);
            (None, false)
        }
    }
}

/// Says each line to the people daemon, or false once it's gone.
fn say(lines: &[Value]) -> bool {
    let mut out = std::io::stdout().lock();
    lines.iter().all(|line| writeln!(out, "{line}").is_ok()) && out.flush().is_ok()
}

/// Watches the meetings and the mail on `stream`, saying who they name as
/// that changes, until the connection ends.
fn follow(mut stream: UnixStream, told: &mut BTreeMap<String, Met>) -> std::io::Result<()> {
    let watch = json!({"id": "people", "ask": "watch", "kind": ["meeting", "mail", "held"]});
    writeln!(stream, "{watch}")?;
    let mut records: BTreeMap<String, Value> = BTreeMap::new();
    for line in BufReader::new(stream).lines() {
        let Ok(said) = serde_json::from_str::<Value>(&line?) else {
            continue;
        };
        if !took(&said, &mut records) {
            continue;
        }
        let found = people(&records);
        if !say(&changes(told, &found)) {
            std::process::exit(0);
        }
        *told = found;
    }
    Ok(())
}

/// Takes the records an answer or a change says into those held, and
/// whether any were said.
fn took(said: &Value, records: &mut BTreeMap<String, Value>) -> bool {
    let keyed = |record: &Value| {
        record
            .get("key")
            .and_then(Value::as_str)
            .map(|key| (key.to_owned(), record.clone()))
    };
    if let Some(whole) = said.get("records").and_then(Value::as_array) {
        *records = whole.iter().filter_map(keyed).collect();
        return true;
    }
    let changed = said.get("changed").and_then(Value::as_array);
    let gone = said.get("gone").and_then(Value::as_array);
    for (key, record) in changed.into_iter().flatten().filter_map(keyed) {
        records.insert(key, record);
    }
    for key in gone.into_iter().flatten().filter_map(Value::as_str) {
        records.remove(key);
    }
    changed.is_some() || gone.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_that_found_nothing_waits_twice_as_long_up_to_looks() {
        assert_eq!(next_wait(FIRST, false), FIRST * 2);
        assert_eq!(next_wait(LOOKS, false), LOOKS);
    }

    #[test]
    fn a_socket_just_made_is_looked_at_again_soon() {
        assert_eq!(next_wait(Duration::from_secs(8), true), FIRST);
    }
}
