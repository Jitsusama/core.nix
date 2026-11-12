//! The offer's tests: a call on a microphone never picked for at its
//! spot offers Choose by Ear there once, after the call and while nothing
//! refuses, and never again once it's taken or picked for.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::*;
use crate::denoise::Cleaner;

const MV7: &str = "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono";
const SHURE: &str = "Shure MV7";

/// A folder of picks of its own, emptied.
fn picks(name: &str) -> Chosen {
    let folder = std::env::temp_dir().join(format!("offer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
}

/// A spot by its key, with a name of Joel's.
fn spot(key: &str, name: &str) -> Spot {
    Spot {
        key: key.to_owned(),
        name: name.to_owned(),
    }
}

fn desk() -> Spot {
    spot("home=6366013", "the desk")
}

fn roadside() -> Spot {
    spot("home=6366035", "Roadside Inn")
}

/// A refusal that holds, as a meeting near does, or doesn't.
fn refusal(holds: bool) -> Refusal {
    Refusal {
        command: vec![if holds { "true" } else { "false" }.to_owned()],
        why: "a meeting is near".to_owned(),
    }
}

/// The offer file beside the picks.
fn file(chosen: &Chosen) -> PathBuf {
    chosen.beside("offer").expect("picks are kept")
}

/// What the offer standing says, or none while none stands.
fn standing(chosen: &Chosen) -> Option<Value> {
    let said = std::fs::read_to_string(file(chosen)).ok()?;
    Some(serde_json::from_str(&said).expect("the offer is JSON"))
}

/// The MV7 used for a call at `spot`.
fn called(offers: &mut Offers, chosen: &Chosen, spot: &Spot) {
    offers.heard(chosen, MV7, SHURE, spot, Duration::from_mins(20));
}

#[test]
fn a_call_on_a_microphone_never_picked_for_offers_it_after() {
    let chosen = picks("new");
    let mut offers = Offers::new(&chosen, vec![refusal(false)]);
    called(&mut offers, &chosen, &desk());
    assert_eq!(standing(&chosen), None, "nothing stands before a pass");
    offers.stand(&chosen);
    assert_eq!(
        standing(&chosen),
        Some(json!({
            "microphone": SHURE, "node": MV7, "spot": "the desk",
            "key": "home=6366013", "new": "microphone",
        })),
    );
}

#[test]
fn a_microphone_picked_elsewhere_is_offered_for_the_new_spot() {
    let chosen = picks("elsewhere");
    chosen
        .choose(MV7, &roadside(), Cleaner::RNNoise)
        .expect("the pick is kept");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    assert_eq!(
        standing(&chosen).map(|offer| offer["new"].clone()),
        Some(json!("spot")),
    );
}

#[test]
fn a_microphone_picked_where_it_was_used_is_not_offered() {
    let chosen = picks("picked");
    chosen
        .choose(MV7, &desk(), Cleaner::RNNoise)
        .expect("the pick is kept");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    assert_eq!(standing(&chosen), None);
}

#[test]
fn a_moment_recorded_is_not_a_call() {
    let chosen = picks("moment");
    let mut offers = Offers::new(&chosen, Vec::new());
    offers.heard(&chosen, MV7, SHURE, &desk(), Duration::from_secs(5));
    offers.stand(&chosen);
    assert_eq!(
        standing(&chosen),
        None,
        "a level meter's moment offers nothing"
    );
}

#[test]
fn an_offer_stands_only_while_no_refusal_holds() {
    let chosen = picks("refused");
    let mut offers = Offers::new(&chosen, vec![refusal(false), refusal(true)]);
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    assert_eq!(
        standing(&chosen),
        None,
        "a meeting near keeps it from standing"
    );
    let mut clear = Offers::new(&chosen, vec![refusal(false)]);
    called(&mut clear, &chosen, &desk());
    clear.stand(&chosen);
    assert!(
        standing(&chosen).is_some(),
        "it stands once nothing refuses"
    );
}

#[test]
fn a_call_withdraws_the_offer_until_it_ends() {
    let chosen = picks("withdrawn");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    assert!(standing(&chosen).is_some(), "it stands after the call");
    offers.withheld(&chosen);
    assert!(!file(&chosen).exists(), "withdrawn while a call is on");
    offers.stand(&chosen);
    assert!(standing(&chosen).is_some(), "standing again after it");
}

#[test]
fn an_offer_taken_is_never_made_again() {
    let chosen = picks("taken");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    taken(&chosen).expect("it's taken");
    assert_eq!(standing(&chosen), None, "taken, it's withdrawn");
    offers.stand(&chosen);
    assert_eq!(standing(&chosen), None, "and it doesn't stand again");
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    assert_eq!(standing(&chosen), None, "nor after the next call there");
    called(&mut offers, &chosen, &roadside());
    offers.stand(&chosen);
    assert_eq!(
        standing(&chosen).map(|offer| offer["key"].clone()),
        Some(json!("home=6366035")),
        "a new spot is offered all the same"
    );
}

#[test]
fn a_pick_by_ear_withdraws_the_offer() {
    let chosen = picks("by-ear");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    chosen
        .choose(MV7, &desk(), Cleaner::DeepFilterNetGentle)
        .expect("the pick is kept");
    offers.stand(&chosen);
    assert_eq!(standing(&chosen), None);
}

#[test]
fn an_offer_standing_stays_through_a_restart() {
    let chosen = picks("restart");
    let mut offers = Offers::new(&chosen, Vec::new());
    called(&mut offers, &chosen, &desk());
    offers.stand(&chosen);
    let mut restarted = Offers::new(&chosen, Vec::new());
    restarted.withheld(&chosen);
    assert_eq!(standing(&chosen), None, "the service knows it stands");
    restarted.stand(&chosen);
    assert!(standing(&chosen).is_some(), "and stands it again");
}
