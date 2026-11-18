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
fn a_flag_near_none_of_its_own_is_refused_naming_them_all() {
    let out = ran(&["--zzzzzzzz"]);
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("its flags are --op, --release, --open-unit, --opened and --systemctl"),
        "its flags aren't named: {said}"
    );
}

#[test]
fn a_release_that_isnt_a_1password_reference_is_refused() {
    let out = ran(&["--release", "-x"]);
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "",
        "it said how it stands"
    );
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("--release takes an op:// reference, not -x"),
        "{said}"
    );
}

#[test]
fn a_unit_with_nowhere_to_watch_it_open_is_refused() {
    let out = Command::new(env!("CARGO_BIN_EXE_passwords-1password"))
        .args(["--open-unit", "onepassword.service"])
        .env_remove("JOURNAL_STREAM")
        .env_remove("XDG_RUNTIME_DIR")
        .stdin(Stdio::null())
        .output()
        .expect("the manager runs");
    assert_eq!(out.status.code(), Some(64), "{out:?}");
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(
        said.contains("--open-unit needs --opened"),
        "a unit it could never see open was taken: {said}"
    );
}

#[test]
fn where_the_app_opens_is_refused_without_its_unit() {
    for given in [["--opened", "/run/open"], ["--systemctl", "/bin/systemctl"]] {
        let out = ran(&given);
        assert_eq!(out.status.code(), Some(64), "{given:?}: {out:?}");
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(
            said.contains("--opened and --systemctl go only with --open-unit"),
            "{given:?}: {said}"
        );
    }
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
