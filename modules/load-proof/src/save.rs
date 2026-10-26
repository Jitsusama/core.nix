//! Saves: whether an app's save reaches the disk as quickly as on an
//! idle machine.
//!
//! An editor that keeps what it saves safe writes the new text beside the
//! old, flushes it to the disk, renames it over the old and flushes the
//! folder, so a crash leaves one or the other whole. Each flush waits for
//! the disk, and for the filesystem's log, behind whatever the builds have
//! written, and the editor waits on the flush. The probe saves that way,
//! on the filesystem Joel's files are on, a file of the size of a long
//! source file, a few times a second.
//!
//! The text doesn't compress, so the filesystem's compression can't make a
//! save cheaper than a real one; source text compresses about threefold,
//! so a real save of this size writes less.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often the file is saved.
pub const EVERY: Duration = Duration::from_millis(250);

/// How much each save writes.
pub const SIZE: usize = 64 * 1024;

/// The file saved, in the folder given.
const SAVED: &str = "saved";

/// Where it's written before it's renamed over the last.
const WRITING: &str = "saved.new";

/// The folder saves go to: load-proof's own under the user's cache, on
/// the filesystem their files are on rather than a tmpfs.
pub fn folder(cache: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    cache
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|home| home.join(".cache")))
        .map(|cache| cache.join("load-proof"))
}

/// Saves a file in `folder` every `EVERY` for `length`, giving how long
/// each save took, and takes the file away after.
///
/// # Errors
///
/// When the folder can't be made or a save fails.
pub fn watch(folder: &Path, length: Duration) -> Result<Vec<Duration>, String> {
    fs::create_dir_all(folder).map_err(|e| format!("making {}: {e}", folder.display()))?;
    let saves =
        Saves::in_folder(folder).map_err(|e| format!("opening {}: {e}", folder.display()))?;
    let mut text = Text::new();
    let start = Instant::now();
    let mut next = start;
    let mut times = Vec::new();
    while start.elapsed() < length {
        times.push(
            saves
                .save(text.next())
                .map_err(|e| format!("saving: {e}"))?,
        );
        next += EVERY;
        if let Some(wait) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    Ok(times)
}

/// A folder saved into, its file taken away once the probe is done.
struct Saves {
    folder: PathBuf,
    handle: File,
}

impl Saves {
    fn in_folder(folder: &Path) -> io::Result<Saves> {
        Ok(Saves {
            folder: folder.to_owned(),
            handle: File::open(folder)?,
        })
    }

    /// Saves `text` as a careful editor does, giving how long it took.
    fn save(&self, text: &[u8]) -> io::Result<Duration> {
        let start = Instant::now();
        let writing = self.folder.join(WRITING);
        let mut file = File::create(&writing)?;
        file.write_all(text)?;
        file.sync_all()?;
        fs::rename(&writing, self.folder.join(SAVED))?;
        self.handle.sync_all()?;
        Ok(start.elapsed())
    }
}

impl Drop for Saves {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.folder.join(SAVED));
        let _ = fs::remove_file(self.folder.join(WRITING));
    }
}

/// Text that doesn't compress, changed at each save as an edit would.
struct Text {
    bytes: Vec<u8>,
    edits: u64,
}

impl Text {
    fn new() -> Text {
        // xorshift64: random enough that zstd finds nothing to shrink.
        let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
        let bytes = (0..SIZE)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state.to_le_bytes()[0]
            })
            .collect();
        Text { bytes, edits: 0 }
    }

    fn next(&mut self) -> &[u8] {
        self.edits += 1;
        self.bytes[..8].copy_from_slice(&self.edits.to_le_bytes());
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of its own under the system's temporary folder.
    fn scratch(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("load-proof-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn saves_go_under_the_cache_and_fall_back_to_the_home_folder() {
        let at = |cache: Option<&str>, home: Option<&str>| {
            folder(cache.map(PathBuf::from), home.map(PathBuf::from))
        };
        assert_eq!(
            at(Some("/c"), Some("/h")),
            Some(PathBuf::from("/c/load-proof"))
        );
        assert_eq!(
            at(None, Some("/h")),
            Some(PathBuf::from("/h/.cache/load-proof"))
        );
        // The base directory spec ignores a relative path.
        assert_eq!(
            at(Some("c"), Some("/h")),
            Some(PathBuf::from("/h/.cache/load-proof"))
        );
        assert_eq!(at(None, None), None);
    }

    #[test]
    fn a_save_leaves_the_newest_text_whole_and_nothing_half_written() {
        let folder = scratch("whole");
        fs::create_dir_all(&folder).unwrap();
        let saves = Saves::in_folder(&folder).unwrap();
        let mut text = Text::new();
        saves.save(text.next()).unwrap();
        let newest = text.next().to_vec();
        saves.save(&newest).unwrap();
        assert_eq!(fs::read(folder.join(SAVED)).unwrap(), newest);
        assert!(!folder.join(WRITING).exists());
        drop(saves);
        assert!(!folder.join(SAVED).exists(), "the probe tidies up");
        fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn each_edit_changes_the_text_and_nothing_else_does() {
        let mut text = Text::new();
        let first = text.next().to_vec();
        let second = text.next().to_vec();
        assert_eq!(first.len(), SIZE);
        assert_ne!(first[..8], second[..8]);
        assert_eq!(first[8..], second[8..]);
        // Not one byte repeated through the text, as plain text would be.
        let distinct: std::collections::BTreeSet<u8> = first.iter().copied().collect();
        assert!(distinct.len() > 200, "{}", distinct.len());
    }

    #[test]
    fn a_watch_saves_at_its_pace_and_times_each() {
        let folder = scratch("watch");
        let times = watch(&folder, EVERY * 3).unwrap();
        assert!((3..=4).contains(&times.len()), "{times:?}");
        assert!(
            times.iter().all(|time| *time < Duration::from_secs(5)),
            "{times:?}"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}
