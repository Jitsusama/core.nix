//! The shadow's tests: only a call's speech over its room is held, after a
//! warm-up and up to ten seconds, and its line is numbers and names alone.

use serde_json::json;

use super::*;

/// A frame at `amplitude`, steady, so its level is the amplitude's in dB.
fn steady(amplitude: f32) -> [f32; FRAME] {
    std::array::from_fn(|at| if at % 2 == 0 { amplitude } else { -amplitude })
}

/// Speech heard past the warm-up: `frames` of it at -20 dB, over a room
/// at -60.
fn spoken(frames: usize) -> Speech {
    let mut speech = Speech::default();
    for _ in 0..WARM {
        speech.heard(&steady(0.1), Some(-60));
    }
    for _ in 0..frames {
        speech.heard(&steady(0.1), Some(-60));
    }
    speech
}

fn marks(overall: f64) -> Marks {
    Marks {
        voice: 4.0,
        background: 3.5,
        overall,
    }
}

#[test]
fn speech_in_the_warm_up_is_not_held() {
    let mut speech = spoken(0);
    assert!(speech.taken().is_empty(), "the warm-up was held");
}

#[test]
fn only_speech_over_the_room_is_held() {
    let mut speech = spoken(0);
    for _ in 0..50 {
        speech.heard(&steady(0.002), Some(-60));
        speech.heard(&steady(0.1), Some(-60));
    }
    assert_eq!(
        speech.taken().len(),
        50 * FRAME,
        "the room between words was held as speech"
    );
}

#[test]
fn a_room_not_yet_measured_holds_nothing() {
    let mut speech = Speech::default();
    for _ in 0..(WARM + 50) {
        speech.heard(&steady(0.1), None);
    }
    assert!(speech.taken().is_empty(), "speech was held with no floor");
}

#[test]
fn ten_seconds_of_speech_are_held_at_most() {
    let mut speech = spoken(1_200);
    assert_eq!(speech.taken().len(), MOST);
}

#[test]
fn three_seconds_of_speech_are_enough() {
    assert!(!spoken(299).enough(), "under three seconds was enough");
    assert!(spoken(300).enough(), "three seconds weren't enough");
}

#[test]
fn the_best_way_scores_best_overall() {
    let marks = BTreeMap::from([
        ("raw", marks(2.9)),
        ("rnnoise", marks(3.4)),
        ("deepfilternet", marks(3.2)),
    ]);
    assert_eq!(best(&marks), Some("rnnoise"));
    assert_eq!(best(&BTreeMap::new()), None);
}

#[test]
fn a_calls_line_is_numbers_and_names_alone() {
    let call = Call {
        node: "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono".to_owned(),
        microphone: "Shure MV7".to_owned(),
        spot: "the desk".to_owned(),
        running: Cleaner::DeepFilterNet,
        speech: vec![0.5; LEAST],
    };
    let marks = BTreeMap::from([("raw", marks(2.9)), ("rnnoise", marks(3.4))]);
    let line = said(&call, &marks, 1_700_000_000_000);
    assert_eq!(
        line,
        json!({
            "at": 1_700_000_000_000_u64,
            "microphone": "Shure MV7",
            "node": "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono",
            "spot": "the desk",
            "running": "deepfilternet",
            "best": "rnnoise",
            "marks": {
                "raw": {"voice": 4.0, "background": 3.5, "overall": 2.9},
                "rnnoise": {"voice": 4.0, "background": 3.5, "overall": 3.4},
            },
        }),
    );
}

#[test]
fn each_calls_line_is_kept_after_the_last() {
    let folder = std::env::temp_dir().join(format!("shadow-kept-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let file = folder.join("judged");
    kept(&file, &json!({"best": "raw"}));
    kept(&file, &json!({"best": "rnnoise"}));
    let said = std::fs::read_to_string(&file).unwrap_or_default();
    assert_eq!(said, "{\"best\":\"raw\"}\n{\"best\":\"rnnoise\"}\n");
    let _ = std::fs::remove_dir_all(&folder);
}
