use disk_guard::band::Band;
use disk_guard::daemon::{FilesystemStatus, LastPass, Status, Writing};
use disk_guard::notices::{Change, Notice, Urgency, change};
use std::path::PathBuf;

const GIB: u64 = 1 << 30;

fn status(bands: &[(&str, Band, u64)], last: Option<LastPass>) -> Status {
    Status {
        version: 1,
        heartbeat: 0.0,
        filesystems: bands
            .iter()
            .map(|&(path, band, free)| FilesystemStatus {
                path: PathBuf::from(path),
                band,
                free: free * GIB,
                total: 1000 * GIB,
                forecast: None,
            })
            .collect(),
        last,
        frozen: Vec::new(),
        writer: None,
    }
}

fn pass(at: f64, band: Band, end: &str, freed: u64) -> LastPass {
    LastPass {
        at,
        path: PathBuf::from("/home"),
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
        &[("/home", Band::Critical, 2)],
        Some(pass(1.0, Band::Critical, "exhausted", 0)),
    );
    let mut frozen = short.clone();
    frozen.frozen.push(disk_guard::writers::Frozen {
        unit: "disk-guard-freeze-700.scope".to_owned(),
        pid: 700,
        start: 500,
        name: "dd".to_owned(),
        path: PathBuf::from("/home"),
        at: 2.0,
    });
    let notice = shown(change(&short, &frozen));
    assert_eq!(notice.summary, "Froze dd, which was filling /home");
    assert_eq!(
        notice.body,
        "Nothing else was safe to take. It goes on by itself once there's room, or now with \
         disk-guard thaw."
    );
    assert_eq!(notice.urgency, Urgency::Critical);
    assert_eq!(change(&frozen, &frozen), None, "told once");
}

#[test]
fn nothing_changing_says_nothing() {
    let calm = status(&[("/home", Band::Healthy, 400)], None);
    assert_eq!(change(&calm, &calm), None);
    let pressed = status(
        &[("/home", Band::Pressure, 140)],
        Some(pass(1.0, Band::Pressure, "reached", 1)),
    );
    assert_eq!(change(&pressed, &pressed), None);
}

#[test]
fn entering_a_band_says_so_and_critical_is_urgent() {
    let calm = status(&[("/home", Band::Healthy, 400)], None);
    let pressed = status(&[("/home", Band::Pressure, 140)], None);
    let notice = shown(change(&calm, &pressed));
    assert_eq!(notice.summary, "/home is under pressure");
    assert_eq!(notice.body, "140.0 GiB free of 1000.0 GiB. Making room.");
    assert_eq!(notice.urgency, Urgency::Normal);
    let critical = status(&[("/home", Band::Critical, 20)], None);
    let notice = shown(change(&pressed, &critical));
    assert_eq!(notice.summary, "/home is critically full");
    assert_eq!(notice.urgency, Urgency::Critical);
    let urgent = status(&[("/home", Band::Urgent, 60)], None);
    assert_eq!(
        change(&critical, &urgent),
        None,
        "easing below healthy says nothing"
    );
}

#[test]
fn a_pass_left_short_names_who_is_writing_the_most() {
    let before = status(&[("/home", Band::Urgent, 60)], None);
    let mut short = status(
        &[("/home", Band::Critical, 25)],
        Some(pass(1.0, Band::Critical, "exhausted", 5)),
    );
    short.writer = Some(Writing {
        name: "cargo".to_owned(),
        pid: 4242,
        rate: 120.0 * f64::from(1 << 20),
    });
    let notice = shown(change(&before, &short));
    assert_eq!(
        notice.body,
        "25.0 GiB free. Nothing more is safe to take; disk-guard plan shows what's left. \
         Most of the writing is cargo's (pid 4242), at 120.0 MiB a second."
    );
}

#[test]
fn the_worst_of_several_filesystems_is_told() {
    let calm = status(
        &[("/", Band::Healthy, 400), ("/home", Band::Healthy, 400)],
        None,
    );
    let both = status(
        &[("/", Band::Pressure, 140), ("/home", Band::Urgent, 60)],
        None,
    );
    assert_eq!(shown(change(&calm, &both)).summary, "/home is nearly full");
}

#[test]
fn a_pass_below_healthy_tells_what_it_freed_and_how_it_ended() {
    let before = status(&[("/home", Band::Urgent, 60)], None);
    let reached = status(
        &[("/home", Band::Healthy, 200)],
        Some(pass(1.0, Band::Urgent, "reached", 140)),
    );
    let notice = shown(change(&before, &reached));
    assert_eq!(notice.summary, "Freed 140.0 GiB on /home");
    assert_eq!(notice.body, "200.0 GiB free now.");
    assert_eq!(notice.urgency, Urgency::Normal);
    let short = status(
        &[("/home", Band::Critical, 25)],
        Some(pass(1.0, Band::Critical, "exhausted", 5)),
    );
    let notice = shown(change(&before, &short));
    assert_eq!(notice.summary, "Freed 5.0 GiB on /home, short of room");
    assert_eq!(
        notice.body,
        "25.0 GiB free. Nothing more is safe to take; disk-guard plan shows what's left."
    );
    assert_eq!(notice.urgency, Urgency::Critical);
    let stalled = status(
        &[("/home", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "stalled", 0)),
    );
    let notice = shown(change(&before, &stalled));
    assert_eq!(notice.summary, "Nothing freed on /home, short of room");
    assert_eq!(
        notice.body,
        "60.0 GiB free. Deleting stopped freeing space, so the guard waits before trying again."
    );
    let failed = status(
        &[("/home", Band::Urgent, 60)],
        Some(pass(1.0, Band::Urgent, "failed: reading space: gone", 0)),
    );
    assert_eq!(
        shown(change(&before, &failed)).body,
        "60.0 GiB free. It failed: reading space: gone."
    );
}

#[test]
fn routine_upkeep_says_nothing() {
    let before = status(&[("/home", Band::Healthy, 400)], None);
    let upkept = status(
        &[("/home", Band::Healthy, 410)],
        Some(pass(1.0, Band::Healthy, "reached", 10)),
    );
    assert_eq!(change(&before, &upkept), None);
}

#[test]
fn the_notice_goes_once_every_disk_is_healthy_again() {
    let pressed = status(&[("/home", Band::Pressure, 140)], None);
    let calm = status(&[("/home", Band::Healthy, 400)], None);
    assert_eq!(change(&pressed, &calm), Some(Change::Withdraw));
}
