//! The install's rules that guard a disk, held against what the tools
//! printed in the guide's VM (fixtures/qemu-canokey/): nothing is wiped
//! until the machine's name is typed back, never a removable disk, no key
//! enrolled twice, and the install's key comes off only once every
//! YubiKey and the recovery key can open the disk.

mod common;

use common::Scratch;
use std::io::Write;
use std::path::PathBuf;

use guide::fake::Fake;
use guide::install::disk::{ConfirmDisk, LayOutDisk, OpenStoppedInstall, WipeInstallKey};
use guide::install::keys::{EnrolYubikey, NAMED, NameYubikeys, Named};
use guide::install::memory::{self, RunFromMemory};
use guide::install::system::{
    self, CheckFirmware, KeepOnStick, ReadyEnrolment, SealRescueBundle, SecureBootKeys,
};
use guide::install::vault::OpenVault;
use guide::install::{
    INSTALL_KEY, MOUNT_SCRIPT, RECOVERY_KEY, STICK_VAULT, SYSTEM_CACHE, Start, TARGET, Target,
    VAULT_KEY,
};
use guide::rescue::{self, Bundle};
use guide::vault::store::{Directory, Store};
use guide::vault::{self, Manifest, Packer, Vault, VaultKey};
use guide::{Answer, Answers, Ask, Fault, Journey, Machine, Program, Secret, Standing, Step, Tool};

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
        recipients: vec![],
        start_from_stick: None,
        clear_secure_boot_keys: None,
    }
}

/// A fake machine on the stick, its vault open and holding this target.
fn stick(directory: &std::path::Path, target: &Target) -> Fake {
    stick_carrying(directory, target, &[])
}

/// The same, its vault holding these files too.
fn stick_carrying(directory: &std::path::Path, target: &Target, files: &[(&str, &[u8])]) -> Fake {
    let mut store = Directory::new(PathBuf::from(&directory));
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer
        .add(TARGET, serde_json::to_vec(target).unwrap().as_slice())
        .unwrap();
    for (path, bytes) in files {
        packer.add(path, *bytes).unwrap();
    }
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

/// These keys plugged in: each canokey as `fido2-token` lists and
/// describes one, and "linux-id" as the XPS 14's virtual key.
fn plugged(fake: &mut Fake, keys: &[(&str, &str)]) {
    let mut listed = String::new();
    for (path, kind) in keys {
        let (line, info) = match *kind {
            "linux-id" => (
                format!("{path}: vendor=0x15d9, product=0x0a37 ( linux-id)\n"),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/fixtures/dell-xps-14-da14260/fido2-token-I.txt"
                )),
            ),
            _ => (
                fixture!("fido2-token-L.txt").replace("/dev/hidraw1", path),
                fixture!("fido2-token-I.txt"),
            ),
        };
        listed.push_str(&line);
        fake.prints(Tool::Fido2Token, &["-I", path], info);
    }
    fake.prints(Tool::Fido2Token, &["-L"], &listed);
}

const PASSPHRASE: &str = "the vault's recovery passphrase";

/// A fake machine on the stick, its vault still shut: its key sealed to
/// `PASSPHRASE`, and a copy for the YubiKeys that nothing here opens, so
/// a test that reaches for a key fails.
fn shut_stick(directory: &std::path::Path, target: &Target) -> Fake {
    let mut fake = stick(directory, target);
    let key = guide::install::vault_key(&mut fake).unwrap().unwrap();
    let mut store = Directory::new(PathBuf::from(directory));
    store
        .create(vault::KEY_PASSPHRASE)
        .unwrap()
        .write_all(
            &key.seal_with_passphrase(&Secret::new(PASSPHRASE.into()), Some(2))
                .unwrap(),
        )
        .unwrap();
    store
        .create(vault::KEY)
        .unwrap()
        .write_all(b"no key opens this")
        .unwrap();
    let manifest = Manifest {
        format: 1,
        machine: target.machine.clone(),
        opened_by: vec![
            "the Nano".into(),
            "the 5C".into(),
            "the recovery passphrase".into(),
        ],
    };
    store
        .create(vault::MANIFEST)
        .unwrap()
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    fake.with_vault(Box::new(store)).no_file(VAULT_KEY);
    fake
}

fn passphrase(text: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("key", Answer::Done);
    answers.give("passphrase", Answer::Secret(Secret::new(text.into())));
    answers
}

#[test]
fn the_passphrase_typed_with_a_key_plugged_in_opens_the_vault_without_the_key() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-passphrase");
    let mut fake = shut_stick(&scratch, &target);
    plugged(&mut fake, &[("/dev/hidraw1", "canokey")]);
    assert!(matches!(
        OpenVault.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    OpenVault.run(&mut fake, &passphrase(PASSPHRASE)).unwrap();
    assert!(matches!(
        OpenVault.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn what_isnt_the_passphrase_goes_to_the_key_plugged_in_and_says_both_were_tried() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-pin");
    let mut fake = shut_stick(&scratch, &target);
    plugged(&mut fake, &[("/dev/hidraw1", "canokey")]);
    let refused = OpenVault.run(&mut fake, &passphrase("123456"));
    assert!(
        matches!(&refused, Err(Fault::Unopened { why, .. }) if why.starts_with("that isn't the recovery passphrase, and ")),
        "{refused:?}"
    );
}

#[test]
fn a_laptops_virtual_key_is_never_counted_as_a_key_plugged_in() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-linux-id");
    let mut fake = shut_stick(&scratch, &target);
    plugged(
        &mut fake,
        &[("/dev/hidraw1", "canokey"), ("/dev/hidraw6", "linux-id")],
    );
    let refused = OpenVault.run(&mut fake, &passphrase("123456"));
    assert!(
        matches!(&refused, Err(Fault::Unopened { why, .. }) if why.starts_with("that isn't")),
        "the canokey alone is plugged in: {refused:?}"
    );
}

#[test]
fn the_vault_asks_on_one_screen_for_the_passphrase_by_its_name_on_the_phone_or_a_key() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-one-screen");
    let mut fake = shut_stick(&scratch, &target);
    let Standing::Needed { asks } = OpenVault.observe(&mut fake).unwrap() else {
        panic!("the vault is shut")
    };
    let [Ask::SecretOr { name, or, .. }] = asks.as_slice() else {
        panic!("one field, saying both ways in: {asks:?}")
    };
    assert!(
        name.contains("1Password on your phone named \"machine stick passphrase\""),
        "{name}"
    );
    assert!(
        or.starts_with("plug in the Nano or the 5C beside the stick, type its PIN"),
        "{or}"
    );
}

#[test]
fn once_the_vault_is_open_the_steps_read_what_it_installs_with_the_stick_out() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-held");
    let mut fake = shut_stick(&scratch, &target);
    OpenVault.run(&mut fake, &passphrase(PASSPHRASE)).unwrap();
    fake.stick_out(true);
    assert_eq!(guide::install::target(&mut fake).unwrap(), target);
    assert!(matches!(
        OpenVault.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
    // What only the vault holds says the stick is out, to be put back.
    assert!(matches!(
        guide::install::vault(&mut fake).err(),
        Some(Fault::StickOut)
    ));
    fake.stick_out(false);
    assert!(guide::install::vault(&mut fake).is_ok());
}

#[test]
fn with_the_stick_out_and_nothing_held_the_target_says_to_put_it_back() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-unheld");
    let mut fake = stick(&scratch, &target);
    fake.no_file(guide::install::HELD_TARGET).stick_out(true);
    assert!(matches!(
        guide::install::target(&mut fake),
        Err(Fault::StickOut)
    ));
}

const STORE_TABLE: &str = "0 16 verity 1 8:3 8:2 4096 4096 2 1 sha256 1f2e3d4c5b6a -";

/// The stick's store as the kernel shows it: two data blocks on sda3, its
/// hashes on sda2, with this much memory free.
fn store_on_the_stick(fake: &mut Fake, available_kilobytes: u64) {
    fake.prints(Tool::Dmsetup, &["table", "usr"], STORE_TABLE)
        .file(
            "/proc/meminfo",
            format!("MemTotal: 16000000 kB\nMemAvailable: {available_kilobytes} kB\n").as_bytes(),
        )
        .file("/sys/dev/block/8:2/size", b"16\n")
        .file(
            "/sys/dev/block/8:3/uevent",
            b"MAJOR=8\nMINOR=3\nDEVNAME=sda3\nDEVTYPE=partition\n",
        )
        .file(
            "/sys/dev/block/8:2/uevent",
            b"MAJOR=8\nMINOR=2\nDEVNAME=sda2\nDEVTYPE=partition\n",
        )
        .file("/proc/mounts", b"tmpfs / tmpfs rw 0 0\n")
        .file("/dev/sda3", &[3u8; 12288])
        .file("/dev/sda2", &[2u8; 8192]);
}

const VERIFY: &[&str] = &[
    "verify",
    "/run/stick/store",
    "/run/stick/store-verity",
    "1f2e3d4c5b6a",
    "--no-superblock",
    "--format=1",
    "--data-block-size=4096",
    "--hash-block-size=4096",
    "--data-blocks=2",
    "--hash-offset=4096",
    "--hash=sha256",
    "--salt=-",
];

fn position(fake: &Fake, call: &str) -> usize {
    fake.calls
        .iter()
        .position(|made| made.starts_with(call))
        .unwrap_or_else(|| panic!("{call} wasn't run: {:#?}", fake.calls))
}

#[test]
fn the_sticks_system_is_copied_into_memory_checked_then_run_from_there() {
    let mut fake = Fake::new();
    store_on_the_stick(&mut fake, 8_000_000);
    fake.prints(
        Tool::Mount,
        &[
            "-t",
            "tmpfs",
            "-o",
            "size=1024k,mode=0700,X-mount.mkdir=0700",
            "stick",
            "/run/stick",
        ],
        "",
    )
    .prints(Tool::Veritysetup, VERIFY, "")
    .prints(Tool::Losetup, &["--associated", "/run/stick/store"], "")
    .prints(
        Tool::Losetup,
        &["--associated", "/run/stick/store-verity"],
        "",
    )
    .prints(
        Tool::Losetup,
        &["--find", "--show", "--read-only", "/run/stick/store"],
        "/dev/loop0\n",
    )
    .prints(
        Tool::Losetup,
        &["--find", "--show", "--read-only", "/run/stick/store-verity"],
        "/dev/loop1\n",
    )
    .file("/sys/class/block/loop0/dev", b"7:0\n")
    .file("/sys/class/block/loop1/dev", b"7:1\n");
    let swapped = STORE_TABLE.replace("8:3 8:2", "7:0 7:1");
    fake.prints(
        Tool::Dmsetup,
        &[
            "reload",
            "--readonly",
            "--noudevsync",
            "usr",
            "--table",
            &swapped,
        ],
        "",
    )
    .prints(Tool::Dmsetup, &["resume", "--noudevsync", "usr"], "");
    assert_eq!(
        RunFromMemory.observe(&mut fake).unwrap(),
        Standing::Needed { asks: vec![] },
        "it asks nothing, and runs as soon as the stick starts"
    );
    // Once the table is swapped, the kernel shows the loop devices.
    fake.prints(Tool::Dmsetup, &["table", "usr"], STORE_TABLE)
        .prints(Tool::Dmsetup, &["table", "usr"], &swapped);
    Journey::attempt(&RunFromMemory, &mut fake, &Answers::new()).unwrap();

    assert_eq!(fake.holds("/run/stick/store"), Some(&[3u8; 8192][..]));
    assert_eq!(
        fake.holds("/run/stick/store-verity"),
        Some(&[2u8; 8192][..])
    );
    assert!(position(&fake, "veritysetup verify") < position(&fake, "dmsetup reload"));
    assert!(position(&fake, "dmsetup reload") < position(&fake, "dmsetup resume"));
    assert!(
        memory::port_words(&mut fake).contains("take the stick out first"),
        "the key steps say the stick can come out"
    );
}

#[test]
fn a_copy_that_doesnt_match_the_sticks_hash_is_never_run_from() {
    let mut fake = Fake::new();
    store_on_the_stick(&mut fake, 8_000_000);
    fake.prints(
        Tool::Mount,
        &[
            "-t",
            "tmpfs",
            "-o",
            "size=1024k,mode=0700,X-mount.mkdir=0700",
            "stick",
            "/run/stick",
        ],
        "",
    )
    .refuses(
        Tool::Veritysetup,
        VERIFY,
        1,
        "Verification failed at position 4096.",
    );
    let refused = RunFromMemory.run(&mut fake, &Answers::new());
    assert!(
        matches!(&refused, Err(Fault::Refused { said, .. }) if said.contains("Verification failed")),
        "{refused:?}"
    );
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.starts_with("losetup") || call.starts_with("dmsetup reload")),
        "{:#?}",
        fake.calls
    );
    assert_eq!(memory::port_words(&mut fake), "");
}

#[test]
fn a_machine_without_the_memory_keeps_the_stick_in_and_says_so() {
    let mut fake = Fake::new();
    // 16 KiB of copies and a gibibyte for the install, and not quite that.
    store_on_the_stick(&mut fake, (1 << 20) + 15);
    RunFromMemory.run(&mut fake, &Answers::new()).unwrap();
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("mount")),
        "{:#?}",
        fake.calls
    );
    let Standing::Done { evidence } = RunFromMemory.observe(&mut fake).unwrap() else {
        panic!("nothing more to do")
    };
    assert!(evidence.contains("stays in"), "{evidence}");
    assert!(memory::port_words(&mut fake).contains("Keep the stick in"));
}

#[test]
fn a_system_not_running_from_a_sticks_store_has_nothing_to_copy() {
    let mut fake = Fake::new();
    fake.refuses(
        Tool::Dmsetup,
        &["table", "usr"],
        1,
        "device-mapper: table ioctl on usr failed: No such device or address",
    );
    assert!(matches!(
        RunFromMemory.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_key_step_says_the_stick_can_come_out_once_its_system_is_in_memory() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-key-port");
    let mut fake = stick(&scratch, &target);
    fake.prints(
        Tool::Cryptsetup,
        &[
            "luksDump",
            "--dump-json-metadata",
            "/dev/disk/by-partlabel/system",
        ],
        fixture!("luks-dump-before-wipe.json"),
    );
    guide::Machine::journal(&mut fake)
        .record("run-from-memory", "in memory")
        .unwrap();
    let Standing::Needed { asks } = EnrolYubikey::new("the Nano").observe(&mut fake).unwrap()
    else {
        panic!("the Nano isn't enrolled yet")
    };
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("{asks:?}")
    };
    assert_eq!(
        what,
        "Plug in the Nano, and only the Nano. If no port is free, take the stick out first: the guide asks for it back when it needs it. Touch the key each time it blinks."
    );
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
    plugged(
        &mut fake,
        &[("/dev/hidraw1", "canokey"), ("/dev/hidraw2", "canokey")],
    );
    fake.prints(
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
    plugged(&mut fake, &[("/dev/hidraw1", "canokey")]);
    fake.prints(
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

#[test]
fn the_install_key_coming_off_takes_the_journals_copy_with_it() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-copy-goes");
    let mut fake = stick(&scratch, &target);
    fake.prints(
        Tool::Cryptsetup,
        &DUMP,
        fixture!("luks-dump-installed.json"),
    );
    fake.journal()
        .keep_a_copy(std::path::Path::new("/mnt/boot/guide/install-journal"))
        .unwrap();
    // Every slot a token's and the key forgotten, but the copy still says
    // an install is under way: not done until it goes.
    let step = WipeInstallKey { yubikeys: 2 };
    assert!(matches!(
        step.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    Journey::attempt(&step, &mut fake, &Answers::new()).unwrap();
    assert_eq!(fake.journal().copy(), None);
}

const DUMP: [&str; 3] = [
    "luksDump",
    "--dump-json-metadata",
    "/dev/disk/by-partlabel/system",
];

#[test]
fn carrying_on_opens_the_disk_as_it_was_left_rather_than_wiping_it() {
    let afresh: Vec<&str> = guide::install::steps(&["the Nano".into()], Start::Afresh)
        .iter()
        .map(|step| step.id())
        .collect();
    let carrying_on: Vec<&str> = guide::install::steps(&["the Nano".into()], Start::CarryingOn)
        .iter()
        .map(|step| step.id())
        .collect();
    assert!(!carrying_on.contains(&"lay-out-disk"), "{carrying_on:?}");
    let opened = carrying_on
        .iter()
        .position(|id| *id == "open-stopped-install")
        .unwrap();
    assert_eq!(
        afresh.iter().position(|id| *id == "lay-out-disk"),
        Some(opened)
    );
    assert_eq!(afresh.len(), carrying_on.len());
}

/// A stick on a disk an install stopped on: the volume there but shut,
/// with this header.
fn stopped_with(scratch: &std::path::Path, target: &Target, header: &str) -> Fake {
    let mut fake = stick_carrying(scratch, target, &[(MOUNT_SCRIPT, b"mount it")]);
    lsblk(&mut fake, fixture!("lsblk-blank.json"));
    fake.link("/dev/disk/by-partlabel/system", None)
        .prints(Tool::Cryptsetup, &DUMP, header);
    fake
}

/// The same, its header as it was before the install key came off.
fn stopped_on(scratch: &std::path::Path, target: &Target) -> Fake {
    stopped_with(scratch, target, fixture!("luks-dump-before-wipe.json"))
}

fn first_credential() -> String {
    guide::observe::luks::parse(fixture!("luks-dump-before-wipe.json"))
        .unwrap()
        .credentials()[0]
        .to_string()
}

#[test]
fn a_stopped_install_asks_for_the_key_it_proved_opens_the_disk() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-stopped-asks");
    let mut fake = stopped_on(&scratch, &target);
    let step = OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    };
    // With no key proven, either may be the one.
    let Standing::Needed { asks } = step.observe(&mut fake).unwrap() else {
        panic!("the disk is still to open")
    };
    let said = format!("{asks:?}");
    assert!(said.contains("Plug in the Nano or the 5C"), "{said}");

    fake.journal()
        .record("enrol-nano", &format!("opened:{}", first_credential()))
        .unwrap();
    let Standing::Needed { asks } = step.observe(&mut fake).unwrap() else {
        panic!("the disk is still to open")
    };
    let said = format!("{asks:?}");
    assert!(
        said.contains("Plug in the Nano, which opened the disk"),
        "{said}"
    );
    assert!(said.contains("the Nano's PIN"), "{said}");
}

#[test]
fn a_stopped_install_no_key_opens_can_only_start_again() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-stopped-shut");
    let mut header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-before-wipe.json")).unwrap();
    header["tokens"]
        .as_object_mut()
        .unwrap()
        .retain(|_, token| token["type"] != "systemd-fido2");
    let mut fake = stopped_with(&scratch, &target, &header.to_string());
    let standing = OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .observe(&mut fake)
    .unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { remedy, .. } if remedy.contains("install it again")),
        "{standing:?}"
    );
}

#[test]
fn a_stopped_install_gets_a_new_install_key_through_a_yubikey_then_is_mounted() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-stopped-opens");
    let mut fake = stopped_on(&scratch, &target);
    plugged(&mut fake, &[("/dev/hidraw1", "canokey")]);
    let enrol = [
        "--unlock-fido2-device=/dev/hidraw1",
        "--wipe-slot=password",
        "--password",
        "/dev/disk/by-partlabel/system",
    ];
    fake.prints(Tool::SystemdCryptenroll, &enrol, "")
        .prints(Tool::Bash, &["/run/guide/mount"], "")
        .prints(Tool::Udevadm, &["settle"], "");
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .run(&mut fake, &answers)
    .unwrap();

    // The new key held before it's enrolled, given as cryptenroll reads a
    // new password, and the PIN under both names systemd reads it by.
    let key = String::from_utf8(fake.holds(INSTALL_KEY).unwrap().to_vec()).unwrap();
    assert_eq!(key.len(), 64, "{key}");
    assert!(
        fake.secrets.contains(&("NEWPASSWORD".into(), key.clone())),
        "{:?}",
        fake.secrets
    );
    for name in ["cryptsetup.fido2-pin", "cryptenroll.fido2-pin"] {
        assert!(
            fake.secrets.contains(&(name.into(), "123456".into())),
            "{:?}",
            fake.secrets
        );
    }
    assert_eq!(fake.holds("/run/guide/mount"), Some(b"mount it".as_slice()));
    let at = |call: &str| fake.calls.iter().position(|made| made.starts_with(call));
    assert!(
        at("systemd-cryptenroll").unwrap() < at("bash /run/guide/mount").unwrap(),
        "{:#?}",
        fake.calls
    );
    assert!(!wiped(&fake), "{:#?}", fake.calls);
    assert_eq!(
        fake.journal().copy(),
        Some(std::path::Path::new("/mnt/boot/guide/install-journal"))
    );
}

const GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
const DB: &str = "/sys/firmware/efi/efivars/db-d719b2cb-3d3a-4596-a3bc-dad00e67656f";
const SBCTL: &str = "/mnt/var/lib/sbctl";

macro_rules! hardware {
    ($machine:literal, $file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/",
            $machine,
            "/",
            $file
        ))
    };
}

/// The Surface's firmware as it stands: user mode, enforcing with its own
/// keys, and the db that trusts them.
fn surface_firmware(fake: &mut Fake, db: &[u8]) {
    fake.file(
        &format!("/sys/firmware/efi/efivars/SecureBoot-{GLOBAL}"),
        hardware!(
            "microsoft-surface-laptop-4-1958",
            "efivar-SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
        ),
    )
    .file(
        &format!("/sys/firmware/efi/efivars/SetupMode-{GLOBAL}"),
        hardware!(
            "microsoft-surface-laptop-4-1958",
            "efivar-SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
        ),
    )
    .no_file(&format!("/sys/firmware/efi/efivars/AuditMode-{GLOBAL}"))
    .no_file(&format!("/sys/firmware/efi/efivars/DeployedMode-{GLOBAL}"))
    .file(DB, db)
    .no_file(&format!("{SBCTL}/keys/db/db.key"))
    .no_file(&format!("{SBCTL}/GUID"));
}

const SURFACE_DB: &[u8] = hardware!(
    "microsoft-surface-laptop-4-1958",
    "efivar-db-d719b2cb-3d3a-4596-a3bc-dad00e67656f"
);
const SURFACE_CERT: &[u8] = hardware!("microsoft-surface-laptop-4-1958", "sbctl-db.pem");

#[test]
fn the_keys_the_firmware_trusts_are_put_back_with_no_firmware_visit() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-back");
    let carried: &[(&str, &[u8])] = &[
        ("sbctl/GUID", b"22f0175f-c166-49d2-96ab-c61575cddac5"),
        ("sbctl/keys/db/db.key", b"the db key"),
        ("sbctl/keys/db/db.pem", SURFACE_CERT),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut fake, SURFACE_DB);
    assert!(matches!(
        SecureBootKeys.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    SecureBootKeys.run(&mut fake, &Answers::new()).unwrap();
    assert_eq!(
        fake.holds(&format!("{SBCTL}/keys/db/db.key")),
        Some(&b"the db key"[..])
    );
    assert_eq!(
        fake.holds(&format!("{SBCTL}/GUID")),
        Some(&b"22f0175f-c166-49d2-96ab-c61575cddac5"[..])
    );
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("sbctl")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn the_firmware_is_checked_against_the_carried_keys_with_the_stick_out() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-held");
    let carried: &[(&str, &[u8])] = &[("sbctl/keys/db/db.pem", SURFACE_CERT)];
    let mut fake = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut fake, SURFACE_DB);
    let with_the_stick_in = CheckFirmware.observe(&mut fake).unwrap();
    assert!(
        matches!(&with_the_stick_in, Standing::Done { evidence } if evidence.contains("trusts the keys this stick carries")),
        "{with_the_stick_in:?}"
    );
    guide::install::system::hold_carried_db(&mut fake).unwrap();
    fake.stick_out(true);
    assert_eq!(CheckFirmware.observe(&mut fake).unwrap(), with_the_stick_in);
}

#[test]
fn a_firmware_in_setup_mode_gets_new_keys_whatever_the_stick_carries() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-setup");
    // Keys a forged bundle could have put on the stick, its db one the
    // firmware's db still trusts, as it may once only the platform key
    // is cleared.
    let carried: &[(&str, &[u8])] = &[
        ("sbctl/GUID", b"22f0175f-c166-49d2-96ab-c61575cddac5"),
        ("sbctl/keys/db/db.key", b"the db key"),
        ("sbctl/keys/db/db.pem", SURFACE_CERT),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut fake, SURFACE_DB);
    // Secure Boot off and setup mode on, as clearing the keys leaves it,
    // each variable's attributes and then its one byte.
    fake.file(
        &format!("/sys/firmware/efi/efivars/SecureBoot-{GLOBAL}"),
        &[6, 0, 0, 0, 0],
    )
    .file(
        &format!("/sys/firmware/efi/efivars/SetupMode-{GLOBAL}"),
        &[6, 0, 0, 0, 1],
    );
    let create = [
        "--disable-landlock".to_string(),
        "create-keys".into(),
        "--export".into(),
        format!("{SBCTL}/keys"),
        "--database-path".into(),
        format!("{SBCTL}/GUID"),
    ];
    let create: Vec<&str> = create.iter().map(String::as_str).collect();
    fake.prints(Tool::Sbctl, &create, "");
    let standing = SecureBootKeys.observe(&mut fake).unwrap();
    assert!(matches!(standing, Standing::Needed { .. }), "{standing:?}");
    SecureBootKeys.run(&mut fake, &Answers::new()).unwrap();
    assert!(
        fake.calls.iter().any(|call| call.starts_with("sbctl")),
        "{:#?}",
        fake.calls
    );
    assert_ne!(
        fake.holds(&format!("{SBCTL}/keys/db/db.key")),
        Some(&b"the db key"[..])
    );
}

#[test]
fn keys_the_firmware_doesnt_trust_are_refused() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-other");
    let carried: &[(&str, &[u8])] = &[
        ("sbctl/GUID", b"guid"),
        ("sbctl/keys/db/db.key", b"key"),
        ("sbctl/keys/db/db.pem", SURFACE_CERT),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    // The Surface's keys, but the XPS 14's db.
    surface_firmware(
        &mut fake,
        hardware!(
            "dell-xps-14-da14260",
            "efivar-db-d719b2cb-3d3a-4596-a3bc-dad00e67656f"
        ),
    );
    let standing = SecureBootKeys.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("trusts other keys")),
        "{standing:?}"
    );
}

#[test]
fn with_no_keys_carried_a_firmware_in_user_mode_has_to_be_cleared_first() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-none");
    let mut fake = stick(&scratch, &target);
    surface_firmware(&mut fake, SURFACE_DB);
    let standing = SecureBootKeys.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("won't take new keys")),
        "{standing:?}"
    );
}

#[test]
fn a_firmware_that_wont_take_the_keys_stops_the_install_before_the_disk_is_touched() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-firmware-first");
    let mut fake = stick(&scratch, &target);
    // A Surface enforcing its own keys, with none carried.
    surface_firmware(&mut fake, SURFACE_DB);
    assert!(matches!(
        CheckFirmware.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));

    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()], Start::Afresh)
        .iter()
        .map(|step| step.id())
        .collect();
    let at = |id: &str| ids.iter().position(|found| *found == id).unwrap();
    assert!(at("check-firmware") < at("confirm-disk"), "{ids:?}");
    assert!(at("confirm-disk") < at("lay-out-disk"), "{ids:?}");
}

#[test]
fn a_firmware_trusting_the_carried_keys_lets_the_install_go_on() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-firmware-trusts");
    let carried: &[(&str, &[u8])] = &[
        ("sbctl/GUID", b"guid"),
        ("sbctl/keys/db/db.key", b"key"),
        ("sbctl/keys/db/db.pem", SURFACE_CERT),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut fake, SURFACE_DB);
    let standing = CheckFirmware.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("trusts the keys this stick carries")),
        "{standing:?}"
    );
}

#[test]
fn the_copy_onto_the_disk_is_measured_against_the_paths_unpacked() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-unpacked-size");
    let carried: &[(&str, &[u8])] = &[
        (
            "cache/system/a.narinfo",
            b"StorePath: /nix/store/a\nNarSize: 1000\n",
        ),
        (
            "cache/system/b.narinfo",
            b"StorePath: /nix/store/b\nNarSize: 2500\n",
        ),
        ("cache/system/nar/a.nar.zst", b"squeezed"),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    let key = guide::install::vault_key(&mut fake).unwrap().unwrap();
    fake.unpack(key.clone(), SYSTEM_CACHE, system::CACHE)
        .unwrap();
    assert_eq!(system::unpacked_size(&mut fake, key).unwrap(), Some(3500));
}

#[test]
fn a_narinfo_without_its_size_measures_nothing() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-unpacked-unsized");
    let carried: &[(&str, &[u8])] = &[
        (
            "cache/system/a.narinfo",
            b"StorePath: /nix/store/a\nNarSize: 1000\n",
        ),
        ("cache/system/b.narinfo", b"StorePath: /nix/store/b\n"),
    ];
    let mut fake = stick_carrying(&scratch, &target, carried);
    let key = guide::install::vault_key(&mut fake).unwrap().unwrap();
    fake.unpack(key.clone(), SYSTEM_CACHE, system::CACHE)
        .unwrap();
    assert_eq!(system::unpacked_size(&mut fake, key).unwrap(), None);
}

#[test]
fn once_the_keys_are_made_the_firmware_check_says_what_it_found_not_what_they_are() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-firmware-after-keys");
    let mut fake = stick(&scratch, &target);
    fake.file(&format!("{SBCTL}/keys/db/db.key"), b"key")
        .file(&format!("{SBCTL}/GUID"), b"guid");
    let evidence = |standing: Standing| match standing {
        Standing::Done { evidence } => evidence,
        other => panic!("{other:?}"),
    };
    let firmware = evidence(CheckFirmware.observe(&mut fake).unwrap());
    let keys = evidence(SecureBootKeys.observe(&mut fake).unwrap());
    assert_ne!(firmware, keys, "each step done is listed once");
    assert!(firmware.contains("firmware"), "{firmware}");
}

const BUNDLE: &str = "/mnt/var/lib/guide/rescue-bundle.json";
const RECOVERY: &str = "fkhvbicb-urbtgcbh-gdbbcbkf-ekdjvcrg-jhcvdbbe-jtbifkkv-ncdvurrb-tutbkbcl";

/// The install's root holding the keys it made, its recovery key held.
fn keys_made(fake: &mut Fake, db: &str) {
    for name in rescue::KEYS {
        let path = format!("{SBCTL}/{name}");
        let text = match name {
            "keys/db/db.pem" => db.to_string(),
            "GUID" => "22f0175f-c166-49d2-96ab-c61575cddac5\n".to_string(),
            other => format!("the {other}"),
        };
        fake.prints(Tool::Cat, &[&path], &text);
    }
    fake.file(&format!("{SBCTL}/keys/db/db.pem"), db.as_bytes())
        .file(RECOVERY_KEY, RECOVERY.as_bytes());
}

#[test]
fn the_install_seals_the_keys_to_the_recovery_key_before_its_typed_back() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-rescue-bundle");
    let mut fake = stick(&scratch, &target);
    keys_made(&mut fake, "the db certificate");
    fake.no_file(BUNDLE);
    assert!(matches!(
        SealRescueBundle.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    SealRescueBundle.run(&mut fake, &Answers::new()).unwrap();

    let bundle = Bundle::read(fake.holds(BUNDLE).expect("the bundle")).unwrap();
    assert_eq!(bundle.machine(), "machine");
    assert_eq!(bundle.passphrase(), rescue::RECOVERY_KEY);
    let keys = bundle
        .open_with_passphrase(&Secret::new(RECOVERY.into()))
        .unwrap();
    assert_eq!(keys.owner(), "22f0175f-c166-49d2-96ab-c61575cddac5");
    let standing = SealRescueBundle.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("recovery key")),
        "{standing:?}"
    );

    // Keys made again, as a second install does, need a bundle again.
    fake.file(
        &format!("{SBCTL}/keys/db/db.pem"),
        b"another db certificate",
    );
    assert!(matches!(
        SealRescueBundle.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));

    // With the recovery key gone there's nothing to seal it to.
    fake.no_file(RECOVERY_KEY);
    assert!(matches!(
        SealRescueBundle.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));

    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()], Start::Afresh)
        .iter()
        .map(|step| step.id())
        .collect();
    let at = |id: &str| ids.iter().position(|found| *found == id).unwrap();
    assert!(at("secure-boot-keys") < at("seal-rescue-bundle"), "{ids:?}");
    assert!(
        at("make-recovery-key") < at("seal-rescue-bundle"),
        "{ids:?}"
    );
    assert!(
        at("seal-rescue-bundle") < at("keep-recovery-key"),
        "{ids:?}"
    );
}

fn run(program: &str, args: &[&std::ffi::OsStr]) -> std::process::Output {
    let output = std::process::Command::new(program)
        .args(args)
        .env("MTOOLS_SKIP_CHECK", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// A machine's db key and certificate, made as sbctl makes them.
fn db_key(directory: &std::path::Path) -> (String, String) {
    let (key, pem) = (directory.join("db.key"), directory.join("db.pem"));
    let mut args: Vec<&std::ffi::OsStr> = [
        "req",
        "-new",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "3650",
        "-subj",
        "/CN=Database Key",
        "-keyout",
    ]
    .iter()
    .map(AsRef::as_ref)
    .collect();
    args.extend([key.as_os_str(), "-out".as_ref(), pem.as_os_str()]);
    run("openssl", &args);
    (
        std::fs::read_to_string(key).unwrap(),
        std::fs::read_to_string(pem).unwrap(),
    )
}

/// The stick's EFI system partition, holding systemd-boot where firmware
/// boots a removable disk from, unsigned as the image has it.
fn stick_esp(directory: &std::path::Path) -> PathBuf {
    let esp = directory.join("esp.fat");
    let boot = PathBuf::from(std::env::var("GUIDE_TEST_EFI").unwrap()).join("systemd-bootx64.efi");
    run(
        "mkfs.fat",
        &["-C".as_ref(), esp.as_os_str(), "1024".as_ref()],
    );
    run(
        "mmd",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            "::EFI".as_ref(),
            "::EFI/BOOT".as_ref(),
        ],
    );
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            boot.as_os_str(),
            "::EFI/BOOT/BOOTX64.EFI".as_ref(),
        ],
    );
    esp
}

/// Whether the stick's boot program verifies with this certificate.
fn boots_with(directory: &std::path::Path, esp: &std::path::Path, pem: &str) -> bool {
    let (program, certificate) = (directory.join("BOOTX64.EFI"), directory.join("check.pem"));
    let _ = std::fs::remove_file(&program);
    std::fs::write(&certificate, pem).unwrap();
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            "::EFI/BOOT/BOOTX64.EFI".as_ref(),
            program.as_os_str(),
        ],
    );
    std::process::Command::new("sbverify")
        .args([
            "--cert".as_ref(),
            certificate.as_os_str(),
            program.as_os_str(),
        ])
        .output()
        .unwrap()
        .status
        .success()
}

/// A stick whose install has made the machine's keys and sealed its
/// bundle, its partitions as lsblk saw them in the guide's VM: its own
/// boot partition mounted at /boot, as the stick mounts it.
fn installed_on_stick(scratch: &std::path::Path, key: &str, pem: &str) -> (Fake, PathBuf) {
    installed_on_stick_seen(scratch, key, pem, fixture!("lsblk-laid-out.json"))
}

fn installed_on_stick_seen(
    scratch: &std::path::Path,
    key: &str,
    pem: &str,
    lsblk_json: &str,
) -> (Fake, PathBuf) {
    let vault = scratch.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let mut fake = stick(&vault, &target("/dev/disk/by-id/virtio-root"));
    for name in rescue::KEYS {
        let text = match name {
            "keys/db/db.key" => key.to_string(),
            "keys/db/db.pem" => pem.to_string(),
            "GUID" => "22f0175f-c166-49d2-96ab-c61575cddac5\n".to_string(),
            other => format!("the {other}"),
        };
        fake.prints(Tool::Cat, &[&format!("{SBCTL}/{name}")], &text);
    }
    let esp = stick_esp(scratch);
    fake.file(&format!("{SBCTL}/keys/db/db.pem"), pem.as_bytes())
        .file(BUNDLE, b"the sealed bundle")
        .link(STICK_VAULT, Some("/dev/sda4"))
        .disk("/dev/sda1", &esp)
        .prints(Tool::Umount, &["/boot"], "");
    lsblk(&mut fake, lsblk_json);
    (fake, vault)
}

#[test]
fn the_stick_carries_the_machines_keys_and_bundle_and_boots_with_its_key() {
    let scratch = Scratch::new("install-keep-on-stick");
    let (key, pem) = db_key(&scratch);
    let (mut fake, vault) = installed_on_stick(&scratch, &key, &pem);
    assert!(matches!(
        KeepOnStick.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    KeepOnStick.run(&mut fake, &Answers::new()).unwrap();

    let opened = VaultKey::from_secret(&Secret::new(
        String::from_utf8(fake.holds(VAULT_KEY).unwrap().to_vec()).unwrap(),
    ))
    .unwrap();
    let mut store = Directory::new(vault.clone());
    let mut carried = Vault::open(&mut store, opened).unwrap();
    for name in rescue::KEYS {
        let mut bytes = vec![];
        carried
            .extract(&format!("sbctl/{name}"), &mut bytes)
            .unwrap();
        if name == "keys/db/db.key" {
            assert_eq!(bytes, key.as_bytes());
        }
    }
    assert_eq!(
        std::fs::read(vault.join("machine-rescue-bundle.json")).unwrap(),
        b"the sealed bundle"
    );
    assert!(boots_with(&scratch, &scratch.join("esp.fat"), &pem));
    // Unmounted from /boot before it's opened to be changed.
    let at = |call: &str| fake.calls.iter().position(|made| made.starts_with(call));
    assert!(
        at("umount /boot").unwrap() < at("opening /dev/sda1 to change in place").unwrap(),
        "{:#?}",
        fake.calls
    );
    assert!(matches!(
        KeepOnStick.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));

    // Run again with the same keys and a newer bundle, as a step tried
    // again finds the stick: the bundle is the new one, the vault holds no
    // more files than it did, and the boot partition isn't written to.
    let listed = || std::fs::read_dir(&vault).unwrap().count();
    let (files, esp) = (listed(), std::fs::read(scratch.join("esp.fat")).unwrap());
    fake.file(BUNDLE, b"a newer sealed bundle");
    KeepOnStick.run(&mut fake, &Answers::new()).unwrap();
    assert_eq!(
        std::fs::read(vault.join("machine-rescue-bundle.json")).unwrap(),
        b"a newer sealed bundle"
    );
    assert_eq!(listed(), files);
    // Said in words, since the whole partition's bytes would bury the
    // failure.
    assert!(
        std::fs::read(scratch.join("esp.fat")).unwrap() == esp,
        "the ESP its key signed already was changed"
    );
    assert!(boots_with(&scratch, &scratch.join("esp.fat"), &pem));

    // Keys made again, as a second install from the stick does, make it
    // that machine's again.
    let other = Scratch::new("install-keep-on-stick-other");
    let (_, another) = db_key(&other);
    fake.file(&format!("{SBCTL}/keys/db/db.pem"), another.as_bytes());
    assert!(matches!(
        KeepOnStick.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_boot_partition_on_another_disk_than_the_vaults_is_never_signed() {
    let scratch = Scratch::new("install-keep-on-stick-elsewhere");
    let (key, pem) = db_key(&scratch);
    // Named as the stick names its own, but on the disk installed onto.
    let mut seen: serde_json::Value =
        serde_json::from_str(fixture!("lsblk-laid-out.json")).unwrap();
    for disk in seen["blockdevices"].as_array_mut().unwrap() {
        let partitions = disk
            .get_mut("children")
            .and_then(|found| found.as_array_mut());
        for partition in partitions.into_iter().flatten() {
            let label = match partition["path"].as_str() {
                Some("/dev/sda1") => "boot",
                Some("/dev/vda1") => "esp",
                _ => continue,
            };
            partition["partlabel"] = label.into();
        }
    }
    let (mut fake, vault) = installed_on_stick_seen(&scratch, &key, &pem, &seen.to_string());
    fake.disk("/dev/vda1", &scratch.join("esp.fat"));
    let before = std::fs::read(scratch.join("esp.fat")).unwrap();
    let result = KeepOnStick.run(&mut fake, &Answers::new());
    assert!(
        matches!(&result, Err(Fault::Blocked { why, .. }) if why.contains("no boot partition")),
        "{result:?}"
    );
    assert_eq!(std::fs::read(scratch.join("esp.fat")).unwrap(), before);
    assert!(!vault.join("machine-rescue-bundle.json").exists());
}

#[test]
fn the_stick_is_made_the_machines_own_only_once_the_machine_is_finished() {
    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()], Start::Afresh)
        .iter()
        .map(|step| step.id())
        .collect();
    let at = |id: &str| ids.iter().position(|found| *found == id).unwrap();
    assert!(at("set-password") < at("keep-on-stick"), "{ids:?}");
    assert!(at("wipe-install-key") < at("keep-on-stick"), "{ids:?}");
    assert_eq!(ids.last(), Some(&"keep-on-stick"), "{ids:?}");
}

const VOLUME: &str = "/dev/disk/by-partlabel/system";
const TEST: [&str; 5] = [
    "open",
    "--test-passphrase",
    "--verbose",
    "--key-file=-",
    VOLUME,
];
const IMPORT: [&str; 4] = ["token", "import", "--json-file=-", VOLUME];

/// The install's disk with both keys enrolled and no recovery key yet: the
/// real header, its recovery token taken out.
fn without_a_recovery_key(fake: &mut Fake) {
    let mut header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-before-wipe.json")).unwrap();
    header["tokens"]
        .as_object_mut()
        .unwrap()
        .retain(|_, token| token["type"] != "systemd-recovery");
    fake.prints(
        Tool::Cryptsetup,
        &["luksDump", "--dump-json-metadata", VOLUME],
        &header.to_string(),
    );
}

fn handed<'a>(fake: &'a Fake, place: &str) -> Vec<&'a str> {
    fake.secrets
        .iter()
        .filter(|(at, _)| at == place)
        .map(|(_, secret)| secret.as_str())
        .collect()
}

#[test]
fn the_recovery_key_is_seven_words_held_before_theyre_enrolled_then_marked_as_one() {
    use guide::install::keys::MakeRecoveryKey;
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-recovery-words");
    let mut fake = stick(&scratch, &target);
    without_a_recovery_key(&mut fake);
    fake.no_file(RECOVERY_KEY);
    // The words open nothing until they're enrolled, then slot 3.
    fake.refuses(
        Tool::Cryptsetup,
        &TEST,
        2,
        "No key available with this passphrase.",
    )
    .prints(
        Tool::Cryptsetup,
        &TEST,
        "Key slot 3 unlocked.\nCommand successful.\n",
    );
    let unlock = format!("--unlock-key-file={INSTALL_KEY}");
    fake.prints(
        Tool::SystemdCryptenroll,
        &[&unlock, "--password", VOLUME],
        "",
    )
    .prints(Tool::Cryptsetup, &IMPORT, "");
    assert!(matches!(
        MakeRecoveryKey.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    MakeRecoveryKey.run(&mut fake, &Answers::new()).unwrap();

    let words = String::from_utf8(fake.holds(RECOVERY_KEY).unwrap().to_vec()).unwrap();
    let each: Vec<&str> = words.split(' ').collect();
    assert_eq!(each.len(), 7, "{words}");
    assert!(each.iter().all(|word| guide::words::list().contains(word)));
    assert_eq!(handed(&fake, "NEWPASSWORD"), [words.as_str()]);
    // Marked in the slot the words opened, as systemd marks its own.
    assert!(handed(&fake, "stdin").contains(&r#"{"type":"systemd-recovery","keyslots":["3"]}"#));
    let at = |start: &str| {
        fake.calls
            .iter()
            .position(|call| call.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} in {:#?}", fake.calls))
    };
    assert!(at("writing /run/guide/recovery-key") < at("systemd-cryptenroll"));
    assert!(at("systemd-cryptenroll") < at("cryptsetup token import"));
    // New words mean a bundle sealed to any before them goes, as after a
    // restart lost them, so the next step seals it to these.
    assert!(at(&format!("removing {BUNDLE}")) < at("writing /run/guide/recovery-key"));
}

#[test]
fn a_recovery_key_cut_short_keeps_its_words_and_enrols_no_second_slot() {
    use guide::install::keys::MakeRecoveryKey;
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-recovery-again");
    let mut fake = stick(&scratch, &target);
    without_a_recovery_key(&mut fake);
    let held = "abacus abdomen abdominal abide abiding ability ablaze";
    fake.file(RECOVERY_KEY, held.as_bytes());
    fake.prints(Tool::Cryptsetup, &TEST, "Key slot 5 unlocked.\n")
        .prints(Tool::Cryptsetup, &IMPORT, "");
    MakeRecoveryKey.run(&mut fake, &Answers::new()).unwrap();
    assert_eq!(fake.holds(RECOVERY_KEY), Some(held.as_bytes()));
    assert!(
        !fake.calls.iter().any(|call| call.contains(BUNDLE)),
        "{:#?}",
        fake.calls
    );
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        fake.calls
    );
    assert!(handed(&fake, "stdin").contains(&r#"{"type":"systemd-recovery","keyslots":["5"]}"#));
}

#[test]
fn the_recovery_key_is_shown_alone_for_the_phone_then_typed_back_from_it() {
    use guide::install::keys::KeepRecoveryKey;
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-recovery-kept");
    let mut fake = stick(&scratch, &target);
    let held = "abacus abdomen abdominal abide abiding ability ablaze";
    fake.file(RECOVERY_KEY, held.as_bytes());
    let standing = KeepRecoveryKey.observe(&mut fake).unwrap();
    let Standing::Needed { asks } = &standing else {
        panic!("{standing:?}");
    };
    let [
        guide::Ask::Show { how, shown, .. },
        guide::Ask::Recall { expected, .. },
    ] = asks.as_slice()
    else {
        panic!("{asks:#?}");
    };
    assert_eq!((shown.as_str(), expected.as_str()), (held, held));
    assert!(how.contains("\"machine disk recovery key\""), "{how}");
    assert!(
        how.contains("Copy All") && how.contains("1Password"),
        "{how}"
    );
    // The steps say nothing of the words themselves.
    assert!(!how.contains("abacus"), "{how}");

    // Typed back with a capital and a doubled space, it's the same key,
    // and the key as it's held is what's tried on the disk.
    fake.prints(
        Tool::Cryptsetup,
        &["open", "--test-passphrase", "--key-file=-", VOLUME],
        "",
    );
    let mut answers = Answers::new();
    answers.give("shown", Answer::Done);
    answers.give(
        "key",
        Answer::Typed("Abacus abdomen  abdominal abide abiding ability ablaze".into()),
    );
    KeepRecoveryKey.run(&mut fake, &answers).unwrap();
    assert_eq!(handed(&fake, "stdin"), [held]);
    assert_eq!(fake.holds(RECOVERY_KEY), None);

    // A word off by a letter isn't it.
    let mut fake = stick(&scratch, &target);
    fake.file(RECOVERY_KEY, held.as_bytes());
    let mut answers = Answers::new();
    answers.give(
        "key",
        Answer::Typed("abacus abdomen abdominal abide abiding ability ablase".into()),
    );
    assert!(matches!(
        KeepRecoveryKey.run(&mut fake, &answers),
        Err(Fault::Blocked { .. })
    ));
}

/// The installed system's profile leading to the target's system, and
/// that system with core's program to ready its keys, or without.
fn installed(fake: &mut Fake, target: &Target, readies: bool) {
    fake.pointer("/mnt/nix/var/nix/profiles/system", &target.system);
    if readies {
        fake.pointer(
            &format!("/mnt{}/secure-boot-enrolment", target.system),
            "/nix/store/def-secure-boot-enrolment",
        );
    }
}

/// The firmware in setup mode, or in user mode enforcing.
fn firmware_in_setup(fake: &mut Fake, setup: bool) {
    let (enforcing, setup): (&[u8], &[u8]) = if setup {
        (&[6, 0, 0, 0, 0], &[6, 0, 0, 0, 1])
    } else {
        (&[6, 0, 0, 0, 1], &[6, 0, 0, 0, 0])
    };
    fake.file(
        &format!("/sys/firmware/efi/efivars/SecureBoot-{GLOBAL}"),
        enforcing,
    )
    .file(
        &format!("/sys/firmware/efi/efivars/SetupMode-{GLOBAL}"),
        setup,
    )
    .no_file(&format!("/sys/firmware/efi/efivars/AuditMode-{GLOBAL}"))
    .no_file(&format!("/sys/firmware/efi/efivars/DeployedMode-{GLOBAL}"));
}

const AUTO: [&str; 3] = [
    "/mnt/boot/loader/keys/auto/PK.auth",
    "/mnt/boot/loader/keys/auto/KEK.auth",
    "/mnt/boot/loader/keys/auto/db.auth",
];

#[test]
fn a_firmware_in_setup_mode_gets_the_keys_readied_inside_the_installed_system() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-enrolment");
    let mut fake = stick(&scratch, &target);
    installed(&mut fake, &target, true);
    firmware_in_setup(&mut fake, true);
    for file in AUTO {
        fake.no_file(file);
    }
    assert!(matches!(
        ReadyEnrolment.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    let program = format!("{}/secure-boot-enrolment", target.system);
    fake.prints(Tool::NixosEnter, &["--root", "/mnt", "-c", &program], "");
    ReadyEnrolment.run(&mut fake, &Answers::new()).unwrap();
    assert!(
        fake.calls
            .iter()
            .any(|call| call == &format!("nixos-enter --root /mnt -c {program}")),
        "{:#?}",
        fake.calls
    );
    // What it left on the ESP is what proves it.
    for file in AUTO {
        fake.file(file, b"an authenticated variable");
    }
    let Standing::Done { evidence } = ReadyEnrolment.observe(&mut fake).unwrap() else {
        panic!("the keys are readied")
    };
    assert!(evidence.contains("before the disk is opened"), "{evidence}");
}

#[test]
fn a_firmware_with_keys_of_its_own_gets_nothing_readied() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-nothing");
    let mut fake = stick(&scratch, &target);
    installed(&mut fake, &target, true);
    firmware_in_setup(&mut fake, false);
    let Standing::Done { evidence } = ReadyEnrolment.observe(&mut fake).unwrap() else {
        panic!("a firmware with its own keys enrols nothing")
    };
    assert!(evidence.contains("isn't in setup mode"), "{evidence}");
    ReadyEnrolment.run(&mut fake, &Answers::new()).unwrap();
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.starts_with("nixos-enter")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn a_system_that_readies_no_keys_itself_is_left_to_its_first_boot() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-no-program");
    let mut fake = stick(&scratch, &target);
    installed(&mut fake, &target, false);
    // The firmware isn't even looked at: nothing here would ready keys.
    assert!(matches!(
        ReadyEnrolment.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn nothing_is_readied_before_the_system_is_installed() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-first");
    let mut fake = stick(&scratch, &target);
    fake.pointer("/mnt/nix/var/nix/profiles/system", "/nix/store/old-system");
    assert!(matches!(
        ReadyEnrolment.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

/// The installed disk's FIDO2 credentials, in the order of their tokens.
fn installed_credentials() -> Vec<String> {
    let header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-installed.json")).unwrap();
    let tokens = header["tokens"].as_object().unwrap();
    let mut ids: Vec<&String> = tokens.keys().collect();
    ids.sort();
    ids.iter()
        .filter_map(|id| tokens[*id]["fido2-credential"].as_str())
        .map(String::from)
        .collect()
}

/// A stick whose journal knows the Nano, and the 5C when `both`, opened
/// the installed disk with its own credential.
fn enrolled_stick(scratch: &Scratch, target: &Target, both: bool) -> (Fake, Vec<String>) {
    let mut fake = stick(scratch, target);
    fake.prints(
        Tool::Cryptsetup,
        &["luksDump", "--dump-json-metadata", VOLUME],
        fixture!("luks-dump-installed.json"),
    );
    let credentials = installed_credentials();
    fake.journal()
        .record("enrol-nano", &format!("opened:{}", credentials[1]))
        .unwrap();
    if both {
        fake.journal()
            .record("enrol-5c", &format!("opened:{}", credentials[0]))
            .unwrap();
    }
    (fake, credentials)
}

#[test]
fn the_installed_system_is_told_which_credential_each_named_key_enrolled() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named");
    let (mut fake, credentials) = enrolled_stick(&scratch, &target, true);
    let kept = format!("/mnt{NAMED}");
    fake.no_file(&kept);
    assert!(matches!(
        NameYubikeys.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    NameYubikeys.run(&mut fake, &Answers::new()).unwrap();
    assert!(
        fake.calls.contains(&format!("publishing {kept}")),
        "{:#?}",
        fake.calls
    );
    let named: Vec<Named> = serde_json::from_slice(&fake.read(&kept).unwrap().unwrap()).unwrap();
    assert_eq!(
        named,
        [
            Named {
                name: "the Nano".into(),
                credential: credentials[1].clone(),
            },
            Named {
                name: "the 5C".into(),
                credential: credentials[0].clone(),
            },
        ]
    );
    let Standing::Done { evidence } = NameYubikeys.observe(&mut fake).unwrap() else {
        panic!("the names are kept")
    };
    assert!(evidence.contains("knows the Nano and the 5C"), "{evidence}");
}

#[test]
fn names_kept_from_another_enrolment_are_written_again() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named-again");
    let (mut fake, credentials) = enrolled_stick(&scratch, &target, true);
    // Swapped, as an earlier install's would differ.
    let stale = format!(
        r#"[{{"name":"the Nano","credential":"{}"}},{{"name":"the 5C","credential":"{}"}}]"#,
        credentials[0], credentials[1]
    );
    fake.file(&format!("/mnt{NAMED}"), stale.as_bytes());
    assert!(matches!(
        NameYubikeys.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_key_not_yet_enrolled_leaves_nothing_to_name() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named-early");
    let (mut fake, _) = enrolled_stick(&scratch, &target, false);
    let Standing::Blocked { why, .. } = NameYubikeys.observe(&mut fake).unwrap() else {
        panic!("the 5C isn't enrolled")
    };
    assert!(why.contains("the 5C isn't enrolled"), "{why}");
    assert!(NameYubikeys.run(&mut fake, &Answers::new()).is_err());
}
