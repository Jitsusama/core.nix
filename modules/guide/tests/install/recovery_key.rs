//! The recovery key: seven words held before they're enrolled, kept when cut
//! short, shown alone for the phone and typed back from it.

use guide::install::{INSTALL_KEY, RECOVERY_KEY};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Fault, Standing, Step, Tool};

use super::{BUNDLE, IMPORT, TEST, VOLUME, stick, target};
use crate::common::Scratch;

/// The install's disk with both keys enrolled and no recovery key yet: the
/// real header, its recovery token taken out.
fn without_a_recovery_key(stand_in: &mut StandIn) {
    let mut header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-before-wipe.json")).unwrap();
    header["tokens"]
        .as_object_mut()
        .unwrap()
        .retain(|_, token| token["type"] != "systemd-recovery");
    stand_in.prints(
        Tool::Cryptsetup,
        &["luksDump", "--dump-json-metadata", VOLUME],
        &header.to_string(),
    );
}

fn handed<'a>(stand_in: &'a StandIn, place: &str) -> Vec<&'a str> {
    stand_in
        .secrets
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
    let mut stand_in = stick(&scratch, &target);
    without_a_recovery_key(&mut stand_in);
    stand_in.no_file(RECOVERY_KEY);
    // The words open nothing until they're enrolled, then slot 3.
    stand_in
        .refuses(
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
    stand_in
        .prints(
            Tool::SystemdCryptenroll,
            &[&unlock, "--password", VOLUME],
            "",
        )
        .prints(Tool::Cryptsetup, &IMPORT, "");
    assert!(matches!(
        MakeRecoveryKey.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    MakeRecoveryKey.run(&mut stand_in, &Answers::new()).unwrap();

    let words = String::from_utf8(stand_in.holds(RECOVERY_KEY).unwrap().to_vec()).unwrap();
    let each: Vec<&str> = words.split(' ').collect();
    assert_eq!(each.len(), 7, "{words}");
    assert!(each.iter().all(|word| guide::words::list().contains(word)));
    assert_eq!(handed(&stand_in, "NEWPASSWORD"), [words.as_str()]);
    // Marked in the slot the words opened, as systemd marks its own.
    assert!(
        handed(&stand_in, "stdin").contains(&r#"{"type":"systemd-recovery","keyslots":["3"]}"#)
    );
    let at = |start: &str| {
        stand_in
            .calls
            .iter()
            .position(|call| call.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} in {:#?}", stand_in.calls))
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
    let mut stand_in = stick(&scratch, &target);
    without_a_recovery_key(&mut stand_in);
    let held = "abacus abdomen abdominal abide abiding ability ablaze";
    stand_in.file(RECOVERY_KEY, held.as_bytes());
    stand_in
        .prints(Tool::Cryptsetup, &TEST, "Key slot 5 unlocked.\n")
        .prints(Tool::Cryptsetup, &IMPORT, "");
    MakeRecoveryKey.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(stand_in.holds(RECOVERY_KEY), Some(held.as_bytes()));
    assert!(
        !stand_in.calls.iter().any(|call| call.contains(BUNDLE)),
        "{:#?}",
        stand_in.calls
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("systemd-cryptenroll")),
        "{:#?}",
        stand_in.calls
    );
    assert!(
        handed(&stand_in, "stdin").contains(&r#"{"type":"systemd-recovery","keyslots":["5"]}"#)
    );
}

#[test]
fn the_recovery_key_is_shown_alone_for_the_phone_then_typed_back_from_it() {
    use guide::install::keys::KeepRecoveryKey;
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-recovery-kept");
    let mut stand_in = stick(&scratch, &target);
    let held = "abacus abdomen abdominal abide abiding ability ablaze";
    stand_in.file(RECOVERY_KEY, held.as_bytes());
    let standing = KeepRecoveryKey.observe(&mut stand_in).unwrap();
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
    assert!(how.contains("\"Machine Recovery Phrase\""), "{how}");
    assert!(
        how.contains("Copy All") && how.contains("1Password"),
        "{how}"
    );
    // The steps say nothing of the words themselves.
    assert!(!how.contains("abacus"), "{how}");

    // Typed back with a capital and a doubled space, it's the same key,
    // and the key as it's held is what's tried on the disk.
    stand_in.prints(
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
    KeepRecoveryKey.run(&mut stand_in, &answers).unwrap();
    assert_eq!(handed(&stand_in, "stdin"), [held]);
    assert_eq!(stand_in.holds(RECOVERY_KEY), None);

    // A word off by a letter isn't it.
    let mut stand_in = stick(&scratch, &target);
    stand_in.file(RECOVERY_KEY, held.as_bytes());
    let mut answers = Answers::new();
    answers.give(
        "key",
        Answer::Typed("abacus abdomen abdominal abide abiding ability ablase".into()),
    );
    assert!(matches!(
        KeepRecoveryKey.run(&mut stand_in, &answers),
        Err(Fault::Blocked { .. })
    ));
}
