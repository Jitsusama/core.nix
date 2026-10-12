use disk_guard::linux::processes;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};

fn text(root: &Path, path: &str, value: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, value).unwrap();
}

fn link(root: &Path, path: &str, to: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(to, path).unwrap();
}

/// `/proc/<pid>/stat` with its name, flags and start in clock ticks.
fn stat(pid: u32, name: &str, flags: u64, start: u64) -> String {
    let mut fields = vec!["0".to_string(); 49];
    fields[0] = "S".into();
    fields[6] = flags.to_string();
    fields[19] = start.to_string();
    format!("{pid} ({name}) {}\n", fields.join(" "))
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
        "HOME=/home/u\0CARGO_TARGET_DIR=/home/u/t\0SECRET=x\0",
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
    let root = tempfile::tempdir().unwrap();
    fake_proc(root.path());
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
        vec![(
            OsString::from("CARGO_TARGET_DIR"),
            OsString::from("/home/u/t")
        )]
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
    let root = tempfile::tempdir().unwrap();
    fake_proc(root.path());
    let all = processes(root.path()).unwrap();
    let sshd = all.iter().find(|p| p.pid == 200).expect("sshd wasn't read");
    assert_eq!(sshd.uid, 0);
    assert!(!sshd.readable);
    assert_eq!(sshd.cgroup.as_deref(), Some("/system.slice/sshd.service"));
}

#[test]
fn kernel_threads_and_other_entries_are_left_out() {
    let root = tempfile::tempdir().unwrap();
    fake_proc(root.path());
    let mut pids: Vec<u32> = processes(root.path())
        .unwrap()
        .iter()
        .map(|p| p.pid)
        .collect();
    pids.sort();
    assert_eq!(pids, [100, 200]);
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
