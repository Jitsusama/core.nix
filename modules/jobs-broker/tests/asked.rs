//! The broker's command line asked as an agent might ask it: for its
//! help, with a word it doesn't know, short of what a command needs, or
//! with no broker to reach. Each ends in the terminal kit's shape, with
//! the code docs/commands.md gives it. Every run looks for the broker in
//! a runtime folder of its own, which holds none, so nothing here reaches
//! this machine's broker or its systemd.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// A runtime folder of the test's own, empty.
fn runtime(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("jobs-broker-asked-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a runtime folder");
    folder
}

/// What the broker's command line says and ends with, run with `args`.
fn ran(name: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jobs-broker"))
        .args(args)
        .env("NO_COLOR", "1")
        .env("XDG_RUNTIME_DIR", runtime(name))
        .env_remove("DBUS_SESSION_BUS_ADDRESS")
        .stdin(Stdio::null())
        .output()
        .expect("jobs-broker ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Ends `code`, saying `words` and `look`, and nothing on stdout.
fn ended(name: &str, args: &[&str], code: i32, words: &str, look: &str) {
    let output = ran(name, args);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(code),
        "{args:?} ended {:?}: {why}",
        output.status
    );
    assert!(why.contains(words), "{args:?} never said {words:?}: {why}");
    assert!(why.contains(look), "{args:?} never said {look:?}: {why}");
    assert!(
        output.stdout.is_empty(),
        "{args:?} said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn its_help_opens_with_its_name_and_names_its_commands() {
    let output = ran("help", &["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("jobs-broker"), "help opened {help:?}");
    for word in ["run", "status", "serve"] {
        assert!(help.contains(word), "help never named {word}: {help}");
    }
}

#[test]
fn each_command_answers_help_of_its_own() {
    for (command, word) in [
        ("run", "--quest"),
        ("status", "UNIT"),
        ("serve", "--socket"),
    ] {
        let output = ran("helps", &[command, "--help"]);
        let help = said(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{command}'s help ended {:?}",
            output.status
        );
        assert!(
            help.starts_with(&format!("jobs-broker {command}")),
            "{command}'s help opened {help:?}"
        );
        assert!(help.contains(word), "{command}'s help never named {word}");
    }
}

#[test]
fn a_mistyped_command_names_the_nearest_and_is_refused_as_asked_wrongly() {
    ended(
        "typo",
        &["stats", "jobs.slice"],
        64,
        "status",
        "jobs-broker --help",
    );
}

#[test]
fn a_run_short_of_its_command_is_refused_as_asked_wrongly() {
    ended(
        "no-split",
        &["run", "true"],
        64,
        "--",
        "jobs-broker run --help",
    );
    ended(
        "no-command",
        &["run", "--"],
        64,
        "COMMAND",
        "jobs-broker run --help",
    );
}

#[test]
fn a_status_short_of_its_units_is_refused_as_asked_wrongly() {
    ended(
        "no-units",
        &["status"],
        64,
        "UNIT",
        "jobs-broker status --help",
    );
}

#[test]
fn a_run_with_no_broker_never_runs_its_command() {
    let marker = runtime("no-broker").join("ran");
    let marker_path = marker.to_str().expect("a path in words");
    // `--help` after `--` is the command's, so it's carried, not answered.
    ended(
        "no-broker",
        &["run", "--", "touch", marker_path, "--help"],
        1,
        "no scope for touch",
        "the broker",
    );
    assert!(!marker.exists(), "the command ran outside a scope");
}

#[test]
fn a_status_with_no_broker_fails_saying_so() {
    ended(
        "status-no-broker",
        &["status", "jobs.slice"],
        1,
        "the broker",
        "jobs-broker.sock",
    );
}
