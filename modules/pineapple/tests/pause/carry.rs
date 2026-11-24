//! A stop a reader keeps here, given to each other machine running agents
//! so their guards hold it too, and its resume after it: who is given
//! what, once, and the script that writes it there.

use std::fs;
use std::path::Path;

use pineapple::pause::carry::{AGAIN_MS, Carried, Carrier, script};
use serde_json::{Value, json};

use super::{FORKS, Machine, asked_with, json, machine, plan, scratch};

/// A live stop from Slack, standing, as a reader keeps it.
fn heard(id: &str) -> Value {
    json!({
        "id": id,
        "shadow": false,
        "said_by": "U0SOMEONE1",
        "where": "https://example.slack.com/archives/C1/p1",
        "words": "<@U0JOEL0001> pineapple override on this PR, it's queued",
        "said_at": "1.000000",
        "heard_at": 2000,
        "by_joel": false,
        "reaches": ["pr:owner/name#12"],
        "fell_back": false,
        "resumed": null,
    })
}

fn keep(state: &Path, name: &str, record: &Value) {
    let stops = state.join("stops");
    fs::create_dir_all(&stops).expect("a stops folder");
    fs::write(stops.join(format!("{name}.json")), format!("{record:#}\n")).expect("kept");
}

fn carry(carrier: &mut Carrier, state: &Path, at: u64, here: &mut Machine) -> Vec<Carried> {
    let (carried, listing) = carrier.carry(&plan(), state, "here", at, here);
    assert!(listing.is_empty(), "every machine was listed: {listing:?}");
    carried
}

fn gave(name: &str, machine: &str, resumed: bool) -> Carried {
    Carried {
        name: name.to_owned(),
        machine: machine.to_owned(),
        resumed,
        failed: None,
    }
}

/// How many times mycelium was asked to run something on the Mac.
fn runs_on_mac(here: &Machine) -> usize {
    asked_with(here, "mycelium run --need machine=mac -- sh -c").len()
}

#[test]
fn a_live_stop_heard_here_is_given_once_to_each_machine_running_a_session() {
    let state = scratch("carry-stop");
    let record = heard("slack:C1:1.000000");
    keep(&state, "slack-C1-1.000000", &record);
    let mut here = machine();
    let mut carrier = Carrier::default();

    let carried = carry(&mut carrier, &state, 0, &mut here);

    assert_eq!(
        carried,
        vec![gave("slack-C1-1.000000", "mac", false)],
        "only the Mac runs a session elsewhere, and it's given the stop"
    );
    let asked = format!(
        "mycelium run --need machine=mac -- sh -c {}",
        script("slack-C1-1.000000", &record, "here")
    );
    assert!(
        here.asked.contains(&asked),
        "the Mac runs the script writing the record: {:?}",
        here.asked
    );
    let noted = json(&state.join("carried/slack-C1-1.000000.json"));
    assert_eq!(
        noted["machines"][0]["holds"], record,
        "what the Mac holds is noted: {noted}"
    );

    let again = carry(&mut carrier, &state, 1, &mut here);
    assert!(again.is_empty(), "nothing is given twice: {again:?}");
    assert_eq!(
        runs_on_mac(&here),
        1,
        "the Mac is asked once: {:?}",
        here.asked
    );
    let fresh = carry(&mut Carrier::default(), &state, 2, &mut here);
    assert!(
        fresh.is_empty(),
        "nor once serve starts again, from what was noted: {fresh:?}"
    );
}

#[test]
fn a_stop_in_shadow_or_on_everything_is_never_given() {
    let state = scratch("carry-shadow");
    let mut shadow = heard("slack:C1:1.000000");
    shadow["shadow"] = json!(true);
    keep(&state, "slack-C1-1.000000", &shadow);
    let mut all = heard("stop-all-5");
    all["reaches"] = json!(["everything"]);
    keep(&state, "stop-all-5", &all);
    let mut here = machine();

    let carried = carry(&mut Carrier::default(), &state, 0, &mut here);

    assert!(carried.is_empty(), "neither is given: {carried:?}");
    assert!(
        here.asked.is_empty(),
        "and nothing is asked, not even who runs sessions: {:?}",
        here.asked
    );
}

#[test]
fn a_resume_is_given_only_to_the_machines_that_held_the_stop() {
    let state = scratch("carry-resume");
    keep(&state, "slack-C1-1.000000", &heard("slack:C1:1.000000"));
    let mut here = machine();
    let mut carrier = Carrier::default();
    carry(&mut carrier, &state, 0, &mut here);
    let mut never = heard("slack:C2:3.000000");
    never["resumed"] = json!({"at": 1, "by": "joel"});
    keep(&state, "slack-C2-3.000000", &never);
    pineapple::resume::resume(&state.join("stops"), "all", 5, "joel").expect("resumed");
    let listed = asked_with(&here, "sessions").len();

    let carried = carry(&mut carrier, &state, 10, &mut here);

    assert_eq!(
        carried,
        vec![gave("slack-C1-1.000000", "mac", true)],
        "the Mac is given the resume of the stop it held, and nothing of one it never did"
    );
    let resumed: Value = serde_json::from_str(
        &fs::read_to_string(state.join("stops/slack-C1-1.000000.json")).expect("kept"),
    )
    .expect("json");
    let asked = format!(
        "mycelium run --need machine=mac -- sh -c {}",
        script("slack-C1-1.000000", &resumed, "here")
    );
    assert!(
        here.asked.contains(&asked),
        "the Mac is given the record as resumed here: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "sessions").len(),
        listed,
        "with nothing standing, nobody is asked who runs sessions"
    );
}

#[test]
fn a_machine_that_cant_take_a_stop_is_tried_again_after_a_minute() {
    let state = scratch("carry-again");
    keep(&state, "slack-C1-1.000000", &heard("slack:C1:1.000000"));
    let mut here = Machine {
        refusing: Some("machine=mac"),
        ..machine()
    };
    let mut carrier = Carrier::default();

    let refused = carry(&mut carrier, &state, 0, &mut here);
    assert_eq!(
        refused,
        vec![Carried {
            failed: Some("refused here".to_owned()),
            ..gave("slack-C1-1.000000", "mac", false)
        }],
        "the Mac's refusal is said"
    );
    assert!(
        !state.join("carried/slack-C1-1.000000.json").exists(),
        "nothing is noted as held"
    );
    let early = carry(&mut carrier, &state, AGAIN_MS - 1, &mut here);
    assert!(
        early.is_empty(),
        "it isn't tried within the minute: {early:?}"
    );
    assert_eq!(runs_on_mac(&here), 1, "nor asked: {:?}", here.asked);

    here.refusing = None;
    let later = carry(&mut carrier, &state, AGAIN_MS, &mut here);
    assert_eq!(
        later,
        vec![gave("slack-C1-1.000000", "mac", false)],
        "after a minute it's tried again, and takes it"
    );
}

#[test]
fn the_script_writes_the_record_whole_under_its_name_there() {
    let home = scratch("carry-script");
    // Kept apart from the stop of the same message, as one read from an
    // answer that couldn't be read is, with a quote in its words.
    let record = heard("slack:C1:1.000000");
    let name = "slack-C1-1.000000-unconfirmed-00ff";

    let ran = {
        let _held = FORKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::process::Command::new("sh")
            .arg("-c")
            .arg(script(name, &record, "listener"))
            .env("HOME", &home)
            .env_remove("XDG_STATE_HOME")
            .output()
            .expect("sh runs")
    };

    assert_eq!(
        ran.status.code(),
        Some(0),
        "the script runs: {}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let stops = home.join(".local/state/pineapple/stops");
    let mut there = record.clone();
    there["carried_from"] = json!("listener");
    assert_eq!(
        json(&stops.join(format!("{name}.json"))),
        there,
        "the record lands whole under its own name, quote and all, naming where it came from"
    );
    let left: Vec<_> = fs::read_dir(&stops)
        .expect("the stops folder")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        vec![format!("{name}.json")],
        "nothing half written is left, and nothing under its id"
    );
}
