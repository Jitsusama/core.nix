//! changed-checks run as a person runs it, with stand-ins for git and nix
//! first on its path: git names a repository and a merge base, and nix
//! names each side's checks and their derivations from files the test
//! writes, and records what it's asked to build.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Held while a stand-in is written and while a program is started. The
/// tests run on threads of one process, and a program started while another
/// thread's stand-in is open for writing holds it open too until it's under
/// way, so running that stand-in fails as a busy file.
static STARTING: Mutex<()> = Mutex::new(());

/// Hold `STARTING`, even when a test that held it failed.
fn starting() -> MutexGuard<'static, ()> {
    STARTING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// git as changed-checks asks it: where the repository is, the merge base
/// with origin's main and nothing else, and no untracked files. The working
/// tree has a change in it, until a test writes `clean`, and then it's its
/// commit; origin has that commit once a test writes `pushed`.
const GIT: &str = r#"#!/bin/sh
case "$1" in
  rev-parse)
    if [ "$2" = HEAD ]; then echo fedcba9876543210fedc; else echo "$STAND_IN/repo"; fi ;;
  status) [ -e "$STAND_IN/clean" ] || echo " M flake.nix" ;;
  branch) if [ -e "$STAND_IN/pushed" ]; then echo "  origin/topic"; fi ;;
  remote) echo https://github.com/someone/thing.git ;;
  -C)
    case "$3" in
      merge-base) [ "$5" = refs/remotes/origin/main ] || exit 1; echo 0123456789abcdef0123 ;;
      ls-files) ;;
    esac ;;
esac
"#;

/// nix as changed-checks asks it: a side's checks by name, then their
/// derivations, the base's being any flake at a revision; and a build,
/// which it records. It refuses an evaluation through its evaluation
/// cache, which two at once would find busy. Once a test writes `meet`, an
/// evaluation of
/// derivations waits for another to be under way beside it before
/// answering, and fails if none comes within ten seconds.
const NIX: &str = r#"#!/bin/sh
case "$1" in
  build) shift; echo "$@" >> "$STAND_IN/built"; exit 0 ;;
esac
[ "$6" = --no-eval-cache ] || { echo "evaluated through the cache" >&2; exit 1; }
side=working
case "$3" in *'?rev='*) side=base ;; esac
under_way() { find "$STAND_IN" -maxdepth 1 -name 'evaluating.*' | wc -l; }
case "$5" in
  builtins.attrNames) cat "$STAND_IN/$side-names.json" ;;
  *)
    if [ -e "$STAND_IN/meet" ]; then
      mkdir "$STAND_IN/evaluating.$$"
      tries=0
      until [ -e "$STAND_IN/met" ]; do
        if [ "$(under_way)" -ge 2 ]; then touch "$STAND_IN/met"; break; fi
        tries=$((tries + 1))
        if [ "$tries" -ge 200 ]; then echo "evaluated alone" >&2; exit 1; fi
        sleep 0.05
      done
      rmdir "$STAND_IN/evaluating.$$"
    fi
    cat "$STAND_IN/$side.json" ;;
esac
"#;

/// ssh as changed-checks asks it: it records the host and the command it
/// was given, and what the known hosts file it was given holds, and logs
/// what it sent as ssh's verbose log does.
const SSH: &str = r#"#!/bin/sh
while [ $# -gt 0 ]; do
  case "$1" in
    -E) log=$2; shift 2 ;;
    -o)
      case "$2" in UserKnownHostsFile=*) cp "${2#*=}" "$STAND_IN/known" ;; esac
      shift 2 ;;
    -i) echo "$2" > "$STAND_IN/key"; shift 2 ;;
    *) break ;;
  esac
done
printf '%s\n' "$@" > "$STAND_IN/sent"
echo "Transferred: sent 4321, received 98765 bytes, in 3.2 seconds" >> "$log"
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
        let _starting = starting();
        for (name, script) in [("git", GIT), ("nix", NIX), ("ssh", SSH)] {
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
        let started = {
            let _starting = starting();
            Command::new(env!("CARGO_BIN_EXE_changed-checks"))
                .args(arguments)
                .env("PATH", path)
                .env("STAND_IN", &self.folder)
                .env("XDG_CACHE_HOME", self.folder.join("cache"))
                .env("XDG_STATE_HOME", self.folder.join("state"))
                .env("NIX_OFFLOAD_DIR", self.folder.join("offload"))
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("changed-checks starts")
        };
        started.wait_with_output().expect("changed-checks runs")
    }

    fn built(&self) -> String {
        fs::read_to_string(self.path("built")).unwrap_or_default()
    }

    /// The runs logged so far, each as its line of JSON.
    fn runs(&self) -> Vec<serde_json::Value> {
        fs::read_to_string(self.path("state/changed-checks/runs.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).expect("a run as JSON"))
            .collect()
    }
}

/// The sides a logged run evaluated, in order.
fn evaluated(run: &serde_json::Value) -> Vec<&str> {
    run["evaluated"]
        .as_array()
        .expect("its evaluations")
        .iter()
        .map(|evaluation| evaluation["of"].as_str().expect("a side"))
        .collect()
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
    let runs = stand_in.runs();
    assert_eq!(runs.len(), 1, "one run logged: {runs:?}");
    assert_eq!(runs[0]["built"]["checks"], 2, "{}", runs[0]);
    assert_eq!(runs[0]["built"]["passed"], true, "{}", runs[0]);
}

#[test]
fn a_pushed_commit_is_built_on_the_lent_builder_and_anything_else_here() {
    let stand_in = StandIn::new("elsewhere");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    let offload = stand_in.path("offload");
    fs::create_dir_all(&offload).expect("its folder");
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    let standing = offload.join("machines-with-standing");
    let here = format!(
        "--no-link --keep-going -L --builders @{} {flake}.new {flake}.a\n",
        standing.display()
    );
    let asked = || stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);
    fs::write(&standing, "").expect("written");

    fs::write(offload.join("store"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "no builder is lent");

    // c3NoLWVkMjU1MTkgQUFBQQ== is "ssh-ed25519 AAAA".
    let lent = "ssh-ng://lent?compress=true&ssh-key=/the/key&\
                base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n";
    fs::write(offload.join("store"), lent).expect("written");
    for (step, why) in [
        ("clean", "the working tree differs from its commit"),
        (
            "pushed",
            "origin doesn't have fedcba987654; push it to build there",
        ),
    ] {
        fs::remove_file(stand_in.path("built")).expect("cleared");
        let ran = asked();
        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(stand_in.built(), here, "{why}");
        assert!(text(&ran.stderr).contains(&format!("building here, since {why}")));
        fs::write(stand_in.path(step), "").expect("written");
    }

    fs::remove_file(stand_in.path("built")).expect("cleared");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), "", "nothing is built here");
    let fetched = "github:someone/thing/fedcba9876543210fedc#checks.x86_64-linux";
    assert_eq!(
        fs::read_to_string(stand_in.path("sent")).expect("it was sent"),
        format!("lent\nnix build --no-link --keep-going -L '{fetched}.new' '{fetched}.a'\n")
    );
    assert_eq!(
        fs::read_to_string(stand_in.path("known")).expect("its host key"),
        "lent ssh-ed25519 AAAA\n"
    );
    assert_eq!(
        fs::read_to_string(stand_in.path("key")).expect("its key"),
        "/the/key\n"
    );
    let runs = stand_in.runs();
    let built = &runs.last().expect("the run")["built"];
    assert_eq!(built["lent"], true, "{built}");
    assert_eq!(built["sent"], 4321, "{built}");
    assert_eq!(built["checks"], 2, "{built}");
}

#[test]
fn it_builds_each_check_from_the_derivation_it_evaluated_rather_than_evaluating_again() {
    // An evaluator holds its heap until its last build ends, so building
    // from the flake again held gigabytes through every VM test.
    let stand_in = StandIn::new("derivations");
    let new = stand_in.path("n.drv");
    fs::write(&new, "").expect("a derivation");
    let missing = stand_in.path("a2.drv");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        &format!(
            r#"{{"a": "{}", "b": "b.drv", "new": "{}", "vm-x": "v2.drv"}}"#,
            missing.display(),
            new.display()
        ),
    );

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {}^* {flake}.a\n", new.display()),
        "a derivation collected since its evaluation is built from the flake"
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
    let runs = stand_in.runs();
    assert_eq!(runs.len(), 2, "both runs logged: {runs:?}");
    assert_eq!(evaluated(&runs[0]), ["base", "tree"], "{}", runs[0]);
    assert_eq!(evaluated(&runs[1]), ["tree"], "{}", runs[1]);
    assert_eq!(runs[1]["base"], "0123456789abcdef0123", "{}", runs[1]);
    assert_eq!(runs[1]["named"], 0, "{}", runs[1]);
    let chunks = &runs[0]["evaluated"][1]["chunks"];
    assert_eq!(chunks[0]["checks"], 4, "the tree's one chunk: {}", runs[0]);
}

#[test]
fn a_clean_tree_is_kept_by_its_commit() {
    let stand_in = StandIn::new("clean");
    stand_in.checks("base", BASE);
    stand_in.checks("working", BASE);
    fs::write(stand_in.path("clean"), "").expect("written");
    assert!(stand_in.run(&["--system", "x86_64-linux"]).status.success());
    // Were the tree evaluated again, its checks would now all be removed.
    stand_in.checks("working", "{}");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let kept = Path::new("cache/changed-checks/fedcba9876543210fedc-x86_64-linux.json");
    assert!(stand_in.folder.join(kept).exists());
    let runs = stand_in.runs();
    assert_eq!(evaluated(&runs[0]), ["base", "tree"], "{}", runs[0]);
    assert!(evaluated(&runs[1]).is_empty(), "{}", runs[1]);
}

#[test]
fn two_chunks_are_evaluated_at_once() {
    let stand_in = StandIn::new("at-once");
    // Ninety ordinary checks are three chunks: forty, forty and ten.
    let many: serde_json::Map<String, serde_json::Value> = (0..90)
        .map(|at| (format!("check-{at}"), format!("{at}.drv").into()))
        .collect();
    let many = serde_json::to_string(&many).expect("derivations");
    stand_in.checks("base", &many);
    stand_in.checks("working", &many);
    fs::write(stand_in.path("meet"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    let runs = stand_in.runs();
    let chunks: Vec<&serde_json::Value> = runs[0]["evaluated"][0]["chunks"]
        .as_array()
        .expect("the base's chunks")
        .iter()
        .map(|chunk| &chunk["checks"])
        .collect();
    assert_eq!(chunks, [40, 40, 10], "logged in their order: {}", runs[0]);
}

#[test]
fn asked_to_keep_the_base_it_keeps_it_and_evaluates_nothing_else() {
    let stand_in = StandIn::new("keeps");
    // The working tree has no checks to give, so evaluating it would fail.
    stand_in.checks("base", BASE);

    let ran = stand_in.run(&["--system", "x86_64-linux", "--keep-base"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    assert!(text(&ran.stderr).contains("kept x86_64-linux's checks at 0123456789ab"));
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
