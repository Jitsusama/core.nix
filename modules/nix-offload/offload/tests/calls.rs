//! The words nix-offload's units, gates and callers give it, each still
//! reaching the subcommand it names now that its ending is the kit's:
//! the kit hands every word on, so a lender's own flags are never
//! refused as unknown, and each refusal ends with the table's code.
//! Each call is asked wrongly on purpose, so it ends before it would
//! start a lender, a keeper or a build.

use std::process::{Command, Output};

/// nix-offload asked `words`, with every place it would write kept in a
/// folder of its own.
fn asked(words: &[&str]) -> (Output, String) {
    let home = tempfile::tempdir().expect("a home folder");
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args(words)
        .env_clear()
        .env("PATH", "/nonexistent")
        .env("HOME", home.path())
        .env("XDG_STATE_HOME", home.path().join("state"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_RUNTIME_DIR", home.path().join("runtime"))
        .env("NIX_OFFLOAD_DIR", home.path().join("lender"))
        .output()
        .expect("nix-offload runs");
    let said = String::from_utf8_lossy(&ran.stderr).into_owned();
    (ran, said)
}

#[test]
fn each_call_its_units_and_gates_make_reaches_its_subcommand() {
    for (words, code, says) in [
        (
            &["serve", "--helper", "h", "--prefix", "p", "--rate", "free"][..],
            64,
            "--rate takes a price",
        ),
        (
            &["serve", "--helper", "h", "--prefix", "p", "--rate", "0.68"][..],
            64,
            "serve needs each of its settings",
        ),
        (
            &["report", "--rate", "free"][..],
            64,
            "--rate takes a price",
        ),
        (&["stand"][..], 64, "stand needs --standing or --beside"),
        (&["lines"][..], 64, "lines needs --config"),
        (
            &["lines", "--config", "/nonexistent"][..],
            78,
            "can't be read",
        ),
        (&["prove-standing"][..], 64, "prove-standing needs --config"),
        (
            &["prove-standing", "--config", "/nonexistent"][..],
            78,
            "can't be read",
        ),
        (
            &["keep", "--folder", "/nonexistent"][..],
            64,
            "started by its socket",
        ),
        (
            &["cargo", "--cargo", "cargo"][..],
            64,
            "cargo takes --cargo",
        ),
        (
            &["relay", "--listen", "a"][..],
            64,
            "relay needs --listen and --to",
        ),
        (&["passed"][..], 64, "passed takes forget COMMIT"),
    ] {
        let (ran, said) = asked(words);
        assert_eq!(ran.status.code(), Some(code), "{words:?}: {said}");
        assert!(said.contains(says), "{words:?} should say {says:?}: {said}");
        assert!(
            !said.contains("no flag called"),
            "{words:?} had a word of its own refused by the kit: {said}"
        );
    }
}

#[test]
fn a_mistyped_subcommand_ends_64_and_says_so() {
    let (ran, said) = asked(&["serv", "--rate", "0.68"]);
    assert_eq!(ran.status.code(), Some(64), "{said}");
    assert!(
        said.contains("no subcommand called serv"),
        "the word is named: {said}"
    );
    let (ran, said) = asked(&[]);
    assert_eq!(ran.status.code(), Some(64), "{said}");
    assert!(
        said.contains("needs a subcommand"),
        "it says what it needs: {said}"
    );
}

#[test]
fn every_ending_is_one_of_the_tables_codes() {
    for words in [
        &["stand"][..],
        &["lines", "--config", "/nonexistent"],
        &["serv"],
    ] {
        let (ran, said) = asked(words);
        let code = ran.status.code().expect("it ended with a code");
        assert!(
            [0, 1, 64, 65, 66, 69, 70, 75, 77, 78].contains(&code),
            "{words:?} ended {code}, which isn't the table's: {said}"
        );
    }
}
