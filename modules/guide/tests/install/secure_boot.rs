//! Secure Boot: the keys the firmware trusts put back or made, refused when it
//! doesn't trust them, and readied inside the installed system when the
//! firmware is in setup mode.

use guide::install::Target;
use guide::install::system::{CheckFirmware, ReadyEnrolment, SecureBootKeys};
use guide::stand_in::StandIn;
use guide::{Answers, Standing, Step, Tool};

use super::{SBCTL, step_ids, stick, stick_carrying, target};
use crate::common::Scratch;

#[test]
fn a_machine_without_secure_boot_is_installed_with_no_keys_to_sign_with() {
    let mut target = target("/dev/disk/by-id/virtio-root");
    target.secure_boot = false;
    let ids = step_ids(&target);
    for absent in [
        "check-firmware",
        "secure-boot-keys",
        "seal-rescue-bundle",
        "ready-enrolment",
        "keep-on-stick",
    ] {
        assert!(!ids.contains(&absent), "{absent} in {ids:?}");
    }
    for kept in [
        "confirm-disk",
        "lay-out-disk",
        "enrol-nano",
        "make-recovery-key",
        "keep-recovery-key",
        "install-system",
        "set-password",
        "wipe-install-key",
    ] {
        assert!(ids.contains(&kept), "{kept} not in {ids:?}");
    }
}

#[test]
fn a_target_that_says_nothing_of_secure_boot_has_it() {
    // Sticks written before the field was there carry targets without it,
    // and every machine they were written for boots with Secure Boot.
    let mut written = serde_json::to_value(target("/dev/disk/by-id/virtio-root")).unwrap();
    written.as_object_mut().unwrap().remove("secure_boot");
    let read: Target = serde_json::from_value(written).unwrap();
    assert!(read.secure_boot);
    assert!(step_ids(&read).contains(&"secure-boot-keys"));
}

const GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
const DB: &str = "/sys/firmware/efi/efivars/db-d719b2cb-3d3a-4596-a3bc-dad00e67656f";

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
fn surface_firmware(stand_in: &mut StandIn, db: &[u8]) {
    stand_in
        .file(
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut stand_in, SURFACE_DB);
    assert!(matches!(
        SecureBootKeys.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    SecureBootKeys.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(
        stand_in.holds(&format!("{SBCTL}/keys/db/db.key")),
        Some(&b"the db key"[..])
    );
    assert_eq!(
        stand_in.holds(&format!("{SBCTL}/GUID")),
        Some(&b"22f0175f-c166-49d2-96ab-c61575cddac5"[..])
    );
    assert!(
        !stand_in.calls.iter().any(|call| call.starts_with("sbctl")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn the_firmware_is_checked_against_the_carried_keys_with_the_stick_out() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-held");
    let carried: &[(&str, &[u8])] = &[("sbctl/keys/db/db.pem", SURFACE_CERT)];
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut stand_in, SURFACE_DB);
    let with_the_stick_in = CheckFirmware.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&with_the_stick_in, Standing::Done { evidence } if evidence.contains("trusts the keys this stick carries")),
        "{with_the_stick_in:?}"
    );
    guide::install::system::hold_carried_db(&mut stand_in).unwrap();
    stand_in.stick_out(true);
    assert_eq!(
        CheckFirmware.observe(&mut stand_in).unwrap(),
        with_the_stick_in
    );
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut stand_in, SURFACE_DB);
    // Secure Boot off and setup mode on, as clearing the keys leaves it,
    // each variable's attributes and then its one byte.
    stand_in
        .file(
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
    stand_in.prints(Tool::Sbctl, &create, "");
    let standing = SecureBootKeys.observe(&mut stand_in).unwrap();
    assert!(matches!(standing, Standing::Needed { .. }), "{standing:?}");
    SecureBootKeys.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        stand_in.calls.iter().any(|call| call.starts_with("sbctl")),
        "{:#?}",
        stand_in.calls
    );
    assert_ne!(
        stand_in.holds(&format!("{SBCTL}/keys/db/db.key")),
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    // The Surface's keys, but the XPS 14's db.
    surface_firmware(
        &mut stand_in,
        hardware!(
            "dell-xps-14-da14260",
            "efivar-db-d719b2cb-3d3a-4596-a3bc-dad00e67656f"
        ),
    );
    let standing = SecureBootKeys.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("trusts other keys")),
        "{standing:?}"
    );
}

#[test]
fn with_no_keys_carried_a_firmware_in_user_mode_has_to_be_cleared_first() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-keys-none");
    let mut stand_in = stick(&scratch, &target);
    surface_firmware(&mut stand_in, SURFACE_DB);
    let standing = SecureBootKeys.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("won't take new keys")),
        "{standing:?}"
    );
}

#[test]
fn a_firmware_that_wont_take_the_keys_stops_the_install_before_the_disk_is_touched() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-firmware-first");
    let mut stand_in = stick(&scratch, &target);
    // A Surface enforcing its own keys, with none carried.
    surface_firmware(&mut stand_in, SURFACE_DB);
    assert!(matches!(
        CheckFirmware.observe(&mut stand_in).unwrap(),
        Standing::Blocked { .. }
    ));

    let ids = step_ids(&target);
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    surface_firmware(&mut stand_in, SURFACE_DB);
    let standing = CheckFirmware.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("trusts the keys this stick carries")),
        "{standing:?}"
    );
}

#[test]
fn once_the_keys_are_made_the_firmware_check_says_what_it_found_not_what_they_are() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-firmware-after-keys");
    let mut stand_in = stick(&scratch, &target);
    stand_in
        .file(&format!("{SBCTL}/keys/db/db.key"), b"key")
        .file(&format!("{SBCTL}/GUID"), b"guid");
    let evidence = |standing: Standing| match standing {
        Standing::Done { evidence } => evidence,
        other => panic!("{other:?}"),
    };
    let firmware = evidence(CheckFirmware.observe(&mut stand_in).unwrap());
    let keys = evidence(SecureBootKeys.observe(&mut stand_in).unwrap());
    assert_ne!(firmware, keys, "each step done is listed once");
    assert!(firmware.contains("firmware"), "{firmware}");
}

/// The installed system's profile leading to the target's system, and
/// that system with core's program to ready its keys, or without.
fn installed(stand_in: &mut StandIn, target: &Target, readies: bool) {
    stand_in.pointer("/mnt/nix/var/nix/profiles/system", &target.system);
    if readies {
        stand_in.pointer(
            &format!("/mnt{}/secure-boot-enrolment", target.system),
            "/nix/store/def-secure-boot-enrolment",
        );
    }
}

/// The firmware in setup mode, or in user mode enforcing.
fn firmware_in_setup(stand_in: &mut StandIn, setup: bool) {
    let (enforcing, setup): (&[u8], &[u8]) = if setup {
        (&[6, 0, 0, 0, 0], &[6, 0, 0, 0, 1])
    } else {
        (&[6, 0, 0, 0, 1], &[6, 0, 0, 0, 0])
    };
    stand_in
        .file(
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
    let mut stand_in = stick(&scratch, &target);
    installed(&mut stand_in, &target, true);
    firmware_in_setup(&mut stand_in, true);
    for file in AUTO {
        stand_in.no_file(file);
    }
    assert!(matches!(
        ReadyEnrolment.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    let program = format!("{}/secure-boot-enrolment", target.system);
    stand_in.prints(Tool::NixosEnter, &["--root", "/mnt", "-c", &program], "");
    ReadyEnrolment.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        stand_in
            .calls
            .iter()
            .any(|call| call == &format!("nixos-enter --root /mnt -c {program} {{PATH}}")),
        "{:#?}",
        stand_in.calls
    );
    // What it left on the boot partition is what proves it.
    for file in AUTO {
        stand_in.file(file, b"an authenticated variable");
    }
    let Standing::Done { evidence } = ReadyEnrolment.observe(&mut stand_in).unwrap() else {
        panic!("the keys are readied")
    };
    assert!(evidence.contains("before the disk is opened"), "{evidence}");
}

#[test]
fn a_firmware_with_keys_of_its_own_gets_nothing_readied() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-nothing");
    let mut stand_in = stick(&scratch, &target);
    installed(&mut stand_in, &target, true);
    firmware_in_setup(&mut stand_in, false);
    let Standing::Done { evidence } = ReadyEnrolment.observe(&mut stand_in).unwrap() else {
        panic!("a firmware with its own keys enrols nothing")
    };
    assert!(evidence.contains("isn't in setup mode"), "{evidence}");
    ReadyEnrolment.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("nixos-enter")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_system_that_readies_no_keys_itself_is_left_to_its_first_boot() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-no-program");
    let mut stand_in = stick(&scratch, &target);
    installed(&mut stand_in, &target, false);
    // The firmware isn't even looked at: nothing here would ready keys.
    assert!(matches!(
        ReadyEnrolment.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn nothing_is_readied_before_the_system_is_installed() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-ready-first");
    let mut stand_in = stick(&scratch, &target);
    stand_in.pointer("/mnt/nix/var/nix/profiles/system", "/nix/store/old-system");
    assert!(matches!(
        ReadyEnrolment.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}
