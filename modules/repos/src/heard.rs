//! Hearing the disk move, so the daemon (serve.rs) reads it again only
//! when git has written something a reading says. inotify watches folders,
//! not whole trees of them, so each git folder is watched by itself: every
//! checkout's own, the folders of its refs, its reftable, its `worktrees/`
//! and each tree's folder in it, and the folder the timers keep GitHub's
//! part and the counts in. A large repository holds hundreds of trees, so
//! no tree's own files are watched, only what its checkout keeps of it.
//!
//! Of what's written in them, only the names a reading reads are heeded:
//! HEAD, packed-refs, config, the refs and the reftable's list, and each
//! tree coming or going. The index is left out, since git rewrites it at
//! every status an agent asks for; when a tree was last written moves with
//! the daemon's pace instead. A lock file is git's own on the way to the
//! name it renames to, which is heard as it lands.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};

use inotify::{Inotify, WatchDescriptor, WatchMask, Watches};
use repos::model::Record;

/// What a watched folder is, which says the names in it that matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Folder {
    /// A checkout's own git folder, or a tree's in its `worktrees/`.
    Git,
    /// A folder of refs, at any depth under `refs/`.
    Refs,
    /// A reftable, whose list names its tables.
    Reftable,
    /// A checkout's `worktrees/`, a folder a tree.
    Trees,
    /// Where a timer keeps a file every reading merges in, by its names.
    Kept(BTreeSet<String>),
}

impl Folder {
    /// Whether a name written in this folder changes what a reading says.
    pub fn heeds(&self, name: &str) -> bool {
        // git names its locks in lower case alone.
        if Path::new(name)
            .extension()
            .is_some_and(|extension| extension == "lock")
        {
            return false;
        }
        match self {
            Folder::Git => matches!(
                name,
                "HEAD" | "packed-refs" | "config" | "refs" | "reftable" | "worktrees"
            ),
            Folder::Refs | Folder::Trees => true,
            Folder::Reftable => name == "tables.list",
            Folder::Kept(names) => names.contains(name),
        }
    }
}

/// Every folder to watch for `record`, and what each is; `kept` names the
/// files the timers keep.
pub fn folders(record: &Record, kept: &[PathBuf]) -> BTreeMap<PathBuf, Folder> {
    let mut folders = BTreeMap::new();
    let checkouts = record
        .repositories
        .iter()
        .flat_map(|repository| &repository.checkouts);
    for checkout in checkouts {
        let path = PathBuf::from(&checkout.path);
        let common = if checkout.bare {
            path
        } else {
            path.join(".git")
        };
        if !common.is_dir() {
            continue;
        }
        git_folder(&common, &mut folders);
        refs(&common.join("refs"), &mut folders);
        let trees = common.join("worktrees");
        if let Ok(entries) = fs::read_dir(&trees) {
            folders.insert(trees, Folder::Trees);
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    git_folder(&entry.path(), &mut folders);
                }
            }
        }
    }
    for file in kept {
        let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
            continue;
        };
        let name = name.to_string_lossy().into_owned();
        // A kept file in a git folder would be a strange place, and that
        // folder's watch is kept as git's.
        if let Folder::Kept(names) = folders
            .entry(folder.to_path_buf())
            .or_insert_with(|| Folder::Kept(BTreeSet::new()))
        {
            names.insert(name);
        }
    }
    folders
}

/// A git folder, and its reftable where it keeps its refs in one.
fn git_folder(folder: &Path, folders: &mut BTreeMap<PathBuf, Folder>) {
    folders.insert(folder.to_path_buf(), Folder::Git);
    let reftable = folder.join("reftable");
    if reftable.is_dir() {
        folders.insert(reftable, Folder::Reftable);
    }
}

/// A folder of refs and every folder under it, past no link.
fn refs(folder: &Path, folders: &mut BTreeMap<PathBuf, Folder>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    folders.insert(folder.to_path_buf(), Folder::Refs);
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            refs(&entry.path(), folders);
        }
    }
}

/// What each watch is, shared with the thread that hears them.
type Held = Arc<Mutex<HashMap<WatchDescriptor, Folder>>>;

fn lock(held: &Held) -> MutexGuard<'_, HashMap<WatchDescriptor, Folder>> {
    held.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// What a folder is watched for: a name in it coming, going or written,
/// and the folder itself going.
const HEARD: WatchMask = WatchMask::CREATE
    .union(WatchMask::DELETE)
    .union(WatchMask::MOVED_TO)
    .union(WatchMask::MOVED_FROM)
    .union(WatchMask::CLOSE_WRITE)
    .union(WatchMask::DELETE_SELF)
    .union(WatchMask::MOVE_SELF)
    .union(WatchMask::ONLYDIR);

/// The folders watched, which `follow` moves as the record does.
pub struct Heard {
    watches: Watches,
    held: Held,
    by_path: HashMap<PathBuf, WatchDescriptor>,
}

impl Heard {
    /// Watches nothing yet, and sends on `tell` from a thread of its own
    /// each time a read of inotify's events holds one that's heeded, until
    /// `tell`'s receiver goes, or inotify fails.
    pub fn start(tell: mpsc::Sender<()>) -> io::Result<Heard> {
        let mut inotify = Inotify::init()?;
        let watches = inotify.watches();
        let held: Held = Arc::new(Mutex::new(HashMap::new()));
        let hearing = Arc::clone(&held);
        std::thread::Builder::new()
            .name("heard".into())
            .spawn(move || {
                let mut buffer = [0u8; 16 * 1024];
                loop {
                    let events = match inotify.read_events_blocking(&mut buffer) {
                        Ok(events) => events,
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => return,
                    };
                    let held = lock(&hearing);
                    // A watch given back is no longer held, so what's
                    // still on its way from it, its own end included, is
                    // passed over; a folder going is always heeded.
                    let moved = events.into_iter().any(|event| {
                        held.get(&event.wd).is_some_and(|folder| {
                            event
                                .name
                                .is_none_or(|name| folder.heeds(&name.to_string_lossy()))
                        })
                    });
                    drop(held);
                    if moved && tell.send(()).is_err() {
                        return;
                    }
                }
            })?;
        Ok(Heard {
            watches,
            held,
            by_path: HashMap::new(),
        })
    }

    /// Watches `folders` from now on, and none it watched that they don't
    /// name. A folder that can't be watched, as one gone since it was read,
    /// is passed over; the next reading names what's there.
    pub fn follow(&mut self, folders: BTreeMap<PathBuf, Folder>) {
        let gone: Vec<PathBuf> = self
            .by_path
            .keys()
            .filter(|path| !folders.contains_key(*path))
            .cloned()
            .collect();
        for path in gone {
            if let Some(watch) = self.by_path.remove(&path) {
                lock(&self.held).remove(&watch);
                let _ = self.watches.remove(watch);
            }
        }
        for (path, folder) in folders {
            if let Ok(watch) = self.watches.add(&path, HEARD) {
                lock(&self.held).insert(watch.clone(), folder);
                self.by_path.insert(path, watch);
            }
        }
    }

    /// How many folders are watched, which health says (serve.rs).
    pub fn watched(&self) -> usize {
        self.by_path.len()
    }
}

#[cfg(test)]
mod tests;
