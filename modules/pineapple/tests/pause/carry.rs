//! A stop a reader keeps here, given to each other machine running agents
//! so their guards hold it too, and what follows it: who is given what,
//! once, its end once it's kept here no longer, and the scripts that
//! write and drop it there.

use std::fs;
use std::path::Path;

use pineapple::pause::Plan;
use pineapple::pause::carry::{AGAIN_MS, Carried, Carrier, Gift, script, taken_away};
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

/// A plan naming no machine, so each is found only by its sessions.
fn unnamed() -> Plan {
    Plan {
        machines: Vec::new(),
        ..plan()
    }
}

fn carry(carrier: &mut Carrier, state: &Path, at: u64, here: &mut Machine) -> Vec<Carried> {
    let (gives, listing) = carrier.carry(&unnamed(), state, "here", at, here);
    assert!(listing.is_empty(), "every machine was listed: {listing:?}");
    gives
}

fn gave(name: &str, machine: &str, gift: Gift) -> Carried {
    Carried {
        name: name.to_owned(),
        machine: machine.to_owned(),
        gift,
        failed: None,
    }
}

/// How many times mycelium was asked to run something on the Mac.
fn runs_on_mac(here: &Machine) -> usize {
    asked_with(here, "mycelium run --need machine=mac -- sh -c").len()
}

fn on_mac(script: &str) -> String {
    format!("mycelium run --need machine=mac -- sh -c {script}")
}

#[test]
fn a_live_stop_heard_here_is_given_once_to_each_machine_running_a_session() {
    let state = scratch("carry-stop");
    let record = heard("slack:C1:1.000000");
    keep(&state, "slack-C1-1.000000", &record);
    let mut here = machine();
    let mut carrier = Carrier::default();

    let gives = carry(&mut carrier, &state, 0, &mut here);

    assert_eq!(
        gives,
        vec![gave("slack-C1-1.000000", "mac", Gift::Stop)],
        "only the Mac runs a session elsewhere, found by its sessions, and it's given the stop"
    );
    let asked = on_mac(&script("slack-C1-1.000000", &record, "here"));
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
fn a_stop_in_shadow_on_everything_carried_in_or_oddly_named_is_never_given() {
    let state = scratch("carry-shadow");
    let mut shadow = heard("slack:C1:1.000000");
    shadow["shadow"] = json!(true);
    keep(&state, "slack-C1-1.000000", &shadow);
    let mut all = heard("stop-all-5");
    all["reaches"] = json!(["everything"]);
    keep(&state, "stop-all-5", &all);
    let mut copy = heard("slack:C2:2.000000");
    copy["carried_from"] = json!("elsewhere");
    keep(&state, "slack-C2-2.000000", &copy);
    keep(&state, "odd name", &heard("slack:C3:3.000000"));
    let mut here = machine();

    let gives = carry(&mut Carrier::default(), &state, 0, &mut here);

    assert!(gives.is_empty(), "none is given: {gives:?}");
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

    let gives = carry(&mut carrier, &state, 10, &mut here);

    assert_eq!(
        gives,
        vec![gave("slack-C1-1.000000", "mac", Gift::Resume)],
        "the Mac is given the resume of the stop it held, and nothing of one it never did"
    );
    let resumed = json(&state.join("stops/slack-C1-1.000000.json"));
    let asked = on_mac(&script("slack-C1-1.000000", &resumed, "here"));
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

    keep(&state, "slack-C3-4.000000", &heard("slack:C3:4.000000"));
    let later = carry(&mut carrier, &state, 20, &mut here);
    assert_eq!(
        later,
        vec![gave("slack-C3-4.000000", "mac", Gift::Stop)],
        "once the Mac is listed for a stop standing, it's still given nothing of one it never held"
    );
}

#[test]
fn a_machine_that_cant_take_a_stop_is_tried_again_after_a_minute() {
    let state = scratch("carry-again");
    keep(&state, "slack-C1-1.000000", &heard("slack:C1:1.000000"));
    keep(&state, "slack-C2-2.000000", &heard("slack:C2:2.000000"));
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
            ..gave("slack-C1-1.000000", "mac", Gift::Stop)
        }],
        "the Mac's refusal is said, and it isn't asked again for the other stop"
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
        vec![
            gave("slack-C1-1.000000", "mac", Gift::Stop),
            gave("slack-C2-2.000000", "mac", Gift::Stop),
        ],
        "after a minute it's tried again, and takes both"
    );
}

#[test]
fn a_stop_goes_to_every_machine_lacking_it_at_once_so_one_out_of_reach_holds_none_up() {
    let state = scratch("carry-at-once");
    keep(&state, "slack-C1-1.000000", &heard("slack:C1:1.000000"));
    let named = Plan {
        machines: vec!["mac".to_owned(), "builder".to_owned(), "far".to_owned()],
        ..plan()
    };
    let mut here = Machine {
        unreached: Some("machine=builder"),
        ..machine()
    };

    let (gives, _) = Carrier::default().carry(&named, &state, "here", 0, &mut here);

    let failed: Vec<(&str, bool)> = gives
        .iter()
        .map(|one| (one.machine.as_str(), one.failed.is_some()))
        .collect();
    assert_eq!(
        failed,
        vec![("mac", false), ("builder", true), ("far", false)],
        "each is given it, and only the one out of reach fails: {gives:?}"
    );
    assert_eq!(
        here.batches,
        vec![3],
        "all three are asked in one batch, run together: {:?}",
        here.asked
    );
}

#[test]
fn a_machine_whose_give_may_have_landed_is_given_its_resume() {
    let state = scratch("carry-landed");
    keep(&state, "slack-C1-1.000000", &heard("slack:C1:1.000000"));
    let mut here = Machine {
        dropped: Some("machine=mac"),
        ..machine()
    };
    let mut carrier = Carrier::default();
    let dropped = carry(&mut carrier, &state, 0, &mut here);
    assert!(
        dropped[0].failed.is_some(),
        "the link dropped once the script began: {dropped:?}"
    );
    pineapple::resume::resume(&state.join("stops"), "all", 5, "joel").expect("resumed");
    here.dropped = None;

    let gives = carry(&mut carrier, &state, AGAIN_MS, &mut here);

    assert_eq!(
        gives,
        vec![gave("slack-C1-1.000000", "mac", Gift::Resume)],
        "the Mac may hold the stop, so it's given the resume"
    );
}

#[test]
fn a_stop_kept_here_no_longer_is_taken_away_from_each_machine_given_it() {
    let state = scratch("carry-gone");
    let name = "slack-C1-1.000000-unconfirmed-00ff";
    keep(&state, name, &heard("slack:C1:1.000000"));
    let mut here = machine();
    let mut carrier = Carrier::default();
    carry(&mut carrier, &state, 0, &mut here);
    fs::remove_file(state.join(format!("stops/{name}.json"))).expect("forgotten here");

    let gives = carry(&mut carrier, &state, 10, &mut here);

    assert_eq!(
        gives,
        vec![gave(name, "mac", Gift::Removal)],
        "the Mac is given its end"
    );
    assert!(
        here.asked.contains(&on_mac(&taken_away(name))),
        "the Mac runs the script dropping it: {:?}",
        here.asked
    );
    assert!(
        !state.join(format!("carried/{name}.json")).exists(),
        "and nothing is noted as held of it"
    );
    let asked = here.asked.len();
    let again = carry(&mut carrier, &state, 20, &mut here);
    assert!(again.is_empty(), "nothing more is given: {again:?}");
    assert_eq!(here.asked.len(), asked, "nor asked: {:?}", here.asked);
}

#[test]
fn a_changed_stop_goes_to_each_machine_holding_it_though_none_is_listed() {
    let state = scratch("carry-changed");
    let mut record = heard("slack:C1:1.000000");
    keep(&state, "slack-C1-1.000000", &record);
    let mut here = machine();
    let mut carrier = Carrier::default();
    carry(&mut carrier, &state, 0, &mut here);
    record["reaches"] = json!(["repo:owner/name"]);
    keep(&state, "slack-C1-1.000000", &record);
    here.refusing = Some("sessions");

    let (gives, listing) = carrier.carry(&unnamed(), &state, "here", 10, &mut here);

    assert!(
        !listing.is_empty(),
        "why the machines couldn't be listed is said"
    );
    assert_eq!(
        gives,
        vec![gave("slack-C1-1.000000", "mac", Gift::Stop)],
        "the Mac holds the stop, so it's given it widened"
    );
    assert!(
        here.asked
            .contains(&on_mac(&script("slack-C1-1.000000", &record, "here"))),
        "as it stands here: {:?}",
        here.asked
    );
}

#[test]
fn the_scripts_write_the_record_whole_under_its_name_there_and_drop_only_a_copy() {
    let home = scratch("carry-script");
    // Kept apart from the stop of the same message, as one read from an
    // answer that couldn't be read is, with a quote in its words.
    let record = heard("slack:C1:1.000000");
    let name = "slack-C1-1.000000-unconfirmed-00ff";
    let sh = |script: String| {
        let _held = FORKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let ran = std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .env("HOME", &home)
            .env_remove("XDG_STATE_HOME")
            .output()
            .expect("sh runs");
        assert_eq!(
            ran.status.code(),
            Some(0),
            "the script runs: {}",
            String::from_utf8_lossy(&ran.stderr)
        );
    };
    let stops = home.join(".local/state/pineapple/stops");
    let left = || -> Vec<String> {
        let mut files: Vec<String> = fs::read_dir(&stops)
            .expect("the stops folder")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        files.sort();
        files
    };

    sh(script(name, &record, "listener"));

    let mut there = record.clone();
    there["carried_from"] = json!("listener");
    assert_eq!(
        json(&stops.join(format!("{name}.json"))),
        there,
        "the record lands whole under its own name, quote and all, naming where it came from"
    );
    assert_eq!(
        left(),
        vec![format!("{name}.json")],
        "nothing half written is left, and nothing under its id"
    );

    let own = "slack-C2-2.000000";
    fs::write(stops.join(format!("{own}.json")), record.to_string()).expect("its own");
    sh(taken_away(name));
    sh(taken_away(own));
    assert_eq!(
        left(),
        vec![format!("{own}.json")],
        "the copy is dropped, and a stop of the machine's own is left"
    );
}
