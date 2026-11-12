//! The picks' tests: each microphone keeps its own at each spot, one at a
//! spot it's never been picked at takes its newest anywhere, one never
//! picked for takes the most picked, and a file that can't be read cleans
//! as Nix says.

use super::*;

const MV7: &str = "alsa_input.usb-Shure_Inc_Shure_MV7-00.analog-mono";
const LAPTOP: &str = "alsa_input.pci-0000_00_1f.3.analog-stereo";
const HEADSET: &str = "bluez_input.headset";

fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("chosen-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    folder
}

/// A spot by its key, named for it.
fn spot(key: &str) -> Spot {
    Spot {
        key: key.to_owned(),
        name: key.to_owned(),
    }
}

/// The desk, where most of these are picked.
fn desk() -> Spot {
    spot("home=6366013+screen=10ac41a1000000022022")
}

#[test]
fn nothing_picked_cleans_as_nix_says() {
    let chosen = Chosen::new(Some(folder("none").join("cleaner")), Cleaner::DeepFilterNet);
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::DeepFilterNet
    );
    assert_eq!(chosen.cleaner(None, None), Cleaner::DeepFilterNet);
}

#[test]
fn a_pick_is_kept_and_cleans_from_then_on() {
    let file = folder("kept").join("noise-cancelled").join("cleaner");
    let chosen = Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, &desk(), Cleaner::RNNoise)
        .expect("the pick is kept");
    assert_eq!(chosen.cleaner(Some(MV7), Some(&desk())), Cleaner::RNNoise);
    let other = Chosen::new(Some(file), Cleaner::DeepFilterNetGentle);
    assert_eq!(
        other.cleaner(Some(MV7), Some(&desk())),
        Cleaner::RNNoise,
        "another process reads the pick"
    );
}

#[test]
fn each_microphone_keeps_its_own_pick() {
    let chosen = Chosen::new(Some(folder("own").join("cleaner")), Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, &desk(), Cleaner::RNNoise)
        .expect("the MV7's is kept");
    chosen
        .choose(LAPTOP, &desk(), Cleaner::DeepFilterNetGentle)
        .expect("the laptop's is kept");
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::RNNoise,
        "the laptop's pick took the MV7's place"
    );
    assert_eq!(
        chosen.cleaner(Some(LAPTOP), Some(&desk())),
        Cleaner::DeepFilterNetGentle
    );
    chosen
        .choose(MV7, &desk(), Cleaner::DeepFilterNet)
        .expect("picked again");
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::DeepFilterNet
    );
    assert_eq!(
        chosen.cleaner(Some(LAPTOP), Some(&desk())),
        Cleaner::DeepFilterNetGentle,
        "picking again for the MV7 changed the laptop's"
    );
}

#[test]
fn each_spot_keeps_its_own_pick() {
    let chosen = Chosen::new(
        Some(folder("spots").join("cleaner")),
        Cleaner::DeepFilterNet,
    );
    let couch = spot("home=6366035");
    chosen
        .choose(MV7, &desk(), Cleaner::DeepFilterNetGentle)
        .expect("the desk's is kept");
    chosen
        .choose(MV7, &couch, Cleaner::RNNoise)
        .expect("the couch's is kept");
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::DeepFilterNetGentle,
        "the couch's pick took the desk's place"
    );
    assert_eq!(chosen.cleaner(Some(MV7), Some(&couch)), Cleaner::RNNoise);
}

#[test]
fn a_spot_never_picked_at_takes_the_microphones_newest_pick() {
    let chosen = Chosen::new(
        Some(folder("guess").join("cleaner")),
        Cleaner::DeepFilterNet,
    );
    chosen.choose(MV7, &desk(), Cleaner::RNNoise).expect("kept");
    chosen
        .choose(LAPTOP, &spot("home=6366035"), Cleaner::DeepFilterNetGentle)
        .expect("kept");
    chosen
        .choose(
            LAPTOP,
            &spot("network=50-wifi-Lantern"),
            Cleaner::DeepFilterNetGentle,
        )
        .expect("kept");
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&spot("network=50-wifi-Roadside%20Inn"))),
        Cleaner::RNNoise,
        "the MV7 somewhere new should start from its own pick, not the most picked"
    );
}

#[test]
fn a_microphone_never_picked_for_takes_the_most_picked() {
    let chosen = Chosen::new(Some(folder("most").join("cleaner")), Cleaner::DeepFilterNet);
    chosen.choose(MV7, &desk(), Cleaner::RNNoise).expect("kept");
    chosen
        .choose(LAPTOP, &desk(), Cleaner::RNNoise)
        .expect("kept");
    chosen
        .choose("usb-webcam", &desk(), Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(
        chosen.cleaner(Some(HEADSET), Some(&desk())),
        Cleaner::RNNoise
    );
    assert_eq!(chosen.cleaner(None, None), Cleaner::RNNoise);
}

#[test]
fn a_tie_goes_to_the_newest_pick() {
    let chosen = Chosen::new(Some(folder("tie").join("cleaner")), Cleaner::DeepFilterNet);
    chosen.choose(MV7, &desk(), Cleaner::RNNoise).expect("kept");
    chosen
        .choose(LAPTOP, &desk(), Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(
        chosen.cleaner(Some(HEADSET), Some(&desk())),
        Cleaner::DeepFilterNetGentle
    );
}

#[test]
fn each_pick_keeps_its_spot_in_the_file() {
    let file = folder("file").join("cleaner");
    let chosen = Chosen::new(Some(file.clone()), Cleaner::DeepFilterNet);
    chosen.choose(MV7, &desk(), Cleaner::RNNoise).expect("kept");
    assert_eq!(
        std::fs::read_to_string(file).expect("the file is there"),
        format!("rnnoise {MV7} {}\n", desk().key),
    );
}

#[test]
fn a_pick_from_before_they_were_kept_apart_still_counts() {
    let folder = folder("before");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(folder.join("cleaner"), "rnnoise\n").expect("the file is written");
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet);
    assert_eq!(chosen.cleaner(Some(MV7), Some(&desk())), Cleaner::RNNoise);
    chosen
        .choose(LAPTOP, &desk(), Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(
        chosen.cleaner(Some(LAPTOP), Some(&desk())),
        Cleaner::DeepFilterNetGentle
    );
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::DeepFilterNetGentle,
        "the tie should go to the newest pick"
    );
}

#[test]
fn a_microphones_pick_from_before_spots_counts_anywhere_until_its_picked_again() {
    let folder = folder("spotless");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(folder.join("cleaner"), format!("rnnoise {MV7}\n"))
        .expect("the file is written");
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet);
    assert_eq!(chosen.cleaner(Some(MV7), Some(&desk())), Cleaner::RNNoise);
    let couch = spot("home=6366035");
    chosen
        .choose(MV7, &couch, Cleaner::DeepFilterNetGentle)
        .expect("kept");
    assert_eq!(
        std::fs::read_to_string(folder.join("cleaner")).expect("the file is there"),
        format!("deepfilternet-gentle {MV7} {}\n", couch.key),
        "a pick at a spot should take the place of the microphone's pick from before spots"
    );
}

#[test]
fn a_file_naming_no_cleaner_cleans_as_nix_says() {
    let folder = folder("garbled");
    std::fs::create_dir_all(&folder).expect("the folder is made");
    std::fs::write(folder.join("cleaner"), "loud mv7\nquiet\n").expect("the file is written");
    let chosen = Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNetGentle);
    assert_eq!(
        chosen.cleaner(Some("mv7"), Some(&desk())),
        Cleaner::DeepFilterNetGentle
    );
}

/// Picks kept in `folder`, at a machine that knows nothing of where it
/// is, so its spot is `network=none`, called "away".
fn somewhere(folder: &Path) -> Chosen {
    Chosen::new(Some(folder.join("cleaner")), Cleaner::DeepFilterNet)
        .within(Where::at(&folder.join("machine")))
}

#[test]
fn a_spot_takes_the_name_joel_gives_it() {
    let folder = folder("named");
    let chosen = somewhere(&folder);
    let away = chosen.spot();
    assert_eq!(away.name, "away");
    assert!(!chosen.named(&away.key), "a spot nobody named is named");
    let named = chosen
        .name(&away, "  the couch ")
        .expect("the name is kept");
    assert_eq!(named.key, away.key, "naming moved the spot's picks");
    assert_eq!(named.name, "the couch");
    assert_eq!(
        somewhere(&folder).spot().name,
        "the couch",
        "another process reads the name"
    );
    assert!(somewhere(&folder).named(&away.key));
}

#[test]
fn a_spot_renamed_keeps_only_its_new_name() {
    let folder = folder("renamed");
    let chosen = somewhere(&folder);
    chosen.name(&desk(), "the desk").expect("the desk is named");
    chosen
        .name(&chosen.spot(), "the couch")
        .expect("the couch is named");
    chosen
        .name(&chosen.spot(), "the sofa")
        .expect("the couch is renamed");
    assert_eq!(chosen.spot().name, "the sofa");
    let said = std::fs::read_to_string(folder.join("spots")).expect("the names are kept");
    assert_eq!(
        said,
        format!("{} the desk\nnetwork=none the sofa\n", desk().key),
        "a rename should leave one line a spot"
    );
}

#[test]
fn a_name_saying_nothing_or_too_much_is_refused() {
    let folder = folder("unnamed");
    let chosen = somewhere(&folder);
    let away = chosen.spot();
    for name in ["", "   ", "the\ncouch", &"long ".repeat(9)] {
        assert!(
            chosen.name(&away, name).is_err(),
            "{name:?} was taken as a name"
        );
    }
    assert_eq!(chosen.spot().name, "away", "a refused name was kept");
}

#[test]
fn nowhere_to_keep_it_keeps_nothing() {
    let chosen = Chosen::new(None, Cleaner::DeepFilterNet);
    chosen
        .choose(MV7, &desk(), Cleaner::RNNoise)
        .expect("nothing to write is no failure");
    assert_eq!(
        chosen.cleaner(Some(MV7), Some(&desk())),
        Cleaner::DeepFilterNet
    );
}
