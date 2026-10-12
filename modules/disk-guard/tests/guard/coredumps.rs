use disk_guard::clock::Awake;
use disk_guard::coredumps::{Dump, candidate, find};
use disk_guard::plan::Class;
use std::fs;
use std::path::PathBuf;

const BOOT: &str = "8166d71665444de08066416cb0f1dee8";

#[test]
fn dumps_are_read_from_systemds_names() {
    let root = tempfile::tempdir().unwrap();
    let names = [
        format!("core.git.1000.{BOOT}.390316.1791843607000000.zst"),
        // A process name can hold dots of its own.
        format!("core.nix.daemon.0.{BOOT}.4115963.1791838029000000"),
        // systemd writes into a hidden file and renames it when done.
        format!(".#core.cargo.1000.{BOOT}.7.1791843607000000.zst"),
        "core".to_owned(),
        "notes.txt".to_owned(),
    ];
    for name in &names {
        fs::write(root.path().join(name), "dump").unwrap();
    }
    let mut found = find(root.path());
    found.sort_by_key(|dump| dump.pid);
    assert_eq!(
        found,
        [
            Dump {
                path: root.path().join(&names[0]),
                uid: 1000,
                pid: 390316,
                at: 1_791_843_607.0,
                size: 4,
            },
            Dump {
                path: root.path().join(&names[1]),
                uid: 0,
                pid: 4115963,
                at: 1_791_838_029.0,
                size: 4,
            },
        ]
    );
}

#[test]
fn a_dump_ages_from_when_it_was_taken() {
    let dump = Dump {
        path: PathBuf::from("/var/lib/systemd/coredump/core.x"),
        uid: 0,
        pid: 1,
        at: 1_000.0,
        size: 9,
    };
    let made = candidate(&dump, &Awake::default(), 5_000.0);
    assert_eq!(made.class, Class::Coredump);
    assert_eq!(made.idle, 4_000.0);
    assert_eq!(made.size, 9);
    assert_eq!(made.path, dump.path);
}

#[test]
fn a_dump_dated_ahead_of_the_clock_is_new() {
    let dump = Dump {
        path: PathBuf::from("/c/core.x"),
        uid: 0,
        pid: 1,
        at: 9_000.0,
        size: 9,
    };
    assert_eq!(candidate(&dump, &Awake::default(), 5_000.0).idle, 0.0);
}
