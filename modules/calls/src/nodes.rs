//! The mycelium nodes this machine runs, asked for every machine's record,
//! on Linux: each account running one has a socket a realm,
//! `mycelium/<realm>.sock`, in its runtime folder under RUNTIMES, and
//! answers one JSON line, a `Machines` request, with another. Root reaches
//! every account's; anyone else reaches their own, and another account's
//! folder they can't read isn't theirs to ask.

use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a node has to take the request and answer it.
const WITHIN: Duration = Duration::from_secs(3);

/// Every node's answer, or why one couldn't be asked: a node that won't
/// answer leaves its machine's peers unheard, so it can't tell. None at
/// all when no account runs a node, as on a machine without mycelium.
pub fn answers(runtimes: &Path) -> Result<Vec<Value>, String> {
    sockets(runtimes).iter().map(|socket| ask(socket)).collect()
}

/// Each node's socket under `runtimes`, in order.
fn sockets(runtimes: &Path) -> Vec<PathBuf> {
    let Ok(accounts) = fs::read_dir(runtimes) else {
        return Vec::new();
    };
    let mut sockets: Vec<PathBuf> = accounts
        .flatten()
        .filter_map(|account| fs::read_dir(account.path().join("mycelium")).ok())
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "sock")
        })
        .collect();
    sockets.sort();
    sockets
}

/// The node at `socket`'s list of machines.
fn ask(socket: &Path) -> Result<Value, String> {
    let node = || format!("the mycelium node at {}", socket.display());
    let failed = |error: std::io::Error| format!("{}: {error}", node());
    let mut stream = UnixStream::connect(socket).map_err(failed)?;
    stream.set_read_timeout(Some(WITHIN)).map_err(failed)?;
    stream.set_write_timeout(Some(WITHIN)).map_err(failed)?;
    stream.write_all(b"\"Machines\"\n").map_err(failed)?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(failed)?;
    let answer: Value = serde_json::from_str(&line)
        .map_err(|error| format!("{} answered no JSON: {error}", node()))?;
    if answer["Ok"]["Machines"].is_array() {
        Ok(answer)
    } else {
        Err(format!("{} listed no machines: {}", node(), line.trim()))
    }
}
