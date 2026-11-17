//! Writing one of the net's small files whole while other `ready`s write
//! it too. Several sessions ask `ready` at once, and each keeps what it
//! learned in the same few files (`kit-owed.json`, `waited.json`,
//! `places.json`): it reads one, changes its own key and writes it back.
//!
//! At 15:15 on 16 November two of them wrote `kit-owed.json` through one
//! shared partial file: the first rename took it, the second failed with
//! "No such file or directory", and the reader lost its wait for the kit,
//! as each read, change and write lost the other's key. So each writer's
//! partial is its own, and a lock beside the file keeps a read, a change
//! and a write together.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// The longest a writer waits for another's lock. A change holds it for a
/// read, a few git asks and a write, well under a second; one stopped or
/// hung past this is gone round, as before the lock.
const LOCK_WAIT: Duration = Duration::from_secs(5);

/// Write `text` to `path` whole, by a rename, so a reader never sees half.
/// The partial is named for this process and this write, so two writers
/// at once never rename each other's.
pub fn write(path: &Path, text: &str) -> Result<(), String> {
    static WRITES: AtomicU64 = AtomicU64::new(0);
    let write = WRITES.fetch_add(1, Ordering::Relaxed);
    let partial = path.with_extension(format!("{}-{write}.partial", std::process::id()));
    let written = fs::write(&partial, text).and_then(|()| fs::rename(&partial, path));
    if written.is_err() {
        let _ = fs::remove_file(&partial);
    }
    written.map_err(|error| format!("writing {}: {error}", path.display()))
}

/// Run `change`, a read of `path`, a change and a write back, holding the
/// lock beside it (`<name>.lock`) so no other writer's change comes
/// between. The lock goes when this ends, or when its process dies, so a
/// writer killed mid-change holds no one up. Where it can't be taken
/// within [`LOCK_WAIT`], `change` runs anyway, saying so, as before the
/// lock: losing a key costs a wait kept again, and a `ready` that stopped
/// for a stopped holder would cost more.
pub fn changed<T>(path: &Path, change: impl FnOnce() -> T) -> T {
    let lock = locked(&path.with_extension("lock"));
    if let Err(why) = &lock {
        eprintln!(
            "merge-net: {} is changed without its lock: {why}",
            path.display()
        );
    }
    let changed = change();
    drop(lock);
    changed
}

/// The lock at `path`, taken within [`LOCK_WAIT`].
fn locked(path: &Path) -> Result<fs::File, String> {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .map_err(|error| format!("opening {}: {error}", path.display()))?;
    let since = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(lock),
            Err(fs::TryLockError::WouldBlock) if since.elapsed() < LOCK_WAIT => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(fs::TryLockError::WouldBlock) => {
                return Err(format!("another writer held it past {LOCK_WAIT:?}"));
            }
            Err(fs::TryLockError::Error(error)) => return Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// A folder of the test's own, emptied first and removed once it ends.
    struct Folder(PathBuf);

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn folder(name: &str) -> Folder {
        let root =
            std::env::temp_dir().join(format!("merge-net-whole-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("made");
        Folder(root)
    }

    #[test]
    fn writers_at_once_each_write_the_file_whole() {
        let folder = folder("at-once");
        let path = folder.0.join("kit-owed.json");
        let failed: Vec<String> = thread::scope(|scope| {
            let writers: Vec<_> = (0..8)
                .map(|writer| {
                    let path = &path;
                    scope.spawn(move || {
                        (0..100)
                            .filter_map(|time| write(path, &format!("{writer} {time}\n")).err())
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            writers
                .into_iter()
                .flat_map(|writer| writer.join().expect("a writer"))
                .collect()
        });
        assert!(
            failed.is_empty(),
            "{} of 800 writes at once failed, as at 15:15 on 16 November: {:?}",
            failed.len(),
            failed.first()
        );
        let left = fs::read_dir(path.parent().expect("its folder"))
            .expect("the folder")
            .count();
        assert_eq!(
            left, 1,
            "writes at once left {left} files where one is kept"
        );
        let kept = fs::read_to_string(&path).expect("the file");
        let whole = kept.strip_suffix('\n').is_some_and(|line| {
            line.split(' ').count() == 2 && line.split(' ').all(|n| n.parse::<u32>().is_ok())
        });
        assert!(
            whole,
            "the file kept wasn't one writer's words whole: {kept:?}"
        );
    }

    #[test]
    fn a_write_that_fails_leaves_no_partial_behind() {
        let folder = folder("fails");
        let path = folder.0.join("waited.json");
        // A folder with something in it where the file goes: the partial
        // is written, and its rename onto the folder fails.
        fs::create_dir_all(path.join("in-the-way")).expect("made");
        assert!(
            write(&path, "{}\n").is_err(),
            "a rename onto a folder was taken as written"
        );
        let left: Vec<_> = fs::read_dir(&folder.0)
            .expect("the folder")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .collect();
        assert_eq!(
            left.len(),
            1,
            "a failed write left its partial behind: {left:?}"
        );
    }

    #[test]
    fn a_lock_held_past_its_wait_is_gone_round() {
        let folder = folder("held");
        let path = folder.0.join("places.json");
        let held = locked(&path.with_extension("lock")).expect("the lock");
        let since = Instant::now();
        let ran = changed(&path, || true);
        drop(held);
        assert!(ran, "a change under a lock held elsewhere never ran");
        let waited = since.elapsed();
        assert!(
            waited >= LOCK_WAIT && waited < LOCK_WAIT * 3,
            "a change waited {waited:?} for a held lock, not about {LOCK_WAIT:?}"
        );
    }

    #[test]
    fn a_key_each_writer_adds_at_once_is_kept() {
        let folder = folder("keys");
        let path = folder.0.join("waited.json");
        thread::scope(|scope| {
            for writer in 0..8 {
                let path = &path;
                scope.spawn(move || {
                    changed(path, || {
                        let mut said: BTreeMap<String, u64> = fs::read_to_string(path)
                            .ok()
                            .and_then(|text| serde_json::from_str(&text).ok())
                            .unwrap_or_default();
                        // A `ready` reads, then works out its answer, then
                        // writes: the pause is that work.
                        thread::sleep(Duration::from_millis(20));
                        said.insert(format!("writer {writer}"), writer);
                        let text = serde_json::to_string(&said).expect("its words");
                        write(path, &text)
                    })
                    .expect("written");
                });
            }
        });
        let said: BTreeMap<String, u64> =
            serde_json::from_str(&fs::read_to_string(&path).expect("the file")).expect("json");
        assert_eq!(
            said.len(),
            8,
            "writers at once lost each other's keys, keeping only {:?}",
            said.keys().collect::<Vec<_>>()
        );
    }
}
