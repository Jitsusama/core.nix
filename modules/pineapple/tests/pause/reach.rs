//! What a stop says of another machine that refuses it, that mycelium
//! never reached or whose link dropped part way, and which of them its
//! resume goes on trying.

use std::fs;
use std::path::PathBuf;

use pineapple::pause;
use serde_json::Value;

use super::{DROPPED, Machine, NEVER_RAN, asked_with, json, machine, plan, resumed, scratch, tend};

/// Stops `stop-all-1` with the Mac answering as `mac` sets up, resumes
/// it with the Mac answering the same, and answers what the stop and the
/// resume each said, with the fake machine and its state folder.
fn stopped_and_resumed(
    name: &str,
    mac: impl Fn(&mut Machine),
) -> (PathBuf, Machine, String, String) {
    stopped_then(name, &mac, |_| {})
}

/// As [`stopped_and_resumed`], with the Mac's answers changed as `then`
/// says before its resume.
fn stopped_then(
    name: &str,
    mac: impl Fn(&mut Machine),
    then: impl Fn(&mut Machine),
) -> (PathBuf, Machine, String, String) {
    let state = scratch(name);
    let mut here = machine();
    mac(&mut here);
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    let stopping = pause::said("stopping", &took, sessions);
    resumed(&state, 2);
    here.asked.clear();
    then(&mut here);
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    let resuming = pause::said("resuming", &took, None);
    (state, here, stopping, resuming)
}

#[test]
fn a_machine_that_refuses_is_named_with_why_and_resumed_until_it_can_be() {
    let (state, here, stopping, resuming) = stopped_and_resumed("refused", |mac| {
        mac.refusing = Some("machine=mac");
    });
    assert_eq!(
        stopping,
        "stopping 2 sessions on here; mac couldn't stop there: refused here, \
         so its agents may run on",
        "Joel is told which machine refused and why, and only its sessions are left out"
    );
    assert_eq!(
        resuming,
        "resuming every session on here; mac couldn't resume there: refused here, \
         tried again each minute",
        "a refusal may have come after the stop ran there, so its resume is kept: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_the_stop_never_ran_on_is_resumed_once_and_let_go() {
    let (state, mut here, stopping, resuming) = stopped_and_resumed("never-ran", |mac| {
        mac.unreached = Some("machine=mac");
    });
    assert_eq!(
        stopping,
        format!(
            "stopping 2 sessions on here; mac couldn't stop there: {NEVER_RAN}, \
             so its agents run on"
        ),
        "Joel is told the stop never reached the Mac"
    );
    assert_eq!(
        (resuming.as_str(), asked_with(&here, "machine=mac").len()),
        ("resuming every session on here", 1),
        "its resume is tried once, and says nothing of the Mac when that never runs either"
    );
    here.asked.clear();
    tend(&state, 2 + 60_000, &mut here);
    assert!(
        asked_with(&here, "machine=mac").is_empty() && pause::left(&state).is_empty(),
        "nor is it tried again each minute: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_whose_link_drops_once_the_stop_began_is_resumed_until_it_can_be() {
    let (state, _, stopping, resuming) = stopped_then(
        "dropped",
        |mac| mac.dropped = Some("machine=mac"),
        |mac| {
            mac.dropped = None;
            mac.unreached = Some("machine=mac");
        },
    );
    assert_eq!(
        stopping,
        format!(
            "stopping 2 sessions on here; mac couldn't stop there: {DROPPED}, \
             so its agents may run on"
        ),
        "the stop may have taken there, though mycelium ended as for one never run"
    );
    assert_eq!(
        resuming,
        format!(
            "resuming every session on here; mac couldn't resume there: {NEVER_RAN}, \
             tried again each minute"
        ),
        "so its resume is never given up on, even where mycelium can't run it"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn each_machine_is_kept_as_asked_before_it_is_asked() {
    let state = scratch("asked-first");
    let mut here = Machine {
        peek: Some(state.join("paused/stop-all-1.json")),
        ..machine()
    };
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let peeked: Value =
        serde_json::from_str(here.peeked.as_deref().unwrap_or_default()).unwrap_or_default();
    assert_eq!(
        peeked["machines"][0]["asked"],
        Value::Bool(true),
        "a stop cut off as it asks the Mac still resumes it: {peeked}"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_stop_that_cannot_note_whom_it_reached_still_stops_and_resumes_each() {
    let state = scratch("unnoted");
    let mut here = Machine {
        unreached: Some("machine=mac"),
        block: Some(state.join("paused/stop-all-1.new")),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here)
        .expect("a stop that froze everything isn't failed by its last note");
    assert!(
        took[0]
            .failed
            .iter()
            .any(|why| why.starts_with("couldn't note which machines it reached")),
        "Joel is told the note failed: {:?}",
        took[0].failed
    );
    let paused = json(&state.join("paused/stop-all-1.json"));
    assert_eq!(
        paused["machines"][0]["asked"],
        Value::Bool(true),
        "the note from before the asks stands, so the Mac is resumed until it can be: {paused}"
    );
    let _ = fs::remove_dir_all(&state);
}

/// Stops `stop-all-1` with the Mac taking it, resumes it, and answers the
/// Mac's resume as `answer` sets the Mac up to, with what the stop kept
/// of the Mac as `kept` says.
fn resumed_answering(
    name: &str,
    kept: impl Fn(&mut Value),
    answer: impl Fn(&mut Machine),
) -> (PathBuf, Machine, String) {
    let state = scratch(name);
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let path = state.join("paused/stop-all-1.json");
    let mut paused = json(&path);
    kept(&mut paused["machines"][0]);
    fs::write(&path, paused.to_string()).expect("rewritten");
    resumed(&state, 2);
    answer(&mut here);
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    let said = pause::said("resuming", &took, None);
    (state, here, said)
}

/// Clears what a stop kept saying it asked the machine, as stops before
/// it was kept did.
fn kept_before(mac: &mut Value) {
    mac.as_object_mut().expect("a machine kept").remove("asked");
}

#[test]
fn a_machine_that_took_the_stop_is_resumed_until_it_can_be() {
    let (state, mut here, said) =
        resumed_answering("took-it", |_| {}, |mac| mac.unreached = Some("machine=mac"));
    assert_eq!(
        said,
        format!(
            "resuming every session on here; mac couldn't resume there: {NEVER_RAN}, \
             tried again each minute"
        ),
        "a machine whose agents the stop holds is never given up on"
    );
    here.asked.clear();
    here.unreached = None;
    tend(&state, 2 + 60_000, &mut here);
    assert_eq!(
        asked_with(&here, "machine=mac").len(),
        1,
        "its resume is asked again after a minute: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_kept_before_stops_said_they_asked_is_let_go_once_it_never_ran() {
    let (state, mut here, said) = resumed_answering("kept-before", kept_before, |mac| {
        mac.unreached = Some("machine=mac");
    });
    assert_eq!(
        said, "resuming every session on here",
        "a machine kept by an older stop, whose resume mycelium never ran, isn't named"
    );
    here.asked.clear();
    tend(&state, 2 + 60_000, &mut here);
    assert!(
        asked_with(&here, "machine=mac").is_empty() && pause::left(&state).is_empty(),
        "nor tried again each minute: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_kept_before_stops_said_they_asked_is_kept_while_its_resume_may_have_run() {
    let (state, mut here, said) = resumed_answering("kept-before-refused", kept_before, |mac| {
        mac.refusing = Some("machine=mac");
    });
    assert_eq!(
        said,
        "resuming every session on here; mac couldn't resume there: refused here, \
         tried again each minute",
        "a resume that ran and failed there leaves the Mac to try again"
    );
    here.asked.clear();
    here.refusing = None;
    tend(&state, 2 + 60_000, &mut here);
    assert_eq!(
        asked_with(&here, "machine=mac").len(),
        1,
        "and it is tried after a minute: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}
