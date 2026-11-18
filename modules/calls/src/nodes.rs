//! The mycelium nodes this machine runs, asked for every machine's record,
//! on Linux: each account running one has a socket a realm,
//! `mycelium/<realm>.sock`, in its runtime folder under RUNTIMES, and
//! answers one JSON line, a `Machines` request, with another. Root reaches
//! every account's; anyone else reaches their own, and another account's
//! folder they can't read isn't theirs to ask.
//!
//! Root asks as root, so it opens only what an account can't point
//! elsewhere: a socket itself, never a link, owned by the account whose
//! runtime folder holds it, the folder owned by that account too; and it
//! hears an answer only from a process of that account, which a link
//! swapped in after those checks can't fake. Every node is asked at once,
//! each connected without waiting and given [`WITHIN`] in all and
//! [`AT_MOST`] bytes, so a node that won't take a connection, drips or
//! floods, or a thousand of them, can't hold the asker past it.

use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType, sockopt};
use serde_json::Value;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// How long the nodes have to take the request and answer it, in all.
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

/// A node's socket, and the account it must answer as.
struct Node {
    socket: PathBuf,
    owner: u32,
}

/// Every node's answer, each asked at once and whatever another said, so
/// one that won't answer drops none of the others. None at all when no
/// account runs a node, as on a machine without mycelium.
#[must_use]
pub fn answers(runtimes: &Path) -> Asked {
    let deadline = Instant::now() + WITHIN;
    let said: Vec<Result<Value, String>> = thread::scope(|scope| {
        let asking: Vec<_> = nodes(runtimes)
            .into_iter()
            .map(|node| scope.spawn(move || ask(&node, deadline)))
            .collect();
        asking
            .into_iter()
            .map(|asked| {
                asked
                    .join()
                    .unwrap_or_else(|_| Err("a mycelium node's asker panicked".to_owned()))
            })
            .collect()
    });
    let mut asked = Asked::default();
    for said in said {
        match said {
            Ok(answer) => asked.answers.push(answer),
            Err(why) => asked.unanswered.push(why),
        }
    }
    asked
}

/// Each node under `runtimes`, in order: only sockets owned by the account
/// whose runtime folder, itself the account's, holds them.
fn nodes(runtimes: &Path) -> Vec<Node> {
    let Ok(accounts) = fs::read_dir(runtimes) else {
        return Vec::new();
    };
    let mut nodes: Vec<Node> = accounts
        .flatten()
        .filter_map(|account| {
            let folder = fs::symlink_metadata(account.path()).ok()?;
            let owner = folder.is_dir().then_some(folder.uid())?;
            let held = account.path().join("mycelium");
            fs::symlink_metadata(&held)
                .ok()
                .filter(|held| held.is_dir() && held.uid() == owner)?;
            Some((owner, fs::read_dir(&held).ok()?))
        })
        .flat_map(|(owner, entries)| entries.flatten().map(move |entry| (owner, entry.path())))
        .filter(|(owner, path)| {
            path.extension()
                .is_some_and(|extension| extension == "sock")
                && fs::symlink_metadata(path)
                    .is_ok_and(|socket| socket.file_type().is_socket() && socket.uid() == *owner)
        })
        .map(|(owner, socket)| Node { socket, owner })
        .collect();
    nodes.sort_by(|one, other| one.socket.cmp(&other.socket));
    nodes
}

/// The list of machines `node` gives by `deadline`.
fn ask(node: &Node, deadline: Instant) -> Result<Value, String> {
    let name = || format!("the mycelium node at {}", node.socket.display());
    let failed = |error: io::Error| format!("{}: {error}", name());
    let mut stream = connect(&node.socket).map_err(failed)?;
    let answerer = sockopt::socket_peercred(&stream)
        .map_err(io::Error::from)
        .map_err(failed)?
        .uid
        .as_raw();
    if answerer != node.owner {
        return Err(format!(
            "{} answers as uid {answerer}, not its folder's {}",
            name(),
            node.owner
        ));
    }
    stream.set_write_timeout(Some(WITHIN)).map_err(failed)?;
    stream.write_all(b"\"Machines\"\n").map_err(failed)?;
    let line = read_line(&mut stream, deadline).map_err(failed)?;
    let answer: Value = serde_json::from_slice(&line)
        .map_err(|error| format!("{} answered no JSON: {error}", name()))?;
    if answer["Ok"]["Machines"].is_array() {
        Ok(answer)
    } else {
        Err(format!(
            "{} listed no machines: {}",
            name(),
            String::from_utf8_lossy(&line).trim()
        ))
    }
}

/// A stream to `socket`, connected without waiting: a node whose queue is
/// full refuses at once rather than holding the asker.
fn connect(socket: &Path) -> io::Result<UnixStream> {
    let stream = net::socket_with(
        AddressFamily::UNIX,
        SocketType::STREAM,
        SocketFlags::CLOEXEC | SocketFlags::NONBLOCK,
        None,
    )?;
    net::connect(&stream, &SocketAddrUnix::new(socket)?)?;
    let stream = UnixStream::from(stream);
    stream.set_nonblocking(false)?;
    Ok(stream)
}

/// One line from `stream`, read by `deadline` and at most [`AT_MOST`]
/// bytes long.
fn read_line(stream: &mut UnixStream, deadline: Instant) -> io::Result<Vec<u8>> {
    let late = || {
        io::Error::new(
            ErrorKind::TimedOut,
            format!("answered nothing within {WITHIN:?}"),
        )
    };
    let mut line = Vec::new();
    let mut chunk = [0_u8; 8 << 10];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(late());
        }
        stream.set_read_timeout(Some(left))?;
        let read = match stream.read(&mut chunk) {
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Err(late());
            }
            read => read?,
        };
        if read == 0 {
            return Ok(line);
        }
        if let Some(end) = chunk[..read].iter().position(|byte| *byte == b'\n') {
            line.extend_from_slice(&chunk[..end]);
            return Ok(line);
        }
        line.extend_from_slice(&chunk[..read]);
        if line.len() > AT_MOST {
            return Err(io::Error::other(format!(
                "answered more than {AT_MOST} bytes"
            )));
        }
    }
}
