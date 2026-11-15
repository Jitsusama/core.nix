//! The sorter asked in ways its units never ask it: for its help, with a
//! flag or a word it doesn't know, or short of the process to read back.
//! Each ends at once in the terminal kit's shape, with the code
//! docs/commands.md gives it. None reaches the sorting itself, which
//! would move this machine's own processes.

use std::process::{Command, Output, Stdio};

/// What the sorter says and ends with, run with `args`.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jobs-sorter"))
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("jobs-sorter ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Ends 64, saying `words` and where to look, and nothing on stdout.
fn refused(args: &[&str], words: &str, look: &str) {
    let output = ran(args);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "{args:?} ended {:?}: {why}",
        output.status
    );
    assert!(why.contains(words), "{args:?} never said {words:?}: {why}");
    assert!(
        why.contains(look),
        "{args:?} never said where to look: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "{args:?}'s refusal said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn its_help_opens_with_its_name_and_names_its_commands_and_flags() {
    let output = ran(&["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("jobs-sorter"), "help opened {help:?}");
    for word in ["probe", "swap-in", "--under", "--busy", "--door"] {
        assert!(help.contains(word), "help never named {word}: {help}");
    }
    assert!(
        output.stderr.is_empty(),
        "help said {:?} on stderr",
        said(&output.stderr)
    );
}

#[test]
fn each_command_answers_help_of_its_own() {
    for (command, word) in [("probe", "--every"), ("swap-in", "--owned")] {
        let output = ran(&[command, "--help"]);
        let help = said(&output.stdout);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{command}'s help ended {:?}",
            output.status
        );
        assert!(
            help.starts_with(&format!("jobs-sorter {command}")),
            "{command}'s help opened {help:?}"
        );
        assert!(help.contains(word), "{command}'s help never named {word}");
    }
}

#[test]
fn a_mistyped_flag_names_the_nearest_and_is_refused_as_asked_wrongly() {
    refused(&["--unde", "app.slice"], "--under", "jobs-sorter --help");
}

#[test]
fn a_mistyped_command_names_the_nearest_and_is_refused_as_asked_wrongly() {
    refused(&["prob"], "probe", "jobs-sorter --help");
}

#[test]
fn a_flag_short_of_its_value_is_refused_as_asked_wrongly() {
    refused(&["--busy"], "--busy", "jobs-sorter --help");
}

#[test]
fn a_swap_in_short_of_its_process_is_refused_as_asked_wrongly() {
    refused(&["swap-in"], "PID", "jobs-sorter swap-in --help");
    refused(
        &["swap-in", "1"],
        "isn't a process",
        "jobs-sorter swap-in --help",
    );
}
