//! The spare SSH key on each YubiKey, held against what the machine and
//! the key hold: asked for by the name the stick enrolled it under, made
//! only on a key holding none and downloaded from one that does, the PIN
//! only ever in ssh-keygen's environment, and a key holding another
//! name's credential, or one that doesn't open the disk, refused; a
//! step for each key, each done on its own.

use guide::install::keys::NAMED;
use guide::installed::FACTS;
use guide::installed::keys::ASKPASS;
use guide::installed::spares::{SpareKey, steps};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Ask, Fault, Program, Secret, Standing, Step, Tool};

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

/// The disk's FIDO2 credentials, the Nano's then the 5C's.
fn credentials() -> Vec<String> {
    let header: serde_json::Value =
        serde_json::from_str(fixture!("luks-dump-sealed.json")).unwrap();
    let tokens = header["tokens"].as_object().unwrap();
    let mut ids: Vec<&String> = tokens.keys().collect();
    ids.sort();
    ids.iter()
        .filter_map(|id| tokens[*id]["fido2-credential"].as_str())
        .map(String::from)
        .collect()
}

/// alice's machine, keeping a spare on each of the two YubiKeys the
/// stick enrolled, one canokey plugged in.
fn machine() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.file(
        FACTS,
        br#"{"disk_name":"system","spares":[
            {"account":"alice","folder":"/home/alice/.ssh","comment":"alice@yubikey"}
        ]}"#,
    );
    let [nano, five_c] = credentials()
        .try_into()
        .expect("the disk has two keys' credentials");
    let named = format!(
        r#"[{{"name":"the Nano","credential":"{nano}"}},{{"name":"the 5C","credential":"{five_c}"}}]"#
    );
    stand_in.file(NAMED, named.as_bytes());
    stand_in.prints(Tool::Fido2Token, &["-L"], fixture!("fido2-token-L.txt"));
    stand_in.prints(
        Tool::Fido2Token,
        &["-I", DEVICE],
        fixture!("fido2-token-I.txt"),
    );
    for file in [NANO, FIVE_C] {
        without(&mut stand_in, file);
    }
    for file in ["id_ed25519_sk_rk", "id_ecdsa_sk_rk"] {
        stand_in.no_file(&format!("{MAKING}/{file}"));
    }
    stand_in
}

/// Whether the plugged key holds the next credential asked of it: each
/// answer is given in turn, and the last stays.
fn opens(stand_in: &mut StandIn, opens: bool) {
    let (status, stderr) = if opens {
        (0, "")
    } else {
        (
            1,
            "fido2-assert: fido_dev_get_assert: FIDO_ERR_NO_CREDENTIALS\n",
        )
    };
    stand_in.answers(
        Program::Tool(Tool::Fido2Assert),
        &["-G", "-t", "up=false", DEVICE],
        status,
        "",
        stderr,
    );
}

fn without(stand_in: &mut StandIn, file: &str) {
    stand_in.no_file(file);
    stand_in.no_file(&format!("{file}.pub"));
}

/// Both of `file`'s halves, the public one what ssh-keygen reads from
/// the private one when `matching`.
fn whole(stand_in: &mut StandIn, file: &str, matching: bool) {
    let public = format!("sk-ssh-ed25519@openssh.com AAAA{file} alice@yubikey\n");
    stand_in.file(file, b"-----BEGIN OPENSSH PRIVATE KEY-----");
    stand_in.file(&format!("{file}.pub"), public.as_bytes());
    let read = if matching {
        format!("sk-ssh-ed25519@openssh.com AAAA{file} ssh:\n")
    } else {
        "sk-ssh-ed25519@openssh.com AAAAsomething-else ssh:\n".into()
    };
    stand_in.prints(Tool::SshKeygen, &["-y", "-f", file], &read);
}

fn nano() -> SpareKey {
    SpareKey::new("the Nano")
}

fn five_c() -> SpareKey {
    SpareKey::new("the 5C")
}

fn answers() -> Answers {
    let mut answers = Answers::new();
    answers.give("plugged", Answer::Done);
    answers.give("pin", Answer::Secret(Secret::new(PIN.into())));
    answers
}

/// What looking for a spare on the key runs: its folder in memory, then
/// ssh-keygen downloading whatever it holds, which leaves `left` there.
fn downloads(stand_in: &mut StandIn, left: Option<&str>) {
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
    stand_in.prints(Tool::Mkdir, &["-p", "-m", "700", MAKING], "");
    match left {
        Some(file) => {
            stand_in.prints(
                Tool::Env,
                &download,
                &format!("Saved ED25519-SK key to {file}\n"),
            );
            stand_in.leaves(
                Tool::Env,
                &download,
                &format!("{MAKING}/{file}"),
                b"-----BEGIN OPENSSH PRIVATE KEY-----",
            );
        }
        None => {
            stand_in.prints(Tool::Env, &download, "No keys to download\n");
        }
    }
}

/// What giving the spare in `key` to alice as `file` runs.
fn keeps(stand_in: &mut StandIn, key: &str, file: &str) {
    stand_in.prints(
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
    stand_in.prints(Tool::Chown, &["alice:", key, &format!("{key}.pub")], "");
    stand_in.prints(
        Tool::Mv,
        &["-n", &format!("{key}.pub"), &format!("{file}.pub")],
        "",
    );
    stand_in.prints(Tool::Mv, &["-n", key, file], "");
}

fn makes(stand_in: &mut StandIn, comment: &str) {
    stand_in.prints(
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
fn a_missing_spare_is_asked_for_by_the_name_the_stick_enrolled_its_key_under() {
    let mut stand_in = machine();
    let Standing::Needed { asks } = nano().observe(&mut stand_in).unwrap() else {
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
    let mut stand_in = machine();
    opens(&mut stand_in, true);
    downloads(&mut stand_in, None);
    makes(&mut stand_in, "alice@yubikey nano");
    keeps(&mut stand_in, &format!("{MAKING}/made"), NANO);
    nano().run(&mut stand_in, &answers()).unwrap();

    let keygen: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.contains("ssh-keygen"))
        .collect();
    assert_eq!(keygen.len(), 2, "{:#?}", stand_in.calls);
    for call in &keygen {
        assert!(call.contains(&format!("[{ASKPASS}]")), "{call}");
        assert!(!call.contains(PIN), "{call}");
    }
    assert!(
        stand_in.secrets.iter().any(|(_, value)| value == PIN),
        "{:?}",
        stand_in.secrets
    );
}

#[test]
fn a_key_holding_a_spare_has_it_downloaded_rather_than_made_over() {
    let mut stand_in = machine();
    opens(&mut stand_in, true);
    downloads(&mut stand_in, Some("id_ed25519_sk_rk"));
    let downloaded = format!("{MAKING}/id_ed25519_sk_rk");
    stand_in.prints(
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
    keeps(&mut stand_in, &downloaded, NANO);
    nano().run(&mut stand_in, &answers()).unwrap();
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.contains("-t ed25519-sk")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_key_holding_another_names_credential_is_refused() {
    let mut stand_in = machine();
    // Not the 5C's credential, asked first, but the Nano's.
    opens(&mut stand_in, false);
    opens(&mut stand_in, true);
    let Standing::Needed { asks } = five_c().observe(&mut stand_in).unwrap() else {
        panic!("the 5C's spare is still to keep")
    };
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("plugging it in is something to do")
    };
    assert!(what.contains("Plug in the 5C"), "{what}");
    let refused = five_c().run(&mut stand_in, &answers());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("this key is the Nano, not the 5C")),
        "{refused:?}"
    );
    // Nothing is downloaded from it, or made on it.
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.contains(" -K ") || call.contains(" -t ed25519-sk ")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_key_that_doesnt_open_the_disk_is_refused() {
    let mut stand_in = machine();
    opens(&mut stand_in, false);
    let refused = nano().run(&mut stand_in, &answers());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("doesn't open the disk, so it isn't the Nano")),
        "{refused:?}"
    );
}

#[test]
fn spares_whose_halves_match_are_done_and_nothing_is_asked() {
    let mut stand_in = machine();
    whole(&mut stand_in, NANO, true);
    whole(&mut stand_in, FIVE_C, true);
    for (step, name) in [(nano(), "the Nano"), (five_c(), "the 5C")] {
        let Standing::Done { evidence } = step.observe(&mut stand_in).unwrap() else {
            panic!("{name} holds alice's spare")
        };
        assert_eq!(evidence, format!("{name} holds alice's spare SSH key"));
    }
}

#[test]
fn one_keys_spare_kept_leaves_the_other_still_to_keep() {
    let mut stand_in = machine();
    whole(&mut stand_in, NANO, true);
    assert!(matches!(
        nano().observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
    let Standing::Needed { asks } = five_c().observe(&mut stand_in).unwrap() else {
        panic!("the 5C's spare is still to keep")
    };
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("plugging it in is something to do")
    };
    assert!(what.contains("Plug in the 5C"), "{what}");
}

#[test]
fn each_key_the_stick_named_has_a_step_of_its_own() {
    let mut stand_in = machine();
    let ids: Vec<&str> = steps(&mut stand_in)
        .unwrap()
        .iter()
        .map(|step| step.id())
        .collect();
    assert_eq!(ids, ["spare-nano", "spare-5c"]);
}

#[test]
fn a_public_half_that_isnt_the_spares_blocks() {
    let mut stand_in = machine();
    whole(&mut stand_in, NANO, false);
    let Standing::Blocked { why, .. } = nano().observe(&mut stand_in).unwrap() else {
        panic!("a mismatched spare is left alone")
    };
    assert!(why.contains("isn't the public half"), "{why}");
}

#[test]
fn half_a_spare_blocks_rather_than_being_made_over() {
    let mut stand_in = machine();
    stand_in.file(NANO, b"-----BEGIN OPENSSH PRIVATE KEY-----");
    let Standing::Blocked { why, .. } = nano().observe(&mut stand_in).unwrap() else {
        panic!("half a spare is left alone")
    };
    assert!(why.contains("without its public half"), "{why}");
}

#[test]
fn a_machine_the_guide_didnt_install_has_nothing_to_keep() {
    let mut stand_in = machine();
    stand_in.no_file(NAMED);
    assert!(steps(&mut stand_in).unwrap().is_empty());
    let Standing::Done { evidence } = nano().observe(&mut stand_in).unwrap() else {
        panic!("no YubiKey is named here")
    };
    assert!(
        evidence.contains("didn't name the Nano among this machine's YubiKeys"),
        "{evidence}"
    );
}
