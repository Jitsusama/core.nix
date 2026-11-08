//! The daemon over its socket, with stand-in password managers of its own:
//! a list holds every entry without a secret, even one a manager gives
//! with its secrets left on; a get reads an entry in full, still without
//! them; a held list or get asks no manager anything; a secret goes only
//! to the client that asked to see it, or to the clipboard, cleared when
//! its time is up; a one-time code says its period, and its detail gives
//! the code, never its seed; a manager signed out is said so and refuses
//! a secret; a watch hears entries arrive; an ask or kind it doesn't
//! know is refused; and an audit, asked by Joel alone, finds the entries
//! sharing a password and those Have I Been Pwned's stand-in has seen in a
//! breach, sending it each hash's five characters alone, and saying no
//! secret, nor any hash, in an answer or a log.

mod common;

use std::path::Path;

use common::{AUDITING, Breaches, KEEPING, Running, SIGNED_OUT, until};
use passwords::audit::hash;
use serde_json::{Value, json};

fn keys(answer: &Value) -> Vec<String> {
    answer["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["key"].as_str().unwrap().to_owned())
        .collect()
}

fn holds_no_secret(said: &Value) {
    let said = said.to_string();
    for secret in ["hunter2", "SEED"] {
        assert!(!said.contains(secret), "{secret} was held: {said}");
    }
}

#[test]
fn a_list_holds_every_entry_and_no_secret() {
    let running = Running::with("list", &[("keeping", KEEPING)]);
    let mut client = running.client();
    until("the manager answers", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "answering"
    });
    let hello = client.ask(&json!({"id": 1, "ask": "hello"}));
    assert_eq!(hello["domain"], "passwords");
    assert_eq!(hello["version"], 1);
    let listed = client.ask(&json!({"id": 2, "ask": "list", "asked": true}));
    assert_eq!(
        keys(&listed),
        [
            "entry:keeping:gh",
            "entry:keeping:card",
            "entry:keeping:note"
        ]
    );
    assert_eq!(listed["records"][0]["username"], "ada");
    let got =
        client.ask(&json!({"id": 3, "ask": "get", "keys": ["entry:keeping:gh"], "asked": true}));
    let details = got["records"][0]["details"].as_array().unwrap();
    assert_eq!(details.len(), 3, "{got}");
    assert_eq!(details[0]["value"], "ada");
    assert_eq!(details[1]["hidden"], true);
    holds_no_secret(&got);
    holds_no_secret(&client.ask(&json!({"id": 4, "ask": "list"})));
    // Once read, the list was asked of the manager just once.
    let log = running.asked_of("keeping");
    assert_eq!(log.matches("\"list\"").count(), 1, "{log}");
    assert!(
        log.contains("\"asked\":true"),
        "Joel's key wasn't passed on: {log}"
    );
}

#[test]
fn a_held_list_or_get_asks_no_manager_anything() {
    let running = Running::with("held", &[("keeping", KEEPING)]);
    let mut client = running.client();
    until("the manager answers", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "answering"
    });
    // Nothing's been read, so nothing's held, and the manager isn't asked.
    let held = client.ask(&json!({"id": 1, "ask": "list", "held": true}));
    assert_eq!(held["records"], json!([]), "{held}");
    let got = client.ask(&json!({"id": 2, "ask": "get", "keys": ["entry:keeping:gh"],
                                 "held": true}));
    assert_eq!(got["records"], json!([]), "{got}");
    assert_eq!(running.asked_of("keeping"), "");
    client.ask(&json!({"id": 3, "ask": "list", "asked": true}));
    // Held in summary, it's answered so, and still nothing more is asked.
    let got = client.ask(&json!({"id": 4, "ask": "get", "keys": ["entry:keeping:gh"],
                                 "held": true}));
    assert_eq!(got["records"][0]["title"], "GitHub");
    assert!(got["records"][0].get("details").is_none(), "{got}");
    let held = client.ask(&json!({"id": 5, "ask": "list", "held": true, "fresh": true}));
    assert_eq!(keys(&held).len(), 3);
    let log = running.asked_of("keeping");
    assert_eq!(log.lines().count(), 1, "{log}");
}

#[test]
fn a_secret_goes_only_where_it_was_asked_for() {
    let running = Running::with("secret", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let shown = client.ask(&json!({"id": 2, "ask": "reveal", "key": "entry:keeping:gh"}));
    assert_eq!(shown["value"], "hunter2");
    assert_eq!(shown["detail"], "password");
    let mut other = running.client();
    holds_no_secret(&other.ask(&json!({"id": 1, "ask": "get", "keys": ["entry:keeping:gh"]})));
    holds_no_secret(&other.ask(&json!({"id": 2, "ask": "search", "words": "git"})));
    let health = other.ask(&json!({"id": 3, "ask": "health"}));
    holds_no_secret(&health);
    assert_eq!(health["entries"], 3);
}

#[test]
fn a_copy_is_marked_sensitive_and_cleared_when_its_time_is_up() {
    let running = Running::with("copy", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    holds_no_secret(&copied);
    until("the password is copied", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "hunter2")
    });
    let flags = std::fs::read_to_string(running.copied().with_extension("flags")).unwrap();
    assert!(
        flags.contains("--sensitive") && flags.contains("--foreground"),
        "{flags}"
    );
    // The copier is ended once its time is up, which clears the clipboard.
    until("the copy is cleared", || !copier_runs(&running.copied()));
}

/// Whether the stand-in copier still holds the clipboard.
fn copier_runs(copied: &Path) -> bool {
    let wanted = copied.to_string_lossy().into_owned();
    std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .any(|process| {
            std::fs::read(process.path().join("environ"))
                .is_ok_and(|environ| String::from_utf8_lossy(&environ).contains(&wanted))
                && std::fs::read_to_string(process.path().join("cmdline"))
                    .is_ok_and(|line| line.contains("copier"))
        })
}

#[test]
fn a_one_time_code_says_how_long_it_lasts() {
    let running = Running::with("code", &[("keeping", KEEPING)]);
    let mut client = running.client();
    let code = client.ask(&json!({"id": 1, "ask": "code", "key": "entry:keeping:gh"}));
    assert_eq!(code["code"], "123456");
    assert_eq!(code["period"], 30);
    assert_eq!(code["left"], 12);
}

#[test]
fn a_one_time_codes_detail_gives_its_code_never_its_seed() {
    let running = Running::with("code-detail", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh",
                                    "detail": "otp"}));
    assert_eq!(copied["copied"], true, "{copied}");
    assert_eq!(copied["detail"], "one-time password");
    // How long the code lasts goes with it, so the shell can say so.
    assert_eq!(copied["period"], 30, "{copied}");
    assert_eq!(copied["left"], 12, "{copied}");
    until("the code is copied", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "123456")
    });
    let shown = client.ask(&json!({"id": 3, "ask": "reveal", "key": "entry:keeping:gh",
                                   "detail": "otp"}));
    assert_eq!(shown["value"], "123456", "{shown}");
    // The seed was never asked of the manager, so it went nowhere.
    let log = std::fs::read_to_string(running.folder.join("keeping.log")).unwrap();
    assert!(!log.contains("\"reveal\""), "the seed was asked for: {log}");
}

#[test]
fn a_manager_signed_out_is_said_so_and_gives_no_secret() {
    let running = Running::with(
        "signed-out",
        &[("locked", SIGNED_OUT), ("keeping", KEEPING)],
    );
    let mut client = running.client();
    until("both managers have said how they stand", || {
        let hello = running.client().ask(&json!({"id": 0, "ask": "hello"}));
        hello["sources"]["locked"]["state"] == "signed-out"
            && hello["sources"]["keeping"]["state"] == "answering"
    });
    let listed = client.ask(&json!({"id": 1, "ask": "list"}));
    assert_eq!(listed["sources"]["locked"]["reason"], "Let the launcher in");
    assert_eq!(
        keys(&listed).len(),
        3,
        "the manager answering wasn't listed: {listed}"
    );
    let refused = client.ask(&json!({"id": 2, "ask": "reveal", "key": "entry:locked:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    let refused = client.ask(&json!({"id": 3, "ask": "code", "key": "entry:locked:gh"}));
    assert_eq!(refused["error"]["code"], "signed-out", "{refused}");
    let let_in = client.ask(&json!({"id": 4, "ask": "let-in", "manager": "locked"}));
    assert_eq!(let_in["error"]["code"], "signed-out", "{let_in}");
    let nobody = client.ask(&json!({"id": 5, "ask": "let-in", "manager": "nobody"}));
    assert_eq!(nobody["error"]["code"], "refused", "{nobody}");
}

#[test]
fn a_watch_hears_entries_arrive() {
    let running = Running::with("watch", &[("keeping", KEEPING)]);
    let mut watching = running.client();
    watching.say(&json!({"id": "w", "ask": "watch"}));
    let first = watching.hear();
    assert_eq!(first["id"], "w");
    assert!(
        first["records"].is_array(),
        "a watch's first answer isn't whole: {first}"
    );
    running
        .client()
        .ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let changed = loop {
        let heard = watching.hear();
        if heard["changed"]
            .as_array()
            .is_some_and(|changed| !changed.is_empty())
        {
            break heard;
        }
    };
    assert_eq!(changed["changed"].as_array().unwrap().len(), 3);
    holds_no_secret(&changed);
}

/// Have I Been Pwned's stand-in, having seen "password" 1,204 times and
/// listing "abc"'s hash only as padding, which counts 0.
fn breaches() -> Breaches {
    let password = hash("password");
    let abc = hash("abc");
    Breaches::answering(vec![
        (
            password[..5].to_owned(),
            format!(
                "0018A45C4D1DEF81644B54AB7F969B88D65:1\r\n{}:1204\r\n",
                &password[5..]
            ),
        ),
        (abc[..5].to_owned(), format!("{}:0\r\n", &abc[5..])),
    ])
}

/// Fails if anything said holds a password the auditing manager keeps,
/// or any part of a hash past the five characters sent.
fn says_no_secret(said: &str) {
    for secret in [
        "\"value\"".to_owned(),
        hash("password")[5..].to_owned(),
        hash("abc")[5..].to_owned(),
    ] {
        assert!(!said.contains(&secret), "{secret} was said: {said}");
    }
}

#[test]
fn an_audit_finds_passwords_reused_and_breached() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    let audited = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(audited["audited"], 4, "{audited}");
    assert_eq!(audited["reused"], 3, "{audited}");
    assert_eq!(audited["breached"], 3, "{audited}");
    assert!(audited.get("unreached").is_none(), "{audited}");
    assert_eq!(
        keys(&audited),
        [
            "entry:auditing:gh",
            "entry:auditing:gist",
            "entry:auditing:mail"
        ]
    );
    // Each of three sharing a password counts the two others.
    assert_eq!(
        audited["records"][0]["health"],
        json!({"reused": 2, "breached": 1204})
    );
    // Bank's hash is in its range only as padding, so it's no breach.
    let listed = client.ask(&json!({"id": 2, "ask": "list", "held": true}));
    assert_eq!(
        listed["records"][3]["health"],
        json!({"reused": 0, "breached": 0})
    );
    // Each range was asked by its five characters alone, padded.
    let requests = breaches.requests();
    let mut paths: Vec<&str> = requests
        .iter()
        .filter_map(|request| request.split_whitespace().nth(1))
        .collect();
    paths.sort_unstable();
    let mut wanted = [
        format!("/range/{}", &hash("password")[..5]),
        format!("/range/{}", &hash("abc")[..5]),
    ];
    wanted.sort();
    assert_eq!(paths, wanted);
    for request in &requests {
        assert!(
            request.to_lowercase().contains("add-padding: true"),
            "{request}"
        );
        says_no_secret(request);
        assert!(!request.contains("abc"), "{request}");
    }
    says_no_secret(&audited.to_string());
    says_no_secret(&listed.to_string());
    says_no_secret(&running.logged());
}

#[test]
fn an_audit_unasked_by_joel_is_refused() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit-unasked",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "audit"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    let refused = client.ask(&json!({"id": 3, "ask": "audit", "asked": false}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    // Nothing was read and nothing sent, and no list says a health.
    assert!(!running.asked_of("auditing").contains("audit"));
    assert!(breaches.requests().is_empty());
    let listed = client.ask(&json!({"id": 4, "ask": "list", "held": true}));
    assert!(
        listed["records"]
            .as_array()
            .unwrap()
            .iter()
            .all(|record| record.get("health").is_none()),
        "{listed}"
    );
}

#[test]
fn an_audit_of_a_manager_signed_out_is_refused() {
    let breaches = breaches();
    let running = Running::with_env(
        "audit-signed-out",
        &[("locked", SIGNED_OUT)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(refused["error"]["code"], "signed-out", "{refused}");
    assert!(breaches.requests().is_empty());
}

#[test]
fn an_audit_unable_to_ask_about_breaches_says_reuse_alone() {
    // A port nothing listens on any more.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}/range/", closed.local_addr().unwrap());
    drop(closed);
    let running = Running::with_env(
        "audit-unreached",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &address)],
    );
    let mut client = running.client();
    let audited = client.ask(&json!({"id": 1, "ask": "audit", "asked": true}));
    assert_eq!(audited["breached"], Value::Null, "{audited}");
    assert!(audited["unreached"].is_string(), "{audited}");
    assert_eq!(audited["records"][0]["health"], json!({"reused": 2}));
    says_no_secret(&audited.to_string());
    says_no_secret(&running.logged());
}

#[test]
fn an_ask_or_a_kind_it_doesnt_know_is_refused() {
    let running = Running::with("unknown", &[("keeping", KEEPING)]);
    let mut client = running.client();
    let refused = client.ask(&json!({"id": 1, "ask": "dance"}));
    assert_eq!(refused["error"]["code"], "unknown-ask");
    let refused = client.ask(&json!({"id": 2, "ask": "list", "kind": "person"}));
    assert_eq!(refused["error"]["code"], "unknown-kind");
}
