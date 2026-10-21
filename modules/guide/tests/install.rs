//! The install's rules that guard a disk, held against what the tools
//! printed in the guide's VM (fixtures/qemu-canokey/): nothing is wiped
//! until the machine's name is typed back, never a removable disk, no key
//! enrolled twice, and the install's key comes off only once every
//! YubiKey and the recovery key can open the disk.

mod common;

use common::Scratch;
use std::path::PathBuf;

use guide::fake::Fake;
use guide::install::disk::{ConfirmDisk, LayOutDisk, WipeInstallKey};
use guide::install::keys::EnrolYubikey;
use guide::install::{INSTALL_KEY, TARGET, Target, VAULT_KEY};
use guide::vault::store::Directory;
use guide::vault::{Manifest, Packer, VaultKey};
use guide::{Answer, Answers, Fault, Journey, Program, Secret, Standing, Step, Tool};

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

fn target(disk: &str) -> Target {
    Target {
        machine: "machine".into(),
        disk: disk.into(),
        disk_name: "system".into(),
        install_key: INSTALL_KEY.into(),
        layout_packages: vec!["cryptsetup".into()],
        system: "/nix/store/abc-nixos-system-machine".into(),
        account: "alice".into(),
        yubikeys: vec!["the Nano".into(), "the 5C".into()],
    }
}

/// A fake machine on the stick, its vault open and holding this target.
fn stick(directory: &std::path::Path, target: &Target) -> Fake {
    let mut store = Directory::new(PathBuf::from(&directory));
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer
        .add(TARGET, serde_json::to_vec(target).unwrap().as_slice())
        .unwrap();
    packer
        .finish(&Manifest {
            format: 1,
            machine: target.machine.clone(),
            opened_by: vec![],
        })
        .unwrap();
    let mut fake = Fake::new();
    fake.with_vault(Box::new(store))
        .file(VAULT_KEY, key.to_secret().expose().as_bytes())
        .no_file(INSTALL_KEY)
        .link("/dev/disk/by-id/virtio-root", Some("/dev/vda"))
        .link("/dev/disk/by-id/usb-stick", Some("/dev/sda"));
    fake
}

fn lsblk(fake: &mut Fake, json: &str) {
    fake.prints(Tool::Lsblk, &["--json", "--output-all", "--bytes"], json);
}

fn typed(text: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("machine", Answer::Typed(text.into()));
    answers
}

fn wiped(fake: &Fake) -> bool {
    fake.calls
        .iter()
        .any(|call| call.starts_with("wipefs") || call.starts_with("sgdisk"))
}

#[test]
fn the_disk_is_named_and_the_machine_typed_back_before_anything_is_wiped() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-confirm");
    let mut fake = stick(&scratch, &target);
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    let Standing::Needed { asks } = ConfirmDisk.observe(&mut fake).unwrap() else {
        panic!("the disk is still to confirm")
    };
    assert!(
        format!("{asks:?}").contains("expected: \"machine\""),
        "{asks:?}"
    );

    let refused = Journey::attempt(&ConfirmDisk, &mut fake, &typed("machin"));
    assert!(matches!(refused, Err(Fault::Blocked { .. })), "{refused:?}");
    // Run itself, not only observing first, refuses a disk nobody confirmed.
    let laid_out = LayOutDisk.run(&mut fake, &Answers::new());
    assert!(
        matches!(&laid_out, Err(Fault::Blocked { why, .. }) if why.contains("hasn't been confirmed")),
        "{laid_out:?}"
    );
    assert!(!wiped(&fake), "{:#?}", fake.calls);

    Journey::attempt(&ConfirmDisk, &mut fake, &typed("machine")).unwrap();
}

#[test]
fn a_removable_disk_is_never_the_one_installed_onto() {
    // The stick itself, which lsblk says is on USB.
    let target = target("/dev/disk/by-id/usb-stick");
    let scratch = Scratch::new("install-removable");
    let mut fake = stick(&scratch, &target);
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    let standing = ConfirmDisk.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("removable")),
        "{standing:?}"
    );
}

#[test]
fn a_disk_laid_out_and_open_at_the_root_stays_done() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-laid-out");
    let mut fake = stick(&scratch, &target);
    lsblk(&mut fake, fixture!("lsblk-laid-out.json"));
    fake.link("/dev/disk/by-partlabel/system", Some("/dev/vda2"))
        .link("/dev/mapper/system", Some("/dev/dm-1"))
        .answers(
            Program::Tool(Tool::Cryptsetup),
            &["isLuks", "/dev/vda2"],
            0,
            "",
            "",
        );
    assert!(matches!(
        LayOutDisk.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_key_plugged_in_beside_another_is_never_enrolled() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-two-keys");
    let mut fake = stick(&scratch, &target);
    let one = fixture!("fido2-token-L.txt");
    fake.prints(
        Tool::Fido2Token,
        &["-L"],
        &format!("{one}{}", one.replace("hidraw1", "hidraw2")),
    )
    .prints(
        Tool::Cryptsetup,
        &[
            "luksDump",
            "--dump-json-metadata",
            "/dev/disk/by-partlabel/system",
        ],
        fixture!("luks-dump-before-wipe.json"),
    );
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    let nano = EnrolYubikey::new("the Nano");
    let refused = nano.run(&mut fake, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("2 keys are plugged in")),
        "{refused:?}"
    );
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn a_key_that_opens_the_disk_already_isnt_enrolled_again() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-enrolled");
    let mut fake = stick(&scratch, &target);
    let header = fixture!("luks-dump-before-wipe.json");
    fake.prints(Tool::Fido2Token, &["-L"], fixture!("fido2-token-L.txt"))
        .prints(
            Tool::Cryptsetup,
            &[
                "luksDump",
                "--dump-json-metadata",
                "/dev/disk/by-partlabel/system",
            ],
            header,
        );
    // The plugged key answers a silent assertion for the first credential.
    fake.answers(
        Program::Tool(Tool::Fido2Assert),
        &["-G", "-t", "up=false", "/dev/hidraw1"],
        0,
        "",
        "",
    );
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    let refused = EnrolYubikey::new("the 5C").run(&mut fake, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the 5C")),
        "{refused:?}"
    );
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        fake.calls
    );
    // The PIN never went near the assertion, which asks nothing secret.
    assert!(
        fake.secrets
            .iter()
            .all(|(_, value)| !value.contains("123456")),
        "{:?}",
        fake.secrets
    );
}

#[test]
fn the_install_key_comes_off_only_once_every_key_and_the_recovery_key_open_the_disk() {
    let target = target("/dev/disk/by-id/virtio-root");
    let dump = [
        "luksDump",
        "--dump-json-metadata",
        "/dev/disk/by-partlabel/system",
    ];

    // Three YubiKeys named, two enrolled: it waits.
    let scratch = Scratch::new("install-too-few");
    let mut fake = stick(&scratch, &target);
    fake.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-before-wipe.json"),
    );
    assert!(matches!(
        WipeInstallKey { yubikeys: 3 }.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));

    // Both keys enrolled but no recovery key: it waits, or nothing would
    // open the disk without a YubiKey. The real header, its recovery token
    // taken out.
    let mut header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-before-wipe.json")).unwrap();
    header["tokens"]
        .as_object_mut()
        .unwrap()
        .retain(|_, token| token["type"] != "systemd-recovery");
    let scratch = Scratch::new("install-no-recovery");
    let mut fake = stick(&scratch, &target);
    fake.prints(Tool::Cryptsetup, &dump, &header.to_string());
    fake.file(INSTALL_KEY, b"0123");
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));

    // Both enrolled, the key's slot still there: it's next.
    let scratch = Scratch::new("install-ready");
    let mut fake = stick(&scratch, &target);
    fake.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-before-wipe.json"),
    );
    fake.file(INSTALL_KEY, b"0123");
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));

    // Every slot a token's, and the key forgotten: done.
    let scratch = Scratch::new("install-wiped");
    let mut fake = stick(&scratch, &target);
    fake.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-installed.json"),
    );
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}
