use disk_guard::activity::Process;
use disk_guard::vouch::{Answer, answer, merge, reached_by};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const ASKER: u32 = 1000;

fn mine(path: &Path) -> bool {
    !path.starts_with("/home/u/src/shared")
}

fn root_process() -> Process {
    Process {
        pid: 300,
        uid: 0,
        start_ticks: 77,
        name: "rustc".into(),
        exe: Some(PathBuf::from("/nix/store/abc-rustc/bin/rustc")),
        cwd: Some(PathBuf::from("/home/u/src/app")),
        cmdline: vec![
            OsString::from("rustc"),
            OsString::from("--crate-name"),
            OsString::from("app"),
            OsString::from("--secret=hunter2"),
        ],
        env: vec![
            (
                OsString::from("CARGO_TARGET_DIR"),
                OsString::from("/home/u/src/app/target"),
            ),
            (
                OsString::from("CARGO_BUILD_BUILD_DIR"),
                OsString::from("/var/build"),
            ),
        ],
        files: vec![
            PathBuf::from("/home/u/src/app/target/debug/.cargo-lock"),
            PathBuf::from("/root/.bash_history"),
            PathBuf::from("/home/u/src/shared/notes"),
        ],
        maps: vec![
            PathBuf::from("/nix/store/abc-glibc/lib/libc.so.6"),
            PathBuf::from("/home/u/src/app/target/debug/deps/libserde.so"),
        ],
        readable: true,
        write_bytes: Some(9),
        cgroup: Some("0::/system.slice/x.service".into()),
    }
}

fn elsewhere() -> Process {
    Process {
        pid: 301,
        uid: 0,
        start_ticks: 78,
        name: "sshd".into(),
        exe: Some(PathBuf::from("/nix/store/abc-openssh/bin/sshd")),
        cwd: Some(PathBuf::from("/")),
        cmdline: vec![OsString::from("sshd: listener")],
        files: vec![PathBuf::from("/var/log/auth.log")],
        readable: true,
        ..Default::default()
    }
}

fn asked() -> Answer {
    answer(
        &[root_process(), elsewhere()],
        &[PathBuf::from("/home/u/src")],
        mine,
    )
}

#[test]
fn a_process_touching_the_askers_roots_is_told_of_only_there() {
    let told = asked();
    assert_eq!(told.touching.len(), 1);
    let seen = &told.touching[0];
    assert_eq!((seen.pid, seen.start_ticks, seen.uid), (300, 77, 0));
    assert_eq!(seen.cwd.as_deref(), Some(Path::new("/home/u/src/app")));
    assert_eq!(
        seen.files,
        [PathBuf::from("/home/u/src/app/target/debug/.cargo-lock")]
    );
    assert_eq!(
        seen.maps,
        [PathBuf::from(
            "/home/u/src/app/target/debug/deps/libserde.so"
        )]
    );
    assert_eq!(
        seen.env,
        [(
            OsString::from("CARGO_TARGET_DIR"),
            OsString::from("/home/u/src/app/target")
        )]
    );
    assert_eq!(
        seen.exe.as_deref(),
        Some(Path::new("rustc")),
        "its name, from elsewhere"
    );
    assert_eq!(
        seen.cmdline,
        [OsString::from("rustc")],
        "only the name it was called by"
    );
    assert!(seen.readable);
    assert_eq!((seen.write_bytes, seen.cgroup.as_deref()), (None, None));
}

#[test]
fn a_process_touching_nothing_of_the_askers_is_vouched_for_and_no_more() {
    let told = asked();
    assert!(told.vouched.contains(&(301, 78)));
    assert!(told.vouched.contains(&(300, 77)));
    assert!(!told.touching.iter().any(|seen| seen.pid == 301));
}

#[test]
fn the_proc_macro_server_keeps_the_word_that_says_so() {
    let mut server = root_process();
    server.name = "rust-analyzer".into();
    server.cmdline = vec![
        OsString::from("rust-analyzer"),
        OsString::from("proc-macro"),
    ];
    let told = answer(&[server], &[PathBuf::from("/home/u/src")], mine);
    assert_eq!(
        told.touching[0].cmdline,
        [
            OsString::from("rust-analyzer"),
            OsString::from("proc-macro")
        ]
    );
}

#[test]
fn a_process_the_system_could_not_read_is_not_vouched_for() {
    let mut hidden = elsewhere();
    hidden.readable = false;
    let told = answer(&[hidden], &[PathBuf::from("/home/u/src")], mine);
    assert!(told.vouched.is_empty());
}

fn unreadable(pid: u32, start_ticks: u64) -> Process {
    Process {
        pid,
        uid: 0,
        start_ticks,
        name: "?".into(),
        readable: false,
        ..Default::default()
    }
}

#[test]
fn what_the_system_vouches_for_replaces_what_the_asker_could_not_read() {
    let mut mine = Process {
        pid: 500,
        uid: ASKER,
        readable: true,
        ..Default::default()
    };
    mine.cwd = Some(PathBuf::from("/home/u"));
    let local = vec![
        mine.clone(),
        unreadable(300, 77),
        unreadable(301, 78),
        unreadable(302, 79),
    ];
    let merged = merge(local, &asked());
    assert_eq!(merged[0], mine, "what the asker read itself stands");
    assert_eq!(
        merged[1].files,
        [PathBuf::from("/home/u/src/app/target/debug/.cargo-lock")]
    );
    assert!(merged[1].readable);
    assert!(merged[2].readable, "vouched for, touching nothing");
    assert!(merged[2].files.is_empty() && merged[2].cwd.is_none());
    assert!(!merged[3].readable, "never asked about, still unknown");
}

#[test]
fn a_reused_pid_is_not_what_the_system_vouched_for() {
    let merged = merge(vec![unreadable(300, 999)], &asked());
    assert!(!merged[0].readable);
}

#[test]
fn a_path_is_the_askers_to_hear_of_beneath_anything_it_owns() {
    let me = unsafe { libc::geteuid() };
    if me == 0 {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("target/debug/.cargo-lock");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, "").unwrap();
    assert!(reached_by(&file, me));
    assert!(reached_by(&dir.path().join("gone (deleted)"), me));
    assert!(
        !reached_by(&file, me + 1),
        "nothing on the way is another's"
    );
    assert!(!reached_by(Path::new("/etc/hostname"), me));
    assert!(
        !reached_by(Path::new("src/vouch.rs"), me),
        "a relative path means nothing to the system daemon"
    );
}
