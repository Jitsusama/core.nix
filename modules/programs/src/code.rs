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
//!
//! A checkout's files are listed only while a watch holds its Open a File
//! by Name row ahead (ready-rows.md's "What's read only while it's looked
//! at"), since one holds thousands: listed as Neovim's finder lists them
//! (repositories.rs), on the thread that follows the watches, outside the
//! lock, and served inside the row. Once no watch names the row, its
//! files go.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

use programs::{code_rows, command, repositories};
use repos::github;
use repos::read::{Reading, read};
use serde_json::Value;

use super::{Daemon, Shared, keep, keys_watched, lock};

/// Hears each watch begin or end, on a thread of its own, and hands the
/// keys watched then to `ahead`. daemon-serve says so with every record
/// held, so it's only passed on there, and the keys read here after.
pub fn follow(shared: &Arc<Shared>) {
    let (moved, heard) = mpsc::channel();
    shared.served().on_watches(Box::new(move || {
        let _ = moved.send(());
    }));
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        while heard.recv().is_ok() {
            let _ = heard.try_iter().count();
            let watched = keys_watched(&lock(&shared.daemon));
            ahead(&shared, &watched);
        }
    });
}

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
    /// The rows the last read made: each repository's and those inside it.
    made: Mutex<Vec<Value>>,
    /// The files listed of each Open a File by Name row ahead, by its key.
    listed: Mutex<BTreeMap<String, Vec<Value>>>,
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
            made: Mutex::new(Vec::new()),
            listed: Mutex::new(BTreeMap::new()),
        }
    }

    /// Code's rows as the disk holds them now, and the rows inside them.
    fn rows(&self) -> Vec<Value> {
        let mut record = read(&Reading {
            roots: &self.roots,
            home: self.home.as_deref(),
            machine: &self.machine,
            now: programs::now_ms() / 1000,
        });
        if let Some(kept) = github::seen_file(self.home.as_deref()) {
            github::merge_kept(&mut record, &kept);
        }
        let mut rows = code_rows::rows(&record, self.home.as_deref(), &self.words);
        rows.extend(code_rows::inside(&record, &self.words));
        rows
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
fn take(shared: &Shared, daemon: &mut Daemon, rows: Vec<Value>) {
    *lock(&shared.code.made) = rows;
    served(shared, daemon);
    shared.code.read_once.store(true, Ordering::Release);
}

/// Serves the rows the last read made and the files listed inside them,
/// telling each watch only what moved.
fn served(shared: &Shared, daemon: &mut Daemon) {
    let mut rows = lock(&shared.code.made).clone();
    for (parent, files) in lock(&shared.code.listed).iter() {
        rows.extend(code_rows::files(parent, files, &shared.code.words));
    }
    let moved = daemon.code.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}

/// As the watches move: lists the files of each Open a File by Name row a
/// watch now names, on this thread and outside the lock, and lets go of
/// those of each no watch names any more, then serves them. Those still
/// named keep what was listed as their row came ahead.
pub fn ahead(shared: &Shared, watched: &[String]) {
    let wanted: Vec<&String> = watched
        .iter()
        .filter(|key| key.starts_with("row:code:") && key.ends_with(code_rows::FILES))
        .collect();
    let was = lock(&shared.code.listed).clone();
    if wanted.len() == was.len() && wanted.iter().all(|key| was.contains_key(*key)) {
        return;
    }
    let checkouts: Vec<(String, Option<String>)> = {
        let made = lock(&shared.code.made);
        wanted
            .iter()
            .map(|key| {
                let row = made.iter().find(|row| row["key"] == key.as_str());
                let path = row.and_then(|row| row["held"]["path"].as_str());
                ((*key).clone(), path.map(str::to_owned))
            })
            .collect()
    };
    let listed: BTreeMap<String, Vec<Value>> = checkouts
        .into_iter()
        .filter_map(|(key, path)| {
            let files = match was.get(&key) {
                Some(files) => files.clone(),
                None => repositories::files(&path?),
            };
            Some((key, files))
        })
        .collect();
    *lock(&shared.code.listed) = listed;
    served(shared, &mut lock(&shared.daemon));
}

/// New Terminal Here on a repository's row: niri starts kitty in its
/// checkout, rather than this daemon, so the terminal is no child of a
/// service a switch restarts, and niri, which spawned it, gives its window
/// the keys. Answered at once and asked on a thread of its own, as a
/// kitty tab's acts are (tabs.rs), so no other ask waits on niri.
pub fn terminal(id: &Value, folder: &str) -> Value {
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
