//! The twins' rank: unranked until Joel keeps a cleaned pick, then just
//! above the microphone each cleans, whatever that microphone's rank.

use super::*;
use crate::denoise::Cleaner;
use crate::spot::Spot;

/// Picks kept in a folder of their own, none yet.
fn picks(name: &str) -> Chosen {
    let folder = std::env::temp_dir().join(format!("pipe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
}

/// A cleaned pick kept for the MV7 at the desk.
fn picked(chosen: &Chosen) {
    let desk = Spot {
        key: "home=6366013".to_owned(),
        name: "the desk".to_owned(),
    };
    chosen
        .choose("mv7", &desk, Cleaner::RNNoise)
        .expect("the pick is kept");
}

#[test]
fn a_twin_is_unranked_until_a_cleaned_pick_is_kept() {
    let chosen = picks("unranked");
    assert_eq!(
        rank(Some("1680"), &chosen),
        None,
        "a twin outranked its microphone before any pick"
    );
}

#[test]
fn a_twin_ranks_just_above_its_microphone_once_a_cleaned_pick_is_kept() {
    let chosen = picks("ranked");
    picked(&chosen);
    assert_eq!(rank(Some("1680"), &chosen), Some(1681));
    assert_eq!(
        rank(Some("2000"), &chosen),
        Some(2001),
        "a microphone never picked for is cleaned by the most picked, so its twin ranks too"
    );
}

#[test]
fn an_unknown_resource_said_on_the_core_leaves_the_twins_serving() {
    // What PipeWire said on the core as the dock's cards went, on
    // 2026-11-12 at 20:11: "unknown resource 5 op:3", for a stream
    // whose node it had already taken away.
    assert!(
        !ends(pw::core::PW_ID_CORE, -2),
        "an error about an object already gone ended every twin"
    );
}

#[test]
fn a_broken_connection_said_on_the_core_ends_serving() {
    assert!(
        ends(pw::core::PW_ID_CORE, -32),
        "PipeWire's connection breaking left the twins serving nothing"
    );
    assert!(
        ends(pw::core::PW_ID_CORE, -104),
        "PipeWire's connection reset left the twins serving nothing"
    );
}

#[test]
fn an_error_on_another_object_leaves_the_twins_serving() {
    assert!(!ends(5, -32), "one stream's error ended every twin");
}

#[test]
fn a_microphone_of_no_rank_gives_its_twin_none() {
    let chosen = picks("rankless");
    picked(&chosen);
    assert_eq!(rank(None, &chosen), None);
    assert_eq!(rank(Some("loud"), &chosen), None);
}
