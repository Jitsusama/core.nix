//! a-call-is-on against a stand-in pw-dump, which prints the graph in
//! graph.json in the runtime folder it's given, or later.json once asked
//! before when there is one, and fails while that folder holds `fails`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

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
        let path = format!(
            "{}:{}",
            self.stand_ins.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let started = Instant::now();
        let status = Command::new(env!("CARGO_BIN_EXE_a-call-is-on"))
            .arg(&self.runtimes)
            .env("PATH", path)
            .status()
            .expect("a-call-is-on ran");
        (status.code().expect("an exit code"), started.elapsed())
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
    assert_eq!(world.asked().0, 2);
    let world = World::new();
    world.account("1000", "not a graph");
    assert_eq!(world.asked().0, 2);
}
