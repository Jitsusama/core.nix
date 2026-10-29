//! Writing a stick on a host, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/), whose stick is `/dev/sda` on USB:
//! only ever the one stick plugged in, what's mounted from it handed over
//! to be unmounted, and a stick that sfdisk reads back whole.

mod common;

use common::Scratch;
use std::cell::Cell;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

use guide::fake::Fake;
use guide::host::carried::{self, Carried};
use guide::host::{ChoosePhrase, GatherBundles, WriteStick};
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
    let mut bytes = vec![0u8; usize::try_from(sectors * SECTOR).unwrap()];
    for (at, rendered) in table.render(sectors) {
        let at = usize::try_from(at).unwrap();
        bytes[at..at + rendered.len()].copy_from_slice(&rendered);
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
    WriteStick::new(image(directory), None, vec![])
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
    // The screen was told what the write was doing, last reading it back.
    let said = guide::Machine::meter(&mut fake).said();
    assert!(said.starts_with("Reading it back to check it: "), "{said}");

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
    fs::write(payload.join("mount.sh"), "").unwrap();
    let step = WriteStick::new(
        image(&directory),
        Some(guide::host::Payload::open(&payload).unwrap()),
        vec![],
    );
    let asked = |fake: &mut Fake| match step.observe(fake).unwrap() {
        Standing::Needed { asks } => asks,
        standing => panic!("{standing:?}"),
    };
    let asks = asked(&mut fake);
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
    fs::write(payload.join("mount.sh"), "").unwrap();
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
        !carrying.next().contains("Hold volume-up"),
        "{}",
        carrying.next()
    );
}

/// The stick's passphrase as its vault was sealed to it before.
const PHRASE: &str = "abacus abdomen abdominal abide abiding ability ablaze";

/// The copy of a vault's key that `PHRASE` opens.
fn sealed_to_phrase() -> Vec<u8> {
    guide::vault::VaultKey::generate()
        .seal_with_passphrase(&guide::Secret::new(PHRASE.into()), Some(1))
        .unwrap()
}

/// The copy of its key the vault written onto `stick` keeps for its
/// passphrase, read straight off it.
fn phrase_on_stick(stick: &Path) -> Vec<u8> {
    let mut disk = File::open(stick).unwrap();
    let mut start = vec![0u8; 1 << 20];
    std::io::Read::read_exact(&mut disk, &mut start).unwrap();
    let vault = Table::read(&start)
        .unwrap()
        .partitions()
        .into_iter()
        .find(|partition| partition.name == "vault")
        .unwrap();
    let mut region =
        guide::stick::blocks::Partition::new(&mut disk, vault.offset(), vault.bytes()).unwrap();
    let mut fat = guide::vault::store::Fat::open(&mut region).unwrap();
    let mut sealed = vec![];
    guide::vault::store::Store::open(&mut fat, guide::vault::KEY_PASSPHRASE)
        .unwrap()
        .read_to_end(&mut sealed)
        .unwrap();
    sealed
}

#[test]
fn a_stick_that_has_a_passphrase_asks_whether_to_keep_it_and_a_new_one_doesnt() {
    let mut fake = Fake::new();
    let keep = Rc::new(Cell::new(None));
    let choose = ChoosePhrase {
        sealed: Some(sealed_to_phrase()),
        keep: Rc::clone(&keep),
    };
    let Standing::Needed { asks } = choose.observe(&mut fake).unwrap() else {
        panic!("the stick's vault has a passphrase")
    };
    let [
        Ask::Choose {
            question, choices, ..
        },
    ] = asks.as_slice()
    else {
        panic!("{asks:#?}")
    };
    assert!(
        question.contains("named \"NixOS Install Phrase\""),
        "{question}"
    );
    assert!(choices[0].starts_with("Keep it"), "{choices:?}");
    let mut answers = Answers::new();
    answers.give("phrase", Answer::Chosen(0));
    Journey::attempt(&choose, &mut fake, &answers).unwrap();
    assert_eq!(keep.get(), Some(true));

    // A stick the guide never wrote has no passphrase to keep.
    let fresh = ChoosePhrase {
        sealed: None,
        keep: Rc::new(Cell::new(None)),
    };
    assert!(matches!(
        fresh.observe(&mut fake).unwrap(),
        Standing::Done { evidence } if evidence.contains("new words")
    ));
}

#[test]
fn new_words_chosen_for_a_stick_that_had_a_passphrase_are_shown_for_the_phone() {
    let directory = Scratch::new("host-new-phrase");
    let mut fake = Fake::new();
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    fake.disk("/dev/sda", &stick_file(&directory));
    let write = WriteStick::new(
        image(&directory),
        Some(payload(
            &directory,
            r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
        )),
        vec![],
    );
    let steps = guide::host::writing(write, Some(sealed_to_phrase()));
    let [choose, write] = steps.as_slice() else {
        panic!("asked whether to keep it, then written")
    };
    let mut answers = Answers::new();
    answers.give("phrase", Answer::Chosen(1));
    Journey::attempt(choose.as_ref(), &mut fake, &answers).unwrap();
    let Standing::Needed { asks } = write.observe(&mut fake).unwrap() else {
        panic!("the stick isn't written yet")
    };
    assert!(
        matches!(
            asks.as_slice(),
            [Ask::Confirm { .. }, Ask::Show { .. }, Ask::Recall { .. }]
        ),
        "{asks:#?}"
    );
}

#[test]
fn a_kept_passphrase_is_proven_against_the_old_vault_and_seals_the_new_one() {
    let directory = Scratch::new("host-keeps-phrase");
    let stick = stick_file(&directory);
    let mut fake = Fake::new();
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    fake.disk("/dev/sda", &stick);
    let step = WriteStick::new(
        image(&directory),
        Some(payload(
            &directory,
            r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
        )),
        vec![],
    )
    .keeping_phrase(sealed_to_phrase(), Rc::new(Cell::new(Some(true))));
    let Standing::Needed { asks } = step.observe(&mut fake).unwrap() else {
        panic!("the stick isn't written yet")
    };
    let [
        Ask::Confirm { .. },
        Ask::Secret {
            id: "passphrase",
            name,
        },
    ] = asks.as_slice()
    else {
        panic!("no new words shown: {asks:#?}")
    };
    assert!(name.contains("named \"NixOS Install Phrase\""), "{name}");

    let typed = |words: &str| {
        let mut answers = Answers::new();
        answers.give("stick", Answer::Done);
        answers.give(
            "passphrase",
            Answer::Secret(guide::Secret::new(words.into())),
        );
        answers
    };
    let refused = step.run(
        &mut fake,
        &typed("abacus abdomen abdominal abide abiding ability"),
    );
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the stick's passphrase")),
        "{refused:?}"
    );
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("opening")),
        "{:#?}",
        fake.calls
    );

    // Typed as a phone's Live Text gives it, capitals and all.
    Journey::attempt(
        &step,
        &mut fake,
        &typed("Abacus abdomen  abdominal abide abiding ability ablaze"),
    )
    .unwrap();
    guide::vault::VaultKey::open_with_passphrase(
        &phrase_on_stick(&stick),
        &guide::Secret::new(PHRASE.into()),
    )
    .unwrap();
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

/// The host's clock, in seconds since 1970, as the tests have it.
const NOW: u64 = 1_000_000;

#[test]
fn a_host_keeps_each_machines_newest_bundle_and_never_an_older_one() {
    let directory = Scratch::new("host-keeps-newest");
    let mut store = Directory::new(directory.join("rescue"));
    carried::keep(
        &mut store,
        None,
        vec![carried("laptop", Some(100)), carried("desktop", None)],
        NOW,
    )
    .unwrap();
    let kept = carried::keep(
        &mut store,
        None,
        vec![
            carried("laptop", Some(200)),
            carried("laptop", Some(90)),
            carried("laptop", None),
            carried("desktop", Some(5)),
        ],
        NOW,
    )
    .unwrap();
    assert_eq!(
        kept,
        vec![carried("desktop", Some(5)), carried("laptop", Some(200))]
    );
    // Nothing older replaces it later, either, nor one from before
    // bundles said when they were sealed.
    for older in [Some(150), None] {
        let kept = carried::keep(&mut store, None, vec![carried("laptop", older)], NOW).unwrap();
        assert_eq!(kept[1], carried("laptop", Some(200)));
    }
    // Nor anything but whole bundles is left beside them.
    let mut names: Vec<_> = fs::read_dir(directory.join("rescue"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["desktop-rescue-bundle.json", "laptop-rescue-bundle.json"]
    );
}

#[test]
fn the_hosts_own_machines_bundle_is_kept_over_any_a_stick_carries_for_it() {
    let directory = Scratch::new("host-keeps-own");
    let mut store = Directory::new(directory.join("rescue"));
    carried::keep(&mut store, None, vec![carried("laptop", Some(500))], NOW).unwrap();
    // Its own, from before bundles said when they were sealed, against a
    // stick's that says it's newer.
    let own = carried("laptop", None);
    let kept = carried::keep(
        &mut store,
        Some(own.clone()),
        vec![carried("laptop", Some(900))],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![own.clone()]);
    // And it stays, however new a stick's says it is.
    let kept = carried::keep(&mut store, None, vec![carried("laptop", Some(NOW))], NOW).unwrap();
    assert_eq!(kept, vec![carried("laptop", Some(NOW))]);
    let kept = carried::keep(
        &mut store,
        Some(own.clone()),
        vec![carried("laptop", Some(NOW))],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![own]);
}

#[test]
fn a_bundle_said_to_be_sealed_more_than_a_day_ahead_is_never_kept() {
    let directory = Scratch::new("host-keeps-ahead");
    let mut store = Directory::new(directory.join("rescue"));
    let kept = carried::keep(
        &mut store,
        None,
        vec![
            carried("laptop", Some(NOW + 60 * 60)),
            carried("desktop", Some(NOW + 2 * 24 * 60 * 60)),
            carried("laptop", Some(u64::MAX)),
        ],
        NOW,
    )
    .unwrap();
    assert_eq!(kept, vec![carried("laptop", Some(NOW + 60 * 60))]);
}

#[test]
fn a_host_keeps_bundles_for_only_so_many_machines_each_named_plainly() {
    let directory = Scratch::new("host-keeps-few");
    let mut store = Directory::new(directory.join("rescue"));
    let found: Vec<Carried> = (0..carried::MACHINES + 8)
        .map(|n| carried(&format!("machine-{n:02}"), Some(1)))
        .collect();
    let kept = carried::keep(&mut store, None, found, NOW).unwrap();
    assert_eq!(kept.len(), carried::MACHINES);
    assert_eq!(
        fs::read_dir(directory.join("rescue")).unwrap().count(),
        carried::MACHINES
    );
    // Names FAT would mix up, or couldn't hold, aren't bundles' names.
    for machine in ["Laptop", "lap/top", "-laptop", "", &"a".repeat(64)] {
        assert_eq!(
            Carried::read(bundle(machine, Some(1)), None),
            None,
            "{machine}"
        );
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
    // Only the stick whose bundles were kept is written over.
    let standing = step.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("isn't the stick whose rescue bundles were kept")),
        "{standing:?}"
    );
    fake.no_file(guide::rescue::KEPT);
    Journey::attempt(
        &GatherBundles::new(directory.join("first-host")),
        &mut fake,
        &Answers::new(),
    )
    .unwrap();
    Journey::attempt(&step, &mut fake, &answers).unwrap();
    let on_it = carried::on_stick(&mut File::open(&stick).unwrap());
    assert_eq!(
        on_it,
        vec![carried("desktop", Some(3)), carried("laptop", Some(7))]
    );

    // Another host, about to write over it, keeps them first, and its
    // own machine's beside them, the vault mounted as the stick holds it.
    let mounted = directory.join("mounted");
    carried::keep(&mut Directory::new(&mounted), None, on_it, NOW).unwrap();
    let mut host = Fake::new();
    lsblk(&mut host, fixture!("lsblk-laid-out.json"));
    host.mount("/dev/sda4", &mounted)
        .file(guide::rescue::KEPT, &bundle("tower", Some(9)));
    let gather = GatherBundles::new(directory.join("host"));
    assert!(matches!(
        gather.observe(&mut host).unwrap(),
        Standing::Needed { .. }
    ));
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    // Mounted only for reading, which needs no password, and never the
    // disk opened, which would.
    let mounting: Vec<&String> = host
        .calls
        .iter()
        .filter(|call| call.contains("mounting") || call.starts_with("opening"))
        .collect();
    assert_eq!(
        mounting,
        ["mounting /dev/sda4 for reading", "unmounting /dev/sda4"],
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
    fs::write(mounted.join(guide::vault::KEY_PASSPHRASE), b"sealed").unwrap();
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
    let gather = GatherBundles::new(directory.join("host"));
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    assert!(
        !host
            .calls
            .iter()
            .any(|call| call.starts_with("opening") || call.contains("mounting")),
        "{:#?}",
        host.calls
    );
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("host"))),
        vec![carried("laptop", Some(4))]
    );
    // The copy of its key the stick's passphrase opens, kept for the
    // write to prove a passphrase kept against.
    assert_eq!(
        gather.phrase.borrow().as_deref(),
        Some(b"sealed".as_slice())
    );
}

#[test]
fn a_vault_that_wont_mount_carries_nothing_and_stops_nothing() {
    let directory = Scratch::new("host-gathers-unread");
    let mut host = Fake::new();
    lsblk(&mut host, fixture!("lsblk-laid-out.json"));
    // Nothing scripted for mounting it, so it fails as a vault that
    // isn't one would.
    host.no_file(guide::rescue::KEPT);
    let gather = GatherBundles::new(directory.join("host"));
    Journey::attempt(&gather, &mut host, &Answers::new()).unwrap();
    assert_eq!(
        carried::kept(&mut Directory::new(directory.join("host"))),
        vec![]
    );
    // Which it says, so the person knows the stick's copies are gone.
    let standing = gather.observe(&mut host).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("its vault couldn't be read")),
        "{standing:?}"
    );
}
