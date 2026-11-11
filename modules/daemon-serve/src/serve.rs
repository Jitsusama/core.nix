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
//!
//! A watch may be paced, as the shut launcher paces Agents': what moves
//! is held back by key and told at most once a pace, all of it in one
//! line, so the rows stay held for a fast opening while the shut shell
//! wakes for one line in ten seconds rather than one in three. Asked
//! again under its id for the same records, a watch keeps on at its new
//! pace, telling at once what it held back rather than everything whole,
//! as mycelium's node does (its PROTOCOL.md, The Daemon Interface).
//!
//! A daemon that serves something only while it's watched, as the notices
//! daemon keeps a notice read while For You's Desktop is looked at, is
//! told as each watch begins or ends, and asks which are watched.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// What's told as a watch begins or ends. It's told with every record
/// held, so it may only pass that on, as onto a channel, and never ask
/// anything of what serves it.
pub type Heed = Box<dyn Fn() + Send + Sync>;

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
    /// How long what moves waits to be told, or none to tell it at once.
    every: Option<Duration>,
    /// The keys that moved since it was last told, while it's paced.
    moved: BTreeSet<String>,
    /// When what moved is told, once something has.
    due: Option<Instant>,
}

impl Watch {
    /// Whether it's the watch an ask names: on this connection, under
    /// this id, of the same records.
    fn is(
        &self,
        connection: u64,
        id: &Value,
        records: (&[String], Option<&[String]>, Option<&[String]>),
    ) -> bool {
        let (kinds, keys, places) = records;
        self.connection == connection
            && self.id == *id
            && self.kinds == kinds
            && self.keys.as_deref() == keys
            && self.places.as_deref() == places
    }

    /// What it held back, told now: each key that moved as its record now
    /// reads, or as gone; none when nothing moved.
    fn held_back(&mut self, records: &BTreeMap<String, Value>) -> Option<Value> {
        self.due = None;
        if self.moved.is_empty() {
            return None;
        }
        let keys = std::mem::take(&mut self.moved);
        let mut changed = Vec::new();
        let mut gone = Vec::new();
        for key in &keys {
            match records.get(key) {
                Some(record) => changed.push(record),
                None => gone.push(key.as_str()),
            }
        }
        Some(moved(&self.id, &changed, &gone))
    }

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

impl Held {
    /// Hangs up on these connections, which stopped reading, and drops
    /// their watches, answering whether any went.
    fn hang_up(&mut self, stuck: &[u64]) -> bool {
        for watch in &self.watches {
            if stuck.contains(&watch.connection) {
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        self.drop_watches(|watch| stuck.contains(&watch.connection))
    }

    /// Drops the watches `going` holds of, answering whether any went.
    fn drop_watches(&mut self, going: impl Fn(&Watch) -> bool) -> bool {
        let before = self.watches.len();
        self.watches.retain(|watch| !going(watch));
        self.watches.len() != before
    }
}

/// The line telling a watch what moved: the records that changed, and
/// the keys that went, when any did.
fn moved(id: &Value, changed: &[&Value], gone: &[&str]) -> Value {
    let mut told = json!({"id": id, "changed": changed});
    if !gone.is_empty() {
        told["gone"] = json!(gone);
    }
    told
}

/// What every connection shares with the daemon's loop.
pub struct Shared {
    domain: Domain,
    own: Own,
    held: Mutex<Held>,
    /// Woken as a paced watch first holds something back, so what tells
    /// it sleeps no longer than its pace.
    paced: Condvar,
    /// What's told as a watch begins or ends, once the daemon says.
    heed: OnceLock<Heed>,
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
            paced: Condvar::new(),
            heed: OnceLock::new(),
        })
    }

    /// Tells `heed` each time a watch begins or ends, from then on; only
    /// the first a daemon gives is kept.
    pub fn on_watches(&self, heed: Heed) {
        let _ = self.heed.set(heed);
    }

    /// Whether any watch is of `kind` in `place`, or of every place.
    pub fn watches(&self, kind: &str, place: &str) -> bool {
        lock(self).watches.iter().any(|watch| {
            watch.keys.is_none()
                && watch.kinds.iter().any(|one| one == kind)
                && watch
                    .places
                    .as_ref()
                    .is_none_or(|places| places.iter().any(|one| one == place))
        })
    }

    fn heeded(&self, moved: bool) {
        if let Some(heed) = self.heed.get().filter(|_| moved) {
            heed();
        }
    }

    /// Holds `records` as every record there is, and tells each watch
    /// those that changed and the keys that went, and, on a line of its
    /// own as the shell's Daemon.qml reads it, how the sources stand once
    /// that changes. A paced watch is told how the sources stand at once
    /// too, and holds back the keys that moved until its pace is up. A
    /// connection whose lines aren't being read is hung up on, and its
    /// watches dropped, rather than told less than it asked for.
    pub fn change(&self, records: Vec<Value>, sources: Value) {
        let mut guard = lock(self);
        let held: &mut Held = &mut guard;
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
        let mut pacing = false;
        for watch in &mut held.watches {
            let records: Vec<&Value> = changed
                .iter()
                .copied()
                .filter(|record| {
                    record["key"]
                        .as_str()
                        .is_some_and(|key| watch.wants(&self.domain, key))
                })
                .collect();
            let went: Vec<&str> = gone
                .iter()
                .map(String::as_str)
                .filter(|key| watch.wants(&self.domain, key))
                .collect();
            // The records first, so a watch told how the sources stand
            // already holds what the same reading changed.
            let mut lines = Vec::new();
            if let Some(every) = watch.every {
                let keys = records.iter().filter_map(|record| record["key"].as_str());
                watch
                    .moved
                    .extend(keys.chain(went.iter().copied()).map(str::to_owned));
                if !watch.moved.is_empty() && watch.due.is_none() {
                    watch.due = Some(Instant::now() + every);
                    pacing = true;
                }
            } else if !records.is_empty() || !went.is_empty() {
                lines.push(moved(&watch.id, &records, &went));
            }
            if let Some(sources) = &standing {
                lines.push(json!({"id": watch.id, "sources": sources}));
            }
            if lines
                .iter()
                .any(|line| watch.told.try_send(line.to_string()).is_err())
            {
                stuck.push(watch.connection);
            }
        }
        held.records = now;
        self.heeded(held.hang_up(&stuck));
        if pacing {
            self.paced.notify_all();
        }
    }

    /// Tells each paced watch what it held back once its pace is up, for
    /// as long as the program runs: asleep until the next is due, and for
    /// as long as nothing is held back, so a daemon whose watches are all
    /// told at once keeps no clock.
    fn pace(&self) {
        let mut guard = lock(self);
        loop {
            let held: &mut Held = &mut guard;
            let now = Instant::now();
            let mut stuck = Vec::new();
            for watch in &mut held.watches {
                if watch.due.is_some_and(|due| due <= now)
                    && let Some(line) = watch.held_back(&held.records)
                    && watch.told.try_send(line.to_string()).is_err()
                {
                    stuck.push(watch.connection);
                }
            }
            self.heeded(held.hang_up(&stuck));
            let next = held.watches.iter().filter_map(|watch| watch.due).min();
            guard = match next {
                Some(due) => {
                    let wait = due.saturating_duration_since(Instant::now());
                    self.paced
                        .wait_timeout(guard, wait)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
                None => self
                    .paced
                    .wait(guard)
                    .unwrap_or_else(PoisonError::into_inner),
            };
        }
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
/// program runs, and tells paced watches on another.
pub fn accept(shared: Arc<Shared>, listener: UnixListener) {
    let pacing = Arc::clone(&shared);
    std::thread::spawn(move || pacing.pace());
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
        let answer = match own {
            Some(own) => Some(own),
            None => answer(shared, &mut held, asked, connection, &told, &hangup),
        };
        // Queued while the lock is held, so a change can't be queued before
        // the answer it follows; one that can't be queued is hung up on, as
        // a change would hang up on it. A watch kept on has nothing to say
        // when it held nothing back.
        let queued = answer.is_none_or(|answer| told.try_send(answer.to_string()).is_ok());
        drop(held);
        if !queued {
            break;
        }
    }
    let went = lock(shared).drop_watches(|watch| watch.connection == connection);
    shared.heeded(went);
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

/// What one ask is answered with, if anything.
fn answer(
    shared: &Shared,
    held: &mut Held,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    let domain = &shared.domain;
    if let Ask::Watch {
        id,
        kinds,
        keys,
        places,
        every,
    } = asked
    {
        let records = (kinds.as_slice(), keys.as_deref(), places.as_deref());
        if let Some(running) = held
            .watches
            .iter_mut()
            .find(|watch| watch.is(connection, &id, records))
        {
            return kept_on(shared, running, &held.records, every);
        }
        let watch = Watch {
            connection,
            id: id.clone(),
            kinds,
            keys,
            places,
            told: told.clone(),
            stream: Arc::clone(hangup),
            every,
            moved: BTreeSet::new(),
            due: None,
        };
        let answer = watched(domain, held, watch);
        shared.heeded(true);
        return Some(answer);
    }
    Some(match asked {
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
        Ask::Watch { .. } => unreachable!("a watch is answered above"),
        Ask::Unwatch { id } => {
            let went = held.drop_watches(|watch| watch.connection == connection && watch.id == id);
            shared.heeded(went);
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
    })
}

/// A new watch, answered with every record it wants whole; one asked
/// again by its id for other records replaces the one before.
fn watched(domain: &Domain, held: &mut Held, watch: Watch) -> Value {
    held.watches
        .retain(|was| !(was.connection == watch.connection && was.id == watch.id));
    let records: Vec<&Value> = held
        .records
        .iter()
        .filter(|(key, _)| watch.wants(domain, key))
        .map(|(_, record)| record)
        .collect();
    let answer = json!({"id": watch.id, "records": records});
    held.watches.push(watch);
    answer
}

/// A watch asked again for the same records, kept on at its new pace:
/// told at once what it held back when it's no longer paced, and
/// otherwise told it when its new pace says, never later than the old.
fn kept_on(
    shared: &Shared,
    running: &mut Watch,
    records: &BTreeMap<String, Value>,
    every: Option<Duration>,
) -> Option<Value> {
    running.every = every;
    let Some(every) = every else {
        return running.held_back(records);
    };
    if let Some(due) = running.due {
        running.due = Some(due.min(Instant::now() + every));
        shared.paced.notify_all();
    }
    None
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
    fn a_paced_watch_is_told_all_that_moved_in_one_line_once_its_pace_is_up() {
        let (shared, mut client, _folder) = served("paced", Box::new(|_| None));
        shared.change(
            vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
            json!({}),
        );
        let pace = Duration::from_millis(300);
        client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "every": 300}));
        let began = Instant::now();
        shared.change(
            vec![row("agents", "O", "harbour"), row("agents", "P", "reef")],
            json!({}),
        );
        shared.change(vec![row("agents", "O", "buoy")], json!({}));
        let told = client.heard();
        assert!(
            began.elapsed() >= pace,
            "told before its pace was up: {told}"
        );
        assert_eq!(
            told["changed"],
            json!([row("agents", "O", "buoy")]),
            "the row as it last read, once"
        );
        assert_eq!(
            told["gone"],
            json!(["row:agents:P"]),
            "and the key that went"
        );
    }

    #[test]
    fn a_watch_asked_again_keeps_on_at_its_new_pace_and_tells_what_it_held_back() {
        let (shared, mut client, _folder) = served("kept", Box::new(|_| None));
        shared.change(
            vec![row("agents", "O", "kelp"), row("agents", "P", "reef")],
            json!({}),
        );
        let watch = json!({"id": "all", "ask": "watch", "kind": "row", "place": "agents"});
        client.ask(&watch);
        let mut shut = watch.clone();
        shut["every"] = json!(60_000);
        client.send(&shut);
        // Asks are answered in turn, so once this is, the watch is paced.
        client.ask(&json!({"id": "paced", "ask": "health"}));
        shared.change(vec![row("agents", "O", "harbour")], json!({}));
        shared.change(
            vec![row("agents", "O", "buoy"), row("agents", "Q", "lantern")],
            json!({}),
        );
        client.send(&json!({"id": "health", "ask": "health"}));
        let next = client.heard();
        assert_eq!(
            next["id"], "health",
            "paced, it was told nothing yet, and asked again nothing whole: {next}"
        );
        client.send(&watch);
        let told = client.heard();
        assert_eq!(
            (&told["id"], &told["changed"], &told["gone"]),
            (
                &json!("all"),
                &json!([row("agents", "O", "buoy"), row("agents", "Q", "lantern")]),
                &json!(["row:agents:P"])
            ),
            "at once again, it's told at once all it held back, in one line"
        );
    }

    #[test]
    fn a_daemon_hears_each_watch_begin_and_end() {
        let (shared, mut client, folder) = served("heeded", Box::new(|_| None));
        let (telling, told) = channel();
        shared.on_watches(Box::new(move || {
            let _ = telling.send(());
        }));
        assert!(!shared.watches("row", "agents"), "nothing watches yet");
        let watch = json!({"id": 1, "ask": "watch", "kind": "row", "place": "agents"});
        client.ask(&watch);
        assert!(told.recv().is_ok(), "told as it begins");
        assert!(shared.watches("row", "agents"), "its place is watched");
        assert!(!shared.watches("row", "mail"), "another isn't");
        client.ask(&json!({"id": 1, "ask": "unwatch"}));
        assert!(told.recv().is_ok(), "told as it's unwatched");
        assert!(!shared.watches("row", "agents"), "unwatched, it isn't");
        let mut another = Client::connect(&folder.0.join("daemon.sock"));
        another.ask(&watch);
        assert!(told.recv().is_ok(), "told as the other begins");
        drop(another);
        assert!(told.recv().is_ok(), "told as its connection ends");
        assert!(!shared.watches("row", "agents"), "its watch went with it");
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
