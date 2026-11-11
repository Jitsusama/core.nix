//! The `passwords` command as a program runs it: no command reaches an
//! entry or a secret, since the daemon answers the shell alone, each such
//! command being refused as mistyped, and the usage says where passwords
//! are now.

use std::process::{Command, Output};

/// What the command says with these words.
fn passwords(words: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_passwords"))
        .args(words)
        .output()
        .unwrap()
}

#[test]
fn no_command_reaches_an_entry_or_a_secret() {
    for words in [
        &["list"][..],
        &["find", "git"],
        &["show", "GitHub"],
        &["copy", "GitHub"],
        &["code", "GitHub"],
        &["audit"],
        &["health"],
        &["let-in", "1password"],
    ] {
        let said = passwords(words);
        assert_eq!(said.status.code(), Some(64), "{words:?} wasn't refused");
        assert!(
            said.stdout.is_empty(),
            "{words:?} said something: {:?}",
            String::from_utf8_lossy(&said.stdout)
        );
    }
}

#[test]
fn the_usage_says_passwords_are_in_the_launcher() {
    let said = passwords(&["help"]);
    assert_eq!(said.status.code(), Some(0), "help failed");
    let usage = String::from_utf8_lossy(&said.stdout);
    assert!(
        usage.contains("Passwords place"),
        "the usage doesn't say where passwords are: {usage}"
    );
}
