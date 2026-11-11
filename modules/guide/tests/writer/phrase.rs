//! The stick's passphrase: seven words shown for the phone and typed back, kept
//! or chosen afresh when the stick had one, and a kept one proven against the
//! old vault before it seals the new one.

use std::cell::Cell;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::rc::Rc;

use guide::stand_in::StandIn;
use guide::stick::gpt::Table;
use guide::writer::{ChoosePhrase, WriteStick};
use guide::{Answer, Answers, Ask, Fault, Journey, Standing, Step};

use super::{image, lsblk, payload, stick_file};
use crate::common::Scratch;

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
    let mut stand_in = StandIn::new();
    let keep = Rc::new(Cell::new(None));
    let choose = ChoosePhrase {
        sealed: Some(sealed_to_phrase()),
        keep: Rc::clone(&keep),
    };
    let Standing::Needed { asks } = choose.observe(&mut stand_in).unwrap() else {
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
    Journey::attempt(&choose, &mut stand_in, &answers).unwrap();
    assert_eq!(keep.get(), Some(true));

    // A stick the guide never wrote has no passphrase to keep.
    let fresh = ChoosePhrase {
        sealed: None,
        keep: Rc::new(Cell::new(None)),
    };
    assert!(matches!(
        fresh.observe(&mut stand_in).unwrap(),
        Standing::Done { evidence } if evidence.contains("new words")
    ));
}

#[test]
fn new_words_chosen_for_a_stick_that_had_a_passphrase_are_shown_for_the_phone() {
    let directory = Scratch::new("writer-new-phrase");
    let mut stand_in = StandIn::new();
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    stand_in.disk("/dev/sda", &stick_file(&directory));
    let write = WriteStick::new(
        image(&directory),
        Some(payload(
            &directory,
            r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
        )),
        vec![],
    );
    let steps = guide::writer::writing(write, Some(sealed_to_phrase()));
    let [choose, write] = steps.as_slice() else {
        panic!("asked whether to keep it, then written")
    };
    let mut answers = Answers::new();
    answers.give("phrase", Answer::Chosen(1));
    Journey::attempt(choose.as_ref(), &mut stand_in, &answers).unwrap();
    let Standing::Needed { asks } = write.observe(&mut stand_in).unwrap() else {
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
    let directory = Scratch::new("writer-keeps-phrase");
    let stick = stick_file(&directory);
    let mut stand_in = StandIn::new();
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    stand_in.disk("/dev/sda", &stick);
    let step = WriteStick::new(
        image(&directory),
        Some(payload(
            &directory,
            r#"{"machine":"laptop","disk":"/dev/disk/by-id/virtio-root","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[]}"#,
        )),
        vec![],
    )
    .keeping_phrase(sealed_to_phrase(), Rc::new(Cell::new(Some(true))));
    let Standing::Needed { asks } = step.observe(&mut stand_in).unwrap() else {
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
        &mut stand_in,
        &typed("abacus abdomen abdominal abide abiding ability"),
    );
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("isn't the stick's passphrase")),
        "{refused:?}"
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("opening")),
        "{:#?}",
        stand_in.calls
    );

    // Typed as a phone's Live Text gives it, capitals and all.
    Journey::attempt(
        &step,
        &mut stand_in,
        &typed("Abacus abdomen  abdominal abide abiding ability ablaze"),
    )
    .unwrap();
    guide::vault::VaultKey::open_with_passphrase(
        &phrase_on_stick(&stick),
        &guide::Secret::new(PHRASE.into()),
    )
    .unwrap();
}
