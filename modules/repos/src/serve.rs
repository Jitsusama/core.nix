//! `repos serve`: the repos daemon. It holds the record ready on the
//! socket systemd holds for it, in the shell's daemon interface
//! (docs/daemons.md), split by kind (feed.rs), so the local sites' Code
//! page is told what moved rather than reading every repository again.
//! The record is the command's own, git's files with GitHub's part and the
//! counts merged in, as the launcher's Code rows read it too, so the page
//! and the launcher agree.
//!
//! It reads the disk again when inotify says git wrote a name a reading
//! reads (heard.rs), once the writing has settled, since one commit or
//! fetch writes several. A pace reads it too, for what's not watched: a
//! checkout cloned under a root, and when a tree was last written, which
//! moves with its index. A reading takes about 60 ms: a quiet disk costs
//! one a minute, and a busy one at most one every two seconds.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use daemon_serve::serve::{self, Shared};
use repos::model::Record;
use repos::{github, upstreams};
use serde_json::json;
use terminal_kit::{Exit, Failure};

use crate::{feed, heard};

/// How often the disk is read with nothing heard.
const PACE: Duration = Duration::from_mins(1);

/// How long the disk is to be quiet after something's heard before it's
/// read: a commit writes its ref, its log and HEAD's in a few
/// milliseconds, and a fetch its refs over a few hundred.
const QUIET: Duration = Duration::from_millis(300);

/// The longest a reading waits for the disk to go quiet, so agents
/// writing without pause are still told within this.
const LONGEST: Duration = Duration::from_secs(2);

/// The least time between two readings' starts. Dozens of agents write
/// in their trees all day, and each reading costs about 60 ms, so this
/// keeps the daemon to a few readings a minute however busy they are,
/// with a view still within a breath of the disk.
const SPACED: Duration = Duration::from_secs(2);

/// Holds the record, read again as the disk moves, for as long as it
/// runs.
pub fn run(home: Option<&Path>) -> Result<(), Failure> {
    let socket = socket()?;
    let shared = Shared::new(feed::DOMAIN, Box::new(|_| None));
    // What health says beside daemon-serve's own: how many folders are
    // watched, and how long the last reading took.
    let watched = Arc::new(AtomicUsize::new(0));
    let took = Arc::new(AtomicU64::new(0));
    let (watching, taking) = (Arc::clone(&watched), Arc::clone(&took));
    shared.health_too(Box::new(move || {
        json!({
            "watched": watching.load(Ordering::Relaxed),
            "read_ms": taking.load(Ordering::Relaxed),
        })
    }));
    let mut began = Instant::now();
    let mut record = read(home, &took)?;
    shared.change(feed::records(&record), feed::sources(&record));
    let listener = serve::listen(&socket).map_err(|why| {
        let socket = socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    let (tell, told) = mpsc::channel();
    let mut heard = heard::Heard::start(tell).map_err(|why| deaf(&why.to_string()))?;
    let kept: Vec<PathBuf> = [github::seen_file(home), upstreams::counted_file(home)]
        .into_iter()
        .flatten()
        .collect();
    loop {
        heard.follow(heard::folders(&record, &kept));
        watched.store(heard.watched(), Ordering::Relaxed);
        match told.recv_timeout(PACE) {
            Ok(()) => settle(&told, began)?,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(deaf("inotify stopped")),
        }
        began = Instant::now();
        record = read(home, &took)?;
        shared.change(feed::records(&record), feed::sources(&record));
    }
}

/// The record as the command reads it, and how long that took.
fn read(home: Option<&Path>, took: &AtomicU64) -> Result<Record, Failure> {
    let began = Instant::now();
    let record = crate::record(home, crate::now())?;
    let ms = u64::try_from(began.elapsed().as_millis()).unwrap_or(u64::MAX);
    took.store(ms, Ordering::Relaxed);
    Ok(record)
}

/// Waits for the disk to go quiet, or for `LONGEST` at most, and until
/// `SPACED` has passed since the last reading `began`. What's heard
/// meanwhile is taken in by the reading that follows.
fn settle(told: &Receiver<()>, began: Instant) -> Result<(), Failure> {
    let heard = Instant::now();
    while heard.elapsed() < LONGEST {
        match told.recv_timeout(QUIET) {
            Ok(()) => {}
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => return Err(deaf("inotify stopped")),
        }
    }
    if let Some(left) = SPACED.checked_sub(began.elapsed()) {
        std::thread::sleep(left);
    }
    while told.try_recv().is_ok() {}
    Ok(())
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
