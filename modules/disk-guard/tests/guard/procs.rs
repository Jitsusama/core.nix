use disk_guard::linux::{lock_waiters, processes};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

pub(crate) fn text(root: &Path, path: &str, value: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

pub(crate) fn link(root: &Path, path: &str, to: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(to, path).unwrap();
}

/// `/proc/<pid>/stat` with its name, flags and start in clock ticks.
pub(crate) fn stat(pid: u32, name: &str, flags: u64, start: u64) -> String {
    let mut fields = vec!["0".to_string(); 49];
    fields[0] = "S".into();
    fields[6] = flags.to_string();
    fields[19] = start.to_string();
    format!("{pid} ({name}) {}\n", fields.join(" "))
}

/// A fake `/proc` that opens its hidden directory again when dropped, or
/// the temp directory holding it couldn't be removed.
struct FakeProc(tempfile::TempDir);

impl FakeProc {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fake_proc(dir.path());
        FakeProc(dir)
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Drop for FakeProc {
    fn drop(&mut self) {
        let hidden = self.0.path().join("200/fd");
        let _ = fs::set_permissions(hidden, fs::Permissions::from_mode(0o700));
    }
}

/// A fake `/proc`: a cargo build, a process of another account whose
/// details are hidden, and a kernel thread.
fn fake_proc(root: &Path) {
    text(root, "100/stat", &stat(100, "cargo) (build", 0, 500));
    text(
        root,
        "100/status",
        "Name:\tcargo\nUid:\t1000\t1000\t1000\t1000\n",
    );
    text(root, "100/cmdline", "cargo\0build\0--release\0");
    text(
        root,
        "100/environ",
        "HOME=/home/u\0CARGO_TARGET_DIR=/home/u/t\0SECRET=x\0CARGO_HOME=/home/u/.dev/cargo/a\0",
    );
    link(root, "100/cwd", "/home/u/src/app");
    link(root, "100/exe", "/nix/store/abc-cargo/bin/cargo");
    link(root, "100/fd/0", "/dev/null");
    link(root, "100/fd/3", "/home/u/src/app/target/debug/.cargo-lock");
    link(root, "100/fd/4", "socket:[12345]");
    text(
        root,
        "100/maps",
        "55d0-55d1 r-xp 00000000 00:23 10 /home/u/src/app/target/debug/deps/app-0123 (deleted)\n\
         7f00-7f01 r--p 00000000 00:00 0 [heap]\n\
         7f02-7f03 r-xp 00000000 00:23 11 /nix/store/abc-glibc/lib/libc.so.6\n",
    );
    text(
        root,
        "100/io",
        "rchar: 1\nwchar: 2\nread_bytes: 3\nwrite_bytes: 4096\n",
    );
    text(
        root,
        "100/cgroup",
        "0::/user.slice/user-1000.slice/app.slice/kitty.scope\n",
    );

    text(root, "200/stat", &stat(200, "sshd", 0, 7));
    text(root, "200/status", "Name:\tsshd\nUid:\t0\t0\t0\t0\n");
    text(root, "200/cmdline", "sshd: listener\0");
    text(root, "200/cgroup", "0::/system.slice/sshd.service\n");
    fs::create_dir_all(root.join("200/fd")).unwrap();
    fs::set_permissions(root.join("200/fd"), fs::Permissions::from_mode(0o000)).unwrap();

    text(root, "2/stat", &stat(2, "kthreadd", 0x0020_0000, 0));
    text(root, "2/status", "Name:\tkthreadd\nUid:\t0\t0\t0\t0\n");
    text(root, "self/stat", "not a process\n");
}

#[test]
fn a_process_is_read_from_its_proc_directory() {
    let root = FakeProc::new();
    let all = processes(root.path()).unwrap();
    let cargo = all
        .iter()
        .find(|p| p.pid == 100)
        .expect("cargo wasn't read");
    assert_eq!(cargo.uid, 1000);
    assert_eq!(cargo.name, "cargo) (build");
    assert_eq!(cargo.start_ticks, 500);
    assert!(cargo.readable);
    assert_eq!(cargo.cwd.as_deref(), Some(Path::new("/home/u/src/app")));
    assert_eq!(
        cargo.exe.as_deref(),
        Some(Path::new("/nix/store/abc-cargo/bin/cargo"))
    );
    let words: Vec<OsString> = ["cargo", "build", "--release"].map(OsString::from).into();
    assert_eq!(cargo.cmdline, words);
    // Only build variables are kept from the environment.
    assert_eq!(
        cargo.env,
        vec![
            (
                OsString::from("CARGO_TARGET_DIR"),
                OsString::from("/home/u/t")
            ),
            (
                OsString::from("CARGO_HOME"),
                OsString::from("/home/u/.dev/cargo/a")
            )
        ]
    );
    let files: Vec<PathBuf> = ["/dev/null", "/home/u/src/app/target/debug/.cargo-lock"]
        .map(PathBuf::from)
        .into();
    assert_eq!(cargo.files, files);
    let maps: Vec<PathBuf> = [
        "/home/u/src/app/target/debug/deps/app-0123",
        "/nix/store/abc-glibc/lib/libc.so.6",
    ]
    .map(PathBuf::from)
    .into();
    assert_eq!(cargo.maps, maps);
    assert_eq!(cargo.write_bytes, Some(4096));
    assert_eq!(
        cargo.cgroup.as_deref(),
        Some("/user.slice/user-1000.slice/app.slice/kitty.scope")
    );
}

#[test]
fn a_process_whose_details_are_hidden_is_unreadable() {
    let root = FakeProc::new();
    let all = processes(root.path()).unwrap();
    let sshd = all.iter().find(|p| p.pid == 200).expect("sshd wasn't read");
    assert_eq!(sshd.uid, 0);
    assert!(!sshd.readable);
    assert_eq!(sshd.cgroup.as_deref(), Some("/system.slice/sshd.service"));
}

#[test]
fn kernel_threads_and_other_entries_are_left_out() {
    let root = FakeProc::new();
    let mut pids: Vec<u32> = processes(root.path())
        .unwrap()
        .iter()
        .map(|p| p.pid)
        .collect();
    pids.sort_unstable();
    assert_eq!(pids, [100, 200]);
}

/// `/proc/locks` as Linux writes it. The guard (pid 42) holds cargo's
/// lock (inode 7) by flock with a build waiting on it, and a rustc session
/// lock (inode 8) by OFD, which shows no pid, with another waiting. Another
/// filesystem has a lock of its own on an inode 7, with a waiter of its own.
pub(crate) const LOCKS: &str = "\
1: FLOCK  ADVISORY  WRITE 42 00:23:7 0 EOF
1: -> FLOCK  ADVISORY  WRITE 500 00:23:7 0 EOF
2: OFDLCK ADVISORY  WRITE -1 00:23:8 0 EOF
2: -> OFDLCK ADVISORY  WRITE -1 00:23:8 0 EOF
3: FLOCK  ADVISORY  WRITE 900 fd:01:7 0 EOF
3: -> FLOCK  ADVISORY  WRITE 901 fd:01:7 0 EOF
4: POSIX  ADVISORY  READ 77 00:23:9 0 EOF
5: FLOCK  ADVISORY  WRITE 42 00:23:10 0 EOF
";

#[test]
fn waiters_on_the_guards_own_locks_are_found() {
    let root = tempfile::tempdir().unwrap();
    text(root.path(), "locks", LOCKS);
    let locks = root.path().join("locks");
    assert_eq!(lock_waiters(&locks, 42, &[7]).unwrap(), [500]);
    assert_eq!(lock_waiters(&locks, 42, &[7, 8]).unwrap(), [500, -1]);
}

#[test]
fn a_lock_no_one_waits_on_has_no_waiters() {
    let root = tempfile::tempdir().unwrap();
    text(root.path(), "locks", LOCKS);
    let locks = root.path().join("locks");
    assert!(lock_waiters(&locks, 42, &[10]).unwrap().is_empty());
}

/// The kernel lists locks a page at a time, and other locks coming and
/// going between pages can skip a record, so a lock the guard holds that
/// the listing doesn't show means its waiters can't be known.
#[test]
fn a_listing_without_a_lock_the_guard_holds_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    text(root.path(), "locks", LOCKS);
    let locks = root.path().join("locks");
    assert!(lock_waiters(&locks, 42, &[9]).is_err());
    assert!(lock_waiters(&locks, 43, &[7]).is_err());
    assert!(lock_waiters(&locks, 42, &[7, 11]).is_err());
}

/// The same paging can print a record twice.
#[test]
fn a_waiter_listed_twice_is_one_waiter() {
    let root = tempfile::tempdir().unwrap();
    let twice = format!(
        "{LOCKS}1: FLOCK  ADVISORY  WRITE 42 00:23:7 0 EOF\n1: -> FLOCK  ADVISORY  WRITE 500 00:23:7 0 EOF\n"
    );
    text(root.path(), "locks", &twice);
    let locks = root.path().join("locks");
    assert_eq!(lock_waiters(&locks, 42, &[7]).unwrap(), [500]);
}

#[test]
fn a_build_waiting_on_a_held_lock_is_seen() {
    use rustix::fs::{FlockOperation, flock};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".cargo-lock");
    let held = fs::File::create(&path).unwrap();
    flock(&held, FlockOperation::LockExclusive).unwrap();
    let inode = std::os::unix::fs::MetadataExt::ino(&held.metadata().unwrap());
    let waiter = {
        let path = path.clone();
        std::thread::spawn(move || {
            flock(fs::File::open(path).unwrap(), FlockOperation::LockExclusive).unwrap();
        })
    };
    let me = std::process::id();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut waiters = Vec::new();
    while waiters.is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
        waiters = lock_waiters(Path::new("/proc/locks"), me, &[inode]).unwrap();
    }
    assert_eq!(waiters, [me.cast_signed()]);
    drop(held);
    waiter.join().unwrap();
    let again = fs::File::open(&path).unwrap();
    flock(&again, FlockOperation::LockExclusive).unwrap();
    assert!(
        lock_waiters(Path::new("/proc/locks"), me, &[inode])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn this_process_reads_as_itself() {
    let held = tempfile::NamedTempFile::new().unwrap();
    let all = processes(Path::new("/proc")).unwrap();
    let me = all
        .iter()
        .find(|p| p.pid == std::process::id())
        .expect("not found in /proc");
    assert!(me.readable);
    assert_eq!(
        me.cwd.as_deref(),
        Some(std::env::current_dir().unwrap().as_path())
    );
    assert_eq!(
        me.exe.as_deref(),
        Some(std::env::current_exe().unwrap().as_path())
    );
    assert!(
        me.files.iter().any(|file| file == held.path()),
        "{:?}",
        me.files
    );
    assert!(me.maps.contains(&std::env::current_exe().unwrap()));
}
