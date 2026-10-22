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
use guide::install::system::{CheckFirmware, KeepOnStick, SealRescueBundle, SecureBootKeys};
use guide::install::{INSTALL_KEY, RECOVERY_KEY, STICK_VAULT, TARGET, Target, VAULT_KEY};
use guide::rescue::{self, Bundle};
use guide::vault::store::Directory;
use guide::vault::{Manifest, Packer, Vault, VaultKey};
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

    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()])
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

    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()])
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
    .map(|arg| arg.as_ref())
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
    assert!(std::fs::read(scratch.join("esp.fat")).unwrap() == esp);
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
    let ids: Vec<&str> = guide::install::steps(&["the Nano".into()])
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
