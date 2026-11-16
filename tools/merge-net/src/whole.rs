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
/// writer killed mid-change holds no one up. Where it can't be taken,
/// `change` runs anyway, as before the lock: losing a key costs a wait
/// kept again, and a `ready` that stopped for it would cost more.
pub fn changed<T>(path: &Path, change: impl FnOnce() -> T) -> T {
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path.with_extension("lock"))
        .ok()
        .filter(|lock| lock.lock().is_ok());
    let changed = change();
    drop(lock);
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::thread;
    use std::time::Duration;

    /// A folder of the test's own, emptied first.
    fn folder(name: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("merge-net-whole-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("made");
        root
    }

    #[test]
    fn writers_at_once_each_write_the_file_whole() {
        let path = folder("at-once").join("kit-owed.json");
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
    }

    #[test]
    fn a_key_each_writer_adds_at_once_is_kept() {
        let path = folder("keys").join("waited.json");
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
