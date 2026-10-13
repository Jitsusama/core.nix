use disk_guard::clock::Awake;
use disk_guard::plan::Class;
use disk_guard::scratch::{Entry, candidate, entries};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};

fn touch(path: &Path, seconds: i64) {
    let at = Timespec {
        tv_sec: seconds,
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

#[test]
fn each_entry_is_dated_by_its_last_write() {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("stress-run");
    fs::create_dir_all(run.join("out")).unwrap();
    fs::write(run.join("out/result.bin"), vec![0u8; 2048]).unwrap();
    touch(&run.join("out/result.bin"), 5_000);
    touch(&run.join("out"), 1_000);
    touch(&run, 1_000);
    fs::write(root.path().join("notes.txt"), "abc").unwrap();
    touch(&root.path().join("notes.txt"), 2_000);
    std::os::unix::fs::symlink("/nowhere", root.path().join("link")).unwrap();
    touch(&root.path().join("link"), 3_000);
    let mut found = entries(root.path());
    found.sort_by(|a, b| a.path.cmp(&b.path));
    let expect = |name: &str, written: f64| Entry {
        path: root.path().join(name),
        written,
    };
    assert_eq!(
        found,
        [
            expect("link", 3_000.0),
            expect("notes.txt", 2_000.0),
            expect("stress-run", 5_000.0)
        ]
    );
}

#[test]
fn a_missing_scratch_directory_has_nothing_in_it() {
    assert_eq!(entries(Path::new("/nonexistent/scratch")), []);
}

#[test]
fn an_entry_is_idle_since_its_last_write_with_its_sources_healthy_window() {
    let entry = Entry {
        path: PathBuf::from("/home/u/.dev/cargo/env-1"),
        written: 1_000.0,
    };
    let made = candidate(
        &entry,
        Some(30.0 * 86_400.0),
        77,
        &Awake::default(),
        5_000.0,
    );
    assert_eq!(
        made.class,
        Class::Scratch {
            healthy: Some(30.0 * 86_400.0)
        }
    );
    assert_eq!(made.idle, 4_000.0);
    assert_eq!(made.size, 77);
    let unread = Entry {
        written: f64::INFINITY,
        ..entry
    };
    assert_eq!(
        candidate(&unread, None, 77, &Awake::default(), 5_000.0).idle,
        0.0
    );
}
