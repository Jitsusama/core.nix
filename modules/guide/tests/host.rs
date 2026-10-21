//! Writing a stick on a host, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/), whose stick is `/dev/sda` on USB:
//! only ever the one stick plugged in, what's mounted from it handed over
//! to be unmounted, and a stick that sfdisk reads back whole.

mod common;

use common::Scratch;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use guide::fake::Fake;
use guide::host::WriteStick;
use guide::stick::gpt::{SECTOR, Table};
use guide::{Answer, Answers, Fault, Journey, Secret, Standing, Step, Tool};

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

/// An image laid out as core's is, 8 MiB, its three partitions filled.
fn image(directory: &Path) -> PathBuf {
    let parts: [(&str, u8, u64, u64); 3] = [
        ("esp", 0x11, 2048, 4095),
        ("store-verity", 0x22, 4096, 4999),
        ("store", 0x33, 5000, 14000),
    ];
    let mut slots = vec![[0u8; 128]; 128];
    for (slot, (name, fill, first, last)) in parts.iter().enumerate() {
        let entry = &mut slots[slot];
        entry[..16].copy_from_slice(&[*fill; 16]);
        entry[16..32].copy_from_slice(&[fill + 1; 16]);
        entry[32..40].copy_from_slice(&first.to_le_bytes());
        entry[40..48].copy_from_slice(&last.to_le_bytes());
        for (unit, at) in name.encode_utf16().zip((56..).step_by(2)) {
            entry[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }
    let table = Table {
        disk_guid: [0x5a; 16],
        slots,
    };
    let sectors = 16384;
    let mut bytes = vec![0u8; (sectors * SECTOR) as usize];
    for (at, rendered) in table.render(sectors) {
        bytes[at as usize..at as usize + rendered.len()].copy_from_slice(&rendered);
    }
    let path = directory.join("image.raw");
    fs::write(&path, bytes).unwrap();
    path
}

/// A used stick of 100 MiB, standing in for the device.
fn stick_file(directory: &Path) -> PathBuf {
    let path = directory.join("stick.raw");
    let mut file = File::create(&path).unwrap();
    file.write_all(&vec![0xee; 100 << 20]).unwrap();
    path
}

/// lsblk's listing with only the disks this keeps.
fn listing(json: &str, keep: impl Fn(&str) -> bool) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    value["blockdevices"]
        .as_array_mut()
        .unwrap()
        .retain(|device| keep(device["name"].as_str().unwrap()));
    value.to_string()
}

fn repair(directory: &Path) -> WriteStick {
    WriteStick {
        image: image(directory),
        payload: None,
        signer: None,
    }
}

fn lsblk(fake: &mut Fake, json: &str) {
    fake.prints(Tool::Lsblk, &["--json", "--output-all", "--bytes"], json);
}

#[test]
fn with_no_stick_plugged_in_it_waits_for_one() {
    let directory = Scratch::new("none");
    let mut fake = Fake::new();
    lsblk(
        &mut fake,
        &listing(fixture!("lsblk-blank.json"), |name| name != "sda"),
    );
    let standing = repair(&directory).observe(&mut fake).unwrap();
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
    let mut fake = Fake::new();
    lsblk(&mut fake, &value.to_string());
    let step = repair(&directory);
    let standing = step.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.starts_with("2 removable disks")),
        "{standing:?}"
    );
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    assert!(step.run(&mut fake, &answers).is_err());
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("opening")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn the_one_stick_is_written_whole_and_what_was_mounted_from_it_is_handed_over() {
    let directory = Scratch::new("written");
    let stick = stick_file(&directory);
    let mut fake = Fake::new();
    // The stick's own partitions are mounted here, as the VM's booted
    // stick has them.
    lsblk(&mut fake, fixture!("lsblk-laid-out.json"));
    fake.disk("/dev/sda", &stick);
    let step = repair(&directory);
    let Standing::Needed { asks } = step.observe(&mut fake).unwrap() else {
        panic!("the stick is still to write")
    };
    assert_eq!(
        asks.len(),
        1,
        "a repair stick asks for nothing secret: {asks:?}"
    );
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    Journey::attempt(&step, &mut fake, &answers).unwrap();

    let opened = fake
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
        step.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn passphrases_that_differ_open_nothing() {
    let directory = Scratch::new("passphrases");
    let mut fake = Fake::new();
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    fake.disk("/dev/sda", &stick_file(&directory));
    // A payload of nothing but its target, which is all this reads first.
    let payload = directory.join("payload");
    fs::create_dir_all(payload.join("cache")).unwrap();
    fs::write(
        payload.join("target.json"),
        r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
    )
    .unwrap();
    fs::write(payload.join("layout.sh"), "").unwrap();
    let step = WriteStick {
        image: image(&directory),
        payload: Some(guide::host::Payload::open(&payload).unwrap()),
        signer: None,
    };
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    answers.give(
        "passphrase",
        Answer::Secret(Secret::new("one long passphrase".into())),
    );
    answers.give(
        "again",
        Answer::Secret(Secret::new("another long passphrase".into())),
    );
    let refused = step.run(&mut fake, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("weren't the same")),
        "{refused:?}"
    );
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("opening")),
        "{:#?}",
        fake.calls
    );
}

/// A payload of nothing but its target, which is all the card reads.
fn payload(directory: &Path, target: &str) -> guide::host::Payload {
    let payload = directory.join("payload");
    fs::create_dir_all(payload.join("cache")).unwrap();
    fs::write(payload.join("target.json"), target).unwrap();
    fs::write(payload.join("layout.sh"), "").unwrap();
    guide::host::Payload::open(&payload).unwrap()
}

#[test]
fn the_last_card_says_how_to_reach_the_machines_firmware() {
    let directory = Scratch::new("host-card");
    let surface = payload(
        &directory,
        r#"{"machine":"laptop","disk":"/dev/x","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":["the Nano"],"recipients":["age1fido2-hmac1x"],"start_from_stick":"Hold volume-down, press and release power.","clear_secure_boot_keys":"Hold volume-up; set Secure Boot to None."}"#,
    );
    let said = surface.next();
    // Carrying no keys, the firmware is cleared first, then started.
    let clear = said.find("Hold volume-up").expect("how to clear the keys");
    let start = said
        .find("Hold volume-down")
        .expect("how to start from the stick");
    assert!(clear < start, "{said}");
    assert!(
        said.ends_with("asks for a YubiKey's PIN, or the recovery passphrase first."),
        "{said}"
    );

    // Carrying them, nothing is cleared.
    let keys = directory.join("sbctl");
    fs::create_dir_all(keys.join("keys/db")).unwrap();
    for file in ["GUID", "keys/db/db.key", "keys/db/db.pem"] {
        fs::write(keys.join(file), "").unwrap();
    }
    let carrying = surface.with_secure_boot_keys(&keys).unwrap();
    assert!(
        !carrying.next().contains("Hold volume-up"),
        "{}",
        carrying.next()
    );
}
