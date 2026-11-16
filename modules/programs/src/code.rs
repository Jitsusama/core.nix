//! Code's rows as this daemon serves them (`code_rows.rs`): every
//! repository on the machine, read from git's own files by the repos
//! library (modules/repos) under the folders `REPOS_ROOTS` names, with no
//! process started. The first watch of them waits for the first read,
//! outside the daemon's lock, so its answer holds them; every watch after
//! is answered from the rows held at once, and the disk is read again on
//! a thread of its own, telling each watch only the rows that read
//! differently and those gone. A read takes about 60 ms warm and 230 ms
//! cold over 18 repositories and 800 trees (#4653), well past the
//! launcher's 50 ms, which is why no watch but the first waits on one.
//! Each read takes in what the repos timer's last refresh kept of each
//! repository's open changes, as the repos command does, so a row
//! counts them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use programs::{code_rows, command};
use repos::github;
use repos::read::{Reading, read};

use super::{Daemon, Shared, keep, lock};

/// Where Code looks and how it words its rows, set as the daemon starts.
pub struct Code {
    roots: Vec<PathBuf>,
    home: Option<PathBuf>,
    machine: String,
    words: code_rows::Words,
    /// Whether the disk has been read once, so a watch knows whether rows
    /// are held to answer with.
    read_once: AtomicBool,
    /// Whether a read is under way on a thread of its own, so watches
    /// beginning together read the disk once.
    reading: AtomicBool,
}

impl Code {
    /// Code's settings from the environment: the folders `REPOS_ROOTS`
    /// names, colon-separated as PATH is, else ~/src, as repos itself reads
    /// them.
    pub fn new(home: &str, vocabulary: &serde_json::Value) -> Code {
        let home = (!home.is_empty()).then(|| PathBuf::from(home));
        let mut roots: Vec<PathBuf> = std::env::var("REPOS_ROOTS")
            .unwrap_or_default()
            .split(':')
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
            .collect();
        if roots.is_empty() {
            roots.extend(home.as_ref().map(|home| home.join("src")));
        }
        Code {
            roots,
            home,
            machine: machine(),
            words: code_rows::Words::read(vocabulary),
            read_once: AtomicBool::new(false),
            reading: AtomicBool::new(false),
        }
    }

    /// Code's rows as the disk holds them now.
    fn rows(&self) -> Vec<serde_json::Value> {
        let mut record = read(&Reading {
            roots: &self.roots,
            home: self.home.as_deref(),
            machine: &self.machine,
            now: programs::now_ms() / 1000,
        });
        if let Some(kept) = github::seen_file(self.home.as_deref()) {
            github::merge_kept(&mut record, &kept);
        }
        code_rows::rows(&record, self.home.as_deref(), &self.words)
    }
}

/// As a watch of Code's rows begins: the first ever reads the disk on
/// this thread, outside the lock, so its answer holds them; any later one
/// is answered from the rows held, and the disk read again behind it.
pub fn begin(shared: &Arc<Shared>) {
    if shared.code.read_once.load(Ordering::Acquire) {
        again(shared);
        return;
    }
    let rows = shared.code.rows();
    take(shared, &mut lock(&shared.daemon), rows);
}

/// Reads the disk again on a thread of its own, unless a read is under way.
fn again(shared: &Arc<Shared>) {
    if shared.code.reading.swap(true, Ordering::AcqRel) {
        return;
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let rows = shared.code.rows();
        take(&shared, &mut lock(&shared.daemon), rows);
        shared.code.reading.store(false, Ordering::Release);
    });
}

/// Keeps the rows read, telling each watch only what moved.
fn take(shared: &Shared, daemon: &mut Daemon, rows: Vec<serde_json::Value>) {
    let moved = daemon.code.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
    shared.code.read_once.store(true, Ordering::Release);
}

/// New Terminal Here on a repository's row: niri starts kitty in its
/// checkout, rather than this daemon, so the terminal is no child of a
/// service a switch restarts, and niri, which spawned it, gives its window
/// the keys. Answered at once and asked on a thread of its own, as a
/// kitty tab's acts are (tabs.rs), so no other ask waits on niri.
pub fn terminal(id: &serde_json::Value, folder: &str) -> serde_json::Value {
    let niri = std::env::var("PROGRAMS_NIRI").unwrap_or_else(|_| "niri".to_owned());
    let argv: Vec<String> = [
        niri.as_str(),
        "msg",
        "action",
        "spawn",
        "--",
        "kitty",
        "--directory",
        folder,
    ]
    .iter()
    .map(|&word| word.to_owned())
    .collect();
    std::thread::spawn(move || {
        if let Err(why) = command::run(&argv) {
            eprintln!("a terminal wasn't started: {why}");
        }
    });
    serde_json::json!({"id": id, "asked": "terminal"})
}

/// The machine's name as it calls itself, which the record carries.
fn machine() -> String {
    ["/proc/sys/kernel/hostname", "/etc/hostname"]
        .iter()
        .find_map(|file| {
            std::fs::read_to_string(Path::new(file))
                .ok()
                .map(|name| name.trim().to_owned())
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "this machine".to_owned())
}
