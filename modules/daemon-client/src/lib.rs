//! A daemon of the launcher's daemon interface, started for a test, and a
//! client asking it over its socket, one line of JSON each way, for each
//! daemon's tests (whereabouts's, in modules/quickshell/whereabouts/tests),
//! which name this crate in their Cargo.toml as a dev-dependency. The
//! helpers were copied from one daemon's tests into the next; kept here,
//! a fix to one reaches every test that asks a daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use serde_json::Value;

/// How long a test waits for the daemon's socket, or for an answer.
const PATIENCE: Duration = Duration::from_secs(10);

/// A daemon serving its socket, stopped and its folder taken away once
/// it's dropped.
pub struct Daemon {
    child: Child,
    socket: PathBuf,
    folder: PathBuf,
}

impl Daemon {
    /// Starts `command`, which serves `socket`, once the socket's there;
    /// `folder` holds what the test made for it.
    pub fn start(command: &mut Command, socket: PathBuf, folder: PathBuf) -> Daemon {
        let child = command.spawn().expect("the daemon starts");
        let daemon = Daemon {
            child,
            socket,
            folder,
        };
        until("the socket never came", || daemon.socket.exists());
        daemon
    }

    /// The folder the test made for the daemon.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// A client of its own, asking over the socket.
    pub fn client(&self) -> Client {
        let stream = UnixStream::connect(&self.socket).expect("the socket answers");
        stream
            .set_read_timeout(Some(PATIENCE))
            .expect("the socket takes a timeout");
        Client {
            lines: BufReader::new(stream.try_clone().expect("the socket is cloned")),
            stream,
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

/// One client of a daemon, which asks and hears what a watch pushes.
pub struct Client {
    stream: UnixStream,
    lines: BufReader<UnixStream>,
}

impl Client {
    /// The answer to an ask, passing over what a watch pushed meanwhile.
    pub fn ask(&mut self, asked: &Value) -> Value {
        writeln!(self.stream, "{asked}").expect("the ask is sent");
        loop {
            let said = self.heard();
            if said["id"] == asked["id"] && said.get("changed").is_none() {
                return said;
            }
        }
    }

    /// The next line the daemon says, whatever it is.
    pub fn heard(&mut self) -> Value {
        let mut line = String::new();
        self.lines
            .read_line(&mut line)
            .expect("the daemon answers in time");
        serde_json::from_str(&line).expect("the daemon says JSON")
    }
}

/// Waits until `done`, failing with `why` past the test's patience.
pub fn until(why: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + PATIENCE;
    while !done() {
        assert!(Instant::now() < deadline, "{why}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
