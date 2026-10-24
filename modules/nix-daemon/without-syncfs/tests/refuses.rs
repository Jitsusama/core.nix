//! A program run through without-syncfs is refused `syncfs` and nothing
//! else. The program is this test binary itself, run again as `probe`.
//!
//! A Nix build on a machine whose daemon refuses `syncfs` already sees
//! `ENOSYS`, so the filter's own doing shows as one more seccomp filter
//! than the same probe run without it.

use std::env;
use std::ffi::c_int;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::process::{Command, Output};

unsafe extern "C" {
    fn syncfs(fd: c_int) -> c_int;
}

/// Set for the probe, which otherwise does nothing.
const PROBE: &str = "WITHOUT_SYNCFS_PROBE";

const ENOSYS: i32 = 38;

#[test]
fn probe() {
    if env::var_os(PROBE).is_none() {
        return;
    }
    let path = env::temp_dir().join(format!("without-syncfs-{}", std::process::id()));
    let file = File::create(&path).expect("a file to sync");
    // SAFETY: the descriptor is open for as long as `file` lives.
    let synced = unsafe { syncfs(file.as_raw_fd()) };
    let refused = std::io::Error::last_os_error().raw_os_error();
    match synced {
        0 => println!("syncfs ran"),
        _ => println!("syncfs answered {}", refused.unwrap_or_default()),
    }
    match file.sync_all() {
        Ok(()) => println!("fsync ran"),
        Err(error) => println!("fsync answered {error}"),
    }
    let status = fs::read_to_string("/proc/self/status").expect("its status");
    let filters = status
        .lines()
        .find_map(|line| line.strip_prefix("Seccomp_filters:"))
        .map_or("unknown", str::trim);
    println!("seccomp filters {filters}");
    drop(file);
    fs::remove_file(path).expect("the file removed");
}

/// What the probe says, run through without-syncfs or not.
fn probed(through: bool) -> String {
    let me = env::current_exe().expect("this test's path");
    let mut command = if through {
        let mut launcher = Command::new(env!("CARGO_BIN_EXE_without-syncfs"));
        launcher.arg(&me);
        launcher
    } else {
        Command::new(&me)
    };
    let said = command
        .args(["--exact", "probe", "--nocapture", "--test-threads=1"])
        .env(PROBE, "1")
        .output()
        .expect("the probe runs");
    assert!(said.status.success(), "{said:?}");
    String::from_utf8(said.stdout).expect("the probe says text")
}

fn filters(said: &str) -> u32 {
    said.lines()
        .find_map(|line| line.strip_prefix("seccomp filters "))
        .and_then(|count| count.parse().ok())
        .unwrap_or_else(|| panic!("no filter count in {said}"))
}

#[test]
fn refuses_syncfs_as_not_implemented_while_fsync_runs() {
    let said = probed(true);

    assert!(
        said.contains(&format!("syncfs answered {ENOSYS}")),
        "{said}"
    );
    assert!(said.contains("fsync ran"), "{said}");
    assert_eq!(filters(&said), filters(&probed(false)) + 1, "{said}");
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_without-syncfs"))
        .args(arguments)
        .output()
        .expect("without-syncfs runs")
}

#[test]
fn says_how_it_is_used_when_told_nothing_to_run() {
    let said = run(&[]);

    assert_eq!(said.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&said.stderr).starts_with("Usage:"));
}

#[test]
fn answers_as_a_shell_does_for_a_program_it_cannot_run() {
    let said = run(&["/nonexistent/program"]);

    assert_eq!(said.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&said.stderr).contains("/nonexistent/program"));
}
