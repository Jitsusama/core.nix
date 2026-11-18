//! a-call-is-on against a stand-in pw-dump, which prints the graph in
//! graph.json in the runtime folder it's given, or later.json once asked
//! before when there is one, and fails while that folder holds `fails`,
//! and against a stand-in mycelium node in an account's runtime folder,
//! which tells one other machine's record. A Mac reads no PipeWire, so
//! these are Linux's; src/lib.rs's tests say what counts as a call on a
//! Mac.
#![cfg(target_os = "linux")]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct World {
    _dir: tempfile::TempDir,
    runtimes: PathBuf,
    stand_ins: PathBuf,
}

impl World {
    fn new() -> World {
        let dir = tempfile::tempdir().expect("a folder");
        let runtimes = dir.path().join("user");
        let stand_ins = dir.path().join("bin");
        fs::create_dir_all(&runtimes).expect("runtimes");
        fs::create_dir_all(&stand_ins).expect("stand-ins");
        let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
        let dump = stand_ins.join("pw-dump");
        fs::write(
            &dump,
            format!(
                "#!{shell}\nPATH='{}'\nhere=$XDG_RUNTIME_DIR\n\
                 [ -e \"$here/fails\" ] && exit 1\n\
                 if [ -e \"$here/asked\" ] && [ -e \"$here/later.json\" ]; then \
                 cat \"$here/later.json\"; else cat \"$here/graph.json\"; fi\n\
                 touch \"$here/asked\"\n",
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .expect("pw-dump");
        fs::set_permissions(&dump, fs::Permissions::from_mode(0o755)).expect("runnable");
        World {
            _dir: dir,
            runtimes,
            stand_ins,
        }
    }

    /// An account's runtime folder with PipeWire running, showing `graph`.
    fn account(&self, uid: &str, graph: &str) -> PathBuf {
        let folder = self.runtimes.join(uid);
        fs::create_dir_all(&folder).expect("a runtime folder");
        fs::write(folder.join("pipewire-0"), "").expect("a socket");
        fs::write(folder.join("graph.json"), graph).expect("a graph");
        folder
    }

    /// What a-call-is-on exits with, and how long it took.
    fn asked(&self) -> (i32, Duration) {
        let started = Instant::now();
        let told = self.told(&[self.runtimes.to_str().expect("a folder as text")]);
        (told.status.code().expect("an exit code"), started.elapsed())
    }

    /// All a-call-is-on says when asked with `args`.
    fn told(&self, args: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            self.stand_ins.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(env!("CARGO_BIN_EXE_a-call-is-on"))
            .args(args)
            .env("PATH", path)
            .env_remove("JOURNAL_STREAM")
            .output()
            .expect("a-call-is-on ran")
    }
}

fn recording(id: u64, binary: &str) -> String {
    format!(
        r#"[{{"id": {id}, "type": "PipeWire:Interface:Node", "info": {{"state": "running",
        "props": {{"media.class": "Stream/Input/Audio", "application.process.binary": "{binary}"}}}}}}]"#
    )
}

#[test]
fn a_call_app_still_recording_two_seconds_on_is_a_call() {
    let world = World::new();
    world.account("1000", "[]");
    world.account("1001", &recording(40, "zoom"));
    let (code, took) = world.asked();
    assert_eq!(code, 0);
    assert!(took >= Duration::from_secs(2), "it waited: {took:?}");
}

#[test]
fn a_call_app_recording_through_pipewire_itself_is_a_call() {
    // Tuple records through PipeWire's own API, so its node names no
    // program and only its client says it's Tuple.
    let world = World::new();
    world.account(
        "1000",
        r#"[{"id": 59, "type": "PipeWire:Interface:Client",
             "info": {"props": {"application.process.binary": "tuple"}}},
            {"id": 60, "type": "PipeWire:Interface:Node", "info": {"state": "running",
             "props": {"media.class": "Stream/Input/Audio", "node.name": "TupleAudio",
                        "client.id": 59}}}]"#,
    );
    assert_eq!(world.asked().0, 0);
}

#[test]
fn a_moment_of_recording_is_no_call() {
    let world = World::new();
    let folder = world.account("1000", &recording(40, "chrome"));
    fs::write(folder.join("later.json"), "[]").expect("a later graph");
    assert_eq!(world.asked().0, 1);
}

#[test]
fn another_program_recording_later_is_no_call_yet() {
    let world = World::new();
    let folder = world.account("1000", &recording(40, "chrome"));
    fs::write(folder.join("later.json"), recording(41, "chrome")).expect("a later graph");
    assert_eq!(world.asked().0, 1);
}

#[test]
fn what_records_without_calling_is_no_call_and_waits_for_nothing() {
    let world = World::new();
    world.account("1000", &recording(2, "pw-record"));
    let (code, took) = world.asked();
    assert_eq!(code, 1);
    assert!(took < Duration::from_secs(2), "it waited: {took:?}");
}

#[test]
fn a_call_on_or_none_is_said_by_the_exit_alone() {
    let world = World::new();
    world.account("1000", "[]");
    let runtimes = world.runtimes.to_str().expect("a folder as text");
    let told = world.told(&[runtimes]);
    assert_eq!(told.status.code(), Some(1), "no call isn't 1: {told:?}");
    assert!(
        told.stdout.is_empty() && told.stderr.is_empty(),
        "a no said something: {told:?}"
    );
}

#[test]
fn a_question_asked_wrongly_is_refused_with_its_help() {
    let world = World::new();
    let told = world.told(&["/run/user", "/run/more"]);
    assert_eq!(
        told.status.code(),
        Some(64),
        "two folders weren't refused: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("a-call-is-on --help"),
        "the refusal names no help: {told:?}"
    );
}

#[test]
fn nobody_logged_in_is_no_call() {
    let world = World::new();
    fs::remove_dir(&world.runtimes).expect("removed");
    assert_eq!(world.asked().0, 1);
    let world = World::new();
    // An account logged in with no PipeWire, whose pw-dump would fail.
    let folder = world.runtimes.join("1000");
    fs::create_dir_all(&folder).expect("a runtime folder");
    fs::write(folder.join("fails"), "").expect("failing");
    assert_eq!(world.asked().0, 1);
}

#[test]
fn a_pipewire_that_wont_answer_cant_tell() {
    let world = World::new();
    let folder = world.account("1000", "[]");
    fs::write(folder.join("fails"), "").expect("failing");
    assert_eq!(world.asked().0, 69);
    let world = World::new();
    world.account("1000", "not a graph");
    let runtimes = world.runtimes.to_str().expect("a folder as text");
    let told = world.told(&[runtimes]);
    assert_eq!(told.status.code(), Some(69), "a bad graph told: {told:?}");
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("couldn't tell whether a call is on"),
        "it didn't say it couldn't tell: {told:?}"
    );
}

/// A stand-in mycelium node for the account `uid`, serving `realm`: it
/// answers each `Machines` request with a list holding `peer`, as a node
/// lists what it heard from another machine, for as long as the test runs.
fn node(world: &World, uid: &str, realm: &str, peer: &str) {
    let folder = world.runtimes.join(uid).join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    serve(&folder.join(format!("{realm}.sock")), peer);
}

/// A stand-in node's socket at `socket`, listing `peer`.
fn serve(socket: &std::path::Path, peer: &str) {
    let listener = UnixListener::bind(socket).expect("a socket");
    let answer = format!("{{\"Ok\":{{\"Machines\":[{peer}]}}}}\n");
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut asked = String::new();
            let _ = BufReader::new(&stream).read_line(&mut asked);
            if asked.trim() == "\"Machines\"" {
                let _ = (&stream).write_all(answer.as_bytes());
            }
        }
    });
}

/// A peer's record, measured `ago` seconds since, declaring the call
/// probe when `declares_call`, its probes saying `reached`.
fn peer(ago: u64, declares_call: bool, reached: &str) -> String {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs()
        - ago;
    let measured = Command::new("date")
        .args(["-u", "-d", &format!("@{at}"), "+%Y-%m-%dT%H:%M:%S.5Z"])
        .output()
        .expect("date ran");
    let measured = String::from_utf8(measured.stdout).expect("a time");
    let probes = if declares_call {
        r#"{"call":{"run":["a-call-is-on","--here"]}}"#
    } else {
        r#"{"home":{"run":["probe"]}}"#
    };
    format!(
        r#"{{"name":"skiff","measured_at":"{}","declared":{{"probes":{probes}}},"reached":{reached}}}"#,
        measured.trim()
    )
}

#[test]
fn a_call_on_another_machine_is_a_call_without_waiting_on_pipewire() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let (code, _) = world.asked();
    assert_eq!(code, 0, "a call on a peer wasn't a call");
    assert!(
        !world.runtimes.join("1000").join("asked").exists(),
        "it asked PipeWire though a peer had a call"
    );
}

#[test]
fn a_node_that_wont_answer_hides_no_call_another_heard() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    drop(UnixListener::bind(folder.join("a-dead.sock")).expect("a socket"));
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "a dead node hid a call: {told:?}"
    );
}

#[test]
fn a_socket_linked_from_elsewhere_is_never_asked() {
    let world = World::new();
    world.account("1000", "[]");
    let elsewhere = world
        .runtimes
        .parent()
        .expect("a parent")
        .join("elsewhere.sock");
    serve(&elsewhere, &peer(10, true, r#"{"call":true}"#));
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    std::os::unix::fs::symlink(&elsewhere, folder.join("work.sock")).expect("a link");
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a linked socket was asked: {told:?}"
    );
}

#[test]
fn asked_about_here_alone_a_call_elsewhere_is_none() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, r#"{"call":true}"#));
    let runtimes = world.runtimes.to_str().expect("a folder as text");
    let told = world.told(&["--here", runtimes]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "here alone heard a peer: {told:?}"
    );
    let world = World::new();
    world.account("1000", &recording(40, "zoom"));
    let told = world.told(&["--here", world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(0),
        "here alone missed its own: {told:?}"
    );
}

#[test]
fn a_peer_saying_no_call_and_a_call_gone_stale_are_no_call() {
    for (record, why) in [
        (peer(10, true, r#"{"call":false}"#), "a fresh no"),
        (
            peer(240, true, r#"{"call":true}"#),
            "a yes four minutes old",
        ),
        (peer(240, true, "{}"), "a stale record with no reading"),
    ] {
        let world = World::new();
        world.account("1000", "[]");
        node(&world, "1000", "personal", &record);
        let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
        assert_eq!(
            told.status.code(),
            Some(1),
            "{why} wasn't no call: {told:?}"
        );
        assert!(told.stderr.is_empty(), "{why} said something: {told:?}");
    }
}

#[test]
fn a_peer_whose_probe_said_nothing_is_named_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, true, "{}"));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "no reading was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("skiff's call probe said nothing lately"),
        "it didn't name the peer: {told:?}"
    );
}

#[test]
fn a_peer_declaring_no_call_probe_is_named_unseen_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    node(&world, "1000", "work", &peer(10, false, r#"{"home":true}"#));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "an unseen peer was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("skiff declares no call probe"),
        "it didn't name the unseen peer: {told:?}"
    );
}

#[test]
fn a_node_that_wont_answer_is_named_and_no_wait() {
    let world = World::new();
    world.account("1000", "[]");
    let folder = world.runtimes.join("1000").join("mycelium");
    fs::create_dir_all(&folder).expect("a node's folder");
    // A socket left by a node that has stopped: nothing listens on it.
    drop(UnixListener::bind(folder.join("personal.sock")).expect("a socket"));
    let told = world.told(&[world.runtimes.to_str().expect("a folder as text")]);
    assert_eq!(
        told.status.code(),
        Some(1),
        "a silent node was a wait: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("so its peers' calls go unheard"),
        "it didn't name the silent node: {told:?}"
    );
}
