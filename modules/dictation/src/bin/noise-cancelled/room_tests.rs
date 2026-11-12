//! The room's tests: its floor is the level it sits on between words, a
//! microphone gone silent isn't a room, and each pick's floor is kept
//! apart, nudged once.

use super::*;
use crate::denoise::Cleaner;

const MV7: &str = "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono";
const DESK: &str = "home=6366013";
const ROADSIDE: &str = "home=6366035";

/// A frame at `amplitude`, steady, so its level is the amplitude's in dB.
fn steady(amplitude: f32) -> [f32; FRAME] {
    std::array::from_fn(|at| if at % 2 == 0 { amplitude } else { -amplitude })
}

/// Picks kept in a folder of their own, none yet.
fn picks(name: &str) -> Chosen {
    let folder = std::env::temp_dir().join(format!("room-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
}

#[test]
fn a_steady_noise_is_its_own_floor() {
    let mut room = Room::default();
    for _ in 0..200 {
        room.heard(&steady(0.01));
    }
    assert_eq!(
        room.floor(),
        Some(-40),
        "0.01 of full scale is 40 dB under it"
    );
}

#[test]
fn a_voice_over_the_room_leaves_the_rooms_floor() {
    let mut room = Room::default();
    for at in 0..300 {
        let amplitude = if at % 4 == 0 { 0.001 } else { 0.3 };
        room.heard(&steady(amplitude));
    }
    assert_eq!(room.floor(), Some(-60), "the voice raised the room's floor");
}

#[test]
fn a_microphone_gone_silent_is_no_room() {
    let mut room = Room::default();
    for _ in 0..200 {
        room.heard(&[0.0; FRAME]);
    }
    assert_eq!(room.floor(), None, "silence was heard as a room");
    for _ in 0..200 {
        room.heard(&steady(0.01));
    }
    assert_eq!(room.floor(), Some(-40), "the silence before counted");
}

#[test]
fn under_a_second_is_no_floor() {
    let mut room = Room::default();
    for _ in 0..99 {
        room.heard(&steady(0.01));
    }
    assert_eq!(room.floor(), None);
    room.heard(&steady(0.01));
    assert_eq!(room.floor(), Some(-40));
}

#[test]
fn a_recording_is_heard_a_frame_at_a_time() {
    let samples: Vec<f32> = (0..150).flat_map(|_| steady(0.1)).collect();
    assert_eq!(of(&samples), Some(-20));
    assert_eq!(of(&samples[..FRAME * 10]), None, "a tenth of a second");
}

#[test]
fn each_picks_room_is_kept_apart_until_picked_again() {
    let chosen = picks("kept");
    assert_eq!(picked_in(&chosen, MV7, DESK), None);
    kept(&chosen, MV7, DESK, -62).expect("the room is kept");
    kept(&chosen, MV7, ROADSIDE, -48).expect("the room is kept");
    assert_eq!(picked_in(&chosen, MV7, DESK), Some((-62, false)));
    assert_eq!(picked_in(&chosen, MV7, ROADSIDE), Some((-48, false)));
    kept(&chosen, MV7, DESK, -55).expect("picked again");
    assert_eq!(
        picked_in(&chosen, MV7, DESK),
        Some((-55, false)),
        "a new pick's room replaces the one before"
    );
}

#[test]
fn a_nudge_is_kept_until_picked_again() {
    let chosen = picks("nudged");
    kept(&chosen, MV7, DESK, -62).expect("the room is kept");
    kept(&chosen, MV7, ROADSIDE, -48).expect("the room is kept");
    nudged(&chosen, MV7, DESK).expect("nudged");
    assert_eq!(picked_in(&chosen, MV7, DESK), Some((-62, true)));
    assert_eq!(
        picked_in(&chosen, MV7, ROADSIDE),
        Some((-48, false)),
        "a nudge at the desk reached the Roadside Inn"
    );
    kept(&chosen, MV7, DESK, -50).expect("picked again");
    assert_eq!(picked_in(&chosen, MV7, DESK), Some((-50, false)));
}

#[test]
fn nowhere_to_keep_it_keeps_nothing() {
    let chosen = Chosen::new(None, Cleaner::DeepFilterNet);
    kept(&chosen, MV7, DESK, -62).expect("nothing to write is no failure");
    assert_eq!(picked_in(&chosen, MV7, DESK), None);
}
