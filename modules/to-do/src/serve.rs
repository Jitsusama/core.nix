//! What waits on Joel, held for the shell and answered over the socket
//! systemd holds for it, in version 1 of the shell's daemon interface
//! (interface.rs). The daemon's loop hands it every record whole each
//! time something changes (stream.rs makes them), and each watch is told
//! only the records that changed and the keys that went, in one line.
//! Nothing is read for an ask: everything said is already held. Joel's
//! word on a to-do is an ask too, kept in settled.json (settle.rs), which
//! the daemon's watch on it then tells every watch as it tells any write.
//!
//! Each connection is served on a thread of its own, its lines written by
//! another, so a client that stops reading holds no other back; one with
//! more than a few hundred lines unread is hung up on.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::interface::{Ask, KINDS, VERSION, ask, error, kind_of};
use crate::settle::settle;
use crate::stream::said;

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }
}

struct Held {
    /// Every record, by key.
    records: BTreeMap<String, Value>,
    /// How each source stands, in the interface's words.
    sources: Value,
    watches: Vec<Watch>,
    /// When it started, in milliseconds since the epoch.
    since: u64,
}

/// What every connection shares with the daemon's loop.
pub struct Shared {
    held: Mutex<Held>,
    /// Where Joel's word is kept, if anywhere.
    settled: Option<PathBuf>,
}

fn lock(shared: &Shared) -> MutexGuard<'_, Held> {
    shared
        .held
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

impl Shared {
    pub fn new(settled: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Shared {
            settled,
            held: Mutex::new(Held {
                records: BTreeMap::new(),
                sources: json!({}),
                watches: Vec::new(),
                since: now_ms(),
            }),
        })
    }

    /// Holds `records` as every record there is, and tells each watch
    /// those that changed and the keys that went, and, on a line of its
    /// own as the shell's Daemon.qml reads it, how the sources stand once
    /// that changes. A connection whose lines aren't being read is hung up
    /// on, and its watches dropped, rather than told less than it asked
    /// for.
    pub fn change(&self, records: Vec<Value>, sources: Value) {
        let mut held = lock(self);
        let standing = (held.sources != sources).then(|| sources.clone());
        held.sources = sources;
        let mut now = BTreeMap::new();
        for record in records {
            if let Some(key) = record["key"].as_str() {
                now.insert(key.to_owned(), record);
            }
        }
        let gone: Vec<String> = held
            .records
            .keys()
            .filter(|key| !now.contains_key(*key))
            .cloned()
            .collect();
        let changed: Vec<&Value> = now
            .iter()
            .filter(|(key, record)| held.records.get(*key) != Some(record))
            .map(|(_, record)| record)
            .collect();
        if changed.is_empty() && gone.is_empty() && standing.is_none() {
            return;
        }
        let mut stuck = Vec::new();
        for watch in &held.watches {
            let records: Vec<&&Value> = changed
                .iter()
                .filter(|record| record["key"].as_str().is_some_and(|key| watch.wants(key)))
                .collect();
            let went: Vec<&String> = gone.iter().filter(|key| watch.wants(key)).collect();
            // The records first, so a watch told how the sources stand
            // already holds what the same reading changed.
            let mut lines = Vec::new();
            if !records.is_empty() || !went.is_empty() {
                let mut told = json!({"id": watch.id, "changed": records});
                if !went.is_empty() {
                    told["gone"] = json!(went);
                }
                lines.push(told);
            }
            if let Some(sources) = &standing {
                lines.push(json!({"id": watch.id, "sources": sources}));
            }
            if lines
                .iter()
                .any(|line| watch.told.try_send(line.to_string()).is_err())
            {
                stuck.push(watch.connection);
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        held.records = now;
        held.watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

/// The socket systemd holds for it (to-do.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
pub fn listen(socket: &Path) -> std::io::Result<UnixListener> {
    if let Some(listener) = handed() {
        return Ok(listener);
    }
    if let Some(folder) = socket.parent() {
        std::fs::create_dir_all(folder)?;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    let _ = std::fs::remove_file(socket);
    UnixListener::bind(socket)
}

/// The listening socket systemd hands this process, as the first
/// descriptor after the standard three, when it says so (`sd_listen_fds`).
fn handed() -> Option<UnixListener> {
    let pid: u32 = std::env::var("LISTEN_PID").ok()?.parse().ok()?;
    let count: u32 = std::env::var("LISTEN_FDS").ok()?.parse().ok()?;
    if pid != std::process::id() || count < 1 {
        return None;
    }
    // SAFETY: systemd hands descriptor 3 to this process alone, named by
    // LISTEN_PID, and nothing here has opened or taken it before.
    Some(unsafe { UnixListener::from_raw_fd(3) })
}

/// Takes each connection on a thread of its own, for as long as the
/// program runs.
pub fn accept(shared: Arc<Shared>, listener: UnixListener) {
    std::thread::spawn(move || {
        for (connection, stream) in (1u64..).zip(listener.incoming()) {
            let Ok(stream) = stream else {
                std::thread::sleep(ACCEPT_BACKOFF);
                continue;
            };
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || serve(&shared, stream, connection));
        }
    });
}

fn serve(shared: &Shared, stream: UnixStream, connection: u64) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    let Ok(hangup) = stream.try_clone().map(Arc::new) else {
        return;
    };
    let (told, lines) = mpsc::sync_channel::<String>(QUEUED);
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
        let mut held = lock(shared);
        let asked = ask(&line);
        let answer = match asked {
            Ask::Act { id, act, item } => acted(&held, shared.settled.as_deref(), &id, &act, &item),
            asked => answer(&mut held, asked, connection, &told, &hangup),
        };
        // Queued while the lock is held, so a change can't be queued before
        // the answer it follows; one that can't be queued is hung up on, as
        // a change would hang up on it.
        let queued = told.try_send(answer.to_string()).is_ok();
        drop(held);
        if !queued {
            break;
        }
    }
    lock(shared)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// The records held of these kinds, or these keys, in the daemon's
/// order: by kind as `KINDS` lists them, then by key; keys as asked.
fn records_of(held: &Held, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    if let Some(keys) = keys {
        return keys
            .iter()
            .filter_map(|key| held.records.get(key))
            .cloned()
            .collect();
    }
    KINDS
        .iter()
        .filter(|kind| kinds.iter().any(|k| k == *kind))
        .flat_map(|kind| {
            held.records
                .iter()
                .filter(move |(key, _)| kind_of(key) == Some(kind))
                .map(|(_, record)| record.clone())
        })
        .collect()
}

/// What an act on a to-do is answered with, once Joel's word is kept:
/// done, or why not.
fn acted(held: &Held, settled: Option<&Path>, id: &Value, act: &str, item: &str) -> Value {
    let Some(record) = held.records.get(&format!("to-do:{item}")) else {
        return error(id, "unknown-item", &format!("no to-do {item} here"));
    };
    let Some(path) = settled else {
        return error(id, "refused", "nowhere to keep Joel's word");
    };
    match settle(path, act, item, &said(&record["item"]), now_ms()) {
        Ok(()) => json!({"id": id, "done": true}),
        Err(refused) => error(id, refused.code, &refused.message),
    }
}

/// What one ask is answered with.
fn answer(
    held: &mut Held,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Value {
    match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": "to-do", "state": "answering",
            "since": held.since, "sources": held.sources,
        }),
        Ask::Get { id, keys } => json!({"id": id, "records": records_of(held, &[], Some(&keys))}),
        Ask::List { id, kinds } => json!({"id": id, "records": records_of(held, &kinds, None)}),
        Ask::Watch { id, kinds, keys } => {
            let records = records_of(held, &kinds, keys.as_deref());
            // A watch asked again by its id replaces the one before.
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            held.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds,
                keys,
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            json!({"id": id, "records": records})
        }
        Ask::Unwatch { id } => {
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => json!({
            "id": id, "version": VERSION, "domain": "to-do", "state": "answering",
            "since": held.since, "sources": held.sources,
            "watches": held.watches.len(), "records": held.records.len(),
        }),
        // Answered by `acted`, which alone is handed where it's kept.
        Ask::Act { id, .. } => error(&id, "refused", "an act is answered apart"),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    }
}
