//! changed-checks run as a person runs it, with stand-ins for git and nix
//! first on its path: git names a repository and a merge base, and nix
//! names each side's checks and their derivations from files the test
//! writes, and records what it's asked to build. The stand-ins and what
//! the tests share are here; the tests are beside this, a file to each of
//! building, evaluating, keeping a base, what each check read alone, and
//! the command.

mod alone;
mod building;
mod command;
mod evaluating;
mod keeping;
mod red_first;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
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
///
/// Asked of the change as red first asks it, its commits and their
/// Red-first trailers are `log` (one commit of its own with none, without
/// it), its files with renames found are `diff.whole`, the
/// lines each adds and takes are `diff.counted` (one added to each file
/// asked, without it), and its added lines are `diff.lines`. Laying its tests
/// over the base, each file it lists, only as its literal path, is a blob
/// of the head's, what it indexes
/// is added to `indexed`, and each commit it makes is added to `commits`
/// and is `a1a1` for the tests alone and `b0b0` with files nothing names.
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
          "--is-ancestor 9876543210fedcba9876") ;;
          *) exit 1 ;;
        esac ;;
      ls-files) ;;
      rev-parse) cat "$STAND_IN/tree" 2>/dev/null || echo basetree ;;
      diff)
        case "$4" in
          --find-renames) cat "$STAND_IN/diff.whole" 2>/dev/null || : ;;
          --numstat)
            if [ -e "$STAND_IN/diff.counted" ]; then cat "$STAND_IN/diff.counted"
            else shift 9; for file in "$@"; do printf '1\t0\t%s\n' "$file"; done; fi ;;
          -U0) cat "$STAND_IN/diff.lines" 2>/dev/null || : ;;
          *)
            if [ -n "$7" ]; then cat "$STAND_IN/diff.between" 2>/dev/null
            else cat "$STAND_IN/diff" 2>/dev/null; fi ;;
        esac ;;
      cat-file)
        if [ "$4" = commit ]; then
          printf 'tree basetree\nauthor someone <a@b> 7 +0000\n'
          printf 'committer someone <a@b> 7 +0000\n\nbase\n'
        else grep -qxF "${5#*:}" "$STAND_IN/based" 2>/dev/null; fi ;;
      log) cat "$STAND_IN/log" 2>/dev/null || printf 'change\037\036' ;;
      read-tree|write-tree)
        if [ -e "$STAND_IN/unwritable" ]; then exit 1; fi
        echo laidtree ;;
      ls-tree)
        [ "$GIT_LITERAL_PATHSPECS" = 1 ] || exit 1
        shift 6; for file in "$@"; do printf '100644 blob 1111\t%s\0' "$file"; done ;;
      update-index) tr '\0' '\n' >> "$STAND_IN/indexed" ;;
      hash-object)
        if [ "$4" = -t ]; then
          commit=$(cat)
          printf '%s\n' "$commit" >> "$STAND_IN/commits"
          case "$commit" in
            *'nothing names'*) echo b0b0b0b0b0b0b0b0b0b0 ;;
            *) echo a1a1a1a1a1a1a1a1a1a1 ;;
          esac
        else cat > /dev/null; echo e69de29bb2d1d6434b8b29ae775ad8c2e48c5391; fi ;;
      show) cat "$STAND_IN/quarantine.json" 2>/dev/null || exit 128 ;;
    esac ;;
esac
"#;

/// nix as changed-checks asks it: a side's checks by name, then their
/// derivations, the base's being any flake at a revision but red first's
/// commits, `a1a1`'s being `over` and `b0b0`'s `probed`; and a build,
/// which it records, and which fails when it names a check `failing` lists,
/// or, of a flake at a revision, one `failing-over` lists,
/// saying `failing.said` as its log. It refuses an evaluation through its evaluation
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
  store) echo "$STAND_IN/copy"; exit 0 ;;
  build)
    shift
    echo "$@" | sed 's/ --option pure-eval true --option accept-flake-config false//' \
      >> "$STAND_IN/built"
    exec 3>&2
    for word in "$@"; do
      list="$STAND_IN/failing"
      case "$word" in *'?rev='*) list="$STAND_IN/failing-over" ;; esac
      while read -r check; do
        case "$word" in *".$check") cat "$STAND_IN/failing.said" >&3 2>/dev/null; exit 1 ;; esac
      done < "$list"
    done 2>/dev/null
    exit 0 ;;
esac
if [ "$2" = --raw ]; then echo "$STAND_IN/source"; exit 0; fi
case "$3" in
  *'.checkFiles') [ -e "$STAND_IN/files.json" ] || exit 1; cat "$STAND_IN/files.json"; exit 0 ;;
esac
case "$5" in
  builtins.mapAttrs*)
    cat "$STAND_IN/systems.json" 2>/dev/null || echo '{"x86_64-linux": 4}'
    exit 0 ;;
esac
[ "$5" = builtins.currentSystem ] && { echo "asked Nix for this machine's system" >&2; exit 1; }
[ "$6" = --no-eval-cache ] || { echo "evaluated through the cache" >&2; exit 1; }
side=working
case "$3" in
  *'?rev=a1a1'*) side=over ;;
  *'?rev=b0b0'*) side=probed ;;
  *'?rev='*|path:*) side=base ;;
esac
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

/// nix-store as changed-checks asks it what takes the flake's source in:
/// what `referrers` lists, once a test writes it, and else nothing; and,
/// once asked of the copy of the source `nix store add` names, what
/// `copy-referrers` lists too.
const NIX_STORE: &str = r#"#!/bin/sh
[ "$1 $2" = "--query --referrers-closure" ] || exit 1
shift 2
cat "$STAND_IN/referrers" 2>/dev/null
for path in "$@"; do
  [ "$path" = "$STAND_IN/copy" ] && cat "$STAND_IN/copy-referrers" 2>/dev/null
done
exit 0
"#;

/// strace as changed-checks asks it: the trace it writes is `trace`, each
/// SOURCE in it the flake's source, and it runs what it traces, leaving
/// `traced` to say it began. A check evaluated alone, NAME, is traced as
/// `trace.NAME` says, once a test writes it, and waits first until a test
/// has opened and closed `holds.NAME`, a pipe, once it makes one.
const STRACE: &str = r#"#!/bin/sh
touch "$STAND_IN/traced"
trace="$STAND_IN/trace"
for word in "$@"; do
  case "$word" in
    *'checks: builtins.mapAttrs'*)
      name=$(printf '%s' "$word" | sed -n 's/.*\["\([^" ]*\)"\].*/\1/p')
      [ -n "$name" ] && [ -e "$STAND_IN/trace.$name" ] && trace="$STAND_IN/trace.$name"
      [ -n "$name" ] && [ -p "$STAND_IN/holds.$name" ] && read -r _ < "$STAND_IN/holds.$name" ;;
  esac
done
while [ "$1" != -o ]; do shift; done
sed "s|SOURCE|$STAND_IN/source|g" "$trace" > "$2" 2>/dev/null || : > "$2"
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
            ("nix-store", NIX_STORE),
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
            .env(
                "NIX_OFFLOAD_STANDING_DOOR",
                self.folder.join("standing.sock"),
            )
            // No stall, whatever the machine running the tests holds.
            .env("CHANGED_CHECKS_PRESSURE", self.folder.join("pressure"))
            .env("CHANGED_CHECKS_STRACE", self.folder.join("bin/strace"))
            // A run is the train's candidate only where a case says so.
            .env_remove("CHANGED_CHECKS_CANDIDATE")
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

/// What the base's VM test read, as strace saw it: its flake, a file it
/// imported and a folder it listed.
const TRACE: &str = r#"openat(AT_FDCWD</tmp>, "SOURCE/flake.nix", O_RDONLY) = 7<SOURCE/flake.nix>
4242 newfstatat(AT_FDCWD</tmp>, "SOURCE/tests", {st_mode=S_IFDIR|0555, ...}, 0) = 0
4242 openat(AT_FDCWD</tmp>, "SOURCE/tests/x.nix", O_RDONLY|O_CLOEXEC) = 7<SOURCE/tests/x.nix>
4243 openat(AT_FDCWD</tmp>, "SOURCE/listed", O_RDONLY|O_DIRECTORY) = 8<SOURCE/listed>
4243 getdents64(8<SOURCE/listed>, 0x5555 /* 3 entries */, 32768) = 80
"#;

/// Keeping a base, as keep-main asks for it.
const KEEPING: [&str; 4] = ["--system", "x86_64-linux", "--keep-base", "--here"];

/// Where a base's record of what its VM tests read is kept.
const READS: &str = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.reads.json";

/// Where what each of the base's ordinary checks read alone is kept.
const ALONE: &str = "cache/changed-checks/0123456789abcdef0123-x86_64-linux.alone.reads.json";

/// The base's checks as the tree has them, each but the VM test moved.
const MOVED: &str = r#"{"a": "a2.drv", "b": "b2.drv", "gone": "g2.drv", "vm-x": "v.drv"}"#;

/// A stand-in whose base is kept with what each ordinary check read alone,
/// its flake and a file named for it, whose tree's checks are `working`,
/// and whose change is `diff`.
fn kept_alone(test: &str, working: &str, diff: &str) -> StandIn {
    let stand_in = traced_alone(test);
    let ran = stand_in.run(&KEEPING);
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    stand_in.checks("working", working);
    fs::write(stand_in.path("diff"), diff).expect("written");
    stand_in
}

/// A stand-in whose base's checks each read their own file alone.
fn traced_alone(test: &str) -> StandIn {
    let stand_in = StandIn::new(test);
    stand_in.checks("base", BASE);
    fs::write(stand_in.path("trace"), TRACE).expect("written");
    for name in ["a", "b", "gone"] {
        let trace = format!(
            "openat(AT_FDCWD</tmp>, \"SOURCE/flake.nix\", O_RDONLY) = 7<SOURCE/flake.nix>\n\
             openat(AT_FDCWD</tmp>, \"SOURCE/{name}.nix\", O_RDONLY) = 8<SOURCE/{name}.nix>\n"
        );
        fs::write(stand_in.path(&format!("trace.{name}")), trace).expect("written");
    }
    stand_in
}

/// The base's record of what each check read, kept alone.
fn alone_record(stand_in: &StandIn, kept: &str) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(stand_in.path(kept)).expect("kept")).expect("JSON")
}

/// A gate, as merge-net runs one.
const GATING: [&str; 4] = ["--system", "x86_64-linux", "--build", "--no-vm"];

/// A head of main the base came after, whose checks' reads were recorded.
const RECORDED: &str = "9876543210fedcba9876";

/// Keep the base, then file what was kept as `head`'s, so the base is a
/// head of main past it with nothing kept of its own; `between` is what main
/// changed from `head` to the base, `diff` what changed from `head` to the
/// working tree.
fn kept_behind(test: &str, head: &str, between: &str, diff: &str) -> StandIn {
    let stand_in = kept_alone(test, MOVED, diff);
    filed_as(&stand_in, head);
    fs::write(stand_in.path("diff.between"), between).expect("written");
    stand_in
}

/// File what was kept of the base as `head`'s.
fn filed_as(stand_in: &StandIn, head: &str) {
    for kept in ["json", "reads.json", "alone.reads.json"] {
        let at = |commit: &str| {
            stand_in.path(&format!(
                "cache/changed-checks/{commit}-x86_64-linux.{kept}"
            ))
        };
        fs::rename(at("0123456789abcdef0123"), at(head)).expect("filed as the head's");
    }
}
