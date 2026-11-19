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

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use daemon_serve::serve::{self, Shared};
use repos::model::Record;
use repos::read::{self, Found, Reading};
use repos::{github, upstreams};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure};

use crate::feed;
use crate::heard::{self, Folders, Heard, Of};

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
    let mut began = Instant::now();
    let mut record = timed(&took, || disk.read_whole(&mut heard, &mut folders, &kept));
    shared.change(feed::records(&record), feed::sources(&record));
    let listener = serve::listen(&socket).map_err(|why| {
        let socket = socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    eprintln!("{}", ready(started.elapsed()));
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
        shared.change(feed::records(&record), feed::sources(&record));
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
