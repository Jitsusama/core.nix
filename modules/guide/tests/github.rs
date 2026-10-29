//! `guide github`: GitHub's device sign-in, its code and token never on a
//! command line, and each key an account's modules send to GitHub added
//! once, as the kind they say, titled as they say.

use guide::github::{Account, AddKeys, AskForCode, SignIn};
use guide::installed::FACTS;
use guide::stand_in::StandIn;
use guide::{Answers, Ask, Fault, Program, Standing, Step, Tool};

const HELD: &str = "/run/user/1000/guide-github-code.json";
const STATUS: &[&str] = &["auth", "status", "--hostname", "github.com"];
const ASK: &[&str] = &[
    "--silent",
    "--show-error",
    "--request",
    "POST",
    "--header",
    "Accept: application/json",
    "--data",
    "@-",
];

fn alice() -> Account {
    Account {
        name: "alice".into(),
        home: "/home/alice".into(),
        runtime: "/run/user/1000".into(),
        bus: None,
    }
}

fn curl(url: &str) -> Vec<&str> {
    [ASK, &[url]].concat()
}

/// A machine where gh is signed in or isn't.
fn signed_in(yes: bool) -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.answers(
        Program::Tool(Tool::Gh),
        STATUS,
        i32::from(!yes),
        "",
        if yes {
            ""
        } else {
            "You are not logged into any GitHub hosts."
        },
    );
    stand_in
}

const CODE: &str = r#"{"device_code":"the-device-code","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_at":4102444800}"#;

#[test]
fn the_code_github_gives_is_shown_with_where_to_type_it() {
    let mut stand_in = signed_in(false);
    stand_in.no_file(HELD).answers(
        Program::Tool(Tool::Curl),
        &curl("https://github.com/login/device/code"),
        0,
        r#"{"device_code":"the-device-code","user_code":"WDJB-MJHT","verification_uri":"https://github.com/login/device","expires_in":899,"interval":5}"#,
        "",
    );
    assert!(matches!(
        AskForCode { account: alice() }.observe(&mut stand_in),
        Ok(Standing::Needed { .. })
    ));
    AskForCode { account: alice() }
        .run(&mut stand_in, &Answers::new())
        .unwrap();
    let held = String::from_utf8_lossy(stand_in.holds(HELD).unwrap()).into_owned();
    assert!(held.contains("WDJB-MJHT"), "{held}");

    let Ok(Standing::Needed { asks }) = SignIn { account: alice() }.observe(&mut stand_in) else {
        panic!("not asked")
    };
    let [Ask::Act { what, .. }] = asks.as_slice() else {
        panic!("{asks:?}")
    };
    assert!(
        what.starts_with("On your phone, open https://github.com/login/device and type WDJB-MJHT."),
        "{what}"
    );
    // The form went on curl's standard input, never its command line.
    assert!(
        !stand_in.calls.iter().any(|call| call.contains("client_id")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn the_token_github_hands_out_goes_to_gh_on_its_standard_input() {
    let mut stand_in = signed_in(false);
    stand_in
        .file(HELD, CODE.as_bytes())
        .answers(
            Program::Tool(Tool::Curl),
            &curl("https://github.com/login/oauth/access_token"),
            0,
            r#"{"access_token":"gho_the-token","token_type":"bearer","scope":"repo"}"#,
            "",
        )
        .prints(
            Tool::Gh,
            &[
                "auth",
                "login",
                "--hostname",
                "github.com",
                "--git-protocol",
                "https",
                "--with-token",
            ],
            "",
        );
    SignIn { account: alice() }
        .run(&mut stand_in, &Answers::new())
        .unwrap();
    assert!(
        stand_in
            .secrets
            .iter()
            .any(|(to, secret)| to == "stdin" && secret == "gho_the-token\n"),
        "{:?}",
        stand_in.secrets
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.contains("gho_the-token") || call.contains("the-device-code")),
        "{:#?}",
        stand_in.calls
    );
    assert_eq!(stand_in.holds(HELD), None);
}

#[test]
fn a_code_not_approved_yet_is_kept_to_try_again() {
    let mut stand_in = signed_in(false);
    stand_in.file(HELD, CODE.as_bytes()).answers(
        Program::Tool(Tool::Curl),
        &curl("https://github.com/login/oauth/access_token"),
        0,
        r#"{"error":"authorization_pending"}"#,
        "",
    );
    let refused = SignIn { account: alice() }.run(&mut stand_in, &Answers::new());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("hasn't been told yet")),
        "{refused:?}"
    );
    assert_eq!(stand_in.holds(HELD), Some(CODE.as_bytes()));
}

const FACTS_JSON: &str = r#"{
  "disk_name": "system",
  "keys": [
    {"account": "alice", "file": "/home/alice/.ssh/id_ecdsa", "comment": "alice@machine", "pin": true,
     "github": {"kind": "authentication", "title": "machine TPM"}},
    {"account": "alice", "file": "/home/alice/.local/share/signing/alice-signing", "comment": "alice@machine signing", "pin": true,
     "github": {"kind": "signing", "title": "machine signing"}},
    {"account": "alice", "file": "/home/alice/.ssh/agent-signing", "comment": "alice@machine agents", "pin": false,
     "github": {"kind": "signing", "title": "machine agents"}},
    {"account": "alice", "file": "/home/alice/.ssh/elsewhere", "comment": "alice@machine elsewhere", "pin": false},
    {"account": "bob", "file": "/home/bob/.ssh/id_ecdsa", "comment": "bob@machine", "pin": true,
     "github": {"kind": "authentication", "title": "machine TPM"}}
  ]
}"#;

#[test]
fn only_the_keys_github_lacks_are_added_as_their_modules_say() {
    let mut stand_in = signed_in(true);
    stand_in
        .file(FACTS, FACTS_JSON.as_bytes())
        .file(
            "/home/alice/.ssh/id_ecdsa.pub",
            b"ecdsa-sha2-nistp256 AAAAlogin alice@machine\n",
        )
        .file(
            "/home/alice/.local/share/signing/alice-signing.pub",
            b"ecdsa-sha2-nistp256 AAAAsigning alice@machine signing\n",
        )
        .file(
            "/home/alice/.ssh/agent-signing.pub",
            b"ecdsa-sha2-nistp256 AAAAagents alice@machine agents\n",
        )
        // GitHub holds the login key already, under another comment.
        .prints(
            Tool::Gh,
            &["api", "user/keys", "--jq", ".[].key"],
            "ecdsa-sha2-nistp256 AAAAlogin\nssh-ed25519 AAAAother\n",
        )
        .prints(
            Tool::Gh,
            &["api", "user/ssh_signing_keys", "--jq", ".[].key"],
            "",
        );
    for (file, title) in [
        (
            "/home/alice/.local/share/signing/alice-signing.pub",
            "machine signing",
        ),
        ("/home/alice/.ssh/agent-signing.pub", "machine agents"),
    ] {
        stand_in.prints(
            Tool::Gh,
            &[
                "ssh-key", "add", file, "--title", title, "--type", "signing",
            ],
            "",
        );
    }
    let adding = AddKeys { account: alice() };
    assert!(matches!(
        adding.observe(&mut stand_in),
        Ok(Standing::Needed { .. })
    ));
    adding.run(&mut stand_in, &Answers::new()).unwrap();
    let added: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.contains("ssh-key add"))
        .collect();
    assert_eq!(added.len(), 2, "{added:#?}");
    assert!(
        added.iter().all(|call| call.contains("--type signing")),
        "{added:#?}"
    );
}

#[test]
fn a_machine_with_no_key_for_github_has_nothing_to_add() {
    let mut stand_in = signed_in(false);
    stand_in.file(
        FACTS,
        br#"{"disk_name": "system", "keys": [{"account": "alice", "file": "/home/alice/.ssh/id_ecdsa", "comment": "alice@machine", "pin": true}]}"#,
    );
    let Ok(Standing::Done { evidence }) = AddKeys { account: alice() }.observe(&mut stand_in)
    else {
        panic!("not done")
    };
    assert_eq!(evidence, "no key here goes to GitHub");
}
