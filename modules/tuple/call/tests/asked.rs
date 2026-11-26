//! The daemon asked in ways its unit never asks it: for its help, with a
//! flag it doesn't take, short of a word it needs or with one too many,
//! given a wait of none or of no number, or handed a socket it can't
//! listen on. Each ends at once in the terminal kit's shape, with the
//! code docs/commands.md gives it, before it follows Tuple.

use std::process::{Command, Output, Stdio};

/// What the daemon says and ends with, run with `args`.
fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tuple-call"))
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .env_remove("LISTEN_FDS")
        .env_remove("LISTEN_PID")
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .expect("tuple-call ran")
}

fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
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
    assert!(help.starts_with("tuple-call"), "help opened {help:?}");
    for word in ["SOCKET", "TUPLE"] {
        assert!(help.contains(word), "help never named {word}: {help}");
    }
    assert!(
        output.stderr.is_empty(),
        "help said {:?} on stderr",
        said(&output.stderr)
    );
}

#[test]
fn a_run_short_of_its_tuple_is_refused_as_asked_wrongly() {
    let output = ran(&["tuple-call.sock"]);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(64),
        "a short run ended {:?}",
        output.status
    );
    assert!(
        why.contains("TUPLE"),
        "the refusal never named what was missing: {why}"
    );
    assert!(
        why.contains("tuple-call --help"),
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
        &["tuple-call.sock", "tuple", "more"][..],
        &["--debug", "tuple-call.sock", "tuple"],
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
            why.contains("tuple-call --help"),
            "{words:?}'s refusal never said where: {why}"
        );
    }
}

#[test]
fn a_wait_of_none_or_of_no_number_is_refused_as_asked_wrongly() {
    for (words, flag) in [
        (
            &["--answers-within", "0", "tuple-call.sock", "tuple"][..],
            "--answers-within",
        ),
        (
            &["--held-for", "soon", "tuple-call.sock", "tuple"],
            "--held-for",
        ),
        (&["tuple-call.sock", "tuple", "--held-for"], "--held-for"),
    ] {
        let output = ran(words);
        let why = said(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(64),
            "{words:?} ended {:?}",
            output.status
        );
        let refusal = format!("{flag} takes a whole number of seconds past none");
        assert!(
            why.contains(&refusal),
            "{words:?}'s refusal never said what its wait takes: {why}"
        );
    }
}

#[test]
fn a_socket_it_cant_listen_on_ends_it_as_failed_naming_the_socket() {
    // A folder under a file can't be made, so neither can the socket in it.
    let file = std::env::temp_dir().join(format!("tuple-call-file-{}", std::process::id()));
    std::fs::write(&file, "").expect("a file");
    let socket = file.join("tuple-call.sock");
    let output = ran(&[socket.to_str().expect("a path"), "tuple"]);
    // Removed before the asserts, so a failing run leaves nothing behind.
    let _ = std::fs::remove_file(&file);
    let why = said(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a socket it couldn't listen on ended {:?}",
        output.status
    );
    assert!(
        why.contains("tuple-call.sock"),
        "the failure never named the socket: {why}"
    );
    assert!(
        output.stdout.is_empty(),
        "a failure said {:?} on stdout",
        said(&output.stdout)
    );
}
