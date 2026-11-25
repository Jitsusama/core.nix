//! `repos serve`: the repos daemon. It holds the record ready on the
//! socket systemd holds for it, in the shell's daemon interface
//! (docs/daemons.md), split by kind (feed.rs), so the local sites' Code
//! page is told what moved rather than reading every repository again.
//! The record is the command's own, git's files with GitHub's part and the
//! counts merged in, as the launcher's Code rows read it too, so the page
//! and the launcher agree.
//!
//! It holds each checkout as it last read it, and reads one again when
//! inotify says git wrote a name a reading reads in that checkout's
//! folders (heard.rs), once the writing has settled, since one commit or
//! fetch writes several. Agents write in a few checkouts of hundreds of
//! trees each all day, so reading only the one that moved keeps a busy
//! disk cheap. A pace reads everything, walking the roots, for what's not
//! watched: a checkout cloned under a root, and when a tree was last
//! written, which moves with its index.
//!
//! What works in each checkout and tree is read too (using.rs), from
//! `/proc`, and said as each repository's `using:` record, so the Code
//! page shows the processor and memory beside the space. It's read once a
//! pace on a clock of its own, as a share of time wants two readings a
//! pace apart: a whole reading of a busy disk took over three minutes on
//! 25 Nov, longer than two readings may be apart and still measure one,
//! and a reading of every process at each commit would cost the disk's
//! busiest minutes most. Each whole reading hands it the folders found
//! and takes what it read last.
//!
//! Each reading writes Code's homes too, where `REPOS_HOMES` names a file
//! for them, whenever they changed (homes.rs): so they move with the disk,
//! and nothing reads every repository again to write them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use daemon_serve::serve::{self, Shared};
use repos::model::Record;
use repos::read::{self, Found, Reading};
use repos::using::{self, Sample, Use};
use repos::{github, upstreams};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure};

use crate::feed;
use crate::heard::{self, Folders, Heard, Of};
use crate::homes::Homes;

/// How often the disk is read whole with nothing heard.
const PACE: Duration = Duration::from_mins(1);

/// How long the disk is to be quiet after something's heard before it's
/// read: a commit writes its ref, its log and HEAD's in a few
/// milliseconds, and a fetch its refs over a few hundred.
const QUIET: Duration = Duration::from_millis(300);

/// The longest a reading waits for the disk to go quiet, so agents
/// writing without pause are still told within this.
const LONGEST: Duration = Duration::from_secs(2);

/// The least time between two readings' starts, so dozens of agents
/// writing at once cost a reading every two seconds at most, with a view
/// still within a breath of the disk.
const SPACED: Duration = Duration::from_secs(2);

/// What health says beside daemon-serve's own: how many folders are
/// watched, how many couldn't be and, when any couldn't, why the first
/// couldn't, and how long the last reading took.
fn health(watched: usize, (unwatched, because): (usize, Option<String>), read_ms: u64) -> Value {
    let mut said = json!({"watched": watched, "unwatched": unwatched, "read_ms": read_ms});
    if let Some(because) = because {
        said["unwatched_because"] = json!(because);
    }
    said
}

/// Holds the record, read again as the disk moves, for as long as it
/// runs.
pub fn run(home: Option<&Path>) -> Result<(), Failure> {
    // Timed from its start, as health times the unit's, so the line says the whole wait.
    let started = Instant::now();
    let socket = socket()?;
    let shared = Shared::new(feed::DOMAIN, Box::new(|_| None));
    let watched = Arc::new(AtomicUsize::new(0));
    let unwatched = Arc::new(Mutex::new((0, None::<String>)));
    let took = Arc::new(AtomicU64::new(0));
    let (watching, unheard, taking) = (
        Arc::clone(&watched),
        Arc::clone(&unwatched),
        Arc::clone(&took),
    );
    shared.health_too(Box::new(move || {
        let unwatched = unheard.lock().map_or((0, None), |held| held.clone());
        health(
            watching.load(Ordering::Relaxed),
            unwatched,
            taking.load(Ordering::Relaxed),
        )
    }));
    let mut disk = Disk {
        home,
        roots: crate::roots(home)?,
        machine: terminal_kit::environment::machine(),
        found: BTreeMap::new(),
    };
    let (tell, told) = mpsc::channel();
    let mut heard = Heard::start(tell).map_err(|why| deaf(&why.to_string()))?;
    let kept: Vec<PathBuf> = [github::seen_file(home), upstreams::counted_file(home)]
        .into_iter()
        .flatten()
        .collect();
    // A folder is always watched before it's read, so what git writes
    // while it's read is heard and read again rather than lost until the
    // pace; and it's listening only once it's watching, so a view that
    // reaches the socket is told everything from then on.
    let mut folders = Folders::new();
    let mut homes = homes_named();
    let mut began = Instant::now();
    let mut record = timed(&took, || disk.read_whole(&mut heard, &mut folders, &kept));
    let mut working = Working::start(
        PACE,
        PACE * STALE,
        folders_of(&record),
        |folders: &[PathBuf]| using::sample(Path::new("/proc"), folders),
    );
    // What works in each is said unread until /proc answers, which may
    // take its second or never, so the daemon is ready on the disk alone.
    shared.change(working.said(&record), feed::sources(&record));
    let listener = serve::listen(&socket).map_err(|why| {
        let socket = socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    eprintln!("{}", ready(started.elapsed()));
    working.read(folders_of(&record));
    shared.change(working.said(&record), feed::sources(&record));
    // Once views are answered, so writing the homes never holds up the
    // first.
    keep(homes.as_mut(), &record);
    // The pace keeps its own time, so agents writing more often than it
    // comes never put off the reading that finds new clones.
    let mut whole = Instant::now();
    loop {
        watched.store(heard.watched(), Ordering::Relaxed);
        if let Ok(mut held) = unwatched.lock() {
            *held = (
                heard.unwatched(),
                heard.unwatched_because().map(str::to_owned),
            );
        }
        record = match told.recv_timeout(PACE.saturating_sub(whole.elapsed())) {
            Ok(moved) if whole.elapsed() < PACE => {
                let moved = settle(&told, moved, began)?;
                began = Instant::now();
                follow_again(&mut folders, &moved);
                heard.follow(&folders, |of| moved.contains(of));
                timed(&took, || disk.read_again(&moved))
            }
            Ok(_) | Err(RecvTimeoutError::Timeout) => {
                began = Instant::now();
                whole = began;
                timed(&took, || disk.read_whole(&mut heard, &mut folders, &kept))
            }
            Err(RecvTimeoutError::Disconnected) => return Err(deaf("inotify stopped")),
        };
        // Every reading of the disk, whole or not, holds every folder, so
        // each hands them on and takes what /proc has read since.
        working.read(folders_of(&record));
        shared.change(working.said(&record), feed::sources(&record));
        keep(homes.as_mut(), &record);
    }
}

/// How long the loop waits for the first reading of what works in each
/// tree, so the daemon's first answers say it where `/proc` is quick.
const SAMPLED: Duration = Duration::from_secs(1);

/// How many of its paces old the reading of `/proc` may grow, with none
/// newer, before what it read goes unsaid. Counted on its own clock
/// rather than in readings of the disk, though judged as each of those
/// ends, at least each `PACE`, so it's said late by a pace, or by one
/// reading of a disk slower than that, at most.
const STALE: u32 = 3;

/// Every checkout's and tree's folder `record` holds.
fn folders_of(record: &Record) -> Vec<PathBuf> {
    record
        .repositories
        .iter()
        .flat_map(|repository| &repository.checkouts)
        .flat_map(|checkout| {
            std::iter::once(&checkout.path).chain(checkout.trees.iter().map(|tree| &tree.path))
        })
        .map(PathBuf::from)
        .collect()
}

/// One reading of what works in each folder.
struct Sampled {
    used: BTreeMap<PathBuf, Use>,
    /// Whether it measured a share of processor time.
    measured: bool,
    /// The folders it placed processes in, so one it didn't know of yet
    /// is said unread rather than idle.
    placed: BTreeSet<PathBuf>,
    /// When it was read, so its age is judged on its own clock.
    at: Instant,
}

/// What the loop and the thread reading `/proc` share.
#[derive(Default)]
struct Held {
    /// The folders the next reading places processes in, as the last
    /// reading of the disk found them.
    folders: Vec<PathBuf>,
    /// The newest reading not yet taken. Each replaces the one before, as
    /// each list of folders does, so a loop or a reading held up for hours
    /// leaves no pile of either behind.
    newest: Option<Sampled>,
}

/// What worked in each checkout and tree at the last reading. It's read on
/// a thread of its own, at its own pace, since a read of a process's
/// memory waits while the process holds its memory map, which may be
/// never, the loop answers every view, and a whole reading of the disk
/// may take longer than two readings of `/proc` may be apart.
struct Working {
    /// What's shared with the thread, and the signal of a new reading.
    held: Arc<(Mutex<Held>, Condvar)>,
    /// How old a reading may grow, with none newer, before what it read
    /// goes unsaid: `STALE` paces.
    old: Duration,
    /// When reading began, so a first reading that never answers is said
    /// in the journal as late as one that stopped.
    since: Instant,
    /// Whether the first reading has been waited on.
    waited: bool,
    /// When the reading said was read, once any has answered: until one
    /// has, nothing says what works, which isn't that nothing does.
    at: Option<Instant>,
    /// Whether the last reading has grown `STALE` paces old with none
    /// newer, so what it read goes unsaid.
    stale: bool,
    used: BTreeMap<PathBuf, Use>,
    measured: bool,
    placed: BTreeSet<PathBuf>,
}

/// Takes `held` whatever thread last held it: each holds it only to copy
/// or put a value, so a holder that panicked left no half-written state.
fn holding<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Working {
    /// Starts reading what works in each of `folders` every `pace`, and at
    /// least half a pace apart however long each takes, since each walk
    /// reads every placed process's memory; what a reading read goes
    /// unsaid once it's `old`. Once it's dropped, the thread ends at its
    /// next wake, or at the end of the reading it's in, should that wake
    /// come while one is held up.
    fn start(
        pace: Duration,
        old: Duration,
        folders: Vec<PathBuf>,
        mut sample: impl FnMut(&[PathBuf]) -> Sample + Send + 'static,
    ) -> Self {
        let held = Arc::new((
            Mutex::new(Held {
                folders,
                newest: None,
            }),
            Condvar::new(),
        ));
        let shared = Arc::clone(&held);
        std::thread::spawn(move || {
            let mut last: Option<Sample> = None;
            // None until two readings have said whether a share is
            // measured, so the first says why it isn't.
            let mut measuring: Option<bool> = None;
            // The folders the reading before looked in: a process in one
            // it didn't had no time to measure its share from.
            let mut looked: BTreeSet<PathBuf> = BTreeSet::new();
            // Only the loop and this thread hold it, so it alone means
            // the loop has gone.
            while Arc::strong_count(&shared) > 1 {
                let began = Instant::now();
                let folders = holding(&shared.0).folders.clone();
                let now = sample(&folders);
                let measured = using::measured(last.as_ref(), &now);
                if let Some(why) = measures(last.as_ref(), &now, measuring) {
                    eprintln!("repos: {why}");
                }
                if last.is_some() {
                    measuring = Some(measured);
                }
                let mut used = using::using(last.as_ref(), &now);
                for (folder, one) in &mut used {
                    if !looked.contains(folder) {
                        one.cpu = None;
                    }
                }
                looked = folders.into_iter().collect();
                // Its age counts from when its walk began, so one held up
                // for minutes is as old as the first process it read.
                let reading = Sampled {
                    used,
                    measured,
                    placed: looked.clone(),
                    at: began,
                };
                last = Some(now);
                holding(&shared.0).newest = Some(reading);
                shared.1.notify_all();
                std::thread::sleep(pace.saturating_sub(began.elapsed()).max(pace / 2));
            }
        });
        Self {
            held,
            old,
            since: Instant::now(),
            waited: false,
            at: None,
            stale: false,
            used: BTreeMap::new(),
            measured: false,
            placed: BTreeSet::new(),
        }
    }

    /// Takes the newest reading of what works in each folder, and hands
    /// the next ones `folders`. Only the first is waited on, `SAMPLED` at
    /// most, so one stuck holds up the loop once. Its processor time is
    /// measured over the pace before it, and what was read goes unsaid once
    /// it's `old` with none newer, judged as it's taken too, so a reading
    /// that waited in the slot past that is never said as now. Where none
    /// has answered by then, the journal says so, as a restart reaching a
    /// process still holding its memory map would leave it.
    fn read(&mut self, folders: Vec<PathBuf>) {
        let (lock, ready) = &*self.held;
        let mut held = holding(lock);
        held.folders = folders;
        if held.newest.is_none() && !self.waited {
            held = ready
                .wait_timeout_while(held, SAMPLED, |held| held.newest.is_none())
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        self.waited = true;
        let found = held.newest.take();
        drop(held);
        if let Some(found) = found {
            self.used = found.used;
            self.measured = found.measured;
            self.placed = found.placed;
            self.at = Some(found.at);
        }
        let age = self.at.unwrap_or(self.since).elapsed();
        if age < self.old {
            if self.stale {
                eprintln!("repos: the reading of /proc answers again");
            }
            self.stale = false;
            return;
        }
        let was = self.stale;
        self.stale = true;
        self.used.clear();
        self.measured = false;
        if was {
            return;
        }
        // A restart reads the same process again, and waits on it again,
        // so the line names it rather than offering one.
        let held = using::reading().map_or(String::new(), |pid| {
            format!(", held up reading the memory of process {pid}")
        });
        let late = if self.at.is_some() {
            format!(
                "the reading of /proc is {} s old with none newer",
                age.as_secs()
            )
        } else {
            format!("no reading of /proc has answered in {} s", age.as_secs())
        };
        eprintln!(
            "repos: {late}{held}, so what works in each repository is said unread until it \
             answers"
        );
    }

    /// Whether what works in each repository can't be said: no reading has
    /// answered yet, or the last has grown `STALE` paces old.
    fn unread(&self) -> bool {
        self.at.is_none() || self.stale
    }

    /// Every record the feed says: the disk's, and what works in each
    /// repository as last read, or unread where it can't be said.
    fn said(&self, record: &Record) -> Vec<Value> {
        let mut said = feed::records(record);
        if self.unread() {
            said.extend(feed::unread(record));
        } else {
            said.extend(feed::using(record, &self.used, self.measured, &self.placed));
        }
        said
    }
}

/// Why a share of processor time is, or is no longer, measured by `now`
/// after `before`, where `was` says whether the readings before measured
/// one, for the journal: a share said as null tells no one why. Nothing
/// where nothing changed worth saying: no pair yet, a share measured from
/// the start, or measured or not as the readings before were, so a loss
/// is said once, and again only once it's ended.
fn measures(before: Option<&Sample>, now: &Sample, was: Option<bool>) -> Option<String> {
    let measured = using::measured(before, now);
    if before.is_none() || was == Some(measured) {
        return None;
    }
    if measured {
        return (was == Some(false)).then(|| "the share of a core is measured again".to_owned());
    }
    let seconds = |ticks: u64| ticks / using::TICKS;
    let why = match (before.map(|before| before.awake), now.awake) {
        _ if now.at == 0 => "/proc couldn't be read".to_owned(),
        (_, None) => format!("this walk of /proc took over {} s", seconds(using::EVEN)),
        (None | Some(None), _) => format!(
            "the walk of /proc before this one wasn't read whole within {} s",
            seconds(using::EVEN)
        ),
        (Some(Some(before)), Some(now)) if now <= before => {
            "no waking time passed since the reading before".to_owned()
        }
        (Some(Some(_)), Some(_)) => format!(
            "more than {} s of waking time passed since the reading before, the most a share \
             is measured over",
            seconds(using::LONGEST)
        ),
    };
    Some(format!("the share of a core goes unmeasured, as {why}"))
}

/// Code's homes, at the file `REPOS_HOMES` names, where it names one.
fn homes_named() -> Option<Homes> {
    std::env::var_os("REPOS_HOMES")
        .filter(|file| !file.is_empty())
        .map(|file| Homes::at(PathBuf::from(file)))
}

/// Writes the homes `record` gives, saying to the journal why they
/// couldn't be whenever the reason changes; the last homes stay meanwhile.
fn keep(homes: Option<&mut Homes>, record: &Record) {
    if let Some(said) = homes.and_then(|homes| homes.keep(record)) {
        eprintln!("{said}");
    }
}

/// The disk as last read: each checkout by its own git folder.
struct Disk<'a> {
    home: Option<&'a Path>,
    roots: Vec<PathBuf>,
    machine: String,
    found: BTreeMap<PathBuf, Found>,
}

impl Disk<'_> {
    /// Walks the roots for every checkout, watches each one's folders and
    /// the kept files, then reads each, so a write while it reads is heard.
    fn read_whole(&mut self, heard: &mut Heard, folders: &mut Folders, kept: &[PathBuf]) -> Record {
        let commons = read::commons(&self.roots);
        *folders = heard::folders(commons.iter(), kept);
        heard.follow(folders, |_| true);
        self.found = commons
            .into_iter()
            .map(|common| {
                let found = read::checkout(&common, self.home);
                (common, found)
            })
            .collect();
        self.record()
    }

    /// Reads again only the checkouts that `moved`, letting go of any now
    /// gone; the timers' files moving needs no checkout read.
    fn read_again(&mut self, moved: &BTreeSet<Of>) -> Record {
        for of in moved {
            let Of::Checkout(common) = of else {
                continue;
            };
            if common.is_dir() {
                let found = read::checkout(common, self.home);
                self.found.insert(common.clone(), found);
            } else {
                self.found.remove(common);
            }
        }
        self.record()
    }

    /// The record of the checkouts as held, with the timers' parts merged
    /// in as the command merges them.
    fn record(&self) -> Record {
        let reading = Reading {
            roots: &self.roots,
            home: self.home,
            machine: &self.machine,
            now: crate::now(),
        };
        let mut record = read::assemble(&reading, self.found.values().cloned());
        crate::kept(self.home, &mut record);
        record
    }
}

/// The folders to watch before the checkouts that `moved` are read again:
/// theirs named afresh while they're still there, everyone else's as
/// they were.
fn follow_again(folders: &mut Folders, moved: &BTreeSet<Of>) {
    folders.retain(|_, watch| !moved.contains(&watch.of) || watch.of == Of::Kept);
    for of in moved {
        if let Of::Checkout(common) = of
            && common.is_dir()
        {
            folders.extend(heard::checkout(common));
        }
    }
}

/// The one line it says once its first reading is held and it listens,
/// which health times a restart to (modules/health/src/launches.rs).
fn ready(took: Duration) -> String {
    let ms = took.as_millis();
    format!("repos: ready in {ms} ms, every repository read and the socket listening")
}

/// What `reading` gives, saying how long it took for health.
fn timed(took: &AtomicU64, reading: impl FnOnce() -> Record) -> Record {
    let began = Instant::now();
    let record = reading();
    let ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
    took.store(ms, Ordering::Relaxed);
    record
}

/// Waits for the disk to go quiet, or for `LONGEST` at most, and until
/// `SPACED` has passed since the last reading `began`, gathering whose
/// folders moved meanwhile with what was first heard.
fn settle(
    told: &Receiver<BTreeSet<Of>>,
    mut moved: BTreeSet<Of>,
    began: Instant,
) -> Result<BTreeSet<Of>, Failure> {
    let heard = Instant::now();
    while heard.elapsed() < LONGEST {
        match told.recv_timeout(QUIET) {
            Ok(more) => moved.extend(more),
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => return Err(deaf("inotify stopped")),
        }
    }
    if let Some(left) = SPACED.checked_sub(began.elapsed()) {
        std::thread::sleep(left);
    }
    while let Ok(more) = told.try_recv() {
        moved.extend(more);
    }
    Ok(moved)
}

/// That it can no longer hear the disk, so systemd starts it again rather
/// than it serving a record that never moves.
fn deaf(why: &str) -> Failure {
    Failure::new(Exit::Failed, "repos serve couldn't hear the disk").because(why.to_owned())
}

/// Where it serves: systemd's socket when it hands one, else the runtime
/// folder's, where every client looks.
fn socket() -> Result<PathBuf, Failure> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|folder| !folder.is_empty())
        .map(|folder| PathBuf::from(folder).join("daemons/repos.sock"))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's nowhere to serve")
                .because("XDG_RUNTIME_DIR isn't set")
        })
}

#[cfg(test)]
mod tests;
