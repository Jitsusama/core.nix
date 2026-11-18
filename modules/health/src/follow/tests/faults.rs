//! The faults health sees itself, rather than hears: the lid reading
//! wrong and a disk guard gone quiet, each counted once until it clears,
//! through a start and across midnight.

use super::*;

#[test]
fn a_lid_gone_wrong_is_a_devices_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    for keys in 1..5 {
        follower.look_at_lid(used(keys), NOON);
    }
    let open = Looked {
        shut: false,
        ..used(5)
    };
    follower.look_at_lid(open, NOON);
    follower.look_at_lid(used(6), NOON);
    follower.look_at_lid(used(7), NOON);
    follower.write().expect("the day is written");
    let lid = &read(into.path(), "2026-11-12").things["lid"];
    assert_eq!(lid.kind, Some(Kind::Devices), "a lid is a device");
    assert_eq!(lid.errors, 2, "once a time it went wrong: {lid:?}");
    let said: Vec<&str> = lid
        .signatures
        .values()
        .map(|said| said.example.as_str())
        .collect();
    assert_eq!(said, [LID_WRONG], "said the way Joel reads it");
}

#[test]
fn a_restart_keeps_the_faults_health_saw_and_counts_one_still_seen_no_more() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    let beat = |seconds: i64| {
        fs::write(
            &status,
            format!(r#"{{"heartbeat": {seconds}, "filesystems": []}}"#),
        )
        .expect("a status written");
    };
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    let noon = NOON / 1_000_000;
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    for keys in 1..5 {
        follower.look_at_lid(used(keys), NOON);
    }
    beat(noon);
    follower.guard(NOON + 200_000_000);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        (day.things["lid"].errors, day.things["disk-guard"].errors),
        (1, 1),
        "health saw the lid wrong and the guard stopped, once each"
    );

    // A restart keeps both before health looks again.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(day);
    let errors = |day: &Day, name: &str| day.things.get(name).map(|thing| thing.errors);
    assert_eq!(
        (errors(&again.day, "lid"), errors(&again.day, "disk-guard")),
        (Some(1), Some(1)),
        "a restart lost what health saw itself"
    );
    assert_eq!(
        again.day.things["lid"].kind,
        Some(Kind::Devices),
        "a kept fault is still a device's"
    );

    // With the lid still wrong and the guard still quiet, neither counts again.
    for keys in 5..9 {
        again.look_at_lid(used(keys), NOON + 300_000_000);
    }
    again.guard(NOON + 300_000_000);
    assert_eq!(
        (errors(&again.day, "lid"), errors(&again.day, "disk-guard")),
        (Some(1), Some(1)),
        "a restart counted again what health still saw"
    );

    // The lid reads open, which alone is written; started again, it
    // counts again once it goes wrong again.
    again.look_at_lid(
        Looked {
            shut: false,
            ..used(9)
        },
        NOON + 330_000_000,
    );
    again.write().expect("the lid's clearing is written");
    let mut later = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(read(into.path(), "2026-11-12"));
    for keys in 10..12 {
        later.look_at_lid(used(keys), NOON + 360_000_000);
    }
    assert_eq!(
        errors(&later.day, "lid"),
        Some(2),
        "a lid that cleared before a start wasn't counted once wrong again"
    );
    later.write().expect("the lid's fault is written");

    // The guard beats, which alone is written; started again, it counts
    // again once it stops again.
    beat(noon + 400);
    later.guard(NOON + 400_000_000);
    later.write().expect("the guard's clearing is written");
    let mut last = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(read(into.path(), "2026-11-12"));
    last.guard(NOON + 600_000_000);
    assert_eq!(
        (errors(&last.day, "lid"), errors(&last.day, "disk-guard")),
        (Some(2), Some(2)),
        "a guard that beat before a start wasn't counted once stopped again, or a kept fault \
         was lost"
    );
}

#[test]
fn a_fault_still_seen_at_midnight_counts_once_in_each_day() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    fs::write(
        &status,
        format!(
            r#"{{"heartbeat": {}, "filesystems": []}}"#,
            NOON / 1_000_000
        ),
    )
    .expect("a status written");
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    for keys in 1..3 {
        follower.look_at_lid(used(keys), NOON);
    }
    follower.guard(NOON + 200_000_000);
    follower
        .take(&warning(NOON + DAY))
        .expect("the day rolls over");
    for keys in 3..5 {
        follower.look_at_lid(used(keys), NOON + DAY);
    }
    follower.guard(NOON + DAY);
    let errors = |day: &Day, name: &str| day.things.get(name).map(|thing| thing.errors);
    assert_eq!(
        (
            errors(&follower.day, "lid"),
            errors(&follower.day, "disk-guard")
        ),
        (Some(1), Some(1)),
        "a new day didn't count what health still saw wrong"
    );
    assert_eq!(
        (
            errors(&read(into.path(), "2026-11-12"), "lid"),
            errors(&read(into.path(), "2026-11-12"), "disk-guard")
        ),
        (Some(1), Some(1)),
        "the day before lost what health saw"
    );
}

#[test]
fn a_disk_guard_that_stopped_is_a_services_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    let said = |seconds: i64, free: u64| {
        fs::write(
            &status,
            format!(
                r#"{{"heartbeat": {seconds}, "filesystems": [
                        {{"path": "/", "band": "filling", "free": {free}, "size": 100}}]}}"#
            ),
        )
        .expect("a status written");
    };
    let beat = |seconds: i64| said(seconds, 10);
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    let noon = NOON / 1_000_000;
    beat(noon);
    follower.guard(NOON);
    // Quiet past the two minutes a guard may be, for two looks.
    follower.guard(NOON + 200_000_000);
    follower.guard(NOON + 230_000_000);
    beat(noon + 260);
    follower.guard(NOON + 260_000_000);
    follower.guard(NOON + 500_000_000);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        day.things["disk-guard"].errors, 2,
        "once each time it stopped: {:?}",
        day.things["disk-guard"]
    );
    assert_eq!(
        day.disks.filesystems["/"].band, "filling",
        "the guard's word on the disks wasn't kept"
    );

    // A restart, after which the disk has more free than the day's least.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(day);
    assert_eq!(
        again.day.things["disk-guard"].errors, 2,
        "a restart kept fewer of the guard's stops than the day had"
    );
    said(noon + 600, 60);
    again.guard(NOON + 600_000_000);
    again.write().expect("the day is written again");
    assert_eq!(
        read(into.path(), "2026-11-12").disks.filesystems["/"].least_free,
        10,
        "a restart lost the day's least free space"
    );
}
