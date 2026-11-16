//! The daemon asked in ways its unit never asks it: for its help, with a
//! flag, short of a word it needs or with one too many, or handed sources
//! it can't read. Each ends at once in the terminal kit's shape, with the
//! code docs/commands.md gives it, before it listens.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// What the daemon says and ends with, run with `args`.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_people"))
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .env_remove("LISTEN_PID")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("people ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A folder of its own for one test, so none reads another's files.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("people-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    folder
}

#[test]
fn its_help_opens_with_its_name_and_names_the_words_its_unit_hands_it() {
    let output = ran(&["--help"]);
    let help = said(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "help ended {:?}",
        output.status
    );
    assert!(help.starts_with("people"), "help opened {help:?}");
    for word in ["SOCKET", "SOURCES", "WORDS"] {
        assert!(help.contains(word), "help never named {word}: {help}");
    }
    assert!(
        output.stderr.is_empty(),
        "help said {:?} on stderr",
        said(&output.stderr)
    );
}

#[test]
fn a_run_short_of_its_sources_is_refused_as_asked_wrongly() {
    let output = ran(&["people.sock"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a short run ended {:?}",
        output.status
    );
    assert!(
        why.contains("SOURCES"),
        "the refusal never named what was missing: {why}"
    );
    assert!(
        why.contains("people --help"),
        "the refusal never said where to look: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "a refusal said {:?} on stdout",
        said(&output.stdout)
    );
}

#[test]
fn a_word_too_many_or_a_flag_is_refused_as_asked_wrongly() {
    for words in [
        &["people.sock", "sources.json", "words.json", "more"][..],
        &["--debug", "people.sock", "sources.json"],
    ] {
        let output = ran(words);
        let why = said(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(64),
            "{words:?} ended {:?}",
            output.status
        );
        assert!(
            why.contains("people --help"),
            "{words:?}'s refusal never said where: {why}"
        );
    }
}

#[test]
fn sources_it_cant_read_end_it_as_its_configuration() {
    let folder = folder("sources");
    let socket = folder.join("people.sock");
    let output = ran(&[
        socket.to_str().expect("a path"),
        folder.join("no-sources.json").to_str().expect("a path"),
    ]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(78),
        "unread sources ended {:?}",
        output.status
    );
    assert!(
        why.contains("no-sources.json"),
        "the failure never named the file: {why}"
    );
    assert!(!socket.exists(), "it listened before reading its sources");
    let _ = std::fs::remove_dir_all(&folder);
}
