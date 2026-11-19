//! a-call-is-on asking this machine's PipeWire alone, every account's, and
//! how it's asked: what counts as a call there, what it can't tell, and
//! how a question asked wrongly is refused.

use std::fs;
use std::time::Duration;

use super::{World, recording};

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
    let told = world.told(&["--hear"]);
    assert_eq!(
        told.status.code(),
        Some(64),
        "a mistyped flag was read as a folder: {told:?}"
    );
    assert!(
        String::from_utf8_lossy(&told.stderr).contains("--here"),
        "the refusal doesn't name the flag it knows: {told:?}"
    );
}

#[test]
fn asking_for_more_said_is_no_mistyped_flag() {
    let world = World::new();
    world.account("1000", "[]");
    for args in [
        vec!["--verbose", world.runtimes()],
        vec!["--here", "-v", world.runtimes()],
    ] {
        let told = world.told(&args);
        assert_eq!(
            told.status.code(),
            Some(1),
            "{args:?} was refused: {told:?}"
        );
    }
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
