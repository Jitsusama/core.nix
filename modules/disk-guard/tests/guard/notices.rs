use disk_guard::band::Band;
use disk_guard::daemon::{FilesystemStatus, LastPass, Status, Writing};
use disk_guard::notices::{Change, Notice, Urgency, change, standing};
use std::path::PathBuf;

const GIB: u64 = 1 << 30;

fn status(bands: &[(&str, Band, u64)], last: Option<LastPass>) -> Status {
    Status {
        version: 2,
        heartbeat: 0.0,
        says: String::new(),
        filesystems: bands
            .iter()
            .map(|&(path, band, free)| FilesystemStatus {
                path: PathBuf::from(path),
                band,
                free: free * GIB,
                total: 1000 * GIB,
                size: 1000 * GIB,
                used: (1000 - free) * GIB,
                forecast: None,
                fill_per_hour: None,
                full_at: None,
                says: String::new(),
                exhausted: false,
            })
            .collect(),
        cleared: Vec::new(),
        last,
        frozen: Vec::new(),
        writer: None,
        notice: None,
    }
}

fn pass(at: f64, band: Band, end: &str, freed: u64) -> LastPass {
    LastPass {
        at,
        path: PathBuf::from("/"),
        band,
        end: end.to_owned(),
        freed: freed * GIB,
    }
}

fn shown(change: Option<Change>) -> Notice {
    match change {
        Some(Change::Show(notice)) => notice,
        other => panic!("expected a notice, got {other:?}"),
    }
}

#[test]
fn a_writer_frozen_is_told_at_once_with_how_to_let_it_go_on() {
    let short = status(
        &[("/", Band::Critical, 2)],
        Some(pass(1.0, Band::Critical, "exhausted", 0)),
    );
    let mut frozen = short.clone();
    frozen.frozen.push(disk_guard::writers::Frozen {
        unit: "disk-guard-freeze-700.scope".to_owned(),
        pid: 700,
        start: 500,
        name: "dd".to_owned(),
        path: PathBuf::from("/"),
        at: 2.0,
    });
    let notice = shown(change(&short, &frozen));
    assert_eq!(notice.summary, "Froze dd, which was filling the disk");
    assert_eq!(
        notice.body,
        "The disk had 2.0 GiB free and nothing else was safe to clear. dd carries on by \
         itself once there's room, or now if you run disk-guard thaw."
    );
    assert_eq!(notice.urgency, Urgency::Critical);
    assert_eq!(change(&frozen, &frozen), None, "told once");
    assert_eq!(
        standing(&frozen),
        Some(notice),
        "and it stands while frozen"
    );
}

#[test]
fn what_stands_now_is_kept_for_whoever_reads_the_status_file() {
    let calm = status(&[("/", Band::Healthy, 400)], None);
    assert_eq!(standing(&calm), None, "a healthy disk has nothing to say");
    let pressed = status(&[("/", Band::Pressure, 140)], None);
    assert_eq!(
        standing(&pressed),
        None,
        "pressure before a pass, nothing yet"
    );
    let spent = status(
        &[("/", Band::Pressure, 140)],
        Some(pass(1.0, Band::Pressure, "exhausted", 0)),
    );
    let notice = standing(&spent).unwrap();
    assert_eq!(
        notice.summary,
        "The disk is filling, with nothing left to clear"
    );
    assert_eq!(
        notice.body,
        "Nothing cleared, and only 140.0 GiB is free and nothing else is safe to clear. \
         Delete something you don't need; disk-guard plan lists what's left."
    );
    assert_eq!(notice.urgency, Urgency::Normal);
    assert_eq!(
        change(&pressed, &spent),
        None,
        "still never a desktop notice"
    );
    let urgent = status(&[("/", Band::Urgent, 60)], None);
    assert_eq!(
        standing(&urgent).unwrap().summary,
        "The disk is nearly full"
    );
    let short = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "exhausted", 0)),
    );
    assert_eq!(
        standing(&short).unwrap().summary,
        "The disk is still nearly full"
    );
    let reached = status(
        &[("/", Band::Pressure, 200)],
        Some(pass(1.0, Band::Pressure, "reached", 40)),
    );
    assert_eq!(standing(&reached), None, "a pass that made room");
    let eased = status(
        &[("/", Band::Healthy, 300)],
        Some(pass(1.0, Band::Pressure, "exhausted", 0)),
    );
    assert_eq!(standing(&eased), None, "once the disk is healthy again");
    let worse = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Pressure, "exhausted", 0)),
    );
    assert_eq!(
        standing(&worse).unwrap().summary,
        "The disk is nearly full",
        "a pass in a milder band says nothing of this one"
    );
}

#[test]
fn nothing_changing_says_nothing() {
    let calm = status(&[("/", Band::Healthy, 400)], None);
    assert_eq!(change(&calm, &calm), None);
    let urgent = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "exhausted", 1)),
    );
    assert_eq!(change(&urgent, &urgent), None);
}

#[test]
fn pressure_is_the_launchers_to_show_and_never_a_notice() {
    let calm = status(&[("/", Band::Healthy, 400)], None);
    let pressed = status(&[("/", Band::Pressure, 140)], None);
    assert_eq!(change(&calm, &pressed), None, "entering it");
    let passed = status(
        &[("/", Band::Pressure, 140)],
        Some(pass(1.0, Band::Pressure, "exhausted", 0)),
    );
    assert_eq!(
        change(&pressed, &passed),
        None,
        "a pass there that took nothing, as most do every half hour"
    );
    let freed = status(
        &[("/", Band::Healthy, 300)],
        Some(pass(2.0, Band::Pressure, "reached", 160)),
    );
    assert_eq!(change(&passed, &freed), None, "nor one that made room");
}

#[test]
fn nearly_full_says_what_the_guard_does_and_that_nothing_is_needed_yet() {
    let pressed = status(&[("/", Band::Pressure, 140)], None);
    let urgent = status(&[("/", Band::Urgent, 60)], None);
    let notice = shown(change(&pressed, &urgent));
    assert_eq!(notice.summary, "The disk is nearly full");
    assert_eq!(
        notice.body,
        "60.0 GiB free. The guard is clearing everything it safely can; nothing is needed \
         from you yet."
    );
    assert_eq!(notice.urgency, Urgency::Normal);
}

#[test]
fn critically_full_says_saves_may_fail_and_is_urgent() {
    let urgent = status(&[("/", Band::Urgent, 60)], None);
    let critical = status(&[("/", Band::Critical, 20)], None);
    let notice = shown(change(&urgent, &critical));
    assert_eq!(notice.summary, "The disk is critically full");
    assert_eq!(
        notice.body,
        "20.0 GiB free, and saves may fail. The guard is clearing what it can, and freezes \
         whatever is filling the disk if that isn't enough."
    );
    assert_eq!(notice.urgency, Urgency::Critical);
    assert_eq!(change(&critical, &urgent), None, "easing says nothing");
}

#[test]
fn the_worst_of_several_disks_is_told_by_its_name() {
    let calm = status(
        &[("/", Band::Healthy, 400), ("/data", Band::Healthy, 400)],
        None,
    );
    let both = status(
        &[("/", Band::Pressure, 140), ("/data", Band::Urgent, 60)],
        None,
    );
    assert_eq!(shown(change(&calm, &both)).summary, "/data is nearly full");
}

#[test]
fn room_made_when_nearly_full_says_how_much_and_that_nothing_is_needed() {
    let before = status(&[("/", Band::Urgent, 60)], None);
    let reached = status(
        &[("/", Band::Healthy, 200)],
        Some(pass(1.0, Band::Urgent, "reached", 140)),
    );
    let notice = shown(change(&before, &reached));
    assert_eq!(notice.summary, "Made room on the disk");
    assert_eq!(
        notice.body,
        "Cleared 140.0 GiB of builds and caches; 200.0 GiB free now. Nothing is needed from you."
    );
    assert_eq!(notice.urgency, Urgency::Normal);
}

#[test]
fn a_pass_left_short_says_what_to_do_and_who_is_filling_the_disk() {
    let before = status(&[("/", Band::Urgent, 60)], None);
    let mut short = status(
        &[("/", Band::Critical, 25)],
        Some(pass(1.0, Band::Critical, "exhausted", 5)),
    );
    let notice = shown(change(&before, &short));
    assert_eq!(notice.summary, "The disk is still critically full");
    assert_eq!(
        notice.body,
        "Cleared 5.0 GiB, but only 25.0 GiB is free and nothing else is safe to clear. \
         Delete something you don't need; disk-guard plan lists what's left."
    );
    assert_eq!(notice.urgency, Urgency::Critical);
    short.writer = Some(Writing {
        name: "cargo".to_owned(),
        pid: 4242,
        rate: 120.0 * f64::from(1 << 20),
    });
    assert!(
        shown(change(&before, &short))
            .body
            .ends_with(" cargo (pid 4242) is writing most, at 120.0 MiB a second."),
    );
    let nearly = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "exhausted", 0)),
    );
    let notice = shown(change(&before, &nearly));
    assert_eq!(notice.summary, "The disk is still nearly full");
    assert!(
        notice
            .body
            .starts_with("Nothing cleared, and only 60.0 GiB is free"),
        "{}",
        notice.body
    );
}

#[test]
fn a_pass_that_stalled_or_failed_says_so_plainly() {
    let before = status(&[("/", Band::Urgent, 60)], None);
    let stalled = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "stalled", 0)),
    );
    assert_eq!(
        shown(change(&before, &stalled)).body,
        "Nothing cleared, and only 60.0 GiB is free. Deleting stopped freeing space, so the \
         guard waits a little before trying again; nothing is needed from you yet."
    );
    let failed = status(
        &[("/", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "failed: reading space: gone", 0)),
    );
    assert_eq!(
        shown(change(&before, &failed)).body,
        "Nothing cleared, and only 60.0 GiB is free. The guard failed: reading space: gone. \
         disk-guard journal says more."
    );
}

#[test]
fn routine_upkeep_says_nothing() {
    let before = status(&[("/", Band::Healthy, 400)], None);
    let upkept = status(
        &[("/", Band::Healthy, 410)],
        Some(pass(1.0, Band::Healthy, "reached", 10)),
    );
    assert_eq!(change(&before, &upkept), None);
}

#[test]
fn the_notice_goes_once_no_disk_is_nearly_full() {
    let urgent = status(&[("/", Band::Urgent, 60)], None);
    let pressed = status(&[("/", Band::Pressure, 140)], None);
    assert_eq!(change(&urgent, &pressed), Some(Change::Withdraw));
    let calm = status(&[("/", Band::Healthy, 400)], None);
    assert_eq!(
        change(&pressed, &calm),
        None,
        "nothing was shown for pressure"
    );
}
