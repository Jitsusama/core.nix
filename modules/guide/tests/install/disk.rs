//! The disk installed onto: named and typed back before it's wiped, never a
//! removable one, an old volume wiped unless the install laid it out, and the
//! copy onto it measured.

use guide::install::disk::{ConfirmDisk, LayOutDisk};
use guide::install::system;
use guide::install::{SYSTEM_CACHE, Target};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Fault, Journey, Machine, Program, Standing, Step, Tool};

use super::{lsblk, stick, stick_carrying, target, wiped};
use crate::common::Scratch;

fn typed(text: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("machine", Answer::Typed(text.into()));
    answers
}

#[test]
fn the_disk_is_named_and_the_machine_typed_back_before_anything_is_wiped() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-confirm");
    let mut stand_in = stick(&scratch, &target);
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    let Standing::Needed { asks } = ConfirmDisk.observe(&mut stand_in).unwrap() else {
        panic!("the disk is still to confirm")
    };
    assert!(
        format!("{asks:?}").contains("expected: \"machine\""),
        "{asks:?}"
    );

    let refused = Journey::attempt(&ConfirmDisk, &mut stand_in, &typed("machin"));
    assert!(matches!(refused, Err(Fault::Blocked { .. })), "{refused:?}");
    // Run itself, not only observing first, refuses a disk nobody confirmed.
    let laid_out = LayOutDisk.run(&mut stand_in, &Answers::new());
    assert!(
        matches!(&laid_out, Err(Fault::Blocked { why, .. }) if why.contains("hasn't been confirmed")),
        "{laid_out:?}"
    );
    assert!(!wiped(&stand_in), "{:#?}", stand_in.calls);

    Journey::attempt(&ConfirmDisk, &mut stand_in, &typed("machine")).unwrap();
}

#[test]
fn a_removable_disk_is_never_the_one_installed_onto() {
    // The stick itself, which lsblk says is on USB.
    let target = target("/dev/disk/by-id/usb-stick");
    let scratch = Scratch::new("install-removable");
    let mut stand_in = stick(&scratch, &target);
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    let standing = ConfirmDisk.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Blocked { why, .. } if why.contains("removable")),
        "{standing:?}"
    );
}

/// A disk holding the machine's volume, open and mounted at the install's
/// root, whoever opened it.
fn opened_at_the_root(scratch: &Scratch, target: &Target) -> StandIn {
    let mut stand_in = stick(scratch, target);
    lsblk(&mut stand_in, fixture!("lsblk-laid-out.json"));
    stand_in
        .link("/dev/disk/by-partlabel/system", Some("/dev/vda2"))
        .link("/dev/mapper/system", Some("/dev/dm-1"))
        .answers(
            Program::Tool(Tool::Cryptsetup),
            &["isLuks", "/dev/vda2"],
            0,
            "",
            "",
        );
    stand_in
}

#[test]
fn a_disk_laid_out_and_open_at_the_root_stays_done() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-laid-out");
    let mut stand_in = opened_at_the_root(&scratch, &target);
    stand_in
        .journal()
        .record("lay-out-disk", &target.machine)
        .unwrap();
    assert!(matches!(
        LayOutDisk.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

// An old system left open at the root, by Recover or a carry that
// couldn't put it away, looks just like a disk laid out; only the
// journal knows the install laid this one out.
#[test]
fn a_volume_open_at_the_root_that_the_install_didnt_lay_out_is_still_wiped() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-opened-by-another");
    let mut stand_in = opened_at_the_root(&scratch, &target);
    let standing = LayOutDisk.observe(&mut stand_in).unwrap();
    assert!(matches!(standing, Standing::Needed { .. }), "{standing:?}");
}

#[test]
fn a_volume_laid_out_for_another_machine_is_still_wiped() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-laid-out-for-another");
    let mut stand_in = opened_at_the_root(&scratch, &target);
    stand_in
        .journal()
        .record("lay-out-disk", "another")
        .unwrap();
    let standing = LayOutDisk.observe(&mut stand_in).unwrap();
    assert!(matches!(standing, Standing::Needed { .. }), "{standing:?}");
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    let key = guide::install::vault_key(&mut stand_in).unwrap().unwrap();
    stand_in
        .unpack(key.clone(), SYSTEM_CACHE, system::CACHE, system::COPY_OFF)
        .unwrap();
    assert_eq!(
        system::unpacked_size(&mut stand_in, key).unwrap(),
        Some(3500)
    );
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
    let mut stand_in = stick_carrying(&scratch, &target, carried);
    let key = guide::install::vault_key(&mut stand_in).unwrap().unwrap();
    stand_in
        .unpack(key.clone(), SYSTEM_CACHE, system::CACHE, system::COPY_OFF)
        .unwrap();
    assert_eq!(system::unpacked_size(&mut stand_in, key).unwrap(), None);
}
