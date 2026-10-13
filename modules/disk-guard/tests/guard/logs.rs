use disk_guard::anchor::Dir;
use disk_guard::logs::{oversized, trim};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::Path;

/// A log of numbered lines, `lines` long.
fn log(path: &Path, lines: usize) {
    let text: String = (0..lines).map(|n| format!("line {n:06}\n")).collect();
    fs::write(path, text).unwrap();
}

#[test]
fn only_logs_past_the_limit_are_oversized() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("app/deep")).unwrap();
    log(&root.path().join("app/deep/big.log"), 1000);
    log(&root.path().join("app/small.log"), 10);
    log(&root.path().join("app/big.txt"), 1000);
    symlink(
        root.path().join("app/deep/big.log"),
        root.path().join("app/link.log"),
    )
    .unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    log(&elsewhere.path().join("far.log"), 1000);
    symlink(elsewhere.path(), root.path().join("app/away")).unwrap();
    assert_eq!(
        oversized(root.path(), 1024),
        [root.path().join("app/deep/big.log")]
    );
}

#[test]
fn a_trimmed_log_keeps_its_last_whole_lines() {
    let root = tempfile::tempdir().unwrap();
    log(&root.path().join("a.log"), 1000);
    let dropped = trim(&Dir::open(root.path()).unwrap(), Path::new("a.log"), 100).unwrap();
    let kept = fs::read_to_string(root.path().join("a.log")).unwrap();
    // Each line is 12 bytes: the last 100 hold a partial line and 8 whole.
    assert_eq!(
        kept,
        (992..1000)
            .map(|n| format!("line {n:06}\n"))
            .collect::<String>()
    );
    assert_eq!(dropped, 12_000 - 96);
}

#[test]
fn a_tail_longer_than_one_copy_is_kept_whole() {
    let root = tempfile::tempdir().unwrap();
    log(&root.path().join("a.log"), 400_000);
    // 3.6 MB, four copies' worth.
    let keep = 3 * 12 * 100_000;
    trim(&Dir::open(root.path()).unwrap(), Path::new("a.log"), keep).unwrap();
    let kept = fs::read_to_string(root.path().join("a.log")).unwrap();
    assert_eq!(
        kept,
        (100_000..400_000)
            .map(|n| format!("line {n:06}\n"))
            .collect::<String>()
    );
}

#[test]
fn a_tail_with_no_line_break_is_kept_whole() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.log"), vec![b'x'; 5000]).unwrap();
    trim(&Dir::open(root.path()).unwrap(), Path::new("a.log"), 100).unwrap();
    assert_eq!(
        fs::read(root.path().join("a.log")).unwrap(),
        vec![b'x'; 100]
    );
}

#[test]
fn a_writer_holding_it_open_carries_on_at_its_new_end() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("a.log");
    log(&path, 1000);
    let mut writer = OpenOptions::new().append(true).open(&path).unwrap();
    trim(&Dir::open(root.path()).unwrap(), Path::new("a.log"), 24).unwrap();
    writer.write_all(b"after\n").unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "line 000998\nline 000999\nafter\n"
    );
}

#[test]
fn a_small_log_is_left_alone() {
    let root = tempfile::tempdir().unwrap();
    log(&root.path().join("a.log"), 2);
    assert_eq!(
        trim(&Dir::open(root.path()).unwrap(), Path::new("a.log"), 100).unwrap(),
        0
    );
    assert_eq!(
        fs::read_to_string(root.path().join("a.log")).unwrap(),
        "line 000000\nline 000001\n"
    );
}

#[test]
fn a_log_is_never_trimmed_through_a_symlink() {
    let root = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    log(&elsewhere.path().join("far.log"), 1000);
    symlink(elsewhere.path().join("far.log"), root.path().join("a.log")).unwrap();
    symlink(elsewhere.path(), root.path().join("away")).unwrap();
    let dir = Dir::open(root.path()).unwrap();
    assert!(trim(&dir, Path::new("a.log"), 100).is_err());
    assert!(trim(&dir, Path::new("away/far.log"), 100).is_err());
    assert_eq!(
        fs::metadata(elsewhere.path().join("far.log"))
            .unwrap()
            .len(),
        12_000
    );
}
