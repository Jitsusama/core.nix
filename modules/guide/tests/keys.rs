//! The SSH keys an account keeps in the TPM, held against what its disk
//! holds: each missing key made as its account, the PIN only ever in the
//! keygen's environment, never an argument, nothing made over half a
//! key that's there already, and a key a cleared TPM refuses made again.

use guide::installed::FACTS;
use guide::installed::keys::{ASKPASS, MakeKeys};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Ask, Fault, Program, Secret, Standing, Step, Tool};

const LOGIN: &str = "/home/alice/.ssh/id_ecdsa";
const AGENTS: &str = "/home/alice/.ssh/agent-signing";
const SIGNING: &str = "/home/alice/.local/share/signing/alice-signing";

/// The machine as core's guide module describes it, with alice's three
/// keys: her login key and her own signing key asking for a PIN, and the
/// agents' signing key asking for none.
fn machine() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.file(
        FACTS,
        format!(
            r#"{{"disk_name":"system","keys":[
                {{"account":"alice","file":"{LOGIN}","comment":"alice@machine","pin":true}},
                {{"account":"alice","file":"{SIGNING}","comment":"alice@machine signing","pin":true}},
                {{"account":"alice","file":"{AGENTS}","comment":"alice@machine agents","pin":false}}
            ]}}"#
        )
        .as_bytes(),
    );
    stand_in
}

fn keygen() -> &'static str {
    Tool::SshTpmKeygen
        .program()
        .expect("the guide is built with ssh-tpm-keygen")
}

fn mkdir() -> &'static str {
    Tool::Mkdir
        .program()
        .expect("the guide is built with mkdir")
}

/// None of `files`' halves is on the disk.
fn without(stand_in: &mut StandIn, files: &[&str]) {
    for file in files {
        stand_in.no_file(&format!("{file}.tpm"));
        stand_in.no_file(&format!("{file}.pub"));
    }
}

/// A key ssh-tpm-keygen made, in a simulator.
const KEY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/tpm-simulator/ssh-tpm-keygen.tpm"
));

/// What the TPM says when a key's private half wasn't sealed by the
/// storage key it's loaded under, as a TPM cleared since does.
const NOT_ITS_SEED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/dell-xps-14-da14260/tpm2-load-another-tpms-key.err"
));

const STORAGE_KEY: &[&str] = &[
    "-T",
    "device:/dev/tpmrm0",
    "-C",
    "o",
    "-g",
    "sha256",
    "-G",
    "ecc256:aes128cfb",
    "-a",
    "fixedtpm|fixedparent|sensitivedataorigin|userwithauth|noda|restricted|decrypt",
    "-c",
    "/run/guide/tpm-key/parent.ctx",
];

const LOAD: &[&str] = &[
    "-T",
    "device:/dev/tpmrm0",
    "-C",
    "/run/guide/tpm-key/parent.ctx",
    "-u",
    "/run/guide/tpm-key/public",
    "-r",
    "/run/guide/tpm-key/private",
    "-c",
    "/run/guide/tpm-key/key.ctx",
];

/// The TPM's storage key made, and each key loaded in turn, the TPM
/// taking those in `taken` and refusing the rest as a cleared TPM does.
fn tpm_loads(stand_in: &mut StandIn, taken: &[bool]) {
    stand_in.prints(Tool::Tpm2Createprimary, STORAGE_KEY, "name: 000b\n");
    for &taken in taken {
        let program = Program::Tool(Tool::Tpm2Load);
        if taken {
            stand_in.answers(program, LOAD, 0, "name: 000b\n", "");
        } else {
            stand_in.answers(program, LOAD, 1, "", NOT_ITS_SEED);
        }
    }
}

/// Both of `file`'s halves, the public one what `--print-pubkey` reads
/// from the private one when `matching`.
fn whole(stand_in: &mut StandIn, file: &str, matching: bool) {
    let public = format!("ecdsa-sha2-nistp256 AAAA{file} alice@machine\n");
    stand_in.file(&format!("{file}.tpm"), KEY);
    stand_in.file(&format!("{file}.pub"), public.as_bytes());
    let read = if matching {
        public
    } else {
        "ecdsa-sha2-nistp256 AAAAsomething-else alice@machine\n".into()
    };
    stand_in.prints(
        Tool::SshTpmKeygen,
        &["--print-pubkey", &format!("{file}.tpm")],
        &read,
    );
}

fn answers(pin: &str, again: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("key-pin", Answer::Secret(Secret::new(pin.into())));
    answers.give("again", Answer::Secret(Secret::new(again.into())));
    answers
}

/// What making `file` as alice runs, in order: its folder, then the key.
fn makes(stand_in: &mut StandIn, file: &str, comment: &str) {
    let folder = file.rsplit_once('/').unwrap().0;
    stand_in.prints(
        Tool::Runuser,
        &["-u", "alice", "--", mkdir(), "-p", "-m", "700", folder],
        "",
    );
    stand_in.prints(
        Tool::Runuser,
        &["-u", "alice", "--", keygen(), "-C", comment, "-f", file],
        "Your identification has been saved\n",
    );
}

#[test]
fn every_missing_key_is_made_as_its_account_the_pin_only_in_its_environment() {
    let mut stand_in = machine();
    without(&mut stand_in, &[LOGIN, SIGNING, AGENTS]);
    let Standing::Needed { asks } = MakeKeys.observe(&mut stand_in).unwrap() else {
        panic!("alice's keys are still to make")
    };
    let ids: Vec<&str> = asks.iter().map(Ask::id).collect();
    assert_eq!(ids, ["key-pin", "again"]);
    let Ask::Secret { name, .. } = &asks[0] else {
        panic!("the PIN is typed as a secret")
    };
    assert!(name.contains("alice's SSH keys"), "{name}");

    makes(&mut stand_in, LOGIN, "alice@machine");
    makes(&mut stand_in, SIGNING, "alice@machine signing");
    makes(&mut stand_in, AGENTS, "alice@machine agents");
    MakeKeys
        .run(&mut stand_in, &answers("2468", "2468"))
        .unwrap();

    let made: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.contains(" -f "))
        .collect();
    assert_eq!(made.len(), 3, "{:#?}", stand_in.calls);
    for call in &made {
        assert!(call.starts_with("runuser -u alice -- "), "{call}");
        assert!(call.ends_with(&format!("[{ASKPASS}]")), "{call}");
        assert!(!call.contains("(as root)"), "{call}");
    }
    assert!(stand_in.calls.iter().all(|call| !call.contains("2468")));
    // The two with a PIN get it; the agents' key gets none.
    let given: Vec<(&str, &str)> = stand_in
        .secrets
        .iter()
        .map(|(place, secret)| (place.as_str(), secret.as_str()))
        .collect();
    assert_eq!(given, [(ASKPASS, "2468"), (ASKPASS, "2468"), (ASKPASS, "")]);
}

#[test]
fn pins_that_differ_make_nothing() {
    let mut stand_in = machine();
    without(&mut stand_in, &[LOGIN, SIGNING, AGENTS]);
    let refused = MakeKeys.run(&mut stand_in, &answers("2468", "2469"));
    assert!(matches!(refused, Err(Fault::Blocked { .. })), "{refused:?}");
    assert!(
        stand_in
            .calls
            .iter()
            .all(|call| !call.starts_with("runuser"))
    );
}

#[test]
fn a_pin_its_askpass_would_trim_makes_nothing() {
    let mut stand_in = machine();
    without(&mut stand_in, &[LOGIN, SIGNING, AGENTS]);
    let refused = MakeKeys.run(&mut stand_in, &answers(" 2468", " 2468"));
    let Err(Fault::Blocked { why, .. }) = refused else {
        panic!("a PIN starting with a space is refused: {refused:?}")
    };
    assert!(why.contains("space"), "{why}");
    assert!(
        stand_in
            .calls
            .iter()
            .all(|call| !call.starts_with("runuser"))
    );
}

#[test]
fn keys_whose_halves_match_are_done_and_nothing_is_asked() {
    let mut stand_in = machine();
    tpm_loads(&mut stand_in, &[true]);
    whole(&mut stand_in, LOGIN, true);
    whole(&mut stand_in, SIGNING, true);
    whole(&mut stand_in, AGENTS, true);
    let Standing::Done { evidence } = MakeKeys.observe(&mut stand_in).unwrap() else {
        panic!("alice's keys are made")
    };
    assert!(evidence.contains("id_ecdsa"), "{evidence}");
    assert!(evidence.contains("agent-signing"), "{evidence}");
}

#[test]
fn only_a_key_without_a_pin_to_make_asks_for_nothing() {
    let mut stand_in = machine();
    tpm_loads(&mut stand_in, &[true]);
    whole(&mut stand_in, LOGIN, true);
    whole(&mut stand_in, SIGNING, true);
    without(&mut stand_in, &[AGENTS]);
    let Standing::Needed { asks } = MakeKeys.observe(&mut stand_in).unwrap() else {
        panic!("the agents' key is still to make")
    };
    assert!(asks.is_empty(), "{asks:?}");
    makes(&mut stand_in, AGENTS, "alice@machine agents");
    MakeKeys.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(stand_in.secrets, [(ASKPASS.to_string(), String::new())]);
}

#[test]
fn half_a_key_blocks_rather_than_being_made_over() {
    for half in [".tpm", ".pub"] {
        let mut stand_in = machine();
        tpm_loads(&mut stand_in, &[true]);
        whole(&mut stand_in, LOGIN, true);
        whole(&mut stand_in, SIGNING, true);
        without(&mut stand_in, &[AGENTS]);
        stand_in.file(&format!("{AGENTS}{half}"), b"left from before");
        assert!(
            matches!(
                MakeKeys.observe(&mut stand_in).unwrap(),
                Standing::Blocked { .. }
            ),
            "{half}"
        );
        let refused = MakeKeys.run(&mut stand_in, &Answers::new());
        assert!(matches!(refused, Err(Fault::Blocked { .. })), "{half}");
        assert!(
            stand_in
                .calls
                .iter()
                .all(|call| !call.starts_with("runuser"))
        );
    }
}

#[test]
fn a_public_half_that_isnt_the_keys_blocks() {
    let mut stand_in = machine();
    whole(&mut stand_in, LOGIN, false);
    whole(&mut stand_in, SIGNING, true);
    whole(&mut stand_in, AGENTS, true);
    let Standing::Blocked { why, .. } = MakeKeys.observe(&mut stand_in).unwrap() else {
        panic!("a public half that isn't the key's blocks")
    };
    assert!(why.contains("isn't the public half"), "{why}");
}

#[test]
fn a_machine_whose_accounts_keep_no_keys_has_nothing_to_make() {
    let mut stand_in = StandIn::new();
    stand_in.file(FACTS, br#"{"disk_name":"system"}"#);
    assert!(matches!(
        MakeKeys.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_key_the_cleared_tpm_refuses_is_made_again_in_place_of_the_old() {
    let mut stand_in = machine();
    whole(&mut stand_in, LOGIN, true);
    whole(&mut stand_in, SIGNING, true);
    whole(&mut stand_in, AGENTS, true);
    // Her signing key was made before the TPM was cleared; the others
    // since. Each look loads all three in turn.
    tpm_loads(&mut stand_in, &[true, false, true, true, false, true]);
    let Standing::Needed { asks } = MakeKeys.observe(&mut stand_in).unwrap() else {
        panic!("the signing key is to make again")
    };
    let Ask::Secret { name, .. } = &asks[0] else {
        panic!("the PIN is typed as a secret")
    };
    assert!(name.contains("alice's new SSH keys"), "{name}");
    assert!(name.contains("the TPM was cleared"), "{name}");

    makes(&mut stand_in, SIGNING, "alice@machine signing");
    MakeKeys
        .run(&mut stand_in, &answers("2468", "2468"))
        .unwrap();
    let made: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.contains(" -f "))
        .collect();
    assert_eq!(made.len(), 1, "{:#?}", stand_in.calls);
    assert!(made[0].contains(SIGNING), "{}", made[0]);
    // Both old halves go first, since ssh-tpm-keygen asks before making
    // one over them.
    let at = |wanted: &str| stand_in.calls.iter().position(|call| call == wanted);
    let keygen = stand_in
        .calls
        .iter()
        .position(|call| call.contains(" -f "))
        .unwrap();
    for half in [".tpm", ".pub"] {
        let removed = at(&format!("removing {SIGNING}{half}")).expect(half);
        assert!(removed < keygen, "{:#?}", stand_in.calls);
    }
    assert!(at(&format!("removing {LOGIN}.tpm")).is_none());
}

#[test]
fn the_tpm_makes_its_storage_key_once_a_look_and_nothing_loaded_stays() {
    let mut stand_in = machine();
    whole(&mut stand_in, LOGIN, true);
    whole(&mut stand_in, SIGNING, true);
    whole(&mut stand_in, AGENTS, true);
    tpm_loads(&mut stand_in, &[true]);
    MakeKeys.observe(&mut stand_in).unwrap();
    let tpm: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.starts_with("tpm2_"))
        .collect();
    assert_eq!(tpm.len(), 4, "{tpm:#?}");
    assert!(tpm[0].starts_with("tpm2_createprimary"), "{tpm:#?}");
    assert!(
        tpm.iter().all(|call| call.ends_with("(as root)")),
        "{tpm:#?}"
    );
    assert!(stand_in.holds("/run/guide/tpm-key/public").is_none());
    assert!(stand_in.holds("/run/guide/tpm-key/private").is_none());
    assert_eq!(
        stand_in.calls.last().map(String::as_str),
        Some("removing /run/guide/tpm-key and all it holds")
    );
}

#[test]
fn a_tpm_refusing_for_another_reason_is_a_fault_and_nothing_is_made_again() {
    let mut stand_in = machine();
    whole(&mut stand_in, LOGIN, true);
    whole(&mut stand_in, SIGNING, true);
    whole(&mut stand_in, AGENTS, true);
    stand_in.prints(Tool::Tpm2Createprimary, STORAGE_KEY, "");
    stand_in.answers(
        Program::Tool(Tool::Tpm2Load),
        LOAD,
        1,
        "",
        "ERROR:esys:src/tss2-esys/api/Esys_Load.c:110:Esys_Load() Esys Finish ErrorCode (0x00000902)\nERROR: Esys_Load(0x902) - tpm:warn(2.0): out of memory for object contexts\n",
    );
    let refused = MakeKeys.observe(&mut stand_in);
    assert!(matches!(refused, Err(Fault::Refused { .. })), "{refused:?}");
    assert!(
        stand_in
            .calls
            .iter()
            .all(|call| !call.starts_with("removing /home"))
    );
}
