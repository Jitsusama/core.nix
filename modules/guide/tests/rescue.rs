//! A machine's rescue bundle: its Secure Boot keys read as root, sealed so
//! only its YubiKeys or its recovery passphrase open them, and proven to
//! open again before anything's written. The keys are made by openssl as
//! sbctl makes them, and the YubiKey side is the vault's own sealing,
//! which vm-guide-opens-the-vault proves with a real FIDO2 key.

mod common;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use common::Scratch;
use zeroize::Zeroizing;

use guide::fake::Fake;
use guide::install::Target;
use guide::rescue::{
    Bundle, KEYS, Keys, OpenRescueBundle, RECOVERY_KEY, RECOVERY_PASSPHRASE, SBCTL, SealKeys,
};
use guide::{Answer, Answers, Ask, Fault, Secret, Standing, Step, Tool};

const OWNER: &str = "22f0175f-c166-49d2-96ab-c61575cddac5";

fn target(machine: &str) -> Target {
    serde_json::from_str(&format!(
        r#"{{"machine":"{machine}","disk":"/dev/x","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[],"recipients":[],"start_from_stick":null,"clear_secure_boot_keys":null}}"#
    ))
    .unwrap()
}

/// sbctl's seven files: a real db key and certificate, since a stick is
/// signed with them, and stand-ins for the platform and key exchange keys,
/// which are only carried.
fn files(name: &str) -> BTreeMap<String, Zeroizing<Vec<u8>>> {
    let directory = Scratch::new(name);
    let (key, certificate) = (directory.join("db.key"), directory.join("db.pem"));
    let made = Command::new("openssl")
        .args(["req", "-new", "-x509", "-newkey", "rsa:2048", "-nodes"])
        .args(["-days", "3650", "-subj", "/CN=Database Key", "-keyout"])
        .arg(&key)
        .arg("-out")
        .arg(&certificate)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let mut files = BTreeMap::new();
    for name in KEYS {
        let bytes = match name {
            "GUID" => format!("{OWNER}\n").into_bytes(),
            "keys/db/db.key" => std::fs::read(&key).unwrap(),
            "keys/db/db.pem" => std::fs::read(&certificate).unwrap(),
            other => format!("the {other} of the machine").into_bytes(),
        };
        files.insert(name.to_string(), Zeroizing::new(bytes));
    }
    files
}

fn passphrase() -> Secret {
    Secret::new("a passphrase for the bundle".into())
}

fn sealed(keys: &Keys) -> Vec<u8> {
    Bundle::seal(
        &target("laptop"),
        keys,
        &passphrase(),
        RECOVERY_PASSPHRASE,
        Some(10),
    )
    .unwrap()
}

#[test]
fn a_bundle_opens_with_its_passphrase_to_the_keys_sealed_in_it() {
    let keys = Keys::new(files("bundle-opens")).unwrap();
    let bytes = sealed(&keys);
    let text = String::from_utf8(bytes.clone()).unwrap();
    // Nothing of the keys shows in the bundle but their owner.
    assert!(!text.contains("PRIVATE KEY"), "{text}");
    assert!(!text.contains("of the machine"), "{text}");
    assert!(text.contains(OWNER), "{text}");

    let bundle = Bundle::read(&bytes).unwrap();
    assert_eq!(bundle.machine(), "laptop");
    assert!(bundle.yubikeys().is_empty());
    let opened = bundle.open_with_passphrase(&passphrase()).unwrap();
    assert_eq!(opened, keys);
    assert_eq!(opened.owner(), OWNER);
    // What it opens to signs, as the db key does.
    opened.signer().unwrap();
}

#[test]
fn a_wrong_passphrase_opens_nothing() {
    let bytes = sealed(&Keys::new(files("bundle-wrong")).unwrap());
    let refused = Bundle::read(&bytes)
        .unwrap()
        .open_with_passphrase(&Secret::new("not the passphrase at all".into()));
    assert!(
        matches!(&refused, Err(Fault::Unopened { what, .. }) if what == "the rescue bundle"),
        "{refused:?}"
    );
}

#[test]
fn a_bundle_tells_its_keys_from_others_without_opening() {
    let keys = Keys::new(files("bundle-holds")).unwrap();
    let other = Keys::new(files("bundle-holds-other")).unwrap();
    let bundle = Bundle::read(&sealed(&keys)).unwrap();
    let certificate = |keys: &Keys| {
        keys.files()
            .find(|(name, _)| *name == "keys/db/db.pem")
            .unwrap()
            .1
            .to_vec()
    };
    assert!(bundle.holds(&certificate(&keys)));
    assert!(!bundle.holds(&certificate(&other)));
}

#[test]
fn a_bundle_whose_keys_were_swapped_is_refused() {
    let keys = Keys::new(files("bundle-swapped")).unwrap();
    let other = Keys::new(files("bundle-swapped-other")).unwrap();
    let mut ours: serde_json::Value = serde_json::from_slice(&sealed(&keys)).unwrap();
    let theirs: serde_json::Value = serde_json::from_slice(&sealed(&other)).unwrap();
    // Their keys and the key that opens them, under our name and db.
    for field in ["keys", "to_passphrase"] {
        ours[field] = theirs[field].clone();
    }
    let bytes = serde_json::to_vec(&ours).unwrap();
    let refused = Bundle::read(&bytes)
        .unwrap()
        .open_with_passphrase(&passphrase());
    assert!(
        matches!(&refused, Err(Fault::Unreadable { why, .. }) if why.contains("aren't the ones")),
        "{refused:?}"
    );
}

#[test]
fn keys_missing_any_of_sbctls_files_are_refused() {
    for missing in KEYS {
        let mut files = files("bundle-missing");
        files.remove(missing);
        let refused = Keys::new(files);
        assert!(
            matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains(missing)),
            "{missing}: {refused:?}"
        );
    }
    let mut files = files("bundle-extra");
    files.insert("files.json".into(), Zeroizing::new(vec![]));
    assert!(Keys::new(files).is_err());
}

fn fake_holding(files: &BTreeMap<String, Zeroizing<Vec<u8>>>) -> Fake {
    let mut fake = Fake::new();
    for (name, bytes) in files {
        let path = format!("{SBCTL}/{name}");
        fake.prints(Tool::Cat, &[&path], std::str::from_utf8(bytes).unwrap());
    }
    fake
}

#[test]
fn the_machines_keys_are_each_read_as_root() {
    let files = files("bundle-read");
    let mut fake = fake_holding(&files);
    let keys = Keys::read(&mut fake).unwrap();
    assert_eq!(keys, Keys::new(files).unwrap());
    let expected: Vec<String> = KEYS
        .iter()
        .map(|name| format!("cat {SBCTL}/{name} (as root)"))
        .collect();
    assert_eq!(fake.calls, expected);
}

#[test]
fn a_machine_with_no_keys_of_its_own_is_told_so() {
    let mut fake = Fake::new();
    fake.refuses(
        Tool::Cat,
        &[&format!("{SBCTL}/GUID")],
        1,
        "cat: /var/lib/sbctl/GUID: No such file or directory",
    );
    let refused = Keys::read(&mut fake);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("no Secure Boot keys")),
        "{refused:?}"
    );
}

fn typed(passphrase: &str, again: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("passphrase", Answer::Secret(Secret::new(passphrase.into())));
    answers.give("again", Answer::Secret(Secret::new(again.into())));
    answers
}

#[test]
fn the_bundle_is_written_privately_and_then_done() {
    let files = files("bundle-step");
    let mut fake = fake_holding(&files);
    let path = "/home/alice/laptop-rescue.json";
    fake.no_file(path);
    let step = SealKeys {
        target: target("laptop"),
        bundle: PathBuf::from(path),
    };
    let standing = step.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Needed { asks } if asks.len() == 2),
        "{standing:?}"
    );

    // Two passphrases that differ, or a short one, write nothing.
    let mismatched = typed("a passphrase for it", "another passphrase");
    assert!(step.run(&mut fake, &mismatched).is_err());
    assert!(
        step.run(&mut fake, &typed("too short", "too short"))
            .is_err()
    );
    assert!(fake.holds(path).is_none());

    let chosen = "a passphrase for the bundle";
    step.run(&mut fake, &typed(chosen, chosen)).unwrap();
    let written = fake.holds(path).expect("the bundle").to_vec();
    let opened = Bundle::read(&written)
        .unwrap()
        .open_with_passphrase(&passphrase())
        .unwrap();
    assert_eq!(opened, Keys::new(files).unwrap());
    let standing = step.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("the recovery passphrase")),
        "{standing:?}"
    );
    // No secret of the keys went into the log of what ran.
    assert!(fake.secrets.is_empty(), "{:?}", fake.secrets);
}

#[test]
fn a_bundle_of_old_keys_is_made_again() {
    let old = Keys::new(files("bundle-old")).unwrap();
    let mut fake = fake_holding(&files("bundle-new"));
    let path = "/home/alice/laptop-rescue.json";
    fake.file(path, &sealed(&old));
    let step = SealKeys {
        target: target("laptop"),
        bundle: PathBuf::from(path),
    };
    let standing = step.observe(&mut fake).unwrap();
    assert!(matches!(&standing, Standing::Needed { .. }), "{standing:?}");
}

#[test]
fn a_bundle_says_which_passphrase_opens_it() {
    let keys = Keys::new(files("bundle-called")).unwrap();
    let bytes = Bundle::seal(
        &target("laptop"),
        &keys,
        &passphrase(),
        RECOVERY_KEY,
        Some(10),
    )
    .unwrap();
    let bundle = Bundle::read(&bytes).unwrap();
    assert_eq!(bundle.passphrase(), RECOVERY_KEY);
    let step = OpenRescueBundle {
        bundle,
        opened: Default::default(),
    };
    let standing = step.observe(&mut Fake::new()).unwrap();
    assert!(
        matches!(&standing, Standing::Needed { asks } if matches!(asks.as_slice(), [Ask::Secret { name, .. }] if name.starts_with(RECOVERY_KEY))),
        "{standing:?}"
    );
}

#[test]
fn a_bundle_with_no_yubikey_opens_on_the_host_with_its_passphrase() {
    let keys = Keys::new(files("bundle-host")).unwrap();
    let opened = std::rc::Rc::new(std::cell::RefCell::new(None));
    let step = OpenRescueBundle {
        bundle: Bundle::read(&sealed(&keys)).unwrap(),
        opened: std::rc::Rc::clone(&opened),
    };
    let mut fake = Fake::new();
    let standing = step.observe(&mut fake).unwrap();
    // No key to plug in is asked for when none can open it.
    assert!(
        matches!(&standing, Standing::Needed { asks } if matches!(asks.as_slice(), [Ask::Secret { .. }])),
        "{standing:?}"
    );
    let mut answers = Answers::new();
    answers.give("passphrase", Answer::Secret(passphrase()));
    step.run(&mut fake, &answers).unwrap();
    assert_eq!(opened.borrow().as_ref(), Some(&keys));
    assert!(fake.calls.is_empty(), "{:?}", fake.calls);
    assert!(matches!(
        step.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}
