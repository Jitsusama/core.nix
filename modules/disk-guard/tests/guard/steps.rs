//! A guard's steps: what each saves of its memory, and when.

use crate::daemon::{CALM, Home, wall};
use disk_guard::clock::Tick;
use disk_guard::daemon::Senses;
use disk_guard::moments::Power;
use disk_guard::units::Size;
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

#[test]
fn a_disk_turned_healthy_forgets_its_rest_at_once_though_nothing_passed() {
    // A guard saves its memory after a pass, or every five minutes, and a
    // rest ending on a disk turned healthy is neither: a guard stopped
    // before its next save would start resting an hour more.
    let _held = writes_a_share();
    let home = on_shm();
    std::fs::create_dir_all(home.root.join("scratch")).unwrap();
    let (margin, free) = margin_and_free(&home.root);
    let filling = free - margin * 3 / 2;
    let mut config = home.config("0%", "0%");
    config.bands.filling = Size::Bytes(filling);
    config.bands.target = Size::Bytes(filling);
    let filler = home.root.join("filler");
    fill(&filler, margin * 2);
    let start = wall();
    let at = |seconds: f64| Tick {
        wall: start + seconds,
        monotonic: 3600.0 + seconds,
        ..crate::daemon::tick()
    };
    let mut guard = home.guard(config);
    guard.step(&at(0.0), &CALM).unwrap();
    guard.step(&at(70.0), &CALM).unwrap();
    let emptied = |home: &Home| {
        home.memory()["emptied"]
            .as_object()
            .map_or(0, serde_json::Map::len)
    };
    assert_eq!(
        emptied(&home),
        1,
        "two reaches in a row that found nothing didn't keep the disk's rest: {}",
        home.memory()
    );
    std::fs::remove_file(&filler).unwrap();
    guard.step(&at(100.0), &DRAINED).unwrap();
    assert!(
        !home
            .journal()
            .iter()
            .any(|line| line["event"] == "pass" && line["at"].as_f64() >= Some(start + 100.0)),
        "a healthy disk passed on a drained machine, which saves by itself"
    );
    assert_eq!(
        emptied(&home),
        0,
        "the rest of a disk turned healthy was still kept: {}",
        home.memory()
    );
}
