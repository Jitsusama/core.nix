use crate::discover::file;
use disk_guard::opens::{Seen, Watch};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Everything the watch sees within `for`, once something has been seen.
fn seen(watch: &Watch, wanted: impl Fn(&[Seen]) -> bool, within: Duration) -> Vec<Seen> {
    let deadline = Instant::now() + within;
    let mut all = Vec::new();
    while Instant::now() < deadline && !wanted(&all) {
        all.extend(watch.wait(Duration::from_millis(100)).unwrap());
    }
    all
}

/// A watch, or none where the kernel refuses this account one, which the
/// guard then works without.
fn watch() -> Option<Watch> {
    match Watch::new() {
        Ok(watch) => Some(watch),
        Err(error) => {
            eprintln!("no fanotify here, so nothing to check: {error}");
            None
        }
    }
}

#[test]
fn another_process_opening_an_artifact_is_seen_by_its_path() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let rlib = deps.join("libalpha-1111111111111111.rlib");
    file(&rlib, "built");
    let Some(watch) = watch() else { return };
    watch.watch(&deps).unwrap();
    let status = Command::new("cat").arg(&rlib).output().unwrap().status;
    assert!(status.success());
    let opened = Seen::Opened(rlib.clone());
    let all = seen(&watch, |all| all.contains(&opened), Duration::from_secs(5));
    assert!(all.contains(&opened), "{all:?}");
}

#[test]
fn the_guards_own_opens_and_unwatched_ones_are_not_seen() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    let elsewhere = root.path().join("elsewhere/libbeta-2222222222222222.rlib");
    file(&deps.join("libalpha-1111111111111111.rlib"), "built");
    file(&elsewhere, "built");
    let Some(watch) = watch() else { return };
    watch.watch(&deps).unwrap();
    fs::read(deps.join("libalpha-1111111111111111.rlib")).unwrap();
    Command::new("cat").arg(&elsewhere).output().unwrap();
    // Then one it does see, so the wait has an end that proves it read.
    let marker = deps.join("libgamma-3333333333333333.rlib");
    file(&marker, "built");
    Command::new("cat").arg(&marker).output().unwrap();
    let opened = Seen::Opened(marker.clone());
    let all = seen(&watch, |all| all.contains(&opened), Duration::from_secs(5));
    assert_eq!(all, vec![opened]);
}

#[test]
fn opens_beyond_what_the_queue_holds_are_told_as_lost() {
    let root = tempfile::tempdir().unwrap();
    let deps = root.path().join("deps");
    fs::create_dir(&deps).unwrap();
    // An account's queue holds 16,384 events before it drops the rest, and
    // opens of one file merge into one event, so each open is of its own.
    let make = "seq 1 20000 | sed 's/^/lib/; s/$/.rlib/' | xargs touch";
    let run = |script: &str| {
        let ran = Command::new("sh")
            .args(["-c", script])
            .current_dir(&deps)
            .status();
        assert!(ran.unwrap().success(), "{script}");
    };
    run(make);
    let Some(watch) = watch() else { return };
    watch.watch(&deps).unwrap();
    run("find . -name '*.rlib' -exec cat {} +");
    let all = seen(
        &watch,
        |all| all.contains(&Seen::Lost),
        Duration::from_secs(5),
    );
    assert!(all.contains(&Seen::Lost), "{} seen, none lost", all.len());
}

#[test]
fn a_watch_on_what_is_not_a_directory_is_refused() {
    let Some(watch) = watch() else { return };
    assert!(watch.watch(Path::new("/nonexistent/deps")).is_err());
}
