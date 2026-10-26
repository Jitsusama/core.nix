//! Whether a version is in use, asked of the machine's real processes:
//! one standing in a folder, one holding a file in it open, and none.

use pi_packages::inuse::in_use;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// A process that is stopped when the test ends, passed or failed.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A shell that runs `script` in `dir` and then waits to be stopped.
fn shell(dir: &Path, script: &str) -> Running {
    Running(
        Command::new("sh")
            .args(["-c", &format!("{script} exec sleep 600")])
            .current_dir(dir)
            .spawn()
            .expect("a shell starts"),
    )
}

/// Whether `dir` reads as in use within a few seconds, the time a shell
/// takes to get where it's going.
fn soon_in_use(dir: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if in_use(dir) {
            return true;
        }
        if Instant::now() > deadline {
            return false;
        }
        sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_folder_a_process_stands_in_is_in_use() {
    let version = tempfile::tempdir().expect("a folder");
    let _standing = shell(version.path(), "");
    assert!(soon_in_use(version.path()), "a process stands in it");
}

#[test]
fn a_folder_a_process_holds_a_file_open_in_is_in_use() {
    let version = tempfile::tempdir().expect("a folder");
    let file = version.path().join("binding.node");
    fs::write(&file, "").expect("a file");
    let elsewhere = tempfile::tempdir().expect("a folder");
    let _holding = shell(elsewhere.path(), &format!("exec 3<'{}';", file.display()));
    assert!(
        soon_in_use(version.path()),
        "a process holds a file in it open"
    );
}

#[test]
fn a_folder_nothing_uses_is_not_in_use() {
    let version = tempfile::tempdir().expect("a folder");
    let elsewhere = tempfile::tempdir().expect("a folder");
    let _elsewhere = shell(elsewhere.path(), "");
    sleep(Duration::from_millis(200));
    assert!(!in_use(version.path()), "nothing uses it");
}
