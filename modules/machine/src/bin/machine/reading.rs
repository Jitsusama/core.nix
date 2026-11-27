//! The reads: the counters each kind's rates are measured from, a read
//! of the kinds asked with what the buses last said, and the loops that
//! read what's watched on the second and the processor's telemetry as it
//! tells, and wait for a kind's time to come round; and the state they
//! read into, made at the start.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::making::{
    hand_over, made, makes, making, reads, sources_of, telemetry_asked, watched, watched_trees,
};
use crate::{Daemon, EVERY, KEPT, PLACES, Place, Shared, lock};
use daemon_serve::serve::Shared as Served;
use machine::interface::kind_of;
use machine::words::Words;
use machine::{
    Root, boot, camera, clock_ms, counts, disk, disk_rows, firmware_rows, heat, heat_rows, kept,
    keyboards, light, links, memory, power_rows, processor, sleep, spent::Spent, status, steered,
    supply, telemetry, tree, units, using,
};
use serde_json::{Value, json};

/// How soon after a read whose rates aren't of now it's read again.
const SOON: Duration = Duration::from_millis(200);
/// How near its time a kind is read with others due now.
const LEEWAY: Duration = Duration::from_millis(50);
const HEARD: [&str; 9] = [
    "link",
    "brightness",
    "warmth",
    "camera",
    "keyboards",
    "capslock",
    "layer",
    "counts",
    "status",
];
/// How long to wait before asking the processor's telemetry again once
/// it couldn't be reached, as on a machine with none or while its reader
/// restarts. A refused connection costs one call, so a few seconds keeps
/// Heat's own parts from going missing for long after a switch.
const TELEMETRY_AGAIN: Duration = Duration::from_secs(5);
/// How many of the last reads' times health says.
const TIMES: usize = 32;

/// The counters each kind's rates are measured from, and what's fixed.
pub(crate) struct Reader {
    pub(crate) root: Root,
    fixed: processor::Fixed,
    page_b: u64,
    processor: Option<processor::Counters>,
    /// The processor's last record, said again while a read comes too
    /// soon to measure, since a kind read that says nothing of a key is
    /// taken to have let it go.
    processor_said: Option<Value>,
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
    pub(crate) runtime: Option<String>,
    /// Where a board's watcher says its layer, or none for a session with
    /// no runtime folder.
    layer: Option<String>,
    /// The watchers whose status files are read, and the heartbeat each
    /// was last heard at.
    pub(crate) watchers: Vec<status::Watcher>,
    pub(crate) statuses: status::Kept,
    /// The program behind lag-watch's last wait.
    behind: using::Behind,
    /// What each battery's charging is set to, read once a minute.
    pub(crate) charging: supply::Settings,
    /// Where who reads a camera through `PipeWire` is found, as `Shared`
    /// holds it too.
    pipewire: Arc<camera::PipeWire>,
}

impl Reader {
    /// The processor's record at `at`, or the one read just now, said
    /// again: one measured over less than a few of the kernel's ticks
    /// would say no busy share at all, and one left unsaid would be let
    /// go of.
    fn processor_record(&mut self, at: u64) -> Option<Value> {
        if processor::too_soon(self.processor.as_ref(), at) {
            return self.processor_said.clone();
        }
        let now = processor::counters(&self.root, at);
        let record = processor::record(&self.root, &self.fixed, &now, self.processor.as_ref());
        self.processor_said = Some(record.clone());
        self.processor = Some(now);
        Some(record)
    }

    /// Reads the kinds of these that are heard as their files are
    /// written (`HEARD`).
    fn heard(&mut self, kinds: &HashSet<String>, spent: &mut Spent) -> Vec<Value> {
        let wants = |kind: &str| kinds.contains(kind);
        let mut records = Vec::new();
        if let Some(files) = self.steered.as_ref() {
            if wants("brightness") {
                records.extend(steered::brightness(&self.root, files));
                spent.ended("brightness", Instant::now());
            }
            if wants("warmth") {
                records.extend(steered::warmth(&self.root, files));
                spent.ended("warmth", Instant::now());
            }
        }
        if wants("camera") {
            records.extend(camera::cameras(&self.root, &self.pipewire));
            spent.ended("camera", Instant::now());
        }
        if wants("link") {
            records.extend(links::links(&self.root));
            spent.ended("link", Instant::now());
        }
        if wants("keyboards") {
            records.push(keyboards::connected(&self.root));
            spent.ended("keyboards", Instant::now());
        }
        if wants("capslock") {
            records.push(keyboards::capslock(&self.root));
            spent.ended("capslock", Instant::now());
        }
        if wants("layer") {
            records.extend(keyboards::layer(&self.root, self.layer.as_deref()));
            spent.ended("layer", Instant::now());
        }
        if wants("counts") {
            records.extend(counts::boards(&self.root));
            spent.ended("counts", Instant::now());
        }
        if wants("status") {
            let mut told = self.statuses.records(
                &self.root,
                &self.watchers,
                Instant::now(),
                status::since_epoch(),
            );
            for record in &mut told {
                self.behind
                    .told(&self.root, record, clock_ms(), self.page_b);
            }
            records.extend(told);
            spent.ended("status", Instant::now());
        }
        records
    }

    /// Reads these kinds, as rates from their last read, the trees of
    /// these pids among them, with what the buses last said; and what
    /// each kind read took, in microseconds.
    fn read(
        &mut self,
        kinds: &HashSet<String>,
        trees: &BTreeSet<u32>,
        buses: &Buses,
    ) -> (Vec<Value>, BTreeMap<&'static str, u64>) {
        let health = &buses.health;
        let at = clock_ms();
        let mut records = Vec::new();
        let mut spent = Spent::from(Instant::now());
        let wants = |kind: &str| kinds.contains(kind);
        if wants("processor") {
            records.extend(self.processor_record(at));
            spent.ended("processor", Instant::now());
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
            spent.ended("memory", Instant::now());
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
            spent.ended("disk", Instant::now());
        }
        if wants("boot") {
            records.push(boot::record(&self.root, boot::asleep_s()));
            spent.ended("boot", Instant::now());
        }
        if wants("sensor") {
            records.extend(heat::sensors(&self.root, &mut self.sensors, Instant::now()));
            spent.ended("sensor", Instant::now());
        }
        if wants("zone") {
            records.extend(heat::zones(&self.root, &mut self.sensors, Instant::now()));
            spent.ended("zone", Instant::now());
        }
        if wants("cooling") {
            records.extend(heat::cooling(&self.root, &mut self.sensors, Instant::now()));
            spent.ended("cooling", Instant::now());
        }
        if wants("firmware") {
            records.extend(buses.firmware.iter().cloned());
            spent.ended("firmware", Instant::now());
        }
        if wants("sleep") {
            records.push(sleep::record(&self.root));
            spent.ended("sleep", Instant::now());
        }
        if wants("backlight") {
            records.extend(light::backlights(&self.root));
            spent.ended("backlight", Instant::now());
        }
        if wants("light") {
            records.extend(buses.light.iter().cloned());
            spent.ended("light", Instant::now());
        }
        records.extend(self.heard(kinds, &mut spent));
        if wants("awake") {
            records.push(sleep::awake(&buses.logind));
            spent.ended("awake", Instant::now());
        }
        if wants("kept") {
            records.extend(kept::locks(&self.root, self.runtime.as_deref()));
            spent.ended("kept", Instant::now());
        }
        if wants("profile") {
            records.push(kept::profile(&self.root, &buses.profile));
            spent.ended("profile", Instant::now());
        }
        if wants("telemetry") {
            records.push(telemetry::record(buses.telemetry));
            spent.ended("telemetry", Instant::now());
        }
        if wants("supply") {
            records.extend(self.supplies(&buses.upower));
            spent.ended("supply", Instant::now());
        }
        if wants("using") || (wants("tree") && !trees.is_empty()) {
            records.extend(self.programs_and_trees(wants("using"), wants("tree"), trees, at));
            spent.ended("using", Instant::now());
        }
        (records, spent.by_kind())
    }

    /// Each power supply's record, with what `UPower` last said of it.
    fn supplies(&mut self, upower: &BTreeMap<String, Value>) -> Vec<Value> {
        supply::supplies(&self.root)
            .into_iter()
            .map(|name| {
                let said = upower.get(&name).cloned().unwrap_or(Value::Null);
                supply::record(&self.root, &name, &said, &mut self.charging, Instant::now())
            })
            .collect()
    }

    /// The programs' records where `using` asks them, and these pids'
    /// trees where `trees_too` says. They share one read of every process, and each measures its
    /// rates from whichever read came last, so their time is one kind's,
    /// `using`.
    fn programs_and_trees(
        &mut self,
        using: bool,
        trees_too: bool,
        trees: &BTreeSet<u32>,
        at: u64,
    ) -> Vec<Value> {
        let mut records = Vec::new();
        let now = using::counters(&self.root, at);
        if using {
            records.extend(using::records(&now, self.programs.as_ref(), self.page_b));
        }
        if trees_too && !trees.is_empty() {
            records.extend(tree::records(
                trees,
                &now,
                self.programs.as_ref(),
                self.page_b,
            ));
        }
        self.programs = Some(now);
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

/// What every thread shares, holding nothing yet, reading under `root`,
/// saying rows in `words` and serving through `served`.
pub(crate) fn shared(root: &Root, words: Option<Words>, served: Arc<Served>) -> Shared {
    // SAFETY: sysconf only reads a constant of the running system.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let pipewire = Arc::new(camera::PipeWire::from_environment(root));
    Shared {
        daemon: Mutex::new(Daemon {
            held: BTreeMap::new(),
            read_at: HashMap::new(),
            served,
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
            disk: disk_rows::Disk::default(),
            acting: firmware_rows::Acting::default(),
            kept: BTreeMap::new(),
            telemetry: telemetry::Heard::default(),
            hurried: HashSet::new(),
            slept: Arc::new(Mutex::new(Value::Null)),
            took: VecDeque::new(),
            kinds_took: VecDeque::new(),
        }),
        reader: Mutex::new(Reader {
            fixed: processor::fixed(root),
            root: root.clone(),
            page_b: u64::try_from(page).unwrap_or(4096),
            processor: None,
            processor_said: None,
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
            statuses: status::Kept::asking(units::standing),
            behind: using::Behind::default(),
            charging: supply::Settings::default(),
            pipewire: Arc::clone(&pipewire),
        }),
        woken: Condvar::new(),
        pipewire,
    }
}

/// Asks the processor's telemetry at `path` while it's asked
/// (`telemetry_asked`), and nothing otherwise, since the reader reads the
/// processor's counters only while asked; asked again a second after it
/// goes, or later where it was never reached, unless a watch begins or
/// ends meanwhile, so a reader that came since is heard as it's next
/// shown rather than half a minute on. Each second it tells is
/// read as `telemetry`, so its standing is told as it changes and
/// Machine's rows are made again where they're watched.
pub(crate) fn telemetry_loop(shared: &Shared, path: &std::path::Path) {
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
pub(crate) fn read(shared: &Shared, kinds: &HashSet<String>, asked: &[String]) {
    read_making(shared, kinds, asked, &[]);
}

/// Reads as `read` does, and makes these places' rows too, though no
/// watch holds them yet, as a watch of them is answered before it's held.
pub(crate) fn read_making(
    shared: &Shared,
    kinds: &HashSet<String>,
    asked: &[String],
    places: &[Place],
) {
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
    // The registry's listing counts in the read's whole time, though in
    // none of its kinds', so a slow PipeWire still shows as a slow read.
    let started = Instant::now();
    if kinds.contains("camera") {
        shared.pipewire.refresh();
    }
    let (records, kinds_took) = lock(&shared.reader).read(kinds, &trees, &buses);
    let took = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    let mut daemon = lock(&shared.daemon);
    daemon.took.push_back(took);
    daemon.kinds_took.push_back(kinds_took);
    while daemon.took.len() > TIMES {
        daemon.took.pop_front();
        daemon.kinds_took.pop_front();
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
    if !changed.is_empty() || !gone.is_empty() {
        hand_over(&daemon);
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
pub(crate) fn watch_loop(shared: &Shared) {
    loop {
        let daemon = lock(&shared.daemon);
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
pub(crate) fn due(shared: &Shared, kind: &str, every: Duration, last: &mut Option<Instant>) {
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
pub(crate) fn due_while_watched(
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
