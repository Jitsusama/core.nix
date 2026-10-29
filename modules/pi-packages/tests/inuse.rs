//! Whether a version is in use, asked of the machine's real processes:
//! one standing in a folder, one holding a file in it open, and none;
//! and what counts as use once a process said what it loaded.

use pi_packages::inuse::{self, Seen, in_use};
use pi_packages::loaded::Said;
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
        if in_use(dir, &Said::default()) {
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
    assert!(!in_use(version.path(), &Said::default()), "nothing uses it");
}

const MAPS: Seen = Seen {
    stands: false,
    holds: false,
    maps: true,
};

#[test]
fn a_map_is_use_only_from_a_process_that_said_nothing() {
    assert!(inuse::uses(MAPS, None), "it said nothing, and maps it");
    assert!(
        !inuse::uses(MAPS, Some(false)),
        "it said it runs another version, and only maps this one's addon"
    );
    assert!(
        inuse::uses(Seen::default(), Some(true)),
        "it said it runs it"
    );
}

#[test]
fn standing_in_a_version_or_holding_its_file_is_use_whatever_was_said() {
    let stands = Seen {
        stands: true,
        ..Seen::default()
    };
    let holds = Seen {
        holds: true,
        ..Seen::default()
    };
    assert!(inuse::uses(stands, Some(false)));
    assert!(inuse::uses(holds, Some(false)));
    assert!(!inuse::uses(Seen::default(), None));
}
