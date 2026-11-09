//! Where the laptop is, held for the shell and clock-follows, and answered
//! over the socket systemd holds for it, in version 1 of the shell's daemon
//! interface (interface.rs). One record, `here:laptop`, says it whole: the
//! position GeoClue last found or Joel set, what it's called and the zone
//! it keeps, and beside it the zone the clock keeps and its last move.
//! Each change is told to every watch at once, in one line, and nothing is
//! read for an ask: everything said is already held.
//!
//! Each connection is served on a thread of its own, its lines written by
//! another, so a client that stops reading holds no other back; one with
//! more than a few hundred lines unread is hung up on.

use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::fix::Fix;
use crate::interface::{Ask, HERE, VERSION, ask, error, kind_of};

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// What's known, from which every answer is made.
#[derive(Debug, Default)]
pub struct Known {
    /// Where the laptop is, or none while it isn't known.
    pub here: Option<Fix>,
    /// The zone the clock keeps, or none where it can't be read.
    pub clock_zone: Option<String>,
    /// The clock's last move, as clock.rs reads it.
    pub moved: Option<Value>,
    /// How GeoClue stands, in the interface's words, or none on a machine
    /// that doesn't follow the laptop and never asks it.
    pub geoclue: Option<Value>,
    /// How the last asking of a place's name went, likewise.
    pub names: Option<Value>,
}

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    keys: Option<Vec<String>>,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some(),
        }
    }
}

struct Held {
    known: Known,
    /// The record as every watch was last told it.
    said: Value,
    watches: Vec<Watch>,
    /// When it started, in milliseconds since the epoch.
    since: u64,
}

/// What every connection shares.
pub struct Shared {
    held: Mutex<Held>,
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

/// The one record, made from what's known. Each figure GeoClue doesn't
/// know, as Wi-Fi never knows the altitude, is null.
pub fn record(known: &Known) -> Value {
    let here = known.here.as_ref();
    json!({
        "key": HERE,
        "kind": "here",
        "latitude": here.map(|fix| fix.latitude),
        "longitude": here.map(|fix| fix.longitude),
        "accuracy_m": here.map(|fix| fix.accuracy),
        "at_s": here.map(|fix| fix.when),
        "altitude_m": here.and_then(|fix| fix.altitude),
        "speed_m_s": here.and_then(|fix| fix.speed),
        "heading_deg": here.and_then(|fix| fix.heading),
        "place": here.and_then(|fix| fix.place.as_deref()),
        "country": here.and_then(|fix| fix.country.as_deref()),
        "zone": here.and_then(|fix| fix.zone.as_deref()),
        "hand": here.map(|fix| fix.hand),
        "clock_zone": known.clock_zone,
        "moved": known.moved,
    })
}

impl Shared {
    pub fn new(known: Known) -> Arc<Shared> {
        let said = record(&known);
        Arc::new(Shared {
            held: Mutex::new(Held {
                known,
                said,
                watches: Vec::new(),
                since: now_ms(),
            }),
        })
    }

    /// Changes what's known, and tells every watch the record if it
    /// changed. A connection whose lines aren't being read is hung up on,
    /// and its watches dropped, rather than told less than it asked for.
    pub fn change(&self, how: impl FnOnce(&mut Known)) {
        let mut held = lock(self);
        how(&mut held.known);
        let now = record(&held.known);
        if now == held.said {
            return;
        }
        held.said = now;
        let line = |id: &Value| json!({"id": id, "changed": [&held.said]}).to_string();
        let mut stuck = Vec::new();
        for watch in held.watches.iter().filter(|watch| watch.wants(HERE)) {
            if watch.told.try_send(line(&watch.id)).is_err() {
                stuck.push(watch.connection);
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        held.watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

/// The socket systemd holds for it (location.socket), so the shell never
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

/// Takes each connection, for as long as the program runs.
pub fn accept(shared: &Arc<Shared>, listener: &UnixListener) {
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
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
        let answer = answer(&mut held, ask(&line), connection, &told, &hangup);
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

/// Each of its sources by name, with its word for how it stands.
fn sources(known: &Known) -> Value {
    let mut sources = serde_json::Map::new();
    sources.insert("clock".into(), json!({"state": "answering"}));
    if let Some(geoclue) = &known.geoclue {
        sources.insert("geoclue".into(), geoclue.clone());
    }
    if let Some(names) = &known.names {
        sources.insert("names".into(), names.clone());
    }
    Value::Object(sources)
}

/// What one ask is answered with.
fn answer(
    held: &mut Held,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Value {
    let whole = |held: &Held| vec![held.said.clone()];
    match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": "location", "state": "answering",
            "since": held.since, "sources": sources(&held.known),
        }),
        Ask::Get { id, keys } => {
            let records: Vec<Value> = if keys.iter().any(|key| key == HERE) {
                whole(held)
            } else {
                Vec::new()
            };
            json!({"id": id, "records": records})
        }
        Ask::List { id } => json!({"id": id, "records": whole(held)}),
        Ask::Watch { id, keys } => {
            // A watch asked again by its id replaces the one before.
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            let watch = Watch {
                connection,
                id: id.clone(),
                keys,
                told: told.clone(),
                stream: Arc::clone(hangup),
            };
            let records = if watch.wants(HERE) {
                whole(held)
            } else {
                Vec::new()
            };
            held.watches.push(watch);
            json!({"id": id, "records": records})
        }
        Ask::Unwatch { id } => {
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => json!({
            "id": id, "version": VERSION, "domain": "location", "state": "answering",
            "since": held.since, "sources": sources(&held.known),
            "watches": held.watches.len(),
        }),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nowhere_is_null_and_the_clock_still_said() {
        let known = Known {
            clock_zone: Some("America/Halifax".into()),
            ..Known::default()
        };
        let said = record(&known);
        assert_eq!(said["key"], HERE);
        assert_eq!(said["kind"], "here");
        assert_eq!(said["latitude"], Value::Null);
        assert_eq!(said["hand"], Value::Null);
        assert_eq!(said["clock_zone"], "America/Halifax");
        assert_eq!(said["moved"], Value::Null);
    }

    #[test]
    fn a_fix_is_said_in_its_units() {
        let fix = Fix::new(12.3457, -98.7654, 26_000.0, 1_790_000_000)
            .unwrap()
            .named(
                Some("Kelpwater, Westmark".into()),
                Some("America/Toronto".into()),
            )
            .countried(Some("Canada".into()));
        let said = record(&Known {
            here: Some(fix),
            ..Known::default()
        });
        assert_eq!(said["latitude"], 12.3457);
        assert_eq!(said["accuracy_m"], 26_000.0);
        assert_eq!(said["at_s"], 1_790_000_000_u64);
        assert_eq!(said["place"], "Kelpwater, Westmark");
        assert_eq!(said["country"], "Canada");
        assert_eq!(said["zone"], "America/Toronto");
        assert_eq!(said["hand"], false);
        assert!(said["altitude_m"].is_null(), "Wi-Fi finds no altitude");
    }

    #[test]
    fn a_change_that_changes_nothing_tells_nobody() {
        let shared = Shared::new(Known::default());
        let (told, lines) = mpsc::sync_channel(QUEUED);
        let (one, _two) = UnixStream::pair().unwrap();
        lock(&shared).watches.push(Watch {
            connection: 1,
            id: json!("all"),
            keys: None,
            told,
            stream: Arc::new(one),
        });
        shared.change(|known| known.clock_zone = None);
        assert!(
            lines.try_recv().is_err(),
            "a record unchanged is told to no watch"
        );
        shared.change(|known| known.clock_zone = Some("UTC".into()));
        let line: Value = serde_json::from_str(&lines.try_recv().unwrap()).unwrap();
        assert_eq!(line["id"], "all");
        assert_eq!(line["changed"][0]["clock_zone"], "UTC");
    }
}
