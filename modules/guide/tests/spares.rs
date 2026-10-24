//! The spare SSH key on each YubiKey, held against what the machine and
//! the key hold: asked for by the name the rescue bundle gives it, made
//! only on a key holding none and downloaded from one that does, the PIN
//! only ever in ssh-keygen's environment, and a key the guide met under
//! another name, or one that doesn't open the disk, refused.

use guide::fake::Fake;
use guide::installed::FACTS;
use guide::installed::keys::ASKPASS;
use guide::installed::spares::SpareKeys;
use guide::{Answer, Answers, Ask, Fault, Machine, Program, Secret, Standing, Step, Tool, rescue};

const DUMP: &[&str] = &[
    "luksDump",
    "--dump-json-metadata",
    "/dev/disk/by-partlabel/system",
];
const DEVICE: &str = "/dev/hidraw1";
const MAKING: &str = "/run/guide/spare";
const NANO: &str = "/home/alice/.ssh/yubikey-nano";
const FIVE_C: &str = "/home/alice/.ssh/yubikey-5c";
const PIN: &str = "123456";

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

fn program(tool: Tool) -> &'static str {
    tool.program().expect("the guide is built with its tools")
}

/// The disk's first FIDO2 credential, which the Fake's key holds.
fn first_credential() -> String {
    let header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-sealed.json")).unwrap();
    let tokens = header["tokens"].as_object().unwrap();
    let mut ids: Vec<&String> = tokens.keys().collect();
    ids.sort();
    ids.iter()
        .find_map(|id| tokens[*id]["fido2-credential"].as_str())
        .unwrap()
        .to_string()
}

/// alice's machine, keeping a spare on each of the two YubiKeys its
/// rescue bundle names, one canokey plugged in that opens the disk.
fn machine() -> Fake {
    let mut fake = Fake::new();
    fake.file(
        FACTS,
        br#"{"disk_name":"system","spares":[
            {"account":"alice","folder":"/home/alice/.ssh","comment":"alice@yubikey"}
        ]}"#,
    );
    fake.file(
        rescue::KEPT,
        br#"{"format":1,"machine":"machine","opened_by":["the Nano","the 5C","the disk's recovery key"],
            "owner":"o","db":"d","to_keys":"k","to_passphrase":"p","keys":"s"}"#,
    );
    fake.prints(Tool::Fido2Token, &["-L"], fixture!("fido2-token-L.txt"));
    fake.prints(
        Tool::Fido2Token,
        &["-I", DEVICE],
        fixture!("fido2-token-I.txt"),
    );
    fake.prints(Tool::Cryptsetup, DUMP, fixture!("luks-dump-sealed.json"));
    for file in [NANO, FIVE_C] {
        without(&mut fake, file);
    }
    for file in ["id_ed25519_sk_rk", "id_ecdsa_sk_rk"] {
        fake.no_file(&format!("{MAKING}/{file}"));
    }
    fake
}

/// The plugged key holds the disk's first credential, or none of them.
fn opens(fake: &mut Fake, opens: bool) {
    let (status, stderr) = if opens {
        (0, "")
    } else {
        (
            1,
            "fido2-assert: fido_dev_get_assert: FIDO_ERR_NO_CREDENTIALS\n",
        )
    };
    fake.answers(
        Program::Tool(Tool::Fido2Assert),
        &["-G", "-t", "up=false", DEVICE],
        status,
        "",
        stderr,
    );
}

fn without(fake: &mut Fake, file: &str) {
    fake.no_file(file);
    fake.no_file(&format!("{file}.pub"));
}

/// Both of `file`'s halves, the public one what ssh-keygen reads from
/// the private one when `matching`.
fn whole(fake: &mut Fake, file: &str, matching: bool) {
    let public = format!("sk-ssh-ed25519@openssh.com AAAA{file} alice@yubikey\n");
    fake.file(file, b"-----BEGIN OPENSSH PRIVATE KEY-----");
    fake.file(&format!("{file}.pub"), public.as_bytes());
    let read = if matching {
        format!("sk-ssh-ed25519@openssh.com AAAA{file} ssh:\n")
    } else {
        "sk-ssh-ed25519@openssh.com AAAAsomething-else ssh:\n".into()
    };
    fake.prints(Tool::SshKeygen, &["-y", "-f", file], &read);
}

fn answers() -> Answers {
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new(PIN.into())));
    answers
}

/// What looking for a spare on the key runs: its folder in memory, then
/// ssh-keygen downloading whatever it holds, which leaves `left` there.
fn downloads(fake: &mut Fake, left: Option<&str>) {
    let download = [
        "-C",
        MAKING,
        program(Tool::SshKeygen),
        "-K",
        "-N",
        "",
        "-O",
        &format!("device={DEVICE}"),
    ];
    fake.prints(Tool::Mkdir, &["-p", "-m", "700", MAKING], "");
    match left {
        Some(file) => {
            fake.prints(
                Tool::Env,
                &download,
                &format!("Saved ED25519-SK key to {file}\n"),
            );
            fake.leaves(
                Tool::Env,
                &download,
                &format!("{MAKING}/{file}"),
                b"-----BEGIN OPENSSH PRIVATE KEY-----",
            );
        }
        None => {
            fake.prints(Tool::Env, &download, "No keys to download\n");
        }
    }
}

/// What giving the spare in `key` to alice as `file` runs.
fn keeps(fake: &mut Fake, key: &str, file: &str) {
    fake.prints(
        Tool::Runuser,
        &[
            "-u",
            "alice",
            "--",
            program(Tool::Mkdir),
            "-p",
            "-m",
            "700",
            "/home/alice/.ssh",
        ],
        "",
    );
    fake.prints(Tool::Chown, &["alice:", key, &format!("{key}.pub")], "");
    fake.prints(
        Tool::Mv,
        &["-n", &format!("{key}.pub"), &format!("{file}.pub")],
        "",
    );
    fake.prints(Tool::Mv, &["-n", key, file], "");
}

fn makes(fake: &mut Fake, comment: &str) {
    fake.prints(
        Tool::SshKeygen,
        &[
            "-t",
            "ed25519-sk",
            "-O",
            "resident",
            "-O",
            "verify-required",
            "-O",
            &format!("device={DEVICE}"),
            "-N",
            "",
            "-C",
            comment,
            "-f",
            &format!("{MAKING}/made"),
        ],
        "Your identification has been saved\n",
    );
}

#[test]
fn a_missing_spare_is_asked_for_by_the_name_the_bundle_gives_its_key() {
    let mut fake = machine();
    let Standing::Needed { asks } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("the Nano's spare is still to keep")
    };
    let ids: Vec<&str> = asks.iter().map(Ask::id).collect();
    assert_eq!(ids, ["plugged", "pin"]);
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("plugging it in is something to do")
    };
    assert!(
        what.contains("Plug in the Nano, and only the Nano"),
        "{what}"
    );
    let Ask::Secret { name, .. } = &asks[1] else {
        panic!("its PIN is typed as a secret")
    };
    assert_eq!(name, "the Nano's PIN");
}

#[test]
fn a_key_holding_no_spare_has_one_made_as_root_and_given_to_the_account() {
    let mut fake = machine();
    opens(&mut fake, true);
    downloads(&mut fake, None);
    makes(&mut fake, "alice@yubikey nano");
    keeps(&mut fake, &format!("{MAKING}/made"), NANO);
    SpareKeys.run(&mut fake, &answers()).unwrap();

    let keygen: Vec<&String> = fake
        .calls
        .iter()
        .filter(|call| call.contains("ssh-keygen"))
        .collect();
    assert_eq!(keygen.len(), 2, "{:#?}", fake.calls);
    for call in &keygen {
        assert!(call.contains(&format!("[{ASKPASS}]")), "{call}");
        assert!(!call.contains(PIN), "{call}");
    }
    assert!(
        fake.secrets.iter().any(|(_, value)| value == PIN),
        "{:?}",
        fake.secrets
    );
    let remembered = format!("nano:{}", first_credential());
    assert!(fake.journal().knows("spare-keys", &remembered));
}

#[test]
fn a_key_holding_a_spare_has_it_downloaded_rather_than_made_over() {
    let mut fake = machine();
    opens(&mut fake, true);
    downloads(&mut fake, Some("id_ed25519_sk_rk"));
    let downloaded = format!("{MAKING}/id_ed25519_sk_rk");
    fake.prints(
        Tool::SshKeygen,
        &[
            "-c",
            "-C",
            "alice@yubikey nano",
            "-P",
            "",
            "-f",
            &downloaded,
        ],
        "Comment 'ssh:' changed\n",
    );
    keeps(&mut fake, &downloaded, NANO);
    SpareKeys.run(&mut fake, &answers()).unwrap();
    assert!(
        !fake.calls.iter().any(|call| call.contains("-t ed25519-sk")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn a_key_the_guide_met_under_another_name_is_refused() {
    let mut fake = machine();
    opens(&mut fake, true);
    whole(&mut fake, NANO, true);
    fake.journal()
        .record("spare-keys", &format!("nano:{}", first_credential()))
        .unwrap();
    let Standing::Needed { asks } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("the 5C's spare is still to keep")
    };
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("plugging it in is something to do")
    };
    assert!(what.contains("Plug in the 5C"), "{what}");
    let refused = SpareKeys.run(&mut fake, &answers());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("this key is the Nano, not the 5C")),
        "{refused:?}"
    );
    // Nothing is downloaded from it, or made on it.
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.contains(" -K ") || call.contains(" -t ed25519-sk ")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn a_key_that_doesnt_open_the_disk_is_refused() {
    let mut fake = machine();
    opens(&mut fake, false);
    let refused = SpareKeys.run(&mut fake, &answers());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("doesn't open the disk, so it isn't the Nano")),
        "{refused:?}"
    );
}

#[test]
fn spares_whose_halves_match_are_done_and_nothing_is_asked() {
    let mut fake = machine();
    whole(&mut fake, NANO, true);
    whole(&mut fake, FIVE_C, true);
    let Standing::Done { evidence } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("both keys hold alice's spare")
    };
    assert_eq!(
        evidence,
        "the Nano and the 5C each hold alice's spare SSH key"
    );
}

#[test]
fn a_public_half_that_isnt_the_spares_blocks() {
    let mut fake = machine();
    whole(&mut fake, NANO, false);
    let Standing::Blocked { why, .. } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("a mismatched spare is left alone")
    };
    assert!(why.contains("isn't the public half"), "{why}");
}

#[test]
fn half_a_spare_blocks_rather_than_being_made_over() {
    let mut fake = machine();
    fake.file(NANO, b"-----BEGIN OPENSSH PRIVATE KEY-----");
    let Standing::Blocked { why, .. } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("half a spare is left alone")
    };
    assert!(why.contains("without its public half"), "{why}");
}

#[test]
fn a_machine_without_a_bundle_naming_its_keys_has_nothing_to_keep() {
    let mut fake = machine();
    fake.no_file(rescue::KEPT);
    let Standing::Done { evidence } = SpareKeys.observe(&mut fake).unwrap() else {
        panic!("no YubiKey is named here")
    };
    assert!(evidence.contains("names no YubiKeys"), "{evidence}");
}
