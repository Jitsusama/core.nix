//! What a stop says of another machine that refuses it or that mycelium
//! never reached, and which of them its resume goes on trying.

use std::fs;
use std::path::PathBuf;

use pineapple::pause;
use serde_json::Value;

use super::{Machine, NEVER_RAN, asked_with, json, machine, plan, resumed, scratch, tend};

#[test]
fn a_machine_that_refuses_is_named_with_why() {
    let state = scratch("refused");
    let mut here = Machine {
        refusing: Some("machine=mac"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        "stopping 2 sessions on here; mac couldn't stop there: refused here, \
         so its agents may run on",
        "Joel is told which machine refused and why, and only its sessions are left out"
    );
    let _ = fs::remove_dir_all(&state);
}

#[test]
fn a_machine_the_stop_never_ran_on_is_named_once_and_never_resumed() {
    let state = scratch("never-ran");
    let mut here = Machine {
        unreached: Some("machine=mac"),
        ..machine()
    };
    let took = pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let sessions = pause::sessions(&plan(), &pause::recorded(&took), &mut here);
    assert_eq!(
        pause::said("stopping", &took, sessions),
        format!(
            "stopping 2 sessions on here; mac couldn't stop there: {NEVER_RAN}, \
             so its agents run on, and nothing there is resumed"
        ),
        "Joel is told the stop never reached the Mac, and that its resume won't be tried"
    );
    resumed(&state, 2);
    here.asked.clear();
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    assert_eq!(
        pause::said("resuming", &took, None),
        "resuming every session on here",
        "the resume says nothing of the Mac"
    );
    assert!(
        asked_with(&here, "machine=mac").is_empty() && pause::left(&state).is_empty(),
        "the Mac is never asked to resume, nor left to try again: {:?}",
        here.asked
    );
    let _ = fs::remove_dir_all(&state);
}

/// Stops `stop-all-1` with the Mac taking it, resumes it, and answers the
/// Mac's resume as one mycelium never ran, with what the stop kept of the
/// Mac as `kept` says.
fn resumed_out_of_reach(name: &str, kept: impl Fn(&mut Value)) -> (PathBuf, Machine, String) {
    let state = scratch(name);
    let mut here = machine();
    pause::stop(&plan(), &state, "stop-all-1", "here", 1, &mut here).expect("stops");
    let path = state.join("paused/stop-all-1.json");
    let mut paused = json(&path);
    kept(&mut paused["machines"][0]);
    fs::write(&path, paused.to_string()).expect("rewritten");
    resumed(&state, 2);
    here.unreached = Some("machine=mac");
    let took = pause::settle(&plan(), &state, "here", 2, &mut here).expect("settles");
    let said = pause::said("resuming", &took, None);
    (state, here, said)
}

#[test]
fn a_machine_that_took_the_stop_is_resumed_until_it_can_be() {
    let (state, mut here, said) = resumed_out_of_reach("took-it", |_| {});
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
    let (state, mut here, said) = resumed_out_of_reach("kept-before", |mac| {
        mac.as_object_mut().expect("a machine kept").remove("asked");
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
