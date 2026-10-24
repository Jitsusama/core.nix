use disk_guard::clock::Awake;
use disk_guard::plan::Class;
use disk_guard::temp::{Entry, candidate, leftovers, survey};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn backdate(path: &Path) {
    let at = Timespec {
        tv_sec: 1_000,
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

fn entry(name: &str) -> Entry {
    Entry {
        path: PathBuf::from("/tmp").join(name),
        uid: 1000,
        newest: 1_000.0,
        size: 4096,
        socket: false,
        checkout: false,
    }
}

fn kept(entries: Vec<Entry>, tmpdirs: &[PathBuf]) -> Vec<String> {
    let chosen = leftovers(entries, 1000, tmpdirs);
    chosen
        .iter()
        .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn each_top_entry_is_surveyed_with_what_lies_under_it() {
    let temp = tempfile::tempdir().unwrap();
    let build = temp.path().join("build-a1b2c3d4");
    fs::create_dir_all(build.join("deep/er")).unwrap();
    fs::write(build.join("deep/er/out.bin"), vec![1u8; 10_000]).unwrap();
    let server = temp.path().join("server-q9w8e7");
    fs::create_dir_all(server.join("run")).unwrap();
    let _socket = UnixListener::bind(server.join("run/sock")).unwrap();
    let clone = temp.path().join("clone-z1x2c3");
    fs::create_dir_all(clone.join("repo/.git")).unwrap();
    fs::write(temp.path().join("note-123"), "hi").unwrap();
    let mut found = survey(temp.path());
    found.sort_by(|a, b| a.path.cmp(&b.path));
    let names: Vec<_> = found
        .iter()
        .map(|e| e.path.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "build-a1b2c3d4",
            "clone-z1x2c3",
            "note-123",
            "server-q9w8e7"
        ]
    );
    assert_eq!(found[0].size, 10_000);
    assert!(!found[0].socket && !found[0].checkout);
    assert!(found[1].checkout);
    assert_eq!(found[2].size, 2);
    assert!(found[3].socket);
    assert_eq!(found[0].uid, fs::metadata(temp.path()).unwrap().uid());
}

#[test]
fn an_archive_unpacked_with_old_times_is_new() {
    // Unpacking keeps the archive's modification times, but the change
    // time is when the file landed here, which nothing can set back.
    let temp = tempfile::tempdir().unwrap();
    let unpacked = temp.path().join("unpacked-a1b2c3");
    fs::create_dir_all(unpacked.join("src")).unwrap();
    fs::write(unpacked.join("src/lib.rs"), "").unwrap();
    for path in ["src/lib.rs", "src", ""] {
        backdate(&unpacked.join(path));
    }
    let found = survey(temp.path());
    assert!(found[0].newest > now() - 60.0, "{}", found[0].newest);
}

#[test]
fn an_entry_too_big_to_survey_counts_as_new() {
    let temp = tempfile::tempdir().unwrap();
    let big = temp.path().join("big-a1b2c3");
    fs::create_dir_all(&big).unwrap();
    for n in 0..60_000 {
        fs::write(big.join(n.to_string()), "").unwrap();
    }
    crate::assert_exactly(survey(temp.path())[0].newest, f64::INFINITY);
}

#[test]
fn only_my_own_leftovers_with_a_made_up_name_are_offered() {
    let mut theirs = entry("theirs-a1b2c3");
    theirs.uid = 0;
    let entries = vec![
        entry("cargo-install-Xy12Ab"),
        entry(".tmpAb12Cd"),
        entry("tmpab_cd123"),
        entry("vite-server-4242"),
        theirs,
        // Caches meant to be reused: no random or pid suffix.
        entry("jiti"),
        entry("node-compile-cache"),
        // Hidden, or another tool's own.
        entry(".X11-unix"),
        entry(".font-unix-a1b2c3"),
        entry("pi-quest-QEST-20261012-1ZP2M8-a1b2c3"),
        entry("systemd-private-0123456789abcdef-upower.service-a1b2c3"),
    ];
    assert_eq!(
        kept(entries, &[]),
        [
            "cargo-install-Xy12Ab",
            ".tmpAb12Cd",
            "tmpab_cd123",
            "vite-server-4242"
        ]
    );
}

#[test]
fn a_random_suffix_has_a_digit_or_both_cases_and_may_have_an_extension() {
    let entries = vec![
        entry("bash-9ec78a9f1d182548.log"),
        entry("render-aBcDeF"),
        // Words, which mkdtemp all but never draws.
        entry("build-closure"),
        entry("notes-install.txt"),
        entry("2hn63p-closure.txt"),
        // An extension after nothing random.
        entry("matches.txt"),
    ];
    assert_eq!(
        kept(entries, &[]),
        ["bash-9ec78a9f1d182548.log", "render-aBcDeF"]
    );
}

#[test]
fn a_name_ending_in_my_uid_is_a_programs_own_not_a_one_off() {
    let entries = vec![
        entry("zellij-1000"),
        entry("1Password-1000"),
        entry("server-10001"),
    ];
    assert_eq!(kept(entries, &[]), ["server-10001"]);
}

#[test]
fn a_leftover_holding_a_socket_or_checkout_is_kept() {
    let socket = Entry {
        socket: true,
        ..entry("agent-a1b2c3")
    };
    let checkout = Entry {
        checkout: true,
        ..entry("clone-a1b2c3")
    };
    assert_eq!(kept(vec![socket, checkout], &[]), Vec::<String>::new());
}

#[test]
fn a_link_is_never_surveyed_since_taking_it_frees_nothing() {
    // The harness gives each loaded quest a TMPDIR of
    // /tmp/pi-QEST-<id>, a link into the quest's workspace. Its name
    // ends in a run with digits, so it read as a one-off, and taking it
    // freed its own 85 bytes while a job reading through it met "No such
    // file or directory". Eleven went at once on 24 October.
    let temp = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    fs::write(workspace.path().join("run.log"), vec![1u8; 10_000]).unwrap();
    std::os::unix::fs::symlink(workspace.path(), temp.path().join("pi-QEST-A1B2C3")).unwrap();
    std::os::unix::fs::symlink("/nowhere", temp.path().join("dangling-x9y8z7")).unwrap();
    fs::write(temp.path().join("note-123"), "hi").unwrap();

    let names: Vec<_> = survey(temp.path())
        .iter()
        .map(|e| e.path.file_name().unwrap().to_owned())
        .collect();

    assert_eq!(names, ["note-123"]);
}

#[test]
fn a_live_tmpdir_named_by_a_link_keeps_the_link() {
    // Resolving the TMPDIR alone leaves only where the link points, so
    // the link itself never matched the entry it names.
    let temp = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let link = temp.path().join("pi-QEST-A1B2C3");
    std::os::unix::fs::symlink(workspace.path(), &link).unwrap();
    let entries = vec![Entry {
        path: link.clone(),
        ..entry("pi-QEST-A1B2C3")
    }];

    assert!(leftovers(entries, 1000, &[link]).is_empty());
}

#[test]
fn a_live_processes_tmpdir_keeps_what_holds_it() {
    let tmpdirs = [
        PathBuf::from("/tmp/nix-shell.a1b2c3/inner"),
        PathBuf::from("/tmp/shell-x9y8z7"),
    ];
    let entries = vec![
        entry("nix-shell.a1b2c3"),
        entry("shell-x9y8z7"),
        entry("shell-x9y8z7a"),
    ];
    assert_eq!(kept(entries, &tmpdirs), ["shell-x9y8z7a"]);
}

#[test]
fn a_leftover_is_idle_for_the_awake_time_since_its_last_change() {
    let made = candidate(&entry("build-a1b2c3"), &Awake::default(), 4_600.0);
    assert_eq!(made.class, Class::Temp);
    crate::assert_exactly(made.idle, 3_600.0);
    assert_eq!(made.size, 4096);
    let unread = Entry {
        newest: f64::INFINITY,
        ..entry("build-a1b2c3")
    };
    crate::assert_exactly(candidate(&unread, &Awake::default(), 4_600.0).idle, 0.0);
}
