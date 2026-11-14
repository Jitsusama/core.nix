//! The `passwords` command as a program runs it: no command reaches an
//! entry or a secret, since the daemon answers the shell alone, each such
//! command being refused as asked wrongly with nothing on stdout and
//! nothing typed after it said back; a bare run and the help say where
//! passwords are now, in the terminal kit's shape.

use std::process::{Command, Output};

use serde_json::Value;

/// What the command says with these words.
fn passwords(words: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_passwords"))
        .args(words)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn no_command_reaches_an_entry_or_a_secret() {
    for words in [
        &["list"][..],
        &["find", "git"],
        &["details", "GitHub"],
        &["show", "GitHub"],
        &["show", "GitHub", "PIN", "--json"],
        &["copy", "GitHub"],
        &["code", "GitHub"],
        &["names", "entries"],
        &["audit"],
        &["health"],
        &["let-in", "1password"],
    ] {
        let said = passwords(words);
        assert_eq!(said.status.code(), Some(64), "{words:?} wasn't refused");
        assert!(
            said.stdout.is_empty(),
            "{words:?} said something: {:?}",
            text(&said.stdout)
        );
        let complaint = text(&said.stderr);
        assert!(
            complaint.contains(&format!("passwords {} reads no entry any more", words[0])),
            "{words:?} didn't say why: {complaint}"
        );
        for named in &words[1..] {
            assert!(
                !complaint.contains(named),
                "{words:?} said {named:?} back: {complaint}"
            );
        }
    }
}

#[test]
fn a_bare_run_and_the_help_say_passwords_are_in_the_launcher() {
    for words in [&[][..], &["--help"], &["help"]] {
        let said = passwords(words);
        assert_eq!(said.status.code(), Some(0), "{words:?} failed");
        let shown = text(&said.stdout);
        assert!(
            shown.starts_with("passwords  ") && shown.contains("in the launcher alone"),
            "{words:?} doesn't say where passwords are: {shown}"
        );
    }
    let said = passwords(&["--help", "--json"]);
    let help: Value = serde_json::from_str(&text(&said.stdout)).unwrap();
    assert_eq!(help["schema"], "help/1", "the help isn't help/1: {help}");
}

#[test]
fn a_mistyped_command_names_the_nearest() {
    let said = passwords(&["kyes"]);
    assert_eq!(
        said.status.code(),
        Some(64),
        "a mistyped command wasn't refused"
    );
    let complaint = text(&said.stderr);
    assert!(
        complaint.contains("no command called kyes") && complaint.contains("did you mean keys?"),
        "{complaint}"
    );
}
