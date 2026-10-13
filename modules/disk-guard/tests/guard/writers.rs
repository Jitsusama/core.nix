use crate::procs::text;
use disk_guard::writers::{Writers, descendants, sample};
use std::path::Path;

/// A process in a fake `/proc`: its parent, start, cgroup and what it has
/// written so far.
fn process(proc: &Path, pid: u32, parent: u32, name: &str, written: u64) {
    let mut fields = vec!["0".to_string(); 49];
    fields[0] = "S".into();
    fields[1] = parent.to_string();
    fields[19] = "500".into();
    text(
        proc,
        &format!("{pid}/stat"),
        &format!("{pid} ({name}) {}\n", fields.join(" ")),
    );
    text(
        proc,
        &format!("{pid}/io"),
        &format!("rchar: 0\nwchar: {written}\nread_bytes: 0\nwrite_bytes: {written}\n"),
    );
    text(
        proc,
        &format!("{pid}/cgroup"),
        "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-kitty.scope\n",
    );
}

const GIB: u64 = 1 << 30;

#[test]
fn the_writer_of_more_than_half_the_recent_writes_is_found() {
    let proc = tempfile::tempdir().unwrap();
    let proc = proc.path();
    let mut writers = Writers::default();
    process(proc, 10, 1, "dd", 0);
    process(proc, 11, 1, "cargo", 0);
    process(proc, 12, 1, "gone-soon", 0);
    writers.observe(sample(proc), 1_000.0);
    process(proc, 10, 1, "dd", 3 * GIB);
    process(proc, 11, 1, "cargo", GIB / 2);
    std::fs::remove_dir_all(proc.join("12")).unwrap();
    writers.observe(sample(proc), 1_030.0);
    let top = writers.dominant(60.0, 1_030.0).expect("dd wrote most");
    assert_eq!(
        (top.pid, top.name.as_str(), top.written),
        (10, "dd", 3 * GIB)
    );
    assert!(
        (top.rate - 3.0 * GIB as f64 / 30.0).abs() < 1.0,
        "{}",
        top.rate
    );
    assert_eq!(
        top.cgroup.as_deref(),
        Some("/user.slice/user-1000.slice/user@1000.service/app.slice/app-kitty.scope")
    );
}

#[test]
fn no_one_writer_of_an_even_split_is_dominant() {
    let proc = tempfile::tempdir().unwrap();
    let proc = proc.path();
    let mut writers = Writers::default();
    process(proc, 10, 1, "one", 0);
    process(proc, 11, 1, "other", 0);
    writers.observe(sample(proc), 1_000.0);
    process(proc, 10, 1, "one", GIB);
    process(proc, 11, 1, "other", GIB);
    writers.observe(sample(proc), 1_030.0);
    assert_eq!(writers.dominant(60.0, 1_030.0), None);
}

#[test]
fn writes_before_the_window_count_for_nothing() {
    let proc = tempfile::tempdir().unwrap();
    let proc = proc.path();
    let mut writers = Writers::default();
    process(proc, 10, 1, "dd", 0);
    writers.observe(sample(proc), 1_000.0);
    process(proc, 10, 1, "dd", 5 * GIB);
    writers.observe(sample(proc), 1_010.0);
    // Then nothing more for long past the window.
    writers.observe(sample(proc), 1_200.0);
    assert_eq!(writers.dominant(60.0, 1_200.0), None);
}

#[test]
fn a_writers_descendants_are_its_children_and_theirs() {
    let proc = tempfile::tempdir().unwrap();
    let proc = proc.path();
    process(proc, 10, 1, "make", 0);
    process(proc, 11, 10, "cc", 0);
    process(proc, 12, 11, "as", 0);
    process(proc, 13, 1, "elsewhere", 0);
    let mut found = descendants(&sample(proc), 10);
    found.sort();
    assert_eq!(found, [10, 11, 12]);
}
