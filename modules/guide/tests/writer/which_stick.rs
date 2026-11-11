//! Which stick is written: none plugged in waits for one, two are both named
//! and neither written, the one is written whole with what was mounted from it
//! handed over, and the last card says how to reach the machine's firmware.

use std::fs;
use std::process::Command;

use guide::stand_in::StandIn;
use guide::writer::WriteStick;
use guide::{Answer, Answers, Ask, Fault, Journey, Standing, Step};

use super::{image, lsblk, payload, repair, stick_file};
use crate::common::Scratch;

/// lsblk's listing with only the disks this keeps.
fn listing(json: &str, keep: impl Fn(&str) -> bool) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    value["blockdevices"]
        .as_array_mut()
        .unwrap()
        .retain(|device| keep(device["name"].as_str().unwrap()));
    value.to_string()
}

#[test]
fn with_no_stick_plugged_in_it_waits_for_one() {
    let directory = Scratch::new("none");
    let mut stand_in = StandIn::new();
    lsblk(
        &mut stand_in,
        &listing(fixture!("lsblk-blank.json"), |name| name != "sda"),
    );
    let standing = repair(&directory).observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("no stick")),
        "{standing:?}"
    );
}

#[test]
fn two_sticks_plugged_in_are_both_named_and_neither_is_written() {
    let directory = Scratch::new("two");
    let mut value: serde_json::Value = serde_json::from_str(fixture!("lsblk-blank.json")).unwrap();
    let devices = value["blockdevices"].as_array_mut().unwrap();
    let mut second = devices
        .iter()
        .find(|device| device["name"] == "sda")
        .unwrap()
        .clone();
    second["name"] = "sdb".into();
    second["path"] = "/dev/sdb".into();
    devices.push(second);
    let mut stand_in = StandIn::new();
    lsblk(&mut stand_in, &value.to_string());
    let step = repair(&directory);
    let standing = step.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.starts_with("2 removable disks")),
        "{standing:?}"
    );
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    assert!(step.run(&mut stand_in, &answers).is_err());
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("opening")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn the_one_stick_is_written_whole_and_what_was_mounted_from_it_is_handed_over() {
    let directory = Scratch::new("written");
    let stick = stick_file(&directory);
    let mut stand_in = StandIn::new();
    // The stick's own partitions are mounted here, as the VM's booted
    // stick has them.
    lsblk(&mut stand_in, fixture!("lsblk-laid-out.json"));
    stand_in.disk("/dev/sda", &stick);
    let step = repair(&directory);
    let Standing::Needed { asks } = step.observe(&mut stand_in).unwrap() else {
        panic!("the stick is still to write")
    };
    assert_eq!(
        asks.len(),
        1,
        "a repair stick asks for nothing secret: {asks:?}"
    );
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    Journey::attempt(&step, &mut stand_in, &answers).unwrap();
    // The screen was told what the write was doing, last reading it back.
    let said = guide::Machine::meter(&mut stand_in).said();
    assert!(said.starts_with("Reading it back to check it: "), "{said}");

    let opened = stand_in
        .calls
        .iter()
        .find(|call| call.starts_with("opening /dev/sda"))
        .unwrap();
    assert!(
        opened.contains("/dev/sda1") && opened.contains("/dev/sda2"),
        "{opened}"
    );
    let verified = Command::new("sfdisk")
        .arg("--verify")
        .arg(&stick)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&verified.stdout).contains("No errors detected"),
        "{verified:?}"
    );
    assert!(matches!(
        step.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn the_sticks_passphrase_is_seven_words_shown_for_the_phone_and_typed_back() {
    let directory = Scratch::new("passphrases");
    let mut stand_in = StandIn::new();
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    stand_in.disk("/dev/sda", &stick_file(&directory));
    // A payload of nothing but its target, which is all this reads first.
    let payload = directory.join("payload");
    fs::create_dir_all(payload.join("cache")).unwrap();
    fs::write(
        payload.join("target.json"),
        r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
    )
    .unwrap();
    fs::write(payload.join("layout.sh"), "").unwrap();
    fs::write(payload.join("mount.sh"), "").unwrap();
    let step = WriteStick::new(
        image(&directory),
        Some(guide::writer::Payload::open(&payload).unwrap()),
        vec![],
    );
    let asked = |stand_in: &mut StandIn| match step.observe(stand_in).unwrap() {
        Standing::Needed { asks } => asks,
        standing => panic!("{standing:?}"),
    };
    let asks = asked(&mut stand_in);
    let [
        Ask::Confirm {
            verb: "write it", ..
        },
        Ask::Show { how, shown, .. },
        Ask::Recall {
            id: "passphrase",
            expected,
            ..
        },
    ] = asks.as_slice()
    else {
        panic!("{asks:#?}");
    };
    assert_eq!(shown.split(' ').count(), 7, "{shown}");
    assert_eq!(expected, shown);
    assert!(how.contains("\"NixOS Install Phrase\""), "{how}");
    // Looked at again, it's the same words, or the phone would keep
    // words the vault isn't sealed to.
    assert!(matches!(
        asked(&mut stand_in).as_slice(),
        [_, Ask::Show { shown: again, .. }, _] if again == shown
    ));

    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    answers.give("shown", Answer::Done);
    answers.give(
        "passphrase",
        Answer::Typed("abacus abdomen abdominal abide abiding ability ablaze".into()),
    );
    let refused = step.run(&mut stand_in, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the stick's passphrase")),
        "{refused:?}"
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("opening")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn the_last_card_says_how_to_reach_the_machines_firmware() {
    let directory = Scratch::new("writer-card");
    let surface = payload(
        &directory,
        r#"{"machine":"laptop","disk":"/dev/x","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":["the Nano"],"recipients":["age1fido2-hmac1x"],"start_from_stick":"Hold volume-down, press and release power.","clear_secure_boot_keys":"Hold volume-up; set Secure Boot to None."}"#,
    );
    // Written on another machine, or one with no name.
    assert_eq!(surface.next(Some("desktop")), surface.next(None));
    let said = surface.next(None);
    // Carrying no keys, the firmware is cleared first, then started.
    let clear = said.find("Hold volume-up").expect("how to clear the keys");
    let start = said
        .find("Hold volume-down")
        .expect("how to start from the stick");
    assert!(clear < start, "{said}");
    assert!(
        said.starts_with("Take the stick to laptop, switched off, and plug it in."),
        "{said}"
    );
    // Nothing on the disk is let start between the two.
    let off = said
        .find("hold the power button until laptop is off")
        .expect("switched off");
    assert!(clear < off && off < start, "{said}");
    // What comes first there, in order, as a person meets it: a stick
    // started with no word asked of them was taken for a fault.
    assert!(
        said.ends_with("The guide carries on there. First it copies itself into memory, which takes a few minutes and asks nothing. Then it asks you to type a word to check the keyboard, and then for the stick's passphrase, the seven words you saved on your phone."),
        "{said}"
    );

    // Carrying them, nothing is cleared.
    let keys = || {
        let files = guide::rescue::KEYS
            .iter()
            .map(|name| (name.to_string(), zeroize::Zeroizing::new(vec![])))
            .collect();
        guide::rescue::Keys::new(files).unwrap()
    };
    let other = payload(
        &directory,
        r#"{"machine":"laptop","disk":"/dev/x","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[],"recipients":[],"start_from_stick":null,"clear_secure_boot_keys":null}"#,
    );
    // Another machine's keys aren't carried.
    assert!(other.with_secure_boot_keys("desktop", keys()).is_err());
    let carrying = surface.with_secure_boot_keys("laptop", keys()).unwrap();
    assert!(
        !carrying.next(None).contains("Hold volume-up"),
        "{}",
        carrying.next(None)
    );

    // Written on the machine it's for, it's kept for when that machine
    // needs it, never started from at once, though it still says how.
    let own = carrying.next(Some("laptop\n"));
    assert!(
        own.starts_with("Keep this stick away from laptop until you need it. When laptop won't start, or needs installing again, switch it off and plug the stick in."),
        "{own}"
    );
    assert!(!own.contains("Take the stick to"), "{own}");
    assert!(
        own.contains("Then start it from the stick: Hold volume-down"),
        "{own}"
    );
}
