//! changed-checks run as a person runs it, with stand-ins for git and nix
//! first on its path: git names a repository and a merge base, and nix
//! names each side's checks and their derivations from files the test
//! writes, and records what it's asked to build.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// git as changed-checks asks it: where the repository is, the merge base
/// with origin's main and nothing else, and no untracked files.
const GIT: &str = r#"#!/bin/sh
case "$1" in
  rev-parse) echo "$STAND_IN/repo" ;;
  -C)
    case "$3" in
      merge-base) [ "$5" = refs/remotes/origin/main ] || exit 1; echo 0123456789abcdef0123 ;;
      ls-files) ;;
    esac ;;
esac
"#;

/// nix as changed-checks asks it: a side's checks by name, then their
/// derivations, the base's being any flake at a revision; and a build,
/// which it records.
const NIX: &str = r#"#!/bin/sh
case "$1" in
  build) shift; echo "$@" >> "$STAND_IN/built"; exit 0 ;;
esac
side=working
case "$3" in *'?rev='*) side=base ;; esac
case "$5" in
  builtins.attrNames) cat "$STAND_IN/$side-names.json" ;;
  *) cat "$STAND_IN/$side.json" ;;
esac
"#;

/// A folder holding the stand-ins and what they read, emptied first.
struct StandIn {
    folder: PathBuf,
}

impl StandIn {
    fn new(test: &str) -> Self {
        let folder =
            std::env::temp_dir().join(format!("changed-checks-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("bin")).expect("a folder for the stand-ins");
        for (name, script) in [("git", GIT), ("nix", NIX)] {
            let path = folder.join("bin").join(name);
            fs::write(&path, script).expect("a stand-in");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("it runs");
        }
        Self { folder }
    }

    /// Have nix give these derivations for one side, "base" or "working".
    fn checks(&self, side: &str, derivations: &str) {
        let parsed: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(derivations).expect("derivations as JSON");
        let names: Vec<&String> = parsed.keys().collect();
        fs::write(self.path(&format!("{side}.json")), derivations).expect("written");
        fs::write(
            self.path(&format!("{side}-names.json")),
            serde_json::to_string(&names).expect("names"),
        )
        .expect("written");
    }

    fn path(&self, name: &str) -> PathBuf {
        self.folder.join(name)
    }

    fn run(&self, arguments: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            self.folder.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(env!("CARGO_BIN_EXE_changed-checks"))
            .args(arguments)
            .env("PATH", path)
            .env("STAND_IN", &self.folder)
            .env("XDG_CACHE_HOME", self.folder.join("cache"))
            .output()
            .expect("changed-checks runs")
    }

    fn built(&self) -> String {
        fs::read_to_string(self.path("built")).unwrap_or_default()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("text")
}

const BASE: &str = r#"{"a": "a.drv", "b": "b.drv", "gone": "g.drv", "vm-x": "v.drv"}"#;
const WORKING: &str = r#"{"a": "a2.drv", "b": "b.drv", "new": "n.drv", "vm-x": "v2.drv"}"#;

#[test]
fn it_names_each_check_the_change_moved_and_builds_nothing_unasked() {
    let stand_in = StandIn::new("names");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n"
    );
    assert!(text(&ran.stderr).contains("checks at 0123456789ab with the working tree"));
    assert_eq!(stand_in.built(), "");
}

#[test]
fn asked_to_build_it_builds_what_was_added_and_changed_but_the_vm_tests() {
    let stand_in = StandIn::new("builds");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.new {flake}.a\n")
    );
}

#[test]
fn a_check_that_stops_evaluating_fails_the_run_and_nothing_is_built() {
    let stand_in = StandIn::new("broken");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        r#"{"a": null, "b": "b.drv", "gone": "g.drv", "vm-x": "v.drv"}"#,
    );

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build"]);

    assert_eq!(ran.status.code(), Some(1));
    assert_eq!(text(&ran.stdout), "broken   a\n");
    assert_eq!(stand_in.built(), "");
}

#[test]
fn the_base_is_evaluated_once_and_kept_by_its_commit() {
    let stand_in = StandIn::new("kept");
    stand_in.checks("base", BASE);
    stand_in.checks("working", BASE);
    assert!(stand_in.run(&["--system", "x86_64-linux"]).status.success());
    // Were the base evaluated again, its checks would now all be removed.
    stand_in.checks("base", "{}");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let kept = Path::new("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(stand_in.folder.join(kept).exists());
}

#[test]
fn a_base_sharing_no_history_is_refused_as_asked_wrongly() {
    let stand_in = StandIn::new("unrelated");

    let ran = stand_in.run(&["--base", "elsewhere", "--system", "x86_64-linux"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(text(&ran.stderr).contains("elsewhere shares no history with HEAD"));
}

#[test]
fn an_option_it_doesnt_take_is_refused_saying_how_it_is_used() {
    let stand_in = StandIn::new("misused");

    let ran = stand_in.run(&["--bild"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(text(&ran.stderr).starts_with("changed-checks: --bild isn't something it takes\n"));
}
