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
//! Each pace reads what works in each checkout and tree too (using.rs),
//! from `/proc`, and says it as each repository's `using:` record, so the
//! Code page shows the processor and memory beside the space. It's read
//! only at the pace, as a share of time wants two readings a while apart,
//! and a reading of every process at each commit would cost the disk's
//! busiest minutes most.
//!
//! Each reading writes Code's homes too, where `REPOS_HOMES` names a file
//! for them, whenever they changed (homes.rs): so they move with the disk,
//! and nothing reads every repository again to write them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
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
    let mut working =
        Working::start(|folders: &[PathBuf]| using::sample(Path::new("/proc"), folders));
    working.read(folders_of(&record));
    shared.change(working.said(&record), feed::sources(&record));
    let listener = serve::listen(&socket).map_err(|why| {
        let socket = socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    eprintln!("{}", ready(started.elapsed()));
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
                let record = timed(&took, || disk.read_whole(&mut heard, &mut folders, &kept));
                working.read(folders_of(&record));
                record
            }
            Err(RecvTimeoutError::Disconnected) => return Err(deaf("inotify stopped")),
        };
        shared.change(working.said(&record), feed::sources(&record));
        keep(homes.as_mut(), &record);
    }
}

/// How long the loop waits at the pace for what works in each tree.
const SAMPLED: Duration = Duration::from_secs(1);

/// How many paces may pass with no reading before what was last read
/// goes unsaid.
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

/// What one reading found in each folder, and whether it measured a share.
type Measured = (BTreeMap<PathBuf, Use>, bool);

/// What worked in each checkout and tree at the last pace. It's read on a
/// thread of its own, since a read of a process's memory waits while the
/// process holds its memory map, which may be never, and the loop answers
/// every view.
struct Working {
    ask: mpsc::Sender<Vec<PathBuf>>,
    told: Receiver<Measured>,
    /// Whether a reading was asked for and hasn't answered.
    asked: bool,
    /// Paces in a row with no reading taken.
    missed: u32,
    used: BTreeMap<PathBuf, Use>,
    measured: bool,
}

impl Working {
    fn start(mut sample: impl FnMut(&[PathBuf]) -> Sample + Send + 'static) -> Self {
        let (ask, asked) = mpsc::channel::<Vec<PathBuf>>();
        let (tell, told) = mpsc::channel();
        std::thread::spawn(move || {
            let mut last: Option<Sample> = None;
            for folders in asked {
                let now = sample(&folders);
                let found = (
                    using::using(last.as_ref(), &now),
                    using::measured(last.as_ref(), &now),
                );
                last = Some(now);
                if tell.send(found).is_err() {
                    return;
                }
            }
        });
        Self {
            ask,
            told,
            asked: false,
            missed: 0,
            used: BTreeMap::new(),
            measured: false,
        }
    }

    /// Reads what works in each of `folders`, waiting `SAMPLED` at most,
    /// and only while no earlier reading is still out: one stuck holds up
    /// the loop once. Its processor time is measured once time has passed
    /// since the last, and what was read goes unsaid once `STALE` paces
    /// pass without a reading.
    fn read(&mut self, folders: Vec<PathBuf>) {
        let mut found = self.told.try_iter().last();
        if found.is_some() {
            self.asked = false;
        }
        if !self.asked && self.ask.send(folders).is_ok() {
            self.asked = true;
            if let Ok(now) = self.told.recv_timeout(SAMPLED) {
                found = Some(now);
                self.asked = false;
            }
        }
        if let Some((used, measured)) = found {
            self.used = used;
            self.measured = measured;
            self.missed = 0;
        } else {
            self.missed += 1;
            if self.missed >= STALE {
                self.used.clear();
                self.measured = false;
            }
            if self.missed == STALE {
                eprintln!(
                    "repos: the reading of /proc hasn't answered in {STALE} minutes, so what \
                     works in each repository is said unread until it does; a restart reads \
                     it again"
                );
            }
        }
    }

    /// Every record the feed says: the disk's, and what works in each
    /// repository as last read, or unread once the reading has gone stale.
    fn said(&self, record: &Record) -> Vec<Value> {
        let mut said = feed::records(record);
        if self.missed >= STALE {
            said.extend(feed::unread(record));
        } else {
            said.extend(feed::using(record, &self.used, self.measured));
        }
        said
    }
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
