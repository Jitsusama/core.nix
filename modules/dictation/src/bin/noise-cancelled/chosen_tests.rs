//! The picks' tests: each microphone keeps its own, one never picked for
//! takes the most picked, and a file that can't be read cleans as Nix says.

use super::*;

const MV7: &str = "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono";
const LAPTOP: &str = "alsa_input.pci-0000_00_1f.3.analog-stereo";
const HEADSET: &str = "bluez_input.headset";

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("chosen-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

#[test]
fn nothing_picked_cleans_as_nix_says() {
    let chosen = Chosen::new(Some(folder("none").join("cleaner")), Cleaner::DeepFilterNet);
    assert_eq!(chosen.cleaner(Some(MV7)), Cleaner::DeepFilterNet);
    assert_eq!(chosen.cleaner(None), Cleaner::DeepFilterNet);
}

#[test]
fn a_pick_is_kept_and_cleans_from_then_on() {
    let file = folder("kept").join("noise-cancelled").join("cleaner");
    let chosen = Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, Cleaner::RNNoise)
        .expect("the pick is kept");
    assert_eq!(chosen.cleaner(Some(MV7)), Cleaner::RNNoise);
    let other = Chosen::new(Some(file), Cleaner::DeepFilterNetGentle);
    assert_eq!(
        other.cleaner(Some(MV7)),
        Cleaner::RNNoise,
        "another process reads the pick"
    );
}

#[test]
fn each_microphone_keeps_its_own_pick() {
    let chosen = Chosen::new(Some(folder("own").join("cleaner")), Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, Cleaner::RNNoise)
        .expect("the MV7's is kept");
    chosen
        .choose(LAPTOP, Cleaner::DeepFilterNetGentle)
        .expect("the laptop's is kept");
    assert_eq!(
        chosen.cleaner(Some(MV7)),
        Cleaner::RNNoise,
        "the laptop's pick took the MV7's place"
    );
    assert_eq!(chosen.cleaner(Some(LAPTOP)), Cleaner::DeepFilterNetGentle);
    chosen
        .choose(MV7, Cleaner::DeepFilterNet)
        .expect("picked again");
    assert_eq!(chosen.cleaner(Some(MV7)), Cleaner::DeepFilterNet);
    assert_eq!(
        chosen.cleaner(Some(LAPTOP)),
        Cleaner::DeepFilterNetGentle,
        "picking again for the MV7 changed the laptop's"
    );
}

#[test]
fn a_microphone_never_picked_for_takes_the_most_picked() {
    let chosen = Chosen::new(Some(folder("most").join("cleaner")), Cleaner::DeepFilterNet);
    chosen.choose(MV7, Cleaner::RNNoise).expect("kept");
    chosen.choose(LAPTOP, Cleaner::RNNoise).expect("kept");
    chosen
        .choose("usb-webcam", Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(chosen.cleaner(Some(HEADSET)), Cleaner::RNNoise);
    assert_eq!(chosen.cleaner(None), Cleaner::RNNoise);
}

#[test]
fn a_tie_goes_to_the_newest_pick() {
    let chosen = Chosen::new(Some(folder("tie").join("cleaner")), Cleaner::DeepFilterNet);
    chosen.choose(MV7, Cleaner::RNNoise).expect("kept");
    chosen
        .choose(LAPTOP, Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(chosen.cleaner(Some(HEADSET)), Cleaner::DeepFilterNetGentle);
}

#[test]
fn a_pick_from_before_they_were_kept_apart_still_counts() {
    let folder = folder("before");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(folder.join("cleaner"), "rnnoise\n").expect("the file is written");
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet);
    assert_eq!(chosen.cleaner(Some(MV7)), Cleaner::RNNoise);
    chosen
        .choose(LAPTOP, Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(chosen.cleaner(Some(LAPTOP)), Cleaner::DeepFilterNetGentle);
    assert_eq!(
        chosen.cleaner(Some(MV7)),
        Cleaner::DeepFilterNetGentle,
        "the tie should go to the newest pick"
    );
}

#[test]
fn a_file_naming_no_cleaner_cleans_as_nix_says() {
    let folder = folder("garbled");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(folder.join("cleaner"), "loud mv7\nquiet\n").expect("the file is written");
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNetGentle);
    assert_eq!(chosen.cleaner(Some("mv7")), Cleaner::DeepFilterNetGentle);
}

#[test]
fn nowhere_to_keep_it_keeps_nothing() {
    let chosen = Chosen::new(None, Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, Cleaner::RNNoise)
        .expect("nothing to write is no failure");
    assert_eq!(chosen.cleaner(Some(MV7)), Cleaner::DeepFilterNet);
}
