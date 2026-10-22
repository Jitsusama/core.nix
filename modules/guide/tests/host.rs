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
use guide::host::carried::{self, Carried};
use guide::host::{GatherBundles, WriteStick};
use guide::rescue::Bundle;
use guide::stick::gpt::{SECTOR, Table};
use guide::vault::store::Directory;
use guide::{Answer, Answers, Ask, Fault, Journey, Standing, Step, Tool};

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
    WriteStick::new(image(directory), None, None)
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
fn the_sticks_passphrase_is_seven_words_shown_for_the_phone_and_typed_back() {
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
    let step = WriteStick::new(
        image(&directory),
        Some(guide::host::Payload::open(&payload).unwrap()),
        None,
    );
    let asked = |fake: &mut Fake| match step.observe(fake).unwrap() {
        Standing::Needed { asks } => asks,
        standing => panic!("{standing:?}"),
    };
    let asks = asked(&mut fake);
    let [
        Ask::Act { .. },
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
    assert!(how.contains("\"laptop stick passphrase\""), "{how}");
    // Looked at again, it's the same words, or the phone would keep
    // words the vault isn't sealed to.
    assert!(matches!(
        asked(&mut fake).as_slice(),
        [_, Ask::Show { shown: again, .. }, _] if again == shown
    ));

    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    answers.give("shown", Answer::Done);
    answers.give(
        "passphrase",
        Answer::Typed("abacus abdomen abdominal abide abiding ability ablaze".into()),
    );
    let refused = step.run(&mut fake, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the stick's passphrase")),
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
        !carrying.next().contains("Hold volume-up"),
        "{}",
        carrying.next()
    );
}

/// A bundle as a machine's install seals it, as far as a host reads one
/// it carries on: which machine's it is, and when it was sealed.
fn bundle(machine: &str, sealed: Option<u64>) -> Vec<u8> {
    let sealed = sealed
        .map(|at| format!(r#","sealed":{at}"#))
        .unwrap_or_default();
    format!(
        r#"{{"format":1,"machine":"{machine}","opened_by":["the Nano"],"owner":"22f0175f-c166-49d2-96ab-c61575cddac5","db":"ab","to_keys":"a","to_passphrase":"b","keys":"c"{sealed}}}"#
    )
    .into_bytes()
}

fn carried(machine: &str, sealed: Option<u64>) -> Carried {
    Carried::read(bundle(machine, sealed), None).unwrap()
}

#[test]
fn a_host_keeps_each_machines_newest_bundle_and_never_an_older_one() {
    let directory = Scratch::new("host-keeps-newest");
    let mut store = Directory::new(directory.join("rescue"));
    carried::keep(
        &mut store,
        vec![carried("laptop", Some(100)), carried("desktop", None)],
    )
    .unwrap();
    let kept = carried::keep(
        &mut store,
        vec![
            carried("laptop", Some(200)),
            carried("laptop", Some(90)),
            carried("laptop", None),
            carried("desktop", Some(5)),
        ],
    )
    .unwrap();
    assert_eq!(
        kept,
        vec![carried("desktop", Some(5)), carried("laptop", Some(200))]
    );
    // Nothing older replaces it later, either, nor one from before
    // bundles said when they were sealed.
    for older in [Some(150), None] {
        let kept = carried::keep(&mut store, vec![carried("laptop", older)]).unwrap();
        assert_eq!(kept[1], carried("laptop", Some(200)));
    }
}

#[test]
fn a_stick_for_a_machine_whose_bundle_the_host_keeps_is_signed_from_it_unless_another_is_given() {
    let kept = vec![carried("desktop", Some(3)), carried("laptop", Some(7))];
    let chosen = |given: Option<&str>, machine: Option<&str>| {
        carried::bundle_for(
            given.map(|machine| Bundle::read(&bundle(machine, Some(1))).unwrap()),
            machine,
            &kept,
        )
        .unwrap()
        .map(|bundle| (bundle.machine().to_string(), bundle.sealed()))
    };
    assert_eq!(chosen(None, Some("laptop")), Some(("laptop".into(), 7)));
    assert_eq!(
        chosen(Some("laptop"), Some("laptop")),
        Some(("laptop".into(), 1))
    );
    assert_eq!(chosen(None, Some("tower")), None);
    // A repair stick is for no machine, so none of the host's is used.
    assert_eq!(chosen(None, None), None);
}

#[test]
fn a_bundle_kept_under_another_machines_name_or_none_is_left_unread() {
    let directory = Scratch::new("host-keeps-named");
    let root = directory.join("rescue");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("desktop-rescue-bundle.json"),
        bundle("laptop", Some(1)),
    )
    .unwrap();
    fs::write(root.join("laptop-rescue-bundle.json"), b"not a bundle").unwrap();
    fs::write(root.join("notes.json"), bundle("laptop", Some(1))).unwrap();
    assert_eq!(carried::kept(&mut Directory::new(&root)), vec![]);
}

#[test]
fn a_stick_carries_every_bundle_on_and_the_next_host_keeps_them_off_it() {
    let directory = Scratch::new("host-carries");
    let stick = stick_file(&directory);
    let mut fake = Fake::new();
    lsblk(&mut fake, fixture!("lsblk-laid-out.json"));
    fake.disk("/dev/sda", &stick);
    let bundles = vec![carried("laptop", Some(7)), carried("desktop", Some(3))];
    let step = repair(&directory).carrying(bundles);
    let mut answers = Answers::new();
    answers.give("stick", Answer::Done);
    Journey::attempt(&step, &mut fake, &answers).unwrap();
    let on_it = carried::on_stick(&mut File::open(&stick).unwrap());
    assert_eq!(
        on_it,
        vec![carried("desktop", Some(3)), carried("laptop", Some(7))]
    );

    // Another host, about to write over it, keeps them first, and its
    // own machine's beside them.
    let mut host = Fake::new();
    lsblk(&mut host, fixture!("lsblk-laid-out.json"));
    host.disk("/dev/sda", &stick)
        .file(guide::rescue::KEPT, &bundle("tower", Some(9)));
    let gather = GatherBundles {
        kept: directory.join("host"),
    };
    assert!(matches!(
        gather.observe(&mut host).unwrap(),
        Standing::Needed { .. }
    ));
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    assert!(
        host.calls
            .iter()
            .any(|call| call == "opening /dev/sda for reading"),
        "{:#?}",
        host.calls
    );
    assert!(
        !host.calls.iter().any(|call| call.contains("for writing")),
        "{:#?}",
        host.calls
    );
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("host"))),
        vec![
            carried("desktop", Some(3)),
            carried("laptop", Some(7)),
            carried("tower", Some(9)),
        ]
    );
    assert!(matches!(
        gather.observe(&mut host).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_vault_a_desktop_mounted_is_read_where_it_is_and_the_disk_never_opened() {
    let directory = Scratch::new("host-gathers-mounted");
    let mounted = directory.join("mounted");
    fs::create_dir_all(&mounted).unwrap();
    fs::write(
        mounted.join("laptop-rescue-bundle.json"),
        bundle("laptop", Some(4)),
    )
    .unwrap();
    let mut seen: serde_json::Value =
        serde_json::from_str(fixture!("lsblk-laid-out.json")).unwrap();
    for disk in seen["blockdevices"].as_array_mut().unwrap() {
        let partitions = disk
            .get_mut("children")
            .and_then(|found| found.as_array_mut());
        for partition in partitions.into_iter().flatten() {
            if partition["path"] == "/dev/sda4" {
                partition["mountpoints"] = serde_json::json!([mounted.to_str().unwrap()]);
            }
        }
    }
    let mut host = Fake::new();
    lsblk(&mut host, &seen.to_string());
    host.no_file(guide::rescue::KEPT);
    let gather = GatherBundles {
        kept: directory.join("host"),
    };
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    assert!(
        !host.calls.iter().any(|call| call.starts_with("opening")),
        "{:#?}",
        host.calls
    );
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("host"))),
        vec![carried("laptop", Some(4))]
    );
}

#[test]
fn a_stick_that_cant_be_read_carries_nothing_and_stops_nothing() {
    let directory = Scratch::new("host-gathers-unread");
    let mut host = Fake::new();
    lsblk(&mut host, fixture!("lsblk-laid-out.json"));
    // A stick the guide never wrote: no table at all.
    host.disk("/dev/sda", &stick_file(&directory))
        .no_file(guide::rescue::KEPT);
    let gather = GatherBundles {
        kept: directory.join("host"),
    };
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("host"))),
        vec![]
    );
}
