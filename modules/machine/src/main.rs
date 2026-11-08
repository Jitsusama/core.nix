//! machine SOCKET [ROOT]
//!
//! How this machine is doing, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (interface.rs):
//! the processor, the memory, each disk, this boot and the programs using
//! the machine most, each a record whose figures are the kernel's own
//! (lib.rs says how they're named). The kernel's files are read under
//! ROOT, `/` unless a test says otherwise.
//!
//! Nothing is read while nobody watches, so an idle machine wakes it for
//! nothing. While someone does, what they watch is read once a second,
//! and once more a fifth of a second after a read whose rates aren't of
//! now (a first read's are since boot, and one after a gap since then),
//! so a figure stands for now as soon as it can. Each read tells every
//! watch the records it changed, all in one line. The programs are read
//! only while someone watches `using`, since reading every process costs
//! the most. Each disk's health is asked of UDisks once a minute while
//! disks are watched, on a thread of its own, so a slow bus never holds a
//! read back.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use machine::interface::{Ask, KINDS, VERSION, ask, error, kind_of};
use machine::{Root, boot, disk, memory, now_ms, processor, using};
use serde_json::{Value, json};

/// How often what's watched is read.
const EVERY: Duration = Duration::from_secs(1);
/// How soon after a read whose rates aren't of now it's read again.
const SOON: Duration = Duration::from_millis(200);
/// How often each disk's health is asked while disks are watched.
const HEALTH_EVERY: Duration = Duration::from_secs(60);
/// What a get answers from what's held, rather than reading again.
const FRESH: Duration = Duration::from_millis(1500);
/// How near its time a kind is read with others due now.
const LEEWAY: Duration = Duration::from_millis(50);
/// How many of the last reads' times health says.
const TIMES: usize = 32;

/// One watch a client holds.
struct Watch {
    connection: u64,
    id: Value,
    kinds: Vec<String>,
    keys: Option<Vec<String>>,
    told: Sender<String>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        }
    }
}

/// The counters each kind's rates are measured from, and what's fixed.
struct Reader {
    root: Root,
    fixed: processor::Fixed,
    page_b: u64,
    processor: Option<processor::Counters>,
    memory: Option<memory::Counters>,
    disks: Option<disk::Counters>,
    programs: Option<using::Counters>,
}

impl Reader {
    /// Reads these kinds, as rates from their last read.
    fn read(&mut self, kinds: &HashSet<String>, health: &BTreeMap<String, Value>) -> Vec<Value> {
        let at = now_ms();
        let mut records = Vec::new();
        let wants = |kind: &str| kinds.contains(kind);
        if wants("processor") {
            let now = processor::counters(&self.root, at);
            records.push(processor::record(
                &self.root,
                &self.fixed,
                &now,
                self.processor.as_ref(),
            ));
            self.processor = Some(now);
        }
        if wants("memory") {
            let now = memory::counters(&self.root, at);
            records.push(memory::record(
                &self.root,
                &now,
                self.memory.as_ref(),
                self.page_b,
            ));
            self.memory = Some(now);
        }
        if wants("disk") {
            let names = disk::disks(&self.root);
            let now = disk::counters(&self.root, at, &names);
            for name in names.iter().filter(|name| now.disks.contains_key(*name)) {
                let said = health.get(name).cloned().unwrap_or(Value::Null);
                records.push(disk::record(
                    &self.root,
                    name,
                    &now,
                    self.disks.as_ref(),
                    &said,
                ));
            }
            self.disks = Some(now);
        }
        if wants("boot") {
            records.push(boot::record(&self.root, boot::asleep_s()));
        }
        if wants("using") {
            let now = using::counters(&self.root);
            records.extend(using::records(&now, self.programs.as_ref(), self.page_b));
            self.programs = Some(now);
        }
        records
    }
}

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    /// When each kind was last read.
    read_at: HashMap<String, Instant>,
    watches: Vec<Watch>,
    /// Each disk's health as UDisks last said it.
    health: BTreeMap<String, Value>,
    /// How asking UDisks stands: "answering", or "refused" and why.
    udisks: Value,
    took: VecDeque<u64>,
    since: u64,
}

struct Shared {
    daemon: Mutex<Daemon>,
    reader: Mutex<Reader>,
    woken: Condvar,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(socket) = args.next() else {
        eprintln!("usage: machine SOCKET [ROOT]");
        std::process::exit(64);
    };
    let root = Root::new(args.next().unwrap_or_else(|| "/".to_owned()));
    let socket = PathBuf::from(socket);
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    // SAFETY: sysconf only reads a constant of the running system.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let shared = Arc::new(Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            read_at: HashMap::new(),
            watches: Vec::new(),
            health: BTreeMap::new(),
            udisks: json!({"state": "starting"}),
            took: VecDeque::new(),
            since: now_ms(),
        }),
        reader: Mutex::new(Reader {
            fixed: processor::fixed(&root),
            root,
            page_b: u64::try_from(page).unwrap_or(4096),
            processor: None,
            memory: None,
            disks: None,
            programs: None,
        }),
        woken: Condvar::new(),
    });
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || watch_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || health_loop(&shared));
    }
    for (connection, stream) in (1u64..).zip(listener.incoming()) {
        let Ok(stream) = stream else { continue };
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || serve(&shared, stream, connection));
    }
}

/// The socket systemd holds for it (machine.socket), so the shell never
/// finds it missing while the daemon starts or starts again; or, run by
/// hand, its own, in a folder only Joel reads, replacing one left behind.
fn listen(socket: &std::path::Path) -> std::io::Result<UnixListener> {
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The kinds some watch holds.
fn watched(daemon: &Daemon) -> HashSet<String> {
    daemon
        .watches
        .iter()
        .flat_map(|watch| match &watch.keys {
            Some(keys) => keys
                .iter()
                .filter_map(|key| kind_of(key))
                .map(str::to_owned)
                .collect(),
            None => watch.kinds.clone(),
        })
        .collect()
}

/// Reads these kinds now, keeps what changed, and tells each watch of it.
fn read(shared: &Shared, kinds: &HashSet<String>) {
    let health = lock(&shared.daemon).health.clone();
    let started = Instant::now();
    let records = lock(&shared.reader).read(kinds, &health);
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut daemon = lock(&shared.daemon);
    daemon.took.push_back(took);
    while daemon.took.len() > TIMES {
        daemon.took.pop_front();
    }
    let now = Instant::now();
    for kind in kinds {
        daemon.read_at.insert(kind.clone(), now);
    }
    // A key of a kind read that it no longer says is gone, as a disk
    // taken out is, or a rank no program fills.
    let said: HashSet<String> = records
        .iter()
        .filter_map(|record| record["key"].as_str().map(str::to_owned))
        .collect();
    let gone: Vec<String> = daemon
        .held
        .keys()
        .filter(|key| kind_of(key).is_some_and(|kind| kinds.contains(kind)) && !said.contains(*key))
        .cloned()
        .collect();
    for key in &gone {
        daemon.held.remove(key);
    }
    let mut changed = Vec::new();
    for record in records {
        let Some(key) = record["key"].as_str().map(str::to_owned) else {
            continue;
        };
        if daemon.held.get(&key) != Some(&record) {
            daemon.held.insert(key, record.clone());
            changed.push(record);
        }
    }
    if changed.is_empty() && gone.is_empty() {
        return;
    }
    for watch in &daemon.watches {
        let records: Vec<&Value> = changed
            .iter()
            .filter(|record| record["key"].as_str().is_some_and(|key| watch.wants(key)))
            .collect();
        let went: Vec<&String> = gone.iter().filter(|key| watch.wants(key)).collect();
        if records.is_empty() && went.is_empty() {
            continue;
        }
        let mut told = json!({"id": watch.id, "changed": records});
        if !went.is_empty() {
            told["gone"] = json!(went);
        }
        let _ = watch.told.send(told.to_string());
    }
}

/// Whether any record of these kinds says its rates aren't of now.
fn unsettled(daemon: &Daemon, kinds: &HashSet<String>) -> bool {
    daemon.held.iter().any(|(key, record)| {
        kind_of(key).is_some_and(|kind| kinds.contains(kind))
            && record["since"].as_str().is_some_and(|since| since != "now")
    })
}

/// Reads what's watched while anything is, each kind once its time since
/// its last read comes round, and sleeps while nothing is watched. A
/// watch reads what it wants as it's asked, so this reads it next a
/// fifth of a second or a second after that, never at once again.
fn watch_loop(shared: &Shared) {
    loop {
        let mut daemon = lock(&shared.daemon);
        while daemon.watches.is_empty() {
            daemon = shared
                .woken
                .wait(daemon)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
        let kinds = watched(&daemon);
        let mut due = HashSet::new();
        let mut next = EVERY;
        for kind in &kinds {
            let one = HashSet::from([kind.clone()]);
            let every = if unsettled(&daemon, &one) {
                SOON
            } else {
                EVERY
            };
            let age = daemon
                .read_at
                .get(kind)
                .map_or(Duration::MAX, Instant::elapsed);
            // A kind nearly due is read with the rest, so reads keep together.
            if age + LEEWAY >= every {
                due.insert(kind.clone());
            } else {
                next = next.min(every - age);
            }
        }
        if due.is_empty() {
            drop(
                shared
                    .woken
                    .wait_timeout(daemon, next)
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            continue;
        }
        drop(daemon);
        read(shared, &due);
    }
}

/// Asks UDisks for each disk's health once a minute while disks are
/// watched, and at once when they start to be.
fn health_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last: Option<Instant> = None;
    loop {
        {
            let mut daemon = lock(&shared.daemon);
            loop {
                let watching = watched(&daemon).contains("disk");
                if !watching {
                    last = None;
                }
                let due = last.is_none_or(|at| at.elapsed() >= HEALTH_EVERY);
                if watching && due {
                    break;
                }
                let wait = if watching {
                    HEALTH_EVERY.saturating_sub(last.map_or(Duration::ZERO, |at| at.elapsed()))
                } else {
                    Duration::from_secs(3600)
                };
                daemon = shared
                    .woken
                    .wait_timeout(daemon, wait)
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .0;
            }
        }
        last = Some(Instant::now());
        let names = disk::disks(&lock(&shared.reader).root);
        if bus.is_none() {
            match zbus::blocking::Connection::system() {
                Ok(connection) => bus = Some(connection),
                Err(why) => {
                    lock(&shared.daemon).udisks = json!({"state": "refused", "reason": format!("the system bus isn't there: {why}")});
                    continue;
                }
            }
        }
        let Some(connection) = bus.as_ref() else {
            continue;
        };
        let mut said = BTreeMap::new();
        let mut refused = None;
        for name in names {
            match smart(connection, &name) {
                Ok(health) => {
                    said.insert(name, health);
                }
                Err(why) => refused = Some(why),
            }
        }
        let mut daemon = lock(&shared.daemon);
        daemon.health = said;
        daemon.udisks = match refused {
            Some(why) => json!({"state": "refused", "reason": why}),
            None => json!({"state": "answering"}),
        };
        shared.woken.notify_all();
    }
}

/// A disk's health as its controller tells UDisks, with no prompt: every
/// attribute of its SMART log, and its controller's own figures, or null
/// for a disk UDisks keeps none for, as one that isn't NVMe.
fn smart(connection: &zbus::blocking::Connection, name: &str) -> Result<Value, String> {
    let udisks = "org.freedesktop.UDisks2";
    let block = zbus::blocking::Proxy::new(
        connection,
        udisks,
        format!("/org/freedesktop/UDisks2/block_devices/{name}"),
        "org.freedesktop.UDisks2.Block",
    )
    .map_err(|why| why.to_string())?;
    let Ok(drive) = block.get_property::<zbus::zvariant::OwnedObjectPath>("Drive") else {
        return Ok(Value::Null);
    };
    if drive.as_str() == "/" {
        return Ok(Value::Null);
    }
    let controller = zbus::blocking::Proxy::new(
        connection,
        udisks,
        drive.as_str().to_owned(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )
    .map_err(|why| why.to_string())?;
    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert(
        "auth.no_user_interaction",
        zbus::zvariant::Value::from(true),
    );
    let Ok(attributes) = controller.call::<_, _, HashMap<String, zbus::zvariant::OwnedValue>>(
        "SmartGetAttributes",
        &(options,),
    ) else {
        return Ok(Value::Null);
    };
    let mut health: serde_json::Map<String, Value> = attributes
        .iter()
        .map(|(name, value)| (name.clone(), plain(value)))
        .collect();
    for (property, field) in [
        ("SmartCriticalWarning", "critical_warning"),
        ("SmartPowerOnHours", "power_on_hours"),
        ("SmartTemperature", "temperature_k"),
        ("SmartUpdated", "updated_s"),
        ("NVMeRevision", "nvme_revision"),
        ("UnallocatedCapacity", "unallocated_b"),
        ("State", "state"),
    ] {
        if let Ok(value) = controller.get_property::<zbus::zvariant::OwnedValue>(property) {
            health.insert(field.to_owned(), plain(&value));
        }
    }
    Ok(Value::Object(health))
}

/// A D-Bus value as plain JSON.
fn plain(value: &zbus::zvariant::Value) -> Value {
    use zbus::zvariant::Value as D;
    match value {
        D::U8(n) => json!(n),
        D::Bool(b) => json!(b),
        D::I16(n) => json!(n),
        D::U16(n) => json!(n),
        D::I32(n) => json!(n),
        D::U32(n) => json!(n),
        D::I64(n) => json!(n),
        D::U64(n) => json!(n),
        D::F64(n) => json!(n),
        D::Str(s) => json!(s.as_str()),
        D::ObjectPath(p) => json!(p.as_str()),
        D::Array(items) => Value::Array(items.iter().map(plain).collect()),
        D::Value(inner) => plain(inner),
        D::Dict(dict) => {
            let mut object = serde_json::Map::new();
            for (key, value) in dict.iter() {
                let key = match plain(key) {
                    Value::String(s) => s,
                    other => other.to_string(),
                };
                object.insert(key, plain(value));
            }
            Value::Object(object)
        }
        _ => Value::Null,
    }
}

fn serve(shared: &Arc<Shared>, stream: UnixStream, connection: u64) {
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
        let answer = answer(shared, ask(&line), connection, &told);
        if told.send(answer.to_string()).is_err() {
            break;
        }
    }
    lock(&shared.daemon)
        .watches
        .retain(|watch| watch.connection != connection);
}

/// The records held of these kinds or keys, read first where they aren't
/// fresh, in the daemon's order: by kind, then by key.
fn records(shared: &Shared, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    let wanted: HashSet<String> = match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| kind_of(key))
            .map(str::to_owned)
            .collect(),
        None => kinds.iter().cloned().collect(),
    };
    let stale: HashSet<String> = {
        let daemon = lock(&shared.daemon);
        wanted
            .iter()
            .filter(|kind| {
                daemon
                    .read_at
                    .get(*kind)
                    .is_none_or(|at| at.elapsed() > FRESH)
            })
            .cloned()
            .collect()
    };
    if !stale.is_empty() {
        read(shared, &stale);
    }
    let daemon = lock(&shared.daemon);
    let mut found: Vec<Value> = match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| daemon.held.get(key))
            .cloned()
            .collect(),
        None => daemon
            .held
            .iter()
            .filter(|(key, _)| kind_of(key).is_some_and(|kind| wanted.contains(kind)))
            .map(|(_, record)| record.clone())
            .collect(),
    };
    if keys.is_none() {
        found.sort_by_key(|record| {
            let kind = record["kind"].as_str().unwrap_or("");
            KINDS.iter().position(|k| *k == kind).unwrap_or(KINDS.len())
        });
    }
    found
}

fn sources(daemon: &Daemon) -> Value {
    json!({"kernel": {"state": "answering"}, "udisks": daemon.udisks})
}

/// What one ask is answered with.
fn answer(shared: &Arc<Shared>, asked: Ask, connection: u64, told: &Sender<String>) -> Value {
    match asked {
        Ask::Hello { id } => {
            let daemon = lock(&shared.daemon);
            json!({
                "id": id, "version": VERSION, "domain": "machine", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
            })
        }
        Ask::Get { id, keys } => {
            json!({"id": id, "records": records(shared, &[], Some(&keys))})
        }
        Ask::List { id, kinds } => json!({"id": id, "records": records(shared, &kinds, None)}),
        Ask::Watch { id, kinds, keys } => {
            let held = records(shared, &kinds, keys.as_deref());
            let mut daemon = lock(&shared.daemon);
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds,
                keys,
                told: told.clone(),
            });
            shared.woken.notify_all();
            json!({"id": id, "records": held})
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
                "id": id, "version": VERSION, "domain": "machine", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
                "watches": daemon.watches.len(),
                "took_us": daemon.took.iter().collect::<Vec<_>>(),
            })
        }
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    }
}
