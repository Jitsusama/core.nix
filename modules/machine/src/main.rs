//! machine SOCKET [ROOT] [--words FILE] [--numbers FILE]
//!
//! How this machine is doing, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (interface.rs):
//! the processor, the memory, each disk, this boot, the programs using
//! the machine most, its sensors, thermal zones and cooling, each power
//! supply, how it sleeps, its firmware, the screen's backlight and the
//! room's light, the brightness and warmth the backlight program
//! steers the built-in screen to, who has each camera open, Keep Awake's
//! locks and the power profile (kept.rs), and the keyboards connected,
//! caps lock, a board's layer and what key-counts has tallied of each
//! keyboard, each a record whose figures are
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
//! so a program opening a camera is said as it does, and the keyboards'
//! watchers' files, so caps lock is lit on the screen as it is on the
//! board. Keep Awake's files and the
//! machine's chosen profile are heard as they're written, and
//! power-profiles-daemon as it says it changed, from the daemon's start,
//! since the shell watches them while the launcher's shut: neither is
//! read on a clock, so watching them wakes nothing until they change.
//!
//! Heat's, Power's and Firmware's rows and headlines are made here too,
//! ready for the launcher (`heat_rows.rs`, `power_rows.rs`,
//! `firmware_rows.rs`), in the launcher's words
//! (`--words`, its `words.json`) and number settings (`--numbers`,
//! `{ group, point }`): a watch of kind `row` or `head` with `place`
//! `heat` has the sensors, zones, cooling and processor read as their
//! own watch would, one with `place` `power` the supplies, and one with
//! `place` `firmware` fwupd, and one with `place` `machine` the processor,
//! the memory and the boot (`machine_rows.rs`, `processor_rows.rs`), and
//! each read makes that place's rows again and tells only those that
//! changed. Machine's are made again as the programs using them are read
//! too, though watching its rows doesn't read them, since that reads
//! every process's files: the shell watches `using` while it's looked at.
//! The processor's own telemetry is asked only while Heat's rows are
//! watched, or `telemetry` is, as the shell watches it while a Processor
//! answer can be seen; Machine's rows are made again with each of its
//! seconds then, for what held the processor back and kept it from
//! resting (`telemetry_rows.rs`).
//! Handed no words, it makes no rows.
//!
//! It acts as well as answers, so the shell spawns and writes nothing of
//! its own: `charge-limit` moves a battery's charge limit, and the supply
//! is read again before the act is answered, so each watch is told the
//! new limit first. The firmware's acts (acts.rs) can wait minutes on
//! fwupd or a polkit prompt, so each goes on a thread of its own, one at
//! a time, and is answered once it's done, the connection answering
//! everything else meanwhile; fwupd is asked again as each ends.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use machine::interface::{Ask, KINDS, MADE, VERSION, ask, error, kind_of, place_of};
use machine::words::Words;
use machine::{
    Root, acts, boot, camera, clock_ms, counts, disk, firmware, firmware_rows, heat, heat_rows,
    kept, keyboards, light, machine_rows, memory, now_ms, power_rows, processor, rounded, rows,
    sleep, status, steered, supply, telemetry, tree, units, using, written::Written,
};
use ready_rows::kept::Kept;
use serde_json::{Value, json};

/// How often what's watched is read.
const EVERY: Duration = Duration::from_secs(1);
/// How soon after a read whose rates aren't of now it's read again.
const SOON: Duration = Duration::from_millis(200);
/// How long a firmware act that says no patience of its own is waited
/// for: a polkit prompt left unanswered, not a change still going in.
const ACT_PATIENCE: Duration = Duration::from_secs(120);
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
const KEYBOARDS: [&str; 2] = ["keyboards", "capslock"];
const HEARD: [&str; 8] = [
    "brightness",
    "warmth",
    "camera",
    "keyboards",
    "capslock",
    "layer",
    "counts",
    "status",
];
/// How often a watch on files heard looks to see whether it's still
/// wanted, once every folder is watched.
const RESTING: Duration = Duration::from_secs(60);
/// The kinds read as their files are written, their bus says they
/// changed or their reader tells a second (`kept_loop`, `profile_loop`,
/// `telemetry_loop`).
const KEPT: [&str; 3] = ["kept", "profile", "telemetry"];
/// How long `kept_loop` waits on its files before looking again for a
/// folder that isn't there, as `/run/power-profile` isn't while its
/// service starts again.
const KEPT_LOOKS_AGAIN: Duration = Duration::from_secs(10);
/// A place whose rows and headline are made here, and the kinds they're
/// made of, read together while they're watched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Place {
    name: &'static str,
    sources: &'static [&'static str],
    /// Kinds whose reads make its rows again, though watching them
    /// doesn't read them.
    heard: &'static [&'static str],
}

impl Place {
    /// Its headline's key.
    fn head(self) -> String {
        format!("head:{}", self.name)
    }

    fn holds(self, key: &str) -> bool {
        place_of(key) == Some(self.name)
    }
}

const HEAT: Place = Place {
    name: "heat",
    sources: &["sensor", "zone", "cooling", "processor"],
    heard: &[],
};
const POWER: Place = Place {
    name: "power",
    sources: &["supply"],
    heard: &[],
};
const FIRMWARE: Place = Place {
    name: "firmware",
    sources: &["firmware"],
    heard: &[],
};
const MACHINE: Place = Place {
    name: "machine",
    sources: &["processor", "memory", "boot"],
    heard: &["using", "telemetry"],
};
const PLACES: [Place; 4] = [HEAT, POWER, FIRMWARE, MACHINE];
/// How lately the programs must have been read for Machine's rows to say
/// them: two reads' time, so one read a little late doesn't lose them.
const USING_FRESH: Duration = Duration::from_secs(2);
/// How long to wait before asking the processor's telemetry again once
/// it couldn't be reached, as on a machine with none.
const TELEMETRY_AGAIN: Duration = Duration::from_secs(30);
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
    /// The places whose rows and headline it watches, or none for every
    /// place's.
    places: Option<Vec<String>>,
    told: SyncSender<String>,
    /// Its connection, to hang up on once it stops reading.
    stream: Arc<UnixStream>,
}

impl Watch {
    fn wants(&self, key: &str) -> bool {
        let named = match &self.keys {
            Some(keys) => keys.iter().any(|k| k == key),
            None => kind_of(key).is_some_and(|kind| self.kinds.iter().any(|k| k == kind)),
        };
        named && placed(key, self.places.as_deref())
    }

    /// Whether it watches any of a place's rows or its headline.
    fn wants_place(&self, place: Place) -> bool {
        match &self.keys {
            Some(keys) => keys.iter().any(|key| place.holds(key)),
            None => self.wants(&format!("row:{}:", place.name)) || self.wants(&place.head()),
        }
    }
}

/// Whether a key is of one of these places, or of none, as a record read
/// rather than made is.
fn placed(key: &str, places: Option<&[String]>) -> bool {
    place_of(key).is_none_or(|place| places.is_none_or(|places| places.iter().any(|p| p == place)))
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
    /// Where a board's watcher says its layer, or none for a session with
    /// no runtime folder.
    layer: Option<String>,
    /// The watchers whose status files are read, and the heartbeat each
    /// was last heard at.
    watchers: Vec<status::Watcher>,
    statuses: status::Kept,
}

impl Reader {
    /// Reads the kinds of these that are heard as their files are
    /// written (`HEARD`).
    fn heard(&mut self, kinds: &HashSet<String>) -> Vec<Value> {
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
        if wants("keyboards") {
            records.push(keyboards::connected(&self.root));
        }
        if wants("capslock") {
            records.push(keyboards::capslock(&self.root));
        }
        if wants("layer") {
            records.extend(keyboards::layer(&self.root, self.layer.as_deref()));
        }
        if wants("counts") {
            records.extend(counts::boards(&self.root));
        }
        if wants("status") {
            records.extend(self.statuses.records(
                &self.root,
                &self.watchers,
                Instant::now(),
                status::since_epoch(),
            ));
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
        if wants("telemetry") {
            records.push(telemetry::record(buses.telemetry));
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
    /// How the processor's telemetry stands, as `telemetry:processor`
    /// says it: asking, heard, refused or missing.
    telemetry: &'static str,
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
    /// The launcher's words, which rows are said in, or none to make none.
    words: Option<Words>,
    /// The reads Heat's sparks keep while its rows are watched.
    heat: heat_rows::Heat,
    /// The reads Power's sparks keep while its rows are watched.
    power: power_rows::Power,
    /// How Firmware's acts stand: the one going, so another waits its
    /// turn, and how each ended while its rows are watched.
    acting: firmware_rows::Acting,
    /// Each place's rows and headline as last told, by the place's name,
    /// so each row keeps its order and one coming or going sends itself
    /// alone, where ordering siblings by where they stood sent every one
    /// after it again (FEH1HE, 2026-11-10).
    kept: BTreeMap<&'static str, Kept>,
    /// What the processor's telemetry last told, while it's asked.
    telemetry: telemetry::Heard,
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
    let mut named = Vec::new();
    let (mut words, mut numbers) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--words" => words = args.next(),
            "--numbers" => numbers = args.next(),
            _ => named.push(arg),
        }
    }
    let mut named = named.into_iter();
    let Some(socket) = named.next() else {
        eprintln!("usage: machine SOCKET [ROOT] [--words FILE] [--numbers FILE]");
        std::process::exit(64);
    };
    let root = Root::new(named.next().unwrap_or_else(|| "/".to_owned()));
    let socket = PathBuf::from(socket);
    let words = worded(words.as_deref(), numbers.as_deref());
    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(why) => {
            eprintln!("<3>couldn't listen on {}: {why}", socket.display());
            std::process::exit(71);
        }
    };
    let shared = Arc::new(shared(&root, words));
    {
        let shared = Arc::clone(&shared);
        let path = root.path(telemetry::PATH);
        std::thread::spawn(move || telemetry_loop(&shared, &path));
    }
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
    hear_keyboards_and_watchers(&shared, &root);
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

/// The launcher's words, from its words.json at `words` and its number
/// settings at `numbers`, or none, said in the journal, where it was
/// handed none or they can't be read.
fn worded(words: Option<&str>, numbers: Option<&str>) -> Option<Words> {
    let read = |path: &str| {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    };
    let Some(vocabulary) = words.and_then(read) else {
        eprintln!("<5>handed no words it can read, so it makes no rows");
        return None;
    };
    let numbers = numbers.and_then(read).unwrap_or(Value::Null);
    let mark = |name: &str| numbers[name].as_str().unwrap_or("").to_owned();
    Some(Words::new(&vocabulary, &mark("group"), &mark("point")))
}

/// Hears the keyboards' lights, the layer and the key counts, and the
/// watchers' status, each as its files are written.
fn hear_keyboards_and_watchers(shared: &Arc<Shared>, root: &Root) {
    {
        let shared = Arc::clone(shared);
        let folders = vec![root.path(keyboards::LIGHTS)];
        std::thread::spawn(move || heard_loop(&shared, &KEYBOARDS, &folders, keyboards::names));
    }
    {
        let shared = Arc::clone(shared);
        let folders = vec![root.path(counts::FOLDER)];
        std::thread::spawn(move || heard_loop(&shared, &["counts"], &folders, counts::names));
    }
    if let Some(folder) = keyboards::layer_file()
        .as_deref()
        .and_then(|file| std::path::Path::new(file).parent())
        .map(|folder| root.path(&folder.to_string_lossy()))
    {
        let shared = Arc::clone(shared);
        let folders = vec![folder];
        std::thread::spawn(move || {
            heard_loop(&shared, &["layer"], &folders, keyboards::names_the_layer);
        });
    }
    let shared = Arc::clone(shared);
    let folders = lock(&shared.reader)
        .watchers
        .iter()
        .map(|watcher| root.path(&watcher.folder))
        .collect::<Vec<_>>();
    std::thread::spawn(move || {
        let due = |shared: &Shared| {
            let reader = lock(&shared.reader);
            reader.statuses.due(&reader.watchers)
        };
        heard_until(&shared, &["status"], &folders, status::names, due);
    });
}

/// What every thread shares, holding nothing yet, reading under `root`
/// and saying rows in `words`.
fn shared(root: &Root, words: Option<Words>) -> Shared {
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
            words,
            heat: heat_rows::Heat::default(),
            power: power_rows::Power::default(),
            acting: firmware_rows::Acting::default(),
            kept: BTreeMap::new(),
            telemetry: telemetry::Heard::default(),
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
            layer: keyboards::layer_file(),
            watchers: status::of_session(),
            statuses: status::Kept::default(),
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

/// Whether some watch holds a place's rows or its headline.
fn makes(daemon: &Daemon, place: Place) -> bool {
    daemon.words.is_some() && daemon.watches.iter().any(|watch| watch.wants_place(place))
}

/// The places whose rows some watch holds.
fn making(daemon: &Daemon) -> Vec<Place> {
    PLACES
        .into_iter()
        .filter(|place| makes(daemon, *place))
        .collect()
}

/// The kinds read for these: each as it is, but a made kind, which is
/// read as what these places' rows are made of.
fn sources_of(kinds: &HashSet<String>, places: &[Place]) -> HashSet<String> {
    let mut read: HashSet<String> = kinds
        .iter()
        .filter(|kind| !MADE.contains(&kind.as_str()))
        .cloned()
        .collect();
    for place in places {
        read.extend(place.sources.iter().map(|kind| (*kind).to_owned()));
    }
    read
}

/// The kinds read while what's watched is: those watched, and what the
/// places watched are made of, as fwupd is asked for Firmware's rows.
fn reads(daemon: &Daemon) -> HashSet<String> {
    sources_of(&watched(daemon), &making(daemon))
}

/// Lets a place's reads go once nothing watches its rows, so a shut
/// launcher keeps none, and the next watch starts afresh.
fn let_go(daemon: &mut Daemon) {
    for place in PLACES {
        if makes(daemon, place) {
            continue;
        }
        if place == HEAT {
            daemon.heat.forget();
        } else if place == POWER {
            daemon.power.forget();
        } else {
            // How an act ended is said until the launcher shuts, as
            // Firmware kept it; one still going is kept.
            daemon.acting.ended.clear();
        }
        daemon.kept.remove(place.name);
        daemon.held.retain(|key, _| !place.holds(key));
    }
    if !telemetry_asked(daemon) {
        daemon.telemetry.dropped();
    }
}

/// Whether the processor's telemetry is asked: while Heat's rows are
/// watched, for its temperatures, or `telemetry` is, for what held the
/// processor back and kept it from resting.
fn telemetry_asked(daemon: &Daemon) -> bool {
    makes(daemon, HEAT) || watched(daemon).contains("telemetry")
}

/// The records of one kind held.
fn held_of(held: &BTreeMap<String, Value>, kind: &str) -> Vec<Value> {
    held.iter()
        .filter(|(key, _)| kind_of(key) == Some(kind))
        .map(|(_, record)| record.clone())
        .collect()
}

/// A place's rows and headline as they read now, from what's held.
fn ready(daemon: &mut Daemon, place: Place) -> rows::Ready {
    let asked = telemetry_asked(daemon);
    let Daemon {
        held,
        heat,
        power,
        acting,
        words,
        telemetry,
        read_at,
        ..
    } = daemon;
    let Some(words) = words.as_ref() else {
        return rows::Ready::default();
    };
    if place == POWER {
        return power.read(&held_of(held, "supply"), clock_ms(), words);
    }
    if place == FIRMWARE {
        return firmware_rows::read(&held_of(held, "firmware"), acting, words);
    }
    if place == MACHINE {
        let using = held_of(held, "using");
        let fresh = read_at
            .get("using")
            .is_some_and(|at| at.elapsed() < USING_FRESH);
        let memory = held.get("memory:main");
        let boot = held_of(held, "boot").into_iter().next();
        let said = machine_rows::Held {
            processor: held.get("processor:0"),
            memory,
            boot: boot.as_ref(),
            using: fresh.then_some(using.as_slice()),
            telemetry: asked.then_some(&*telemetry),
        };
        return machine_rows::read(said, clock_ms(), words);
    }
    let (sensors, zones, cooling) = (
        held_of(held, "sensor"),
        held_of(held, "zone"),
        held_of(held, "cooling"),
    );
    let processor = held_of(held, "processor").into_iter().next();
    let temperatures = telemetry.temperatures();
    let said = heat_rows::Held {
        sensors: &sensors,
        zones: &zones,
        cooling: &cooling,
        processor: processor.as_ref(),
        telemetry: &temperatures,
    };
    heat.read(said, clock_ms(), words)
}

/// Makes a place's rows and headline again from what's held, keeps them,
/// and says those that changed and the keys of those gone.
fn made(daemon: &mut Daemon, place: Place) -> (Vec<Value>, Vec<String>) {
    let ready = ready(daemon, place);
    let head = ready
        .head
        .map(|head| json!({"key": place.head(), "kind": "head", "head": head}));
    let moved = daemon
        .kept
        .entry(place.name)
        .or_default()
        .take(ready.rows, head.into_iter().collect());
    for key in &moved.gone {
        daemon.held.remove(key);
    }
    for record in &moved.changed {
        if let Some(key) = record["key"].as_str() {
            daemon.held.insert(key.to_owned(), record.clone());
        }
    }
    (moved.changed, moved.gone)
}

/// Asks the processor's telemetry at `path` while it's asked
/// (`telemetry_asked`), and nothing otherwise, since the reader reads the
/// processor's counters only while asked; asked again a second after it
/// goes, or later where it was never reached, unless a watch begins or
/// ends meanwhile, so a reader that came since is heard as it's next
/// shown rather than half a minute on. Each second it tells is
/// read as `telemetry`, so its standing is told as it changes and
/// Machine's rows are made again where they're watched.
fn telemetry_loop(shared: &Shared, path: &std::path::Path) {
    let kinds = HashSet::from(["telemetry".to_owned()]);
    loop {
        {
            let mut daemon = lock(&shared.daemon);
            while !telemetry_asked(&daemon) {
                daemon = shared
                    .woken
                    .wait(daemon)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
        }
        let stop = AtomicBool::new(false);
        let reached = telemetry::watch(path, &stop, |said| {
            {
                let mut daemon = lock(&shared.daemon);
                // Once nothing asks, what it said was let go, and its
                // last word, the connection ending, is no refusal.
                if !telemetry_asked(&daemon) {
                    stop.store(true, Ordering::Relaxed);
                    return;
                }
                daemon.telemetry.heard(said);
            }
            read(shared, &kinds, &[]);
        });
        let again = if reached { EVERY } else { TELEMETRY_AGAIN };
        drop(
            shared
                .woken
                .wait_timeout(lock(&shared.daemon), again)
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        );
    }
}

/// Reads these kinds now, the trees watched and those `asked` names among
/// them, keeps what changed, and tells each watch of it. A place some
/// watch holds the rows of has them made again where what they're made
/// of was read.
fn read(shared: &Shared, kinds: &HashSet<String>, asked: &[String]) {
    read_making(shared, kinds, asked, &[]);
}

/// Reads as `read` does, and makes these places' rows too, though no
/// watch holds them yet, as a watch of them is answered before it's held.
fn read_making(shared: &Shared, kinds: &HashSet<String>, asked: &[String], places: &[Place]) {
    let kinds = &sources_of(kinds, places);
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
            telemetry: daemon.telemetry.standing().name(),
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
    let mut gone = gone;
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
    for place in PLACES {
        let read = place
            .sources
            .iter()
            .chain(place.heard)
            .any(|kind| kinds.contains(*kind));
        let wanted = places.contains(&place) || makes(&daemon, place);
        if read && wanted && daemon.words.is_some() {
            let (remade, unmade) = made(&mut daemon, place);
            changed.extend(remade);
            gone.extend(unmade);
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
        let_go(&mut daemon);
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
        // A place's rows are read for only where some watch holds them,
        // as rows of another place are made by another daemon.
        let places = making(&daemon);
        let kinds = sources_of(&kinds, &places);
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
        // What a place's rows are made of is read together, so each read
        // makes them once.
        for place in &places {
            if place.sources.iter().any(|kind| due.contains(*kind)) {
                due.extend(place.sources.iter().map(|kind| (*kind).to_owned()));
            }
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
        let watching = reads(&daemon).contains(kind);
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
        if !reads(&daemon).contains(kind) {
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
/// backlight program's, the cameras' holders', and the keyboards'
/// watchers'. They're read at no
/// other time but a watch's first answer and a get. They're read once more
/// as the watch on them begins, so one written since the first answer is
/// said all the same. With the last watch the files are let be, so an
/// idle machine is woken by none of their writes.
fn heard_loop(shared: &Shared, kinds: &[&str], folders: &[PathBuf], ours: impl Fn(&str) -> bool) {
    heard_until(shared, kinds, folders, ours, |_| None);
}

/// As `heard_loop`, and read again too when `due` says, as a watcher's
/// status is when its heartbeat is late, though nothing was written.
fn heard_until(
    shared: &Shared,
    kinds: &[&str],
    folders: &[PathBuf],
    ours: impl Fn(&str) -> bool,
    due: impl Fn(&Shared) -> Option<Instant>,
) {
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
            let resting = if written.looking() { EVERY } else { RESTING };
            let wait = due(shared).map_or(resting, |at| {
                resting.min(at.saturating_duration_since(Instant::now()))
            });
            let heard = written.wait(wait);
            let late = due(shared).is_some_and(|at| at <= Instant::now());
            if late || heard.iter().any(|name| name.is_empty() || ours(name)) {
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
    // A file written after a watch was answered but before the kernel's
    // watch began would be heard by neither, so what's there is read once
    // now, telling only the kinds watched: a test that keeps itself awake
    // just after watching waited the whole of its patience for it.
    heard(shared, &KEPT);
    loop {
        let names = written.wait(KEPT_LOOKS_AGAIN);
        let locks = names
            .iter()
            .any(|name| name.is_empty() || kept::names_a_lock(name));
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
    let mut daemon = lock(&shared.daemon);
    daemon
        .watches
        .retain(|watch| watch.connection != connection);
    let_go(&mut daemon);
}

/// The records held of these kinds or keys, read first where they aren't
/// fresh or a key isn't held, in the daemon's order: by kind, then by key.
fn records(shared: &Shared, kinds: &[String], keys: Option<&[String]>) -> Vec<Value> {
    refresh(shared, kinds, keys, None);
    held(&lock(&shared.daemon), kinds, keys, None)
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

/// The places whose rows these kinds or keys ask for, of these places or
/// every place's.
fn asked_places(
    kinds: &[String],
    keys: Option<&[String]>,
    places: Option<&[String]>,
) -> Vec<Place> {
    if !wanted_kinds(kinds, keys)
        .iter()
        .any(|kind| MADE.contains(&kind.as_str()))
    {
        return Vec::new();
    }
    PLACES
        .into_iter()
        .filter(|place| match keys {
            Some(keys) => keys.iter().any(|key| place.holds(key)),
            None => places.is_none_or(|places| places.iter().any(|p| p == place.name)),
        })
        .collect()
}

/// Reads the kinds of these kinds or keys that weren't read lately, and
/// what a place's rows are made of where they're asked for and not yet
/// made.
fn refresh(shared: &Shared, kinds: &[String], keys: Option<&[String]>, places: Option<&[String]>) {
    let asked = wanted_kinds(kinds, keys);
    let mut made = asked_places(kinds, keys, places);
    let stale: HashSet<String> = {
        let daemon = lock(&shared.daemon);
        if daemon.words.is_none() {
            made.clear();
        }
        let wanted = sources_of(&asked, &made);
        // A place whose headline isn't held has never been made.
        let unmade: Vec<&str> = made
            .iter()
            .filter(|place| !daemon.held.contains_key(&place.head()))
            .flat_map(|place| place.sources.iter().copied())
            .collect();
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
                    || unmade.contains(&kind.as_str())
                    || daemon
                        .read_at
                        .get(*kind)
                        .is_none_or(|at| at.elapsed() > FRESH)
            })
            .cloned()
            .collect()
    };
    // Read for rows not yet watched, they're made though no watch holds
    // them yet.
    if !stale.is_empty() {
        read_making(shared, &stale, keys.unwrap_or_default(), &made);
    }
}

/// The records held of these kinds or keys, and of the rows these
/// places' alone, or every place's, in the daemon's order.
fn held(
    daemon: &Daemon,
    kinds: &[String],
    keys: Option<&[String]>,
    places: Option<&[String]>,
) -> Vec<Value> {
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
            .filter(|(key, _)| placed(key, places))
            .map(|(_, record)| record.clone())
            .collect(),
    };
    // By the kind its key names, as a row's own kind is its thing's.
    if keys.is_none() {
        found.sort_by_key(|record| {
            let kind = record["key"].as_str().and_then(kind_of).unwrap_or("");
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
        Ask::Watch {
            id,
            kinds,
            keys,
            places,
        } => {
            refresh(shared, &kinds, keys.as_deref(), places.as_deref());
            // What it's told first is taken, and queued, under the lock
            // that holds the watch, so a read between them is either in it
            // or told after it. Queued once the lock was let go, a change
            // read in between could be said first, and the whole answer
            // after it would put back what the change moved on from.
            let mut daemon = lock(&shared.daemon);
            let snapshot = held(&daemon, &kinds, keys.as_deref(), places.as_deref());
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
                places,
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
            let_go(&mut daemon);
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
        Ask::Act { id, act, said } if act.starts_with("firmware-") => {
            firmware_act(shared, id, act, said, told.clone());
            return None;
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
        "restart-unit" => match units::restart(said["unit"].as_str().unwrap_or("")) {
            Ok(()) => json!({"id": id, "done": true}),
            Err(unrestarted) => error(id, unrestarted.code, &unrestarted.message),
        },
        _ => error(id, "unknown-act", &format!("no act {act:?} here")),
    }
}

/// Does a firmware act on a thread of its own and tells how it went once
/// it's done, waiting at most the `patience_s` it's given; another asked
/// while one goes is refused as `busy`, since fwupd puts in one thing at
/// a time. fwupd is asked again as it ends, whatever it said.
fn firmware_act(
    shared: &Arc<Shared>,
    id: Value,
    act: String,
    said: Value,
    told: SyncSender<String>,
) {
    let text = |name: &str| said[name].as_str().unwrap_or("").to_owned();
    let key = match act.as_str() {
        "firmware-install" => format!("install:{}", text("device")),
        "firmware-fix" => format!("fix:{}", text("attribute")),
        "firmware-undo" => format!("undo:{}", text("attribute")),
        _ => "setup".to_owned(),
    };
    {
        let mut daemon = lock(&shared.daemon);
        if daemon.acting.going.is_some() {
            let busy = error(&id, "busy", "another firmware act is going");
            let _ = told.send(busy.to_string());
            return;
        }
        daemon.acting.going = Some(key.clone());
        daemon.acting.ended.remove(&key);
    }
    // Firmware's rows say it's going before anything else is said.
    let firmware = HashSet::from(["firmware".to_owned()]);
    read(shared, &firmware, &[]);
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let patience = said["patience_s"]
            .as_u64()
            .map_or(ACT_PATIENCE, Duration::from_secs);
        let text = |name: &str| said[name].as_str().unwrap_or("");
        let done = match act.as_str() {
            "firmware-install" => {
                let fwupdmgr = std::env::var(acts::FWUPDMGR).unwrap_or_default();
                acts::install(&fwupdmgr, text("device"), patience)
            }
            "firmware-fix" => acts::change(text("attribute"), false, patience),
            "firmware-undo" => acts::change(text("attribute"), true, patience),
            "firmware-setup" => acts::setup(patience),
            _ => Err(acts::refused("unknown-act", format!("no act {act:?} here"))),
        };
        {
            let mut daemon = lock(&shared.daemon);
            daemon.acting.going = None;
            // How it ended is said while Firmware's rows are watched, and
            // a launcher opened after has nothing to be told of it.
            if makes(&daemon, FIRMWARE) {
                let ended = match &done {
                    Ok(()) => firmware_rows::Ended::Done,
                    Err(refused) if refused.code == "stopped" => {
                        firmware_rows::Ended::Stopped(patience.as_secs().div_ceil(60))
                    }
                    Err(refused) => firmware_rows::Ended::Refused(refused.message.clone()),
                };
                daemon.acting.ended.insert(key, ended);
            }
            daemon.hurried.insert("firmware".to_owned());
        }
        shared.woken.notify_all();
        read(&shared, &firmware, &[]);
        let answer = match done {
            Ok(()) => json!({"id": id, "done": true}),
            Err(refused) => error(&id, refused.code, &refused.message),
        };
        let _ = told.send(answer.to_string());
    });
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
        let shared = Arc::new(shared(&Root::new(std::env::temp_dir()), None));
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

    // Firmware's rows say an act is going while it goes, and how it
    // ended once it has, before the act is answered: here an install on a
    // machine with no fwupdmgr, which is refused at once.
    #[test]
    fn firmwares_rows_say_an_act_going_then_how_it_ended() {
        let vocabulary = json!({ "things": [
            { "id": "firmware", "name": "Firmware", "said": { "one": "1 update ready" } },
            { "id": "verb:firmware:install", "name": "Install {version}",
                "sure": "Install {version} on {device} Now", "said": {
                "installing": "installing", "failed": "didn't install" } }
        ] });
        let words = Words::new(&vocabulary, ",", ".");
        let root = Root::new(std::env::temp_dir());
        let shared = Arc::new(shared(&root, Some(words)));
        lock(&shared.daemon).firmware = vec![json!({ "key": "firmware:b10",
            "kind": "firmware", "id": "b10", "name": "System Firmware",
            "version": "1.12.0", "update": { "version": "1.13.0" } })];
        let (told, lines) = mpsc::sync_channel(QUEUED);
        let (end, _other) = UnixStream::pair().unwrap();
        let hangup = Arc::new(end);
        let watch = ask(r#"{"id":"f","ask":"watch","kind":["row"],"place":"firmware"}"#);
        // A watch's whole answer is queued as it's held, not returned.
        let _ = answer(&shared, watch, 1, &told, &hangup);
        let first: Value = serde_json::from_str(&lines.try_recv().unwrap()).unwrap();
        let records = first["records"].as_array().into_iter().flatten();
        let keys: Vec<&Value> = records.map(|record| &record["key"]).collect();
        assert!(
            keys.contains(&&json!("row:firmware:b10:install")),
            "Install is offered: {first}"
        );
        let said = json!({ "device": "b10", "patience_s": 60 });
        firmware_act(
            &shared,
            json!(1),
            "firmware-install".to_owned(),
            said,
            told.clone(),
        );
        let mut notes = Vec::new();
        loop {
            let line = lines
                .recv_timeout(Duration::from_secs(10))
                .expect("the act is answered");
            let line: Value = serde_json::from_str(&line).unwrap();
            if line["id"] == 1 {
                assert_eq!(line["error"]["code"], "bad-act", "{line}");
                break;
            }
            let changed = line["changed"].as_array().into_iter().flatten();
            notes.extend(
                changed
                    .filter(|record| record["key"] == "row:firmware:b10:install")
                    .map(|record| record["row"]["note"].clone()),
            );
        }
        assert_eq!(
            notes,
            [
                "installing",
                "didn't install \u{b7} no fwupdmgr on this machine"
            ],
            "going, then how it ended, before the answer"
        );
        assert!(
            lock(&shared.daemon).acting.going.is_none(),
            "nothing is going"
        );
    }
}
