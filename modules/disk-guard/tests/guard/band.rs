use disk_guard::band::{Band, BtrfsSpace, Space, Thresholds};
use disk_guard::config::Bands;
use disk_guard::units::Size;

const TOTAL: u64 = 1000;

fn thresholds() -> Thresholds {
    Thresholds::new(&Bands::default(), TOTAL)
}

fn space(free: u64) -> Space {
    Space {
        total: TOTAL,
        free,
        btrfs: None,
    }
}

#[test]
fn shares_resolve_against_the_filesystem() {
    let t = thresholds();
    assert_eq!(
        (t.pressure, t.urgent, t.critical, t.target),
        (150, 80, 30, 200)
    );
}

#[test]
fn bytes_resolve_as_they_are() {
    let bands = Bands {
        urgent: Size::Bytes(100),
        ..Bands::default()
    };
    assert_eq!(Thresholds::new(&bands, TOTAL).urgent, 100);
}

#[test]
fn free_space_names_the_band() {
    let t = thresholds();
    assert_eq!(t.band(&space(500), Band::Healthy), Band::Healthy);
    assert_eq!(t.band(&space(150), Band::Healthy), Band::Healthy);
    assert_eq!(t.band(&space(149), Band::Healthy), Band::Pressure);
    assert_eq!(t.band(&space(79), Band::Healthy), Band::Urgent);
    assert_eq!(t.band(&space(29), Band::Healthy), Band::Critical);
}

#[test]
fn a_band_is_left_only_past_its_threshold_and_a_margin() {
    let t = thresholds();
    assert_eq!(t.band(&space(35), Band::Critical), Band::Critical);
    assert_eq!(t.band(&space(40), Band::Critical), Band::Urgent);
    assert_eq!(t.band(&space(155), Band::Pressure), Band::Pressure);
    assert_eq!(t.band(&space(160), Band::Pressure), Band::Healthy);
    assert_eq!(t.band(&space(85), Band::Critical), Band::Urgent);
    assert_eq!(t.band(&space(95), Band::Critical), Band::Pressure);
}

#[test]
fn falling_deeper_never_waits() {
    let t = thresholds();
    assert_eq!(t.band(&space(29), Band::Pressure), Band::Critical);
}

#[test]
fn btrfs_short_of_metadata_room_is_critical_whatever_data_says() {
    let t = thresholds();
    let mut tight = space(500);
    tight.btrfs = Some(BtrfsSpace {
        unallocated: 512,
        metadata_chunk: 1024,
        metadata_free: 100,
        global_reserve: 60,
    });
    assert_eq!(t.band(&tight, Band::Healthy), Band::Critical);
    let roomy_metadata = BtrfsSpace {
        metadata_free: 200,
        ..tight.btrfs.unwrap()
    };
    tight.btrfs = Some(roomy_metadata);
    assert_eq!(t.band(&tight, Band::Healthy), Band::Healthy);
    let unallocated = BtrfsSpace {
        metadata_free: 100,
        unallocated: 2048,
        ..roomy_metadata
    };
    tight.btrfs = Some(unallocated);
    assert_eq!(t.band(&tight, Band::Healthy), Band::Healthy);
}
