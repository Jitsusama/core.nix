//! pain-points asked in ways its unit never asks it: bare, for its help,
//! with a flag it doesn't know, or a time it can't read. Each ends at once
//! in the terminal kit's shape, with the code docs/commands.md gives it,
//! before it reads or writes anything.

use std::process::{Command, Output, Stdio};

/// What pain-points says and ends with, run with `args` over a home with
/// nothing in it.
fn ran(args: &[&str]) -> Output {
    let home = std::env::temp_dir().join(format!("pain-points-asked-{}", std::process::id()));
    Command::new(env!("CARGO_BIN_EXE_pain-points"))
        .args(args)
        .env("HOME", &home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env_remove("JOURNAL_STREAM")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("pain-points ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn bare_it_says_how_to_ask_and_ends_done() {
    let output = ran(&[]);
    let brief = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a bare run ended {:?}",
        output.status
    );
    assert!(
        brief.contains("pain-points read"),
        "a bare run never said to read: {brief}"
    );
}

#[test]
fn reads_help_names_what_its_unit_hands_it() {
    let output = ran(&["read", "--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("pain-points read"), "help opened {help:?}");
    for flag in [
        "--trials",
        "--sessions",
        "--journalctl",
        "--chrome",
        "--date",
        "--at",
    ] {
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
    let output = ran(&["read", "--journalct", "journalctl"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a wrong flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--journalct"),
        "the refusal never named the flag: {why}"
    );
    assert!(
        why.contains("--journalctl"),
        "the refusal never named the nearest: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "a refusal said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn a_time_it_cant_read_is_refused_rather_than_taken_as_now() {
    let output = ran(&["read", "--at", "yesterday"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "an unread time ended {:?}",
        output.status
    );
    assert!(
        why.contains("yesterday"),
        "the refusal never named the time: {why}"
    );
    assert!(
        why.contains("pain-points read --help"),
        "the refusal never said where: {why}"
    );
}

#[test]
fn a_flag_with_no_value_is_refused_as_asked_wrongly() {
    let output = ran(&["read", "--date"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a bare flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--date"),
        "the refusal never named the flag: {why}"
    );
}

#[test]
fn hears_help_names_what_its_unit_hands_it() {
    let output = ran(&["hear", "--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("pain-points hear"), "help opened {help:?}");
    for flag in [
        "--sessions",
        "--ledgers",
        "--void",
        "--mycelium",
        "--date",
        "--at",
    ] {
        assert!(help.contains(flag), "help never named {flag}: {help}");
    }
}

#[test]
fn hear_refuses_a_flag_it_doesnt_know_with_the_nearest() {
    let output = ran(&["hear", "--sesions", "x"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a wrong flag ended {:?}",
        output.status
    );
    assert!(
        why.contains("--sessions"),
        "the refusal never named the nearest: {why}"
    );
    assert!(
        why.contains("pain-points hear --help"),
        "the refusal never said where: {why}"
    );
}

#[test]
fn hear_over_nothing_hears_nothing_and_says_the_hour() {
    let home = std::env::temp_dir().join(format!("pain-points-hear-{}", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_pain-points"))
        .args([
            "hear",
            "--mycelium",
            "/nowhere/mycelium",
            "--at",
            "1794859200000",
        ])
        .env("HOME", &home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env_remove("JOURNAL_STREAM")
        .stdin(Stdio::null())
        .output()
        .expect("pain-points ran");
    assert_eq!(
        output.status.code(),
        Some(0),
        "a quiet hear ended {:?}: {}",
        output.status,
        said(&output.stderr)
    );
    assert!(
        said(&output.stdout).contains("0 pains heard"),
        "it never said the hour: {}",
        said(&output.stdout)
    );
    assert!(
        home.join("state/pain-points/hour.json").is_file(),
        "no hour.json for cadence"
    );
}
