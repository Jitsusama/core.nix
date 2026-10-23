//! The system daemon's socket: one JSON request a connection, answered
//! with one JSON line. The kernel says who asks, each account may ask each
//! kind of request once a second, and a request is cut off past 64 KiB or
//! a second of silence, so no asker can hold the daemon up. A reply may
//! come later, from elsewhere, while the server hears others.

use crate::band::Band;
use crate::vouch::Answer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often each account may ask the system daemon each kind of request.
pub const ASK_EVERY: Duration = Duration::from_millis(500);

/// The refusal of an ask that came too soon after the last, which an
/// asker meets by asking again once `ASK_EVERY` has passed.
pub const TOO_SOON: &str = "too soon: ask at most twice a second";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    /// The processes touching the asker's files beneath these roots.
    Processes { roots: Vec<PathBuf> },
    /// A user daemon's heartbeat: the filesystems it watches, by a path on
    /// each. Answered with those the system collects garbage on, whose
    /// passes the user daemon then drives.
    Watching { paths: Vec<PathBuf> },
    /// The garbage on the filesystem holding `path`, taken as a pass in
    /// `band` would take it. Answered once that pass ends.
    Collect { path: PathBuf, band: Band },
}

impl Request {
    fn kind(&self) -> &'static str {
        match self {
            Request::Processes { .. } => "processes",
            Request::Watching { .. } => "watching",
            Request::Collect { .. } => "collect",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Reply {
    Answer(Answer),
    /// Of the paths watched, those on a filesystem the system collects on.
    Collects(Vec<PathBuf>),
    /// How the collecting pass ended.
    Collected(String),
    Error(String),
}

/// A connection waiting for its reply, which may be sent from any thread.
pub struct Responder {
    stream: UnixStream,
}

impl Responder {
    pub fn new(stream: UnixStream) -> Responder {
        Responder { stream }
    }

    /// Sends the reply and closes the connection. An asker that went away
    /// hears nothing, which is its own concern.
    pub fn send(mut self, reply: &Reply) {
        let mut line = serde_json::to_vec(reply).unwrap_or_else(|error| {
            serde_json::to_vec(&Reply::Error(format!("the answer won't go: {error}")))
                .unwrap_or_default()
        });
        line.push(b'\n');
        let _ = self.stream.write_all(&line);
    }
}

/// The most a request may hold.
const REQUEST_LIMIT: u64 = 64 << 10;
/// The most an answer may hold.
const ANSWER_LIMIT: u64 = 16 << 20;
/// How long an asker may take to send its request.
const PATIENCE: Duration = Duration::from_secs(1);

pub struct Server {
    listener: UnixListener,
    every: Duration,
    asked: HashMap<(u32, &'static str), Instant>,
}

impl Server {
    /// Listens at `path`, which any account may connect to, replacing a
    /// socket an earlier run left. Each account may ask once `every`.
    pub fn bind(path: &Path, every: Duration) -> io::Result<Server> {
        if let Ok(meta) = fs::symlink_metadata(path)
            && meta.file_type().is_socket()
        {
            fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o666))?;
        Ok(Server {
            listener,
            every,
            asked: HashMap::new(),
        })
    }

    /// Takes one connection and hands its request to `handle`, with the
    /// connection to reply on, now or later.
    pub fn serve(&mut self, handle: &mut dyn FnMut(u32, Request, Responder)) -> io::Result<()> {
        let (stream, _) = self.listener.accept()?;
        match self.read(&stream) {
            Ok((uid, request)) => handle(uid, request, Responder::new(stream)),
            Err(why) => Responder::new(stream).send(&Reply::Error(why)),
        }
        Ok(())
    }

    fn read(&mut self, stream: &UnixStream) -> Result<(u32, Request), String> {
        let uid = peer(stream).map_err(|error| format!("who asks can't be told: {error}"))?;
        stream
            .set_read_timeout(Some(PATIENCE))
            .and_then(|()| stream.set_write_timeout(Some(PATIENCE)))
            .map_err(|error| error.to_string())?;
        // Read before refusing anything: a connection closed with its
        // request unread is reset, and the asker never hears why.
        let mut line = Vec::new();
        BufReader::new(stream.take(REQUEST_LIMIT))
            .read_until(b'\n', &mut line)
            .map_err(|error| format!("reading the request: {error}"))?;
        if !line.ends_with(b"\n") {
            return Err("a request is one line of at most 64 KiB".to_owned());
        }
        let request: Request = serde_json::from_slice(&line)
            .map_err(|error| format!("the request won't read: {error}"))?;
        let now = Instant::now();
        let turn = (uid, request.kind());
        if self
            .asked
            .get(&turn)
            .is_some_and(|at| now.duration_since(*at) < self.every)
        {
            return Err(TOO_SOON.to_owned());
        }
        self.asked.insert(turn, now);
        Ok((uid, request))
    }
}

/// The account on the other end of a socket, as the kernel says.
#[cfg(target_os = "linux")]
fn peer(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size =
        libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>()).map_err(io::Error::other)?;
    // SAFETY: credentials is a ucred of the size given, which the kernel
    // fills in for SO_PEERCRED.
    let done = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut size,
        )
    };
    if done != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

/// The account on the other end of a socket, as macOS's kernel says.
#[cfg(target_os = "macos")]
fn peer(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = std::mem::MaybeUninit::<libc::xucred>::zeroed();
    let mut size =
        libc::socklen_t::try_from(std::mem::size_of::<libc::xucred>()).map_err(io::Error::other)?;
    // SAFETY: credentials is an xucred of the size given, which the kernel
    // fills in for LOCAL_PEERCRED.
    let done = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERCRED,
            credentials.as_mut_ptr().cast(),
            &raw mut size,
        )
    };
    if done != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel filled it in, as it answered 0.
    let credentials = unsafe { credentials.assume_init() };
    if credentials.cr_version != libc::XUCRED_VERSION {
        return Err(io::Error::other(
            "the kernel's credentials are of another version",
        ));
    }
    Ok(credentials.cr_uid)
}

/// Asks the daemon listening at `path`, waiting at most `timeout` for each
/// part of the exchange. A reply of an error is an error.
pub fn ask(path: &Path, request: &Request, timeout: Duration) -> io::Result<Reply> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let mut line = serde_json::to_vec(request).map_err(io::Error::other)?;
    line.push(b'\n');
    stream.write_all(&line)?;
    let mut reply = Vec::new();
    (&mut stream).take(ANSWER_LIMIT).read_to_end(&mut reply)?;
    match serde_json::from_slice(&reply) {
        Ok(Reply::Error(why)) => Err(io::Error::other(why)),
        Ok(reply) => Ok(reply),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}
