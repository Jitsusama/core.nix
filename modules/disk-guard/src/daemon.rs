//! The guard's loop, one step at a time: read the clocks, band and
//! forecast every filesystem a source lies on, and run a pass on each one
//! that needs it, journalling what it did and keeping what it learned.

use crate::anchor::Dir;
use crate::band::{Band, Space, Thresholds};
use crate::clock::Tick;
use crate::config::{Config, Source};
use crate::decide::{Decision, Fill, Situation, calm, decide, powered, spare};
use crate::dedupe::{self, Deduped};
use crate::discover;
use crate::finished::{self, Answers, Predicate};
use crate::garbage::{self, BATCH_BYTES, Batch, GC_TIMEOUT, Listed, Listing};
use crate::generations::{self, Boot};
use crate::host::{FRESH, Filesystem, Reclaim, Scan};
use crate::journal::{self, Journal, Journaled, Line};
use crate::moments::{Power, Pressure};
use crate::opens::Seen;
use crate::pace::{Monotonic, Pacer, Probe};
use crate::pass::{Disk, End, Goal, Learned, Outcome, Reclaimer, pass};
use crate::plan::{Candidate, Class};
use crate::pool::pool;
use crate::replaced::Uses;
use crate::socket::{Reply, Request, Responder};
use crate::state::{self, Memory, Owners};
use crate::survey::{Action, Offer, World, named, survey};
use crate::units::{Size, float, whole};
use crate::writers::{self, Frozen, Writers};
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// What the machine is doing, read by the caller each step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Senses {
    pub power: Power,
    /// Whether the power profile is saving power.
    pub saver: bool,
    pub pressure: Pressure,
}

/// A filesystem a source lies on, as last seen.
struct Watched {
    path: PathBuf,
    band: Band,
    fill: Fill,
    space: Option<Space>,
    /// After a pass that fell short: no other until this monotonic time,
    /// unless the band gets worse than it was.
    rest: Option<Rest>,
}

#[derive(Clone, Copy, Debug)]
struct Rest {
    until: f64,
    seconds: f64,
    band: Band,
}

/// How long a filesystem rests after a pass that fell short, at first and
/// at most: what was kept is in use or can't be judged, and looking again
/// at once would only find the same and spend a scan saying so. The worse
/// the band, the sooner it looks again, since what's in use there may
/// stop being so any moment and the disk can't wait long.
const FIRST_REST: f64 = 60.0;
fn longest_rest(band: Band) -> f64 {
    match band {
        Band::Critical => 60.0,
        Band::Urgent => 300.0,
        _ => 900.0,
    }
}

/// How long a pass is assumed to take before one has been timed.
const FIRST_PASS_SECONDS: f64 = 60.0;
/// The weight each timed pass carries against those before it.
const PASS_WEIGHT: f64 = 0.3;
/// How often what was learned is saved when no pass has run.
const SAVE_EVERY: f64 = 300.0;

/// How often the targets are walked for a `deps` to watch: one built since
/// goes unheard until then, and is dated from when it's first watched.
const MARK_EVERY: f64 = 3600.0;

/// How far back writes are weighed when choosing what to freeze.
const WRITING_WINDOW: f64 = 60.0;

/// The least a writer must write, in bytes a second, to be what fills a
/// disk at critical: anything slower isn't worth stopping.
const RUNAWAY: f64 = 1024.0 * 1024.0;

/// Whether process `pid` has a file open on the filesystem `device`.
fn writes_on(processes: &Path, pid: u32, device: u64) -> bool {
    let Ok(fds) = fs::read_dir(processes.join(pid.to_string()).join("fd")) else {
        return false;
    };
    fds.filter_map(Result::ok)
        .any(|fd| pool(&fd.path()).is_ok_and(|pool| pool == device))
}

/// Thaws each frozen process whose disk has room again, and lets go of one
/// somebody thawed already, journaling either; the rest stay frozen.
fn thaw(
    freeze: &mut dyn crate::freeze::Freeze,
    watched: &BTreeMap<u64, Watched>,
    memory: &mut Memory,
    journal: &mut Journal,
    tick: &Tick,
) -> io::Result<()> {
    let band_of = |path: &Path| {
        watched
            .values()
            .find(|watched| watched.path == path)
            .map(|watched| watched.band)
    };
    let mut still = Vec::new();
    for frozen in std::mem::take(&mut memory.frozen) {
        let roomy = band_of(&frozen.path).is_none_or(|band| band < Band::Urgent);
        let (kept, outcome) = if roomy {
            match freeze.thaw(&frozen.unit) {
                Ok(()) => (false, Outcome::Reclaimed),
                Err(why) => (true, Outcome::Failed(why)),
            }
        } else if let Ok(true) = freeze.frozen(&frozen.unit) {
            (true, Outcome::Reclaimed)
        } else {
            // Thawed by somebody else, or gone: theirs to decide.
            memory.spared.push((frozen.pid, frozen.start));
            (false, Outcome::Kept("thawed already".to_owned()))
        };
        if !kept || roomy {
            journal.write(&Line::Action {
                at: tick.wall,
                band: band_of(&frozen.path).unwrap_or(Band::Healthy),
                path: named(&frozen.path, &format!("{}:{}", frozen.name, frozen.pid)),
                kind: "thaw".to_owned(),
                size: 0,
                outcome,
            })?;
        }
        if kept {
            still.push(frozen);
        }
    }
    memory.frozen = still;
    Ok(())
}
/// How often the status file is written when nothing in it has changed,
/// so its heartbeat says the guard is alive.
const HEARTBEAT_EVERY: f64 = 30.0;

/// What the garbage's commands and candidates are named for.
const GARBAGE: &str = "garbage";
/// What linking the store's identical files is journaled and run as.
const DEDUPE: &str = "dedupe";
/// How long after a failed run linking the store's files the next waits.
const DEDUPE_RETRY: f64 = 86400.0;

type DedupeOut = (f64, Arc<AtomicBool>, Receiver<Result<Option<u64>, String>>);

/// The candidates a profile's generations are offered as.
const GENERATIONS: &str = "generations";
const YOUNG_GENERATIONS: &str = "generations:young";
/// How old Nix's garbage listing may grow on a healthy disk before a spare
/// moment lists it again, as often as upkeep comes, and how old under
/// pressure. Then how long a listing that failed waits for the next, on a
/// healthy disk and under pressure, where a listing is all garbage can be
/// collected by.
const LISTING_FRESH: f64 = 6.0 * 3600.0;
const LISTING_PRESSED: f64 = 3600.0;
const LISTING_RETRY: f64 = 3600.0;
const LISTING_RETRY_PRESSED: f64 = 300.0;
/// How soon after the last listing began one comes again once a root has
/// left, so a build's result deleted is garbage gone within a minute; and
/// once a process holding paths has ended, which happens every few seconds
/// on a busy machine, so the listing's minute of the disk's time is spent
/// at most every two.
const ROOT_LEFT_GAP: f64 = 30.0;
const HELD_ENDED_GAP: f64 = 120.0;

/// How often the guard looks for an owner that has ended: a tree's last
/// holder, a quest finished or parked, a boot, or a guard with new rules.
/// Reading every quest's README and a few processes takes milliseconds, and
/// a minute keeps what an owner leaves from outliving it by much more.
const OWNERS_EVERY: f64 = 60.0;

/// How soon upkeep comes again when it kept something only the system
/// daemon could have vouched for, and couldn't be asked. A switch restarts
/// both daemons, so the user's upkeep can begin while the system's is down;
/// ten minutes lets it come back, and costs a daemon that stays down one
/// walk in ten minutes rather than one a minute.
const UNHEARD_AGAIN: f64 = 600.0;

/// What holds the store, as cheaply read as it changes: the processes with
/// temporary roots, by their files' names, and each of Nix's roots that
/// still leads somewhere, by its link.
#[derive(Clone, Debug, Default, PartialEq)]
struct Holding {
    processes: BTreeSet<OsString>,
    roots: BTreeSet<PathBuf>,
}

impl Holding {
    fn read(temp_roots: &Path, roots: &Path) -> Holding {
        let names = |dir: &Path| -> Vec<fs::DirEntry> {
            fs::read_dir(dir)
                .map(|entries| entries.filter_map(Result::ok).collect())
                .unwrap_or_default()
        };
        let processes = names(temp_roots)
            .into_iter()
            .map(|entry| entry.file_name())
            .collect();
        // Following each link, as Nix does: a result deleted leaves its
        // link behind, leading nowhere, until a collection tidies it.
        let roots = names(&roots.join("auto"))
            .into_iter()
            .map(|entry| entry.path())
            .filter(|link| fs::metadata(link).is_ok())
            .collect();
        Holding { processes, roots }
    }
}
/// The most paths deleted in one go. Each delete costs Nix a whole look
/// for roots, most of a second on a busy machine, which batches of 300 paid
/// almost twice a second, most of a core, while a backlog drained. Nix
/// deletes nothing of a batch it refuses, so a smaller one saves nothing
/// then, and paths something holds are left out before it's made. Its
/// arguments, at most about 500 kB however long the names, stay under the
/// megabyte a Mac's command line carries.
const MOST_PER_BATCH: usize = 2000;
/// How long a user daemon waits for the system daemon to hear its
/// heartbeat.
const HEARTBEAT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
/// How long a user daemon's word holds a filesystem's passes for it.
const SPOKEN_FOR: std::time::Duration = std::time::Duration::from_secs(60);

/// Where a guard keeps and reads things.
#[derive(Clone, Debug, PartialEq)]
pub struct Places {
    /// Its memory and journal.
    pub state: PathBuf,
    /// Its status file, when the config asks for one.
    pub status: Option<PathBuf>,
    /// Where processes are read from: `/proc`.
    pub processes: PathBuf,
    /// The system daemon's socket, for a user daemon to ask about the
    /// processes it can't read.
    pub system: Option<PathBuf>,
    /// Links to the systems running and booted, whose generations stay.
    pub running: Vec<PathBuf>,
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "its four flags are independent facts about unrelated work, not one state"
)]
pub struct Guard {
    config: Config,
    processes: PathBuf,
    system: Option<PathBuf>,
    memory_path: PathBuf,
    memory: Memory,
    journal: Journal,
    status_path: Option<PathBuf>,
    predicates: Vec<Predicate>,
    answers: Answers,
    /// Where the answers are kept for the next guard.
    answers_path: PathBuf,
    watched: BTreeMap<u64, Watched>,
    last: Option<LastPass>,
    saved_at: Option<f64>,
    written: Option<(f64, Status)>,
    /// The status last written, which each beat writes again fresh.
    beaten: Arc<Mutex<Option<Status>>>,
    /// Whether what an earlier run left moved aside has been seen to.
    resumed: bool,
    /// Nix's garbage as last listed, for the system daemon, and where it's
    /// kept for the next guard.
    listing: Option<Listing>,
    listing_path: PathBuf,
    /// When a listing last began, so one that failed waits.
    listing_begun: Option<f64>,
    /// A listing under way on its own thread, and when it began.
    listing_out: Option<(f64, Listening)>,
    /// What a system daemon hears from user daemons.
    hearing: Option<Hearing>,
    /// For a user daemon, the filesystems the system daemon collects
    /// garbage on, by device, as it last said.
    collected_on: BTreeSet<u64>,
    /// Links to the systems running and booted.
    running: Vec<PathBuf>,
    /// For a user daemon, whether the system daemon answered its last
    /// heartbeat.
    system_heard: bool,
    /// Whether garbage is to be listed again at once, generations having
    /// expired since the listing began, or a batch having found it stale.
    relist: bool,
    /// Whether a batch from the listing held was refused as stale. Builds
    /// root again what a listing called dead, on 28 October 538 roots
    /// reaching into one batch of 2,000 two minutes after it was made, so
    /// its other batches meet the same, each refusal a whole look for roots, while the listing
    /// that would cure it waits on the lock those deletes take: none is
    /// offered until the next listing is heard.
    stale_listing: bool,
    /// What held the store at the last look: the processes with temporary
    /// roots, and the results Nix's roots link to that are still there.
    holding: Option<Holding>,
    /// When owners were last looked at, in monotonic seconds.
    owners_at: Option<f64>,
    /// Whether a root left since the listing began, or a process holding
    /// paths ended, either of which makes garbage the listing lacks.
    root_left: bool,
    held_ended: bool,
    /// Whether the listing's garbage is to be looked at for a pass of its
    /// own, something having let some of it go.
    collect: bool,
    /// A run linking the store's identical files: when it began, what
    /// stops it, and where it tells how it ended.
    dedupe_out: Option<DedupeOut>,
    /// When the last such run failed.
    dedupe_failed: Option<f64>,
    /// When each build's artifacts were last opened, shared with whatever
    /// hears the opens, and where it's kept for the next guard.
    uses: Arc<Mutex<Uses>>,
    uses_path: PathBuf,
    /// What hears the opens, once the guard listens, and when it last
    /// marked what's built.
    opens: Option<Arc<crate::opens::Watch>>,
    marked_at: Option<f64>,
    /// The account's home, which sentences write as `~`, for a guard run
    /// in one; the system's guard has none.
    home: Option<PathBuf>,
    /// What freezes a runaway writer, once given one, and what each
    /// process has written lately.
    freeze: Option<Box<dyn crate::freeze::Freeze>>,
    writers: Writers,
    /// What the last survey couldn't read, and why it went unvouched.
    unread: (usize, Option<String>),
    /// Whether the memory read was damaged, and is still to be set aside.
    damaged: bool,
    /// The trees the last survey kept for their use without offering.
    kept: Vec<(PathBuf, crate::activity::Use)>,
    /// What the last survey offered, less what passes took since, so a
    /// holder can say how much of it the guard may clear, and how much a
    /// clear would take now.
    offered: Vec<crate::holders::Offered>,
    /// Whether the holders are to be measured again on the next step,
    /// since a clear has just changed them.
    remeasure: bool,
    /// What holds the disks, as last measured and when, in wall seconds,
    /// and how much each grew in the day before.
    measured: Measured,
    /// The thread measuring them, while it does.
    measuring: Option<std::thread::JoinHandle<()>>,
}

/// What holds the disks, as a thread of its own measured it.
type Measured = Arc<Mutex<Option<(f64, Vec<(crate::holders::Place, u64)>, crate::ledger::Grown)>>>;

/// How often what holds the disks is measured again while one is below
/// healthy. Sizing every folder the sources name reads millions of
/// entries, so it's done on a thread at idle priority, and not often.
const HOLDERS_EVERY: f64 = 1800.0;

/// Why a path stays or would go.
#[derive(Clone, Debug, PartialEq)]
pub enum Explained {
    /// A source offers it, or what holds it, in use this way now.
    Offered {
        candidate: Candidate,
        used: crate::activity::Use,
    },
    /// A source found the tree holding it, which is kept for its use.
    Kept {
        path: PathBuf,
        used: crate::activity::Use,
    },
    /// No source finds it or anything holding it.
    Nothing,
}

/// Where a listing on its own thread sends the dead paths, or why not.
type Listening = Receiver<Result<Vec<Listed>, String>>;

/// What a system daemon hears from user daemons through its socket.
pub struct Hearing {
    /// When a user daemon last spoke for each filesystem, by device.
    pub spoken: Arc<Mutex<HashMap<u64, Instant>>>,
    /// The collects asked for: a path on the filesystem, the band to take
    /// its garbage as, and the connection to answer on.
    pub collects: Receiver<(PathBuf, Band, Responder)>,
}

/// The status file's shape. The second added sentences, each disk's use
/// and fill, and what was cleared, beside everything the first had.
pub const STATUS_VERSION: u32 = 2;

/// How many things cleared the status file lists.
const CLEARED_SHOWN: usize = 20;

/// What the status file says, for the desktop's disk piece and
/// `disk-guard status`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    pub version: u32,
    /// When the guard last wrote this, in wall seconds.
    pub heartbeat: f64,
    /// One sentence for the machine as a whole: its worst disk, or how
    /// much room its tightest has.
    pub says: String,
    pub filesystems: Vec<FilesystemStatus>,
    /// The last things cleared, newest first.
    pub cleared: Vec<ClearedStatus>,
    pub last: Option<LastPass>,
    /// The writers frozen now, for the desktop to offer to thaw.
    pub frozen: Vec<crate::writers::Frozen>,
    /// Who wrote most of the last minute's writes, while a disk is below
    /// healthy and one process did.
    pub writer: Option<Writing>,
    /// What stands to be told now, from any band below healthy, for
    /// whoever reads this file: the desktop is told only from urgent on.
    pub notice: Option<crate::notices::Notice>,
    /// What holds the disks, biggest first, measured while one is below
    /// healthy, and when it was measured, in wall seconds.
    pub holders: Vec<crate::holders::Holder>,
    pub holders_at: Option<f64>,
}

/// A process writing most of what's written, as the status file says it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Writing {
    pub name: String,
    pub pid: u32,
    /// Bytes a second.
    pub rate: f64,
}

/// One thing cleared, as the status file lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ClearedStatus {
    /// When, in wall seconds.
    pub at: f64,
    pub kind: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub says: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FilesystemStatus {
    pub path: PathBuf,
    pub band: Band,
    pub free: u64,
    /// Its size, as `size`; kept for readers of the first version.
    pub total: u64,
    pub size: u64,
    pub used: u64,
    /// Seconds until the next band at the fill's trend, if it's filling.
    pub forecast: Option<f64>,
    /// Bytes an hour it's filling at over the last two hours or so,
    /// negative when space is coming back; none until the guard has
    /// watched it ten minutes.
    pub fill_per_hour: Option<f64>,
    /// When it's full at the fill's trend, in wall seconds, if it's
    /// filling, both over the trend and now.
    pub full_at: Option<f64>,
    pub says: String,
    /// Whether the last pass here, in this band or a worse one, found
    /// nothing more it may clear.
    pub exhausted: bool,
}

impl FilesystemStatus {
    /// Takes a fresh measure, during a pass long enough that its start's
    /// would mislead: its band is worked out as the guard's watch works it
    /// out, from the band it showed. A forecast made for another band no
    /// longer holds, so it goes; when it fills is kept, from the fill the
    /// pass began with.
    pub fn measured(
        &mut self,
        space: &Space,
        bands: &crate::config::Bands,
        now: f64,
        home: Option<&Path>,
    ) {
        let band = Thresholds::new(bands, space.total).band(space, self.band);
        if band != self.band {
            self.forecast = None;
        }
        // A worse band takes more than the pass that found nothing left, and
        // its own pass hasn't run yet.
        if band > self.band {
            self.exhausted = false;
        }
        self.band = band;
        self.free = space.free;
        self.total = space.total;
        self.size = space.total;
        self.used = space.total.saturating_sub(space.free);
        let full_in = self.full_at.map(|at| at - now);
        self.says = crate::words::filesystem(&self.room(full_in), home);
    }

    fn room(&self, full_in: Option<f64>) -> crate::words::Room<'_> {
        crate::words::Room {
            path: &self.path,
            band: self.band,
            free: self.free,
            size: self.size,
            full_in,
            cleared_all: self.exhausted,
        }
    }
}

/// What a pass on one filesystem would take, in order, done nowhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Preview {
    pub path: PathBuf,
    pub band: Band,
    pub free: u64,
    pub total: u64,
    pub target: u64,
    pub end: String,
    pub taken: Vec<crate::plan::Candidate>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LastPass {
    pub at: f64,
    pub path: PathBuf,
    pub band: Band,
    pub end: String,
    pub freed: u64,
}

impl Guard {
    pub fn new(config: Config, places: &Places) -> io::Result<Guard> {
        let state = &places.state;
        let memory_path = state.join("memory.json");
        let mut damaged = false;
        let memory = match state::load(&memory_path) {
            Ok(memory) => memory.unwrap_or_default(),
            // Begun afresh, and set aside at the first save for whoever
            // wants to know why, so a plan that never saves changes nothing.
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                damaged = true;
                Memory::default()
            }
            Err(error) => return Err(error),
        };
        let predicates = config
            .sources
            .values()
            .filter_map(|source| match source {
                Source::Finished {
                    remote,
                    command,
                    timeout,
                } => Some(Predicate {
                    remote: regex_lite::Regex::new(remote).ok()?,
                    command: command.iter().map(OsString::from).collect(),
                    timeout: timeout.0,
                }),
                _ => None,
            })
            .collect();
        let status_path = places.status.clone().filter(|_| config.status_file);
        let listing_path = state.join("garbage.json");
        // One that can't be read is listed afresh.
        let listing: Option<Listing> = fs::read(&listing_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        // What can't be read starts afresh: an unwatched deps dates nothing.
        let answers_path = state.join("finished.json");
        let mut answers: Answers = state::load(&answers_path)
            .ok()
            .flatten()
            .unwrap_or_default();
        answers.forget_gone();
        let uses_path = state.join("uses.json");
        let mut uses: Uses = state::load(&uses_path).ok().flatten().unwrap_or_default();
        uses.forget_gone();
        let home = matches!(config.scope, crate::config::Scope::User)
            .then(|| std::env::var_os("HOME").map(PathBuf::from))
            .flatten();
        Ok(Guard {
            config,
            processes: places.processes.clone(),
            system: places.system.clone(),
            memory_path,
            memory,
            journal: Journal::new(&state.join("journal.jsonl"), journal::LIMIT),
            status_path,
            predicates,
            answers,
            answers_path,
            watched: BTreeMap::new(),
            last: None,
            saved_at: None,
            written: None,
            beaten: Arc::default(),
            resumed: false,
            listing_begun: listing.as_ref().map(|listing| listing.listed_at),
            listing,
            listing_path,
            listing_out: None,
            hearing: None,
            collected_on: BTreeSet::new(),
            running: places.running.clone(),
            system_heard: false,
            relist: false,
            stale_listing: false,
            holding: None,
            owners_at: None,
            root_left: false,
            held_ended: false,
            collect: false,
            dedupe_out: None,
            dedupe_failed: None,
            uses: Arc::new(Mutex::new(uses)),
            uses_path,
            opens: None,
            marked_at: None,
            home,
            freeze: None,
            writers: Writers::default(),
            unread: (0, None),
            damaged,
            kept: Vec::new(),
            offered: Vec::new(),
            remeasure: false,
            measured: Arc::new(Mutex::new(None)),
            measuring: None,
        })
    }

    /// Starts measuring what holds the disks, when one is below healthy or
    /// holds more than its budget, and the last measure is old or missing,
    /// unless a measure still runs.
    fn measure_holders(&mut self, now: f64) {
        if self
            .measuring
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return;
        }
        self.measuring = None;
        let budget = self.config.budget;
        let pressed = self.watched.values().any(|watched| {
            watched.band != Band::Healthy
                || budget
                    .zip(watched.space.as_ref())
                    .is_some_and(|(budget, space)| {
                        space.total.saturating_sub(space.free) > budget.of(space.total)
                    })
        });
        let fresh = self
            .measured
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_some_and(|(at, _, _)| now - at < HOLDERS_EVERY);
        if !pressed || (fresh && !self.remeasure) {
            return;
        }
        self.remeasure = false;
        let places = crate::holders::places(&self.config);
        let measured = Arc::clone(&self.measured);
        let state = self.state().to_owned();
        let started = std::thread::Builder::new()
            .name("holders".to_owned())
            .spawn(move || {
                // At idle priority, so sizing never slows what the person
                // or a build does. A Mac's only way lowers the whole
                // process, passes too, so there it runs as it is.
                #[cfg(target_os = "linux")]
                crate::commands::idle();
                let sized = crate::holders::measure(places);
                // A ledger that won't write leaves the holders unsaid only
                // in how much they grew.
                let grew = crate::ledger::record(&state, now, &sized).unwrap_or_else(|error| {
                    eprintln!("disk-guard: keeping the ledger: {error}");
                    crate::ledger::Grown::new()
                });
                *measured.lock().unwrap_or_else(PoisonError::into_inner) = Some((now, sized, grew));
            });
        match started {
            Ok(thread) => self.measuring = Some(thread),
            Err(error) => eprintln!("disk-guard: measuring what holds the disks: {error}"),
        }
    }

    /// The candidate a source offers at `path`, or the nearest one holding
    /// it, and how it's in use now. Nothing is taken.
    pub fn explain(&mut self, tick: &Tick, path: &Path) -> io::Result<Explained> {
        self.memory.awake.observe(tick);
        self.watch();
        let (mut scan, offers) = self.offers(tick, None)?;
        let holds = |held: &Path| path.starts_with(held);
        let offered = offers
            .into_iter()
            .map(|offer| offer.candidate)
            .filter(|candidate| holds(&candidate.path))
            .max_by_key(|candidate| candidate.path.as_os_str().len());
        let kept = self
            .kept
            .iter()
            .filter(|(tree, _)| holds(tree))
            .max_by_key(|(tree, _)| tree.as_os_str().len())
            .cloned();
        // The nearest of either is what decides it.
        let deeper = |kept: &PathBuf, candidate: &Candidate| {
            kept.as_os_str().len() > candidate.path.as_os_str().len()
        };
        Ok(match (offered, kept) {
            (Some(candidate), Some((tree, _))) if !deeper(&tree, &candidate) => {
                let used = crate::reclaim::Looker::judge(
                    &mut scan,
                    &candidate.path,
                    candidate.checkout.as_deref(),
                )?;
                Explained::Offered { candidate, used }
            }
            (_, Some((path, used))) => Explained::Kept { path, used },
            (Some(candidate), None) => {
                let used = crate::reclaim::Looker::judge(
                    &mut scan,
                    &candidate.path,
                    candidate.checkout.as_deref(),
                )?;
                Explained::Offered { candidate, used }
            }
            (None, None) => Explained::Nothing,
        })
    }

    /// How many processes the last survey couldn't read, even vouched for,
    /// and why the system daemon couldn't be asked, if it couldn't: a tree
    /// one of them might use is kept, so a plan doesn't show it.
    pub fn unread(&self) -> (usize, Option<String>) {
        self.unread.clone()
    }

    /// Freezes a runaway writer through `freeze`, when nothing else is
    /// left to take; without one, nothing is ever frozen.
    pub fn freeze_with(&mut self, freeze: Box<dyn crate::freeze::Freeze>) {
        self.freeze = Some(freeze);
    }

    /// Freezes what's filling a disk that's critical with nothing left to
    /// take, and thaws what was frozen once its disk has room again, or
    /// lets it go when somebody thawed it by hand. Only the account's own
    /// processes, outside its sessions, and never the guard.
    fn runaways(&mut self, tick: &Tick) -> io::Result<()> {
        // Who writes is sampled while any disk is short, freezing or not,
        // so the status and the notices can name what fills one.
        let pressed = self
            .watched
            .values()
            .any(|watched| watched.band > Band::Healthy);
        let samples = if pressed {
            // SAFETY: geteuid only reads the process's own credentials.
            let me = unsafe { libc::geteuid() };
            let samples: Vec<_> = writers::sample(&self.processes)
                .into_iter()
                .filter(|sample| sample.uid == me)
                .collect();
            self.writers.observe(samples.clone(), tick.wall);
            Some(samples)
        } else {
            self.writers = Writers::default();
            None
        };
        let Some(freeze) = self.freeze.as_mut() else {
            return Ok(());
        };
        // First what's frozen: thawed with room, let go when thawed already.
        thaw(
            freeze.as_mut(),
            &self.watched,
            &mut self.memory,
            &mut self.journal,
            tick,
        )?;
        let Some(samples) = samples else {
            return Ok(());
        };
        // SAFETY: geteuid only reads the process's own credentials.
        let me = unsafe { libc::geteuid() };
        // A process spared is spared while it lives; a pid reused is another.
        self.memory.spared.retain(|&(pid, start)| {
            samples
                .iter()
                .any(|sample| sample.pid == pid && sample.start == start)
        });
        let Some(writer) = self.writers.dominant(WRITING_WINDOW, tick.wall) else {
            return Ok(());
        };
        let freezable = writer.rate >= RUNAWAY
            && writer.pid != std::process::id()
            && !self.memory.spared.contains(&(writer.pid, writer.start))
            && !self
                .memory
                .frozen
                .iter()
                .any(|frozen| frozen.pid == writer.pid)
            && writer.cgroup.as_deref().is_some_and(|cgroup| {
                cgroup.starts_with(&format!("/user.slice/user-{me}.slice/user@{me}.service/"))
                    && !cgroup.contains("/session.slice/")
                    && !cgroup.contains("disk-guard")
            });
        if !freezable {
            return Ok(());
        }
        // A disk at critical whose last pass found nothing left, and that
        // the writer has a file open on.
        let Some((path, band)) = self
            .watched
            .iter()
            .filter(|(_, watched)| {
                watched.band == Band::Critical
                    && watched.rest.is_some_and(|rest| rest.band == Band::Critical)
            })
            .find(|(device, _)| writes_on(&self.processes, writer.pid, **device))
            .map(|(_, watched)| (watched.path.clone(), watched.band))
        else {
            return Ok(());
        };
        let unit = crate::freeze::unit_for(writer.pid);
        let pids = writers::descendants(&samples, writer.pid);
        let description = format!(
            "{} (pid {}), frozen by disk-guard for filling {}",
            writer.name,
            writer.pid,
            path.display()
        );
        let outcome = match freeze.freeze(&unit, &pids, &description) {
            Ok(()) => {
                self.memory.frozen.push(Frozen {
                    unit,
                    pid: writer.pid,
                    start: writer.start,
                    name: writer.name.clone(),
                    path: path.clone(),
                    at: tick.wall,
                });
                Outcome::Reclaimed
            }
            Err(why) => Outcome::Failed(why),
        };
        self.journal.write(&Line::Action {
            at: tick.wall,
            band,
            path: named(&path, &format!("{}:{}", writer.name, writer.pid)),
            kind: "freeze".to_owned(),
            size: writer.written,
            outcome,
        })
    }

    /// Hears from `watch` who opens the artifacts of the targets it finds,
    /// on a thread of its own, so a busy build's thousands of opens never
    /// wait on a step. The thread ends with the guard.
    pub fn listen(&mut self, watch: Arc<crate::opens::Watch>) {
        let uses = Arc::downgrade(&self.uses);
        let heard = Arc::clone(&watch);
        let spawned = std::thread::Builder::new()
            .name("opens".to_owned())
            .spawn(move || {
                loop {
                    let seen = heard.wait(Duration::from_secs(1));
                    let Some(uses) = uses.upgrade() else { return };
                    let Ok(seen) = seen else { return };
                    if seen.is_empty() {
                        continue;
                    }
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0.0, |since| since.as_secs_f64());
                    let mut uses = uses.lock().unwrap_or_else(PoisonError::into_inner);
                    for seen in seen {
                        match seen {
                            Seen::Opened(path) => uses.opened(&path, now),
                            Seen::Lost => uses.lost(now),
                        }
                    }
                }
            });
        if spawned.is_ok() {
            self.opens = Some(watch);
        }
    }

    /// Marks every `deps` of the targets the cargo sources hold, at first
    /// and hourly after, so a build made since is heard too. A `deps` that
    /// was made anew loses its mark with the old one and gets one again.
    fn mark(&mut self, tick: &Tick) {
        let Some(watch) = &self.opens else { return };
        if self.marked_at.is_some_and(|at| tick.wall - at < MARK_EVERY) {
            return;
        }
        self.marked_at = Some(tick.wall);
        let mut marked = Vec::new();
        for source in self.config.sources.values() {
            let Source::Cargo { roots, depth } = source else {
                continue;
            };
            for root in roots {
                let found = discover::walk(root, depth.map(|depth| depth as usize));
                for tree in found.unwrap_or_default() {
                    if tree.kind != discover::Kind::Target {
                        continue;
                    }
                    for deps in crate::replaced::deps_dirs(&tree.path) {
                        if watch.watch(&deps).is_ok() {
                            marked.push(deps);
                        }
                    }
                }
            }
        }
        let mut uses = self.uses.lock().unwrap_or_else(PoisonError::into_inner);
        for deps in marked {
            uses.watching(&deps, tick.wall);
        }
    }

    pub fn step(&mut self, tick: &Tick, senses: &Senses) -> io::Result<()> {
        // Once: what it resumes it journals as ended, so looking again
        // would only re-read the journal every step to find nothing.
        if !self.resumed {
            self.resumed = true;
            self.resume(tick)?;
        }
        self.memory.awake.observe(tick);
        self.watch();
        self.mark(tick);
        self.heartbeat();
        self.heard_garbage(tick.wall)?;
        self.watch_owners(tick);
        let spare_moment = spare(senses.power, senses.saver, &self.memory.awake, tick.wall);
        let mut due = Vec::new();
        for (&device, watched) in &mut self.watched {
            let Ok(space) = crate::sys::space(&watched.path) else {
                watched.space = None;
                continue;
            };
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            watched.band = thresholds.band(&space, watched.band);
            watched.fill.observe(tick.monotonic, space.free);
            watched.space = Some(space);
            if watched.band == Band::Healthy {
                watched.rest = None;
            }
            let decision = decide(&Situation {
                band: watched.band,
                free: space.free,
                thresholds,
                fill: &watched.fill,
                pass_seconds: self.memory.pass_seconds.unwrap_or(FIRST_PASS_SECONDS),
                upkept: self.memory.upkept,
                now: tick.wall,
                spare: spare_moment,
            });
            let resting = watched.rest.is_some_and(|rest| {
                tick.monotonic < rest.until
                    && matches!(decision, Decision::Reach(band) if band <= rest.band)
            });
            // A user daemon on this filesystem asks for the garbage in its
            // turn; the system's own upkeep goes on.
            let driven =
                matches!(decision, Decision::Reach(_)) && spoken_for(self.hearing.as_ref(), device);
            if decision != Decision::Rest && !resting && !driven {
                due.push((device, decision, thresholds));
            }
        }
        // Once the bands are read, and before a pass: on a restart every
        // band reads healthy until this step reads it, so the first measure
        // waited out a whole pass, and passes took 20 and 29 minutes on 26
        // October.
        self.measure_holders(tick.wall);
        self.watch_roots();
        self.list_garbage(tick.wall, spare_moment);
        self.collect_garbage(tick.wall, &mut due);
        // Deduplicating reads every file not yet linked and relinks its
        // twins, and nothing is lost by putting it off, so it alone still
        // waits for a calm disk.
        let calm_moment = spare_moment && calm(senses.pressure);
        self.dedupe(tick, calm_moment, powered(senses.power, senses.saver))?;
        self.collects(tick);
        let asked = self.clear_asked(tick)?;
        let passed = asked || !due.is_empty();
        if passed {
            // So a pass's heartbeat starts from what the disks are now.
            self.write_status(tick.wall)?;
            self.passes(tick, &due)?;
        }
        let frozen = self.memory.frozen.len();
        self.runaways(tick)?;
        let passed = passed || frozen != self.memory.frozen.len();
        if passed || self.saved_at.is_none_or(|at| tick.wall - at >= SAVE_EVERY) {
            if self.damaged {
                let aside = self.memory_path.with_file_name("memory.json.damaged");
                fs::rename(&self.memory_path, aside)?;
                self.damaged = false;
            }
            state::save(&self.memory_path, &self.memory)?;
            let uses = self
                .uses
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            state::save(&self.uses_path, &uses)?;
            state::save(&self.answers_path, &self.answers)?;
            self.saved_at = Some(tick.wall);
        }
        self.write_status(tick.wall)
    }

    /// Empties what a run that died mid reclaim left moved aside: the trash
    /// in each directory it began and never ended. Never put back, since
    /// its paths have been missing since, and outputs restored beside a
    /// build that has moved on could pass for fresh.
    fn resume(&mut self, tick: &Tick) -> io::Result<()> {
        for (path, root) in self.journal.unfinished()? {
            let emptied = (|| -> io::Result<()> {
                let relative = path
                    .strip_prefix(&root)
                    .map_err(|_| io::Error::other("outside its root"))?;
                let dir = match Dir::open(&root).and_then(|root| root.child(relative)) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                    opened => opened?,
                };
                for left in dir.leftovers()? {
                    left.empty()?;
                }
                Ok(())
            })();
            self.journal.write(&Line::Action {
                at: tick.wall,
                band: Band::Healthy,
                path,
                kind: "resumed".to_owned(),
                size: 0,
                outcome: match emptied {
                    Ok(()) => Outcome::Reclaimed,
                    Err(error) => Outcome::Failed(format!("emptying what was left: {error}")),
                },
            })?;
        }
        Ok(())
    }

    /// Tells the system daemon which filesystems this user daemon watches,
    /// learning those it collects garbage on, whose passes this one then
    /// drives. A system daemon that doesn't answer collects on none, and
    /// passes alone a minute later.
    fn heartbeat(&mut self) {
        let Some(socket) = &self.system else {
            self.system_heard = false;
            return;
        };
        let paths: Vec<PathBuf> = self.watched.values().map(|w| w.path.clone()).collect();
        if paths.is_empty() {
            self.collected_on.clear();
            return;
        }
        let watching = Request::Watching { paths };
        let heard = crate::socket::ask(socket, &watching, HEARTBEAT_TIMEOUT);
        self.system_heard = heard.is_ok();
        self.collected_on = match heard {
            Ok(Reply::Collects(held)) => held.iter().filter_map(|path| pool(path).ok()).collect(),
            _ => BTreeSet::new(),
        };
    }

    /// What a user daemon offers of the system daemon's garbage: on each
    /// filesystem it collects on, a collect asked of it, in garbage's turn
    /// from pressure on and measured like any command. Routine upkeep of
    /// garbage is the system daemon's own.
    fn asked_offers(&self) -> Vec<Offer> {
        let Some(socket) = &self.system else {
            return Vec::new();
        };
        self.collected_on
            .iter()
            .filter_map(|&device| {
                let mount = &self.watched.get(&device)?.path;
                Some(Offer {
                    candidate: Candidate {
                        path: named(mount, GARBAGE),
                        class: Class::Garbage {
                            kept: true,
                            order: 0,
                        },
                        idle: 0.0,
                        size: 0,
                        occupied: false,
                        spent: false,
                        checkout: None,
                    },
                    action: Action::Ask {
                        root: mount.clone(),
                        socket: socket.clone(),
                    },
                    device,
                })
            })
            .collect()
    }

    /// Clears each folder Joel asked cleared, saying whether there were
    /// any. Each takes what the survey offers inside it that a clear takes
    /// (`crate::plan::clears`), judged for use again as it goes, paced by
    /// its disk's own band, so a clear on a healthy disk is a gentle one.
    /// An ask ends once its pass has, so until then the status says the
    /// folder is clearing.
    fn clear_asked(&mut self, tick: &Tick) -> io::Result<bool> {
        let asks = crate::clears::asked(self.state())?;
        if asks.is_empty() {
            return Ok(false);
        }
        self.write_status(tick.wall)?;
        let folders: Vec<PathBuf> = asks.iter().map(|(_, ask)| ask.path.clone()).collect();
        let (mut scan, offers) = self.offers(tick, Some(&folders))?;
        for (file, ask) in asks {
            // A source's root may be named through a link, and an ask's
            // folder never is.
            let inside: Vec<Offer> = offers
                .iter()
                .filter(|offer| {
                    let path = &offer.candidate.path;
                    let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
                    resolved.starts_with(&ask.path) && crate::plan::clears(&offer.candidate)
                })
                .cloned()
                .collect();
            let cleared = self.clear_on(tick, &inside, &mut scan);
            // Ended whatever the pass did, since one that failed would
            // fail again, each step surveying everything to.
            crate::clears::done(&file)?;
            cleared?;
        }
        self.remeasure = true;
        Ok(true)
    }

    /// A clear's passes over `inside`, one on each filesystem it spans.
    fn clear_on(&mut self, tick: &Tick, inside: &[Offer], scan: &mut Scan) -> io::Result<()> {
        let devices: BTreeSet<u64> = inside.iter().map(|offer| offer.device).collect();
        for device in devices {
            let Some(watched) = self.watched.get(&device) else {
                continue;
            };
            let band = watched.band;
            let space = crate::sys::space(&watched.path)?;
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            let here: Vec<Offer> = inside
                .iter()
                .filter(|offer| offer.device == device)
                .cloned()
                .collect();
            self.pass_on(tick, device, &here, (band, Goal::Clear), thresholds, scan)?;
        }
        Ok(())
    }

    /// Hears user daemons from now on.
    pub fn hear(&mut self, hearing: Hearing) {
        self.hearing = Some(hearing);
    }

    /// Answers each collect asked for since the last step.
    fn collects(&mut self, tick: &Tick) {
        let asked: Vec<_> = match &self.hearing {
            Some(hearing) => hearing.collects.try_iter().collect(),
            None => return,
        };
        for (path, band, responder) in asked {
            let reply = match self.collect(tick, &path, band) {
                Ok(end) => Reply::Collected(ending(&end)),
                Err(why) => Reply::Error(why),
            };
            responder.send(&reply);
        }
    }

    /// A pass of only the system's share of the store on the filesystem
    /// holding `path`, its generations and its garbage, in `band`, for the
    /// user daemon driving its order.
    fn collect(&mut self, tick: &Tick, path: &Path, band: Band) -> Result<End, String> {
        let device = pool(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let mount = match (self.store(), self.watched.get(&device)) {
            (Some((_, store)), Some(watched)) if store == device => watched.path.clone(),
            _ => return Err(format!("no garbage is collected on {}", path.display())),
        };
        let space = crate::sys::space(&mount).map_err(|error| error.to_string())?;
        let thresholds = Thresholds::new(&self.config.bands, space.total);
        let mut here = self.generation_offers(tick);
        here.extend(self.garbage_offers(tick));
        // Neither takes a process to judge, so the scan is never read.
        let me = std::process::id();
        let mut scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
        self.pass_on(
            tick,
            device,
            &here,
            (band, Goal::Reach),
            thresholds,
            &mut scan,
        )
        .map_err(|error| error.to_string())
    }

    /// Takes a listing of Nix's garbage that has finished, keeping it for
    /// the next guard, and makes upkeep due when routine would take some.
    fn heard_garbage(&mut self, now: f64) -> io::Result<()> {
        let Some((begun, out)) = &self.listing_out else {
            return Ok(());
        };
        let heard = match out.try_recv() {
            Err(TryRecvError::Empty) => return Ok(()),
            Ok(heard) => heard,
            Err(TryRecvError::Disconnected) => Err("the listing stopped".to_owned()),
        };
        let begun = *begun;
        self.listing_out = None;
        match heard {
            Ok(dead) => {
                let listing = Listing {
                    listed_at: begun,
                    dead,
                };
                state::save(&self.listing_path, &listing)?;
                self.collect = true;
                self.stale_listing = false;
                if self.batches(&listing, now).iter().any(|batch| !batch.kept) {
                    self.memory.upkept = None;
                }
                self.listing = Some(listing);
            }
            // Tried again in an hour; until then the listing before stands.
            Err(why) => eprintln!("disk-guard: listing Nix's garbage: {why}"),
        }
        Ok(())
    }

    /// Notes what holds the store, and whether anything let go of it since
    /// the last look: a root that left is garbage made, and a process that
    /// ended may have let go of a build nothing else holds.
    fn watch_roots(&mut self) {
        if self.store().is_none() {
            return;
        }
        let nix = &self.config.nix;
        let now = Holding::read(&nix.temp_roots, &nix.roots);
        if let Some(before) = &self.holding {
            if before.roots.iter().any(|root| !now.roots.contains(root)) {
                self.root_left = true;
            }
            if before
                .processes
                .iter()
                .any(|process| !now.processes.contains(process))
            {
                self.held_ended = true;
                self.collect = true;
            }
        }
        self.holding = Some(now);
    }

    /// Makes upkeep due at once when an owner has ended since the last look:
    /// a tree's last holder or the quest it was kept for, a boot, which
    /// ends every program and so everything they left in temp, a guard
    /// with new rules, which may judge more spent, or a producer's watched
    /// file changing, as a cache gaining a version ends an old one's use. What an owner leaves is
    /// spent the moment it ends, so waiting six hours for routine upkeep
    /// only keeps bytes nobody will read.
    fn watch_owners(&mut self, tick: &Tick) {
        if self
            .owners_at
            .is_some_and(|at| tick.monotonic - at < OWNERS_EVERY)
        {
            return;
        }
        self.owners_at = Some(tick.monotonic);
        let Ok(boot) = crate::sys::boot(Path::new("/proc/stat")) else {
            return;
        };
        let processes = &self.processes;
        let running = |pid: u32, started: f64| {
            crate::sys::started(processes, pid).is_some_and(|ticks| {
                let since = boot.wall + crate::units::float(ticks) / boot.ticks;
                (since - started).abs() <= 2.0
            })
        };
        let seen = Owners {
            boot: tick.boot.clone(),
            guard: std::env::current_exe().ok(),
            spent: spent_trees(&self.config, running)
                .into_iter()
                .filter(|tree| tree.exists())
                .collect(),
        };
        let watched = self
            .config
            .sources
            .iter()
            .any(|(name, source)| match source {
                Source::Producer { watch, .. } => {
                    crate::survey::changed_since(watch, self.memory.ran.get(name).copied())
                }
                _ => false,
            });
        let ended = watched
            || self.memory.owners.as_ref().is_none_or(|before| {
                before.boot != seen.boot
                    || before.guard != seen.guard
                    || seen.spent.iter().any(|tree| !before.spent.contains(tree))
            });
        if ended {
            self.memory.upkept = None;
        }
        self.memory.owners = Some(seen);
    }

    /// Has the listing's garbage go in a pass of its own, once something
    /// let some of it go and no other pass is due on the store's disk: a
    /// healthy disk keeps none of it, unless told to as a rebuild cache.
    fn collect_garbage(&mut self, now: f64, due: &mut Vec<(u64, Decision, Thresholds)>) {
        if !self.collect {
            return;
        }
        self.collect = false;
        let Some((_, device)) = self.store() else {
            return;
        };
        if due.iter().any(|(on, _, _)| *on == device) {
            return;
        }
        let Some(watched) = self.watched.get(&device) else {
            return;
        };
        // Under pressure a user daemon speaking for the disk orders what
        // goes, its own caches against the store's garbage; healthy, the
        // garbage is the store's alone.
        if watched.band > Band::Healthy && spoken_for(self.hearing.as_ref(), device) {
            return;
        }
        let Some(space) = watched.space else {
            return;
        };
        let Some(listing) = &self.listing else {
            return;
        };
        if self.batches(listing, now).iter().any(|batch| !batch.kept) {
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            due.push((device, Decision::Collect, thresholds));
        }
    }

    /// Begins listing Nix's garbage on its own thread when it's due: soon
    /// after a root leaves or a process holding paths ends, six hours on
    /// at a spare moment, or a day on whenever there's power, at idle
    /// priority; and at once when the store's filesystem is under pressure
    /// and the listing is an hour old.
    fn list_garbage(&mut self, now: f64, spare: bool) {
        if self.listing_out.is_some() {
            return;
        }
        let Some((_, device)) = self.store() else {
            return;
        };
        let pressed = self
            .watched
            .get(&device)
            .is_some_and(|watched| watched.band > Band::Healthy);
        let age = |at: Option<f64>| at.map_or(f64::INFINITY, |at| now - at);
        let listed = age(self.listing.as_ref().map(|listing| listing.listed_at));
        let retry = if pressed {
            LISTING_RETRY_PRESSED
        } else {
            LISTING_RETRY
        };
        let since = age(self.listing_begun);
        let due = self.relist
            || self.root_left && since >= ROOT_LEFT_GAP
            || self.held_ended && since >= HELD_ENDED_GAP
            || since >= retry
                && if pressed {
                    listed >= LISTING_PRESSED
                } else {
                    spare && listed >= LISTING_FRESH
                };
        if !due {
            return;
        }
        let nix = self.config.nix.bin.clone();
        let (tell, out) = mpsc::channel();
        let listing = std::thread::Builder::new()
            .name(GARBAGE.to_owned())
            .spawn(move || {
                // A guard that stopped meanwhile has no one to hear it.
                let _ = tell.send(garbage::list(&nix, !pressed));
            });
        self.listing_begun = Some(now);
        self.relist = false;
        self.root_left = false;
        self.held_ended = false;
        match listing {
            Ok(_) => self.listing_out = Some((now, out)),
            Err(error) => eprintln!("disk-guard: listing Nix's garbage: {error}"),
        }
    }

    /// Links the store's identical files on a thread, in a spare moment
    /// while it's healthy and a run is due, and stops a run once the store
    /// leaves healthy or the power stops being spare. Not once the disk
    /// stalls: its own reads stall it. Each run is journaled, and only one
    /// that finished counts as done.
    fn dedupe(&mut self, tick: &Tick, spare_moment: bool, powered: bool) -> io::Result<()> {
        let Some(system) = &self.config.system else {
            return Ok(());
        };
        let store = self.config.nix.store.clone();
        let Some(watched) = pool(&store)
            .ok()
            .and_then(|device| self.watched.get(&device))
        else {
            return Ok(());
        };
        let (band, space) = (watched.band, watched.space);
        if let Some((begun, stop, out)) = &self.dedupe_out {
            if band > Band::Healthy || !powered {
                stop.store(true, Ordering::Relaxed);
            }
            let ended = match out.try_recv() {
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => Err("its thread ended unheard".to_owned()),
                Ok(ended) => ended,
            };
            let begun = *begun;
            self.dedupe_out = None;
            let (outcome, size) = match ended {
                Ok(Some(freed)) => {
                    self.memory.deduped = Some(Deduped { at: begun, freed });
                    self.saved_at = None;
                    (Outcome::Reclaimed, freed)
                }
                Ok(None) => (Outcome::Kept("stopped as the moment ended".to_owned()), 0),
                Err(why) => {
                    self.dedupe_failed = Some(tick.wall);
                    (Outcome::Failed(why), 0)
                }
            };
            return self.journal.write(&Line::Action {
                at: tick.wall,
                band,
                path: store,
                kind: DEDUPE.to_owned(),
                size,
                outcome,
            });
        }
        let failed_lately = self
            .dedupe_failed
            .is_some_and(|at| tick.wall - at < DEDUPE_RETRY);
        let Some(space) = space else {
            return Ok(());
        };
        let every = system.dedupe.every.0.as_secs_f64();
        let due = dedupe::due(self.memory.deduped.as_ref(), tick.wall, every, space.total);
        if band > Band::Healthy || !spare_moment || failed_lately || !due {
            return Ok(());
        }
        let (systemd_run, nix) = (
            system.dedupe.systemd_run.clone(),
            self.config.nix.bin.clone(),
        );
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let (tell, out) = mpsc::channel();
        let running = std::thread::Builder::new()
            .name(DEDUPE.to_owned())
            .spawn(move || {
                // A guard that stopped meanwhile has no one to hear it.
                let _ = tell.send(dedupe::optimise(systemd_run.as_deref(), &nix, &stopping));
            });
        match running {
            Ok(_) => self.dedupe_out = Some((tick.wall, stop, out)),
            Err(error) => eprintln!("disk-guard: linking the store's files: {error}"),
        }
        Ok(())
    }

    /// The garbage config and the store's device, for a system daemon.
    fn store(&self) -> Option<(&crate::config::Garbage, u64)> {
        let garbage = &self.config.system.as_ref()?.garbage;
        Some((garbage, pool(&self.config.nix.store).ok()?))
    }

    /// The listing's garbage that is still there, in the batches and the
    /// order it goes in.
    fn batches(&self, listing: &Listing, now: f64) -> Vec<Batch> {
        let Some((garbage, _)) = self.store() else {
            return Vec::new();
        };
        let Ok(space) = crate::sys::space(&self.config.nix.store) else {
            return Vec::new();
        };
        let keep = garbage.keep.0.as_secs_f64();
        let mut roots = crate::garbage::temp_roots(&self.config.nix.temp_roots);
        roots.extend(crate::garbage::runtime_roots(
            &self.processes,
            &self.config.nix.store,
        ));
        let held = listing.held_back(&roots);
        listing.batches(
            |dead| {
                !held.contains(dead.path.as_str())
                    && Path::new(&dead.path).symlink_metadata().is_ok()
            },
            now,
            keep,
            garbage.budget.of(space.total),
            MOST_PER_BATCH,
        )
    }

    /// What Nix's garbage offers: the listing's batches in the policy's
    /// order; or with no listing yet, from urgent on, hourly steps of
    /// `nix store gc`, which collects in no order of its own.
    fn garbage_offers(&self, tick: &Tick) -> Vec<Offer> {
        let Some((_, device)) = self.store() else {
            // A user daemon no system daemon answers collects alone, as any
            // of Nix's clients may: the nix-daemon refuses only to ignore
            // liveness.
            let alone = self.config.system.is_none() && !self.system_heard;
            return match pool(&self.config.nix.store) {
                Ok(device) if alone => vec![self.gc_offer(tick, device)],
                _ => Vec::new(),
            };
        };
        let store = &self.config.nix.store;
        let candidate = |path, class, idle, size| Candidate {
            path,
            class,
            idle,
            size,
            occupied: false,
            spent: false,
            checkout: None,
        };
        let Some(listing) = &self.listing else {
            return vec![self.gc_offer(tick, device)];
        };
        if self.stale_listing {
            return vec![self.gc_offer(tick, device)];
        }
        self.batches(listing, tick.wall)
            .into_iter()
            .enumerate()
            .map(|(order, batch)| Offer {
                candidate: candidate(
                    named(store, &format!("{GARBAGE}-{order}")),
                    Class::Garbage {
                        kept: batch.kept,
                        order: u16::try_from(order).unwrap_or(u16::MAX),
                    },
                    (tick.wall - batch.newest).max(0.0),
                    batch.size,
                ),
                action: Action::Collect {
                    root: store.clone(),
                    nix: self.config.nix.bin.clone(),
                    roots: self.config.nix.temp_roots.clone(),
                    paths: batch.paths,
                },
                device,
            })
            .collect()
    }

    /// Hourly steps of `nix store gc` from urgent on, which collects in
    /// no order of its own: for want of a listing, or of a system daemon.
    fn gc_offer(&self, tick: &Tick, device: u64) -> Offer {
        let store = &self.config.nix.store;
        let ran = self.memory.ran.get(GARBAGE).map_or(f64::INFINITY, |&at| {
            self.memory.awake.since(at.min(tick.wall), tick.wall)
        });
        let max = BATCH_BYTES.to_string();
        let nix = self.config.nix.bin.to_string_lossy();
        Offer {
            candidate: Candidate {
                path: named(store, GARBAGE),
                class: Class::Producer {
                    every: None,
                    from: Some(Band::Urgent),
                },
                idle: ran,
                size: 0,
                occupied: false,
                spent: false,
                checkout: None,
            },
            action: Action::Run {
                root: store.clone(),
                source: GARBAGE.to_owned(),
                argv: std::iter::once(&*nix)
                    .chain(crate::garbage::NIX_COMMAND)
                    .chain(["store", "gc", "--max", &max])
                    .map(str::to_owned)
                    .collect(),
                timeout: GC_TIMEOUT,
            },
            device,
        }
    }

    /// A profile's generations that may go, each profile offering those
    /// routine takes and, apart, those only pressure takes, in garbage's
    /// group ahead of it and measured as commands, though what they free
    /// shows only as garbage. The system profile's are judged against
    /// the boot partition's entries; no other profile boots.
    fn generation_offers(&self, tick: &Tick) -> Vec<Offer> {
        let nix = &self.config.nix;
        let Some(dir) = &nix.profiles else {
            return Vec::new();
        };
        let (Ok(device), Ok(profiles)) = (pool(&nix.store), generations::profiles(dir)) else {
            return Vec::new();
        };
        let in_use: Vec<PathBuf> = self
            .running
            .iter()
            .filter_map(|link| fs::read_link(link).ok())
            .collect();
        let mut offers = Vec::new();
        for profile in profiles {
            let boot = match &self.config.system {
                Some(system) if profile.path == dir.join("system") => {
                    match generations::entries(&system.boot) {
                        Ok(numbers) => Boot::Entries(numbers),
                        Err(_) => Boot::Unreadable,
                    }
                }
                _ => Boot::NotBooted,
            };
            let (routine, young) = generations::expired(
                &profile,
                &in_use,
                &boot,
                usize::try_from(nix.generations.keep).unwrap_or(usize::MAX),
                nix.generations.young.0.as_secs_f64(),
                tick.wall,
            );
            let Some(root) = profile.path.parent() else {
                continue;
            };
            for (numbers, kept, name) in [
                (routine, false, GENERATIONS),
                (young, true, YOUNG_GENERATIONS),
            ] {
                if numbers.is_empty() {
                    continue;
                }
                offers.push(Offer {
                    candidate: Candidate {
                        path: named(&profile.path, name),
                        class: Class::Generations { kept },
                        idle: 0.0,
                        size: 0,
                        occupied: false,
                        spent: false,
                        checkout: None,
                    },
                    action: Action::Expire {
                        root: root.to_owned(),
                        nix: nix.bin.clone(),
                        profile: profile.path.clone(),
                        numbers,
                    },
                    device,
                });
            }
        }
        offers
    }

    /// Notes every filesystem a source or the store lies on, keeping what
    /// was seen of those already watched. A user daemon watches the store
    /// when it looks after an account's generations, which free it.
    fn watch(&mut self) {
        let mut seen = BTreeMap::new();
        let store = (self.config.system.is_some() || self.config.nix.profiles.is_some())
            .then_some(self.config.nix.store.as_path());
        for path in self
            .config
            .sources
            .values()
            .flat_map(Source::under)
            .chain(store)
        {
            let (Ok(meta), Ok(device)) = (fs::metadata(path), pool(path)) else {
                continue;
            };
            if !meta.is_dir() || seen.contains_key(&device) {
                continue;
            }
            let watched = self.watched.remove(&device).unwrap_or_else(|| Watched {
                path: mounted(path, device),
                band: Band::Healthy,
                fill: Fill::default(),
                space: None,
                rest: None,
            });
            seen.insert(device, watched);
        }
        self.watched = seen;
    }

    /// Surveys once and runs each filesystem's pass over what lies on it.
    fn passes(&mut self, tick: &Tick, due: &[(u64, Decision, Thresholds)]) -> io::Result<()> {
        // Garbage alone needs no survey, which walks every source.
        let (mut scan, offers) = if due
            .iter()
            .all(|(_, decision, _)| *decision == Decision::Collect)
        {
            let me = std::process::id();
            let scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
            (scan, self.garbage_offers(tick))
        } else {
            self.offers(tick, None)?
        };
        for &(device, decision, thresholds) in due {
            if !self.watched.contains_key(&device) {
                continue;
            }
            let here: Vec<Offer> = offers
                .iter()
                .filter(|offer| offer.device == device)
                .cloned()
                .collect();
            let end = self.pass_on(tick, device, &here, aim(decision), thresholds, &mut scan)?;
            let Decision::Reach(band) = decision else {
                continue;
            };
            if let Some(watched) = self.watched.get_mut(&device) {
                watched.rest = (end != End::Reached).then(|| {
                    let seconds = watched.rest.map_or(FIRST_REST, |rest| {
                        (rest.seconds * 2.0).min(longest_rest(band))
                    });
                    Rest {
                        until: tick.monotonic + seconds,
                        seconds,
                        band,
                    }
                });
            }
        }
        Ok(())
    }

    /// Every process, scanned for the pass to judge by, and what every
    /// source offers.
    /// What the sources offer now, or with `folders`, only what the
    /// sources that touch them offer, for a clear: a survey of that part
    /// leaves what the holders say of the rest as the last whole one did.
    fn offers(
        &mut self,
        tick: &Tick,
        folders: Option<&[PathBuf]>,
    ) -> io::Result<(Scan, Vec<Offer>)> {
        let me = std::process::id();
        let mut scan = Scan::new(&self.processes, Path::new("/proc/locks"), me, FRESH);
        if let Some(socket) = &self.system {
            let roots: Vec<PathBuf> = self
                .config
                .sources
                .values()
                .flat_map(Source::under)
                .map(Path::to_owned)
                .collect();
            scan = scan.vouched_by(socket, &roots);
        }
        let processes = scan.processes()?.to_vec();
        self.unread = (
            processes.iter().filter(|process| !process.readable).count(),
            scan.unheard().map(str::to_owned),
        );
        let boot = crate::sys::boot(Path::new("/proc/stat"))?;
        // A copy, so a long survey never holds up what hears the opens.
        let uses = self
            .uses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let (predicates, answers) = (&self.predicates, &mut self.answers);
        // A tree a finished quest cut is finished with it, whatever its
        // branch says: its builds were the quest's. So is one nobody holds
        // any more, its builds having been its holders'. A pid holds its
        // tree only while it runs as the process that started then; one
        // that can't be read can't be ruled out, so holds it.
        let running = |pid: u32, started: f64| {
            processes.iter().any(|process| {
                let since = boot.wall + crate::units::float(process.start_ticks) / boot.ticks;
                process.pid == pid && (!process.readable || (since - started).abs() <= 2.0)
            })
        };
        let quests_over = spent_trees(&self.config, running);
        let mut finished = |dir: &Path| {
            if quests_over.iter().any(|tree| dir.starts_with(tree)) {
                return true;
            }
            let Some(checkout) = finished::read(dir) else {
                return false;
            };
            let Some(predicate) = finished::predicate_for(predicates, &checkout.remote) else {
                return false;
            };
            // Asked afresh each time, before what's remembered of its head:
            // a merged head stays merged while someone goes on editing.
            if finished::changed(dir) {
                return false;
            }
            answers.finished(dir, &checkout.head, tick.wall, || {
                finished::ask(predicate, dir, &checkout)
            })
        };
        let mut world = World {
            processes: &processes,
            me,
            // SAFETY: geteuid only reads the process's own credentials.
            uid: unsafe { libc::geteuid() },
            boot,
            awake: &self.memory.awake,
            now: tick.wall,
            finished: &mut finished,
            ran: &self.memory.ran,
            uses: &uses,
            kept: Vec::new(),
            counted: std::cell::RefCell::default(),
        };
        let scoped;
        let config = match folders {
            Some(folders) => {
                scoped = crate::survey::within(&self.config, folders);
                &scoped
            }
            None => &self.config,
        };
        let mut offers = survey(config, &mut world);
        let kept = std::mem::take(&mut world.kept);
        drop(world);
        // Read afresh each survey, so a keep holds from the next step. One
        // that won't read stops the survey rather than risk what it keeps.
        let keeps = crate::keeps::load(self.state())?;
        // A keep names its folder with every link resolved, and a source's
        // root may be named through one, so each offer is resolved too.
        offers.retain(|offer| {
            let path = &offer.candidate.path;
            let resolved = fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            !crate::keeps::touches(&keeps, &resolved)
        });
        if folders.is_some() {
            return Ok((scan, offers));
        }
        self.kept = kept;
        self.offered = offers
            .iter()
            .map(|offer| crate::holders::Offered {
                path: offer.candidate.path.clone(),
                size: offer.candidate.size,
                now: crate::plan::clears(&offer.candidate),
            })
            .collect();
        offers.extend(self.generation_offers(tick));
        offers.extend(self.garbage_offers(tick));
        offers.extend(self.asked_offers());
        Ok((scan, offers))
    }

    /// Runs a pass over what lies on the watched filesystem `device` and
    /// journals it.
    fn pass_on(
        &mut self,
        tick: &Tick,
        device: u64,
        here: &[Offer],
        (band, goal): (Band, Goal),
        thresholds: Thresholds,
        scan: &mut Scan,
    ) -> io::Result<End> {
        let Some(path) = self
            .watched
            .get(&device)
            .map(|watched| watched.path.clone())
        else {
            return Ok(End::Exhausted);
        };
        let candidates: Vec<_> = here.iter().map(|offer| offer.candidate.clone()).collect();
        let started = Instant::now();
        let gentle = band < Band::Urgent;
        let pacer = Rc::new(RefCell::new(self.pacer(&path, band, thresholds)));
        let reclaimer = Reclaim::new(here, scan)
            .during(band)
            .paced(Rc::clone(&pacer));
        let mut reclaim = Journaled {
            inner: if gentle {
                reclaimer.gently()
            } else {
                reclaimer
            },
            journal: &mut self.journal,
            roots: here
                .iter()
                .map(|offer| (offer.candidate.path.clone(), offer.action.root().to_owned()))
                .collect(),
            at: tick.wall,
            band,
        };
        // A pass that can wait waits for the filesystem's own commits; one
        // that can't asks for them. Either beats while it waits.
        let disk = Filesystem::new(&path).paced(Rc::clone(&pacer));
        let mut disk = if gentle { disk.patient() } else { disk };
        let report = pass(
            &mut disk,
            &mut reclaim,
            &candidates,
            &thresholds,
            band,
            goal,
            &mut self.memory.learned,
        );
        if goal == Goal::Reach {
            let seconds = started.elapsed().as_secs_f64();
            self.memory.pass_seconds = Some(match self.memory.pass_seconds {
                Some(before) => before + PASS_WEIGHT * (seconds - before),
                None => seconds,
            });
        } else if goal == Goal::Upkeep {
            // Kept by the survey before it was offered, or by the pass.
            let unvouched = scan.went_unheard()
                && (self
                    .kept
                    .iter()
                    .any(|(_, used)| matches!(used, crate::activity::Use::Unknown(_)))
                    || report
                        .actions
                        .iter()
                        .any(|(_, outcome)| matches!(outcome, Outcome::Kept(_))));
            self.memory.upkept = Some(if unvouched {
                tick.wall - crate::decide::UPKEEP_EVERY + UNHEARD_AGAIN
            } else {
                tick.wall
            });
        }
        self.note_actions(here, &report.actions, tick.wall);
        let end = ending(&report.end);
        self.journal.write(&Line::Pass {
            at: tick.wall,
            band: report.band,
            end: end.clone(),
            freed: report.freed,
            seconds: started.elapsed().as_secs_f64(),
        })?;
        self.last = Some(LastPass {
            at: tick.wall,
            path,
            band: report.band,
            end,
            freed: report.freed,
        });
        Ok(report.end)
    }

    /// Notes what came of each action a pass took, whatever it was, so a
    /// failing command waits its window too.
    fn note_actions(&mut self, here: &[Offer], actions: &[(PathBuf, Outcome)], wall: f64) {
        for (taken, outcome) in actions {
            if *outcome == Outcome::Reclaimed {
                self.offered.retain(|offer| &offer.path != taken);
            }
            let offered = here.iter().find(|offer| &offer.candidate.path == taken);
            match offered.map(|offer| &offer.action) {
                Some(Action::Run { source, .. }) => {
                    self.memory.ran.insert(source.clone(), wall);
                }
                // What they kept is garbage now, which no listing holds.
                Some(Action::Expire { .. }) if *outcome == Outcome::Reclaimed => {
                    self.relist = true;
                }
                // The listing's other batches would meet the same, so the
                // store is listed again, a listing under way being new
                // enough, and none of them is offered meanwhile.
                Some(Action::Collect { .. }) if stale(outcome) => {
                    self.stale_listing = true;
                    if self.listing_out.is_none() {
                        self.relist = true;
                    }
                }
                _ => {}
            }
        }
    }

    /// What a pass would take now on each filesystem, in `band` and with
    /// `free` bytes free when given, taking nothing. It runs the pass
    /// itself against a disk that frees what learning predicts, so it
    /// shows the order a pass goes in and where it would stop; whether
    /// each would be found in use is only known when it's tried.
    pub fn preview(
        &mut self,
        tick: &Tick,
        band: Option<Band>,
        free: Option<Size>,
    ) -> io::Result<Vec<Preview>> {
        self.memory.awake.observe(tick);
        self.watch();
        let (_, offers) = self.offers(tick, None)?;
        let mut previews = Vec::new();
        for (&device, watched) in &self.watched {
            let Ok(space) = crate::sys::space(&watched.path) else {
                continue;
            };
            let thresholds = Thresholds::new(&self.config.bands, space.total);
            // A band asked for alone is pretended by free space just short
            // of it, so a plan for urgent shows what urgent would take.
            let short_of = match band {
                Some(Band::Pressure) => Some(thresholds.pressure),
                Some(Band::Urgent) => Some(thresholds.urgent),
                Some(Band::Critical) => Some(thresholds.critical),
                Some(Band::Healthy) | None => None,
            };
            let pretended = match (free, short_of) {
                (Some(free), _) => free.of(space.total),
                (None, Some(start)) => space.free.min(start.saturating_sub(1)),
                (None, None) => space.free,
            };
            let space = Space {
                free: pretended,
                ..space
            };
            let band = band.unwrap_or_else(|| thresholds.band(&space, Band::Healthy));
            let goal = if band == Band::Healthy {
                Goal::Upkeep
            } else {
                Goal::Reach
            };
            let candidates: Vec<_> = offers
                .iter()
                .filter(|offer| offer.device == device)
                .map(|offer| offer.candidate.clone())
                .collect();
            let freed = Rc::new(Cell::new(space.free));
            let mut learned = self.memory.learned.clone();
            let report = pass(
                &mut Pretend {
                    space,
                    free: freed.clone(),
                },
                &mut Pretending {
                    learned: self.memory.learned.clone(),
                    free: freed,
                    total: space.total,
                },
                &candidates,
                &thresholds,
                band,
                goal,
                &mut learned,
            );
            let taken = report
                .actions
                .iter()
                .filter_map(|(path, _)| candidates.iter().find(|c| &c.path == path).cloned())
                .collect();
            previews.push(Preview {
                path: watched.path.clone(),
                band,
                free: space.free,
                total: space.total,
                target: thresholds.target,
                end: ending(&report.end),
                taken,
            });
        }
        Ok(previews)
    }

    pub fn status(&self, now: f64) -> Status {
        let bands = &self.config.bands;
        let home = self.home.as_deref();
        let filesystems = self
            .watched
            .values()
            .filter_map(|watched| {
                let space = watched.space?;
                let thresholds = Thresholds::new(bands, space.total);
                let next = match watched.band {
                    Band::Healthy => Some(thresholds.pressure),
                    Band::Pressure => Some(thresholds.urgent),
                    Band::Urgent => Some(thresholds.critical),
                    Band::Critical => None,
                };
                // What a person is told comes from the trend, not the
                // two-minute rate passes are planned by, which a churning
                // disk swings either way by tens of gigabytes an hour.
                let full_in = watched.fill.steady_until(space.free, 0);
                let mut status = FilesystemStatus {
                    path: watched.path.clone(),
                    band: watched.band,
                    free: space.free,
                    total: space.total,
                    size: space.total,
                    used: space.total.saturating_sub(space.free),
                    forecast: next.and_then(|start| watched.fill.steady_until(space.free, start)),
                    fill_per_hour: watched.fill.trend().map(|trend| trend * 3600.0),
                    full_at: full_in.map(|seconds| now + seconds),
                    says: String::new(),
                    exhausted: self.last.as_ref().is_some_and(|last| {
                        last.path == watched.path
                            && last.end == "exhausted"
                            && watched.band != Band::Healthy
                            && last.band >= watched.band
                    }),
                };
                status.says = crate::words::filesystem(&status.room(full_in), home);
                Some(status)
            })
            .collect::<Vec<_>>();
        let rooms: Vec<_> = filesystems
            .iter()
            .map(|status| status.room(status.full_at.map(|at| at - now)))
            .collect();
        let says = crate::words::machine(&rooms, home);
        let cleared = self
            .journal
            .cleared()
            .into_iter()
            .take(CLEARED_SHOWN)
            .map(|one| ClearedStatus {
                says: crate::words::cleared(&one.kind, &one.path, one.bytes, home),
                at: one.at,
                kind: one.kind,
                path: one.path,
                bytes: one.bytes,
            })
            .collect();
        let mut status = Status {
            version: STATUS_VERSION,
            heartbeat: now,
            says,
            filesystems,
            cleared,
            last: self.last.clone(),
            frozen: self.memory.frozen.clone(),
            writer: self
                .writers
                .dominant(WRITING_WINDOW, now)
                .map(|writer| Writing {
                    name: writer.name,
                    pid: writer.pid,
                    rate: writer.rate,
                }),
            notice: None,
            holders: Vec::new(),
            holders_at: None,
        };
        status.notice = crate::notices::standing(&status);
        if let Some((at, measured, grew)) =
            &*self.measured.lock().unwrap_or_else(PoisonError::into_inner)
        {
            status.holders =
                crate::holders::holders(measured, &self.offered, &self.kept, grew, home);
            status.holders_at = Some(*at);
        }
        mark(&mut status, self.state(), home);
        status
    }

    /// The folder the guard keeps its memory, journal and Joel's keeps in.
    fn state(&self) -> &Path {
        self.memory_path.parent().unwrap_or(Path::new("/"))
    }

    /// The pacer for a pass in `band` on the filesystem at `path`. Below
    /// urgent it rests between batches, feeling the disk through a probe
    /// beside the guard's state when that lies on the same filesystem, and
    /// hurries once the disk falls to urgent; from urgent on it never
    /// rests. Either keeps the status file's heartbeat.
    fn pacer(&self, path: &Path, band: Band, thresholds: Thresholds) -> Pacer {
        let clock = Box::new(Monotonic::new());
        if band >= Band::Urgent {
            return Pacer::hurried(clock, self.beating());
        }
        let watched = path.to_owned();
        let pressed = move || {
            crate::sys::space(&watched)
                .is_ok_and(|space| thresholds.band(&space, band) >= Band::Urgent)
        };
        Pacer::paced(
            self.config.pacing,
            clock,
            self.probe(path),
            self.beating(),
            Box::new(pressed),
        )
    }

    /// A probe in the guard's state directory, when that lies on the same
    /// filesystem as `path`. Only on Linux: a Mac's sync flushes the drive's
    /// own cache, which a probe once a second has no business doing.
    fn probe(&self, path: &Path) -> Option<Box<dyn Probe>> {
        if cfg!(not(target_os = "linux")) {
            return None;
        }
        let state = self.memory_path.parent()?;
        if pool(state).ok()? != pool(path).ok()? {
            return None;
        }
        match crate::pace::SyncedWrite::open(state, "probe") {
            Ok(probe) => Some(Box::new(probe)),
            Err(error) => {
                eprintln!("disk-guard: opening its probe: {error}");
                None
            }
        }
    }

    /// Says the guard is alive: the status last written, with a fresh
    /// heartbeat and each filesystem's free space as it is now, so the
    /// desktop neither calls it silent nor shows stale room. A pass's
    /// pacer calls it as it works, and `beat_always`'s thread every
    /// `HEARTBEAT_EVERY`. Until this guard has written a status of its
    /// own it stamps the one the last guard left, so a restart, whose
    /// first step can take minutes, never reads as a guard that stopped.
    pub fn beating(&self) -> Box<dyn FnMut() + Send> {
        let Some(path) = self.status_path.clone() else {
            return Box::new(|| {});
        };
        let beaten = Arc::clone(&self.beaten);
        let bands = self.config.bands.clone();
        let home = self.home.clone();
        let state = self.state().to_owned();
        Box::new(move || {
            let now = wall_now();
            let mut beaten = beaten.lock().unwrap_or_else(PoisonError::into_inner);
            let saved = match beaten.as_mut() {
                None => restamp(&path, now),
                Some(status) => {
                    status.heartbeat = now;
                    for filesystem in &mut status.filesystems {
                        if let Ok(space) = crate::sys::space(&filesystem.path) {
                            filesystem.measured(&space, &bands, now, home.as_deref());
                        }
                    }
                    let rooms: Vec<_> = status
                        .filesystems
                        .iter()
                        .map(|filesystem| filesystem.room(filesystem.full_at.map(|at| at - now)))
                        .collect();
                    status.says = crate::words::machine(&rooms, home.as_deref());
                    mark(status, &state, home.as_deref());
                    state::save(&path, status)
                }
            };
            if let Err(error) = saved {
                eprintln!("disk-guard: writing its status: {error}");
            }
        })
    }

    /// Beats on a thread of its own for as long as the process runs, so
    /// the status file's heartbeat says what the unit's state does: that
    /// the guard is running, however long a step, a delete Nix holds up
    /// or a restart's first look takes. The desktop calls a guard silent
    /// two minutes after its last heartbeat.
    pub fn beat_always(&self) -> io::Result<()> {
        let mut beat = self.beating();
        std::thread::Builder::new()
            .name("heartbeat".to_owned())
            .spawn(move || {
                loop {
                    beat();
                    std::thread::sleep(Duration::from_secs_f64(HEARTBEAT_EVERY));
                }
            })?;
        Ok(())
    }

    fn write_status(&mut self, now: f64) -> io::Result<()> {
        let Some(path) = self.status_path.clone() else {
            return Ok(());
        };
        let mut status = self.status(now);
        // Stamped as it's written, not as its step began: a step can run
        // for minutes, and a status written old reads as a guard stopped.
        status.heartbeat = wall_now();
        let unchanged = self.written.as_ref().is_some_and(|(at, written)| {
            now - at < HEARTBEAT_EVERY
                && Status {
                    heartbeat: status.heartbeat,
                    ..written.clone()
                } == status
        });
        if unchanged {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        // One writer at a time, since a beat saves through the same file.
        let mut beaten = self.beaten.lock().unwrap_or_else(PoisonError::into_inner);
        state::save(&path, &status)?;
        *beaten = Some(status.clone());
        drop(beaten);
        self.written = Some((now, status));
        Ok(())
    }
}

/// Marks a status's holders with the keeps and clears in `state` as they
/// are now, since `disk-guard keep` and `clear` change them between
/// steps. Keeps that won't read leave the holders as they were.
fn mark(status: &mut Status, state: &Path, home: Option<&Path>) {
    if let Ok(keeps) = crate::keeps::load(state) {
        let clearing = crate::clears::clearing(state);
        crate::holders::mark(&mut status.holders, &keeps, &clearing, home);
    }
}

/// The band a decision's pass plans by, and what it's for.
fn aim(decision: Decision) -> (Band, Goal) {
    match decision {
        Decision::Reach(band) => (band, Goal::Reach),
        _ => (Band::Healthy, Goal::Upkeep),
    }
}

/// The time now, in wall seconds.
fn wall_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64())
}

/// Writes the status file a guard left again with a fresh heartbeat and
/// all else as it said, or nothing when there's none or none that reads.
fn restamp(path: &Path, now: f64) -> io::Result<()> {
    match state::load::<serde_json::Map<String, serde_json::Value>>(path) {
        Ok(Some(mut left)) => {
            left.insert("heartbeat".to_owned(), now.into());
            state::save(path, &left)
        }
        _ => Ok(()),
    }
}

/// Whether a user daemon spoke for the filesystem within the minute, so it
/// drives that filesystem's passes.
/// The trees whose builds are spent: every tree a finished quest lists, and
/// every tree whose broker record names no holder still `running`, unless a
/// quest at work lists it.
fn spent_trees(config: &Config, running: impl Fn(u32, f64) -> bool + Copy) -> Vec<PathBuf> {
    config
        .sources
        .values()
        .filter_map(|source| match source {
            Source::Workspaces { quests, trees, .. } => Some((quests, trees)),
            _ => None,
        })
        .flat_map(|(quests, trees)| {
            let unheld = trees.as_deref().map_or_else(Vec::new, |records| {
                crate::workspaces::unheld_trees(records, quests, running)
            });
            crate::workspaces::finished_trees(quests)
                .into_iter()
                .chain(unheld)
        })
        .collect()
}

fn spoken_for(hearing: Option<&Hearing>, device: u64) -> bool {
    hearing.is_some_and(|hearing| {
        let spoken = hearing
            .spoken
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        spoken
            .get(&device)
            .is_some_and(|at| at.elapsed() < SPOKEN_FOR)
    })
}

/// Where the filesystem holding `path`, the pool `device`, is mounted:
/// the highest directory above it still on that pool, which is how a
/// person knows the disk, so a Btrfs pool is named by its topmost mount
/// rather than by whichever subvolume a source lies in. Failing that,
/// `path` itself.
fn mounted(path: &Path, device: u64) -> PathBuf {
    let Ok(mut top) = fs::canonicalize(path) else {
        return path.to_owned();
    };
    while let Some(above) = top.parent()
        && pool(above).is_ok_and(|above| above == device)
    {
        top = above.to_owned();
    }
    top
}

/// Whether a delete failed for garbage that came alive since the listing.
fn stale(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Failed(why) if garbage::stale(why))
}

/// How a pass ended, in the journal's words.
fn ending(end: &End) -> String {
    match end {
        End::Reached => "reached".to_owned(),
        End::Exhausted => "exhausted".to_owned(),
        End::Stalled => "stalled".to_owned(),
        End::Failed(why) => format!("failed: {why}"),
    }
}

/// A disk for a preview, whose free space is what was pretended plus
/// what the pretend reclaimer has freed.
struct Pretend {
    space: Space,
    free: Rc<Cell<u64>>,
}

impl Disk for Pretend {
    fn space(&mut self) -> io::Result<Space> {
        Ok(Space {
            free: self.free.get(),
            ..self.space
        })
    }

    fn settle(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn now(&self) -> f64 {
        0.0
    }
}

/// Frees what learning predicts each candidate would, taking nothing.
struct Pretending {
    learned: Learned,
    free: Rc<Cell<u64>>,
    total: u64,
}

impl Reclaimer for Pretending {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let freed = whole(float(candidate.size) * self.learned.ratio(candidate.class));
        self.free
            .set(self.free.get().saturating_add(freed).min(self.total));
        Outcome::Reclaimed
    }
}
