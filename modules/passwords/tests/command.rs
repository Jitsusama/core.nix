//! The `passwords` command against the daemon, with a stand-in password
//! manager, read as a program reads it, through a pipe: each command's
//! rows tab-separated, its `--json` the daemon's own answer with no secret
//! but the one `show` asks for, a title or a detail's label naming what it
//! names, each exit status the usage says, what zsh's completion asks
//! answered from what's held, never asking the manager anything, and an
//! audit a row an entry with a problem, saying no secret.

mod common;

use std::process::{Command, Output};

use common::{AUDITING, Breaches, KEEPING, Running, SIGNED_OUT, until};
use serde_json::{Value, json};

/// The command run with these words against the running daemon.
fn passwords(running: &Running, words: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_passwords"))
        .args(words)
        .env("PASSWORDS_SOCKET", &running.socket)
        .output()
        .unwrap()
}

fn said(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn complained(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn status(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn answering(test: &str) -> Running {
    let running = Running::with(test, &[("keeping", KEEPING)]);
    until("the manager answers", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "answering"
    });
    running
}

#[test]
fn a_list_is_a_row_an_entry_tab_separated() {
    let running = answering("list");
    let listed = passwords(&running, &["list"]);
    assert_eq!(status(&listed), 0, "{}", complained(&listed));
    assert_eq!(
        said(&listed),
        "GitHub\tlogin\tada\thttps://github.com\t\tentry:keeping:gh\n\
         Bank Card\tcard\t\t\t\tentry:keeping:card\n\
         Door Codes\tnote\t\t\t\tentry:keeping:note\n"
    );
    let json: Value =
        serde_json::from_str(&said(&passwords(&running, &["list", "--json"]))).unwrap();
    assert_eq!(json[0]["key"], "entry:keeping:gh");
    assert_eq!(json[2]["tags"], json!(["home"]));
}

#[test]
fn a_find_gives_what_it_finds_and_fails_on_nothing() {
    let running = answering("find");
    let found = passwords(&running, &["find", "git"]);
    assert_eq!(status(&found), 0);
    assert!(said(&found).starts_with("GitHub\t"), "{}", said(&found));
    assert_eq!(said(&found).lines().count(), 1);
    let nothing = passwords(&running, &["find", "zebra"]);
    assert_eq!(status(&nothing), 1);
    assert_eq!(said(&nothing), "");
    assert!(complained(&nothing).contains("No entry is found by \"zebra\""));
}

#[test]
fn details_say_every_detail_and_no_secret() {
    let running = answering("details");
    let shown = passwords(&running, &["details", "GitHub"]);
    assert_eq!(status(&shown), 0, "{}", complained(&shown));
    assert_eq!(
        said(&shown),
        "username\t\ttext\t\tada\t\tusername\n\
         password\t\thidden\thidden\t\t\tpassword\n\
         one-time password\t\tcode\thidden\t\t\totp\n"
    );
    let json = said(&passwords(&running, &["details", "GitHub", "--json"]));
    assert!(json.contains("\"username\": \"ada\""), "{json}");
    for secret in ["hunter2", "SEED"] {
        assert!(!json.contains(secret), "{secret} was shown: {json}");
    }
}

#[test]
fn show_gives_a_secret_by_a_title_and_a_label() {
    let running = answering("show");
    let shown = passwords(&running, &["show", "github"]);
    assert_eq!(status(&shown), 0, "{}", complained(&shown));
    assert_eq!(said(&shown), "hunter2\n");
    let named = passwords(&running, &["show", "GitHub", "Password"]);
    assert_eq!(said(&named), "hunter2\n");
    let json: Value = serde_json::from_str(&said(&passwords(
        &running,
        &["--json", "show", "entry:keeping:gh"],
    )))
    .unwrap();
    assert_eq!(json, json!({"value": "hunter2", "detail": "password"}));
    let none = passwords(&running, &["show", "GitHub", "cvv"]);
    assert_eq!(status(&none), 1);
    assert!(
        complained(&none).contains("no detail \"cvv\"; it has username, password"),
        "{}",
        complained(&none)
    );
}

#[test]
fn a_copy_and_a_code_say_how_long_they_last() {
    let running = answering("copy");
    let copied = passwords(&running, &["copy", "GitHub"]);
    assert_eq!(status(&copied), 0, "{}", complained(&copied));
    let copied = said(&copied);
    let (detail, clears) = copied.trim_end().split_once('\t').unwrap();
    assert_eq!(detail, "password");
    assert!(clears.parse::<u64>().unwrap() > passwords::now());
    until("the password is copied", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "hunter2")
    });
    assert_eq!(
        said(&passwords(&running, &["code", "GitHub"])),
        "123456\t12\t30\n"
    );
}

#[test]
fn health_is_a_row_a_manager() {
    let running = answering("health");
    let health = passwords(&running, &["health"]);
    assert_eq!(said(&health), "keeping\tanswering\t\n");
    let json: Value =
        serde_json::from_str(&said(&passwords(&running, &["health", "--json"]))).unwrap();
    assert_eq!(json["domain"], "passwords");
    assert!(json.get("id").is_none(), "{json}");
}

#[test]
fn an_audit_is_a_row_an_entry_with_a_problem() {
    // Have I Been Pwned's stand-in, knowing no password, answers padding.
    let breaches = Breaches::answering(Vec::new());
    let running = Running::with_env(
        "audit",
        &[("auditing", AUDITING)],
        &[("PASSWORDS_BREACHES", &breaches.address)],
    );
    let audited = passwords(&running, &["audit"]);
    assert_eq!(status(&audited), 0, "{}", complained(&audited));
    assert_eq!(
        said(&audited),
        "GitHub\t2\t0\tentry:auditing:gh\n\
         Gist\t2\t0\tentry:auditing:gist\n\
         Mail\t2\t0\tentry:auditing:mail\n"
    );
    let json: Value =
        serde_json::from_str(&said(&passwords(&running, &["audit", "--json"]))).unwrap();
    assert_eq!(json["audited"], 4);
    assert_eq!(
        json["records"][0]["health"],
        json!({"reused": 2, "breached": 0})
    );
    assert!(json.get("id").is_none(), "{json}");
    for said in [said(&audited), json.to_string(), running.logged()] {
        assert!(!said.contains("\"value\""), "{said}");
    }
    assert_eq!(breaches.requests().len(), 4);
}

#[test]
fn each_exit_status_is_the_one_the_usage_says() {
    let running = Running::with("statuses", &[("locked", SIGNED_OUT)]);
    until("the manager has said how it stands", || {
        running.client().ask(&json!({"id": 0, "ask": "hello"}))["state"] == "signed-out"
    });
    let refused = passwords(&running, &["code", "entry:locked:gh"]);
    assert_eq!(status(&refused), 77, "{}", complained(&refused));
    let listed = passwords(&running, &["list"]);
    assert_eq!(status(&listed), 0);
    assert!(
        complained(&listed).contains("locked is signed out: Let the launcher in; passwords"),
        "{}",
        complained(&listed)
    );
    let mistyped = passwords(&running, &["dance"]);
    assert_eq!(status(&mistyped), 64);
    assert!(complained(&mistyped).contains("Exit status:"));
    let help = passwords(&running, &["help"]);
    assert_eq!(status(&help), 0);
    assert!(said(&help).starts_with("usage: passwords"));
    let nowhere = Command::new(env!("CARGO_BIN_EXE_passwords"))
        .arg("list")
        .env("PASSWORDS_SOCKET", running.folder.join("nowhere.sock"))
        .output()
        .unwrap();
    assert_eq!(status(&nowhere), 69);
}

#[test]
fn completion_is_offered_what_is_held_and_asks_no_manager() {
    let running = answering("names");
    // Nothing's held yet, so nothing's offered, and the manager isn't asked.
    assert_eq!(said(&passwords(&running, &["names", "entries"])), "");
    assert_eq!(
        said(&passwords(&running, &["names", "details", "GitHub"])),
        ""
    );
    assert_eq!(running.asked_of("keeping"), "");
    passwords(&running, &["list"]);
    let before = running.asked_of("keeping");
    assert_eq!(
        said(&passwords(&running, &["names", "entries"])),
        "GitHub:login · ada · github.com\n\
         Bank Card:card\n\
         Door Codes:note\n"
    );
    // Held in summary, its details aren't known yet.
    assert_eq!(
        said(&passwords(&running, &["names", "details", "GitHub"])),
        ""
    );
    assert_eq!(running.asked_of("keeping"), before);
    passwords(&running, &["details", "GitHub"]);
    let before = running.asked_of("keeping");
    assert_eq!(
        said(&passwords(&running, &["names", "details", "github"])),
        "username:text\npassword:hidden\none-time password:code\n"
    );
    assert_eq!(
        said(&passwords(&running, &["names", "managers"])),
        "keeping:answering\n"
    );
    assert_eq!(running.asked_of("keeping"), before);
    let completion = said(&passwords(&running, &["completion", "zsh"]));
    assert!(
        completion.starts_with("#compdef passwords\n"),
        "{completion}"
    );
}
