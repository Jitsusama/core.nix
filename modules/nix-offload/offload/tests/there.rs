//! `nix-offload there` run as a caller runs it, with stand-ins for ssh and
//! nix on its PATH that record what they were given.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

/// ssh as the builder answers: it writes its verbose log where `-E` says,
/// records its last two words (the host and the command) and exits as the
/// `status` file beside it says.
const SSH: &str = r#"#!/bin/sh
here=$(dirname "$0")
while [ "$#" -gt 2 ]; do
  if [ "$1" = -E ]; then log=$2; fi
  shift
done
printf '%s\n%s\n' "$1" "$2" > "$here/ssh-was-given"
echo "Transferred: sent 3812, received 9 bytes, in 1.0 seconds" > "$log"
exit "$(cat "$here/status")"
"#;

/// nix as it builds here: it records its arguments and exits 0.
const NIX: &str = r#"#!/bin/sh
echo "$@" > "$(dirname "$0")/nix-was-given"
"#;

const STORE: &str = "ssh-ng://nix-offload?ssh-key=/key&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n";

fn stand_ins(folder: &Path, status: u8) {
    for (name, script) in [("ssh", SSH), ("nix", NIX)] {
        let path = folder.join(name);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(folder.join("status"), status.to_string()).unwrap();
}

fn there(folder: &Path, offload: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args([
            "there",
            "-L",
            "github:o/r/abc",
            "checks.x.a",
            "--keep-going",
        ])
        .env(
            "PATH",
            format!("{}:{}", folder.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", offload)
        .env("XDG_RUNTIME_DIR", folder)
        .env("XDG_STATE_HOME", folder)
        .output()
        .unwrap()
}

fn journalled(folder: &Path) -> serde_json::Value {
    let journal = fs::read_to_string(folder.join("nix-offload/there.jsonl")).unwrap();
    serde_json::from_str(journal.lines().last().unwrap()).unwrap()
}

#[test]
fn a_lent_builder_is_sent_the_build_and_its_status_comes_back() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 100);
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(100), "{ran:?}");
    assert_eq!(
        fs::read_to_string(folder.path().join("ssh-was-given")).unwrap(),
        "nix-offload\nnix build --no-link -L --keep-going 'github:o/r/abc#checks.x.a'\n"
    );
    assert!(!folder.path().join("nix-was-given").exists());
    let line = journalled(folder.path());
    assert_eq!(line["on"], "nix-offload");
    assert_eq!(line["sent"], 3812);
    assert_eq!(line["status"], 100);
}

#[test]
fn with_no_builder_or_none_reached_it_builds_here_and_says_so() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    stand_ins(folder.path(), 255);
    let here = "build --no-link -L --keep-going github:o/r/abc#checks.x.a\n";
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(String::from_utf8_lossy(&ran.stderr).contains("no builder is lent"));
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        here
    );
    assert!(journalled(folder.path())["on"].is_null());

    fs::remove_file(folder.path().join("nix-was-given")).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(String::from_utf8_lossy(&ran.stderr).contains("couldn't reach nix-offload"));
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        here
    );
}
