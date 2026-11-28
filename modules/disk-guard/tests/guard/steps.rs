//! A guard's steps: what each saves of its memory, and when.

use crate::daemon::{CALM, Home};
use disk_guard::daemon::Senses;
use disk_guard::moments::Power;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, PoisonError};

/// Held by each scenario that moves a disk between bands by writing to it,
/// since another's writes of the same size would move its free space too.
static WRITES_A_SHARE: Mutex<()> = Mutex::new(());

/// Holds the disk for a scenario that moves it between bands by writing.
pub(crate) fn writes_a_share() -> std::sync::MutexGuard<'static, ()> {
    WRITES_A_SHARE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// A place for a scenario whose free space it moves by writing: a tmpfs,
/// whose share of a hundredth of the disk is a few hundred megabytes in
/// memory, where the temp directory may be a disk of terabytes.
pub(crate) fn on_shm() -> Home {
    Home::within(tempfile::tempdir_in("/dev/shm").unwrap())
}

/// A hundredth of the disk `path` is on, by which a band is left past its
/// threshold, and the bytes free there now.
pub(crate) fn margin_and_free(path: &Path) -> (u64, u64) {
    let space = disk_guard::sys::space(path).unwrap();
    (space.total / 100, space.free)
}

/// Writes `bytes` of real data to `path`: a sparse file takes no room.
pub(crate) fn fill(path: &Path, bytes: u64) {
    let chunk = vec![1u8; 1 << 20];
    let mut file = std::fs::File::create(path).unwrap();
    let mut left = bytes;
    while left > 0 {
        let size = left.min(chunk.len() as u64);
        #[expect(clippy::cast_possible_truncation, reason = "at most a chunk's length")]
        file.write_all(&chunk[..size as usize]).unwrap();
        left -= size;
    }
    file.sync_all().unwrap();
}

/// A machine too drained to spare upkeep, so a healthy disk's step passes
/// nothing.
pub(crate) const DRAINED: Senses = Senses {
    power: Power {
        on_charger: false,
        battery: Some(5),
    },
    ..CALM
};
