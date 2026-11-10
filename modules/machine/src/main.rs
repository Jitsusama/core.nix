//! machine SOCKET [ROOT]
//!
//! How this machine is doing, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (interface.rs):
//! the processor, the memory, each disk, this boot, the programs using
//! the machine most, its sensors, thermal zones and cooling, each power
//! supply, how it sleeps, its firmware, the screen's backlight and the
//! room's light, the brightness and warmth the backlight program
//! steers the built-in screen to, who has each camera open, and Keep
//! Awake's locks and the power profile (kept.rs), each a record whose
//! figures are
//! the kernel's own or its services'
//! (lib.rs says how they're named). The kernel's files are read under
//! ROOT, `/` unless a test says otherwise.
//!
//! Nothing is read while nobody watches, so an idle machine wakes it for
//! nothing. While someone does, what they watch is read once a second,
//! and once more a fifth of a second after a read whose rates aren't of
//! now (a first read's are since boot, and one after a gap since then),
//! so a figure stands for now as soon as it can. Each read tells every
//! watch the records it changed, all in one line. The programs are read
//! only while someone watches `using`, or a process's tree by its key,
//! since reading every process costs the most. What the system bus's
//! services say is asked each on a thread of its own, so a slow bus never
//! holds a read back, and only while its kind is watched: each disk's
//! health of `UDisks` once a minute, the battery's times of `UPower` every
//! five seconds, idling and what holds sleep off of logind every two, and
//! the firmware of fwupd once a minute and whenever it says it changed.
//! Its signals are listened for only while firmware is watched, but
//! logind's `PrepareForSleep` always, since a sleep happens while nothing
//! watches, and a signal wakes nothing until it comes. The backlight
//! program's files are heard as it writes them while their kinds are
//! watched, so a switch turned in the launcher reads turned within a
//! frame, not at the next second's read, and so are the cameras' holders,
//! so a program opening a camera is said as it does. Keep Awake's files and the
//! machine's chosen profile are heard as they're written, and
//! power-profiles-daemon as it says it changed, from the daemon's start,
//! since the shell watches them while the launcher's shut: neither is
//! read on a clock, so watching them wakes nothing until they change.
//!
//! It acts as well as answers, so the shell spawns and writes nothing of
//! its own: `charge-limit` moves a battery's charge limit, and the supply
//! is read again before the act is answered, so each watch is told the
//! new limit first.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use machine::interface::{Ask, KINDS, VERSION, ask, error, kind_of};
use machine::{
    Root, boot, camera, clock_ms, disk, firmware, heat, kept, light, memory, now_ms, processor,
    rounded,
    sleep, steered, supply, tree, using, written::Written,
};
use serde_json::{Value, json};

/// How often what's watched is read.
const EVERY: Duration = Duration::from_secs(1);
/// How soon after a read whose rates aren't of now it's read again.
const SOON: Duration = Duration::from_millis(200);
/// How often each disk's health is asked while disks are watched.
const HEALTH_EVERY: Duration = Duration::from_secs(60);
/// The longest a call to a bus service may take, so one that hangs can't
/// hold its thread.
const BUS_WAIT: Duration = Duration::from_secs(10);
/// How often `UPower` is asked while supplies are watched; it works its
/// times out afresh no more often than this.
const UPOWER_EVERY: Duration = Duration::from_secs(5);
/// How often fwupd is asked while firmware is watched, unless it says
/// something changed first.
const FIRMWARE_EVERY: Duration = Duration::from_secs(60);
/// How often logind is asked while `awake` is watched.
const LOGIND_EVERY: Duration = Duration::from_secs(2);
/// What a get answers from what's held, rather than reading again.
const FRESH: Duration = Duration::from_millis(1500);
/// How near its time a kind is read with others due now.
const LEEWAY: Duration = Duration::from_millis(50);
/// The kinds read as their files are written (`heard_loop`), never on
/// the second.
const STEERED: [&str; 2] = ["brightness", "warmth"];
const HEARD: [&str; 3] = ["brightness", "warmth", "camera"];
/// How often a watch on files heard looks to see whether it's still
/// wanted, once every folder is watched.
const RESTING: Duration = Duration::from_secs(60);
/// The kinds read as their files are written or their bus says they
/// changed (`kept_loop`, `profile_loop`).
const KEPT: [&str; 2] = ["kept", "profile"];
/// How long `kept_loop` waits on its files before looking again for a
/// folder that isn't there, as `/run/power-profile` isn't while its
/// service starts again.
const KEPT_LOOKS_AGAIN: Duration = Duration::from_secs(10);
/// How many of the last reads' times health says.
const TIMES: usize = 32;
/// How many lines a connection may have waiting to be written before
/// it's taken as no longer reading and hung up on, so one stuck client
/// can't grow the daemon without end.
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

/// The counters each kind's rates are measured from, and what's fixed.
struct Reader {
    root: Root,
    fixed: processor::Fixed,
    page_b: u64,
    processor: Option<processor::Counters>,
    memory: Option<memory::Counters>,
    disks: Option<disk::Counters>,
    disk_sensors: disk::Kept,
    programs: Option<using::Counters>,
    /// The sensors' labels and limits, read once.
    sensors: heat::Kept,
    /// Where the backlight program says what it steers the screen to, or
    /// none for a session with no runtime folder.
    steered: Option<steered::Files>,
    /// The session's runtime folder, where Keep Awake leaves its files.
    runtime: Option<String>,
}

impl Reader {
    /// Reads the kinds of these that are heard as their files are
    /// written (`HEARD`).
    fn heard(&self, kinds: &HashSet<String>) -> Vec<Value> {
        let wants = |kind: &str| kinds.contains(kind);
        let mut records = Vec::new();
        if let Some(files) = self.steered.as_ref() {
            if wants("brightness") {
                records.extend(steered::brightness(&self.root, files));
            }
            if wants("warmth") {
                records.extend(steered::warmth(&self.root, files));
            }
        }
        if wants("camera") {
            records.extend(camera::cameras(&self.root));
        }
        records
    }

    /// Reads these kinds, as rates from their last read, the trees of
    /// these pids among them, with what the buses last said.
    fn read(
        &mut self,
        kinds: &HashSet<String>,
        trees: &BTreeSet<u32>,
        buses: &Buses,
    ) -> Vec<Value> {
        let health = &buses.health;
        let at = clock_ms();
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
                    &mut self.disk_sensors,
                ));
            }
            self.disks = Some(now);
        }
        if wants("boot") {
            records.push(boot::record(&self.root, boot::asleep_s()));
        }
        if wants("sensor") {
            records.extend(heat::sensors(&self.root, &mut self.sensors));
        }
        if wants("zone") {
            records.extend(heat::zones(&self.root));
        }
        if wants("cooling") {
            records.extend(heat::cooling(&self.root));
        }
        if wants("firmware") {
            records.extend(buses.firmware.iter().cloned());
        }
        if wants("sleep") {
            records.push(sleep::record(&self.root));
        }
        if wants("backlight") {
            records.extend(light::backlights(&self.root));
        }
        if wants("light") {
            records.extend(buses.light.iter().cloned());
        }
        records.extend(self.heard(kinds));
        if wants("awake") {
            records.push(sleep::awake(&buses.logind));
        }
        if wants("kept") {
            records.extend(kept::locks(&self.root, self.runtime.as_deref()));
        }
        if wants("profile") {
            records.push(kept::profile(&self.root, &buses.profile));
        }
        if wants("supply") {
            for name in supply::supplies(&self.root) {
                let said = buses.upower.get(&name).cloned().unwrap_or(Value::Null);
                records.push(supply::record(&self.root, &name, &said));
            }
        }
        // The programs and the trees share one read of every process, and
        // each measures its rates from whichever read came last.
        let trees_wanted = wants("tree") && !trees.is_empty();
        if wants("using") || trees_wanted {
            let now = using::counters(&self.root, at);
            if wants("using") {
                records.extend(using::records(&now, self.programs.as_ref(), self.page_b));
            }
            if trees_wanted {
                records.extend(tree::records(
                    trees,
                    &now,
                    self.programs.as_ref(),
                    self.page_b,
                ));
            }
            self.programs = Some(now);
        }
        records
    }
}

/// What the system bus's services last said, for a read to put in its
/// records.
struct Buses {
    health: BTreeMap<String, Value>,
    upower: BTreeMap<String, Value>,
    logind: Value,
    firmware: Vec<Value>,
    light: Vec<Value>,
    profile: Value,
}

/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    /// When each kind was last read.
    read_at: HashMap<String, Instant>,
    watches: Vec<Watch>,
    /// Each disk's health as `UDisks` last said it.
    health: BTreeMap<String, Value>,
    /// How asking `UDisks` stands: "answering", or "refused" and why.
    udisks: Value,
    /// What `UPower` last worked out of each supply, by its name.
    upower: BTreeMap<String, Value>,
    /// What logind last said of idling and sleep.
    logind: Value,
    /// fwupd's records, as it last said them.
    firmware: Vec<Value>,
    /// The light sensors' records, as `light_loop` last read them.
    light: Vec<Value>,
    /// What power-profiles-daemon last said, as `profile_loop` heard it.
    profile: Value,
    /// The kinds whose source said they changed, to be asked again now.
    hurried: HashSet<String>,
    /// When the last sleep began and ended, as logind's signal said.
    slept: Arc<Mutex<Value>>,
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
    let shared = Arc::new(shared(&root));
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || watch_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || health_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || upower_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || logind_loop(&shared));
    }
    {
        let slept = Arc::clone(&lock(&shared.daemon).slept);
        std::thread::spawn(move || sleeps_loop(&slept));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || firmware_loop(&shared));
    }
    if let Some(files) = steered::Files::of_session() {
        let shared = Arc::clone(&shared);
        let folders = files
            .folders()
            .iter()
            .map(|folder| root.path(folder))
            .collect::<Vec<_>>();
        std::thread::spawn(move || {
            heard_loop(&shared, &STEERED, &folders, |name| files.names(name));
        });
    }
    {
        let shared = Arc::clone(&shared);
        let folders = vec![root.path(camera::FOLDER)];
        std::thread::spawn(move || heard_loop(&shared, &["camera"], &folders, camera::names));
    }
    {
        let shared = Arc::clone(&shared);
        let root = root.clone();
        std::thread::spawn(move || kept_loop(&shared, &root));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || profile_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || light_loop(&shared, &root));
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

/// What every thread shares, holding nothing yet, reading under `root`.
fn shared(root: &Root) -> Shared {
    // SAFETY: sysconf only reads a constant of the running system.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            read_at: HashMap::new(),
            watches: Vec::new(),
            health: BTreeMap::new(),
            udisks: json!({"state": "starting"}),
            upower: BTreeMap::new(),
            logind: Value::Null,
            firmware: Vec::new(),
            light: Vec::new(),
            profile: Value::Null,
            hurried: HashSet::new(),
            slept: Arc::new(Mutex::new(Value::Null)),
            took: VecDeque::new(),
            since: now_ms(),
        }),
        reader: Mutex::new(Reader {
            fixed: processor::fixed(root),
            root: root.clone(),
            page_b: u64::try_from(page).unwrap_or(4096),
            processor: None,
            memory: None,
            disks: None,
            disk_sensors: disk::Kept::default(),
            programs: None,
            sensors: heat::Kept::default(),
            steered: steered::Files::of_session(),
            runtime: std::env::var("XDG_RUNTIME_DIR")
                .ok()
                .filter(|folder| !folder.is_empty()),
        }),
        woken: Condvar::new(),
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

/// The pids whose trees some watch names by key.
fn watched_trees(daemon: &Daemon) -> BTreeSet<u32> {
    daemon
        .watches
        .iter()
        .filter_map(|watch| watch.keys.as_ref())
        .flatten()
        .filter_map(|key| tree::pid_of(key))
        .collect()
}

/// Reads these kinds now, the trees watched and those `asked` names among
/// them, keeps what changed, and tells each watch of it.
fn read(shared: &Shared, kinds: &HashSet<String>, asked: &[String]) {
    let (buses, trees) = {
        let daemon = lock(&shared.daemon);
        let mut trees = watched_trees(&daemon);
        trees.extend(asked.iter().filter_map(|key| tree::pid_of(key)));
        let buses = Buses {
            health: daemon.health.clone(),
            upower: daemon.upower.clone(),
            logind: daemon.logind.clone(),
            firmware: daemon.firmware.clone(),
            light: daemon.light.clone(),
            profile: daemon.profile.clone(),
        };
        (buses, trees)
    };
    let started = Instant::now();
    let records = lock(&shared.reader).read(kinds, &trees, &buses);
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
    // A connection whose lines aren't being read is hung up on, and its
    // watches dropped, rather than told less than it asked for.
    let mut stuck = HashSet::new();
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
        let mut kinds = watched(&daemon);
        kinds.retain(|kind| !HEARD.contains(&kind.as_str()) && !KEPT.contains(&kind.as_str()));
        // Only kinds heard as they're written are watched, as the bar's
        // camera and Keep Awake always are, so nothing is due on the
        // second until a watch of another kind wakes this.
        if kinds.is_empty() {
            drop(
                shared
                    .woken
                    .wait(daemon)
                    .unwrap_or_else(std::sync::PoisonError::into_inner),
            );
            continue;
        }
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
                next = next.min(every.saturating_sub(age));
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
        read(shared, &due, &[]);
    }
}

/// Waits until a kind is watched and `every` has passed since `last`, or
/// its source says it changed, and says when that was. A watch starting
/// within `every` of the last ask doesn't ask again, so a launcher opened
/// and closed doesn't ask each time; what was said then is still held.
/// A change said while nothing watches is kept for the next watch.
fn due(shared: &Shared, kind: &str, every: Duration, last: &mut Option<Instant>) {
    let mut daemon = lock(&shared.daemon);
    loop {
        let watching = watched(&daemon).contains(kind);
        let hurried = watching && daemon.hurried.remove(kind);
        if watching && (hurried || last.is_none_or(|at| at.elapsed() >= every)) {
            break;
        }
        let wait = if watching {
            every.saturating_sub(last.map_or(Duration::ZERO, |at| at.elapsed()))
        } else {
            Duration::from_secs(3600)
        };
        daemon = shared
            .woken
            .wait_timeout(daemon, wait)
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0;
    }
    *last = Some(Instant::now());
}

/// As `due`, but false as soon as nothing watches `kind`, rather than
/// waiting for a watch, so a loop listening for its source's signals can
/// stop listening with the last watch. The next watch reads at once.
fn due_while_watched(
    shared: &Shared,
    kind: &str,
    every: Duration,
    last: &mut Option<Instant>,
) -> bool {
    let mut daemon = lock(&shared.daemon);
    loop {
        if !watched(&daemon).contains(kind) {
            *last = None;
            return false;
        }
        if daemon.hurried.remove(kind) || last.is_none_or(|at| at.elapsed() >= every) {
            break;
        }
        let wait = every.saturating_sub(last.map_or(Duration::ZERO, |at| at.elapsed()));
        daemon = shared
            .woken
            .wait_timeout(daemon, wait)
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .0;
    }
    *last = Some(Instant::now());
    true
}

/// A proxy to one interface of a bus service that keeps none of its
/// properties, since each is asked afresh on every read: one that kept
/// them would ask for all of them and add a match rule each time it's
/// made.
fn proxy<'a>(
    connection: &zbus::blocking::Connection,
    service: &'a str,
    path: impl TryInto<zbus::zvariant::ObjectPath<'a>, Error: Into<zbus::Error>>,
    interface: &'a str,
) -> zbus::Result<zbus::blocking::Proxy<'a>> {
    zbus::blocking::proxy::Builder::new(connection)
        .destination(service)?
        .path(path)?
        .interface(interface)?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
}

/// A connection to the system bus whose calls give up after `BUS_WAIT`,
/// so a service that hangs can't hold its loop.
fn system_bus() -> zbus::Result<zbus::blocking::Connection> {
    zbus::blocking::connection::Builder::system()
        .and_then(|builder| builder.method_timeout(BUS_WAIT).build())
}

/// Reads the light sensors each second while they're watched, on a
/// thread of their own: a sensor's raw count takes about a tenth of a
/// second to read, and read with the rest it would hold every other kind
/// back, a step's first answer among them. What it reads is told at once.
/// With the last watch its readings are let go, so the next watch's first
/// answer says the sensor unread rather than an old count as now.
fn light_loop(shared: &Shared, root: &Root) {
    let mut last = None;
    let kinds = HashSet::from(["light".to_owned()]);
    loop {
        if !due_while_watched(shared, "light", EVERY, &mut last) {
            unheard(&mut lock(&shared.daemon));
            due(shared, "light", EVERY, &mut last);
        }
        let said = light::sensors(root);
        lock(&shared.daemon).light = said;
        read(shared, &kinds, &[]);
    }
}

/// Hears the files of these kinds in these folders as they're written
/// while the kinds are watched, and tells what changed at once: the
/// backlight program's, and the cameras' holders'. They're read at no
/// other time but a watch's first answer and a get. They're read once more
/// as the watch on them begins, so one written since the first answer is
/// said all the same. With the last watch the files are let be, so an
/// idle machine is woken by none of their writes.
fn heard_loop(shared: &Shared, kinds: &[&str], folders: &[PathBuf], ours: impl Fn(&str) -> bool) {
    let kinds: HashSet<String> = kinds.iter().map(|kind| (*kind).to_owned()).collect();
    let watching = |daemon: &Daemon| watched(daemon).iter().any(|kind| kinds.contains(kind));
    loop {
        {
            let mut daemon = lock(&shared.daemon);
            while !watching(&daemon) {
                daemon = shared
                    .woken
                    .wait(daemon)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let Some(mut written) = Written::on(folders.iter().cloned()) else {
            // With no watch the kernel will give, they're read on the
            // second instead, as every other kind is.
            while watching(&lock(&shared.daemon)) {
                read(shared, &kinds, &[]);
                std::thread::sleep(EVERY);
            }
            continue;
        };
        read(shared, &kinds, &[]);
        while watching(&lock(&shared.daemon)) {
            // A folder not there yet is looked for each second; with all
            // of them watched, only to see whether the watch has ended,
            // which can wait, since a file heard while nothing watches
            // tells nobody.
            let heard = written.wait(if written.looking() { EVERY } else { RESTING });
            if heard.iter().any(|name| name.is_empty() || ours(name)) {
                read(shared, &kinds, &[]);
            }
        }
    }
}

/// Reads these of the kinds heard by a thread of their own, those of them
/// watched, and tells what changed; unwatched, they're read afresh by the
/// next watch's first answer.
fn heard(shared: &Shared, kinds: &[&str]) {
    let watching = watched(&lock(&shared.daemon));
    let kinds: HashSet<String> = kinds
        .iter()
        .map(|kind| (*kind).to_owned())
        .filter(|kind| watching.contains(kind))
        .collect();
    if !kinds.is_empty() {
        read(shared, &kinds, &[]);
    }
}

/// Hears Keep Awake's files come and go in the runtime folder, and the
/// machine's chosen profile written, from the daemon's start, and tells
/// what changed at once. The shell watches both while the launcher's
/// shut, so they're never read on a clock: only a write wakes it, and a
/// folder not there, or taken away as systemd does a service's runtime
/// folder, is looked for again every few seconds.
fn kept_loop(shared: &Shared, root: &Root) {
    let runtime = lock(&shared.reader).runtime.clone();
    let runtime = runtime.map(|folder| root.path(&folder));
    let folders = runtime.into_iter().chain([root.path(kept::CHOSEN)]);
    let Some(mut written) = Written::on(folders) else {
        // With no watch the kernel will give, they're read by each watch's
        // first answer and each get alone.
        return;
    };
    loop {
        let names = written.wait(KEPT_LOOKS_AGAIN);
        let locks = names.iter().any(|name| name.is_empty() || kept::names_a_lock(name));
        let choice = names
            .iter()
            .any(|name| name.is_empty() || kept::names_the_choice(name));
        match (locks, choice) {
            (true, true) => heard(shared, &KEPT),
            (true, false) => heard(shared, &["kept"]),
            (false, true) => heard(shared, &["profile"]),
            (false, false) => {}
        }
    }
}

const PROFILES: &str = "org.freedesktop.UPower.PowerProfiles";
const PROFILES_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";

/// Hears power-profiles-daemon from the daemon's start, as long as it
/// runs: what it's on, offers and is held to, asked again each time it
/// says one changed or it starts again, and told at once. It waits on the
/// bus alone, so it wakes for nothing else.
fn profile_loop(shared: &Shared) {
    loop {
        if let Ok(connection) = system_bus() {
            profiles_heard(&connection, shared);
        }
        std::thread::sleep(SLEEPS_AGAIN);
    }
}

/// Asks power-profiles-daemon what it's on now and each time it says
/// something changed, or a new one takes its name, until the connection
/// ends.
fn profiles_heard(connection: &zbus::blocking::Connection, shared: &Shared) {
    let Ok(daemon) = proxy(connection, PROFILES, PROFILES_PATH, PROFILES) else {
        return;
    };
    let Ok(properties) = zbus::blocking::fdo::PropertiesProxy::builder(connection)
        .destination(PROFILES)
        .and_then(|builder| builder.path(PROFILES_PATH))
        .and_then(zbus::blocking::proxy::Builder::build)
    else {
        return;
    };
    let (Ok(changes), Ok(owners)) = (
        properties.receive_properties_changed(),
        daemon.receive_owner_changed(),
    ) else {
        return;
    };
    let ask = || {
        let said = power_profiles(&daemon);
        lock(&shared.daemon).profile = said;
        heard(shared, &["profile"]);
    };
    ask();
    std::thread::scope(|scope| {
        scope.spawn(|| owners.for_each(|_| ask()));
        changes.for_each(|_| ask());
    });
}

/// What power-profiles-daemon says: the profile it's on, those it
/// offers, each hold a program has on one, with why, and why the
/// performance profile's slowed, if it is; null while it doesn't answer.
fn power_profiles(daemon: &zbus::blocking::Proxy<'_>) -> Value {
    type Dicts = Vec<HashMap<String, zbus::zvariant::OwnedValue>>;
    let Ok(active) = daemon.get_property::<String>("ActiveProfile") else {
        return Value::Null;
    };
    let field = |dict: &HashMap<String, zbus::zvariant::OwnedValue>, name: &str| {
        dict.get(name).map_or(Value::Null, |value| plain(value))
    };
    let profiles = daemon.get_property::<Dicts>("Profiles").ok().map(|all| {
        all.iter()
            .map(|profile| field(profile, "Profile"))
            .collect::<Vec<_>>()
    });
    let holds = daemon
        .get_property::<Dicts>("ActiveProfileHolds")
        .ok()
        .map(|all| {
            all.iter()
                .map(|hold| {
                    json!({
                        "profile": field(hold, "Profile"),
                        "app": field(hold, "ApplicationId"),
                        "why": field(hold, "Reason"),
                    })
                })
                .collect::<Vec<_>>()
        });
    // Empty while it isn't slowed.
    let degraded = daemon
        .get_property::<String>("PerformanceDegraded")
        .ok()
        .filter(|why| !why.is_empty());
    json!({"active": active, "profiles": profiles, "holds": holds, "degraded": degraded})
}

/// Lets go of what each light sensor last heard, keeping the sensor and
/// its scale, which don't change.
fn unheard(daemon: &mut Daemon) {
    let forget = |record: &mut Value| {
        record["raw"] = Value::Null;
        record["lux"] = Value::Null;
    };
    daemon.light.iter_mut().for_each(forget);
    daemon
        .held
        .iter_mut()
        .filter(|(key, _)| kind_of(key) == Some("light"))
        .for_each(|(_, record)| forget(record));
}

/// Asks `UPower` what it works out of each supply every few seconds while
/// supplies are watched: its state, and the battery's time to empty or
/// full and the rate it's changing at, by the supply's name.
fn upower_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        due(shared, "supply", UPOWER_EVERY, &mut last);
        if bus.is_none() {
            bus = system_bus().ok();
        }
        let said = bus.as_ref().map(upower).unwrap_or_default();
        let mut daemon = lock(&shared.daemon);
        daemon.upower = said;
    }
}

/// Asks logind every two seconds while `awake` is watched whether the
/// session is idle and what holds sleep off, with when the last sleep
/// began and ended as its signal said.
fn logind_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        due(shared, "awake", LOGIND_EVERY, &mut last);
        if bus.is_none() {
            bus = system_bus().ok();
        }
        let slept = lock(&lock(&shared.daemon).slept).clone();
        let said = bus
            .as_ref()
            .map_or(Value::Null, |connection| logind(connection, &slept));
        lock(&shared.daemon).logind = said;
    }
}

fn login1(connection: &zbus::blocking::Connection) -> zbus::Result<zbus::blocking::Proxy<'_>> {
    proxy(
        connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
}

/// How long the sleep listener waits to try the bus again after it
/// couldn't reach it, or its connection ended.
const SLEEPS_AGAIN: Duration = Duration::from_secs(60);

/// Listens for logind's `PrepareForSleep` from the daemon's start, as long
/// as it runs, so the last sleep is known however soon after a wake it's
/// asked (Keep Awake's Last Slept): a sleep happens while nothing watches.
fn sleeps_loop(slept: &Mutex<Value>) {
    loop {
        if let Ok(connection) = system_bus() {
            sleeps(&connection, slept);
        }
        std::thread::sleep(SLEEPS_AGAIN);
    }
}

/// Keeps when each sleep began and ended, as logind's `PrepareForSleep`
/// says: true as it begins, false once it's woken. It waits on the bus
/// alone, so it wakes for nothing else.
fn sleeps(connection: &zbus::blocking::Connection, slept: &Mutex<Value>) {
    let Ok(manager) = login1(connection) else {
        return;
    };
    let Ok(signals) = manager.receive_signal("PrepareForSleep") else {
        return;
    };
    for signal in signals {
        let Ok(beginning) = signal.body().deserialize::<bool>() else {
            continue;
        };
        let mut slept = lock(slept);
        if beginning {
            *slept = json!({"began_at_ms": now_ms(), "ended_at_ms": null});
        } else {
            slept["ended_at_ms"] = json!(now_ms());
        }
    }
}

/// What logind says of idling and sleep: whether the session's idle and
/// since when, whether it's about to sleep, what's held off, and every
/// inhibitor, with the last sleep this daemon saw.
fn logind(connection: &zbus::blocking::Connection, slept: &Value) -> Value {
    let Ok(manager) = login1(connection) else {
        return Value::Null;
    };
    let inhibitors = manager
        .call::<_, _, Vec<(String, String, String, String, u32, u32)>>("ListInhibitors", &())
        .map(|all| {
            all.into_iter()
                .map(|(what, who, why, mode, uid, pid)| {
                    json!({
                        "what": what.split(':').collect::<Vec<_>>(),
                        "who": who, "why": why, "mode": mode, "uid": uid, "pid": pid,
                    })
                })
                .collect::<Vec<_>>()
        })
        .ok();
    let idle_since_ms = manager
        .get_property::<u64>("IdleSinceHint")
        .ok()
        .filter(|us| *us > 0)
        .map(|us| us / 1000);
    let listed = |property: &str| {
        manager.get_property::<String>(property).ok().map(|what| {
            what.split(':')
                .filter(|w| !w.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
    };
    json!({
        "idle": manager.get_property::<bool>("IdleHint").ok(),
        "idle_since_ms": idle_since_ms,
        "preparing": manager.get_property::<bool>("PreparingForSleep").ok(),
        // Read each time, since the machine's idle sleep reloads logind
        // with other settings on battery.
        "idle_action": manager.get_property::<String>("IdleAction").ok(),
        "idle_action_ms": manager.get_property::<u64>("IdleActionUSec").ok().map(|us| us / 1000),
        "blocked": listed("BlockInhibited"),
        "delayed": listed("DelayInhibited"),
        "inhibitors": inhibitors,
        "last_sleep": slept,
    })
}

/// Asks fwupd once a minute while firmware is watched, and at once when
/// it says something changed, what it knows of each device and what it
/// offers each. Its connection is closed with the last watch, which ends
/// the thread listening for its signals.
fn firmware_loop(shared: &Arc<Shared>) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last = None;
    loop {
        if !due_while_watched(shared, "firmware", FIRMWARE_EVERY, &mut last) {
            if let Some(connection) = bus.take() {
                let _ = connection.close();
            }
            due(shared, "firmware", FIRMWARE_EVERY, &mut last);
        }
        if bus.is_none() {
            bus = system_bus().ok();
            if let Some(connection) = bus.clone() {
                let shared = Arc::clone(shared);
                std::thread::spawn(move || changes(&connection, &shared));
            }
        }
        let said = bus.as_ref().map(fwupd).unwrap_or_default();
        lock(&shared.daemon).firmware = said;
    }
}

fn fwupd_proxy(connection: &zbus::blocking::Connection) -> zbus::Result<zbus::blocking::Proxy<'_>> {
    proxy(
        connection,
        "org.freedesktop.fwupd",
        "/",
        "org.freedesktop.fwupd",
    )
}

/// Says firmware changed each time fwupd says anything, until its
/// connection closes, waiting on the bus alone: `Changed` as its metadata does, and `DeviceAdded`,
/// `DeviceRemoved` and `DeviceChanged` as a device comes, goes, takes an
/// update or gains a problem, as one needing the charger does when it's
/// unplugged.
fn changes(connection: &zbus::blocking::Connection, shared: &Arc<Shared>) {
    let Ok(proxy) = fwupd_proxy(connection) else {
        return;
    };
    let Ok(signals) = proxy.receive_all_signals() else {
        return;
    };
    for _ in signals {
        lock(&shared.daemon).hurried.insert("firmware".to_owned());
        shared.woken.notify_all();
    }
}

/// Every record fwupd's answers make, the host's first.
fn fwupd(connection: &zbus::blocking::Connection) -> Vec<Value> {
    type Dicts = Vec<HashMap<String, zbus::zvariant::OwnedValue>>;
    let Ok(proxy) = fwupd_proxy(connection) else {
        return vec![];
    };
    let text = |property: &str| proxy.get_property::<String>(property).ok();
    let as_json = |dict: &HashMap<String, zbus::zvariant::OwnedValue>| {
        Value::Object(dict.iter().map(|(k, v)| (k.clone(), plain(v))).collect())
    };
    // What the level is made of; an fwupd that can't say has none, and
    // the journal says why.
    let attributes: Vec<Value> = match proxy.call::<_, _, Dicts>("GetHostSecurityAttrs", &()) {
        Ok(all) => all.iter().map(as_json).collect(),
        Err(why) => {
            eprintln!("<4>fwupd couldn't say its security attributes: {why}");
            vec![]
        }
    };
    let mut records = vec![firmware::host(
        text("HostSecurityId").as_deref(),
        &attributes,
        text("HostProduct").as_deref(),
        text("DaemonVersion").as_deref(),
    )];
    let Ok(devices) = proxy.call::<_, _, Dicts>("GetDevices", &()) else {
        return records;
    };
    let devices: Vec<Value> = devices.iter().map(as_json).collect();
    let mut offered = BTreeMap::new();
    for device in &devices {
        let Some(id) = device["DeviceId"].as_str() else {
            continue;
        };
        // fwupd offers only an updatable device anything.
        if device["Flags"]
            .as_u64()
            .is_none_or(|flags| flags & 0b10 == 0)
        {
            continue;
        }
        let said: firmware::Offered = match proxy.call::<_, _, Dicts>("GetUpgrades", &(id,)) {
            Ok(releases) => Ok(releases.first().map(as_json)),
            Err(zbus::Error::MethodError(_, Some(why), _)) => Err(why),
            Err(why) => Err(why.to_string()),
        };
        offered.insert(id.to_owned(), said);
    }
    // fwupd answers an empty history with an error, which is no history.
    let history = proxy
        .call::<_, _, Dicts>("GetHistory", &())
        .map(|entries| entries.iter().map(as_json).collect::<Vec<_>>())
        .unwrap_or_default();
    records.extend(firmware::records(
        &devices,
        &offered,
        &firmware::history(&history),
    ));
    records
}

/// What `UPower` says of each device it keeps, by its supply's name.
fn upower(connection: &zbus::blocking::Connection) -> BTreeMap<String, Value> {
    let service = "org.freedesktop.UPower";
    let Ok(manager) = proxy(
        connection,
        service,
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
    ) else {
        return BTreeMap::new();
    };
    let Ok(paths) =
        manager.call::<_, _, Vec<zbus::zvariant::OwnedObjectPath>>("EnumerateDevices", &())
    else {
        return BTreeMap::new();
    };
    let mut said = BTreeMap::new();
    for path in paths {
        let Ok(device) = proxy(
            connection,
            service,
            path.as_str().to_owned(),
            "org.freedesktop.UPower.Device",
        ) else {
            continue;
        };
        let Ok(name) = device.get_property::<String>("NativePath") else {
            continue;
        };
        let seconds = |property: &str| device.get_property::<i64>(property).ok().filter(|s| *s > 0);
        // UPower's states, as its Device interface numbers them.
        let state = device.get_property::<u32>("State").ok().map(|s| match s {
            1 => "charging",
            2 => "discharging",
            3 => "empty",
            4 => "full",
            5 => "pending charge",
            6 => "pending discharge",
            _ => "unknown",
        });
        let warning = device
            .get_property::<u32>("WarningLevel")
            .ok()
            .map(|w| match w {
                2 => "discharging",
                3 => "low",
                4 => "critical",
                5 => "action",
                _ => "none",
            });
        said.insert(
            name.rsplit('/').next().unwrap_or(&name).to_owned(),
            json!({
                "state": state,
                "warning": warning,
                "time_to_empty_s": seconds("TimeToEmpty"),
                "time_to_full_s": seconds("TimeToFull"),
                "rate_w": device.get_property::<f64>("EnergyRate").ok().map(|w| rounded(w, 3)),
            }),
        );
    }
    said
}

/// Asks `UDisks` for each disk's health once a minute while disks are
/// watched, and when they start to be if it last asked a minute or more
/// ago, so a launcher opened and closed doesn't ask each time.
fn health_loop(shared: &Shared) {
    let mut bus: Option<zbus::blocking::Connection> = None;
    let mut last: Option<Instant> = None;
    loop {
        due(shared, "disk", HEALTH_EVERY, &mut last);
        let names = disk::disks(&lock(&shared.reader).root);
        if bus.is_none() {
            match system_bus() {
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
                Err(why) => refused = Some(why.to_string()),
            }
        }
        // Asked again next time on a new connection, so a bus or a
        // `UDisks` that restarted is reached again.
        if refused.is_some() {
            bus = None;
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

/// Whether a bus error says only that what was asked isn't there, as a
/// disk that isn't NVMe has no controller's figures, rather than that
/// `UDisks` is missing, refused or broken.
fn absent(why: &zbus::Error) -> bool {
    const ABSENT: [&str; 5] = [
        "org.freedesktop.DBus.Error.UnknownInterface",
        "org.freedesktop.DBus.Error.UnknownMethod",
        "org.freedesktop.DBus.Error.UnknownObject",
        "org.freedesktop.DBus.Error.UnknownProperty",
        "org.freedesktop.DBus.Error.InvalidArgs",
    ];
    match why {
        zbus::Error::MethodError(name, _, _) => ABSENT.contains(&name.as_str()),
        zbus::Error::FDO(fdo) => matches!(
            **fdo,
            zbus::fdo::Error::UnknownInterface(_)
                | zbus::fdo::Error::UnknownMethod(_)
                | zbus::fdo::Error::UnknownObject(_)
                | zbus::fdo::Error::UnknownProperty(_)
                | zbus::fdo::Error::InvalidArgs(_)
        ),
        _ => false,
    }
}

/// A disk's health as its controller tells `UDisks`, with no prompt: every
/// attribute of its SMART log, and its controller's own figures, or null
/// for a disk `UDisks` keeps none for, as one that isn't NVMe. Any other
/// error, `UDisks` missing or a refusal, is said as one.
fn smart(connection: &zbus::blocking::Connection, name: &str) -> zbus::Result<Value> {
    let udisks = "org.freedesktop.UDisks2";
    let block = proxy(
        connection,
        udisks,
        format!("/org/freedesktop/UDisks2/block_devices/{name}"),
        "org.freedesktop.UDisks2.Block",
    )?;
    let drive = match block.get_property::<zbus::zvariant::OwnedObjectPath>("Drive") {
        Ok(drive) => drive,
        Err(why) if absent(&why) => return Ok(Value::Null),
        Err(why) => return Err(why),
    };
    if drive.as_str() == "/" {
        return Ok(Value::Null);
    }
    let controller = proxy(
        connection,
        udisks,
        drive.as_str().to_owned(),
        "org.freedesktop.UDisks2.NVMe.Controller",
    )?;
    let mut options: HashMap<&str, zbus::zvariant::Value> = HashMap::new();
    options.insert(
        "auth.no_user_interaction",
        zbus::zvariant::Value::from(true),
    );
    let attributes = match controller.call::<_, _, HashMap<String, zbus::zvariant::OwnedValue>>(
        "SmartGetAttributes",
        &(options,),
    ) {
        Ok(attributes) => attributes,
        Err(why) if absent(&why) => return Ok(Value::Null),
        Err(why) => return Err(why),
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

/// The records held of these kinds or keys, read first where they aren't
/// fresh or a key isn't held, in the daemon's order: by kind, then by key.
fn records(shared: &Shared, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    refresh(shared, kinds, keys);
    held(&lock(&shared.daemon), kinds, keys)
}

/// The kinds these kinds or keys are of.
fn wanted_kinds(kinds: &[String], keys: Option<&[String]>) -> HashSet<String> {
    match keys {
        Some(keys) => keys
            .iter()
            .filter_map(|key| kind_of(key))
            .map(str::to_owned)
            .collect(),
        None => kinds.iter().cloned().collect(),
    }
}

/// Reads the kinds of these kinds or keys that weren't read lately.
fn refresh(shared: &Shared, kinds: &[String], keys: Option<&[String]>) {
    let wanted = wanted_kinds(kinds, keys);
    let stale: HashSet<String> = {
        let daemon = lock(&shared.daemon);
        wanted
            .iter()
            .filter(|kind| {
                // A key not yet held is read for, as a tree first asked is.
                let unheld = keys.is_some_and(|keys| {
                    keys.iter().any(|key| {
                        kind_of(key) == Some(kind.as_str()) && !daemon.held.contains_key(key)
                    })
                });
                unheld
                    || daemon
                        .read_at
                        .get(*kind)
                        .is_none_or(|at| at.elapsed() > FRESH)
            })
            .cloned()
            .collect()
    };
    if !stale.is_empty() {
        read(shared, &stale, keys.unwrap_or_default());
    }
}

/// The records held of these kinds or keys, in the daemon's order.
fn held(daemon: &Daemon, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    let wanted = wanted_kinds(kinds, keys);
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

/// What one ask is answered with, or none where the answer is already
/// queued to be said.
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
                "id": id, "version": VERSION, "domain": "machine", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
            })
        }
        Ask::Get { id, keys } => {
            json!({"id": id, "records": records(shared, &[], Some(&keys))})
        }
        Ask::List { id, kinds } => json!({"id": id, "records": records(shared, &kinds, None)}),
        Ask::Watch { id, kinds, keys } => {
            refresh(shared, &kinds, keys.as_deref());
            // What it's told first is taken, and queued, under the lock
            // that holds the watch, so a read between them is either in it
            // or told after it. Queued once the lock was let go, a change
            // read in between could be said first, and the whole answer
            // after it would put back what the change moved on from.
            let mut daemon = lock(&shared.daemon);
            let snapshot = held(&daemon, &kinds, keys.as_deref());
            // A watch asked again by its id replaces the one before.
            daemon
                .watches
                .retain(|watch| !(watch.connection == connection && watch.id == id));
            // One that can't be queued is hung up on, as a change would
            // hang up on it.
            let first = json!({"id": id, "records": snapshot});
            if told.try_send(first.to_string()).is_err() {
                let _ = hangup.shutdown(Shutdown::Both);
                return None;
            }
            daemon.watches.push(Watch {
                connection,
                id: id.clone(),
                kinds,
                keys,
                told: told.clone(),
                stream: Arc::clone(hangup),
            });
            shared.woken.notify_all();
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
                "id": id, "version": VERSION, "domain": "machine", "state": "answering",
                "since": daemon.since, "sources": sources(&daemon),
                "watches": daemon.watches.len(),
                "took_us": daemon.took.iter().collect::<Vec<_>>(),
            })
        }
        Ask::Act { id, act, said } => acted(shared, &id, &act, &said),
        Ask::UnknownKind { id, kind } => {
            error(&id, "unknown-kind", &format!("no kind {kind} here"))
        }
        Ask::Unknown { id, ask } => error(&id, "unknown-ask", &format!("no ask {ask:?} here")),
    })
}

/// Does what an act asks and says it's done, or why not; the kinds it
/// moved are read again first, so a watch hears of it before the answer.
fn acted(shared: &Shared, id: &Value, act: &str, said: &Value) -> Value {
    match act {
        "charge-limit" => {
            let name = said["supply"].as_str().unwrap_or("");
            let Some(wanted) = said["stop_pct"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
            else {
                return error(id, "bad-act", "a charge limit needs its stop_pct");
            };
            let set = supply::set_limit(&lock(&shared.reader).root, name, wanted);
            if let Err(unset) = set {
                return error(id, unset.code, &unset.message);
            }
            read(shared, &HashSet::from(["supply".to_owned()]), &[]);
            json!({"id": id, "done": true})
        }
        _ => error(id, "unknown-act", &format!("no act {act:?} here")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(raw: &Value) -> Value {
        json!({"key": "light:iio:device0", "kind": "light", "device": "iio:device0",
            "name": "als", "raw": raw, "scale": 0.001, "offset": 0.0})
    }

    // The shell takes a watch's whole answer over what it held, so said
    // after a change read once the watch is held, it puts back what the
    // change moved on from, and nothing tells the change again: the room's
    // sensor stayed unread for the looks VM test's 30 seconds (2026-11-10).
    #[test]
    fn a_watch_is_answered_whole_before_any_change_told_it() {
        let shared = Arc::new(shared(&Root::new(std::env::temp_dir())));
        let (told, lines) = mpsc::sync_channel(QUEUED);
        let (end, _other) = UnixStream::pair().unwrap();
        let hangup = Arc::new(end);
        lock(&shared.daemon).light = vec![sensor(&Value::Null)];
        let watch = ask(r#"{"id":"all","ask":"watch","kind":["light"]}"#);
        let first = answer(&shared, watch, 1, &told, &hangup);
        // The sensor's count is read before the answer is said, as
        // light_loop reads it once the watch wakes it.
        lock(&shared.daemon).light = vec![sensor(&json!(0.0))];
        read(&shared, &HashSet::from(["light".to_owned()]), &[]);
        if let Some(first) = first {
            told.send(first.to_string()).unwrap();
        }
        let said: Vec<Value> = lines
            .try_iter()
            .map(|line| serde_json::from_str(&line).unwrap())
            .collect();
        assert_eq!(said.len(), 2, "the whole answer, then the change: {said:?}");
        assert_eq!(said[0]["records"][0]["raw"], Value::Null, "{said:?}");
        assert_eq!(said[1]["changed"][0]["raw"], json!(0.0), "{said:?}");
    }
}
