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
fn a_microphone_of_no_rank_gives_its_twin_none() {
    let chosen = picks("rankless");
    picked(&chosen);
    assert_eq!(rank(None, &chosen), None);
    assert_eq!(rank(Some("loud"), &chosen), None);
}
