//! An install that stopped part way: carried on from the disk as it was left,
//! opened by the key it proved, or started again.

use guide::install::disk::OpenStoppedInstall;
use guide::install::{INSTALL_KEY, MOUNT_SCRIPT, Start, Target};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Fault, Machine, Secret, Standing, Step, Tool};

use super::{DUMP, lsblk, plugged, step_ids, stick_carrying, target, wiped};
use crate::common::Scratch;

#[test]
fn carrying_on_opens_the_disk_as_it_was_left_rather_than_wiping_it() {
    let target = target("/dev/disk/by-id/virtio-root");
    let afresh = step_ids(&target);
    let carrying_on: Vec<&str> = guide::install::steps(&target, Start::CarryingOn)
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
fn stopped_with(scratch: &std::path::Path, target: &Target, header: &str) -> StandIn {
    let mut stand_in = stick_carrying(scratch, target, &[(MOUNT_SCRIPT, b"mount it")]);
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    stand_in
        .link("/dev/disk/by-partlabel/system", None)
        .prints(Tool::Cryptsetup, &DUMP, header);
    stand_in
}

/// The same, its header as it was before the install key came off.
fn stopped_on(scratch: &std::path::Path, target: &Target) -> StandIn {
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
    let mut stand_in = stopped_on(&scratch, &target);
    let step = OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    };
    // With no key proven, either may be the one.
    let Standing::Needed { asks } = step.observe(&mut stand_in).unwrap() else {
        panic!("the disk is still to open")
    };
    let said = format!("{asks:?}");
    assert!(said.contains("Plug in the Nano or the 5C"), "{said}");

    stand_in
        .journal()
        .record("enrol-nano", &format!("opened:{}", first_credential()))
        .unwrap();
    let Standing::Needed { asks } = step.observe(&mut stand_in).unwrap() else {
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
fn a_stopped_install_opens_with_the_stick_out_for_the_key() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-stopped-stick-out");
    let mut stand_in = stopped_on(&scratch, &target);
    // Held as the vault opened, with the stick in; then out for the key,
    // as on a laptop whose one port takes the stick or a YubiKey.
    guide::install::hold_target(&mut stand_in).unwrap();
    guide::install::hold_mount_script(&mut stand_in).unwrap();
    stand_in.stick_out(true);
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    stand_in
        .prints(
            Tool::SystemdCryptenroll,
            &[
                "--unlock-fido2-device=/dev/hidraw1",
                "--wipe-slot=password",
                "--password",
                "/dev/disk/by-partlabel/system",
            ],
            "",
        )
        .prints(Tool::Bash, &["/run/guide/mount"], "")
        .prints(Tool::Udevadm, &["settle"], "");
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .run(&mut stand_in, &answers)
    .unwrap();
    assert_eq!(
        stand_in.holds("/run/guide/mount"),
        Some(b"mount it".as_slice())
    );
    assert!(
        stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("bash /run/guide/mount")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_stopped_install_with_no_script_held_and_the_stick_out_changes_nothing() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-stopped-unheld");
    let mut stand_in = stopped_on(&scratch, &target);
    guide::install::hold_target(&mut stand_in).unwrap();
    stand_in.no_file("/run/guide/mount").stick_out(true);
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    let ran = OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .run(&mut stand_in, &answers);
    // Said as the stick being out, to be put back, before the disk's
    // key slots are touched.
    assert!(matches!(ran, Err(Fault::StickOut)), "{ran:?}");
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        stand_in.calls
    );
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
    let mut stand_in = stopped_with(&scratch, &target, &header.to_string());
    let standing = OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .observe(&mut stand_in)
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
    let mut stand_in = stopped_on(&scratch, &target);
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    let enrol = [
        "--unlock-fido2-device=/dev/hidraw1",
        "--wipe-slot=password",
        "--password",
        "/dev/disk/by-partlabel/system",
    ];
    stand_in
        .prints(Tool::SystemdCryptenroll, &enrol, "")
        .prints(Tool::Bash, &["/run/guide/mount"], "")
        .prints(Tool::Udevadm, &["settle"], "");
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    OpenStoppedInstall {
        yubikeys: target.yubikeys.clone(),
    }
    .run(&mut stand_in, &answers)
    .unwrap();

    // The new key held before it's enrolled, given as cryptenroll reads a
    // new password, and the PIN under both names systemd reads it by.
    let key = String::from_utf8(stand_in.holds(INSTALL_KEY).unwrap().to_vec()).unwrap();
    assert_eq!(key.len(), 64, "{key}");
    assert!(
        stand_in
            .secrets
            .contains(&("NEWPASSWORD".into(), key.clone())),
        "{:?}",
        stand_in.secrets
    );
    for name in ["cryptsetup.fido2-pin", "cryptenroll.fido2-pin"] {
        assert!(
            stand_in.secrets.contains(&(name.into(), "123456".into())),
            "{:?}",
            stand_in.secrets
        );
    }
    assert_eq!(
        stand_in.holds("/run/guide/mount"),
        Some(b"mount it".as_slice())
    );
    let at = |call: &str| {
        stand_in
            .calls
            .iter()
            .position(|made| made.starts_with(call))
    };
    assert!(
        at("systemd-cryptenroll").unwrap() < at("bash /run/guide/mount").unwrap(),
        "{:#?}",
        stand_in.calls
    );
    assert!(!wiped(&stand_in), "{:#?}", stand_in.calls);
    assert_eq!(
        stand_in.journal().copy(),
        Some(std::path::Path::new("/mnt/boot/guide/install-journal"))
    );
}
