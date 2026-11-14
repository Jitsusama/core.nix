//! Choosing's tests: a cleaner better by the margin in two calls in a row
//! is chosen, once the scorer has agreed with Joel's ear, never where he's
//! picked by ear, and only for the microphone and spot it was heard at.

use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::chosen::tests::spot;

const MV7: &str = "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono";
const LAPTOP: &str = "alsa_input.pci-0000_00_1f.3.analog-stereo";
const DESK: &str = "home=6366013+screen=10ac41a1000000022022";
const COUCH: &str = "home=6366013";

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("chooses-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the test's folder can be made");
    folder
}

/// Every way's marks, each overall as given and the rest at 3.
fn marks(overall: &[(&str, f64)]) -> Value {
    let each: serde_json::Map<String, Value> = overall
        .iter()
        .map(|(way, overall)| {
            (
                (*way).to_owned(),
                json!({"voice": 3.0, "background": 3.0, "overall": overall}),
            )
        })
        .collect();
    Value::Object(each)
}

/// A judged call's line, as shadow.rs keeps it.
fn judged(node: &str, key: &str, running: &str, overall: &[(&str, f64)]) -> String {
    format!(
        "{}\n",
        json!({"at": 1, "microphone": "MV7", "node": node, "spot": key, "key": key,
            "running": running, "best": "", "marks": marks(overall)})
    )
}

/// A call where `RNNoise` scored 0.3 over `DeepFilterNet`, which ran.
fn rnnoise_better(node: &str, key: &str) -> String {
    judged(
        node,
        key,
        "deepfilternet",
        &[
            ("raw", 2.5),
            ("deepfilternet", 3.0),
            ("deepfilternet-gentle", 2.9),
            ("rnnoise", 3.3),
        ],
    )
}

/// A pick by ear's line in `scored`, the scorer's best being `best`.
fn scored(picked: &str, best: &str) -> String {
    let overall: Vec<(&str, f64)> = ["raw", "deepfilternet", "deepfilternet-gentle", "rnnoise"]
        .into_iter()
        .map(|way| (way, if way == best { 3.5 } else { 3.0 }))
        .collect();
    format!(
        "{}\n",
        json!({"at": 1, "microphone": "MV7", "picked": picked, "marks": marks(&overall)})
    )
}

/// `matched` picks the scorer agreed with, then `missed` it didn't.
fn scored_lines(matched: usize, missed: usize) -> String {
    let mut lines = scored("rnnoise", "rnnoise").repeat(matched);
    for _ in 0..missed {
        lines.push_str(&scored("raw", "rnnoise"));
    }
    lines
}

#[test]
fn a_cleaner_better_in_two_calls_in_a_row_is_better() {
    let lines = rnnoise_better(MV7, DESK) + &rnnoise_better(MV7, DESK);
    assert_eq!(
        better(&lines, MV7, DESK),
        Some(Cleaner::RNNoise),
        "two calls in a row saying RNNoise weren't heard"
    );
}

/// A call that skipped noise cancelling, where `RNNoise` scored 0.8 over
/// the microphone as it is.
fn plain_rnnoise_better(node: &str, key: &str) -> String {
    judged(
        node,
        key,
        "raw",
        &[
            ("raw", 2.5),
            ("deepfilternet", 3.0),
            ("deepfilternet-gentle", 2.9),
            ("rnnoise", 3.3),
        ],
    )
}

#[test]
fn calls_that_skip_noise_cancelling_choose_nothing_and_break_no_row() {
    let plain = plain_rnnoise_better(MV7, DESK);
    assert_eq!(
        better(&plain.repeat(2), MV7, DESK),
        None,
        "calls on the microphone itself chose for its twin, which they never heard"
    );
    let lines = rnnoise_better(MV7, DESK) + &plain + &rnnoise_better(MV7, DESK);
    assert_eq!(
        better(&lines, MV7, DESK),
        Some(Cleaner::RNNoise),
        "a call on the microphone itself broke the twin's two in a row"
    );
}

#[test]
fn one_call_is_not_enough() {
    assert_eq!(
        better(&rnnoise_better(MV7, DESK), MV7, DESK),
        None,
        "a single call chose"
    );
}

#[test]
fn a_cleaner_under_the_margin_is_not_better() {
    let close = judged(
        MV7,
        DESK,
        "deepfilternet",
        &[("raw", 2.5), ("deepfilternet", 3.0), ("rnnoise", 3.15)],
    );
    assert_eq!(
        better(&(close.clone() + &close), MV7, DESK),
        None,
        "0.15 over what ran chose"
    );
}

#[test]
fn two_calls_saying_different_cleaners_say_none() {
    let gentle = judged(
        MV7,
        DESK,
        "deepfilternet",
        &[
            ("deepfilternet", 3.0),
            ("deepfilternet-gentle", 3.4),
            ("rnnoise", 3.1),
        ],
    );
    let lines = rnnoise_better(MV7, DESK) + &gentle;
    assert_eq!(
        better(&lines, MV7, DESK),
        None,
        "two calls that disagreed chose"
    );
}

#[test]
fn the_newest_call_breaks_a_run_when_it_says_otherwise() {
    let ran_best = judged(
        MV7,
        DESK,
        "deepfilternet",
        &[("deepfilternet", 3.4), ("rnnoise", 3.1)],
    );
    let lines = rnnoise_better(MV7, DESK) + &rnnoise_better(MV7, DESK) + &ran_best;
    assert_eq!(
        better(&lines, MV7, DESK),
        None,
        "a call where what ran scored best didn't end the run"
    );
}

#[test]
fn calls_on_another_microphone_or_spot_are_not_counted() {
    let lines = rnnoise_better(MV7, DESK)
        + &rnnoise_better(LAPTOP, DESK)
        + &rnnoise_better(MV7, COUCH)
        + &rnnoise_better(MV7, DESK);
    assert_eq!(
        better(&lines, MV7, DESK),
        Some(Cleaner::RNNoise),
        "another microphone's or spot's calls came between the desk's"
    );
    assert_eq!(
        better(
            &(rnnoise_better(LAPTOP, DESK) + &rnnoise_better(MV7, COUCH)),
            MV7,
            DESK
        ),
        None,
        "another microphone's and spot's calls counted for the desk's MV7"
    );
}

#[test]
fn a_voice_best_heard_raw_still_wants_a_cleaner_by_the_margin() {
    let raw_best = judged(
        MV7,
        DESK,
        "deepfilternet",
        &[("raw", 3.6), ("deepfilternet", 3.0), ("rnnoise", 3.1)],
    );
    assert_eq!(
        better(&(raw_best.clone() + &raw_best), MV7, DESK),
        None,
        "raw scoring best chose a cleaner barely over what ran"
    );
}

#[test]
fn a_line_that_is_not_a_call_is_passed_over() {
    let lines = rnnoise_better(MV7, DESK) + "half a line\n" + &rnnoise_better(MV7, DESK);
    assert_eq!(
        better(&lines, MV7, DESK),
        Some(Cleaner::RNNoise),
        "a broken line broke the run"
    );
}

#[test]
fn agreement_is_counted_over_the_newest_twenty_picks() {
    assert_eq!(agreement(""), (0, 0), "no picks agreed with");
    assert_eq!(
        agreement(&scored_lines(3, 2)),
        (3, 5),
        "five picks, three matched, weren't counted so"
    );
    let lines = scored_lines(5, 0) + &scored_lines(9, 11);
    assert_eq!(
        agreement(&lines),
        (9, 20),
        "picks older than the newest twenty counted"
    );
}

/// The picks and scores in `folder`, with the scorer's best having
/// matched `matched` of Joel's twenty newest picks.
fn agreed(folder: &std::path::Path, matched: usize) -> Chosen {
    std::fs::write(
        folder.join("scored"),
        scored_lines(matched, NEWEST - matched),
    )
    .expect("scored can be written");
    Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
}

#[test]
fn once_agreed_with_two_better_calls_choose_for_the_next() {
    let folder = folder("chooses");
    let chosen = agreed(&folder, AGREED);
    std::fs::write(
        folder.join("judged"),
        rnnoise_better(MV7, DESK) + &rnnoise_better(MV7, DESK),
    )
    .expect("judged can be written");
    assert_eq!(
        decided(&chosen, MV7, &spot(DESK)),
        Some(Cleaner::RNNoise),
        "nothing was chosen"
    );
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&spot(DESK))),
        Cleaner::RNNoise,
        "the MV7's next call at the desk doesn't clean as chosen"
    );
    assert_eq!(
        chosen.cleaner(Some(LAPTOP), Some(&spot(DESK))),
        Cleaner::DeepFilterNet,
        "the laptop's microphone took the MV7's choice"
    );
    assert_eq!(
        decided(&chosen, MV7, &spot(DESK)),
        None,
        "what was chosen already was chosen again"
    );
}

#[test]
fn until_the_scorer_agrees_with_joel_nothing_is_chosen() {
    let folder = folder("disagrees");
    let chosen = agreed(&folder, AGREED - 1);
    std::fs::write(
        folder.join("judged"),
        rnnoise_better(MV7, DESK) + &rnnoise_better(MV7, DESK),
    )
    .expect("judged can be written");
    assert_eq!(
        decided(&chosen, MV7, &spot(DESK)),
        None,
        "it chose with the scorer matching 9 of 20"
    );
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&spot(DESK))),
        Cleaner::DeepFilterNet,
        "a choice was kept anyway"
    );
}

#[test]
fn where_joel_picked_by_ear_nothing_is_chosen() {
    let folder = folder("pinned");
    let chosen = agreed(&folder, NEWEST);
    chosen
        .choose(MV7, &spot(DESK), Cleaner::DeepFilterNet)
        .expect("a pick can be kept");
    std::fs::write(
        folder.join("judged"),
        rnnoise_better(MV7, DESK) + &rnnoise_better(MV7, DESK),
    )
    .expect("judged can be written");
    assert_eq!(
        decided(&chosen, MV7, &spot(DESK)),
        None,
        "it chose over Joel's pick by ear"
    );
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&spot(DESK))),
        Cleaner::DeepFilterNet,
        "Joel's pick at the desk was overruled"
    );
}
