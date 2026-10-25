use disk_guard::anchor::Dir;
use disk_guard::clock::Awake;
use disk_guard::pass::Outcome;
use disk_guard::plan::Class;
use disk_guard::reclaim::trashed;
use disk_guard::trash::{Item, candidate, items};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Seconds since the epoch of a local time, as `date` reads it.
fn local(stamp: &str) -> f64 {
    let out = Command::new("date")
        .args(["-d", stamp, "+%s"])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

/// A trash with something deleted at `when`, as a file manager leaves it.
fn trash(root: &Path, name: &str, when: &str) {
    fs::create_dir_all(root.join("files")).unwrap();
    fs::create_dir_all(root.join("info")).unwrap();
    fs::write(root.join("files").join(name), vec![0u8; 3000]).unwrap();
    let info = format!("[Trash Info]\nPath=/home/u/{name}\nDeletionDate={when}\n");
    fs::write(root.join("info").join(format!("{name}.trashinfo")), info).unwrap();
}

fn sorted(mut found: Vec<Item>) -> Vec<Item> {
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

#[test]
fn each_trashed_thing_is_dated_by_when_it_was_deleted_in_local_time() {
    let root = tempfile::tempdir().unwrap();
    trash(root.path(), "report.pdf", "2026-10-01T12:30:00");
    fs::create_dir_all(root.path().join("files/photos/2026")).unwrap();
    fs::write(root.path().join("files/photos/2026/a.jpg"), vec![0u8; 500]).unwrap();
    let info = "[Trash Info]\nPath=/home/u/photos\nDeletionDate=2026-09-01T08:00:00\n";
    fs::write(root.path().join("info/photos.trashinfo"), info).unwrap();
    let found = sorted(items(root.path()));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].path, root.path().join("files/photos"));
    crate::assert_exactly(found[0].deleted, local("2026-09-01T08:00:00"));
    assert_eq!(found[0].size, 500);
    assert_eq!(found[1].path, root.path().join("files/report.pdf"));
    crate::assert_exactly(found[1].deleted, local("2026-10-01T12:30:00"));
    assert_eq!(found[1].size, 3000);
}

#[test]
fn a_file_with_no_readable_date_is_dated_by_when_it_arrived() {
    let root = tempfile::tempdir().unwrap();
    trash(root.path(), "garbled", "yesterday-ish");
    trash(root.path(), "orphan", "2026-10-01T12:30:00");
    fs::remove_file(root.path().join("info/orphan.trashinfo")).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let found = items(root.path());
    assert_eq!(found.len(), 2);
    for item in found {
        assert!((item.deleted - now).abs() < 60.0, "{item:?}");
    }
}

#[test]
fn an_info_file_whose_file_is_gone_is_offered_to_be_tidied() {
    let root = tempfile::tempdir().unwrap();
    trash(root.path(), "gone.txt", "2026-10-01T12:30:00");
    fs::remove_file(root.path().join("files/gone.txt")).unwrap();
    let found = items(root.path());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, root.path().join("files/gone.txt"));
    assert_eq!(found[0].size, 0);
}

#[test]
fn a_trashed_thing_is_idle_for_the_awake_time_since_its_deletion() {
    let item = Item {
        path: "/home/u/.local/share/Trash/files/a".into(),
        deleted: 1_000.0,
        size: 9,
    };
    let made = candidate(&item, &Awake::default(), 5_000.0);
    assert_eq!(made.class, Class::Trash);
    crate::assert_exactly(made.idle, 4_000.0);
    assert_eq!(made.size, 9);
}

#[test]
fn reclaiming_takes_the_file_then_its_info() {
    let root = tempfile::tempdir().unwrap();
    trash(root.path(), "report.pdf", "2026-10-01T12:30:00");
    trash(root.path(), "other.pdf", "2026-10-01T12:30:00");
    let found = items(root.path());
    let item = found
        .iter()
        .find(|item| item.path.ends_with("report.pdf"))
        .unwrap();
    let made = candidate(item, &Awake::default(), 0.0);
    let mut unlinked = 0;
    let outcome = trashed(
        &Dir::open(root.path()).unwrap(),
        root.path(),
        &made,
        &mut || unlinked += 1,
    );
    assert_eq!(outcome, Outcome::Reclaimed);
    assert_eq!(unlinked, 1, "its file, told so a pacer can rest");
    assert!(!root.path().join("files/report.pdf").exists());
    assert!(!root.path().join("info/report.pdf.trashinfo").exists());
    assert!(root.path().join("files/other.pdf").exists());
    assert!(root.path().join("info/other.pdf.trashinfo").exists());
}

#[test]
fn reclaiming_something_outside_the_trash_fails_untouched() {
    let root = tempfile::tempdir().unwrap();
    trash(root.path(), "report.pdf", "2026-10-01T12:30:00");
    fs::create_dir_all(root.path().join("elsewhere")).unwrap();
    fs::write(root.path().join("elsewhere/keep"), "").unwrap();
    let stray = Item {
        path: root.path().join("elsewhere/keep"),
        deleted: 0.0,
        size: 0,
    };
    let made = candidate(&stray, &Awake::default(), 0.0);
    let outcome = trashed(
        &Dir::open(root.path()).unwrap(),
        root.path(),
        &made,
        &mut || {},
    );
    assert!(matches!(outcome, Outcome::Failed(_)), "{outcome:?}");
    assert!(root.path().join("elsewhere/keep").exists());
}
