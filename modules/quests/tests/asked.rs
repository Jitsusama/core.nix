//! The daemon asked in ways its unit never asks it: for its help, with a
//! flag it doesn't know, short of one it needs, or handed words it can't
//! read. Each ends at once in the terminal kit's shape, with the code
//! docs/commands.md gives it, before it listens on anything.

use std::process::{Command, Output, Stdio};

/// What the daemon says and ends with, run with `args`.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quests"))
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("quests ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn its_help_opens_with_its_name_and_names_what_its_unit_hands_it() {
    let output = ran(&["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("quests"), "help opened {help:?}");
    for flag in ["--vocabulary", "--node", "--socket"] {
        assert!(help.contains(flag), "help never named {flag}: {help}");
    }
    assert!(
        output.stderr.is_empty(),
        "help said {:?} on stderr",
        said(&output.stderr)
    );
}

#[test]
fn a_flag_it_doesnt_know_is_refused_with_the_nearest() {
    let output = ran(&["--vocabulry", "words.json", "--node", "n", "--socket", "s"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a wrong flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--vocabulry"),
        "the refusal never named the flag: {why}"
    );
    assert!(
        why.contains("--vocabulary"),
        "the refusal never named the nearest: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "a refusal said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn a_run_short_of_what_it_needs_is_refused_as_asked_wrongly() {
    let output = ran(&["--vocabulary", "words.json", "--socket", "s"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a short run ended {:?}",
        output.status
    );
    assert!(
        why.contains("--node"),
        "the refusal never named what was missing: {why}"
    );
    assert!(
        why.contains("quests --help"),
        "the refusal never said where to look: {why}"
    );
}

#[test]
fn a_flag_with_no_value_is_refused_as_asked_wrongly() {
    let output = ran(&["--node", "n", "--socket", "s", "--vocabulary"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a bare flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--vocabulary"),
        "the refusal never named the flag: {why}"
    );
}

#[test]
fn words_it_cant_read_end_it_as_its_configuration() {
    let folder = std::env::temp_dir().join(format!("quests-asked-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let socket = folder.join("quests.sock");
    let missing = folder.join("no-words.json");
    let output = ran(&[
        "--vocabulary",
        missing.to_str().expect("a path"),
        "--node",
        folder.join("node.sock").to_str().expect("a path"),
        "--socket",
        socket.to_str().expect("a path"),
    ]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(78),
        "unread words ended {:?}",
        output.status
    );
    assert!(
        why.contains("no-words.json"),
        "the failure never named the file: {why}"
    );
    assert!(!socket.exists(), "it listened before reading its words");
    let _ = std::fs::remove_dir_all(&folder);
}
