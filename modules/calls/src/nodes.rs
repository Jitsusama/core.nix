//! The mycelium nodes this machine runs, asked for every machine's record,
//! on Linux: each account running one has a socket a realm,
//! `mycelium/<realm>.sock`, in its runtime folder under RUNTIMES, and
//! answers one JSON line, a `Machines` request, with another. Root reaches
//! every account's; anyone else reaches their own, and another account's
//! folder they can't read isn't theirs to ask.
//!
//! Root asks as root, so it opens only what an account can't point
//! elsewhere: a socket itself, never a link, owned by the account whose
//! runtime folder holds it, the folder owned by that account too. An
//! answer is read for at most [`WITHIN`] in all and [`AT_MOST`] bytes, so
//! a node that drips or floods can't hold the asker.

use serde_json::Value;
use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a node has to take the request and answer it, in all.
const WITHIN: Duration = Duration::from_secs(3);

/// The longest answer read: a realm's machines' records run to tens of
/// kilobytes.
const AT_MOST: usize = 4 << 20;

/// What the nodes said: each answer, and why each node that didn't answer
/// couldn't.
#[derive(Debug, Default)]
pub struct Asked {
    /// Each answering node's answer.
    pub answers: Vec<Value>,
    /// Why each node that didn't answer couldn't, naming its socket.
    pub unanswered: Vec<String>,
}

/// Every node's answer, each asked whatever another said, so one that
/// won't answer drops none of the others. None at all when no account
/// runs a node, as on a machine without mycelium.
#[must_use]
pub fn answers(runtimes: &Path) -> Asked {
    let mut asked = Asked::default();
    for socket in sockets(runtimes) {
        match ask(&socket) {
            Ok(answer) => asked.answers.push(answer),
            Err(why) => asked.unanswered.push(why),
        }
    }
    asked
}

/// Each node's socket under `runtimes`, in order: only sockets owned by
/// the account whose runtime folder, itself the account's, holds them.
fn sockets(runtimes: &Path) -> Vec<PathBuf> {
    let Ok(accounts) = fs::read_dir(runtimes) else {
        return Vec::new();
    };
    let mut sockets: Vec<PathBuf> = accounts
        .flatten()
        .filter_map(|account| {
            let folder = fs::symlink_metadata(account.path()).ok()?;
            let folder = folder.is_dir().then_some(folder.uid())?;
            let nodes = account.path().join("mycelium");
            let owned = fs::symlink_metadata(&nodes)
                .ok()
                .filter(|nodes| nodes.is_dir() && nodes.uid() == folder)?;
            let entries = fs::read_dir(&nodes).ok()?;
            Some((owned.uid(), entries))
        })
        .flat_map(|(owner, entries)| entries.flatten().map(move |entry| (owner, entry.path())))
        .filter(|(owner, path)| {
            path.extension()
                .is_some_and(|extension| extension == "sock")
                && fs::symlink_metadata(path)
                    .is_ok_and(|socket| socket.file_type().is_socket() && socket.uid() == *owner)
        })
        .map(|(_, path)| path)
        .collect();
    sockets.sort();
    sockets
}

/// The node at `socket`'s list of machines.
fn ask(socket: &Path) -> Result<Value, String> {
    let node = || format!("the mycelium node at {}", socket.display());
    let failed = |error: std::io::Error| format!("{}: {error}", node());
    let deadline = Instant::now() + WITHIN;
    let mut stream = UnixStream::connect(socket).map_err(failed)?;
    stream.set_write_timeout(Some(WITHIN)).map_err(failed)?;
    stream.write_all(b"\"Machines\"\n").map_err(failed)?;
    let line = read_line(&mut stream, deadline).map_err(failed)?;
    let answer: Value = serde_json::from_slice(&line)
        .map_err(|error| format!("{} answered no JSON: {error}", node()))?;
    if answer["Ok"]["Machines"].is_array() {
        Ok(answer)
    } else {
        Err(format!(
            "{} listed no machines: {}",
            node(),
            String::from_utf8_lossy(&line).trim()
        ))
    }
}

/// One line from `stream`, read by `deadline` and at most [`AT_MOST`]
/// bytes long.
fn read_line(stream: &mut UnixStream, deadline: Instant) -> std::io::Result<Vec<u8>> {
    let mut line = Vec::new();
    let mut chunk = [0_u8; 8 << 10];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(ErrorKind::TimedOut.into());
        }
        stream.set_read_timeout(Some(left))?;
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(line);
        }
        line.extend_from_slice(&chunk[..read]);
        if let Some(end) = line.iter().position(|byte| *byte == b'\n') {
            line.truncate(end);
            return Ok(line);
        }
        if line.len() > AT_MOST {
            return Err(std::io::Error::other(format!(
                "answered more than {AT_MOST} bytes"
            )));
        }
    }
}
