//! The system daemon's socket: one JSON request a connection, answered
//! with one JSON line. The kernel says who asks, each account may ask once
//! a second, and a request is cut off past 64 KiB or a second of silence,
//! so no asker can hold the daemon up.

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Request {
    /// The processes touching the asker's files beneath these roots.
    Processes { roots: Vec<PathBuf> },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
enum Reply {
    Answer(Answer),
    Error(String),
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
    asked: HashMap<u32, Instant>,
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

    /// Takes one connection and answers it.
    pub fn serve(
        &mut self,
        handle: &mut dyn FnMut(u32, Request) -> Result<Answer, String>,
    ) -> io::Result<()> {
        let (mut stream, _) = self.listener.accept()?;
        let reply = match self.read(&stream) {
            Ok((uid, request)) => match handle(uid, request) {
                Ok(answer) => Reply::Answer(answer),
                Err(why) => Reply::Error(why),
            },
            Err(why) => Reply::Error(why),
        };
        let mut line = serde_json::to_vec(&reply).unwrap_or_else(|error| {
            serde_json::to_vec(&Reply::Error(format!("the answer won't go: {error}")))
                .unwrap_or_default()
        });
        line.push(b'\n');
        // An asker that went away hears nothing, which is its own concern.
        let _ = stream.write_all(&line);
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
        let now = Instant::now();
        if self
            .asked
            .get(&uid)
            .is_some_and(|at| now.duration_since(*at) < self.every)
        {
            return Err("too soon: ask at most once a second".to_owned());
        }
        let request = serde_json::from_slice(&line)
            .map_err(|error| format!("the request won't read: {error}"))?;
        self.asked.insert(uid, now);
        Ok((uid, request))
    }
}

/// The account on the other end of a socket, as the kernel says.
fn peer(stream: &UnixStream) -> io::Result<u32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut size = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: credentials is a ucred of the size given, which the kernel
    // fills in for SO_PEERCRED.
    let done = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &mut size,
        )
    };
    if done != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(credentials.uid)
}

/// Asks the daemon listening at `path`, waiting at most `timeout` for each
/// part of the exchange.
pub fn ask(path: &Path, request: &Request, timeout: Duration) -> io::Result<Answer> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let mut line = serde_json::to_vec(request).map_err(io::Error::other)?;
    line.push(b'\n');
    stream.write_all(&line)?;
    let mut reply = Vec::new();
    (&mut stream).take(ANSWER_LIMIT).read_to_end(&mut reply)?;
    match serde_json::from_slice(&reply) {
        Ok(Reply::Answer(answer)) => Ok(answer),
        Ok(Reply::Error(why)) => Err(io::Error::other(why)),
        Err(error) => Err(io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}
