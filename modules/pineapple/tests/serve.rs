//! pineapple serve, as the launcher asks it: a live stop and a live
//! alert are served as rows titled in the vocabulary's words, a shadow
//! stop isn't and is only written down as what it would have told, and
//! a press from anything but Joel's shell, as this test is, is refused
//! and changes nothing; and a live stop's notice is closed once it's
//! resumed.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use daemon_client::{Daemon, until};
use serde_json::{Value, json};

/// What modes a stand-in program is written with, so it runs.
const RUNS: u32 = 0o755;

fn state(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-serve-{case}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let kept = dir.join("pineapple");
    fs::create_dir_all(kept.join("stops")).expect("a scratch state folder");
    let stop = |id: &str, file: &str, shadow: bool| {
        let record = json!({"id": id, "shadow": shadow, "reaches": ["pr:example/shop#12"],
            "fell_back": false, "heard_at": 10, "by_joel": false, "said_by": "U0OTHER001",
            "where": "https://example.slack.com/archives/C1/p1", "words": "pineapple override",
            "resumed": null});
        fs::write(kept.join("stops").join(file), record.to_string()).expect("a stop is written");
    };
    stop("slack:C1:1.0", "live.json", false);
    stop("slack:C2:1.0", "shadow.json", true);
    let joke = json!({"key": "slack:C3:2.0", "thread": "slack:C3:1.0", "verdict": "waved:joke",
        "heard_at": 4_102_444_800_000_u64, "by_joel": false, "shadow": false,
        "text": "pineapple override jk", "link": "https://example.slack.com/archives/C3/p2",
        "author": "U0OTHER001", "reaches": [], "would_reach": ["everything"],
        "would_fall_back": true});
    fs::write(kept.join("decisions.jsonl"), format!("{joke}\n")).expect("a decision is written");
    let vocabulary = json!({"things": [{"id": "for-you:source:pineapple", "said": {
        "someone": "someone", "stopped": "{who} stopped agents on {reach}",
        "joke": "{who} joked", "reach:pr": "pull request {repo}#{number}"}}]});
    fs::write(dir.join("words.json"), vocabulary.to_string()).expect("words are written");
    dir
}

fn keys(answer: &Value) -> Vec<&str> {
    answer["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str())
        .collect()
}

#[test]
fn live_rows_are_served_shadow_ones_written_down_and_a_stranger_refused() {
    let dir = state("rows");
    let socket = dir.join("pineapple.sock");
    let mut command = Command::new(env!("CARGO_BIN_EXE_pineapple"));
    command
        .args(["serve", "--socket"])
        .arg(&socket)
        .arg("--vocabulary")
        .arg(dir.join("words.json"))
        .args(["--every", "50"])
        .env("XDG_STATE_HOME", &dir);
    let daemon = Daemon::start(&mut command, socket, dir.clone());
    let mut client = daemon.client();
    let would_tell = dir.join("pineapple/would-tell.jsonl");
    until("the shadow stop was never written down", || {
        fs::read_to_string(&would_tell).is_ok_and(|text| text.contains("slack:C2:1.0"))
    });
    let listed = client.ask(&json!({"id": 1, "ask": "list", "kind": "pineapple"}));
    assert_eq!(
        keys(&listed),
        ["pineapple:slack:C1:1.0", "pineapple:joke:slack:C3:1.0"],
        "the live stop and the live joke, oldest first, and no shadow stop: {listed}"
    );
    assert_eq!(
        listed["records"][0]["title"], "Someone stopped agents on pull request example/shop#12",
        "titled in the vocabulary's words"
    );
    let refused = client.ask(&json!({"id": 2, "ask": "resume", "key": "pineapple:slack:C1:1.0"}));
    let code = refused["error"]["code"].as_str().unwrap_or_default();
    assert!(
        code == "not-joel" || code == "no-peer",
        "a press from a test isn't Joel's: {refused}"
    );
    let record: Value = serde_json::from_str(
        &fs::read_to_string(dir.join("pineapple/stops/live.json")).expect("kept"),
    )
    .expect("json");
    assert!(record["resumed"].is_null(), "and the stop still stands");
    drop(daemon);
}

/// A stand-in program that writes the arguments it was given to `heard`,
/// a line each time, and answers `says`.
fn stand_in(dir: &Path, name: &str, heard: &Path, says: &str) -> PathBuf {
    let program = dir.join(name);
    let script = format!(
        "#!/bin/sh\necho \"$*\" >> {}\necho {says}\n",
        heard.display()
    );
    fs::write(&program, script).expect("a stand-in is written");
    fs::set_permissions(&program, fs::Permissions::from_mode(RUNS)).expect("it runs");
    program
}

/// A stand-in busctl: it names the notices server as `server` holds it,
/// and writes any other call it's asked to `closed`, a line each.
fn busctl(dir: &Path, server: &Path, closed: &Path) -> PathBuf {
    let program = dir.join("busctl");
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in\n*GetNameOwner*) echo \"s \\\"$(cat {})\\\"\" ;;\n\
         *) echo \"$*\" >> {} ;;\nesac\n",
        server.display(),
        closed.display()
    );
    fs::write(&program, script).expect("a stand-in is written");
    fs::set_permissions(&program, fs::Permissions::from_mode(RUNS)).expect("it runs");
    program
}

/// The live stop's daemon, started knocking through stand-ins, once it has
/// knocked for the live stop: the daemon, and where its server's name and
/// what it closed are kept.
fn knocked(case: &str) -> (Daemon, PathBuf, PathBuf, PathBuf) {
    let dir = state(case);
    let socket = dir.join("pineapple.sock");
    let notified = dir.join("notified");
    let server = dir.join("server");
    let closed = dir.join("closed");
    fs::write(&server, ":1.42").expect("a server is named");
    let mut command = Command::new(env!("CARGO_BIN_EXE_pineapple"));
    command
        .args(["serve", "--socket"])
        .arg(&socket)
        .arg("--vocabulary")
        .arg(dir.join("words.json"))
        .arg("--notify-send")
        .arg(stand_in(&dir, "notify-send", &notified, "7"))
        .arg("--busctl")
        .arg(busctl(&dir, &server, &closed))
        .args(["--every", "50"])
        .env("XDG_STATE_HOME", &dir);
    let daemon = Daemon::start(&mut command, socket, dir.clone());
    until("the live stop never knocked", || {
        fs::read_to_string(&notified).is_ok_and(|text| text.contains("Someone stopped agents"))
    });
    until("the live stop's notice was never kept", || {
        fs::read_to_string(dir.join("pineapple/told.json")).is_ok_and(|told| told.contains(":1.42"))
    });
    assert!(
        !closed.exists(),
        "nothing is closed while its stop stands: {:?}",
        fs::read_to_string(&closed)
    );
    (daemon, dir, server, closed)
}

/// Resumes the live stop, as Joel's press writes it.
fn resume(dir: &Path) {
    let live = dir.join("pineapple/stops/live.json");
    let mut record: Value =
        serde_json::from_str(&fs::read_to_string(&live).expect("kept")).expect("json");
    record["resumed"] = json!({"at": 20, "by": "joel"});
    fs::write(&live, record.to_string()).expect("the stop is resumed");
}

#[test]
fn a_live_stops_notice_is_closed_once_it_is_resumed() {
    let (daemon, dir, _, closed) = knocked("notices");
    resume(&dir);
    until("the resumed stop's notice was never closed", || {
        fs::read_to_string(&closed).is_ok_and(|text| !text.is_empty())
    });
    let asked = fs::read_to_string(&closed).expect("what closed");
    assert_eq!(
        asked.lines().collect::<Vec<_>>(),
        [
            "--user call org.freedesktop.Notifications /org/freedesktop/Notifications \
          org.freedesktop.Notifications CloseNotification u 7"
        ],
        "the resumed stop's notice alone is closed, by the number it was given"
    );
    drop(daemon);
}

#[test]
fn a_notice_numbered_by_a_server_since_started_again_is_left_alone() {
    let (daemon, dir, server, closed) = knocked("restarted");
    // The shell started again, as a switch starts it, so number 7 may now
    // be another program's notice.
    fs::write(&server, ":1.99").expect("the server is named again");
    resume(&dir);
    until("the resumed stop was never let go", || {
        fs::read_to_string(dir.join("pineapple/told.json"))
            .is_ok_and(|told| !told.contains("pineapple:slack:C1:1.0@"))
    });
    assert!(
        !closed.exists(),
        "a number from the server before is never closed: {:?}",
        fs::read_to_string(&closed)
    );
    drop(daemon);
}
