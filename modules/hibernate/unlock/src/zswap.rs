//! zswap switched off while a hibernation image is made, and on again once
//! the machine is.
//!
//! zswap keeps what's swapped out in a pool in memory, up to a fifth of it,
//! before the swap file, and pages that don't compress go in as they are.
//! The pool goes out to the disk only as zswap gets round to writing it,
//! so what the kernel swaps out to make room for the image can stay in
//! memory and go into the image anyway: a test machine with 300 MiB of
//! random bytes rolled back 28,090 pages short in one run of three.
//! Switched off, zswap stores nothing more, and what the kernel swaps out for
//! the image goes to the disk. What the pool already holds stays in memory
//! until it's read back or written out.
//!
//! Switching zswap off doesn't stop the pool being written out, in two
//! ways. Under reclaim, which the image's own allocation may start, its
//! shrinker writes back when `zswap.shrinker_enabled` is on; modules/memory
//! leaves it off, as the kernel's default does, but `memory-zswap-shrinker@y`
//! turns it on for as long as the machine runs. And a trim of a full pool
//! queued just before zswap went off may still run after. Both reach even the
//! slices that set `MemoryZSwapWriteback = false` (modules/slices says why
//! they do): 7.3's mm/zswap.c asks mm/memcontrol.c whether a group may write
//! back, in the shrinker's count and in `shrink_memcg`, which the trim calls,
//! and with zswap off it answers yes for every group. What either writes out
//! of theirs comes back from the disk after a resume. Otherwise the pool
//! leaves memory only as it's read back or freed: with zswap off nothing is
//! stored, so nothing queues another trim.
//!
//! `fits` counts the whole pool as in use, at what it takes compressed.
//! Writing an entry out doesn't make the image smaller by itself:
//! `zswap_writeback_entry` first takes a whole page in the swap cache and
//! decompresses the entry into it, then frees the entry and starts the
//! page's write, so for a while a compressed entry's page holds more than the
//! entry gave back; one stored as it is gives back the page it took. Once the
//! write is done, the page is clean and already holds its swap slot, so the
//! image's reclaim can drop it without more room in swap. What's written out
//! before that reclaim runs can only make the image smaller than `fits`
//! reckons. A page written after it stays in the image whole, where `fits`
//! counted its entry compressed, so such writing could make the image larger
//! than `fits` reckons, by at most what the pool holds uncompressed less what
//! it takes. With the shrinker off, only a trim queued before zswap went off
//! can do that. That is read from the source; no hibernation has been
//! watched with either writing back.

use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

/// Whether zswap stores what's swapped out, `Y` or `N`.
pub const ENABLED: &str = "/sys/module/zswap/parameters/enabled";

/// Whether a reading of zswap's `enabled` says it stores.
pub fn on(enabled: &str) -> bool {
    enabled.trim() == "Y"
}

/// Switches zswap off when it's on, keeping that it was in `kept`, so it's
/// switched on again even if the machine never comes back to this run.
/// Says whether it switched it off.
pub fn set_aside(enabled: &Path, kept: &Path) -> io::Result<bool> {
    match fs::read_to_string(enabled) {
        Ok(state) if on(&state) => {}
        Ok(_) => return Ok(false),
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    }
    fs::write(kept, "Y\n")?;
    fs::write(enabled, "N")?;
    Ok(true)
}

/// Switches zswap on again when `set_aside` switched it off, then forgets
/// it. Says whether it switched it on.
pub fn put_back(enabled: &Path, kept: &Path) -> io::Result<bool> {
    match fs::read_to_string(kept) {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    }
    fs::write(enabled, "Y")?;
    fs::remove_file(kept)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A folder of its own for a test, holding a stand-in for `enabled`.
    fn folder(name: &str, state: Option<&str>) -> (PathBuf, PathBuf, PathBuf) {
        let folder = std::env::temp_dir().join(format!("zswap-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let enabled = folder.join("enabled");
        if let Some(state) = state {
            fs::write(&enabled, state).unwrap();
        }
        let kept = folder.join("kept");
        (folder, enabled, kept)
    }

    #[test]
    fn zswap_on_is_switched_off_and_on_again() {
        let (folder, enabled, kept) = folder("on", Some("Y\n"));
        assert!(set_aside(&enabled, &kept).unwrap());
        assert_eq!(fs::read_to_string(&enabled).unwrap(), "N");
        assert!(put_back(&enabled, &kept).unwrap());
        assert_eq!(fs::read_to_string(&enabled).unwrap(), "Y");
        assert!(!kept.exists());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn zswap_off_is_left_off() {
        let (folder, enabled, kept) = folder("off", Some("N\n"));
        assert!(!set_aside(&enabled, &kept).unwrap());
        assert!(!put_back(&enabled, &kept).unwrap());
        assert_eq!(fs::read_to_string(&enabled).unwrap(), "N\n");
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn with_no_zswap_nothing_is_done() {
        let (folder, enabled, kept) = folder("none", None);
        assert!(!set_aside(&enabled, &kept).unwrap());
        assert!(!put_back(&enabled, &kept).unwrap());
        assert!(!enabled.exists());
        fs::remove_dir_all(folder).unwrap();
    }
}
