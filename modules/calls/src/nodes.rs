//! The mycelium nodes this machine runs, asked for every machine's record,
//! on Linux: each account running one has a socket a realm,
//! `mycelium/<realm>.sock`, in its runtime folder under RUNTIMES, and
//! answers one JSON line, a `Machines` request, with another, the shape
//! the mycelium node this core locks speaks (`mycelium machines --socket`
//! says the same), so a lock bump that changes it shows here as every node
//! named unanswered. Root asks every account's; anyone else asks only
//! their own, since another account can open its folder to them and
//! answer as a node.
//!
//! Root asks as root, so it lists only what an account can't point
//! elsewhere: a socket itself, never a link, owned by the account whose
//! runtime folder holds it, the folder owned by that account too. The
//! account could still swap a link in before the connect, so root asks
//! nothing of a connection until its other end proves a process of that
//! account. Every node is asked at once, at most [`EACH_ACCOUNT`] an
//! account, each connected without waiting and given [`WITHIN`] in all
//! and [`AT_MOST`] bytes, so a node that won't take a connection, drips or
//! floods can't hold the asker past it.

use rustix::net::{self, AddressFamily, SocketAddrUnix, SocketFlags, SocketType, sockopt};
use serde_json::Value;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

/// How long the nodes have to take the request and answer it, in all: a
/// healthy node answers in milliseconds, so five seconds lets one slowed
/// by a build answer, while a node that won't still leaves a-call-is-on
/// well inside the ten seconds a shaper proof gives it, and that proof
/// hearing Joel come back within them. While a node won't answer, the
/// proof's once-a-second watch looks only every six or seven seconds,
/// this and PipeWire's look between; a longer wait here costs it that.
const WITHIN: Duration = Duration::from_secs(5);

/// The most nodes asked of one account, a realm each: an account offering
/// more is told so, and its first ones asked.
const EACH_ACCOUNT: usize = 16;

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
    let (nodes, crowded) = nodes(runtimes);
    let said: Vec<Result<Value, String>> = thread::scope(|scope| {
        let asking: Vec<_> = nodes
            .into_iter()
            .map(|node| {
                let socket = node.socket.clone();
                thread::Builder::new()
                    .spawn_scoped(scope, move || ask(&node, deadline))
                    .map_err(|error| format!("the mycelium node at {}: {error}", named(&socket)))
            })
            .collect();
        asking
            .into_iter()
            .map(|asking| {
                asking?
                    .join()
                    .unwrap_or_else(|_| Err("a mycelium node's asker panicked".to_owned()))
            })
            .collect()
    });
    let mut asked = Asked {
        unanswered: crowded,
        ..Asked::default()
    };
    for said in said {
        match said {
            Ok(answer) => asked.answers.push(answer),
            Err(why) => asked.unanswered.push(why),
        }
    }
    asked
}

/// Each node under `runtimes`, in order: only sockets owned by the account
/// whose runtime folder, itself the account's, holds them, the first
/// [`EACH_ACCOUNT`] of each; and what's said of each account offering more.
fn nodes(runtimes: &Path) -> (Vec<Node>, Vec<String>) {
    let Ok(accounts) = fs::read_dir(runtimes) else {
        return (Vec::new(), Vec::new());
    };
    let asker = rustix::process::geteuid();
    let mine = |owner: u32| asker.is_root() || owner == asker.as_raw();
    let mut accounts: Vec<(u32, PathBuf, fs::ReadDir)> = accounts
        .flatten()
        .filter_map(|account| {
            let folder = fs::symlink_metadata(account.path()).ok()?;
            let owner = folder
                .is_dir()
                .then_some(folder.uid())
                .filter(|owner| mine(*owner))?;
            let held = account.path().join("mycelium");
            fs::symlink_metadata(&held)
                .ok()
                .filter(|held| held.is_dir() && held.uid() == owner)?;
            let entries = fs::read_dir(&held).ok()?;
            Some((owner, held, entries))
        })
        .collect();
    accounts.sort_by(|one, other| one.1.cmp(&other.1));
    let mut nodes = Vec::new();
    let mut crowded = Vec::new();
    for (owner, held, entries) in accounts {
        let mut sockets: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "sock")
                    && fs::symlink_metadata(path)
                        .is_ok_and(|socket| socket.file_type().is_socket() && socket.uid() == owner)
            })
            .collect();
        sockets.sort();
        if sockets.len() > EACH_ACCOUNT {
            crowded.push(format!(
                "{} offers {} nodes, more than {EACH_ACCOUNT}, and the last {} aren't asked",
                named(&held),
                sockets.len(),
                sockets.len() - EACH_ACCOUNT
            ));
            sockets.truncate(EACH_ACCOUNT);
        }
        nodes.extend(sockets.into_iter().map(|socket| Node { socket, owner }));
    }
    (nodes, crowded)
}

/// The list of machines `node` gives by `deadline`.
fn ask(node: &Node, deadline: Instant) -> Result<Value, String> {
    let name = || format!("the mycelium node at {}", named(&node.socket));
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
        Err(format!("{} listed no machines: {}", name(), shown(&line)))
    }
}

/// `path` as it's said, its control characters escaped: any account names
/// its own sockets, and none of a name may steer a terminal.
fn named(path: &Path) -> String {
    path.display().to_string().escape_debug().to_string()
}

/// The start of what a node said, its control characters escaped, as an
/// answer that isn't a list of machines is shown: enough to say what went
/// wrong, none of it able to steer a terminal.
fn shown(line: &[u8]) -> String {
    let line = String::from_utf8_lossy(line);
    let line = line.trim();
    let start: String = line.chars().take(120).collect();
    let start = start.escape_debug().to_string();
    if line.chars().count() > 120 {
        format!("{start}...")
    } else {
        start
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
    let late = |line: &[u8]| {
        let said = if line.is_empty() {
            "nothing"
        } else {
            "only part of a line"
        };
        io::Error::new(
            ErrorKind::TimedOut,
            format!("answered {said} within {WITHIN:?}"),
        )
    };
    let mut line = Vec::new();
    let mut chunk = [0_u8; 8 << 10];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(late(&line));
        }
        stream.set_read_timeout(Some(left))?;
        let read = match stream.read(&mut chunk) {
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Err(late(&line));
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
