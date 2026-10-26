use disk_guard::band::Band;
use disk_guard::words::{Room, cleared, filesystem, machine};
use std::path::Path;

const GIB: u64 = 1 << 30;

fn room(path: &str, band: Band, free: u64, full_in: Option<f64>) -> Room<'_> {
    Room {
        path: Path::new(path),
        band,
        free,
        size: 1000 * GIB,
        full_in,
    }
}

#[test]
fn a_healthy_disk_says_how_much_is_free_and_nothing_more() {
    assert_eq!(
        filesystem(&room("/", Band::Healthy, 412 * GIB, None), None),
        "The disk has 412.0 GiB free."
    );
}

#[test]
fn a_disk_filling_fast_says_when_it_fills() {
    assert_eq!(
        filesystem(
            &room("/", Band::Healthy, 300 * GIB, Some(5.0 * 3600.0)),
            None
        ),
        "The disk has 300.0 GiB free. At this rate it fills in 5.0 h."
    );
    assert_eq!(
        filesystem(
            &room("/", Band::Healthy, 300 * GIB, Some(30.0 * 86400.0)),
            None
        ),
        "The disk has 300.0 GiB free.",
        "a month away is nothing to tell"
    );
}

#[test]
fn each_band_below_healthy_says_what_the_guard_is_doing() {
    let said = |band, free| filesystem(&room("/", band, free, None), None);
    assert_eq!(
        said(Band::Pressure, 180 * GIB),
        "The disk is filling, with 180.0 GiB free. \
         The guard is clearing builds and caches nobody has used lately."
    );
    assert_eq!(
        said(Band::Urgent, 60 * GIB),
        "The disk is nearly full, with 60.0 GiB free. \
         The guard is clearing everything it safely can."
    );
    assert_eq!(
        said(Band::Critical, 20 * GIB),
        "The disk is critically full, with 20.0 GiB free. Saves may fail; \
         the guard is clearing everything it can and may freeze what is filling it."
    );
}

#[test]
fn a_disk_is_named_as_people_know_it() {
    let home = Path::new("/home/u");
    let said = |path| filesystem(&room(path, Band::Healthy, GIB, None), Some(home));
    assert_eq!(said("/"), "The disk has 1.0 GiB free.");
    assert_eq!(said("/nix/store"), "The Nix store has 1.0 GiB free.");
    assert_eq!(said("/home/u"), "Your home folder has 1.0 GiB free.");
    assert_eq!(said("/home/u/vms"), "~/vms has 1.0 GiB free.");
    assert_eq!(said("/data"), "/data has 1.0 GiB free.");
}

#[test]
fn the_machine_speaks_for_its_worst_disk() {
    let disks = [
        room("/", Band::Healthy, 400 * GIB, None),
        room("/scratch", Band::Urgent, 60 * GIB, None),
    ];
    assert_eq!(machine(&disks, None), filesystem(&disks[1], None));
}

#[test]
fn a_machine_with_room_everywhere_says_so_by_its_tightest_disk() {
    let disks = [
        room("/", Band::Healthy, 400 * GIB, None),
        room("/scratch", Band::Healthy, 300 * GIB, None),
    ];
    assert_eq!(
        machine(&disks, None),
        "Plenty of room: 300.0 GiB free on /scratch."
    );
    assert_eq!(
        machine(&disks[..1], None),
        "Plenty of room: 400.0 GiB free on the disk."
    );
}

#[test]
fn a_machine_filling_fast_says_so_while_still_healthy() {
    let disks = [room("/", Band::Healthy, 300 * GIB, Some(3.0 * 3600.0))];
    assert_eq!(machine(&disks, None), filesystem(&disks[0], None));
}

#[test]
fn a_machine_watching_nothing_says_so() {
    assert_eq!(machine(&[], None), "Not watching any disk yet.");
}

#[test]
fn what_was_cleared_is_said_by_what_it_was() {
    let home = Some(Path::new("/home/u"));
    assert_eq!(
        cleared(
            "target",
            Path::new("/home/u/src/app/target"),
            42 * GIB,
            home
        ),
        "Cleared a Rust build, ~/src/app/target (42.0 GiB)."
    );
    assert_eq!(
        cleared("cache", Path::new("/home/u/.cache/go-build"), GIB, home),
        "Cleared a cache, ~/.cache/go-build (1.0 GiB)."
    );
    assert_eq!(
        cleared("garbage", Path::new("/nix/store/abc-x"), GIB, home),
        "Cleared unused Nix store paths, /nix/store/abc-x (1.0 GiB)."
    );
    assert_eq!(
        cleared("something-new", Path::new("/x"), 2048, home),
        "Cleared /x (2.0 KiB).",
        "a kind it doesn't know is said by its path alone"
    );
}
