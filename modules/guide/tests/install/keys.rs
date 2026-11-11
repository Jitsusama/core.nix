//! YubiKeys enrolled one at a time and never twice, the install's key taken off
//! only once every key and the recovery key open the disk, and the installed
//! system told which credential each named key enrolled.

use guide::install::disk::WipeInstallKey;
use guide::install::keys::{EnrolYubikey, NAMED, NameYubikeys, Named};
use guide::install::{INSTALL_KEY, Target};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Fault, Journey, Machine, Program, Secret, Standing, Step, Tool};

use super::{DUMP, VOLUME, plugged, stick, target};
use crate::common::Scratch;

#[test]
fn a_key_plugged_in_beside_another_is_never_enrolled() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-two-keys");
    let mut stand_in = stick(&scratch, &target);
    plugged(
        &mut stand_in,
        &[("/dev/hidraw1", "canokey"), ("/dev/hidraw2", "canokey")],
    );
    stand_in.prints(
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
    let refused = nano.run(&mut stand_in, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("2 keys are plugged in")),
        "{refused:?}"
    );
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
fn a_touch_or_pin_the_key_refused_is_said_plainly_with_what_to_do() {
    // As a Nano did, touched ahead of its blinks: cryptenroll's own words
    // name libfido2's error, which says nothing to a person.
    let touch = "the Nano didn't get its touch. Press Return to try again, and touch it each time it blinks: twice, a few seconds apart";
    for (said, plainly) in [
        (
            "Failed to ask token for assertion: FIDO_ERR_OPERATION_DENIED",
            touch,
        ),
        (
            "Failed to ask token for assertion: FIDO_ERR_ACTION_TIMEOUT",
            touch,
        ),
        (
            "Failed to generate FIDO2 credential: FIDO_ERR_PIN_INVALID",
            "the Nano didn't take that PIN. Press Return to type it again: a key takes only a few wrong PINs before it locks itself",
        ),
        (
            "Failed to generate FIDO2 credential: FIDO_ERR_PIN_AUTH_BLOCKED",
            "the Nano takes no more PINs after three wrong ones in a row until it's unplugged. Unplug it, plug it back in, then press Return to try again",
        ),
        (
            "Failed to generate FIDO2 credential: FIDO_ERR_PIN_BLOCKED",
            "the Nano has locked its PIN after too many wrong ones, and only a reset frees it, which wipes what it holds. Reset its FIDO2 part and set its PIN again on another computer, then plug it in here and press Return to try again",
        ),
    ] {
        let target = target("/dev/disk/by-id/virtio-root");
        let scratch = Scratch::new("install-touch-missed");
        let mut stand_in = stick(&scratch, &target);
        let mut header: serde_json::Value =
            serde_json::from_str(fixture!("luks-dump-before-wipe.json")).unwrap();
        header["tokens"]
            .as_object_mut()
            .unwrap()
            .retain(|_, token| token["type"] != "systemd-fido2");
        plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
        stand_in
            .prints(
                Tool::Cryptsetup,
                &[
                    "luksDump",
                    "--dump-json-metadata",
                    "/dev/disk/by-partlabel/system",
                ],
                &header.to_string(),
            )
            .refuses(
                Tool::SystemdCryptenroll,
                &[
                    "--unlock-key-file=/run/guide/install-key",
                    "--fido2-device=/dev/hidraw1",
                    "--fido2-with-client-pin=yes",
                    "--fido2-with-user-presence=yes",
                    "/dev/disk/by-partlabel/system",
                ],
                1,
                said,
            );
        let mut answers = Answers::new();
        answers.give("plugged", Answer::Done);
        answers.give("pin", Answer::Secret(Secret::new("123456".into())));
        let refused = EnrolYubikey::new("the Nano").run(&mut stand_in, &answers);
        assert!(
            matches!(&refused, Err(Fault::Blocked { why, .. }) if why == plainly),
            "{said}: {refused:?}"
        );
    }
}

#[test]
fn a_key_that_opens_the_disk_already_isnt_enrolled_again() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-enrolled");
    let mut stand_in = stick(&scratch, &target);
    let header = fixture!("luks-dump-before-wipe.json");
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    stand_in.prints(
        Tool::Cryptsetup,
        &[
            "luksDump",
            "--dump-json-metadata",
            "/dev/disk/by-partlabel/system",
        ],
        header,
    );
    // The plugged key answers a silent assertion for the first credential.
    stand_in.answers(
        Program::Tool(Tool::Fido2Assert),
        &["-G", "-t", "up=false", "/dev/hidraw1"],
        0,
        "",
        "",
    );
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new("123456".into())));
    let refused = EnrolYubikey::new("the 5C").run(&mut stand_in, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the 5C")),
        "{refused:?}"
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        stand_in.calls
    );
    // The PIN never went near the assertion, which asks nothing secret.
    assert!(
        stand_in
            .secrets
            .iter()
            .all(|(_, value)| !value.contains("123456")),
        "{:?}",
        stand_in.secrets
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
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-before-wipe.json"),
    );
    assert!(matches!(
        WipeInstallKey { yubikeys: 3 }
            .observe(&mut stand_in)
            .unwrap(),
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
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(Tool::Cryptsetup, &dump, &header.to_string());
    stand_in.file(INSTALL_KEY, b"0123");
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }
            .observe(&mut stand_in)
            .unwrap(),
        Standing::Blocked { .. }
    ));

    // Both enrolled, the key's slot still there: it's next.
    let scratch = Scratch::new("install-ready");
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-before-wipe.json"),
    );
    stand_in.file(INSTALL_KEY, b"0123");
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }
            .observe(&mut stand_in)
            .unwrap(),
        Standing::Needed { .. }
    ));

    // Every slot a token's, and the key forgotten: done.
    let scratch = Scratch::new("install-wiped");
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(
        Tool::Cryptsetup,
        &dump,
        fixture!("luks-dump-installed.json"),
    );
    assert!(matches!(
        WipeInstallKey { yubikeys: 2 }
            .observe(&mut stand_in)
            .unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn the_install_key_coming_off_takes_the_journals_copy_with_it() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-copy-goes");
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(
        Tool::Cryptsetup,
        &DUMP,
        fixture!("luks-dump-installed.json"),
    );
    stand_in
        .journal()
        .keep_a_copy(std::path::Path::new("/mnt/boot/guide/install-journal"))
        .unwrap();
    // Every slot a token's and the key forgotten, but the copy still says
    // an install is under way: not done until it goes.
    let step = WipeInstallKey { yubikeys: 2 };
    assert!(matches!(
        step.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    Journey::attempt(&step, &mut stand_in, &Answers::new()).unwrap();
    assert_eq!(stand_in.journal().copy(), None);
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
fn enrolled_stick(scratch: &Scratch, target: &Target, both: bool) -> (StandIn, Vec<String>) {
    let mut stand_in = stick(scratch, target);
    stand_in.prints(
        Tool::Cryptsetup,
        &["luksDump", "--dump-json-metadata", VOLUME],
        fixture!("luks-dump-installed.json"),
    );
    let credentials = installed_credentials();
    stand_in
        .journal()
        .record("enrol-nano", &format!("opened:{}", credentials[1]))
        .unwrap();
    if both {
        stand_in
            .journal()
            .record("enrol-5c", &format!("opened:{}", credentials[0]))
            .unwrap();
    }
    (stand_in, credentials)
}

#[test]
fn the_installed_system_is_told_which_credential_each_named_key_enrolled() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named");
    let (mut stand_in, credentials) = enrolled_stick(&scratch, &target, true);
    let kept = format!("/mnt{NAMED}");
    stand_in.no_file(&kept);
    assert!(matches!(
        NameYubikeys.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    NameYubikeys.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        stand_in.calls.contains(&format!("publishing {kept}")),
        "{:#?}",
        stand_in.calls
    );
    let named: Vec<Named> =
        serde_json::from_slice(&stand_in.read(&kept).unwrap().unwrap()).unwrap();
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
    let Standing::Done { evidence } = NameYubikeys.observe(&mut stand_in).unwrap() else {
        panic!("the names are kept")
    };
    assert!(evidence.contains("knows the Nano and the 5C"), "{evidence}");
}

#[test]
fn names_kept_from_another_enrolment_are_written_again() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named-again");
    let (mut stand_in, credentials) = enrolled_stick(&scratch, &target, true);
    // Swapped, as an earlier install's would differ.
    let stale = format!(
        r#"[{{"name":"the Nano","credential":"{}"}},{{"name":"the 5C","credential":"{}"}}]"#,
        credentials[0], credentials[1]
    );
    stand_in.file(&format!("/mnt{NAMED}"), stale.as_bytes());
    assert!(matches!(
        NameYubikeys.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_key_not_yet_enrolled_leaves_nothing_to_name() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-named-early");
    let (mut stand_in, _) = enrolled_stick(&scratch, &target, false);
    let Standing::Blocked { why, .. } = NameYubikeys.observe(&mut stand_in).unwrap() else {
        panic!("the 5C isn't enrolled")
    };
    assert!(why.contains("the 5C isn't enrolled"), "{why}");
    assert!(NameYubikeys.run(&mut stand_in, &Answers::new()).is_err());
}
