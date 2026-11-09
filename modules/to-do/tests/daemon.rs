//! The daemon itself, over a folder of to-dos and a stand-in machine: the
//! verdicts it writes as checks pass, files come, a boot is newer than a
//! to-do, and a to-do leaves its file; a check that hangs holding up no
//! other; and a done to-do staying done through a failing check and a
//! restart.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

struct Running {
    child: Child,
    verdicts: PathBuf,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts the daemon over `dir`, whose `root/proc/stat` says the boot.
fn start(dir: &Path) -> Running {
    let root = dir.join("root");
    fs::create_dir_all(root.join("proc")).unwrap();
    fs::write(root.join("proc/stat"), "cpu 1\nbtime 1794200000\n").unwrap();
    fs::create_dir_all(dir.join("waiting")).unwrap();
    let verdicts = dir.join("verdicts.json");
    let child = Command::new(env!("CARGO_BIN_EXE_to-do"))
        .arg(dir.join("waiting"))
        .arg(&verdicts)
        .arg(dir.join("signins"))
        .arg(dir.join("core.git"))
        .args(["--follower", "none", "--root"])
        .arg(&root)
        .spawn()
        .unwrap();
    Running { child, verdicts }
}

fn to_dos(dir: &Path, quest: &str, items: &str) {
    let file = dir.join("waiting").join(format!("{quest}.json"));
    let writing = file.with_extension("writing");
    fs::write(
        &writing,
        format!(r#"{{"v":2,"quest":"{quest}","items":[{items}]}}"#),
    )
    .unwrap();
    fs::rename(writing, file).unwrap();
}

/// The verdicts once `wanted` holds of them, within five seconds.
fn until(running: &Running, wanted: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let read = fs::read_to_string(&running.verdicts)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        if let Some(read) = read.as_ref().filter(|r| wanted(r)) {
            return read.clone();
        }
        assert!(Instant::now() < deadline, "never so: {read:?}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn state<'a>(verdicts: &'a Value, key: &str) -> Option<&'a str> {
    verdicts["items"][key]["state"].as_str()
}

#[test]
fn a_passing_check_is_done_and_a_failing_one_waits_with_its_figures() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-1",
        r#"{"id":"yes","title":"Y","doneWhen":{"check":"true"}},
           {"id":"no","title":"N","doneWhen":{"check":"echo not yet; exit 1"}}"#,
    );
    let found = until(&running, |v| {
        state(v, "QEST-1:yes").is_some() && state(v, "QEST-1:no").is_some()
    });
    assert_eq!(state(&found, "QEST-1:yes"), Some("done"));
    assert_eq!(state(&found, "QEST-1:no"), Some("waiting"));
    let how = &found["items"]["QEST-1:no"]["how"];
    assert_eq!(how["by"], "check");
    assert_eq!(how["exit"], 1);
    assert_eq!(how["said"], "not yet");
    assert_eq!(how["ran"], 1);
}

#[test]
fn a_file_waited_on_is_done_as_it_comes() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    let wanted = dir.path().join("deep/er/here");
    to_dos(
        dir.path(),
        "QEST-2",
        &format!(
            r#"{{"id":"f","title":"F","doneWhen":{{"signal":"file:{}"}}}}"#,
            wanted.display()
        ),
    );
    until(&running, |v| state(v, "QEST-2:f") == Some("waiting"));
    fs::create_dir_all(wanted.parent().unwrap()).unwrap();
    fs::write(&wanted, "").unwrap();
    until(&running, |v| state(v, "QEST-2:f") == Some("done"));
}

#[test]
fn a_boot_since_it_was_raised_makes_it_ready() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-3",
        r#"{"id":"old","title":"O","raisedAt":"2026-11-09T04:00:00Z",
            "doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}},
           {"id":"new","title":"N","raisedAt":"2026-11-09T05:00:00Z",
            "doneWhen":{"by":"person"},"readyWhen":{"signal":"boot"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-3:new").is_some());
    assert_eq!(state(&found, "QEST-3:old"), Some("ready"));
    assert_eq!(state(&found, "QEST-3:new"), Some("waiting"));
    assert_eq!(
        found["items"]["QEST-3:new"]["how"]["boot_at"],
        "2026-11-09T04:53:20Z"
    );
}

#[test]
fn only_joels_word_has_no_verdict_and_a_to_do_gone_takes_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-4",
        r#"{"id":"his","title":"H","doneWhen":{"by":"person"}},
           {"id":"c","title":"C","doneWhen":{"check":"false"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-4:c").is_some());
    assert_eq!(state(&found, "QEST-4:his"), None);
    to_dos(
        dir.path(),
        "QEST-4",
        r#"{"id":"his","title":"H","doneWhen":{"by":"person"}}"#,
    );
    until(&running, |v| {
        v["items"]
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    });
}

#[test]
fn a_check_that_hangs_holds_up_no_other() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path());
    // The hanging check stands for one that never answers, and is stopped
    // as the daemon is.
    to_dos(
        dir.path(),
        "QEST-5",
        r#"{"id":"hangs","title":"H","doneWhen":{"check":"sleep 20"}},
           {"id":"quick","title":"Q","doneWhen":{"check":"true"}}"#,
    );
    let found = until(&running, |v| state(v, "QEST-5:quick").is_some());
    assert_eq!(state(&found, "QEST-5:quick"), Some("done"));
    assert_eq!(state(&found, "QEST-5:hangs"), None);
}

#[test]
fn a_done_to_do_stays_done_though_its_check_fails_after_and_across_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let once = dir.path().join("once");
    fs::write(&once, "").unwrap();
    // It passes the first time it runs and fails every time after.
    let done = format!(
        r#"{{"id":"once","title":"O","doneWhen":{{"check":"rm {}"}}}}"#,
        once.display()
    );
    // Each later to-do's check takes a second, so a check run again
    // beside it would be heard first.
    let later =
        |id: &str| format!(r#"{{"id":"{id}","title":"L","doneWhen":{{"check":"sleep 1"}}}}"#);
    let running = start(dir.path());
    to_dos(dir.path(), "QEST-6", &done);
    until(&running, |v| state(v, "QEST-6:once") == Some("done"));
    // A sign-in is a signal, which would run every check again.
    fs::create_dir_all(dir.path().join("signins")).unwrap();
    fs::write(dir.path().join("signins/stand-in.json"), "{}").unwrap();
    to_dos(dir.path(), "QEST-6", &format!("{done},{}", later("after")));
    let found = until(&running, |v| state(v, "QEST-6:after").is_some());
    assert_eq!(state(&found, "QEST-6:once"), Some("done"));
    drop(running);
    let running = start(dir.path());
    to_dos(
        dir.path(),
        "QEST-6",
        &format!("{done},{},{}", later("after"), later("restarted")),
    );
    let found = until(&running, |v| state(v, "QEST-6:restarted").is_some());
    assert_eq!(state(&found, "QEST-6:once"), Some("done"));
}
