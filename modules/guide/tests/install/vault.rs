//! Opening the stick's vault: its passphrase alone opens it, whatever key is
//! plugged in, and what it holds is read with the stick out.

use std::io::Write;
use std::path::PathBuf;

use guide::install::vault::OpenVault;
use guide::install::{Target, VAULT_KEY};
use guide::stand_in::StandIn;
use guide::vault::store::{Directory, Store};
use guide::vault::{self, Manifest};
use guide::{Answer, Answers, Ask, Fault, Secret, Standing, Step};

use super::{plugged, stick, target};
use crate::common::Scratch;

const PASSPHRASE: &str = "the stick's own passphrase";

/// A stand-in machine on the stick, its vault still shut: its key sealed to
/// `PASSPHRASE`, as the writer seals it.
fn shut_stick(directory: &std::path::Path, target: &Target) -> StandIn {
    let mut stand_in = stick(directory, target);
    let key = guide::install::vault_key(&mut stand_in).unwrap().unwrap();
    let mut store = Directory::new(PathBuf::from(directory));
    store
        .create(vault::KEY_PASSPHRASE)
        .unwrap()
        .write_all(
            &key.seal_with_passphrase(&Secret::new(PASSPHRASE.into()), Some(2))
                .unwrap(),
        )
        .unwrap();
    let manifest = Manifest {
        format: 1,
        machine: target.machine.clone(),
        opened_by: vec!["the stick's passphrase".into()],
    };
    store
        .create(vault::MANIFEST)
        .unwrap()
        .write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    stand_in.with_vault(Box::new(store)).no_file(VAULT_KEY);
    stand_in
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
    let mut stand_in = shut_stick(&scratch, &target);
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    assert!(matches!(
        OpenVault.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    OpenVault
        .run(&mut stand_in, &passphrase(PASSPHRASE))
        .unwrap();
    assert!(matches!(
        OpenVault.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn what_isnt_the_passphrase_is_refused_with_a_key_plugged_in_and_the_key_left_alone() {
    // A laptop with one port both fit can't have a key beside the stick,
    // and one was pulled out to try a key; the vault takes the words only.
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-pin");
    let mut stand_in = shut_stick(&scratch, &target);
    plugged(&mut stand_in, &[("/dev/hidraw1", "canokey")]);
    let refused = OpenVault.run(&mut stand_in, &passphrase("123456"));
    assert!(
        matches!(&refused, Err(Fault::Unopened { what, why }) if what == "the stick's vault" && why == "what was typed isn't its passphrase, the seven words in 1Password named \"NixOS Install Phrase\""),
        "{refused:?}"
    );
}

#[test]
fn the_vault_asks_only_for_the_passphrase_by_its_name_on_the_phone() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-one-screen");
    let mut stand_in = shut_stick(&scratch, &target);
    let Standing::Needed { asks } = OpenVault.observe(&mut stand_in).unwrap() else {
        panic!("the vault is shut")
    };
    let [Ask::Secret { name, .. }] = asks.as_slice() else {
        panic!("the passphrase alone: {asks:?}")
    };
    assert!(
        name.contains("1Password on your phone named \"NixOS Install Phrase\""),
        "{name}"
    );
}

#[test]
fn once_the_vault_is_open_the_steps_read_what_it_installs_with_the_stick_out() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-held");
    let mut stand_in = shut_stick(&scratch, &target);
    OpenVault
        .run(&mut stand_in, &passphrase(PASSPHRASE))
        .unwrap();
    stand_in.stick_out(true);
    assert_eq!(guide::install::target(&mut stand_in).unwrap(), target);
    assert!(matches!(
        OpenVault.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
    // What only the vault holds says the stick is out, to be put back.
    assert!(matches!(
        guide::install::vault(&mut stand_in).err(),
        Some(Fault::StickOut)
    ));
    stand_in.stick_out(false);
    assert!(guide::install::vault(&mut stand_in).is_ok());
}

#[test]
fn with_the_stick_out_and_nothing_held_the_target_says_to_put_it_back() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-vault-unheld");
    let mut stand_in = stick(&scratch, &target);
    stand_in
        .no_file(guide::install::HELD_TARGET)
        .stick_out(true);
    assert!(matches!(
        guide::install::target(&mut stand_in),
        Err(Fault::StickOut)
    ));
}
