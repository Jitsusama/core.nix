//! The manager's command line, as Passwords starts it: a flag it doesn't
//! know, or one without its value, is refused before anything is read, so
//! a manager started wrongly never answers an ask as if it were right.

use std::process::{Command, Output, Stdio};

fn ran(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_passwords-1password"))
        .args(args)
        .env_remove("JOURNAL_STREAM")
        .stdin(Stdio::null())
        .output()
        .expect("the manager runs")
}

#[test]
fn a_flag_it_doesnt_know_is_refused_as_usage() {
    let out = ran(&["--op", "op", "--open-unti", "onepassword.service"]);
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "",
        "it said how it stands"
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("--open-unti"), "the flag isn't named: {said}");
    assert!(
        said.contains("did you mean --open-unit?"),
        "the nearest flag isn't named: {said}"
    );
}

#[test]
fn a_flag_without_its_value_is_refused_as_usage() {
    let out = ran(&["--op", "op", "--release"]);
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "",
        "it said how it stands"
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("--release needs a value"), "{said}");
}

#[test]
fn a_value_left_out_mid_line_is_refused_under_its_own_flag() {
    let out = ran(&["--release", "--op", "op"]);
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "",
        "it said how it stands"
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.contains("--release needs a value"), "{said}");
}
