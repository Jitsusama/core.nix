//! A stop a reader keeps here, given to each other machine running agents
//! so their guards hold it too, and what follows it: who is given what,
//! once, its end once it's kept here no longer, a machine gone from the
//! mesh parked till it's listed again, and the scripts that write and
//! drop it there.

use std::fs;
use std::ops::RangeInclusive;
use std::path::Path;

use pineapple::pause::Plan;
use pineapple::pause::carry::{
    AGAIN_MS, Away, Carried, Carrier, GONE_MS, Gift, script, taken_away,
};
use serde_json::{Value, json};

use super::{FORKS, Machine, asked_with, json, machine, plan, scratch};

/// A live stop from Slack, standing, as a reader keeps it.
pub(super) fn heard(id: &str) -> Value {
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

pub(super) fn keep(state: &Path, name: &str, record: &Value) {
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
        away: None,
    }
}

/// How many times mycelium was asked to run something on the Mac.
fn runs_on_mac(here: &Machine) -> usize {
    asked_with(here, "mycelium run --need machine=mac -- sh -c").len()
}

fn on_mac(script: &str) -> String {
    format!("mycelium run --need machine=mac -- sh -c {script}")
}

/// The stop the gone tests carry.
const NAME: &str = "slack-C1-1.000000";

/// A carrier that gave the Mac the stop kept under [`NAME`] at 0.
fn given_mac(state: &Path, here: &mut Machine) -> Carrier {
    keep(state, NAME, &heard("slack:C1:1.000000"));
    let mut carrier = Carrier::default();
    let gives = carry(&mut carrier, state, 0, here);
    assert_eq!(
        gives,
        vec![gave(NAME, "mac", Gift::Stop)],
        "the Mac takes the stop"
    );
    carrier
}

/// Carries once a minute through `minutes`, mycelium listing a session on
/// the Mac only in those `listed` holds for; answers what each gave.
fn each_minute(
    carrier: &mut Carrier,
    state: &Path,
    here: &mut Machine,
    minutes: RangeInclusive<u64>,
    listed: impl Fn(u64) -> bool,
) -> Vec<(u64, Vec<Carried>)> {
    minutes
        .map(|minute| {
            here.unlisted = (!listed(minute)).then_some("mac");
            (minute, carry(carrier, state, minute * AGAIN_MS, here))
        })
        .collect()
}

/// The minutes in `rounds` whose gives said the Mac was `away` so.
fn when(rounds: &[(u64, Vec<Carried>)], away: Away) -> Vec<u64> {
    rounds
        .iter()
        .filter(|(_, gives)| gives.iter().any(|one| one.away == Some(away)))
        .map(|(minute, _)| *minute)
        .collect()
}

/// What the Mac is noted as given of [`NAME`].
fn noted_on_mac(state: &Path) -> Value {
    json(&state.join(format!("carried/{NAME}.json")))["machines"][0].clone()
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
    // The last is out of reach, so an answer paired with the wrong machine
    // fails another.
    let mut here = Machine {
        unreached: Some("machine=far"),
        ..machine()
    };

    let (gives, _) = Carrier::default().carry(&named, &state, "here", 0, &mut here);

    let failed: Vec<(&str, bool)> = gives
        .iter()
        .map(|one| (one.machine.as_str(), one.failed.is_some()))
        .collect();
    assert_eq!(
        failed,
        vec![("mac", false), ("builder", false), ("far", true)],
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
fn a_stop_is_taken_away_from_every_machine_at_once_and_kept_noted_where_it_failed() {
    let state = scratch("carry-gone-at-once");
    let name = "slack-C1-1.000000";
    keep(&state, name, &heard("slack:C1:1.000000"));
    let named = Plan {
        machines: vec!["mac".to_owned(), "builder".to_owned(), "far".to_owned()],
        ..plan()
    };
    let mut here = machine();
    let mut carrier = Carrier::default();
    carrier.carry(&named, &state, "here", 0, &mut here);
    fs::remove_file(state.join(format!("stops/{name}.json"))).expect("forgotten here");
    // The last one fails, so answers paired in reverse would show.
    here.unreached = Some("machine=far");

    let (gives, _) = carrier.carry(&named, &state, "here", 10, &mut here);

    let failed: Vec<(&str, bool)> = gives
        .iter()
        .map(|one| (one.machine.as_str(), one.failed.is_some()))
        .collect();
    assert_eq!(
        failed,
        vec![("mac", false), ("builder", false), ("far", true)],
        "each is given its end, and only the one out of reach fails: {gives:?}"
    );
    assert_eq!(
        here.batches.last(),
        Some(&3),
        "in one batch: {:?}",
        here.batches
    );
    let noted = fs::read_to_string(state.join(format!("carried/{name}.json"))).unwrap_or_default();
    assert!(
        noted.contains("\"far\"") && !noted.contains("\"mac\"") && !noted.contains("\"builder\""),
        "only the one still holding it is noted: {noted}"
    );
    let asked = here.asked.len();
    let (early, _) = carrier.carry(&named, &state, "here", 20, &mut here);
    assert!(
        early.is_empty(),
        "none is tried within the minute: {early:?}"
    );
    assert_eq!(here.asked.len(), asked, "nor asked: {:?}", here.asked);
    here.unreached = None;
    let (later, _) = carrier.carry(&named, &state, "here", 10 + AGAIN_MS, &mut here);
    assert_eq!(
        later,
        vec![gave(name, "far", Gift::Removal)],
        "after a minute only the one still holding it is asked again"
    );
    assert!(
        !state.join(format!("carried/{name}.json")).exists(),
        "and nothing is noted as held of it"
    );
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
fn a_machine_gone_an_hour_is_parked_and_given_its_resume_once_listed_again() {
    let state = scratch("carry-parked");
    let mut here = machine();
    let mut carrier = given_mac(&state, &mut here);
    pineapple::resume::resume(&state.join("stops"), "all", 5, "joel").expect("resumed");
    here.unreached = Some("machine=mac");
    // Found gone first in minute 2, once its give failed and none listed it.
    let parked_at = 2 + GONE_MS / AGAIN_MS;

    let rounds = each_minute(&mut carrier, &state, &mut here, 1..=parked_at, |_| false);

    assert_eq!(
        when(&rounds, Away::Parked),
        vec![parked_at],
        "the Mac is parked once, an hour after it was first found gone"
    );
    assert_eq!(
        runs_on_mac(&here) as u64,
        1 + parked_at,
        "and is tried each minute till then: {rounds:?}"
    );
    let noted = noted_on_mac(&state);
    assert!(
        noted["holds"].is_null() && noted["gone_since"] == json!(2 * AGAIN_MS),
        "it's noted as gone since then, beside what it may hold: {noted}"
    );

    let listings = asked_with(&here, "sessions").len();
    let parked = each_minute(
        &mut carrier,
        &state,
        &mut here,
        parked_at + 1..=parked_at + 10,
        |_| false,
    );
    assert!(
        parked.iter().all(|(_, gives)| gives.is_empty()),
        "once parked, it's given nothing: {parked:?}"
    );
    assert_eq!(
        runs_on_mac(&here) as u64,
        1 + parked_at,
        "nor asked: {:?}",
        here.asked
    );
    assert_eq!(
        asked_with(&here, "sessions").len(),
        listings + 10,
        "and mycelium is asked whom it lists once a minute"
    );

    here.unreached = None;
    here.unlisted = None;
    let back = carry(&mut carrier, &state, (parked_at + 11) * AGAIN_MS, &mut here);
    assert_eq!(
        back,
        vec![Carried {
            away: Some(Away::Back),
            ..gave(NAME, "mac", Gift::Resume)
        }],
        "listed again, it's given its resume at once"
    );
    let noted = noted_on_mac(&state);
    assert!(
        noted["holds"] == json(&state.join(format!("stops/{NAME}.json")))
            && noted.get("gone_since").is_none(),
        "and it's noted as holding the resume, no longer gone: {noted}"
    );
}

#[test]
fn a_machine_listed_while_out_of_reach_is_taken_for_gone_afresh_after() {
    let state = scratch("carry-listed-between");
    let mut here = machine();
    let mut carrier = given_mac(&state, &mut here);
    pineapple::resume::resume(&state.join("stops"), "all", 5, "joel").expect("resumed");
    here.unreached = Some("machine=mac");
    let hour = GONE_MS / AGAIN_MS;

    let rounds = each_minute(&mut carrier, &state, &mut here, 1..=31 + hour, |minute| {
        minute == 30
    });

    assert_eq!(
        when(&rounds, Away::Parked),
        vec![31 + hour],
        "listed in minute 30, the Mac is gone only from minute 31, so parked an hour after"
    );
}

#[test]
fn a_machine_gone_is_given_a_stop_standing_for_good() {
    let state = scratch("carry-standing-gone");
    keep(&state, NAME, &heard("slack:C1:1.000000"));
    let mut here = Machine {
        unreached: Some("machine=mac"),
        ..machine()
    };
    let mut carrier = Carrier::default();
    let hour = GONE_MS / AGAIN_MS;

    let rounds = each_minute(&mut carrier, &state, &mut here, 0..=hour + 7, |minute| {
        minute == 0
    });

    assert!(
        when(&rounds, Away::Parked).is_empty(),
        "a stop standing is never parked, since the Mac may come back with agents"
    );
    assert_eq!(
        runs_on_mac(&here) as u64,
        hour + 8,
        "so it's tried each minute: {rounds:?}"
    );
}

#[test]
fn a_machine_whose_give_began_there_is_never_taken_for_gone() {
    let state = scratch("carry-begun-gone");
    let mut here = machine();
    let mut carrier = given_mac(&state, &mut here);
    pineapple::resume::resume(&state.join("stops"), "all", 5, "joel").expect("resumed");
    here.dropped = Some("machine=mac");
    let hour = GONE_MS / AGAIN_MS;

    let rounds = each_minute(&mut carrier, &state, &mut here, 1..=hour + 7, |_| false);

    assert!(
        when(&rounds, Away::Parked).is_empty(),
        "the Mac began each give, so it's on the mesh, listed or not"
    );
    assert_eq!(
        runs_on_mac(&here) as u64,
        hour + 8,
        "so it's tried each minute: {rounds:?}"
    );
}

#[test]
fn a_machine_gone_an_hour_is_given_the_end_of_a_stop_once_listed_again() {
    let state = scratch("carry-parked-gone");
    let mut here = machine();
    let mut carrier = given_mac(&state, &mut here);
    fs::remove_file(state.join(format!("stops/{NAME}.json"))).expect("forgotten here");
    here.unreached = Some("machine=mac");
    let parked_at = 2 + GONE_MS / AGAIN_MS;

    let rounds = each_minute(&mut carrier, &state, &mut here, 1..=parked_at + 10, |_| {
        false
    });

    assert_eq!(
        when(&rounds, Away::Parked),
        vec![parked_at],
        "the Mac is parked once, an hour after it was first found gone"
    );
    assert_eq!(
        runs_on_mac(&here) as u64,
        1 + parked_at,
        "and isn't asked once parked: {rounds:?}"
    );
    assert_eq!(
        noted_on_mac(&state)["gone_since"],
        json!(2 * AGAIN_MS),
        "it's still noted as holding the stop, gone since minute 2"
    );

    here.unreached = None;
    here.unlisted = None;
    let back = carry(&mut carrier, &state, (parked_at + 11) * AGAIN_MS, &mut here);
    assert_eq!(
        back,
        vec![Carried {
            away: Some(Away::Back),
            ..gave(NAME, "mac", Gift::Removal)
        }],
        "listed again, it's given the stop's end at once"
    );
    assert!(
        !state.join(format!("carried/{NAME}.json")).exists(),
        "and nothing is noted as held of it"
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
