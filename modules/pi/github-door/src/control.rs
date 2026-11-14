//! The control socket, where the harness opens and closes sessions and
//! Joel lifts a session's cap. The fence never lets it in.
//!
//! One JSON object a line each way. An ask names its `ask` and the
//! `session`; the answer says `ok`, and for an open, where the session's
//! socket and bundle are:
//!
//! ```text
//! {"ask": "open", "session": "01a1..."}
//! {"ok": true, "folder": ".../01a1...", "socket": ".../door.sock", "bundle": ".../bundle.pem"}
//! ```
//!
//! An open is idempotent, so the harness asks before each fenced command,
//! and a door that restarted makes the session anew, with a new authority,
//! the moment it's next asked.

use std::collections::HashMap;
use std::fs::{self, DirBuilder, Permissions};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{Value, json};

use crate::door::{self, Door, Session};
use crate::peer::plain_session;

/// The most sessions held at once, far past the sessions a machine runs.
const MOST_SESSIONS: usize = 256;
/// The most a control line may be.
const MOST_LINE: u64 = 4096;
/// A session's socket, in its folder.
pub const SOCKET: &str = "door.sock";
/// A session's bundle, in its folder: the system's roots and its own.
pub const BUNDLE: &str = "bundle.pem";

/// Where an opened session is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub folder: PathBuf,
    pub socket: PathBuf,
    pub bundle: PathBuf,
}

/// A session held, and its folder.
struct Held {
    session: Arc<Session>,
    opened: Opened,
}

/// Every session the door holds.
pub struct Sessions {
    door: Arc<Door>,
    root: PathBuf,
    system_bundle: PathBuf,
    held: Mutex<HashMap<String, Held>>,
}

impl Sessions {
    /// No sessions yet, each to be made a folder under `root`, which is
    /// emptied now since no session outlives the door that made it, its
    /// bundle the system's roots from `system_bundle` and its own.
    pub fn new(door: Arc<Door>, root: &Path, system_bundle: &Path) -> io::Result<Self> {
        match fs::remove_dir_all(root) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        DirBuilder::new().recursive(true).mode(0o700).create(root)?;
        Ok(Sessions {
            door,
            root: root.to_owned(),
            system_bundle: system_bundle.to_owned(),
            held: Mutex::new(HashMap::new()),
        })
    }

    fn held(&self) -> std::sync::MutexGuard<'_, HashMap<String, Held>> {
        self.held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The session `id`, opened now if it isn't, or its socket is gone.
    pub fn open(&self, id: &str) -> Result<Opened, String> {
        if !plain_session(id) {
            return Err("a session id is letters, digits, - and _, at most 64".to_owned());
        }
        let mut held = self.held();
        if let Some(existing) = held.get(id) {
            if existing.opened.socket.exists() {
                return Ok(existing.opened.clone());
            }
            let gone = held.remove(id);
            if let Some(gone) = gone {
                end(&gone);
            }
        }
        if held.len() >= MOST_SESSIONS {
            return Err(format!("{MOST_SESSIONS} sessions are open already"));
        }
        let session = Arc::new(Session::new(id)?);
        let folder = self.root.join(id);
        let opened = Opened {
            socket: folder.join(SOCKET),
            bundle: folder.join(BUNDLE),
            folder,
        };
        let listener = make_folder(&opened, &self.system_bundle, session.authority.pem())
            .map_err(|error| format!("the session's folder couldn't be made: {error}"))?;
        let door = Arc::clone(&self.door);
        let serving = Arc::clone(&session);
        thread::spawn(move || door::serve(&door, &serving, &listener));
        held.insert(
            id.to_owned(),
            Held {
                session,
                opened: opened.clone(),
            },
        );
        Ok(opened)
    }

    /// Close the session `id`, its folder taken away.
    pub fn close(&self, id: &str) -> Result<(), String> {
        let gone = self.held().remove(id).ok_or("no session goes by that id")?;
        end(&gone);
        Ok(())
    }

    /// Lift the session `id`'s cap for the rest of its hour.
    pub fn lift(&self, id: &str) -> Result<(), String> {
        let held = self.held();
        let session = &held.get(id).ok_or("no session goes by that id")?.session;
        session.lift();
        eprintln!(
            "pi-github-door: lifted session={} -- its cap is lifted for the rest of its hour",
            crate::said::plain(id)
        );
        Ok(())
    }

    /// The answer to one control line.
    pub fn answer(&self, line: &str) -> Value {
        let asked: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => return refused("an ask is one JSON object a line"),
        };
        let Some(session) = asked.get("session").and_then(Value::as_str) else {
            return refused("an ask names its session");
        };
        let done = match asked.get("ask").and_then(Value::as_str) {
            Some("open") => {
                return match self.open(session) {
                    Ok(opened) => json!({
                        "ok": true,
                        "folder": opened.folder,
                        "socket": opened.socket,
                        "bundle": opened.bundle,
                    }),
                    Err(why) => refused(&why),
                };
            }
            Some("close") => self.close(session),
            Some("lift") => self.lift(session),
            _ => Err("an ask is open, close or lift".to_owned()),
        };
        match done {
            Ok(()) => json!({ "ok": true }),
            Err(why) => refused(&why),
        }
    }

    /// Answer the control socket's clients until the door stops.
    pub fn serve(self: &Arc<Self>, listener: &UnixListener) {
        for client in listener.incoming() {
            match client {
                Ok(stream) => {
                    let sessions = Arc::clone(self);
                    thread::spawn(move || sessions.converse(stream));
                }
                Err(error) => {
                    eprintln!("pi-github-door: a control client couldn't be taken: {error}");
                }
            }
        }
    }

    /// Answer each line one control client sends until it goes.
    fn converse(&self, stream: UnixStream) {
        let Ok(mut out) = stream.try_clone() else {
            return;
        };
        let mut reader = BufReader::new(io::Read::take(stream, u64::MAX));
        loop {
            let mut line = String::new();
            reader.get_mut().set_limit(MOST_LINE);
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let answer = self.answer(line.trim());
            if writeln!(out, "{answer}").is_err() {
                return;
            }
        }
    }
}

/// The answer for an ask refused.
fn refused(why: &str) -> Value {
    json!({ "ok": false, "error": why })
}

/// The session's folder, readable by the account alone, with its bundle
/// and its socket, which is answered by the listener this gives.
fn make_folder(opened: &Opened, system_bundle: &Path, own: &str) -> io::Result<UnixListener> {
    match fs::remove_dir_all(&opened.folder) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    DirBuilder::new().mode(0o700).create(&opened.folder)?;
    let mut bundle = fs::read(system_bundle)?;
    if !bundle.ends_with(b"\n") {
        bundle.push(b'\n');
    }
    bundle.extend_from_slice(own.as_bytes());
    fs::write(&opened.bundle, bundle)?;
    let listener = UnixListener::bind(&opened.socket)?;
    fs::set_permissions(&opened.socket, Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// End a session: its loop told to stop and woken, and its folder gone.
fn end(held: &Held) {
    held.session.closed.store(true, Ordering::SeqCst);
    // A connection wakes the loop to see it's closed; failing, the socket
    // is already gone and so is its loop's reason to wait.
    let _ = UnixStream::connect(&held.opened.socket);
    let _ = fs::remove_dir_all(&held.opened.folder);
}
