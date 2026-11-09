use disk_guard::band::{Band, BtrfsSpace, Space, Thresholds};
use disk_guard::config::Bands;
use disk_guard::units::Size;

const TOTAL: u64 = 1000;

/// Bands of their own, so what these show of bands holds whatever the
/// defaults are.
fn bands() -> Bands {
    Bands {
        filling: Size::Percent(15.0),
        nearly_full: Size::Percent(8.0),
        critically_full: Size::Percent(3.0),
        target: Size::Percent(20.0),
    }
}

fn thresholds() -> Thresholds {
    Thresholds::new(&bands(), TOTAL)
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
        (t.filling, t.nearly_full, t.critically_full, t.target),
        (150, 80, 30, 200)
    );
}

#[test]
fn bytes_resolve_as_they_are() {
    let bands = Bands {
        nearly_full: Size::Bytes(100),
        ..bands()
    };
    assert_eq!(Thresholds::new(&bands, TOTAL).nearly_full, 100);
}

#[test]
fn bands_out_of_order_fold_and_the_target_lies_past_filling() {
    // Bytes against shares can only be ordered on a filesystem: a nearly
    // full line of 500 bytes over a filling line of 15%, and a target
    // under it.
    let bands = Bands {
        nearly_full: Size::Bytes(500),
        critically_full: Size::Bytes(600),
        target: Size::Bytes(10),
        ..bands()
    };
    let t = Thresholds::new(&bands, TOTAL);
    assert_eq!(
        (t.filling, t.nearly_full, t.critically_full),
        (150, 150, 150)
    );
    assert_eq!(
        t.target,
        150 + t.margin,
        "a pass that reaches it leaves filling"
    );
    assert_eq!(t.band(&space(t.target), Band::Filling), Band::Healthy);
}

#[test]
fn free_space_names_the_band() {
    let t = thresholds();
    assert_eq!(t.band(&space(500), Band::Healthy), Band::Healthy);
    assert_eq!(t.band(&space(150), Band::Healthy), Band::Healthy);
    assert_eq!(t.band(&space(149), Band::Healthy), Band::Filling);
    assert_eq!(t.band(&space(79), Band::Healthy), Band::NearlyFull);
    assert_eq!(t.band(&space(29), Band::Healthy), Band::CriticallyFull);
}

#[test]
fn a_band_is_left_only_past_its_threshold_and_a_margin() {
    let t = thresholds();
    assert_eq!(
        t.band(&space(35), Band::CriticallyFull),
        Band::CriticallyFull
    );
    assert_eq!(t.band(&space(40), Band::CriticallyFull), Band::NearlyFull);
    assert_eq!(t.band(&space(155), Band::Filling), Band::Filling);
    assert_eq!(t.band(&space(160), Band::Filling), Band::Healthy);
    assert_eq!(t.band(&space(85), Band::CriticallyFull), Band::NearlyFull);
    assert_eq!(t.band(&space(95), Band::CriticallyFull), Band::Filling);
}

#[test]
fn a_band_is_written_by_its_word_and_still_read_by_the_one_it_once_had() {
    // Journals and status files an older guard wrote carry the old words.
    for (band, word, old) in [
        (Band::Filling, "filling", "pressure"),
        (Band::NearlyFull, "nearly-full", "urgent"),
        (Band::CriticallyFull, "critically-full", "critical"),
    ] {
        assert_eq!(serde_json::to_value(band).unwrap(), word);
        assert_eq!(serde_json::from_value::<Band>(word.into()).unwrap(), band);
        assert_eq!(serde_json::from_value::<Band>(old.into()).unwrap(), band);
    }
}

#[test]
fn falling_deeper_never_waits() {
    let t = thresholds();
    assert_eq!(t.band(&space(29), Band::Filling), Band::CriticallyFull);
}

#[test]
fn btrfs_short_of_metadata_room_is_critically_full_whatever_data_says() {
    let t = thresholds();
    let mut tight = space(500);
    tight.btrfs = Some(BtrfsSpace {
        unallocated: 512,
        metadata_chunk: 1024,
        metadata_free: 100,
        global_reserve: 60,
    });
    assert_eq!(t.band(&tight, Band::Healthy), Band::CriticallyFull);
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
