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
/// with origin's main, with `older`, a commit origin's main holds, and with
/// `unpushed`, one it doesn't, and no untracked files. Main's quarantine
/// list is `quarantine.json`, once a test writes it. The working
/// tree has a change in it, until a test writes `clean`, and then it's its
/// commit; origin has that commit once a test writes `pushed`. A
/// commit's tree is `tree`'s, or `basetree`, the change from the base
/// is `diff`, and the base holds the paths `based` lists.
const GIT: &str = r#"#!/bin/sh
case "$1" in
  rev-parse)
    if [ "$2" = HEAD ]; then echo fedcba9876543210fedc; else echo "$STAND_IN/repo"; fi ;;
  status) [ -e "$STAND_IN/clean" ] || echo " M flake.nix" ;;
  branch) if [ -e "$STAND_IN/pushed" ]; then echo "  origin/topic"; fi ;;
  remote) echo https://github.com/someone/thing.git ;;
  -C)
    case "$3" in
      merge-base)
        case "$4 $5" in
          "HEAD refs/remotes/origin/main") echo 0123456789abcdef0123 ;;
          "HEAD older") echo 1a96207100000000000a ;;
          "HEAD unpushed") echo abcdef0123456789abcd ;;
          "--is-ancestor 0123456789abcdef0123"|"--is-ancestor 1a96207100000000000a") ;;
          *) exit 1 ;;
        esac ;;
      ls-files) ;;
      rev-parse) cat "$STAND_IN/tree" 2>/dev/null || echo basetree ;;
      diff) cat "$STAND_IN/diff" 2>/dev/null ;;
      cat-file) grep -qxF "${5#*:}" "$STAND_IN/based" 2>/dev/null ;;
      show) cat "$STAND_IN/quarantine.json" 2>/dev/null || exit 128 ;;
    esac ;;
esac
"#;

/// nix as changed-checks asks it: a side's checks by name, then their
/// derivations, the base's being any flake at a revision; and a build,
/// which it records, and which fails when it names a check `failing` lists. It refuses an evaluation through its evaluation
/// cache, which two at once would find busy. Once a test writes `meet`, an
/// evaluation of
/// derivations waits for another to be under way beside it before
/// answering, and fails if none comes within ten seconds. Once a test
/// writes `vm-beside-build`, the working tree's VM tests answer only once a
/// build has begun, failing if none begins within ten seconds. Asked how many
/// checks each system has, it answers `systems.json`, or x86_64-linux's
/// alone. Every evaluation and build must be pure and take no flake's
/// config, since a fenced session's flake is evaluated on a lent builder.
/// The flake's source is `source`, and evaluated from it as a plain path
/// it gives the base's checks.
const NIX: &str = r#"#!/bin/sh
case " $* " in
  *" --impure "*) echo "evaluated impurely" >&2; exit 1 ;;
esac
case " $* " in
  *" --option pure-eval true --option accept-flake-config false"*) ;;
  *) echo "not kept pure, or a flake's config taken: $*" >&2; exit 1 ;;
esac
case "$1" in
  build)
    shift
    echo "$@" | sed 's/ --option pure-eval true --option accept-flake-config false//' \
      >> "$STAND_IN/built"
    for word in "$@"; do
      while read -r check; do
        case "$word" in *".$check") exit 1 ;; esac
      done < "$STAND_IN/failing"
    done 2>/dev/null
    exit 0 ;;
esac
if [ "$2" = --raw ]; then echo "$STAND_IN/source"; exit 0; fi
case "$5" in
  builtins.mapAttrs*)
    cat "$STAND_IN/systems.json" 2>/dev/null || echo '{"x86_64-linux": 4}'
    exit 0 ;;
esac
[ "$5" = builtins.currentSystem ] && { echo "asked Nix for this machine's system" >&2; exit 1; }
[ "$6" = --no-eval-cache ] || { echo "evaluated through the cache" >&2; exit 1; }
side=working
case "$3" in *'?rev='*|path:*) side=base ;; esac
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
    if [ -e "$STAND_IN/vm-beside-build" ] && [ "$side" = working ]; then
      case "$5" in
        *'"vm-'*)
          tries=0
          until [ -e "$STAND_IN/built" ]; do
            tries=$((tries + 1))
            if [ "$tries" -ge 200 ]; then
              echo "evaluated the VM tests before the build began" >&2; exit 1
            fi
            sleep 0.05
          done ;;
      esac
    fi
    cat "$STAND_IN/$side.json" ;;
esac
"#;

/// nix-offload as changed-checks asks it to build there: it records what
/// it was given, and journals the build as sent from the lent builder, as
/// `nix-offload there` does. Once a test writes `takes-changed`, it says
/// it takes `--changed`, `takes-never-ask` adds `--never-ask`, and
/// `takes-first` adds `--first`. A run
/// sent so is recorded with the folder it started in and exits with
/// `changed-exit`'s number, or 0.
const NIX_OFFLOAD: &str = r#"#!/bin/sh
[ -f "$STAND_IN/refuses" ] && exit 64
[ -f "$STAND_IN/unsent" ] && exit 75
if [ "$2" = --takes ]; then
  [ -f "$STAND_IN/takes-changed" ] && printf 'tree\nchanged\n'
  [ -f "$STAND_IN/takes-never-ask" ] && printf 'never-ask\n'
  [ -f "$STAND_IN/takes-first" ] && printf 'first\n'
  exit 0
fi
if [ "$2" = --changed ] || [ "$3" = --changed ] || [ "$4" = --changed ]; then
  { pwd; printf '%s\n' "$@"; } > "$STAND_IN/changed"
  exit "$(cat "$STAND_IN/changed-exit" 2>/dev/null || echo 0)"
fi
printf '%s\n' "$@" > "$STAND_IN/sent"
mkdir -p "$XDG_STATE_HOME/nix-offload"
echo "{\"flake\":\"$4\",\"on\":\"lent\",\"sent\":4321}" \
  >> "$XDG_STATE_HOME/nix-offload/there.jsonl"
"#;

/// strace as changed-checks asks it: the trace it writes is `trace`, each
/// SOURCE in it the flake's source, and it runs what it traces.
const STRACE: &str = r#"#!/bin/sh
while [ "$1" != -o ]; do shift; done
sed "s|SOURCE|$STAND_IN/source|g" "$STAND_IN/trace" > "$2" 2>/dev/null || : > "$2"
shift 2
exec "$@"
"#;

/// A folder holding the stand-ins and what they read, emptied first and
/// gone once the test ends, even when it fails: the tests' `$TMPDIR` is a
/// quest's scratch, where a folder left behind stays until the quest ends.
/// One made `here` runs changed-checks as asked to run on this machine,
/// since a run nowhere else took is refused otherwise; one made
/// `elsewhere` offers each run to a nix-offload that takes it first, as a
/// person's run does.
struct StandIn {
    folder: PathBuf,
    here: bool,
}

impl Drop for StandIn {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.folder);
    }
}

impl StandIn {
    fn new(test: &str) -> Self {
        Self::made(test, true)
    }

    fn elsewhere(test: &str) -> Self {
        let made = Self::made(test, false);
        fs::write(made.path("takes-changed"), "").expect("written");
        made
    }

    fn made(test: &str, here: bool) -> Self {
        let folder =
            std::env::temp_dir().join(format!("changed-checks-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("bin")).expect("a folder for the stand-ins");
        let _starting = starting();
        let stand_ins = [
            ("git", GIT),
            ("nix", NIX),
            ("nix-offload", NIX_OFFLOAD),
            ("strace", STRACE),
        ];
        for (name, script) in stand_ins {
            let path = folder.join("bin").join(name);
            fs::write(&path, script).expect("a stand-in");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("it runs");
        }
        Self { folder, here }
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
        self.run_told(arguments, &[])
    }

    /// Run it with these variables set besides.
    fn run_told(&self, arguments: &[&str], told: &[(&str, &str)]) -> Output {
        self.started(arguments, told)
            .wait_with_output()
            .expect("changed-checks runs")
    }

    /// Start it with these variables set besides, its output piped.
    fn started(&self, arguments: &[&str], told: &[(&str, &str)]) -> std::process::Child {
        let path = format!(
            "{}:{}",
            self.folder.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let asked_here = self.here && !arguments.contains(&"--here");
        let _starting = starting();
        Command::new(env!("CARGO_BIN_EXE_changed-checks"))
            .args(arguments)
            .args(asked_here.then_some("--here"))
            .env("PATH", path)
            .env("STAND_IN", &self.folder)
            .env("XDG_CACHE_HOME", self.folder.join("cache"))
            .env("XDG_STATE_HOME", self.folder.join("state"))
            .env("NIX_OFFLOAD_DIR", self.folder.join("offload"))
            // No stall, whatever the machine running the tests holds.
            .env("CHANGED_CHECKS_PRESSURE", self.folder.join("pressure"))
            .env("CHANGED_CHECKS_STRACE", self.folder.join("bin/strace"))
            .envs(told.iter().copied())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("changed-checks starts")
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
fn asked_only_to_name_them_it_names_each_check_the_change_moved_and_builds_nothing() {
    let stand_in = StandIn::new("names");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);

    let ran = stand_in.run(&["--system", "x86_64-linux", "--name-only"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n"
    );
    assert!(text(&ran.stderr).contains("checks at 0123456789ab with the working tree"));
    assert_eq!(stand_in.built(), "");
}

#[test]
fn by_default_or_asked_it_builds_what_was_added_and_changed_but_the_vm_tests() {
    // A named check proves nothing until it's built: #1373 merged naming
    // checks it never built.
    for (label, arguments) in [
        ("builds-unasked", &["--system", "x86_64-linux"][..]),
        (
            "builds",
            &["--system", "x86_64-linux", "--build", "--no-vm"][..],
        ),
    ] {
        let stand_in = StandIn::new(label);
        stand_in.checks("base", BASE);
        stand_in.checks("working", WORKING);

        let ran = stand_in.run(arguments);

        assert!(ran.status.success(), "{arguments:?}: {}", text(&ran.stderr));
        let repo = stand_in.path("repo");
        let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
        assert_eq!(
            stand_in.built(),
            format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
            "{arguments:?}"
        );
        let runs = stand_in.runs();
        assert_eq!(runs.len(), 1, "one run logged: {runs:?}");
        assert_eq!(runs[0]["built"]["checks"], 2, "{}", runs[0]);
        assert_eq!(runs[0]["built"]["passed"], true, "{}", runs[0]);
    }
}

// A run that builds VM tests whole says how many before it sends them,
// and that the gate before a merge doesn't ask for them; one that leaves
// them out says nothing of them.
#[test]
fn building_vm_tests_whole_says_how_many_and_what_the_gate_needs() {
    for (label, arguments, said) in [
        (
            "with-vm",
            &["--system", "x86_64-linux", "--build"][..],
            true,
        ),
        ("without-vm", &["--system", "x86_64-linux"][..], false),
    ] {
        let stand_in = StandIn::new(label);
        stand_in.checks("base", BASE);
        stand_in.checks("working", WORKING);

        let ran = stand_in.run(arguments);

        assert!(ran.status.success(), "{label}: {}", text(&ran.stderr));
        assert_eq!(
            text(&ran.stderr).contains("building 1 VM test whole, as --build asks"),
            said,
            "{label}: {}",
            text(&ran.stderr)
        );
        assert_eq!(
            text(&ran.stderr).contains("needs only those `merge-net needs` names"),
            said,
            "{label}"
        );
    }
}

#[test]
fn a_check_main_quarantines_is_built_alone_and_its_failure_blocks_nothing() {
    let stand_in = StandIn::new("quarantined");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(
        stand_in.path("quarantine.json"),
        r#"[{"test": "new", "reason": "failed then passed at abc", "owner_pr": 7}]"#,
    )
    .expect("written");
    fs::write(stand_in.path("failing"), "new\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains(
            "new failed, but main quarantines it (failed then passed at abc; #7 owns it), \
             so it blocks nothing"
        ),
        "{}",
        text(&ran.stderr)
    );
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.a\n--no-link --keep-going -L {flake}.new\n")
    );
    assert_eq!(stand_in.runs()[0]["built"]["passed"], true);

    // A check main doesn't quarantine still fails the run.
    fs::write(stand_in.path("failing"), "a\n").expect("written");
    let _ = fs::remove_file(stand_in.path("built"));
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(!ran.status.success(), "{}", text(&ran.stderr));
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

    let lent = "ssh-ng://lent?compress=true&ssh-key=/the/key\n";
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
    assert_eq!(
        fs::read_to_string(stand_in.path("sent")).expect("it was sent"),
        "there\n-L\n--keep-going\ngithub:someone/thing/fedcba9876543210fedc\n\
         checks.x86_64-linux.new\nchecks.x86_64-linux.a\n"
    );
    let runs = stand_in.runs();
    let built = &runs.last().expect("the run")["built"];
    assert_eq!(built["lent"], true, "{built}");
    assert_eq!(built["sent"], 4321, "{built}");
    assert_eq!(built["checks"], 2, "{built}");

    // A builder gone before it took the build builds nothing there, and
    // it's built here.
    fs::write(stand_in.path("unsent"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "taken by no builder");
    assert!(text(&ran.stderr).contains("since no builder took the build there"));
    fs::remove_file(stand_in.path("unsent")).expect("cleared");
    fs::remove_file(stand_in.path("built")).expect("cleared");

    // A nix-offload older than `there` refuses it, and it's built here.
    fs::write(stand_in.path("refuses"), "").expect("written");
    let ran = asked();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), here, "refused there");
    assert!(text(&ran.stderr).contains("since nix-offload couldn't take the build there"));
}

#[test]
fn the_whole_run_goes_elsewhere_where_nix_offload_takes_it_naming_this_system() {
    let stand_in = StandIn::elsewhere("sent-whole");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");

    let ran = stand_in.run(&["--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        fs::read_to_string(stand_in.path("changed")).expect("it was sent"),
        format!(
            "{}\nthere\n--changed\n--build\n--no-vm\n--system\nx86_64-linux\n",
            fs::canonicalize(stand_in.path("repo"))
                .expect("the repository")
                .display()
        )
    );
    assert_eq!(stand_in.built(), "", "nothing is built here");
    assert!(stand_in.runs().is_empty(), "nothing is evaluated here");

    // The far side's failure is the run's.
    fs::write(stand_in.path("changed-exit"), "1").expect("written");
    assert_eq!(stand_in.run(&["--build"]).status.code(), Some(1));
}

// Runs no builder took evaluated and built here while the machine looked
// idle, and on 8 November one built openvino here for three hours beside
// Joel's work. Only one asked to runs here.
#[test]
fn it_runs_here_only_when_asked_and_otherwise_says_why_it_went_nowhere() {
    let stand_in = StandIn::elsewhere("kept-here");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    let named = "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n";
    let refused = |ran: &Output, why: &str| {
        let said = text(&ran.stderr);
        assert_eq!(ran.status.code(), Some(75), "{said}");
        assert!(said.contains(why), "{said}");
        assert!(
            said.contains("so nothing was run here; --here runs it on this machine"),
            "{said}"
        );
        assert_eq!(text(&ran.stdout), "", "nothing was evaluated here");
    };

    // Edits not yet committed aren't sent: the far side checks a commit, so
    // it would pass the last one rather than what's in the working tree.
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));
    refused(&ran, "uncommitted");
    let ran = stand_in.run(&["--system", "x86_64-linux", "--here"]);
    assert_eq!(text(&ran.stdout), named, "{}", text(&ran.stderr));
    fs::write(stand_in.path("clean"), "").expect("written");

    for arguments in [
        &["--system", "x86_64-linux", "--here"][..],
        &[
            "--system",
            "x86_64-linux",
            "--base",
            "refs/remotes/origin/main",
        ][..],
    ] {
        let _ = fs::remove_file(stand_in.path("changed"));
        let ran = stand_in.run(arguments);
        let sent = stand_in.path("changed").exists();
        let here = arguments.contains(&"--here");
        assert_eq!(sent, !here, "{arguments:?}: {}", text(&ran.stderr));
        if here {
            assert_eq!(text(&ran.stdout), named, "{arguments:?}");
        }
    }

    // A base only this checkout names shares no history the stand-in git
    // knows, so it's refused as asked wrongly, sent nowhere.
    let _ = fs::remove_file(stand_in.path("changed"));
    let ran = stand_in.run(&["--system", "x86_64-linux", "--base", "topic"]);
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));

    // Offered and taken nowhere, it isn't run here instead, idle or not.
    fs::write(stand_in.path("changed-exit"), "75").expect("written");
    let ran = stand_in.run(&["--system", "x86_64-linux"]);
    assert!(stand_in.path("changed").exists(), "it was offered first");
    refused(&ran, "no builder took the run");
}

#[test]
fn a_base_origins_main_holds_goes_with_the_run_as_its_commit_and_any_other_stays() {
    // A run comparing with an older commit of main, as one checking what
    // changed since main last passed does, stayed here and evaluated twice,
    // about 4 GB each, at a load of 118 on 30 October.
    let stand_in = StandIn::elsewhere("far-base");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");

    let ran = stand_in.run(&["--base", "older", "--system", "x86_64-linux"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--changed\n--base\n1a96207100000000000a\n--system\nx86_64-linux\n"),
        "{sent}"
    );
    assert!(stand_in.runs().is_empty(), "nothing is evaluated here");

    fs::remove_file(stand_in.path("changed")).expect("cleared");
    let ran = stand_in.run(&["--base", "unpushed", "--system", "x86_64-linux"]);

    assert_eq!(ran.status.code(), Some(75), "{}", text(&ran.stderr));
    assert!(!stand_in.path("changed").exists(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("origin's main doesn't hold abcdef012345"),
        "{}",
        text(&ran.stderr)
    );
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
fn a_check_told_it_passed_at_this_commit_isnt_built_again() {
    // A run sent to a builder whose store was new built again what an
    // earlier builder had passed: 631 s, then 1608 s, on 3 November.
    let stand_in = StandIn::new("passed");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("clean"), "").expect("written");
    let arguments = ["--system", "x86_64-linux", "--here"];
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());

    let ran = stand_in.run_told(
        &arguments,
        &[("CHANGED_CHECKS_PASSED", "fedcba9876543210fedc a")],
    );

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
        "what changed is still named"
    );
    assert!(
        text(&ran.stderr).contains("a passed at fedcba987654 already, so it isn't built again"),
        "{}",
        text(&ran.stderr)
    );
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.new\n")
    );

    fs::remove_file(stand_in.path("built")).expect("removed");
    let ran = stand_in.run_told(
        &arguments,
        &[("CHANGED_CHECKS_PASSED", "fedcba9876543210fedc a new")],
    );
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(stand_in.built(), "", "with each passed, nothing is built");

    // Passes at another commit, or told a tree with changes, say nothing
    // of this one.
    for (label, told) in [
        ("another commit", "0123456789abcdef0123 a new"),
        ("changes", ""),
    ] {
        let told = if told.is_empty() {
            fs::remove_file(stand_in.path("clean")).expect("removed");
            "fedcba9876543210fedc a new"
        } else {
            told
        };
        let ran = stand_in.run_told(&arguments, &[("CHANGED_CHECKS_PASSED", told)]);
        assert!(ran.status.success(), "{label}: {}", text(&ran.stderr));
        assert_eq!(
            stand_in.built(),
            format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
            "{label}"
        );
        fs::remove_file(stand_in.path("built")).expect("removed");
    }
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
fn a_run_building_no_vm_test_builds_while_it_evaluates_the_vm_tests() {
    // On 11 November the 90 VM tests were 11% of the checks but half of
    // each run's evaluating, and a gate builds none of them, so its build
    // waited minutes on evaluations it never used.
    let stand_in = StandIn::new("beside");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("vm-beside-build"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
        "every check is named, the VM test among them"
    );
    let repo = stand_in.path("repo");
    let flake = format!("git+file://{}#checks.x86_64-linux", repo.display());
    assert_eq!(
        stand_in.built(),
        format!("--no-link --keep-going -L {flake}.new {flake}.a\n"),
        "the ordinary checks alone are built"
    );
}

#[test]
fn a_vm_test_that_stops_evaluating_beside_the_build_still_fails_the_run() {
    let stand_in = StandIn::new("beside-broken");
    stand_in.checks("base", BASE);
    stand_in.checks(
        "working",
        r#"{"a": "a2.drv", "b": "b.drv", "new": "n.drv", "vm-x": null}"#,
    );
    fs::write(stand_in.path("vm-beside-build"), "").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert_eq!(ran.status.code(), Some(1), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nremoved  gone\nbroken   vm-x\n",
        "the VM test that stopped evaluating is named with the rest"
    );
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
    let chunks: Vec<&serde_json::Value> = runs[0]["evaluated"][1]["chunks"]
        .as_array()
        .expect("the tree's chunks")
        .iter()
        .map(|chunk| &chunk["checks"])
        .collect();
    assert_eq!(
        chunks,
        [3, 1],
        "the tree's ordinary checks, then its VM test: {}",
        runs[0]
    );
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
    fs::write(stand_in.path("trace"), TRACE).expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--keep-base", "--here"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), "");
    assert!(text(&ran.stderr).contains("kept x86_64-linux's checks at 0123456789ab"));
    let kept = Path::new("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(stand_in.folder.join(kept).exists());
    assert!(
        stand_in.path(READS).exists(),
        "a keep records what the VM tests read as it evaluates them, so every kept base has \
         its record"
    );
}

#[test]
fn keeping_a_base_a_gate_kept_first_traces_only_its_vm_tests() {
    let stand_in = StandIn::new("reads-gated");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    fs::create_dir_all(stand_in.path("cache/changed-checks")).expect("made");
    let kept = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.json";
    fs::write(stand_in.path(kept), BASE).expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        stand_in.path(READS).exists(),
        "a base kept from git alone got no record"
    );
    let runs = stand_in.runs();
    assert_eq!(evaluated(&runs[0]), ["reads"], "{}", runs[0]);
}

#[test]
fn a_keep_whose_trace_fails_keeps_the_base_its_vm_tests_evaluated_from_git() {
    let stand_in = StandIn::new("reads-fallback");
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), "").expect("written");

    let ran = stand_in.run(&KEEPING);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let kept = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.json";
    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(kept)).expect("kept"))
            .expect("JSON");
    assert_eq!(kept["vm-x"], "v.drv", "the VM test went unkept: {kept}");
    let runs = stand_in.runs();
    assert_eq!(
        evaluated(&runs[0]),
        ["base", "base"],
        "the ordinary checks, then the VM tests from git: {}",
        runs[0]
    );
}

/// What the base's VM test read, as strace saw it: its flake, a file it
/// imported and a folder it listed.
const TRACE: &str = r#"openat(AT_FDCWD</tmp>, "SOURCE/flake.nix", O_RDONLY) = 7<SOURCE/flake.nix>
4242 newfstatat(AT_FDCWD</tmp>, "SOURCE/tests", {st_mode=S_IFDIR|0555, ...}, 0) = 0
4242 openat(AT_FDCWD</tmp>, "SOURCE/tests/x.nix", O_RDONLY|O_CLOEXEC) = 7<SOURCE/tests/x.nix>
4243 openat(AT_FDCWD</tmp>, "SOURCE/listed", O_RDONLY|O_DIRECTORY) = 8<SOURCE/listed>
4243 getdents64(8<SOURCE/listed>, 0x5555 /* 3 entries */, 32768) = 80
"#;

/// A stand-in whose base has been kept, with what its VM test read, and
/// whose working tree changes `diff`'s files.
fn kept_with_reads(test: &str, diff: &str) -> StandIn {
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", WORKING);
    fs::write(stand_in.path("diff"), diff).expect("written");
    stand_in
}

/// Keeping a base, as keep-main asks for it.
const KEEPING: [&str; 4] = ["--system", "x86_64-linux", "--keep-base", "--here"];

/// Where a base's record of what its VM tests read is kept.
const READS: &str = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.reads.json";

#[test]
fn keeping_a_base_keeps_what_its_vm_tests_read_and_how_long_reading_took() {
    let stand_in = kept_with_reads("reads-kept", "");

    let kept: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(stand_in.path(READS)).expect("kept"))
            .expect("JSON");
    assert_eq!(
        kept,
        serde_json::json!({
            "tree": "basetree",
            "read": ["flake.nix", "listed", "tests", "tests/x.nix"],
            "missing": [],
            "listed": ["listed"],
        })
    );
    let runs = stand_in.runs();
    assert_eq!(evaluated(&runs[0]), ["base", "reads"], "{}", runs[0]);
}

#[test]
fn a_gate_whose_change_reaches_nothing_the_vm_tests_read_keeps_their_base_derivations() {
    // On 12 November 95 of 152 gates that named anything named no VM test,
    // and each spent half its evaluating on them.
    // A listing gives names alone, so a file changed in a folder listed
    // moves nothing, nor one added in a folder that was there already.
    let stand_in = kept_with_reads(
        "reads-unreached",
        "M\ta\nA\tnew\nD\tgone\nM\tlisted/old.nix\nA\tlisted/kept/new.nix\n",
    );
    fs::create_dir_all(stand_in.path("repo/listed/kept")).expect("made");
    fs::write(stand_in.path("based"), "listed/kept\n").expect("written");

    let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(
        text(&ran.stdout),
        "added    new\nchanged  a\nremoved  gone\n",
        "the VM test is the base's, so unchanged"
    );
    assert!(
        text(&ran.stderr).contains(
            "the change reaches nothing x86_64-linux's 1 VM tests read at 0123456789ab, \
             so each keeps the base's derivation unevaluated"
        ),
        "{}",
        text(&ran.stderr)
    );
    let runs = stand_in.runs();
    let tree = &runs[1]["evaluated"][0];
    assert_eq!(tree["of"], "tree", "{}", runs[1]);
    assert_eq!(
        tree["chunks"].as_array().map(Vec::len),
        Some(1),
        "only ordinary: {tree}"
    );
}

#[test]
fn a_gate_whose_change_reaches_what_the_vm_tests_read_evaluates_them() {
    for diff in [
        "M\ttests/x.nix\n",
        "A\tlisted/new.nix\n",
        "A\tlisted/came/new.nix\n",
        "D\ttests/y.nix\n",
        "M\tflake.lock\n",
    ] {
        let stand_in = kept_with_reads("reads-reached", diff);
        fs::create_dir_all(stand_in.path("repo/listed/came")).expect("made");

        let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert_eq!(
            text(&ran.stdout),
            "added    new\nchanged  a\nchanged  vm-x\nremoved  gone\n",
            "{diff:?} reaches the VM test, so it was evaluated"
        );
    }
}

#[test]
fn a_record_made_from_another_tree_or_a_change_git_cant_name_evaluates_them() {
    for (test, odd) in [("reads-tree", "tree"), ("reads-rename", "diff")] {
        let stand_in = kept_with_reads(test, "M\ta\n");
        let said = if odd == "tree" {
            "othertree\n"
        } else {
            "R100\ta\tb\n"
        };
        fs::write(stand_in.path(odd), said).expect("written");

        let ran = stand_in.run(&["--system", "x86_64-linux", "--build", "--no-vm"]);

        assert!(ran.status.success(), "{}", text(&ran.stderr));
        assert!(
            text(&ran.stdout).contains("changed  vm-x\n"),
            "{odd} was odd, yet the record was used"
        );
    }
}

#[test]
fn a_base_whose_trace_cant_be_read_or_whose_inputs_are_local_keeps_no_record() {
    let unread = "4242 chdir(\"SOURCE/tests\") = 0\n";
    let lock = r#"{"nodes": {"x": {"locked": {"type": "path", "path": "/x"}}}}"#;
    for (test, trace, lock) in [
        ("reads-unread", unread, None),
        ("reads-empty", "", None),
        ("reads-local", TRACE, Some(lock)),
    ] {
        let stand_in = StandIn::new(test);
        stand_in.checks("base", BASE);
        fs::write(stand_in.path("trace"), trace).expect("written");
        if let Some(lock) = lock {
            fs::create_dir_all(stand_in.path("source")).expect("made");
            fs::write(stand_in.path("source/flake.lock"), lock).expect("written");
        }

        let ran = stand_in.run(&KEEPING);

        assert!(ran.status.success(), "{test}: {}", text(&ran.stderr));
        assert!(
            text(&ran.stderr)
                .contains("couldn't record what x86_64-linux's VM tests read at 0123456789ab"),
            "{test}: {}",
            text(&ran.stderr)
        );
        assert!(!stand_in.path(READS).exists(), "{test} kept a record");
    }
}

#[test]
fn a_run_evaluating_here_waits_its_turn_while_another_holds_it() {
    let stand_in = StandIn::new("turns");
    stand_in.checks("base", BASE);
    let lock = stand_in.path("cache/changed-checks/evaluating-here.lock");
    fs::create_dir_all(lock.parent().expect("its folder")).expect("made");
    let held = fs::File::create(&lock).expect("the lock");
    held.lock().expect("held");

    let mut running = stand_in.started(&["--system", "x86_64-linux", "--keep-base", "--here"], &[]);
    let mut said = std::io::BufReader::new(running.stderr.take().expect("its stderr"));
    let mut line = String::new();
    while !line.contains("another run evaluates on this machine, so this waits its turn") {
        line.clear();
        let read = std::io::BufRead::read_line(&mut said, &mut line).expect("read");
        assert_ne!(read, 0, "it ended without waiting its turn");
    }
    let kept = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(!kept.exists(), "nothing is evaluated while it waits");
    drop(held);

    let ran = running.wait_with_output().expect("it runs");
    let rest = std::io::read_to_string(said).expect("read");
    assert!(ran.status.success(), "{rest}");
    assert!(kept.exists(), "kept once its turn came: {rest}");
}

#[test]
fn keeping_a_base_elsewhere_never_asks_for_a_builder_nor_goes_where_it_must() {
    let stand_in = StandIn::elsewhere("keeps-elsewhere");
    stand_in.checks("base", BASE);
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("clean"), "").expect("written");
    let keeping = ["--system", "x86_64-linux", "--keep-base"];

    // A nix-offload that can't be told never to ask isn't sent the keep,
    // and it isn't evaluated here instead.
    let ran = stand_in.run(&keeping);
    assert_eq!(ran.status.code(), Some(75), "{}", text(&ran.stderr));
    assert!(text(&ran.stderr).contains("so it's left unkept; --here keeps it on this machine"));
    assert!(!stand_in.path("changed").exists(), "it wasn't sent");
    let kept = stand_in.path("cache/changed-checks/0123456789abcdef0123-x86_64-linux.json");
    assert!(!kept.exists(), "nor kept here");

    fs::write(stand_in.path("takes-never-ask"), "").expect("written");
    let ran = stand_in.run(&keeping);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--never-ask\n--changed\n--system\nx86_64-linux\n--keep-base\n"),
        "{sent}"
    );
}

#[test]
fn a_session_asking_to_go_first_waits_its_turn_and_says_so() {
    let stand_in = StandIn::elsewhere("first");
    fs::create_dir_all(stand_in.path("repo")).expect("the repository");
    fs::write(stand_in.path("takes-first"), "").expect("written");
    fs::write(stand_in.path("clean"), "").expect("written");

    // A test runs outside the net's unit, as any session does.
    let ran = stand_in.run(&["--system", "x86_64-linux", "--name-only", "--first"]);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("--first is the net's alone, so this run waits its turn"),
        "{}",
        text(&ran.stderr)
    );
    let sent = fs::read_to_string(stand_in.path("changed")).expect("it was sent");
    assert!(
        sent.ends_with("there\n--changed\n--system\nx86_64-linux\n--name-only\n"),
        "neither nix-offload nor the far side is told to go first: {sent}"
    );
}

#[test]
fn a_base_sharing_no_history_is_refused_as_asked_wrongly() {
    let stand_in = StandIn::new("unrelated");

    let ran = stand_in.run(&["--base", "elsewhere", "--system", "x86_64-linux"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(text(&ran.stderr).contains("elsewhere shares no history with HEAD"));
}

#[test]
fn unnamed_the_system_compared_is_the_one_with_the_most_checks() {
    // A run on a Mac compared its own checks and passed over the checks
    // about checks a repository held on Linux alone, as #1373 did.
    let stand_in = StandIn::new("busiest");
    stand_in.checks("base", BASE);
    stand_in.checks("working", WORKING);
    fs::write(
        stand_in.path("systems.json"),
        r#"{"aarch64-darwin": 9, "x86_64-linux": 4}"#,
    )
    .expect("written");

    let ran = stand_in.run(&["--name-only"]);

    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert!(
        text(&ran.stderr).contains("comparing aarch64-darwin's checks"),
        "{}",
        text(&ran.stderr)
    );
}

#[test]
fn an_option_it_doesnt_take_is_refused_saying_how_it_is_used() {
    let stand_in = StandIn::new("misused");

    let ran = stand_in.run(&["--bild"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(text(&ran.stderr).starts_with("changed-checks: --bild isn't something it takes\n"));

    let ran = stand_in.run(&["--build", "--name-only"]);

    assert_eq!(ran.status.code(), Some(64));
    assert!(
        text(&ran.stderr).contains("--build and --name-only"),
        "{}",
        text(&ran.stderr)
    );
}
