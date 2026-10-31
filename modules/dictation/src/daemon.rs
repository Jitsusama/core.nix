//! Version 1 of the shell's daemons' interface (RSCH-20261029-0E8050,
//! "Daemons: One Interface"), as the people daemon answers it, for each
//! daemon here: JSON lines on a socket in a folder only Joel's account
//! reaches, each ask with an `id` echoed in its answers, each client on a
//! thread of its own. What's asked is the daemon's own (`Answers`); the
//! socket, the clients, the watches they hold and the error answers are
//! kept here, alike for every daemon.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// The interface's version this speaks.
pub const VERSION: u64 = 1;

/// The milliseconds since the epoch, as the interface gives times.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Writes `text` to `file` whole: into a file beside it, then renamed over
/// it, so nothing ever reads half of it.
///
/// # Errors
///
/// When either file can't be written.
pub fn write_whole(file: &Path, text: &str) -> std::io::Result<()> {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let part = file.with_file_name(format!(".{name}.part"));
    std::fs::write(&part, text)?;
    std::fs::rename(&part, file)
}

/// An error answer, by the interface's codes.
pub fn error(id: &Value, code: &str, message: &str) -> Value {
    json!({"id": id, "error": {"code": code, "message": message}})
}

/// What a daemon answers: one line from a client at a time, on the
/// connection it came from, with where to tell that client of changes.
pub trait Answers: Send + 'static {
    fn answer(&mut self, line: &str, connection: u64, told: &Sender<String>) -> Value;

    /// The connection went, and every watch it held with it.
    fn gone(&mut self, connection: u64);
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    /// The keys watched, or none for every record.
    keys: Option<Vec<String>>,
    told: Sender<String>,
}

/// The watches every client holds.
#[derive(Default)]
pub struct Watches(Vec<Watch>);

impl Watches {
    /// Holds a watch, in place of one the connection held by its id.
    pub fn watch(
        &mut self,
        connection: u64,
        id: &Value,
        keys: Option<Vec<String>>,
        told: &Sender<String>,
    ) {
        self.unwatch(connection, id);
        self.0.push(Watch {
            connection,
            id: id.clone(),
            keys,
            told: told.clone(),
        });
    }

    pub fn unwatch(&mut self, connection: u64, id: &Value) {
        self.0
            .retain(|watch| !(watch.connection == connection && watch.id == *id));
    }

    pub fn gone(&mut self, connection: u64) {
        self.0.retain(|watch| watch.connection != connection);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Tells each watch of `key` that its record changed, `record` being
    /// its record now, or null once it's gone; a watch whose client went
    /// is let go.
    pub fn tell(&mut self, key: &str, record: &Value) {
        self.0.retain(|watch| {
            let watched = watch
                .keys
                .as_ref()
                .is_none_or(|keys| keys.iter().any(|held| held == key));
            let line = || json!({"id": watch.id, "changed": [record]}).to_string();
            !watched || watch.told.send(line()).is_ok()
        });
    }
}

/// The keys an ask names, if it names any.
pub fn keys(said: &Value) -> Option<Vec<String>> {
    said["keys"].as_array().map(|keys| {
        keys.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    })
}

/// The most records an ask wants, every one when it says none.
pub fn limit(said: &Value) -> usize {
    said["limit"]
        .as_u64()
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(usize::MAX)
}

/// A daemon's state, shared between its threads.
pub struct Shared<T>(Mutex<T>);

impl<T> Shared<T> {
    pub fn new(held: T) -> Self {
        Self(Mutex::new(held))
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The socket, in a folder only this account reaches.
///
/// # Errors
///
/// When the folder can't be made or the socket bound.
pub fn listen(socket: &Path) -> Result<UnixListener, String> {
    let made = || -> std::io::Result<UnixListener> {
        if let Some(folder) = socket.parent() {
            std::fs::create_dir_all(folder)?;
            std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
        }
        let _ = std::fs::remove_file(socket);
        UnixListener::bind(socket)
    };
    made().map_err(|why| format!("listening on {}: {why}", socket.display()))
}

/// Answers every client, each on a thread of its own.
pub fn answer_all<T: Answers>(daemon: &Arc<Shared<T>>, listener: &UnixListener) {
    for (connection, stream) in (0..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let daemon = Arc::clone(daemon);
        std::thread::spawn(move || serve(&daemon, stream, connection));
    }
}

/// Answers one client until it goes.
fn serve<T: Answers>(daemon: &Shared<T>, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let (told, lines) = mpsc::channel::<String>();
    let mut writing = stream;
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(writing, "{line}").is_err() {
                break;
            }
        }
    });
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { break };
        let answer = daemon.lock().answer(&line, connection, &told);
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    daemon.lock().gone(connection);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_watch_hears_only_the_keys_it_names_and_goes_with_its_client() {
        let mut watches = Watches::default();
        let (told, heard) = mpsc::channel();
        watches.watch(1, &json!(7), Some(vec!["a".to_owned()]), &told);
        watches.tell("b", &json!({"key": "b"}));
        watches.tell("a", &json!({"key": "a"}));
        let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(line, json!({"id": 7, "changed": [{"key": "a"}]}));
        assert!(heard.try_recv().is_err());
        watches.gone(1);
        assert!(watches.is_empty());
    }

    #[test]
    fn a_watch_asked_again_by_its_id_replaces_the_one_before() {
        let mut watches = Watches::default();
        let (told, _heard) = mpsc::channel();
        watches.watch(1, &json!(7), None, &told);
        watches.watch(1, &json!(7), None, &told);
        watches.watch(2, &json!(7), None, &told);
        assert_eq!(watches.len(), 2);
    }
}
