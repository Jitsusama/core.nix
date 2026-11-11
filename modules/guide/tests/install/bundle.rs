//! The rescue bundle sealed to the recovery key, and the stick carrying the
//! machine's keys and bundle, booting with its key, and made the machine's own
//! once the machine is finished.

use std::path::PathBuf;

use guide::install::system::{KeepOnStick, SealRescueBundle};
use guide::install::{RECOVERY_KEY, STICK_VAULT, VAULT_KEY};
use guide::rescue::{self, Bundle};
use guide::stand_in::StandIn;
use guide::vault::store::Directory;
use guide::vault::{Vault, VaultKey};
use guide::{Answers, Fault, Secret, Standing, Step, Tool};

use super::{BUNDLE, RECOVERY, SBCTL, lsblk, step_ids, stick, target};
use crate::common::Scratch;

/// The install's root holding the keys it made, its recovery key held.
fn keys_made(stand_in: &mut StandIn, db: &str) {
    for name in rescue::KEYS {
        let path = format!("{SBCTL}/{name}");
        let text = match name {
            "keys/db/db.pem" => db.to_string(),
            "GUID" => "22f0175f-c166-49d2-96ab-c61575cddac5\n".to_string(),
            other => format!("the {other}"),
        };
        stand_in.prints(Tool::Cat, &[&path], &text);
    }
    stand_in
        .file(&format!("{SBCTL}/keys/db/db.pem"), db.as_bytes())
        .file(RECOVERY_KEY, RECOVERY.as_bytes());
}

#[test]
fn the_install_seals_the_keys_to_the_recovery_key_before_its_typed_back() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-rescue-bundle");
    let mut stand_in = stick(&scratch, &target);
    keys_made(&mut stand_in, "the db certificate");
    stand_in.no_file(BUNDLE);
    assert!(matches!(
        SealRescueBundle.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    SealRescueBundle
        .run(&mut stand_in, &Answers::new())
        .unwrap();

    let bundle = Bundle::read(stand_in.holds(BUNDLE).expect("the bundle")).unwrap();
    assert_eq!(bundle.machine(), "machine");
    assert_eq!(bundle.passphrase(), rescue::RECOVERY_KEY);
    let keys = bundle
        .open_with_passphrase(&Secret::new(RECOVERY.into()))
        .unwrap();
    assert_eq!(keys.owner(), "22f0175f-c166-49d2-96ab-c61575cddac5");
    let standing = SealRescueBundle.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("recovery key")),
        "{standing:?}"
    );

    // Keys made again, as a second install does, need a bundle again.
    stand_in.file(
        &format!("{SBCTL}/keys/db/db.pem"),
        b"another db certificate",
    );
    assert!(matches!(
        SealRescueBundle.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));

    // With the recovery key gone there's nothing to seal it to.
    stand_in.no_file(RECOVERY_KEY);
    assert!(matches!(
        SealRescueBundle.observe(&mut stand_in).unwrap(),
        Standing::Blocked { .. }
    ));

    let ids = step_ids(&target);
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

/// The stick's boot partition, holding systemd-boot where firmware
/// boots a removable disk from, unsigned as the image has it.
fn stick_boot_partition(directory: &std::path::Path) -> PathBuf {
    let boot_partition = directory.join("boot.fat");
    let boot = PathBuf::from(std::env::var("GUIDE_TEST_EFI").unwrap()).join("systemd-bootx64.efi");
    run(
        "mkfs.fat",
        &["-C".as_ref(), boot_partition.as_os_str(), "1024".as_ref()],
    );
    run(
        "mmd",
        &[
            "-i".as_ref(),
            boot_partition.as_os_str(),
            "::EFI".as_ref(),
            "::EFI/BOOT".as_ref(),
        ],
    );
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            boot_partition.as_os_str(),
            boot.as_os_str(),
            "::EFI/BOOT/BOOTX64.EFI".as_ref(),
        ],
    );
    boot_partition
}

/// Whether the stick's boot program verifies with this certificate.
fn boots_with(directory: &std::path::Path, boot_partition: &std::path::Path, pem: &str) -> bool {
    let (program, certificate) = (directory.join("BOOTX64.EFI"), directory.join("check.pem"));
    let _ = std::fs::remove_file(&program);
    std::fs::write(&certificate, pem).unwrap();
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            boot_partition.as_os_str(),
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
fn installed_on_stick(scratch: &std::path::Path, key: &str, pem: &str) -> (StandIn, PathBuf) {
    installed_on_stick_seen(scratch, key, pem, fixture!("lsblk-laid-out.json"))
}

fn installed_on_stick_seen(
    scratch: &std::path::Path,
    key: &str,
    pem: &str,
    lsblk_json: &str,
) -> (StandIn, PathBuf) {
    let vault = scratch.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    let mut stand_in = stick(&vault, &target("/dev/disk/by-id/virtio-root"));
    for name in rescue::KEYS {
        let text = match name {
            "keys/db/db.key" => key.to_string(),
            "keys/db/db.pem" => pem.to_string(),
            "GUID" => "22f0175f-c166-49d2-96ab-c61575cddac5\n".to_string(),
            other => format!("the {other}"),
        };
        stand_in.prints(Tool::Cat, &[&format!("{SBCTL}/{name}")], &text);
    }
    let boot_partition = stick_boot_partition(scratch);
    stand_in
        .file(&format!("{SBCTL}/keys/db/db.pem"), pem.as_bytes())
        .file(BUNDLE, b"the sealed bundle")
        .link(STICK_VAULT, Some("/dev/sda4"))
        .disk("/dev/sda1", &boot_partition)
        .prints(Tool::Umount, &["/boot"], "");
    lsblk(&mut stand_in, lsblk_json);
    (stand_in, vault)
}

#[test]
fn the_stick_carries_the_machines_keys_and_bundle_and_boots_with_its_key() {
    let scratch = Scratch::new("install-keep-on-stick");
    let (key, pem) = db_key(&scratch);
    let (mut stand_in, vault) = installed_on_stick(&scratch, &key, &pem);
    assert!(matches!(
        KeepOnStick.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    KeepOnStick.run(&mut stand_in, &Answers::new()).unwrap();

    let opened = VaultKey::from_secret(&Secret::new(
        String::from_utf8(stand_in.holds(VAULT_KEY).unwrap().to_vec()).unwrap(),
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
    assert!(boots_with(&scratch, &scratch.join("boot.fat"), &pem));
    // Unmounted from /boot before it's opened to be changed.
    let at = |call: &str| {
        stand_in
            .calls
            .iter()
            .position(|made| made.starts_with(call))
    };
    assert!(
        at("umount /boot").unwrap() < at("opening /dev/sda1 to change in place").unwrap(),
        "{:#?}",
        stand_in.calls
    );
    assert!(matches!(
        KeepOnStick.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));

    // Run again with the same keys and a newer bundle, as a step tried
    // again finds the stick: the bundle is the new one, the vault holds no
    // more files than it did, and the boot partition isn't written to.
    let listed = || std::fs::read_dir(&vault).unwrap().count();
    let (files, boot_partition) = (listed(), std::fs::read(scratch.join("boot.fat")).unwrap());
    stand_in.file(BUNDLE, b"a newer sealed bundle");
    KeepOnStick.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(
        std::fs::read(vault.join("machine-rescue-bundle.json")).unwrap(),
        b"a newer sealed bundle"
    );
    assert_eq!(listed(), files);
    // Said in words, since the whole partition's bytes would bury the
    // failure.
    assert!(
        std::fs::read(scratch.join("boot.fat")).unwrap() == boot_partition,
        "the boot partition its key signed already was changed"
    );
    assert!(boots_with(&scratch, &scratch.join("boot.fat"), &pem));

    // Keys made again, as a second install from the stick does, make it
    // that machine's again.
    let other = Scratch::new("install-keep-on-stick-other");
    let (_, another) = db_key(&other);
    stand_in.file(&format!("{SBCTL}/keys/db/db.pem"), another.as_bytes());
    assert!(matches!(
        KeepOnStick.observe(&mut stand_in).unwrap(),
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
    let (mut stand_in, vault) = installed_on_stick_seen(&scratch, &key, &pem, &seen.to_string());
    stand_in.disk("/dev/vda1", &scratch.join("boot.fat"));
    let before = std::fs::read(scratch.join("boot.fat")).unwrap();
    let result = KeepOnStick.run(&mut stand_in, &Answers::new());
    assert!(
        matches!(&result, Err(Fault::Blocked { why, .. }) if why.contains("no boot partition")),
        "{result:?}"
    );
    assert_eq!(std::fs::read(scratch.join("boot.fat")).unwrap(), before);
    assert!(!vault.join("machine-rescue-bundle.json").exists());
}

#[test]
fn the_stick_is_made_the_machines_own_only_once_the_machine_is_finished() {
    let ids = step_ids(&target("/dev/disk/by-id/virtio-root"));
    let at = |id: &str| ids.iter().position(|found| *found == id).unwrap();
    assert!(at("set-password") < at("keep-on-stick"), "{ids:?}");
    assert!(at("wipe-install-key") < at("keep-on-stick"), "{ids:?}");
    assert_eq!(ids.last(), Some(&"keep-on-stick"), "{ids:?}");
}
