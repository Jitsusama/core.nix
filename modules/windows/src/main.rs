//! windows SOCKET
//!
//! niri's windows, held ready for the shell and anything else that asks,
//! over SOCKET in the shell's daemon interface (interface.rs): each
//! window as a `window` record, named by its program and placed as its
//! row says it, each workspace as a `workspace` record, and what holds
//! the keys as the one `desktop` record (desktop.rs).
//!
//! It hears niri's event stream on `NIRI_SOCKET` for as long as it runs,
//! since a stream can't be paused, and works out every record again at
//! each event, which takes microseconds here where it took the shell's
//! main thread milliseconds; a watch is told only the records that
//! changed, and those gone, so a focus moving tells two windows and a
//! window resizing tells nothing. Each window's program is looked up once
//! by its app id, and again only once inotify says the desktop entries
//! changed. Nothing runs on a timer: with niri gone, it tries niri again
//! every two seconds until it's back.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use programs::notify::Notify;
use programs::{now_ms, xdg};
use serde_json::{Value, json};
use windows::desktop::{Desktop, Heard, Names};
use windows::entries::Entries;
use windows::interface::{Ask, KINDS, VERSION, ask, error, kind_of, tells, trimmed};

/// How many of the last events' times health says.
const TIMES: usize = 32;
/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on.
const QUEUED: usize = 256;
/// How long to wait after a connection couldn't be taken.
const ACCEPT_BACKOFF: Duration = Duration::from_millis(100);
/// How long to wait before asking niri again once it's gone.
const NIRI_AGAIN: Duration = Duration::from_secs(2);

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    /// The fields it's told of each record, or all of them.
    fields: Option<Vec<String>>,
    told: SyncSender<String>,
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

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    watches: Vec<Watch>,
    desktop: Desktop,
    entries: Entries,
    names: Names,
    /// How niri's stream stands: "starting" until it first answers,
    /// "answering", or "not answering" while it's gone.
    niri: &'static str,
    events: u64,
    /// How long each of the last events took to hear, in microseconds.
    took: VecDeque<u64>,
    since: u64,
}

struct Shared {
    daemon: Mutex<Daemon>,
    folders: Vec<PathBuf>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(socket) = args.next() else {
        eprintln!("usage: windows SOCKET");
        std::process::exit(64);
    };
    let Some(niri) = std::env::var_os("NIRI_SOCKET").map(PathBuf::from) else {
        eprintln!("<3>NIRI_SOCKET isn't set, so there's no niri to hear");
        std::process::exit(78);
    };
    let socket = PathBuf::from(socket);
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let folders = xdg::folders_here();
    let (entries, looked) = Entries::read(&folders);
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            watches: Vec::new(),
            desktop: Desktop::default(),
            entries,
            names: Names::default(),
            niri: "starting",
            events: 0,
            took: VecDeque::new(),
            since: now_ms(),
        }),
        folders,
    });
    match Notify::new() {
        Ok(notify) => {
            // The entries' folders are watched before niri is heard, so
            // no change after the first reading goes unheard.
            let heard = heard(&shared, &notify, looked);
            let shared = Arc::clone(&shared);
            std::thread::spawn(move || notice(&shared, &notify, &heard));
        }
        Err(why) => eprintln!("<4>inotify isn't there, so entries changing aren't heard: {why}"),
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || follow(&shared, &niri));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else {
            std::thread::sleep(ACCEPT_BACKOFF);
            continue;
        };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket systemd holds for it (windows.socket), or, run by hand,
/// its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &Path) -> std::io::Result<UnixListener> {
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

/// The listening socket systemd hands this process (`sd_listen_fds`).
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Hears niri's event stream, and asks again every two seconds while
/// niri's gone, as it is between one niri and the next.
fn follow(shared: &Shared, niri: &Path) {
    loop {
        match UnixStream::connect(niri) {
            Ok(stream) => {
                hear(shared, stream);
                eprintln!("<4>niri's event stream ended; asking again");
            }
            Err(why) => {
                if lock(&shared.daemon).niri != "not answering" {
                    eprintln!("<4>niri isn't answering on {}: {why}", niri.display());
                }
            }
        }
        lock(&shared.daemon).niri = "not answering";
        std::thread::sleep(NIRI_AGAIN);
    }
}

/// Hears each event of one stream until it ends. niri describes the
/// whole desktop as the stream begins, so what's held from a stream
/// before is replaced by it.
fn hear(shared: &Shared, mut stream: UnixStream) {
    let Ok(reading) = stream.try_clone() else {
        return;
    };
    if writeln!(stream, "\"EventStream\"").is_err() {
        return;
    }
    for line in BufReader::new(reading).lines() {
        let Ok(line) = line else { return };
        let Ok(event) = serde_json::from_str::<Value>(&line) else {
            eprintln!("<4>niri said what isn't JSON: {line}");
            continue;
        };
        if let Some(why) = event.get("Err") {
            eprintln!("<3>niri refused the event stream: {why}");
            return;
        }
        let began = Instant::now();
        let mut daemon = lock(&shared.daemon);
        daemon.niri = "answering";
        if event.get("Ok").is_some() {
            continue;
        }
        daemon.events += 1;
        if daemon.desktop.hear(&event) == Heard::Changed {
            refresh(&mut daemon);
        }
        let took = u64::try_from(began.elapsed().as_micros()).unwrap_or(u64::MAX);
        if daemon.took.len() == TIMES {
            daemon.took.pop_front();
        }
        daemon.took.push_back(took);
    }
}

/// Works every record out again, keeps those that changed and lets go of
/// those gone, telling each watch what changed of what it wants.
fn refresh(daemon: &mut Daemon) {
    let Daemon {
        desktop,
        entries,
        names,
        ..
    } = &mut *daemon;
    let records = desktop.records(entries, names);
    let said: HashSet<String> = records
        .iter()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect();
    let gone: Vec<String> = daemon
        .held
        .keys()
        .filter(|key| !said.contains(*key))
        .cloned()
        .collect();
    for key in &gone {
        daemon.held.remove(key);
    }
    // Each record changed, with what it was, so a watch of some fields is
    // told it only when one of those changed.
    let mut changed = Vec::new();
    for record in records {
        let Some(key) = record["key"].as_str().map(str::to_owned) else {
            continue;
        };
        if daemon.held.get(&key) != Some(&record) {
            let was = daemon.held.insert(key, record.clone());
            changed.push((was, record));
        }
    }
    if changed.is_empty() && gone.is_empty() {
        return;
    }
    let mut stuck = HashSet::new();
    for watch in &daemon.watches {
        let fields = watch.fields.as_deref();
        let records: Vec<Value> = changed
            .iter()
            .filter(|(was, now)| {
                now["key"].as_str().is_some_and(|key| watch.wants(key))
                    && tells(was.as_ref(), now, fields)
            })
            .map(|(_, now)| trimmed(now, fields))
            .collect();
        let went: Vec<&String> = gone.iter().filter(|key| watch.wants(key)).collect();
        if records.is_empty() && went.is_empty() {
            continue;
        }
        let mut told = json!({"id": watch.id, "changed": records});
        if !went.is_empty() {
            told["gone"] = json!(went);
        }
        if watch.told.try_send(told.to_string()).is_err() {
            stuck.insert(watch.connection);
            let _ = watch.stream.shutdown(Shutdown::Both);
        }
    }
    if !stuck.is_empty() {
        daemon
            .watches
            .retain(|watch| !stuck.contains(&watch.connection));
    }
}

/// The entries changed: each is read again, every app id looked up again,
/// and the windows whose program that changed told.
fn entries_changed(shared: &Shared) {
    let (entries, _) = Entries::read(&shared.folders);
    let mut daemon = lock(&shared.daemon);
    daemon.entries = entries;
    daemon.names.forget();
    refresh(&mut daemon);
}

/// What a watched folder is to this daemon.
#[derive(Default)]
struct Watched {
    /// Whether it's a folder of entries, whose every change counts.
    entries: bool,
    /// The links in it that a data folder resolves through, by name, as
    /// `current-system` in `/run`: replaced, the entries changed.
    links: Vec<String>,
}

/// Watches every folder of entries, and each link a data folder is
/// reached through, answering what each watch is.
fn heard(shared: &Shared, notify: &Notify, looked: Vec<PathBuf>) -> HashMap<i32, Watched> {
    let mut folders: HashMap<i32, Watched> = HashMap::new();
    for folder in looked {
        if let Some(watch) = notify.watch(&folder) {
            folders.entry(watch).or_default().entries = true;
        }
    }
    // On NixOS each data folder is a store path reached through a
    // profile's links, so a switch says nothing in the folder itself: the
    // link it replaces is heard instead.
    for (folder, name) in shared
        .folders
        .iter()
        .flat_map(|folder| xdg::links_toward(folder))
    {
        if let Some(watch) = notify.watch(&folder) {
            let links = &mut folders.entry(watch).or_default().links;
            if !links.contains(&name) {
                links.push(name);
            }
        }
    }
    folders
}

/// Hears every change inotify says, for as long as the daemon runs.
fn notice(shared: &Shared, notify: &Notify, folders: &HashMap<i32, Watched>) {
    loop {
        let Ok(changes) = notify.changes() else {
            return;
        };
        let entries = changes.iter().any(|(watch, name)| {
            folders
                .get(watch)
                .is_some_and(|watched| watched.entries || watched.links.contains(name))
        });
        if entries {
            entries_changed(shared);
        }
    }
}

fn serve(shared: &Arc<Shared>, stream: UnixStream, connection: u64) {
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
        let Some(answer) = answer(shared, ask(&line), connection, &told, &hangup) else {
            continue;
        };
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(&shared.daemon)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// When a record's window was last looked at, for placing it; 0 for any
/// other record.
fn order(record: &Value) -> (u64, u64) {
    (
        record["order"].as_u64().unwrap_or(0),
        record["id"].as_u64().unwrap_or(0),
    )
}

/// The records held of these kinds or keys: by kind, the windows the one
/// used last first, so a whole answer is in the order the shell lists
/// it, and the rest by key.
fn held(daemon: &Daemon, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| daemon.held.get(key))
            .cloned()
            .collect(),
        None => KINDS
            .iter()
            .filter(|kind| kinds.iter().any(|k| k == *kind))
            .flat_map(|kind| {
                let mut records: Vec<Value> = daemon
                    .held
                    .iter()
                    .filter(|(key, _)| kind_of(key) == Some(kind))
                    .map(|(_, record)| record.clone())
                    .collect();
                if *kind == "window" {
                    records.sort_by_key(|record| std::cmp::Reverse(order(record)));
                }
                records
            })
            .collect(),
    }
}

fn sources(daemon: &Daemon) -> Value {
    json!({
        "niri": {"state": daemon.niri, "events": daemon.events},
        "entries": {"state": "answering", "count": daemon.entries.len()},
    })
}

/// What one ask is answered with, or none where it's been told already.
fn answer(
    shared: &Arc<Shared>,
    asked: Ask,
    connection: u64,
    told: &SyncSender<String>,
    hangup: &Arc<UnixStream>,
) -> Option<Value> {
    Some(match asked {
        Ask::Hello { id } => {
            let daemon = lock(&shared.daemon);
            json!({
                "id": id, "version": VERSION, "domain": "windows", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
            })
        }
        Ask::Get { id, keys } => {
            json!({"id": id, "records": held(&lock(&shared.daemon), &[], Some(&keys))})
        }
        Ask::List { id, kinds } => {
            json!({"id": id, "records": held(&lock(&shared.daemon), &kinds, None)})
        }
        Ask::Watch {
            id,
            kinds,
            keys,
            fields,
        } => {
            let mut daemon = lock(&shared.daemon);
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds: kinds.clone(),
                keys: keys.clone(),
                fields: fields.clone(),
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            // Told under the same lock a change is told under, so no
            // change comes before the answer it changes.
            let records: Vec<Value> = held(&daemon, &kinds, keys.as_deref())
                .iter()
                .map(|record| trimmed(record, fields.as_deref()))
                .collect();
            let whole = json!({"id": id, "records": records});
            if told.try_send(whole.to_string()).is_err() {
                let _ = hangup.shutdown(Shutdown::Both);
            }
            return None;
        }
        Ask::Unwatch { id } => {
            let mut daemon = lock(&shared.daemon);
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            json!({"id": id, "unwatched": true})
        }
        Ask::Health { id } => {
            let daemon = lock(&shared.daemon);
            json!({
                "id": id, "version": VERSION, "domain": "windows", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
                "watches": daemon.watches.len(),
                "took_us": daemon.took.iter().collect::<Vec<_>>(),
            })
        }
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}
