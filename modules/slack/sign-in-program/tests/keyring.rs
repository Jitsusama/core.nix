//! The keyring program every Slack reader is given (keyring.sh, which
//! ../keyring.nix wraps), run with stand-ins for secret-tool and busctl:
//! it tells a sign-in held locked from none held at all.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use sign_in_program::LOCKED;

/// A stand-in program in `at`, named `name`, that runs `script`.
fn stand_in(at: &Path, name: &str, script: &str) -> PathBuf {
    let path = at.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).expect("the stand-in written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
    path
}

/// What keyring.sh ends with, and prints, when secret-tool runs
/// `secret_tool` and busctl answers `searched` (busctl's own form).
fn ended(case: &str, secret_tool: &str, searched: &str) -> (Option<i32>, String) {
    let at = std::env::temp_dir().join(format!("sign-in-keyring-{case}-{}", std::process::id()));
    std::fs::create_dir_all(&at).expect("the case's folder is made");
    let secret_tool = stand_in(&at, "secret-tool", secret_tool);
    let busctl = stand_in(&at, "busctl", &format!("printf '%s\\n' '{searched}'"));
    let run = Command::new("/bin/sh")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/keyring.sh"))
        .env("SECRET_TOOL", secret_tool)
        .env("BUSCTL", busctl)
        .output()
        .expect("keyring.sh runs");
    let _ = std::fs::remove_dir_all(&at);
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
    )
}

/// busctl's answer to `SearchItems` when the item is held locked, as a
/// throwaway gnome-keyring answered on 17 Nov.
const HELD_LOCKED: &str = r#"aoao 0 1 "/org/freedesktop/secrets/collection/login/1""#;

#[test]
fn a_sign_in_the_keyring_opens_is_printed() {
    let (code, printed) = ended("open", "echo stand-in-token", "aoao 0 0");
    assert_eq!(code, Some(0));
    assert_eq!(printed, "stand-in-token\n", "secret-tool's output, alone");
}

#[test]
fn a_sign_in_held_locked_ends_as_locked() {
    let (code, printed) = ended("locked", "exit 1", HELD_LOCKED);
    assert_eq!(code, Some(LOCKED), "held, but locked");
    assert_eq!(printed, "", "nothing printed to be taken for a sign-in");
}

#[test]
fn a_keyring_holding_none_isnt_locked_whatever_else_it_holds() {
    for (case, searched) in [
        ("lacking", "aoao 0 0"),
        // Held open but not printed: secret-tool failed some other way.
        (
            "held-open",
            r#"aoao 1 "/org/freedesktop/secrets/collection/login/1" 0"#,
        ),
        ("no-keyring", ""),
        ("garbled", "aoao x"),
        ("short", "aoao 2 \"/a\""),
    ] {
        let (code, _) = ended(case, "exit 1", searched);
        assert_eq!(code, Some(1), "{case}: {searched:?} is none held locked");
    }
}

#[test]
fn the_locked_count_is_read_past_every_unlocked_path() {
    let searched = r#"aoao 2 "/a/1" "/a/2" 1 "/b/1""#;
    let (code, _) = ended("past", "exit 1", searched);
    assert_eq!(code, Some(LOCKED), "the count after the unlocked paths");
}
