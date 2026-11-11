//! Records held for the shell and answered over the socket systemd holds
//! for a daemon, lifted from the to-do daemon's serve.rs. The daemon's
//! loop hands it every record whole each time something changes, and
//! each watch is told only the records that changed and the keys that
//! went, in one line. Nothing is read for an ask: everything said is
//! already held.
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
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::asks::{Ask, Domain, VERSION, error, place_of};

/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken, as when the
/// process is out of descriptors, so the loop doesn't spin a core.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);

/// What answers a daemon's own asks: the ask whole, and its answer
/// without its id, or none for an ask it doesn't know.
pub type Own = Box<dyn Fn(&Value) -> Option<Value> + Send + Sync>;

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    places: Option<Vec<String>>,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, domain: &Domain, key: &str) -> bool {
        if let Some(keys) = &self.keys {
            return keys.iter().any(|one| one == key);
        }
        let kind = domain
            .kind_of(key)
            .is_some_and(|kind| self.kinds.iter().any(|k| k == kind));
        let place = self.places.as_ref().is_none_or(|places| {
            place_of(key).is_some_and(|place| places.iter().any(|one| one == place))
        });
        kind && place
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
    domain: Domain,
    own: Own,
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

impl Shared {
    /// What serves `domain`, its own asks answered by `own`.
    pub fn new(domain: Domain, own: Own) -> Arc<Self> {
        Arc::new(Shared {
            domain,
            own,
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
            let wanted = |key: &str| watch.wants(&self.domain, key);
            let records: Vec<&&Value> = changed
                .iter()
                .filter(|record| record["key"].as_str().is_some_and(wanted))
                .collect();
            let went: Vec<&String> = gone.iter().filter(|key| wanted(key)).collect();
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

/// The socket systemd holds for the daemon, so the shell never finds it
/// missing while the daemon starts or starts again; or, run by hand, its
/// own, in a folder only Joel reads, replacing one left behind.
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
        let asked = shared.domain.ask(&line);
        // The daemon's own asks are answered before the lock is taken, so
        // one handing its ask to the daemon's loop never waits on a change.
        let own = match &asked {
            Ask::Own { id, said } => Some(owned(shared, id, said)),
            _ => None,
        };
        let mut held = lock(shared);
        let answer =
            own.unwrap_or_else(|| answer(shared, &mut held, asked, connection, &told, &hangup));
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

/// The answer to one of the daemon's own asks, or a refusal of an ask
/// it doesn't know.
fn owned(shared: &Shared, id: &Value, said: &Value) -> Value {
    let Some(mut answer) = (shared.own)(said) else {
        let ask = said["ask"].as_str().unwrap_or("");
        return error(id, "unknown-ask", &format!("no ask {ask:?} here"));
    };
    answer["id"] = id.clone();
    answer
}

/// The records held of these kinds, or these keys, in the daemon's
/// order: by kind as the domain lists them, then by key; keys as asked.
fn records_of(
    domain: &Domain,
    held: &Held,
    kinds: &[String],
    keys: Option<&[String]>,
) -> Vec<Value> {
    if let Some(keys) = keys {
        return keys
            .iter()
            .filter_map(|key| held.records.get(key))
            .cloned()
            .collect();
    }
    domain
        .kinds
        .iter()
        .filter(|kind| kinds.iter().any(|k| k == *kind))
        .flat_map(|kind| {
            held.records
                .iter()
                .filter(move |(key, _)| domain.kind_of(key) == Some(kind))
                .map(|(_, record)| record.clone())
        })
        .collect()
}

/// What one ask is answered with.
fn answer(
    shared: &Shared,
    held: &mut Held,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Value {
    let domain = &shared.domain;
    match asked {
        Ask::Hello { id } => json!({
            "id": id, "version": VERSION, "domain": domain.name, "state": "answering",
            "since": held.since, "sources": held.sources,
        }),
        Ask::Get { id, keys } => {
            json!({"id": id, "records": records_of(domain, held, &[], Some(&keys))})
        }
        Ask::List { id, kinds } => {
            json!({"id": id, "records": records_of(domain, held, &kinds, None)})
        }
        Ask::Watch {
            id,
            kinds,
            keys,
            places,
        } => {
            // A watch asked again by its id replaces the one before.
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            let watch = Watch {
                connection,
                id: id.clone(),
                kinds,
                keys,
                places,
                told: told.clone(),
                stream: Arc::clone(hangup),
            };
            let records: Vec<&Value> = held
                .records
                .iter()
                .filter(|(key, _)| watch.wants(domain, key))
                .map(|(_, record)| record)
                .collect();
            let answer = json!({"id": id, "records": records});
            held.watches.push(watch);
            answer
        }
        Ask::Unwatch { id } => {
            held.watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => json!({
            "id": id, "version": VERSION, "domain": domain.name, "state": "answering",
            "since": held.since, "sources": held.sources,
            "watches": held.watches.len(), "records": held.records.len(),
        }),
        Ask::Own { id, said } => owned(shared, &id, &said),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc::channel;

    use daemon_client::Client;

    use super::*;

    const ROWS: Domain = Domain {
        name: "agents",
        kinds: &["row", "summary"],
    };

    /// A folder of the test's own, gone when it ends.
    struct Folder(PathBuf);

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn served(name: &str, own: Own) -> (Arc<Shared>, Client, Folder) {
        let folder =
            std::env::temp_dir().join(format!("daemon-serve-{name}-{}", std::process::id()));
        let socket = folder.join("daemon.sock");
        let shared = Shared::new(ROWS, own);
        accept(
            Arc::clone(&shared),
            listen(&socket).expect("the socket is bound"),
        );
        (shared, Client::connect(&socket), Folder(folder))
    }

    fn row(place: &str, id: &str, name: &str) -> Value {
        json!({"key": format!("row:{place}:{id}"), "kind": "row", "row": {"name": name}})
    }

    #[test]
    fn a_watch_is_answered_whole_then_told_only_what_moved() {
        let (shared, mut client, _folder) = served("moved", Box::new(|_| None));
        shared.change(
            vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
            json!({}),
        );
        let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row"}));
        assert_eq!(
            answer["records"].as_array().map(Vec::len),
            Some(2),
            "both rows, whole"
        );
        shared.change(vec![row("agents", "O", "harbour")], json!({}));
        let told = client.heard();
        assert_eq!(
            told["changed"],
            json!([row("agents", "O", "harbour")]),
            "the row that moved"
        );
        assert_eq!(
            told["gone"],
            json!(["row:agents:P"]),
            "and the key that went"
        );
    }

    #[test]
    fn a_watch_of_a_place_hears_only_its_rows() {
        let (shared, mut client, _folder) = served("place", Box::new(|_| None));
        shared.change(
            vec![row("agents", "O", "kelp"), row("mail", "m", "hi")],
            json!({}),
        );
        let asked = json!({"id": 1, "ask": "watch", "kind": "row", "place": "agents"});
        let answer = client.ask(&asked);
        assert_eq!(
            answer["records"],
            json!([row("agents", "O", "kelp")]),
            "only Agents' row"
        );
        shared.change(vec![row("agents", "O", "reef")], json!({}));
        let told = client.heard();
        assert_eq!(
            told["changed"],
            json!([row("agents", "O", "reef")]),
            "only Agents' row"
        );
        assert!(
            told.get("gone").is_none(),
            "Mail's row going is another place's: {told}"
        );
    }

    #[test]
    fn the_daemons_own_asks_are_its_to_answer() {
        let (telling, told) = channel();
        let own: Own = Box::new(move |said| {
            (said["ask"] == "sharing").then(|| {
                let _ = telling.send(said["sharing"].clone());
                json!({"sharing": said["sharing"]})
            })
        });
        let (_shared, mut client, _folder) = served("own", own);
        let answer = client.ask(&json!({"id": 1, "ask": "sharing", "sharing": true}));
        assert_eq!(
            answer,
            json!({"id": 1, "sharing": true}),
            "answered by the daemon"
        );
        assert_eq!(told.recv().ok(), Some(json!(true)), "which heard it");
        let refused = client.ask(&json!({"id": 2, "ask": "dance"}));
        assert_eq!(
            refused["error"]["code"], "unknown-ask",
            "one it doesn't know is refused"
        );
        let hello = client.ask(&json!({"id": 3, "ask": "hello"}));
        assert_eq!(
            (hello["domain"].as_str(), hello["version"].as_u64()),
            (Some("agents"), Some(1))
        );
    }
}
