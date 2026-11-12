//! `nix-offload there` run as a caller runs it, with stand-ins for ssh and
//! nix on its PATH that record what they were given.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

/// ssh as the builder answers: it writes its verbose log where `-E` says,
/// records every word it was given and its last two (the host and the
/// command) apart, each command after the last in `ssh-commands` and each
/// host after the last in `ssh-hosts`, and both on one line in `ssh-sent`,
/// written at once, for a run whose parts go at once, prints
/// what the `says` file beside it holds and, on stderr, what the
/// `complains` file holds, and exits as the `status` file beside it says.
/// Once a test writes `refuses-name-only`, a command asking `--name-only`
/// is refused with 64, as a changed-checks from before it was refuses it,
/// and once it writes `evaluates-for`, such a command takes that many
/// seconds, adding how many evaluate on its host at once to
/// `evaluating-at-once-<host>` as it starts.
/// A host an `unreached-<host>` file names is never reached, exiting 255,
/// and one a `busy-<host>` file names says it's busy, 76, to a run marked
/// to move on rather than wait.
/// A read of the builder's CPU pressure prints the `pressure-<host>` file
/// beside it, when there is one, and nothing else, and a dry run of a
/// build prints the `plans` file, when there is one, and nothing else.
/// What it's asked to fetch goes in the `fetched` file, and it says it
/// fetched each but those the `refuses-fetch` file names, a line each.
/// Asked for a base's kept checks, it prints the `kept-checks-<host>` file,
/// and for what its VM tests read, `kept-reads-<host>`, or ends 1 without
/// one, and given either to keep, it adds it to `took-kept-<host>`; none
/// of these counts as a run.
const SSH: &str = r#"#!/bin/sh
here=$(dirname "$0")
echo "$@" > "$here/ssh-options"
while [ "$#" -gt 2 ]; do
  if [ "$1" = -E ]; then log=$2; fi
  shift
done
# What changed-checks keeps of its runs there, the far-runs file: its
# length, then what follows a length, unless far-runs-unreadable says
# it can't be read. Asked around a run, so it's kept apart from it.
case "$2" in
  *changed-checks/runs.jsonl*)
    [ -f "$here/unreached-$1" ] && exit 255
    [ -f "$here/far-runs-unreadable" ] && exit 1
    case "$2" in
      *"wc -c"*) wc -c 2>/dev/null < "$here/far-runs" || echo 0 ;;
      *) [ -e "$here/far-runs" ] || exit 0
        from=${2##*tail -c +}; tail -c "+${from%% *}" "$here/far-runs" ;;
    esac
    exit 0 ;;
  # A base's kept checks, read from the builder a run went to and given
  # to the other. Asked after a run, so it's kept apart from it.
  *changed-checks/*.copying*)
    [ -f "$here/unreached-$1" ] && exit 255
    cat >> "$here/took-kept-$1"
    exit 0 ;;
  *"cat \""*/changed-checks/*.reads.json\"*)
    [ -f "$here/unreached-$1" ] && exit 255
    [ -f "$here/kept-reads-$1" ] || exit 1
    cat "$here/kept-reads-$1"
    exit 0 ;;
  *"cat \""*/changed-checks/*.json\"*)
    [ -f "$here/unreached-$1" ] && exit 255
    [ -f "$here/kept-checks-$1" ] || exit 1
    cat "$here/kept-checks-$1"
    exit 0 ;;
  # Whether changed-checks keeps a base evaluated there: the base
  # kept-<host> names. Asked before a run, so it's kept apart from it.
  *"test -e \""*/changed-checks/*)
    [ -f "$here/unreached-$1" ] && exit 255
    printf '%s\n' "$1" >> "$here/asked-kept"
    [ -f "$here/kept-$1" ] || exit 1
    case "$2" in *"/$(cat "$here/kept-$1")-"*) exit 0 ;; esac
    exit 1 ;;
esac
printf '%s\n%s\n' "$1" "$2" > "$here/ssh-was-given"
printf '%s\n' "$2" >> "$here/ssh-commands"
printf '%s\n' "$1" >> "$here/ssh-hosts"
# Written whole, then appended in one write, since a run's parts call
# this at once and a long command printed straight on took more than
# one, so two lines could interleave and a host's go missing.
printf '%s %s\n' "$1" "$2" > "$here/ssh-line-$$"
cat "$here/ssh-line-$$" >> "$here/ssh-sent"
rm -f "$here/ssh-line-$$"
[ -f "$here/unreached-$1" ] && exit 255
if [ -f "$here/busy-$1" ]; then
  case "$2" in *": NIX_OFFLOAD_MOVES=1; "*) exit 76 ;; esac
fi
case "$2" in
  *--name-only*) [ -f "$here/refuses-name-only" ] && exit 64
    [ -f "$here/evaluates-forever" ] && exec sleep 120
    if [ -f "$here/evaluates-for" ]; then
      mkdir -p "$here/evaluating-$1"
      : > "$here/evaluating-$1/$$"
      ls "$here/evaluating-$1" | wc -l >> "$here/evaluating-at-once-$1"
      sleep "$(cat "$here/evaluates-for")"
      rm -f "$here/evaluating-$1/$$"
    fi ;;
  */proc/pressure/cpu*) cat "$here/pressure-$1" 2>/dev/null; exit 0 ;;
  *--dry-run*) cat "$here/plans" 2>/dev/null; exit 0 ;;
  *"&& echo"*) cat > "$here/fetched"
    grep -vxF -f "$here/refuses-fetch" "$here/fetched" 2>/dev/null \
      || { [ -f "$here/refuses-fetch" ] || cat "$here/fetched"; }
    exit 0 ;;
esac
echo "${WORK_RUNS_ELSEWHERE:-}" > "$here/ssh-marked"
echo "Transferred: sent 3812, received 9 bytes, in 1.0 seconds" > "$log"
[ -f "$here/says" ] && cat "$here/says"
[ -f "$here/complains" ] && cat "$here/complains" >&2
case "$2" in
  *"nix build"*)
    if [ -f "$here/holds-$1" ]; then
      mkdir -p "$here/building-$1"
      : > "$here/building-$1/$$"
      ls "$here/building-$1" | wc -l >> "$here/at-once-$1"
      sleep "$(cat "$here/holds-$1")"
      rm -f "$here/building-$1/$$"
    fi ;;
esac
# A run of changed-checks keeps a line of its own, while far-runs-grow says.
case "$2" in
  *changed-checks*) [ -f "$here/far-runs-grow" ] \
    && printf '{"at":%s}\n' "$$" >> "$here/far-runs" ;;
esac
exit "$(cat "$here/status")"
"#;

/// Write a script to run through `cp`, so no thread of this process ever
/// holds it open for writing: a test forking meanwhile would keep that
/// handle until its child execs, and a script run then is refused as busy
/// (ETXTBSY), which failed a run on 31 October.
fn executable(path: &Path, script: &str) {
    let draft = path.with_extension("draft");
    fs::write(&draft, script).unwrap();
    let copied = Command::new("cp").arg(&draft).arg(path).status().unwrap();
    assert!(copied.success(), "copying {}", path.display());
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// nix as it builds here: it records its arguments, each call's after the
/// last in `nix-calls`, and exits 0, or as the `nix-status` file beside it
/// says. While the `nix-fails` file counts above 0, a call fails with 1
/// and counts it down. `derivation show` prints the `derivations` file,
/// `path-info --json` the `held` file and `path-info --recursive` the
/// `closure` file, when there is one. Sending held outputs up takes as
/// many seconds as the `copy-takes` file says, when there is one, and a
/// send of the path the `slow-to-send` file names takes 30.
const NIX: &str = r#"#!/bin/sh
here=$(dirname "$0")
echo "$@" > "$here/nix-was-given"
echo "$@" >> "$here/nix-calls"
echo "${WORK_RUNS_ELSEWHERE:-}" > "$here/nix-marked"
case "$*" in
  "copy --to "*--substitute-on-destination*)
    [ -e "$here/copy-takes" ] && exec sleep "$(cat "$here/copy-takes")"
    slow=$(cat "$here/slow-to-send" 2>/dev/null)
    case "$*" in *" $slow"*) [ -n "$slow" ] && exec sleep 30 ;; esac ;;
esac
case "$1 $2" in
  "derivation show") cat "$here/derivations" 2>/dev/null ;;
  "path-info --json") cat "$here/held" 2>/dev/null ;;
  "path-info --recursive") cat "$here/closure" 2>/dev/null ;;
esac
fails=$(cat "$here/nix-fails" 2>/dev/null || echo 0)
if [ "$fails" -gt 0 ]; then
  echo $((fails - 1)) > "$here/nix-fails"
  exit 1
fi
exit "$(cat "$here/nix-status" 2>/dev/null || echo 0)"
"#;

/// A second lender's folder that holds nothing, so a run never finds a
/// second builder unless a test lends it one: the default,
/// /run/nix-offload-more, is this machine's own.
const NO_SECOND: &str = "/nonexistent/nix-offload-more";

/// And a second VM lender's, as /run/nix-offload-vm-more is this
/// machine's own.
const NO_SECOND_VM: &str = "/nonexistent/nix-offload-vm-more";

/// And the VM lenders' past the second, as /run/nix-offload-vm-3 to -16
/// are this machine's own: left to them, a run here split its VM tests
/// between the real ones from 6 November.
const NO_MORE_VMS: &str = "/nonexistent/nix-offload-vm-3";

/// And a keeper's unit, as this machine's own may run one.
const NO_KEEPER: &str = "/nonexistent/nix-offload-keep.socket";

const STORE: &str = "ssh-ng://nix-offload?ssh-key=/key&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n";

fn stand_ins(folder: &Path, status: u8) {
    for (name, script) in [("ssh", SSH), ("nix", NIX)] {
        let path = folder.join(name);
        executable(&path, script);
    }
    fs::write(folder.join("status"), status.to_string()).unwrap();
}

/// `nix-offload there` among its stand-ins: the programs in `bin` first
/// on its path, `offload` as its lender's folder, `state` as its runtime,
/// state and config folders, and no second lender or VM lender, whatever
/// this machine's own hold, since a run no lent builder takes goes to one
/// that does. Nor does it keep its passes with this machine's keeper,
/// whose record gates read. A test adds its arguments, and whatever it
/// sets besides goes over what this sets.
fn there_command(bin: &Path, offload: &Path, state: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nix-offload"));
    command
        .arg("there")
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", offload)
        .env("NIX_OFFLOAD_MORE_DIR", NO_SECOND)
        .env("NIX_OFFLOAD_KEEP_UNIT", NO_KEEPER)
        .envs(no_vm_builders(state))
        .env("XDG_RUNTIME_DIR", state)
        .env("XDG_STATE_HOME", state)
        .env("XDG_CONFIG_HOME", state);
    command
}

fn there(folder: &Path, offload: &Path) -> Output {
    there_with(folder, offload, &[])
}

/// `there` given `extra` arguments besides.
fn there_with(folder: &Path, offload: &Path, extra: &[&str]) -> Output {
    there_command(folder, offload, folder)
        .args(["-L", "github:o/r/abc", "checks.x.a", "--keep-going"])
        .args(extra)
        .output()
        .unwrap()
}

/// `there` given these arguments alone.
fn there_asked(folder: &Path, offload: &Path, arguments: &[&str]) -> Output {
    there_command(folder, offload, folder)
        .args(arguments)
        .output()
        .unwrap()
}

/// What ssh was told, `host\ncommand\n`, with the command as it was
/// before what makes it end with its caller, which every one is given.
fn given_to(folder: &Path) -> String {
    let given = fs::read_to_string(folder.join("ssh-was-given")).unwrap();
    let (host, command) = given.trim_end().split_once('\n').unwrap();
    format!("{host}\n{}\n", within_far(command))
}

/// A command sent far, without what makes it end with its caller.
fn within_far(command: &str) -> &str {
    command
        .split_once("trap 'kill $watching 2>/dev/null; exit 143' HUP TERM; ")
        .and_then(|(_, rest)| {
            rest.strip_suffix("; status=$?; kill $watching 2>/dev/null; exit $status")
        })
        .unwrap_or_else(|| panic!("it doesn't end with its caller: {command}"))
}

fn journalled(folder: &Path) -> serde_json::Value {
    let journal = fs::read_to_string(folder.join("nix-offload/there.jsonl")).unwrap();
    serde_json::from_str(journal.lines().last().unwrap()).unwrap()
}

#[test]
fn a_check_passed_at_a_commit_isnt_built_again_wherever_it_would_go() {
    // Each builder lent starts with an empty store and passes nothing
    // back, so a check passed on one was built again on the next: 716 s,
    // then 790 s, for one at one commit on 3 November.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    let commit = "0123456789abcdef0123456789abcdef01234567";
    let flake = format!("github:o/r/{commit}");
    let sent = || {
        let given = folder.path().join("ssh-was-given");
        let sent = fs::read_to_string(&given)
            .ok()
            .map(|_| given_to(folder.path()));
        let _ = fs::remove_file(given);
        sent
    };

    let ran = there_asked(
        folder.path(),
        &offload,
        &[&flake, "checks.x.a", "checks.x.b"],
    );
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(sent().is_some(), "the first is built");

    let ran = there_asked(
        folder.path(),
        &offload,
        &[&flake, "checks.x.a", "checks.x.b"],
    );
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(sent(), None, "nothing is sent again");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("checks.x.a passed at 0123456789ab already, so it isn't built again"),
        "{said}"
    );
    let line = journalled(folder.path());
    assert_eq!(line["already"], true, "{line}");
    assert_eq!(line["on"], serde_json::Value::Null, "{line}");

    // Only what hasn't passed goes, and what fails is built again.
    fs::write(folder.path().join("status"), "1").unwrap();
    let ran = there_asked(
        folder.path(),
        &offload,
        &[&flake, "checks.x.a", "checks.x.c"],
    );
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert_eq!(
        sent().as_deref(),
        Some(format!("nix-offload\nnix build --no-link '{flake}#checks.x.c'\n").as_str())
    );
    let ran = there_asked(folder.path(), &offload, &[&flake, "checks.x.c"]);
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert!(sent().is_some(), "a failure is built again");

    // Another commit, or what's built for its output rather than its
    // passing, is built whatever passed.
    fs::write(folder.path().join("status"), "0").unwrap();
    let other = format!("github:o/r/{}", "f".repeat(40));
    for asked in [
        vec![other.as_str(), "checks.x.a"],
        vec!["--print-out-paths", &flake, "checks.x.a"],
        vec!["--apply", "check: check", &flake, "checks.x.a"],
        vec![&flake, "packages.x.a"],
        vec![&flake, "packages.x.a"],
    ] {
        let ran = there_asked(folder.path(), &offload, &asked);
        assert_eq!(ran.status.code(), Some(0), "{asked:?}: {ran:?}");
        assert!(sent().is_some(), "{asked:?} is built");
    }
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
        given_to(folder.path()),
        "nix-offload\nnix build --no-link -L --keep-going 'github:o/r/abc#checks.x.a'\n"
    );
    assert!(!folder.path().join("nix-was-given").exists());
    let line = journalled(folder.path());
    assert_eq!(line["on"], "nix-offload");
    assert_eq!(line["sent"], 3812);
    assert_eq!(line["status"], 100);
}

#[test]
fn a_build_asked_to_outlive_its_caller_is_sent_to_go_on_alone() {
    // A prebuild whose caller hit its own time limit at 05:14 on 3
    // November lost the kernel it was compiling five minutes later.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    let outliving = |extra: &[&str]| {
        let ran = there_with(folder.path(), &offload, extra);
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        let given = fs::read_to_string(folder.path().join("ssh-was-given")).unwrap();
        assert_eq!(
            given_to(folder.path()),
            "nix-offload\nnix build --no-link -L --keep-going 'github:o/r/abc#checks.x.a'\n"
        );
        given.contains("NIX_OFFLOAD_OUTLIVE") && given.contains("nix-offload-outliving-$$")
    };
    assert!(
        outliving(&["--outlive"]),
        "it wasn't sent to outlive its caller"
    );
    assert!(
        !outliving(&[]),
        "one not asked to was sent to outlive its caller"
    );
}

// On 9 November a switch's build was stopped for printing nothing for 12
// minutes, so how far a remote build has got is asked of its builder.
#[test]
fn a_lent_builders_progress_is_its_processor_time_and_bytes_written() {
    let folder = tempfile::tempdir().unwrap();
    let more = lending(folder.path(), "offload-more", "nix-offload-more");
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "17\n4\n").unwrap();
    let progress = |host: &str| {
        Command::new(env!("CARGO_BIN_EXE_nix-offload"))
            .args(["progress", host])
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    folder.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("NIX_OFFLOAD_DIR", folder.path().join("none"))
            .env("NIX_OFFLOAD_MORE_DIR", &more)
            .envs(no_vm_builders(folder.path()))
            .env("XDG_RUNTIME_DIR", folder.path())
            .output()
            .unwrap()
    };
    let ran = progress("nix-offload-more");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said: serde_json::Value = serde_json::from_slice(&ran.stdout).unwrap();
    assert_eq!(
        (said["cpuUsec"].as_u64(), said["writtenBytes"].as_u64()),
        (Some(17), Some(2048))
    );
    assert_eq!(said["host"], "nix-offload-more");
    let sent = fs::read_to_string(folder.path().join("ssh-was-given")).unwrap();
    assert!(sent.contains("nix-daemon.service/cpu.stat"), "{sent}");

    assert_eq!(progress("nowhere").status.code(), Some(75));
    fs::write(folder.path().join("unreached-nix-offload-more"), "").unwrap();
    assert_eq!(progress("nix-offload-more").status.code(), Some(255));
}

/// `there` given a derivation the builder lacks, run with `before` while
/// the test holds every turn at sending up for `held_for`, with how long
/// it took.
fn sent_up_while_the_uplink_is_taken(before: &[&str], held_for: Duration) -> (Output, Duration) {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-x.drv\n").unwrap();
    let turns = folder.path().join("nix-offload-there");
    fs::create_dir_all(&turns).unwrap();
    let held: Vec<fs::File> = (1..=3)
        .map(|turn| {
            let file = fs::File::create(turns.join(format!("uplink-{turn}"))).unwrap();
            file.lock().unwrap();
            file
        })
        .collect();
    let letting_go = std::thread::spawn(move || {
        // Long enough for the run to reach its copy and find every turn
        // taken, which takes it well under a second.
        std::thread::sleep(held_for);
        drop(held);
    });
    let began = std::time::Instant::now();
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(before)
        .args(["-L", "/nix/store/aaaa-x.drv"])
        .output()
        .unwrap();
    let took = began.elapsed();
    letting_go.join().unwrap();
    (ran, took)
}

#[test]
fn a_copy_up_waits_its_turn_while_three_go_up() {
    // On 10 November 30 runs at once shared an 11 Mbit/s uplink, and one
    // run's inputs took 77 minutes to go up.
    let (ran, took) = sent_up_while_the_uplink_is_taken(&[], Duration::from_secs(3));
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(said.contains("so this waits its turn to send"), "{said}");
    assert!(
        took >= Duration::from_secs(3),
        "sent before a turn was free: {took:?}"
    );
}

#[test]
fn a_copy_up_sent_first_takes_no_turn() {
    let (ran, took) = sent_up_while_the_uplink_is_taken(&["--first"], Duration::from_secs(20));
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(!said.contains("waits its turn to send"), "{said}");
    assert!(
        took < Duration::from_secs(20),
        "waited for a turn: {took:?}"
    );
}

#[test]
fn derivations_go_up_to_the_lent_builder_alone_and_build_there() {
    // A build tool's targets come as derivations evaluated here, with no
    // flake a builder could fetch, so they're copied up and built there.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    // The builder is asked once which it lacks, and says this one.
    fs::write(folder.path().join("says"), "/nix/store/aaaa-x.drv\n").unwrap();
    // A machine nearby that takes anything at all is never given them:
    // they may hold what only the builder is let build.
    let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    fs::write(
        folder.path().join("nix-offload/nearby"),
        format!("{} near x86_64-linux *\n", listening.local_addr().unwrap()),
    )
    .unwrap();
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(["-L", "/nix/store/aaaa-x.drv"])
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        format!(
            "copy --no-recursive --substitute-on-destination --to {} /nix/store/aaaa-x.drv\n",
            STORE.trim()
        )
    );
    let commands = fs::read_to_string(folder.path().join("ssh-commands")).unwrap();
    assert_eq!(
        commands.lines().next(),
        Some("xargs -r nix-store --check-validity --print-invalid")
    );
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link -L '/nix/store/aaaa-x.drv^*'\n"
    );
    // Each step before the build says what it does, since a fresh
    // builder given 104 derivations on 8 November took 15 minutes over
    // them while the run said nothing at all.
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("copying 1 derivation up to the lent builder nix-offload, which lacks it"),
        "{said}"
    );
    assert!(
        said.contains("asking the lent builder nix-offload what it would build of it"),
        "{said}"
    );

    // One that can't be copied up isn't built anywhere unasked.
    fs::write(folder.path().join("nix-status"), "1").unwrap();
    fs::remove_file(folder.path().join("ssh-was-given")).unwrap();
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(["/nix/store/aaaa-x.drv"])
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    assert!(
        !folder.path().join("ssh-was-given").exists(),
        "not built there"
    );
    // With no closure to ask about, it tried copying them whole.
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains(
            "copying the derivations up to the lent builder nix-offload with all they're \
             built from, since what it lacks is unknown"
        ),
        "{said}"
    );
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        format!(
            "copy --derivation --substitute-on-destination --to {} /nix/store/aaaa-x.drv\n",
            STORE.trim()
        )
    );
}

/// `there` given exactly `arguments`, with the stand-ins in `folder`.
fn there_given(folder: &Path, offload: &Path, arguments: &[&str]) -> Output {
    there_beside(
        folder,
        offload,
        Path::new("/nonexistent/nix-offload-vm"),
        arguments,
    )
}

/// `there` given `arguments`, with `vm` as the VM builder's lender folder.
fn there_beside(folder: &Path, offload: &Path, vm: &Path, arguments: &[&str]) -> Output {
    there_command(folder, offload, folder)
        .env("NIX_OFFLOAD_VM_DIR", vm)
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn a_send_that_falls_behind_stops_and_the_builder_builds_the_rest() {
    // On 9 November a send of 1.4 GB through an uplink four other tunnels
    // shared fell to 25 KB/s, and would have run past its run's two hours.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(
        folder.path().join("plans"),
        "these 2 derivations will be built:\n\
         \x20 /nix/store/aaaa-system.drv\n\
         \x20 /nix/store/bbbb-kernel.drv\n",
    )
    .unwrap();
    fs::write(
        folder.path().join("derivations"),
        r#"{"version": 4, "derivations": {
            "aaaa-system.drv": {
                "outputs": {"out": {"path": "aaaa-system"}},
                "inputs": {"drvs": {"bbbb-kernel.drv": {"outputs": ["out"]}}}},
            "bbbb-kernel.drv": {
                "outputs": {"out": {"path": "bbbb-kernel"}},
                "inputs": {"drvs": {}}}}}"#,
    )
    .unwrap();
    fs::write(
        folder.path().join("held"),
        r#"{"/nix/store/aaaa-system": null,
            "/nix/store/bbbb-kernel": {"narSize": 24000000, "signatures": ["here-1:A"]}}"#,
    )
    .unwrap();
    fs::write(folder.path().join("copy-takes"), "30").unwrap();
    let began = std::time::Instant::now();
    let ran = there_command(folder.path(), &offload, folder.path())
        .arg("/nix/store/aaaa-system.drv")
        .env("NIX_OFFLOAD_SEND_WITHIN", "1")
        .env("NIX_OFFLOAD_VM_DIR", "/nonexistent/nix-offload-vm")
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(
        began.elapsed() < std::time::Duration::from_secs(20),
        "the send ran to its end: {:?}",
        began.elapsed()
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains(
            "the send took longer than 1 second, slower than this machine's uplink, \
             so it stops and the builder builds what didn't go up"
        ) && !said.contains("couldn't be sent"),
        "{said}"
    );
    // The build is still asked for, there.
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link '/nix/store/aaaa-system.drv^*'\n"
    );
}

#[test]
fn what_the_builder_would_build_and_this_machine_holds_goes_up_instead() {
    // A fresh builder compiled the running system's own kernel for over an
    // hour on 3 November, though this machine held it, since no cache did.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    // It would build the system, the kernel and a tool, and fetch the rest.
    fs::write(
        folder.path().join("plans"),
        "these 3 derivations will be built:\n\
         \x20 /nix/store/aaaa-system.drv\n\
         \x20 /nix/store/bbbb-kernel.drv\n\
         \x20 /nix/store/cccc-tool.drv\n\
         this path will be fetched (1.0 MiB download, 2.0 MiB unpacked):\n\
         \x20 /nix/store/dddd-libc\n",
    )
    .unwrap();
    // The system takes the kernel's own output, not its headers.
    fs::write(
        folder.path().join("derivations"),
        r#"{"version": 4, "derivations": {
            "aaaa-system.drv": {
                "outputs": {"out": {"path": "aaaa-system"}},
                "inputs": {"drvs": {
                    "bbbb-kernel.drv": {"outputs": ["out"]},
                    "cccc-tool.drv": {"outputs": ["out"]},
                    "dddd-libc.drv": {"outputs": ["out"]}}}},
            "bbbb-kernel.drv": {
                "outputs": {"out": {"path": "bbbb-kernel"}, "dev": {"path": "bbbb-kernel-dev"}},
                "inputs": {"drvs": {}}},
            "cccc-tool.drv": {
                "outputs": {"out": {"path": "cccc-tool"}},
                "inputs": {"drvs": {}}}}}"#,
    )
    .unwrap();
    // This machine holds the kernel, though not its headers, and the
    // tool, built before this machine signed what it built, which the
    // builder's Nix, trusting no account, would refuse.
    fs::write(
        folder.path().join("held"),
        r#"{"/nix/store/aaaa-system": null,
            "/nix/store/bbbb-kernel": {"narSize": 24000000, "signatures": ["here-1:A"]},
            "/nix/store/bbbb-kernel-dev": null,
            "/nix/store/cccc-tool": {"narSize": 1000, "signatures": []}}"#,
    )
    .unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-system.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let calls = fs::read_to_string(folder.path().join("nix-calls")).unwrap();
    let sent: Vec<&str> = calls
        .lines()
        .filter(|call| call.starts_with("copy ") && !call.contains(".drv"))
        .collect();
    assert_eq!(
        sent,
        [format!(
            "copy --to {} --substitute-on-destination /nix/store/bbbb-kernel",
            STORE.trim()
        )],
        "{calls}"
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("sending 1 output of 24.0 MB this machine holds rather than building it"),
        "{said}"
    );
    // Asked once, after the derivations went up and before the build.
    let commands = fs::read_to_string(folder.path().join("ssh-commands")).unwrap();
    let commands: Vec<&str> = commands.lines().collect();
    assert_eq!(
        commands[..2],
        [
            "xargs -r nix-store --check-validity --print-invalid",
            "xargs -r nix-store --realise --dry-run 2>&1",
        ]
    );
    assert_eq!(commands.len(), 3, "{commands:?}");
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link '/nix/store/aaaa-system.drv^*'\n"
    );

    // A builder that can't say what it would build is sent nothing more.
    fs::remove_file(folder.path().join("plans")).unwrap();
    fs::remove_file(folder.path().join("nix-calls")).unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-system.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let calls = fs::read_to_string(folder.path().join("nix-calls")).unwrap();
    assert!(
        calls.lines().all(|call| !call.starts_with("copy --to")),
        "{calls}"
    );
}

#[test]
fn a_download_what_goes_up_references_is_fetched_by_the_builder_itself() {
    // A VM test's system referenced a model's 14 GB of downloads, which
    // no cache holds, and sending it went on for a day on 4 November.
    // This store recorded them as fetched by another derivation than the
    // one the builder plans, as it did once curl moved on 5 November,
    // and the same bytes are fetched there all the same.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(
        folder.path().join("plans"),
        "these 3 derivations will be built:\n\
         \x20 /nix/store/aaaa-system.drv\n\
         \x20 /nix/store/bbbb-model.drv\n\
         \x20 /nix/store/cccc-weights.drv\n",
    )
    .unwrap();
    // The weights are fixed, so they show no path among their outputs,
    // only in what their builder is told.
    fs::write(
        folder.path().join("derivations"),
        r#"{"version": 4, "derivations": {
            "aaaa-system.drv": {
                "outputs": {"out": {"path": "aaaa-system"}},
                "inputs": {"drvs": {"bbbb-model.drv": {"outputs": ["out"]}}}},
            "bbbb-model.drv": {
                "outputs": {"out": {"path": "bbbb-model"}},
                "inputs": {"drvs": {"cccc-weights.drv": {"outputs": ["out"]}}}},
            "cccc-weights.drv": {
                "outputs": {"out": {"hash": "sha256-AAAA", "method": "flat"}},
                "env": {"out": "/nix/store/dddd-weights"},
                "inputs": {"drvs": {}}}}}"#,
    )
    .unwrap();
    fs::write(
        folder.path().join("held"),
        r#"{"/nix/store/aaaa-system": {"narSize": 1000000, "signatures": ["here-1:A"]}}"#,
    )
    .unwrap();
    fs::write(
        folder.path().join("closure"),
        r#"{"/nix/store/aaaa-system": {"narSize": 1000000, "ca": null,
                                    "deriver": "/nix/store/aaaa-system.drv"},
            "/nix/store/bbbb-model": {"narSize": 5000, "ca": null,
                                   "deriver": "/nix/store/bbbb-model.drv"},
            "/nix/store/dddd-weights": {"narSize": 14000000000, "ca": "fixed:sha256:00",
                                     "deriver": "/nix/store/ffff-weights.drv"}}"#,
    )
    .unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-system.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains(
            "the builder fetches 1 download of 14.0 GB from their source itself, rather than \
             have this machine send them"
        ),
        "{said}"
    );
    assert_eq!(
        fs::read_to_string(folder.path().join("fetched")).unwrap(),
        "/nix/store/cccc-weights.drv\n"
    );
    // The model goes up with the system: the builder can't fetch it.
    assert!(
        said.contains("sending 1 output of 1.0 MB this machine holds rather than building it"),
        "{said}"
    );
    let commands = fs::read_to_string(folder.path().join("ssh-commands")).unwrap();
    let fetch = commands
        .lines()
        .position(|command| command.contains("&& echo"));
    assert_eq!(
        fetch,
        Some(2),
        "fetched after the plan, before the build: {commands}"
    );

    // A builder that can't fetch them is sent them, and says how much.
    fs::write(
        folder.path().join("refuses-fetch"),
        "/nix/store/cccc-weights.drv\n",
    )
    .unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-system.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("the builder fetched 0 of 1, so what it lacks goes up from here")
            && said.contains("sending 1 output of 14.0 GB"),
        "{said}"
    );
}

/// A Go zone's test as the builder plans it in `folder`: the test takes
/// the zone's modules, which take two downloads, and this machine holds
/// the downloads, of 12 MB, and the modules when `modules` holds their
/// size.
fn a_go_zone(folder: &Path, modules: Option<u64>) {
    fs::write(
        folder.join("plans"),
        "these 4 derivations will be built:\n\
         \x20 /nix/store/aaaa-test.drv\n\
         \x20 /nix/store/bbbb-modules.drv\n\
         \x20 /nix/store/cccc-grpc.tgz.drv\n\
         \x20 /nix/store/dddd-uuid.tgz.drv\n",
    )
    .unwrap();
    fs::write(
        folder.join("derivations"),
        r#"{"version": 4, "derivations": {
            "aaaa-test.drv": {
                "outputs": {"out": {"path": "aaaa-test"}},
                "inputs": {"drvs": {"bbbb-modules.drv": {"outputs": ["out"]}}}},
            "bbbb-modules.drv": {
                "outputs": {"out": {"path": "bbbb-modules"}},
                "inputs": {"drvs": {"cccc-grpc.tgz.drv": {"outputs": ["out"]},
                                     "dddd-uuid.tgz.drv": {"outputs": ["out"]}}}},
            "cccc-grpc.tgz.drv": {
                "outputs": {"out": {"hash": "sha256-CCCC", "method": "flat"}},
                "env": {"out": "/nix/store/cccc-grpc.tgz"}, "inputs": {"drvs": {}}},
            "dddd-uuid.tgz.drv": {
                "outputs": {"out": {"hash": "sha256-DDDD", "method": "flat"}},
                "env": {"out": "/nix/store/dddd-uuid.tgz"}, "inputs": {"drvs": {}}}}}"#,
    )
    .unwrap();
    let modules = modules.map_or("null".to_owned(), |size| {
        format!(r#"{{"narSize": {size}, "signatures": ["here-1:A"]}}"#)
    });
    fs::write(
        folder.join("held"),
        format!(
            r#"{{"/nix/store/aaaa-test": null, "/nix/store/bbbb-modules": {modules},
                "/nix/store/cccc-grpc.tgz": {{"narSize": 9000000, "ca": "fixed:sha256:00"}},
                "/nix/store/dddd-uuid.tgz": {{"narSize": 3000000, "ca": "fixed:sha256:00"}}}}"#
        ),
    )
    .unwrap();
    fs::write(
        folder.join("closure"),
        r#"{"/nix/store/cccc-grpc.tgz": {"narSize": 9000000, "ca": "fixed:sha256:00",
                                       "deriver": "/nix/store/cccc-grpc.tgz.drv"},
            "/nix/store/dddd-uuid.tgz": {"narSize": 3000000, "ca": "fixed:sha256:00",
                                       "deriver": "/nix/store/dddd-uuid.tgz.drv"}}"#,
    )
    .unwrap();
}

#[test]
fn a_download_this_machine_holds_that_the_builder_cant_fetch_goes_up() {
    // On 9 November a lent builder got 403 for a Go zone's 25 modules
    // from the proxy this machine fetched them through, and built none of
    // the zone, though this machine held them all. They're what the
    // zone's modules are built from, not what they reference.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    a_go_zone(folder.path(), None);
    let sends = |folder: &Path| -> Vec<String> {
        fs::read_to_string(folder.join("nix-calls"))
            .unwrap()
            .lines()
            .filter(|call| call.starts_with("copy --to "))
            .map(str::to_owned)
            .collect()
    };
    let send = |paths: &str| {
        format!(
            "copy --to {} --substitute-on-destination {paths}",
            STORE.trim()
        )
    };

    // The builder is asked to fetch them first, and is refused both.
    fs::write(
        folder.path().join("refuses-fetch"),
        "/nix/store/cccc-grpc.tgz.drv\n/nix/store/dddd-uuid.tgz.drv\n",
    )
    .unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-test.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("the builder fetches 2 downloads of 12.0 MB from their source itself")
            && said.contains("the builder fetched 0 of 2, so what it lacks goes up from here")
            && said.contains("sending 2 outputs of 12.0 MB this machine holds"),
        "{said}"
    );
    assert_eq!(
        sends(folder.path()),
        [send("/nix/store/cccc-grpc.tgz /nix/store/dddd-uuid.tgz")]
    );
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link '/nix/store/aaaa-test.drv^*'\n"
    );

    // Refused one, only it goes up.
    fs::write(
        folder.path().join("refuses-fetch"),
        "/nix/store/dddd-uuid.tgz.drv\n",
    )
    .unwrap();
    fs::remove_file(folder.path().join("nix-calls")).unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-test.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("sending 1 output of 3.0 MB this machine holds"),
        "{said}"
    );
    assert_eq!(sends(folder.path()), [send("/nix/store/dddd-uuid.tgz")]);

    // Fetched both, nothing goes up: a download the builder can fetch
    // never goes through the tunnel.
    fs::remove_file(folder.path().join("refuses-fetch")).unwrap();
    fs::remove_file(folder.path().join("nix-calls")).unwrap();
    let ran = there_given(folder.path(), &offload, &["/nix/store/aaaa-test.drv"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(!said.contains("this machine holds rather than"), "{said}");
    assert_eq!(sends(folder.path()), [] as [String; 0]);
    assert_eq!(
        fs::read_to_string(folder.path().join("fetched")).unwrap(),
        "/nix/store/cccc-grpc.tgz.drv\n/nix/store/dddd-uuid.tgz.drv\n"
    );
}

#[test]
fn what_a_stopped_send_leaves_the_builder_to_build_goes_up_from_what_its_built_from() {
    // On 10 November a Go zone's 106 MB of modules hadn't gone up in 15
    // minutes, and the builder, left to build them, was refused the
    // downloads they're built from, which this machine held, of 18 MB.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    a_go_zone(folder.path(), Some(106_000_000));
    fs::write(
        folder.path().join("slow-to-send"),
        "/nix/store/bbbb-modules",
    )
    .unwrap();
    fs::write(
        folder.path().join("refuses-fetch"),
        "/nix/store/cccc-grpc.tgz.drv\n/nix/store/dddd-uuid.tgz.drv\n",
    )
    .unwrap();
    // Asked after, the builder says it lacks the modules still.
    fs::write(folder.path().join("says"), "/nix/store/bbbb-modules\n").unwrap();
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(["/nix/store/aaaa-test.drv"])
        .env("NIX_OFFLOAD_SEND_WITHIN", "1")
        .env("NIX_OFFLOAD_VM_DIR", "/nonexistent/nix-offload-vm")
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("sending 1 output of 106.0 MB this machine holds")
            && said.contains("so it stops and the builder builds what didn't go up")
            && said.contains(
                "the builder lacks 1 output that didn't go up and builds it itself, so the \
                 downloads this machine holds for it go up where it can't fetch them"
            )
            && said.contains("sending 2 outputs of 12.0 MB this machine holds"),
        "{said}"
    );
    let sends: Vec<String> = fs::read_to_string(folder.path().join("nix-calls"))
        .unwrap()
        .lines()
        .filter(|call| call.starts_with("copy --to "))
        .map(str::to_owned)
        .collect();
    let send = |paths: &str| {
        format!(
            "copy --to {} --substitute-on-destination {paths}",
            STORE.trim()
        )
    };
    assert_eq!(
        sends,
        [
            send("/nix/store/bbbb-modules"),
            send("/nix/store/cccc-grpc.tgz /nix/store/dddd-uuid.tgz")
        ]
    );
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link '/nix/store/aaaa-test.drv^*'\n"
    );
}

#[test]
fn printing_out_paths_brings_them_here_from_the_builder_and_prints_them() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "/nix/store/aaaa-a\n");
    assert_eq!(
        given_to(folder.path()),
        "nix-offload\nnix build --no-link --print-out-paths -L --keep-going \
         'github:o/r/abc#checks.x.a'\n"
    );
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        format!(
            "copy --no-check-sigs --from {} /nix/store/aaaa-a\n",
            STORE.trim()
        )
    );
}

// Where the machine names the keys its lent builders sign with, what one
// built comes back only signed, so an account Nix here no longer trusts
// can still bring it, and one that does takes nothing unchecked.
#[test]
fn a_signed_builders_paths_come_back_checked() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    fs::write(offload.join("builder-keys"), "builders-1:AAAA\n").unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "/nix/store/aaaa-a\n");
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-was-given")).unwrap(),
        format!("copy --from {} /nix/store/aaaa-a\n", STORE.trim())
    );
}

// A path the builder got from here goes back signed only once its push
// signs it, on a timer, so a checked copy that fails waits for that.
#[test]
fn a_checked_copy_back_that_fails_waits_for_the_builder_to_sign() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    fs::write(offload.join("builder-keys"), "builders-1:AAAA\n").unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    fs::write(folder.path().join("nix-fails"), "1").unwrap();
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(["-L", "github:o/r/abc", "checks.x.a", "--print-out-paths"])
        .env("NIX_OFFLOAD_SIGNED_WITHIN", "1")
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "/nix/store/aaaa-a\n");
    let copy = format!("copy --from {} /nix/store/aaaa-a\n", STORE.trim());
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-calls")).unwrap(),
        copy.repeat(2)
    );
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains(
            "nix-offload: what came back may hold a path the builder signs only as it \
             pushes, so this waits 1 s for it first"
        ),
        "{ran:?}"
    );
}

// The builder's log and errors go on to the caller as they come, so a
// build that fails says why rather than only that it did.
#[test]
fn printing_out_paths_passes_on_why_a_build_failed() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 1);
    fs::write(
        folder.path().join("complains"),
        "error: builder for a failed\n",
    )
    .unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("error: builder for a failed"),
        "{ran:?}"
    );
}

#[test]
fn a_built_path_that_cant_be_brought_here_fails_and_prints_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    fs::write(folder.path().join("nix-status"), "1").unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "");
}

// A tunnel's connection can stop part way through bringing a big output
// back, and ssh gives up on it: twice running on 3 November, on a 9.5 GB
// path. Trying again keeps the paths that came whole.
#[test]
fn a_copy_back_that_breaks_off_is_tried_again_and_says_what_it_keeps() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    fs::write(folder.path().join("nix-fails"), "1").unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "/nix/store/aaaa-a\n");
    let copy = format!(
        "copy --no-check-sigs --from {} /nix/store/aaaa-a\n",
        STORE.trim()
    );
    assert_eq!(
        fs::read_to_string(folder.path().join("nix-calls")).unwrap(),
        copy.repeat(2)
    );
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains(
            "nix-offload: bringing back what was built broke off, so it's tried again \
             (2 of 3): the paths that came whole stay, and one cut off part way comes \
             again from its start"
        ),
        "{ran:?}"
    );
}

#[test]
fn a_copy_back_that_breaks_off_three_times_running_fails_and_says_so() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    stand_ins(folder.path(), 0);
    fs::write(folder.path().join("says"), "/nix/store/aaaa-a\n").unwrap();
    fs::write(folder.path().join("nix-fails"), "3").unwrap();
    let ran = there_with(folder.path(), &offload, &["--print-out-paths"]);
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert_eq!(String::from_utf8_lossy(&ran.stdout), "");
    let calls = fs::read_to_string(folder.path().join("nix-calls")).unwrap();
    assert_eq!(calls.lines().count(), 3, "{calls}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains(&format!(
            "nix-offload: couldn't bring what was built from {} into this store, \
             in 3 tries",
            STORE.trim()
        )),
        "{ran:?}"
    );
}

#[test]
fn a_folder_no_commit_names_goes_as_a_tarball_and_builds_there() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        let path = bin.join(name);
        executable(&path, script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    // A throwaway copy, as one cut from an archive is: git, but no
    // commit of origin's.
    let copy = folder.path().join("copy");
    fs::create_dir_all(copy.join(".git")).unwrap();
    fs::write(copy.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(copy.join("flake.nix"), "{ }").unwrap();
    std::os::unix::fs::symlink("/nix/store/x", copy.join("result")).unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .args(["-L", &format!("path:{}", copy.display()), "checks.x.a"])
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(!bin.join("nix-was-given").exists(), "not built here");
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    let command = given.lines().nth(1).unwrap();
    assert!(
        command.contains("tar -xzf - -C /tmp/nix-offload-there-"),
        "{command}"
    );
    assert!(
        command.contains("nix build --no-link -L 'path:/tmp/nix-offload-there-"),
        "{command}"
    );
    let listed = Command::new("tar")
        .arg("-tzf")
        .arg(bin.join("received"))
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    assert!(listed.contains("./flake.nix"), "{listed}");
    assert!(
        !listed.contains(".git") && !listed.contains("result"),
        "{listed}"
    );
}

#[test]
fn a_builder_that_stops_answering_is_given_up_on_within_a_minute() {
    // On 30 October a tunnel's connection died under a build, and ssh
    // waited on it for 30 minutes, saying nothing.
    let folder = tempfile::tempdir().unwrap();
    stand_ins(folder.path(), 0);
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let options = fs::read_to_string(folder.path().join("ssh-options")).unwrap();
    assert!(
        options.contains("ServerAliveInterval=15") && options.contains("ServerAliveCountMax=4"),
        "{options}"
    );
}

#[test]
fn a_folders_ignored_files_stay_here_and_its_size_is_said() {
    // A Rust tree's target folder held 793 MB that no build reads.
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        let path = bin.join(name);
        executable(&path, script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    let config = folder.path().join("gitconfig");
    fs::write(&config, "[user]\n  name = t\n  email = t@t\n").unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir_all(tree.join("target/debug")).unwrap();
    git(&tree, &config, &["init", "-q"]);
    fs::write(tree.join(".gitignore"), "/target\n").unwrap();
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&tree, &config, &["add", "."]);
    git(&tree, &config, &["commit", "-qm", "one"]);
    fs::write(tree.join("new.nix"), "{ }").unwrap();
    fs::write(tree.join("target/debug/big"), "built").unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .args([&format!("path:{}", tree.display()), "checks.x.a"])
        .env("GIT_CONFIG_GLOBAL", &config)
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(said.contains("as a tarball of "), "{said}");
    let listed = Command::new("tar")
        .arg("-tzf")
        .arg(bin.join("received"))
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    for kept in ["flake.nix", "new.nix", ".gitignore"] {
        assert!(listed.contains(kept), "{listed}");
    }
    assert!(!listed.contains("target"), "{listed}");
}

#[test]
fn a_folder_with_inputs_only_here_builds_here_and_shows_its_log() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let built = folder.path().join("built");
    fs::create_dir(&built).unwrap();
    fs::write(built.join("guide"), "a guide").unwrap();
    // nix as it builds here: a log line on its error, its output's path
    // on its standard output.
    let nix = format!(
        "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/nix-was-given\"\n\
         echo 'the build said this' >&2\necho {}\n",
        built.display()
    );
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", nix.as_str())] {
        let path = bin.join(name);
        executable(&path, script);
    }
    // The builder's evaluation read an input this machine alone holds.
    fs::write(bin.join("status"), "65").unwrap();
    let copy = folder.path().join("copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("flake.nix"), "{ }").unwrap();
    fs::write(
        copy.join("flake.lock"),
        r#"{"nodes": {"root": {"inputs": {"mint": "mint"}},
            "mint": {"locked": {"type": "path", "path": "/nix/store/x-mint"}},
            "secrets": {"locked": {"type": "git", "url": "file:///home/s"}},
            "nixpkgs": {"locked": {"type": "github", "owner": "o"}}}}"#,
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let out = folder.path().join("out");
    let flake = format!("path:{}", copy.display());
    // Its inputs can't go anywhere, so it builds here once the builder
    // reads one, only when asked to.
    let ran = there_command(&bin, &offload, folder.path())
        .args(["-L", "--here", "--out"])
        .arg(&out)
        .args([flake.as_str(), "packages.x.guide"])
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    assert!(given.contains("-e '/home/s'"), "{given}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("read mint, secrets, which this machine alone holds"),
        "{said}"
    );
    assert!(said.contains("the build said this"), "{said}");
    assert_eq!(fs::read_to_string(out.join("guide")).unwrap(), "a guide");

    // With nothing kept, a 65 is the build's own, and nothing is made here.
    fs::remove_file(copy.join("flake.lock")).unwrap();
    fs::remove_file(bin.join("nix-was-given")).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .args(["-L", "--here", flake.as_str(), "packages.x.guide"])
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(65), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(!said.contains("alone holds"), "{said}");
    assert!(
        !fs::read_to_string(bin.join("nix-was-given"))
            .unwrap_or_default()
            .contains("build"),
        "it was built here"
    );
}

/// No builder standing changes what a flake takes from this machine, so
/// its refusal mustn't send a caller back to wait for one, as 75 does: it
/// sent a run of 93 checks round three times on 3 November.
#[test]
fn a_folder_whose_evaluation_reads_inputs_only_here_says_how_to_send_it_and_not_to_wait() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", "#!/bin/sh\nexit 1\n")] {
        executable(&bin.join(name), script);
    }
    let copy = folder.path().join("copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("flake.nix"), "{ }").unwrap();
    fs::write(
        copy.join("flake.lock"),
        r#"{"nodes": {"mint": {"locked": {"type": "path", "path": "/nix/store/x-mint"}}}}"#,
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    fs::write(bin.join("status"), "65").unwrap();
    let flake = format!("path:{}", copy.display());
    let ran = there_command(&bin, &offload, folder.path())
        .args(["-L", flake.as_str(), "packages.x.guide"])
        .env("NIX_OFFLOAD_MORE_DIR", folder.path().join("no-second"))
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(ran.status.code(), Some(64), "{said}");
    assert!(bin.join("ssh-was-given").exists(), "sent first");
    assert!(
        said.contains("read mint, which this machine alone holds"),
        "{said}"
    );
    assert!(!said.contains("once a builder stands"), "{said}");
    assert!(
        said.contains("evaluate it here and give its derivations, or give --here"),
        "{said}"
    );
}

/// The home zone's lock names mint as a store path and the machine's
/// secrets as a repository here, and its machine reads only mint: on 10
/// November each change paid 25 minutes evaluating it here.
#[test]
fn a_folder_taking_a_store_path_sends_it_first_and_never_its_secrets() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        executable(&bin.join(name), script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    let copy = folder.path().join("copy");
    fs::create_dir(&copy).unwrap();
    fs::write(copy.join("flake.nix"), "{ }").unwrap();
    fs::write(
        copy.join("flake.lock"),
        r#"{"nodes": {
            "mint": {"locked": {"type": "path", "path": "/nix/store/x-mint"},
                     "original": {"type": "path", "path": "/nix/store/x-mint"}},
            "secrets": {"locked": {"type": "git", "url": "file:///home/me/.secrets"},
                        "original": {"type": "git", "url": "file:///home/me/.secrets"}},
            "copied": {"locked": {"type": "path", "path": "/nix/store/y-secrets"},
                       "original": {"type": "git", "url": "file:///home/me/.secrets"}}}}"#,
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let flake = format!("path:{}", copy.display());
    let ran = there_command(&bin, &offload, folder.path())
        .args(["-L", flake.as_str(), "packages.x.guide"])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&ran.stderr);
    assert_eq!(ran.status.code(), Some(0), "{said}");
    let copied = fs::read_to_string(bin.join("nix-was-given")).unwrap();
    assert!(
        copied.contains("copy --to ssh-ng://nix-offload"),
        "{copied}"
    );
    assert!(copied.contains("/nix/store/x-mint"), "{copied}");
    assert!(
        !copied.contains("y-secrets"),
        "secrets' store copy went: {copied}"
    );
    assert!(!copied.contains(".secrets"), "the secrets went: {copied}");
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    assert!(given.contains("nix-offload-kept"), "{given}");
}

#[test]
fn a_build_whose_builder_was_lost_is_sent_again_once_one_stands() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    // A lender runs, as this test does.
    fs::write(offload.join("lender"), format!("{}\n", std::process::id())).unwrap();
    stand_ins(folder.path(), 255);
    // Lost the first time, as a takeback's restart loses its tunnel.
    let ssh = folder.path().join("ssh");
    fs::write(
        &ssh,
        SSH.replace(
            "exit \"$(cat \"$here/status\")\"",
            "echo x >> \"$here/tries\"; status=$(cat \"$here/status\"); \
             echo 0 > \"$here/status\"; exit \"$status\"",
        ),
    )
    .unwrap();
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let tries = fs::read_to_string(folder.path().join("tries")).unwrap();
    assert_eq!(tries.lines().count(), 2, "sent again");
    assert!(
        !folder.path().join("nix-was-given").exists(),
        "never built here"
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("waits for a builder to stand again"),
        "{said}"
    );
    // A builder that stayed up may finish what it was building, and the
    // build sent again then only waits on it, showing none of its log.
    assert!(said.contains("nix log"), "{said}");
}

/// ssh as a nearby machine answers: it adds its last two words to a
/// record, keeps whatever it was sent, and exits as `status` says.
const NEARBY_SSH: &str = r#"#!/bin/sh
here=$(dirname "$0")
while [ "$#" -gt 2 ]; do
  if [ "$1" = -E ]; then log=$2; fi
  shift
done
printf '%s\n%s\n' "$1" "$2" >> "$here/ssh-was-given"
cat >> "$here/received"
echo "Transferred: sent 3812, received 9 bytes, in 1.0 seconds" > "$log"
exit "$(cat "$here/status")"
"#;

/// nix as it runs here: it adds its arguments to a record.
const NEARBY_NIX: &str = r#"#!/bin/sh
echo "$@" >> "$(dirname "$0")/nix-was-given"
"#;

#[test]
fn a_nearby_machine_is_sent_a_folder_flake_and_builds_its_systems() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        let path = bin.join(name);
        executable(&path, script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    let zone = folder.path().join("zone");
    fs::create_dir(&zone).unwrap();
    fs::write(zone.join("flake.nix"), "{ }").unwrap();
    fs::write(
        zone.join("flake.lock"),
        r#"{"nodes": {"mint": {"locked": {"type": "path", "path": "/nix/store/abc-mint"}}}}"#,
    )
    .unwrap();
    std::os::unix::fs::symlink("/nix/store/built", zone.join("result")).unwrap();
    // Something that answers, as the nearby machine's ssh would.
    let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let config = folder.path().join("config/nix-offload");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("nearby"),
        format!("{} mac aarch64-darwin\n", listening.local_addr().unwrap()),
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let flake = format!("path:{}", zone.display());
    let ran = there_command(&bin, &offload, folder.path())
        .args([
            "-L",
            "--here",
            &flake,
            "checks.aarch64-darwin.a",
            "checks.x86_64-linux.b",
        ])
        .env("XDG_CONFIG_HOME", folder.path().join("config"))
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");

    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    let given: Vec<&str> = given.lines().collect();
    assert_eq!(
        given.len(),
        4,
        "one ssh to send it, one to build: {given:?}"
    );
    assert_eq!((given[0], given[2]), ("mac", "mac"));
    assert!(
        given[1].contains("tar -xzf - -C .cache/nix-offload/there/"),
        "{given:?}"
    );
    assert!(
        within_far(given[3])
            .contains("nix build --no-link -L 'path:.#checks.aarch64-darwin.a'; ended=$?"),
        "{given:?}"
    );
    let sent = Command::new("tar")
        .arg("-tzf")
        .arg(bin.join("received"))
        .output()
        .unwrap();
    let sent = String::from_utf8_lossy(&sent.stdout);
    assert!(sent.lines().any(|name| name == "./flake.nix"), "{sent}");
    assert!(
        !sent.contains("result"),
        "a build's link stays here: {sent}"
    );

    assert_eq!(
        fs::read_to_string(bin.join("nix-was-given")).unwrap(),
        format!(
            "copy --no-check-sigs --to ssh-ng://mac /nix/store/abc-mint\n\
             build --no-link -L {flake}#checks.x86_64-linux.b\n"
        ),
        "the pinned input goes to it, and the rest builds here"
    );
    let journal = fs::read_to_string(folder.path().join("nix-offload/there.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines[0]["on"], "mac");
    assert_eq!(lines[0]["sent"], 2 * 3812);
    assert_eq!(lines[0]["attributes"][0], "checks.aarch64-darwin.a");
    assert!(lines[1]["on"].is_null());
}

/// ssh as a nearby machine that can't resolve names answers: it adds
/// what it was asked to a record and fails any fetch, as one whose
/// resolver is down does.
const BLIND_SSH: &str = r#"#!/bin/sh
here=$(dirname "$0")
while [ "$#" -gt 2 ]; do shift; done
printf '%s\n%s\n' "$1" "$2" >> "$here/ssh-was-given"
case "$2" in
  *"flake metadata"*)
    echo "error: Could not resolve host: github.com" >&2
    exit 1 ;;
esac
exit 0
"#;

#[test]
fn a_nearby_machine_that_cant_fetch_the_flake_builds_none_of_it() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", BLIND_SSH), ("nix", NEARBY_NIX)] {
        executable(&bin.join(name), script);
    }
    let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let config = folder.path().join("config/nix-offload");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("nearby"),
        format!("{} mac aarch64-darwin\n", listening.local_addr().unwrap()),
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .args([
            "--here",
            "github:owner/repo/abc",
            "checks.aarch64-darwin.a",
            "checks.x86_64-linux.b",
        ])
        .env("XDG_CONFIG_HOME", folder.path().join("config"))
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    assert_eq!(
        given.lines().count(),
        2,
        "asked to fetch, never to build: {given}"
    );
    assert!(
        !given.contains("nix build"),
        "nothing built where the flake can't be fetched: {given}"
    );
    assert_eq!(
        fs::read_to_string(bin.join("nix-was-given")).unwrap(),
        "build --no-link github:owner/repo/abc#checks.x86_64-linux.b \
         github:owner/repo/abc#checks.aarch64-darwin.a\n",
        "what it took goes the other way, with the rest"
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("couldn't fetch github:owner/repo/abc"),
        "{said}"
    );
}

// On 8 November a run's part for a nearby machine whose ssh then failed
// was built here, compiling biome beside Joel's work, unasked.
#[test]
fn a_part_for_a_nearby_machine_ssh_cant_reach_goes_with_the_rest() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        executable(&bin.join(name), script);
    }
    fs::write(bin.join("status"), "255").unwrap();
    let zone = folder.path().join("zone");
    fs::create_dir(&zone).unwrap();
    fs::write(zone.join("flake.nix"), "{ }").unwrap();
    fs::write(zone.join("flake.lock"), r#"{"nodes": {}}"#).unwrap();
    let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let config = folder.path().join("config/nix-offload");
    fs::create_dir_all(&config).unwrap();
    fs::write(
        config.join("nearby"),
        format!("{} mac aarch64-darwin\n", listening.local_addr().unwrap()),
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let flake = format!("path:{}", zone.display());
    let asked = |extra: &[&str]| {
        there_command(&bin, &offload, folder.path())
            .args(["-L"])
            .args(extra)
            .args([&flake, "checks.aarch64-darwin.a", "checks.x86_64-linux.b"])
            .env("XDG_CONFIG_HOME", folder.path().join("config"))
            .output()
            .unwrap()
    };
    let given = bin.join("nix-was-given");

    let ran = asked(&[]);
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(said.contains("ssh couldn't reach mac"), "{said}");
    assert!(
        said.contains("no builder is lent, so nothing was built"),
        "{said}"
    );
    assert!(!given.exists(), "nothing was built here");

    let ran = asked(&["--here"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(
        fs::read_to_string(&given).unwrap(),
        format!(
            "build --no-link -L {flake}#checks.x86_64-linux.b {flake}#checks.aarch64-darwin.a\n"
        ),
        "asked to, both are built here as one"
    );
}

#[test]
fn a_folder_sent_to_a_nearby_machine_leaves_what_its_git_ignores_here() {
    // A working tree's zone held 509 MB, almost all one ignored target.
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
        let path = bin.join(name);
        executable(&path, script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    let config = folder.path().join("gitconfig");
    fs::write(&config, "[user]\n  name = t\n  email = t@t\n").unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir_all(tree.join("target/debug")).unwrap();
    git(&tree, &config, &["init", "-q"]);
    fs::write(tree.join(".gitignore"), "/target\n").unwrap();
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&tree, &config, &["add", "."]);
    git(&tree, &config, &["commit", "-qm", "one"]);
    fs::write(tree.join("new.nix"), "{ }").unwrap();
    fs::write(tree.join("target/debug/big"), "built").unwrap();
    let listening = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let nearby = folder.path().join("config/nix-offload");
    fs::create_dir_all(&nearby).unwrap();
    fs::write(
        nearby.join("nearby"),
        format!("{} mac aarch64-darwin\n", listening.local_addr().unwrap()),
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .args([
            &format!("path:{}", tree.display()),
            "checks.aarch64-darwin.a",
        ])
        .env("XDG_CONFIG_HOME", folder.path().join("config"))
        .env("GIT_CONFIG_GLOBAL", &config)
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let listed = Command::new("tar")
        .arg("-tzf")
        .arg(bin.join("received"))
        .output()
        .unwrap();
    let listed = String::from_utf8_lossy(&listed.stdout);
    for kept in ["flake.nix", "new.nix", ".gitignore"] {
        assert!(listed.contains(kept), "{listed}");
    }
    assert!(!listed.contains("target"), "{listed}");
    assert!(!listed.contains(".git/"), "{listed}");
}

// Nearby machines are sent a tree's files as they stand, one after
// another, so one sent after the tree moved would build files no commit
// it was asked at holds.
#[test]
fn a_nearby_machine_builds_nothing_of_a_tree_that_moved_since_the_run_began() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    // The first machine's build edits the tree, once.
    let moving = NEARBY_SSH.replace(
        "cat >> \"$here/received\"",
        "case \"$2\" in *'nix build'*) [ -f \"$here/move\" ] && rm \"$here/move\" && \
         echo '# moved' >> \"$MOVES\" ;; esac\ncat >> \"$here/received\"",
    );
    for (name, script) in [("ssh", moving.as_str()), ("nix", NEARBY_NIX)] {
        executable(&bin.join(name), script);
    }
    fs::write(bin.join("status"), "0").unwrap();
    fs::write(bin.join("move"), "").unwrap();
    let config = folder.path().join("gitconfig");
    fs::write(&config, "[user]\n  name = t\n  email = t@t\n").unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir(&tree).unwrap();
    git(&tree, &config, &["init", "-q"]);
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&tree, &config, &["add", "."]);
    git(&tree, &config, &["commit", "-qm", "one"]);
    let listening = [(); 2].map(|()| std::net::TcpListener::bind("127.0.0.1:0").unwrap());
    let nearby = folder.path().join("config/nix-offload");
    fs::create_dir_all(&nearby).unwrap();
    fs::write(
        nearby.join("nearby"),
        format!(
            "{} mac aarch64-darwin\n{} intel x86_64-darwin\n",
            listening[0].local_addr().unwrap(),
            listening[1].local_addr().unwrap()
        ),
    )
    .unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let ran = there_command(&bin, &offload, folder.path())
        .arg(&tree)
        .args(["checks.aarch64-darwin.a", "checks.x86_64-darwin.a"])
        .env("MOVES", tree.join("flake.nix"))
        .env("XDG_CONFIG_HOME", folder.path().join("config"))
        .env("GIT_CONFIG_GLOBAL", &config)
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        !bin.join("move").exists(),
        "the first machine built: {said}"
    );
    assert_ne!(ran.status.code(), Some(0), "{said}");
    assert!(
        said.contains("moved on from the commit this run began at"),
        "{said}"
    );
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    assert_eq!(
        given.matches("nix build").count(),
        1,
        "only the first built: {given}"
    );
}

/// git in `tree`, with a config of the test's own, giving its answer.
fn git(tree: &Path, config: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(tree)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A tree with one commit GitHub has and one on top of it it lacks, its
/// flake edited since, with a builder lent.
struct Unpushed {
    folder: tempfile::TempDir,
    bin: PathBuf,
    config: PathBuf,
    tree: PathBuf,
    offload: PathBuf,
    pushed: String,
    head: String,
}

impl Unpushed {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir(&bin).unwrap();
        for (name, script) in [("ssh", NEARBY_SSH), ("nix", NEARBY_NIX)] {
            let path = bin.join(name);
            executable(&path, script);
        }
        fs::write(bin.join("status"), "0").unwrap();
        let config = folder.path().join("gitconfig");
        fs::write(
            &config,
            "[user]\nname = Tester\nemail = tester@example.com\n",
        )
        .unwrap();
        let tree = folder.path().join("tree");
        fs::create_dir(&tree).unwrap();
        let offload = folder.path().join("offload");
        fs::create_dir(&offload).unwrap();
        fs::write(offload.join("store"), STORE).unwrap();
        let mut made = Self {
            folder,
            bin,
            config,
            tree,
            offload,
            pushed: String::new(),
            head: String::new(),
        };
        made.git(&["init", "-q", "-b", "main"]);
        made.git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
        fs::write(made.tree.join("flake.nix"), "{ }").unwrap();
        made.git(&["add", "flake.nix"]);
        made.git(&["commit", "-q", "-m", "pushed"]);
        made.pushed = made.git(&["rev-parse", "HEAD"]);
        made.git(&["update-ref", "refs/remotes/origin/main", &made.pushed]);
        fs::write(made.tree.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
        made.git(&["commit", "-q", "-am", "not pushed"]);
        made.head = made.git(&["rev-parse", "HEAD"]);
        fs::write(made.tree.join("flake.nix"), "{ outputs = _: { x = 1; }; }").unwrap();
        made
    }

    fn git(&self, arguments: &[&str]) -> String {
        git(&self.tree, &self.config, arguments)
    }

    /// Runs `there` on the tree, and answers with what ssh was told to run.
    fn there(&self) -> String {
        let _ = fs::remove_file(self.bin.join("ssh-was-given"));
        let ran = there_command(&self.bin, &self.offload, self.folder.path())
            .args(["-L", &self.tree.to_string_lossy(), "checks.x.a"])
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        let given = fs::read_to_string(self.bin.join("ssh-was-given")).unwrap();
        given.lines().nth(1).unwrap().to_owned()
    }
}

/// The commit a command sent to the builder builds.
fn rev_in(command: &str) -> String {
    command
        .split("?rev=")
        .nth(1)
        .and_then(|rest| rest.get(..40))
        .unwrap()
        .to_owned()
}

#[test]
fn a_tree_github_lacks_goes_as_a_bundle_over_the_newest_commit_it_has() {
    let unpushed = Unpushed::new();
    let Unpushed {
        bin, pushed, head, ..
    } = &unpushed;
    let git = |arguments: &[&str]| unpushed.git(arguments);
    let command = unpushed.there();
    assert!(
        command.contains(&format!("git fetch -q https://github.com/o/r {pushed}")),
        "the builder fetches the newest commit GitHub has: {command}"
    );
    let rev = rev_in(&command);
    let rev = rev.as_str();
    assert_ne!(rev, head, "the uncommitted change is built too");
    assert!(
        git(&["cat-file", "-p", rev]).contains(&format!("parent {head}")),
        "on top of HEAD"
    );
    let heads = Command::new("git")
        .args(["bundle", "list-heads"])
        .arg(bin.join("received"))
        .current_dir(&unpushed.tree)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&heads.stdout).contains(rev),
        "the bundle carries it: {heads:?}"
    );
    assert_eq!(&git(&["rev-parse", "HEAD"]), head, "the branch stays");
    assert_eq!(
        git(&["status", "--porcelain"]),
        "M flake.nix",
        "and the change"
    );
    assert_eq!(
        git(&["for-each-ref", "refs/nix-offload"]),
        "",
        "no ref is left"
    );

    git(&["checkout", "-q", "--", "flake.nix"]);
    git(&["update-ref", "refs/remotes/origin/main", head]);
    assert_eq!(
        within_far(&unpushed.there()),
        format!("nix build --no-link -L 'github:o/r/{head}#checks.x.a'"),
        "a clean tree GitHub has goes by its commit"
    );
}

// Worktrees share their repository's refs, and on 5 November a builder
// fetched a bundle another worktree's run had pointed the one bundle ref at,
// so the commit it was asked to build wasn't there.
#[test]
fn a_run_bundles_under_a_ref_of_its_own_and_leaves_anothers_alone() {
    let unpushed = Unpushed::new();
    let Unpushed {
        bin, pushed, head, ..
    } = &unpushed;
    let git = |arguments: &[&str]| unpushed.git(arguments);
    // Another run's ref, pointed at its own commit while it bundles.
    git(&["update-ref", "refs/nix-offload/there", pushed]);
    let command = unpushed.there();
    assert_eq!(
        git(&["rev-parse", "refs/nix-offload/there"]),
        *pushed,
        "the other run's ref is where it put it"
    );
    let tip = command
        .split("git fetch -q there.bundle ")
        .nth(1)
        .and_then(|rest| rest.split(':').next())
        .unwrap();
    assert_ne!(tip, "refs/nix-offload/there", "{command}");
    let heads = Command::new("git")
        .args(["bundle", "list-heads"])
        .arg(bin.join("received"))
        .current_dir(&unpushed.tree)
        .output()
        .unwrap();
    let rev = rev_in(&command);
    assert_ne!(&rev, head);
    assert_eq!(
        String::from_utf8_lossy(&heads.stdout).trim(),
        format!("{rev} {tip}"),
        "the bundle carries its commit under the ref the builder fetches"
    );
    assert_eq!(
        git(&["for-each-ref", "--format=%(refname)", "refs/nix-offload"]),
        "refs/nix-offload/there",
        "its own ref is gone"
    );
}

// A run-subtests waited 50 minutes for a builder on 5 November while its
// branch moved on, built the new commit, and was kept as passed at the
// one it began at, which a later run there then took as done.
#[test]
fn a_tree_is_built_at_the_commit_it_was_at_when_asked_however_long_it_waits() {
    let unpushed = Unpushed::new();
    let Unpushed {
        folder,
        bin,
        config,
        tree,
        offload,
        head,
        ..
    } = &unpushed;
    let git = |arguments: &[&str]| unpushed.git(arguments);
    git(&["checkout", "-q", "--", "flake.nix"]);
    git(&["update-ref", "refs/remotes/origin/main", head]);
    // A lender runs and is lending, but no builder stands yet.
    fs::remove_file(offload.join("store")).unwrap();
    fs::write(offload.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(offload.join("lending"), "").unwrap();
    let said = folder.path().join("said");
    let mut run = there_command(bin, offload, folder.path())
        .args(["-L", &tree.to_string_lossy(), "checks.x.a"])
        .env("GIT_CONFIG_GLOBAL", config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stderr(fs::File::create(&said).unwrap())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let asking =
        || fs::read_dir(offload.join("wanted")).is_ok_and(|mut wanted| wanted.next().is_some());
    while !asking() {
        assert!(
            std::time::Instant::now() < deadline,
            "it never asked for a builder"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // The branch moves on while it waits; then a builder stands.
    fs::write(tree.join("flake.nix"), "{ outputs = _: { y = 2; }; }").unwrap();
    git(&["commit", "-q", "-am", "later"]);
    let later = git(&["rev-parse", "HEAD"]);
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = loop {
        if let Some(ran) = run.try_wait().unwrap() {
            break ran;
        }
        assert!(std::time::Instant::now() < deadline, "it never finished");
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let said = fs::read_to_string(&said).unwrap();
    assert!(ran.success(), "{said}");
    let given = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    let command = given.lines().nth(1).unwrap();
    assert!(
        !command.contains(&later),
        "it built the later commit: {command}"
    );
    assert_eq!(
        within_far(command),
        format!("nix build --no-link -L 'github:o/r/{head}#checks.x.a'"),
        "{said}"
    );
}

#[test]
fn a_tree_goes_as_its_index_holds_it_new_files_included_and_untracked_ones_left() {
    // Nix builds a tree as its index has it: a file added, or only meant
    // to be with -N, is in the build, and one git doesn't track isn't.
    let unpushed = Unpushed::new();
    let git = |arguments: &[&str]| unpushed.git(arguments);
    let tree = &unpushed.tree;
    fs::write(tree.join("added.nix"), "1").unwrap();
    git(&["add", "added.nix"]);
    fs::write(tree.join("added.nix"), "2").unwrap();
    fs::write(tree.join("meant.nix"), "3").unwrap();
    git(&["add", "-N", "meant.nix"]);
    fs::write(tree.join("loose.nix"), "4").unwrap();
    let status = git(&["status", "--porcelain"]);

    let rev = rev_in(&unpushed.there());
    for (file, holds) in [("added.nix", "2"), ("meant.nix", "3")] {
        assert_eq!(
            git(&["show", &format!("{rev}:{file}")]),
            holds,
            "{file} goes as it stands"
        );
    }
    assert!(
        !git(&["ls-tree", "-r", "--name-only", &rev]).contains("loose.nix"),
        "an untracked file stays here"
    );
    assert_eq!(
        git(&["status", "--porcelain"]),
        status,
        "the index is left as it was"
    );
}

#[test]
fn it_says_what_it_takes_a_word_a_line() {
    let said = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args(["there", "--takes"])
        .output()
        .unwrap();
    assert_eq!(said.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&said.stdout),
        concat!(
            "tree\napply\nout\nwants\nfolder\nchanged\nhere\n",
            "here-when-idle\nprint\nderivation\noutlive\nbulk\nnever-ask\nfirst\nrun\n",
            "lent-only\n"
        )
    );
}

/// The variables that name VM builders' folders, so none stands: the
/// first's in `folder`, where a test may make one, and no others.
fn no_vm_builders(folder: &Path) -> [(&'static str, PathBuf); 3] {
    [
        ("NIX_OFFLOAD_VM_DIR", folder.join("offload-vm")),
        ("NIX_OFFLOAD_VM_MORE_DIR", PathBuf::from(NO_SECOND_VM)),
        ("NIX_OFFLOAD_VM_EXTRA_DIRS", PathBuf::from(NO_MORE_VMS)),
    ]
}

#[test]
fn changed_checks_run_on_the_lent_builder_else_an_evaluator_else_nowhere() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 0);
    let config = folder.path().join("gitconfig");
    fs::write(
        &config,
        "[user]\nname = Tester\nemail = tester@example.com\n",
    )
    .unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir(&tree).unwrap();
    let git = |arguments: &[&str]| git(&tree, &config, arguments);
    git(&["init", "-q", "-b", "main"]);
    git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&["add", "flake.nix"]);
    git(&["commit", "-q", "-m", "pushed"]);
    let head = git(&["rev-parse", "HEAD"]);
    git(&["update-ref", "refs/remotes/origin/main", &head]);
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let run = || {
        there_command(&bin, &offload, folder.path())
            .args(["--changed", "--build", "--no-vm"])
            .args(["--system", "x86_64-linux"])
            .current_dir(&tree)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap()
    };
    let given = || fs::read_to_string(bin.join("ssh-was-given")).unwrap();

    // The lent builder evaluates and builds, in one place.
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = given();
    assert_eq!(sent.lines().next(), Some("nix-offload"), "{sent}");
    within_far(sent.lines().nth(1).unwrap());
    for part in [
        format!("checkout -q -B nix-offload-changed {head}"),
        "nix run .#changed-checks -- --build --no-vm --system x86_64-linux".to_owned(),
    ] {
        assert!(sent.contains(&part), "{part} in {sent}");
    }
    // That run passed, and is kept, so the same run is asked anew only
    // once it's forgotten.
    fs::remove_dir_all(folder.path().join("nix-offload/passed")).unwrap();

    // With none lent, an evaluator that answers only evaluates, and what it
    // names is built as any `there` build is: nowhere, with no builder, and
    // never handed back to run here whole.
    fs::remove_file(offload.join("store")).unwrap();
    let answering = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = answering.local_addr().unwrap();
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    let evaluators = folder.path().join("nix-offload/evaluators");
    fs::write(bin.join("says"), "changed  a\nadded    vm-b\nremoved  c\n").unwrap();
    // One that takes other owners' repositories alone is never sent this.
    fs::write(&evaluators, format!("{address} far others\n")).unwrap();
    assert_eq!(run().status.code(), Some(75), "it isn't sent there");
    fs::write(&evaluators, format!("{address} far others,o\n")).unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(69), "{ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("no builder took their build"),
        "{ran:?}"
    );
    let sent = given();
    assert_eq!(sent.lines().next(), Some("far"), "{sent}");
    within_far(sent.lines().nth(1).unwrap());
    assert!(
        sent.contains("nix run .#changed-checks -- --no-vm --system x86_64-linux --name-only"),
        "it evaluates and doesn't build: {sent}"
    );
    assert!(
        String::from_utf8_lossy(&ran.stdout).contains("changed  a"),
        "what it named comes here: {ran:?}"
    );
    assert!(
        !bin.join("nix-was-given").exists(),
        "nothing was built here"
    );

    // A commit's changed-checks from before --name-only refuses it, and
    // without --build it only names anyway, so it's asked once without.
    fs::write(bin.join("refuses-name-only"), "").unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(69), "{ran:?}");
    let sent = given();
    assert!(
        sent.contains("nix run .#changed-checks -- --no-vm --system x86_64-linux")
            && !sent.contains("--name-only"),
        "asked again without it: {sent}"
    );
    assert!(String::from_utf8_lossy(&ran.stdout).contains("changed  a"));
    fs::remove_file(bin.join("refuses-name-only")).unwrap();

    // Nowhere answers, so changed-checks is told to run here.
    drop(answering);
    let ran = run();
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
}

/// A tree of one commit GitHub has, with a builder lent, for
/// `there --changed`, its stand-ins in `bin`.
struct Changed {
    folder: tempfile::TempDir,
    bin: PathBuf,
    config: PathBuf,
    tree: PathBuf,
    offload: PathBuf,
    head: String,
}

impl Changed {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir(&bin).unwrap();
        stand_ins(&bin, 0);
        let config = folder.path().join("gitconfig");
        fs::write(
            &config,
            "[user]\nname = Tester\nemail = tester@example.com\n",
        )
        .unwrap();
        let tree = folder.path().join("tree");
        fs::create_dir(&tree).unwrap();
        let git = |arguments: &[&str]| git(&tree, &config, arguments);
        git(&["init", "-q", "-b", "main"]);
        git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
        fs::write(tree.join("flake.nix"), "{ }").unwrap();
        git(&["add", "flake.nix"]);
        git(&["commit", "-q", "-m", "pushed"]);
        let head = git(&["rev-parse", "HEAD"]);
        git(&["update-ref", "refs/remotes/origin/main", &head]);
        let offload = folder.path().join("offload");
        fs::create_dir(&offload).unwrap();
        fs::write(offload.join("store"), STORE).unwrap();
        Self {
            folder,
            bin,
            config,
            tree,
            offload,
            head,
        }
    }

    /// `there --changed` with these arguments, and what ssh was told to
    /// run, if anything.
    fn run(&self, arguments: &[&str]) -> (Output, Option<String>) {
        self.run_with(&[], arguments, Path::new(NO_SECOND))
    }

    /// `run`, with `before` given ahead of `--changed`.
    fn run_after(&self, before: &[&str], arguments: &[&str]) -> (Output, Option<String>) {
        self.run_with(before, arguments, Path::new(NO_SECOND))
    }

    /// A second lender beside the first, standing, with a builder of its
    /// own, `nix-offload-more`.
    fn second(&self) -> PathBuf {
        let more = self.folder.path().join("offload-more");
        fs::create_dir_all(&more).unwrap();
        fs::write(more.join("lender"), std::process::id().to_string()).unwrap();
        fs::write(
            more.join("store"),
            STORE.replace("nix-offload", "nix-offload-more"),
        )
        .unwrap();
        more
    }

    /// `run`, with the second lender `more`'s folder names.
    fn run_beside(&self, arguments: &[&str], more: &Path) -> (Output, Option<String>) {
        self.run_with(&[], arguments, more)
    }

    /// `there`, given `before`, then `--changed` and `arguments`, with the
    /// second lender `more`'s folder names.
    fn run_with(
        &self,
        before: &[&str],
        arguments: &[&str],
        more: &Path,
    ) -> (Output, Option<String>) {
        let given = self.bin.join("ssh-was-given");
        let _ = fs::remove_file(&given);
        let ran = there_command(&self.bin, &self.offload, self.folder.path())
            .args(before)
            .arg("--changed")
            .args(arguments)
            .env("NIX_OFFLOAD_EVALUATING_LONGEST", "5")
            .current_dir(&self.tree)
            .env("NIX_OFFLOAD_MORE_DIR", more)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap();
        let sent = fs::read_to_string(&given)
            .ok()
            .map(|given| given.lines().nth(1).unwrap().to_owned());
        (ran, sent)
    }

    /// Each host ssh was given, with whether its command was marked to
    /// move on rather than wait, in the order they were asked.
    fn asked(&self) -> Vec<(String, bool)> {
        fs::read_to_string(self.bin.join("ssh-sent"))
            .unwrap()
            .lines()
            .map(|line| {
                let (host, command) = line.split_once(' ').unwrap();
                (host.to_owned(), command.contains(": NIX_OFFLOAD_MOVES=1; "))
            })
            .collect()
    }
}

#[test]
fn a_changed_run_a_busy_lent_builder_turns_away_goes_to_the_other() {
    // On 8 November runs queued at one lent builder for an hour while the
    // other stood idle, since each run takes the one its turn names.
    let changed = Changed::new();
    let more = changed.second();
    fs::write(changed.bin.join("busy-nix-offload"), "").unwrap();
    fs::write(changed.bin.join("says"), "changed  a\n").unwrap();
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let asked = changed.asked();
    assert_eq!(
        asked.last(),
        Some(&("nix-offload-more".to_owned(), true)),
        "{asked:?}"
    );
    assert!(asked.iter().all(|(_, marked)| *marked), "{asked:?}");
    assert!(String::from_utf8_lossy(&ran.stdout).contains("changed  a"));
}

#[test]
fn a_changed_run_both_lent_builders_turn_away_waits_at_the_first_it_asked() {
    let changed = Changed::new();
    let more = changed.second();
    fs::write(changed.bin.join("busy-nix-offload"), "").unwrap();
    fs::write(changed.bin.join("busy-nix-offload-more"), "").unwrap();
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let asked = changed.asked();
    assert_eq!(asked.len(), 3, "{asked:?}");
    assert_ne!(asked[0].0, asked[1].0, "{asked:?}");
    assert_eq!(
        asked[2],
        (asked[0].0.clone(), false),
        "waits at the first, unmarked: {asked:?}"
    );
    assert!(asked[0].1 && asked[1].1, "{asked:?}");
}

#[test]
fn a_changed_run_asks_first_the_lent_builder_keeping_its_base_evaluated() {
    // keep-main keeps each head of main on one lent builder, and on 12
    // November 23 of 54 runs evaluated their base again on the other.
    let changed = Changed::new();
    let more = changed.second();
    for host in ["nix-offload", "nix-offload-more"] {
        fs::write(changed.bin.join(format!("kept-{host}")), &changed.head).unwrap();
        // Each run takes the two in turn by its process, so runs go on
        // until one whose turn named the other was sent here instead. A
        // run that passed at the commit is only said again, so each
        // forgets it.
        let mut runs = 0;
        let mut moved = false;
        while runs < 24 && !moved {
            let _ = fs::remove_file(changed.bin.join("ssh-sent"));
            let _ = fs::remove_dir_all(changed.folder.path().join("nix-offload/passed"));
            let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
            assert_eq!(ran.status.code(), Some(0), "{ran:?}");
            let asked = changed.asked();
            assert_eq!(asked.len(), 1, "went beyond the first: {asked:?}");
            assert_eq!(
                asked[0].0, host,
                "didn't go where its base is kept: {asked:?}"
            );
            runs += 1;
            moved = String::from_utf8_lossy(&ran.stderr).contains("so this run asks it first");
        }
        assert!(moved, "no run's turn named the other in {runs} runs");
        fs::remove_file(changed.bin.join(format!("kept-{host}"))).unwrap();
        let kept_asked = fs::read_to_string(changed.bin.join("asked-kept")).unwrap_or_default();
        assert_eq!(
            kept_asked.lines().count(),
            2 * runs,
            "each run asks both: {kept_asked}"
        );
        fs::remove_file(changed.bin.join("asked-kept")).unwrap();
    }
}

#[test]
fn a_changed_run_whose_keeping_builder_is_busy_goes_to_the_other() {
    let changed = Changed::new();
    let more = changed.second();
    fs::write(changed.bin.join("kept-nix-offload-more"), &changed.head).unwrap();
    fs::write(changed.bin.join("busy-nix-offload-more"), "").unwrap();
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let asked = changed.asked();
    assert_eq!(
        asked,
        [
            ("nix-offload-more".to_owned(), true),
            ("nix-offload".to_owned(), true)
        ],
        "didn't ask the keeper first and then the other: {asked:?}"
    );
}

#[test]
fn a_changed_run_gives_the_other_lent_builder_the_checks_its_base_kept() {
    // From 11:00 to 15:00 on 12 November 78 runs evaluated a base, of only
    // 46 bases: 32 were the other builder evaluating one again.
    let changed = Changed::new();
    let more = changed.second();
    fs::write(changed.bin.join("kept-nix-offload-more"), &changed.head).unwrap();
    fs::write(
        changed.bin.join("kept-checks-nix-offload-more"),
        "{\"a\":\"x\"}",
    )
    .unwrap();
    fs::write(
        changed.bin.join("kept-reads-nix-offload-more"),
        "{\"read\":[]}",
    )
    .unwrap();
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(
        fs::read_to_string(changed.bin.join("took-kept-nix-offload"))
            .ok()
            .as_deref(),
        Some("{\"a\":\"x\"}{\"read\":[]}"),
        "the other builder wasn't given what the keeper kept, checks first"
    );
    assert!(
        !changed.bin.join("took-kept-nix-offload-more").exists(),
        "the keeper was given its own checks back"
    );
    let asked = changed.asked();
    assert_eq!(asked.len(), 1, "the copy was sent as a run: {asked:?}");
}

#[test]
fn a_changed_run_with_one_lent_builder_asks_nothing_of_kept_bases() {
    let changed = Changed::new();
    fs::write(changed.bin.join("kept-nix-offload"), &changed.head).unwrap();
    let (ran, _) = changed.run(&["--name-only", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(
        !changed.bin.join("asked-kept").exists(),
        "asked with nowhere else to go"
    );
}

/// A warm VM builder standing in `changed`'s folder, as its lender names it.
fn warm_vm(changed: &Changed) {
    let vm = changed.folder.path().join("offload-vm");
    fs::create_dir_all(&vm).unwrap();
    fs::write(vm.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(
        vm.join("store"),
        STORE.replace("nix-offload", "nix-offload-vm"),
    )
    .unwrap();
}

#[test]
fn a_changed_run_both_lent_builders_turn_away_goes_to_a_warm_vm_builder() {
    // On 10 November evaluations took 4.7 minutes with three or fewer on
    // a lent builder and 7.5 with seven or more, while warm VM builders
    // stood idle beside them.
    let changed = Changed::new();
    let more = changed.second();
    warm_vm(&changed);
    fs::write(changed.bin.join("busy-nix-offload"), "").unwrap();
    fs::write(changed.bin.join("busy-nix-offload-more"), "").unwrap();
    fs::write(changed.bin.join("says"), "changed  a\n").unwrap();
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let asked = changed.asked();
    assert_eq!(
        asked.last(),
        Some(&("nix-offload-vm".to_owned(), true)),
        "{asked:?}"
    );
    assert_eq!(asked.len(), 3, "{asked:?}");
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(said.contains("changed  a"), "what it said: {said}");
}

#[test]
fn a_changed_run_every_builder_turns_away_waits_at_the_first_lent_one() {
    let changed = Changed::new();
    let more = changed.second();
    warm_vm(&changed);
    for host in ["nix-offload", "nix-offload-more", "nix-offload-vm"] {
        fs::write(changed.bin.join(format!("busy-{host}")), "").unwrap();
    }
    let (ran, _) = changed.run_beside(&["--name-only", "--system", "x86_64-linux"], &more);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let asked = changed.asked();
    assert_eq!(asked.len(), 4, "{asked:?}");
    assert_eq!(asked[2], ("nix-offload-vm".to_owned(), true), "{asked:?}");
    assert_eq!(
        asked[3],
        (asked[0].0.clone(), false),
        "waits at the first lent builder, unmarked: {asked:?}"
    );
    assert_ne!(asked[0].0, "nix-offload-vm", "{asked:?}");
}

#[test]
fn a_changed_run_with_one_lent_builder_waits_there_unmarked() {
    // With nowhere else to go, a run turned away would only come back and
    // send its tree again.
    let changed = Changed::new();
    fs::write(changed.bin.join("busy-nix-offload"), "").unwrap();
    let (ran, sent) = changed.run(&["--name-only", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(changed.asked(), [("nix-offload".to_owned(), false)]);
    assert!(!sent.unwrap().contains("NIX_OFFLOAD_MOVES"));
}

#[test]
fn a_changed_run_sent_first_tells_the_builders_gate_so() {
    // As the net's runs are, so the gate lets it past the sessions' runs
    // waiting for a turn; a run not sent first isn't marked.
    let changed = Changed::new();
    let (ran, sent) = changed.run_after(&["--first"], &["--no-vm", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = sent.unwrap();
    assert!(
        sent.contains("NIX_OFFLOAD_FIRST=1 nix run .#changed-checks"),
        "{sent}"
    );
    let _ = fs::remove_dir_all(changed.folder.path().join("nix-offload/passed"));
    let (ran, sent) = changed.run(&["--no-vm", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(!sent.unwrap().contains("NIX_OFFLOAD_FIRST"));
}

#[test]
fn a_changed_run_kept_to_the_lent_builders_runs_this_machines_changed_checks() {
    // A fenced session writes its commits, so the changed-checks a commit
    // names would run what the session put there, on the builder and as
    // the person, and say what passed (12 November).
    let Some(own) = option_env!("NIX_OFFLOAD_CHANGED_CHECKS") else {
        panic!("built without a changed-checks of its own, which package.nix gives it");
    };
    let package = own
        .strip_suffix("/bin/changed-checks")
        .expect("a program in a package's bin");
    let changed = Changed::new();
    fs::write(changed.bin.join("says"), "changed  a\n").unwrap();
    let arguments = ["--build", "--no-vm", "--system", "x86_64-linux"];
    let (ran, sent) = changed.run_after(&["--lent-only"], &arguments);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = sent.expect("it's sent");
    assert!(
        sent.contains(&format!("{own} --build --no-vm --system x86_64-linux")),
        "run by its store path: {sent}"
    );
    assert!(
        !sent.contains("nix run"),
        "nothing the commit names runs: {sent}"
    );
    let calls = fs::read_to_string(changed.bin.join("nix-calls")).unwrap();
    assert!(
        calls.lines().any(|call| {
            call.starts_with("copy --to ssh-ng://nix-offload?")
                && call.ends_with(&format!(" {package}"))
        }),
        "its package goes up to the builder: {calls}"
    );
    // What's kept as passed names the commit it was sent and the run as
    // it was asked, and the far side only what it named.
    let kept = fs::read_to_string(
        changed
            .folder
            .path()
            .join("nix-offload/passed")
            .join(&changed.head),
    )
    .unwrap();
    let run: serde_json::Value = kept
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|line| line.get("run").is_some())
        .unwrap_or_else(|| panic!("no run kept: {kept}"));
    assert_eq!(run["run"], serde_json::json!(arguments), "{kept}");
    assert_eq!(run["printed"], "changed  a\n", "{kept}");
}

#[test]
fn changed_checks_go_to_a_vm_builder_that_stands_while_no_lent_builder_answers() {
    // From 01:00 to 08:05 on 8 November the lent builders' disks stalled
    // them, and every `there --changed` waited on them and ended 69,
    // while the VM builders took every VM test sent them by name.
    let changed = Changed::new();
    fs::write(
        changed.offload.join("lender"),
        std::process::id().to_string(),
    )
    .unwrap();
    fs::write(changed.bin.join("unreached-nix-offload"), "").unwrap();
    let vm = changed.folder.path().join("offload-vm");
    fs::create_dir(&vm).unwrap();
    fs::write(vm.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(
        vm.join("store"),
        STORE.replace("nix-offload", "nix-offload-vm"),
    )
    .unwrap();
    fs::write(changed.bin.join("says"), "changed  a\n").unwrap();
    let began = std::time::Instant::now();
    let (ran, sent) = changed.run(&["--name-only", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(
        began.elapsed() < std::time::Duration::from_secs(60),
        "sent on at once, not after waiting for a lent builder: {:?}",
        began.elapsed()
    );
    let hosts = fs::read_to_string(changed.bin.join("ssh-hosts")).unwrap();
    assert_eq!(hosts.lines().last(), Some("nix-offload-vm"), "{hosts}");
    let sent = sent.unwrap();
    assert!(sent.contains("changed-checks -- --name-only"), "{sent}");
    assert!(String::from_utf8_lossy(&ran.stdout).contains("changed  a"));
}

#[test]
fn an_evaluator_that_names_nothing_is_left_on_its_clock() {
    // On 8 November six runs sat on one evaluator for up to three hours,
    // having printed nothing. With no other, it's as though none answered.
    let changed = Changed::new();
    fs::remove_file(changed.offload.join("store")).unwrap();
    let answering = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = answering.local_addr().unwrap();
    fs::create_dir_all(changed.folder.path().join("nix-offload")).unwrap();
    fs::write(
        changed.folder.path().join("nix-offload/evaluators"),
        format!("{address} far o\n"),
    )
    .unwrap();
    fs::write(changed.bin.join("evaluates-forever"), "").unwrap();
    let asked = std::time::Instant::now();
    let (ran, sent) = changed.run(&["--no-vm"]);
    assert!(
        asked.elapsed() < std::time::Duration::from_secs(60),
        "it waited {:?}",
        asked.elapsed()
    );
    assert!(
        sent.is_some_and(|sent| sent.contains("--name-only")),
        "{ran:?}"
    );
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("far named nothing within 5 seconds"),
        "{ran:?}"
    );
}

#[test]
fn an_evaluator_evaluates_one_run_at_a_time() {
    // On 8 November six runs sat on one evaluator at once for up to three
    // hours, its Nix saying its database was busy.
    let changed = Changed::new();
    fs::remove_file(changed.offload.join("store")).unwrap();
    let answering = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = answering.local_addr().unwrap();
    fs::create_dir_all(changed.folder.path().join("nix-offload")).unwrap();
    fs::write(
        changed.folder.path().join("nix-offload/evaluators"),
        format!("{address} far o\n"),
    )
    .unwrap();
    fs::write(changed.bin.join("says"), "changed  a\n").unwrap();
    fs::write(changed.bin.join("evaluates-for"), "2").unwrap();
    let ran: Vec<Output> = std::thread::scope(|scope| {
        let runs: Vec<_> = (0..2)
            .map(|_| scope.spawn(|| changed.run(&["--name-only"]).0))
            .collect();
        runs.into_iter().map(|run| run.join().unwrap()).collect()
    });
    for run in &ran {
        assert_eq!(run.status.code(), Some(0), "{run:?}");
        assert!(
            String::from_utf8_lossy(&run.stdout).contains("changed  a"),
            "each is evaluated: {run:?}"
        );
    }
    let at_once = fs::read_to_string(changed.bin.join("evaluating-at-once-far")).unwrap();
    assert_eq!(at_once, "1\n1\n", "the second waited for the first");
    assert!(
        ran.iter()
            .any(|run| String::from_utf8_lossy(&run.stderr).contains("waits its turn")),
        "the one waiting says so: {ran:?}"
    );
}

#[test]
fn what_changed_checks_keeps_of_a_run_there_comes_home_once_and_never_fails_it() {
    // A builder goes when its lend ends, taking changed-checks' own record
    // of where each run's time went, which on 12 November could be read
    // only by hand from one still lent.
    let changed = Changed::new();
    let far = changed.bin.join("far-runs");
    fs::write(&far, "{\"at\":1}\n").unwrap();
    fs::write(changed.bin.join("far-runs-grow"), "").unwrap();
    let home = changed.folder.path().join("nix-offload/changed-runs.jsonl");
    let far_lines = || -> Vec<String> {
        fs::read_to_string(&far)
            .unwrap()
            .lines()
            .map(|line| format!("{line}\n"))
            .collect()
    };

    let (ran, _) = changed.run(&["--build", "--no-vm", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let lines = far_lines();
    assert_eq!(lines.len(), 2, "the run kept a line there: {lines:?}");
    assert_eq!(
        fs::read_to_string(&home).unwrap_or_default(),
        lines[1],
        "only the run's own line comes home, never one from before it"
    );

    // The builder kept for the next run sends only that run's line.
    let (ran, _) = changed.run(&["--build", "--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let lines = far_lines();
    assert_eq!(
        fs::read_to_string(&home).unwrap_or_default(),
        lines[1..].concat(),
        "each run's line is home once"
    );

    // A copy that fails is said, and the run passes as it would have.
    fs::write(changed.bin.join("far-runs-unreadable"), "").unwrap();
    let (ran, _) = changed.run(&["--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("couldn't bring changed-checks' runs home from nix-offload"),
        "{said}"
    );
    assert_eq!(
        fs::read_to_string(&home).unwrap_or_default(),
        lines[1..].concat(),
        "nothing more came home"
    );
}

#[test]
fn a_changed_checks_run_that_passed_at_a_commit_isnt_run_again() {
    // A second run on a commit whose run passed, sent to a builder lent
    // since, evaluated both sides again, 400 to 900 s, to build nothing new.
    let changed = Changed::new();
    let head = &changed.head;
    let short = &head[..12];
    fs::write(
        changed.bin.join("says"),
        "changed  a\nadded    vm-b\nremoved  c\n",
    )
    .unwrap();
    let arguments = ["--build", "--no-vm", "--system", "x86_64-linux"];

    let (ran, sent) = changed.run(&arguments);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = sent.expect("the first is sent");
    assert!(
        !sent.contains("CHANGED_CHECKS_PASSED"),
        "nothing passed yet: {sent}"
    );
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "changed  a\nadded    vm-b\nremoved  c\n",
        "what it names comes here as it's said"
    );

    let (ran, sent) = changed.run(&arguments);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(sent, None, "nothing is sent again");
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "changed  a\nadded    vm-b\nremoved  c\n",
        "what it named is said again"
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains(&format!(
            "passed at {short} already, so nothing is evaluated or built"
        )),
        "{said}"
    );

    // A run asking for more is sent, told what passed, so it builds only
    // what hasn't; and one that fails is never kept.
    fs::write(changed.bin.join("status"), "1").unwrap();
    for _ in 0..2 {
        let (ran, sent) = changed.run(&["--build", "--system", "x86_64-linux"]);
        assert_eq!(ran.status.code(), Some(1), "{ran:?}");
        let sent = sent.expect("it's sent");
        assert!(
            sent.contains(&format!("CHANGED_CHECKS_PASSED='{head} a' ")),
            "{sent}"
        );
    }
}

#[test]
fn subtests_that_passed_at_a_commit_arent_run_again_and_their_pictures_come_from_the_last() {
    // run-subtests sent 260 runs in three days, 139 of them passing, at a
    // median of 477 s, and the same words at the same commit ran again.
    let changed = Changed::new();
    let short = &changed.head[..12];
    let built = changed.folder.path().join("built");
    fs::create_dir(&built).unwrap();
    fs::write(built.join("shot.png"), "a picture").unwrap();
    // nix as it builds here: each call noted, its output's path printed.
    let nix = format!(
        "#!/bin/sh\necho \"$@\" >> \"$(dirname \"$0\")/nix-calls\"\necho {}\n",
        built.display()
    );
    executable(&changed.bin.join("nix"), &nix);
    // No builder lent, so it builds here, as asked.
    let nowhere = changed.folder.path().join("nowhere");
    fs::create_dir(&nowhere).unwrap();
    let subtests = |words: &str, out: &str| {
        there_command(&changed.bin, &nowhere, changed.folder.path())
            .args(["--here", "--out"])
            .arg(changed.folder.path().join(out))
            .args([
                "--apply",
                &format!("subtests: subtests {{ words = [ {words} ]; }}"),
            ])
            .arg(&changed.tree)
            .arg("legacyPackages.x.subtests")
            .output()
            .unwrap()
    };
    let builds = || {
        fs::read_to_string(changed.bin.join("nix-calls"))
            .unwrap_or_default()
            .lines()
            .filter(|call| call.starts_with("build "))
            .count()
    };
    let picture = |out: &str| fs::read_to_string(changed.folder.path().join(out).join("shot.png"));

    let ran = subtests("\"a\"", "first");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(builds(), 1);
    assert_eq!(picture("first").unwrap(), "a picture");

    let ran = subtests("\"a\"", "second");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(builds(), 1, "the same subtests aren't run again");
    assert_eq!(
        picture("second").unwrap(),
        "a picture",
        "their pictures come"
    );
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains(&format!("passed at {short} already")),
        "{said}"
    );
    assert!(
        said.contains("first"),
        "it says where they came from: {said}"
    );
    assert_eq!(journalled(changed.folder.path())["already"], true);

    let ran = subtests("\"b\"", "third");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(builds(), 2, "other words are run");

    // With the pictures gone, nothing shows what they were, so they're
    // run again; and so is a tree with a change no commit names.
    fs::remove_dir_all(changed.folder.path().join("first")).unwrap();
    let ran = subtests("\"a\"", "fourth");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(builds(), 3, "run again once their pictures are gone");
    fs::write(changed.tree.join("flake.nix"), "{ outputs = _: { }; }").unwrap();
    let ran = subtests("\"a\"", "fifth");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(builds(), 4, "a change is run");
}

/// An output unpacked with the store's modes leaves the folder writable, so
/// the next run can unpack into it: on 6 November a run that took no
/// pictures left its folder read-only, and the next into it passed its
/// subtests and failed to unpack, its pictures lost.
#[test]
fn a_folder_an_output_was_unpacked_into_takes_the_next() {
    let changed = Changed::new();
    let built = changed.folder.path().join("built");
    let shots = built.join("shots");
    fs::create_dir_all(&shots).unwrap();
    fs::write(shots.join("shot.png"), "a picture").unwrap();
    let mode = |path: &Path, mode| fs::set_permissions(path, fs::Permissions::from_mode(mode));
    // As the store keeps it, nothing in it writable, and as the lent
    // builder sends it back: a tarball on the build's output.
    mode(&shots.join("shot.png"), 0o444).unwrap();
    mode(&shots, 0o555).unwrap();
    mode(&built, 0o555).unwrap();
    let packed = Command::new("tar")
        .arg("-czf")
        .arg(changed.bin.join("says"))
        .arg("-C")
        .arg(&built)
        .arg(".")
        .status()
        .unwrap();
    assert!(packed.success());
    for path in [&built, &shots] {
        mode(path, 0o755).unwrap();
    }
    let out = changed.folder.path().join("out");
    let subtests = |words: &str| {
        there_command(&changed.bin, &changed.offload, changed.folder.path())
            .args(["--out"])
            .arg(&out)
            .args([
                "--apply",
                &format!("subtests: subtests {{ words = [ {words} ]; }}"),
            ])
            .arg(&changed.tree)
            .arg("legacyPackages.x.subtests")
            .output()
            .unwrap()
    };

    let ran = subtests("\"a\"");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let ran = subtests("\"b\"");
    let unpacked = fs::read_to_string(out.join("shots").join("shot.png"));
    let unwritable: Vec<PathBuf> = [out.clone(), out.join("shots"), out.join("shots/shot.png")]
        .into_iter()
        .filter(|path| fs::metadata(path).is_ok_and(|held| held.permissions().mode() & 0o200 == 0))
        .collect();
    // Writable again whatever happened, so the temporary folder can go.
    let _ = Command::new("chmod")
        .arg("-R")
        .arg("u+w")
        .arg(&out)
        .status();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(unpacked.unwrap(), "a picture");
    assert!(unwritable.is_empty(), "left unwritable: {unwritable:?}");
}

/// A run whose output can't be unpacked here has failed, whatever its
/// build said, and isn't kept as passed: on 4 November a gallery's run
/// exited 0 with its pictures unpacked nowhere, and the next at that
/// commit ran nothing and copied the last folder's stale pictures.
#[test]
fn a_run_whose_output_cant_be_unpacked_fails_and_isnt_kept_as_passed() {
    // The lent builder's build passes, and sends back no tarball.
    let changed = Changed::new();
    let subtests = |out: &str| {
        there_command(&changed.bin, &changed.offload, changed.folder.path())
            .args(["--out"])
            .arg(changed.folder.path().join(out))
            .args(["--apply", "subtests: subtests { words = [ \"a\" ]; }"])
            .arg(&changed.tree)
            .arg("legacyPackages.x.subtests")
            .output()
            .unwrap()
    };
    let sent = || {
        fs::read_to_string(changed.bin.join("ssh-commands"))
            .unwrap_or_default()
            .lines()
            .filter(|command| command.contains("tar -czf"))
            .count()
    };

    let ran = subtests("first");
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(said.contains("couldn't unpack the output into"), "{said}");
    assert_eq!(sent(), 1);

    let ran = subtests("second");
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert_eq!(sent(), 2, "nothing was kept as passed, so it's sent again");
}

#[test]
fn changed_checks_on_the_lent_builder_send_their_vm_tests_to_a_builder_of_their_own() {
    // On 2 November nine VM tests of runs sent whole to the lent builder
    // shared it with big builds, at a load of 100 on its 32 cores, while a
    // builder for VM tests stood apart.
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 0);
    let config = folder.path().join("gitconfig");
    fs::write(
        &config,
        "[user]\nname = Tester\nemail = tester@example.com\n",
    )
    .unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir(&tree).unwrap();
    let git = |arguments: &[&str]| git(&tree, &config, arguments);
    git(&["init", "-q", "-b", "main"]);
    git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&["add", "flake.nix"]);
    git(&["commit", "-q", "-m", "pushed"]);
    let head = git(&["rev-parse", "HEAD"]);
    git(&["update-ref", "refs/remotes/origin/main", &head]);
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    // A lender for VM tests runs, as this test does, with its builder lent.
    let vm = folder.path().join("offload-vm");
    fs::create_dir(&vm).unwrap();
    fs::write(vm.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(
        vm.join("store"),
        STORE.replace("nix-offload", "nix-offload-vm"),
    )
    .unwrap();
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    fs::write(
        folder.path().join("nix-offload/vm-tests"),
        "checks.*.vm-*\n",
    )
    .unwrap();
    fs::write(bin.join("says"), "changed  a\nadded    vm-b\nremoved  c\n").unwrap();

    let ran = there_command(&bin, &offload, folder.path())
        .args(["--changed", "--build", "--system", "x86_64-linux"])
        .current_dir(&tree)
        .env("NIX_OFFLOAD_VM_DIR", &vm)
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let commands = fs::read_to_string(bin.join("ssh-commands")).unwrap();
    let hosts = fs::read_to_string(bin.join("ssh-hosts")).unwrap();
    let sent: Vec<(&str, &str)> = hosts.lines().zip(commands.lines()).collect();
    let asked = |host: &str, part: &str| {
        sent.iter()
            .any(|(to, command)| *to == host && command.contains(part))
    };
    assert!(
        asked(
            "nix-offload",
            "changed-checks -- --system x86_64-linux --name-only"
        ),
        "the lent builder names what changed first: {sent:?}"
    );
    assert!(
        asked(
            "nix-offload",
            "changed-checks -- --build --system x86_64-linux --no-vm"
        ),
        "and builds all but the VM tests: {sent:?}"
    );
    assert!(
        asked("nix-offload-vm", "vm-b"),
        "the VM test goes to its own builder: {sent:?}"
    );
    assert!(
        !sent
            .iter()
            .any(|(to, command)| *to == "nix-offload-vm" && command.contains("#a")),
        "and nothing else does: {sent:?}"
    );
}

/// A VM lender's folder in `folder` whose lender last ran as a pid that's
/// gone, as while a switch restarts it, with VM tests named by the
/// patterns file, and a lent builder beside it.
fn vm_lender_restarting(folder: &Path) -> (PathBuf, PathBuf) {
    let offload = folder.join("offload");
    fs::create_dir_all(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let vm = folder.join("offload-vm");
    fs::create_dir_all(&vm).unwrap();
    fs::write(vm.join("lender"), gone_pid().to_string()).unwrap();
    fs::create_dir_all(folder.join("nix-offload")).unwrap();
    fs::write(
        folder.join("nix-offload/vm-tests"),
        "checks.*.vm-*\nlegacyPackages.*.subtests\n",
    )
    .unwrap();
    (offload, vm)
}

/// A pid no process has: a child's, once it's been waited for.
fn gone_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// The VM lender standing again in `vm` a moment from now, its builder
/// lent, as the lender a switch restarted does.
fn vm_lender_back(vm: &Path) -> std::thread::JoinHandle<()> {
    let vm = vm.to_owned();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        fs::write(
            vm.join("store"),
            STORE.replace("nix-offload", "nix-offload-vm"),
        )
        .unwrap();
        fs::write(vm.join("lender"), std::process::id().to_string()).unwrap();
    })
}

/// Where each `nix build` sent by ssh went, `host: command` a line.
fn builds_sent(bin: &Path) -> Vec<String> {
    let hosts = fs::read_to_string(bin.join("ssh-hosts")).unwrap_or_default();
    let commands = fs::read_to_string(bin.join("ssh-commands")).unwrap_or_default();
    hosts
        .lines()
        .zip(commands.lines())
        .filter(|(_, command)| command.contains("nix build") || command.contains("changed-checks"))
        .map(|(host, command)| format!("{host}: {command}"))
        .collect()
}

#[test]
fn a_vm_test_waits_for_its_own_builder_and_never_goes_the_lent_way() {
    // While a switch restarted the VM lender, its VM tests went to the lent
    // builder beside kernel builds, where good ones timed out.
    let folder = tempfile::tempdir().unwrap();
    stand_ins(folder.path(), 0);
    let (offload, vm) = vm_lender_restarting(folder.path());
    let back = vm_lender_back(&vm);
    let ran = there_command(folder.path(), &offload, folder.path())
        .args(["-L", "github:o/r/abc"])
        .args(["checks.x.vm-a", "checks.x.a"])
        .env("NIX_OFFLOAD_VM_DIR", &vm)
        .output()
        .unwrap();
    back.join().unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = builds_sent(folder.path());
    assert!(
        sent.iter()
            .any(|build| build.starts_with("nix-offload-vm: ") && build.contains("#checks.x.vm-a")),
        "the VM test went to its own builder once it stood: {sent:?}"
    );
    assert!(
        !sent
            .iter()
            .any(|build| build.starts_with("nix-offload: ") && build.contains("vm-a")),
        "and never to the lent one: {sent:?}"
    );
    assert!(
        sent.iter()
            .any(|build| build.starts_with("nix-offload: ") && build.contains("#checks.x.a'")),
        "the rest went the lent way: {sent:?}"
    );
}

#[test]
fn a_vm_test_given_as_its_derivation_goes_to_its_own_builder() {
    let folder = tempfile::tempdir().unwrap();
    stand_ins(folder.path(), 0);
    let (offload, vm) = vm_lender_restarting(folder.path());
    fs::write(
        vm.join("store"),
        STORE.replace("nix-offload", "nix-offload-vm"),
    )
    .unwrap();
    fs::write(vm.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(
        folder.path().join("derivations"),
        r#"{"version": 4, "derivations": {
            "aaaa-vm-test-run-x.drv": {"env": {"requiredSystemFeatures": "kvm nixos-test"}},
            "bbbb-kernel.drv": {"env": {"requiredSystemFeatures": "big-parallel"}}}}"#,
    )
    .unwrap();
    let ran = there_beside(
        folder.path(),
        &offload,
        &vm,
        &[
            "-L",
            "/nix/store/aaaa-vm-test-run-x.drv",
            "/nix/store/bbbb-kernel.drv",
        ],
    );
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = builds_sent(folder.path());
    let built_on = |host: &str| -> Vec<&str> {
        sent.iter()
            .filter_map(|build| build.strip_prefix(&format!("{host}: ")))
            .filter_map(|command| command.split("nix build").nth(1))
            .collect()
    };
    let vm_built = built_on("nix-offload-vm");
    let lent_built = built_on("nix-offload");
    assert!(
        vm_built.len() == 1 && vm_built[0].contains("aaaa-vm-test-run-x.drv^*"),
        "the VM test went to its own builder alone: {sent:?}"
    );
    assert!(
        lent_built.len() == 1
            && lent_built[0].contains("bbbb-kernel.drv^*")
            && !lent_built[0].contains("aaaa"),
        "and the rest the lent way: {sent:?}"
    );
}

#[test]
fn changed_checks_send_their_vm_tests_apart_while_the_vm_lender_restarts() {
    // Decided only while the VM lender ran, a run sent as a switch
    // restarted it ran whole on the lent builder, VM tests and all.
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 0);
    let config = folder.path().join("gitconfig");
    fs::write(
        &config,
        "[user]\nname = Tester\nemail = tester@example.com\n",
    )
    .unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir(&tree).unwrap();
    let git = |arguments: &[&str]| git(&tree, &config, arguments);
    git(&["init", "-q", "-b", "main"]);
    git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
    fs::write(tree.join("flake.nix"), "{ }").unwrap();
    git(&["add", "flake.nix"]);
    git(&["commit", "-q", "-m", "pushed"]);
    let head = git(&["rev-parse", "HEAD"]);
    git(&["update-ref", "refs/remotes/origin/main", &head]);
    let (offload, vm) = vm_lender_restarting(folder.path());
    fs::write(bin.join("says"), "changed  a\nadded    vm-b\n").unwrap();
    let run = |arguments: &[&str]| {
        let _ = fs::remove_file(bin.join("ssh-hosts"));
        let _ = fs::remove_file(bin.join("ssh-commands"));
        there_command(&bin, &offload, folder.path())
            .args(["--changed", "--build"])
            .args(arguments)
            .current_dir(&tree)
            .env("NIX_OFFLOAD_VM_DIR", &vm)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap()
    };

    // Given no system, it's this machine's.
    let back = vm_lender_back(&vm);
    let ran = run(&[]);
    back.join().unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    let sent = builds_sent(&bin);
    let system = format!("{}-linux", std::env::consts::ARCH);
    assert!(
        sent.iter().any(|build| build.starts_with("nix-offload: ")
            && build.contains(&format!(
                "changed-checks -- --build --system {system} --no-vm"
            ))),
        "the lent builder builds all but the VM tests: {sent:?}"
    );
    assert!(
        sent.iter()
            .any(|build| build.starts_with("nix-offload-vm: ")
                && build.contains(&format!("checks.{system}.vm-b"))),
        "the VM test waits for its own builder: {sent:?}"
    );

    // A commit's changed-checks from before --name-only builds all but its
    // VM tests there, and says they weren't built. The run above passed at
    // this commit, so what passed is forgotten for it to run again.
    fs::remove_dir_all(folder.path().join("nix-offload/passed")).unwrap();
    fs::write(bin.join("refuses-name-only"), "").unwrap();
    let ran = run(&["--system", "x86_64-linux"]);
    assert_eq!(ran.status.code(), Some(69), "{ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("VM tests weren't built"),
        "{ran:?}"
    );
    let sent = builds_sent(&bin);
    assert!(
        sent.iter()
            .filter(|build| build.contains("changed-checks -- --build"))
            .all(|build| build.contains("--no-vm")),
        "never with its VM tests: {sent:?}"
    );
}

/// CPU pressure as a builder's /proc/pressure/cpu says it, `some` waiting
/// `avg60` percent of the last minute.
fn pressure(avg60: &str) -> String {
    format!(
        "some avg10={avg60} avg60={avg60} avg300={avg60} total=1\n\
         full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n"
    )
}

// A builder whose CPUs are all taken slows every build sent it: at 00:33
// on 3 November the lent one ran at load 50 to 63 on its 32. So while a
// second lender runs, a build goes to its builder once the first is full,
// and stays on the first while that has room.
#[test]
fn a_build_goes_to_a_second_builder_while_the_first_is_full() {
    let folder = tempfile::tempdir().unwrap();
    stand_ins(folder.path(), 0);
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let more = folder.path().join("offload-more");
    fs::create_dir(&more).unwrap();
    fs::write(more.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(
        more.join("store"),
        STORE.replace("nix-offload", "nix-offload-more"),
    )
    .unwrap();
    fs::write(
        folder.path().join("pressure-nix-offload-more"),
        pressure("5.00"),
    )
    .unwrap();
    let built_on = |first: &str| {
        fs::write(folder.path().join("pressure-nix-offload"), pressure(first)).unwrap();
        let _ = fs::remove_file(folder.path().join("ssh-hosts"));
        let _ = fs::remove_file(folder.path().join("ssh-commands"));
        let ran = there_command(folder.path(), &offload, folder.path())
            .args(["-L", "github:o/r/abc", "checks.x.a"])
            .env("NIX_OFFLOAD_MORE_DIR", &more)
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        let hosts = fs::read_to_string(folder.path().join("ssh-hosts")).unwrap();
        let commands = fs::read_to_string(folder.path().join("ssh-commands")).unwrap();
        let built: Vec<&str> = hosts
            .lines()
            .zip(commands.lines())
            .filter(|(_, command)| command.contains("nix build"))
            .map(|(host, _)| host)
            .collect();
        built.join(" ")
    };
    assert_eq!(built_on("55.00"), "nix-offload-more", "the first is full");
    assert_eq!(built_on("10.00"), "nix-offload", "the first has room");
}

#[test]
fn what_it_starts_is_marked_as_waiting_on_work_elsewhere() {
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    stand_ins(folder.path(), 255);
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there_with(folder.path(), &offload, &["--here"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    for marked in ["ssh-marked", "nix-marked"] {
        assert_eq!(
            fs::read_to_string(folder.path().join(marked)).unwrap(),
            "1\n",
            "{marked}"
        );
    }
}

#[test]
fn with_no_builder_or_none_reached_it_builds_nothing_unless_asked_to_build_here() {
    // With no builder lent after midnight on 30 October, `there` built
    // here, beside every other build, at a load of 145.
    let folder = tempfile::tempdir().unwrap();
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    stand_ins(folder.path(), 255);
    let given = folder.path().join("nix-was-given");
    let here = "build --no-link -L --keep-going github:o/r/abc#checks.x.a\n";

    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("no builder is lent, so nothing was built"),
        "{said}"
    );
    assert!(said.contains("--here"), "{said}");
    assert!(!given.exists(), "nothing was built here");

    let ran = there_with(folder.path(), &offload, &["--here"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("no builder is lent, so this builds here")
    );
    assert_eq!(fs::read_to_string(&given).unwrap(), here);
    assert!(journalled(folder.path())["on"].is_null());

    fs::remove_file(&given).unwrap();
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = there(folder.path(), &offload);
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    assert!(String::from_utf8_lossy(&ran.stderr).contains("couldn't reach nix-offload"));
    assert!(!given.exists(), "nothing was built here");

    let ran = there_with(folder.path(), &offload, &["--here"]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(fs::read_to_string(&given).unwrap(), here);
}

// From 14:00 to 20:15 on 5 November, 189 checks built here while being
// changed took 34,033 s at a CPU pressure of 46% beside them, the load
// the person felt: asked to, `there` builds here only while this machine
// is idle, and never a VM test.
#[test]
fn asked_to_build_here_when_idle_it_builds_here_only_while_this_machine_is_idle() {
    let folder = tempfile::tempdir().unwrap();
    stand_ins(folder.path(), 0);
    let offload = lending(folder.path(), "offload", "nix-offload");
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    fs::write(
        folder.path().join("nix-offload/vm-tests"),
        "checks.*.vm-*\n",
    )
    .unwrap();
    let pressure = folder.path().join("cpu-pressure");
    let given = folder.path().join("nix-was-given");
    let commands = folder.path().join("ssh-commands");
    let run = |attribute: &str, some: (&str, &str)| {
        fs::write(
            &pressure,
            format!(
                "some avg10={} avg60={} avg300=30.00 total=1\n\
                 full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
                some.0, some.1
            ),
        )
        .unwrap();
        let _ = fs::remove_file(&given);
        let _ = fs::remove_file(&commands);
        let ran = there_command(folder.path(), &offload, folder.path())
            .args(["--here-when-idle", "-L", "github:o/r/abc", attribute])
            .env("NIX_OFFLOAD_CPU_PRESSURE", &pressure)
            // No VM lender has run, so a VM test goes the lent way.
            .env("NIX_OFFLOAD_VM_DIR", folder.path().join("no-vm"))
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        let sent = fs::read_to_string(&commands)
            .unwrap_or_default()
            .contains("nix build");
        let built_here = fs::read_to_string(&given)
            .unwrap_or_default()
            .starts_with("build ");
        (
            built_here,
            sent,
            String::from_utf8_lossy(&ran.stderr).into_owned(),
        )
    };

    let (built_here, sent, said) = run("checks.x.a", ("2.00", "4.50"));
    assert!(built_here && !sent, "idle, it builds here: {said}");
    assert!(
        said.contains("this machine is idle, so this builds here"),
        "{said}"
    );

    let (built_here, sent, said) = run("checks.x.a", ("2.00", "55.27"));
    assert!(
        sent && !built_here,
        "busy this last minute, it's sent: {said}"
    );

    let (built_here, sent, said) = run("checks.x.a", ("43.38", "4.50"));
    assert!(sent && !built_here, "busy now, it's sent: {said}");

    let (built_here, _, said) = run("checks.x.vm-a", ("2.00", "4.50"));
    assert!(!built_here, "a VM test is never built here: {said}");
}

/// A lender's folder in `folder`, named `name`, its lender this test and
/// its builder `host`.
fn lending(folder: &Path, name: &str, host: &str) -> PathBuf {
    let dir = folder.join(name);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("lender"), std::process::id().to_string()).unwrap();
    fs::write(dir.join("store"), STORE.replace("nix-offload", host)).unwrap();
    dir
}

// A VM test goes to a builder of its own beside the rest of its build, and
// passes no more cheaply there: its pass is kept with the rest's.
#[test]
fn a_vm_test_sent_apart_that_passed_at_a_commit_isnt_built_again() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 0);
    let offload = lending(folder.path(), "offload", "nix-offload");
    let vm = lending(folder.path(), "offload-vm", "nix-offload-vm");
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    fs::write(
        folder.path().join("nix-offload/vm-tests"),
        "checks.*.vm-*\n",
    )
    .unwrap();
    let flake = format!("github:o/r/{}", "a".repeat(40));
    let sent = || {
        let ran = there_command(&bin, &offload, folder.path())
            .args(["-L", &flake, "checks.x.a", "checks.x.vm-a"])
            .env("NIX_OFFLOAD_VM_DIR", &vm)
            .env("NIX_OFFLOAD_VM_MORE_DIR", NO_SECOND)
            .output()
            .unwrap();
        assert_eq!(ran.status.code(), Some(0), "{ran:?}");
        let sent = fs::read_to_string(bin.join("ssh-sent")).unwrap_or_default();
        let _ = fs::remove_file(bin.join("ssh-sent"));
        let mut hosts: Vec<String> = sent
            .lines()
            .filter(|line| line.contains("nix build"))
            .filter_map(|line| Some(line.split_once(' ')?.0.to_owned()))
            .collect();
        hosts.sort();
        (hosts, String::from_utf8_lossy(&ran.stderr).into_owned())
    };

    let (hosts, _) = sent();
    assert_eq!(hosts, ["nix-offload", "nix-offload-vm"], "split");

    let (hosts, said) = sent();
    assert!(hosts.is_empty(), "nothing is sent again: {hosts:?}");
    assert!(
        said.contains("checks.x.vm-a passed at aaaaaaaaaaaa already"),
        "{said}"
    );
}

// Nix's max-jobs bounds each client's build, not the builder: on 4
// November the VM builder, told 4 at once, ran 15 VM tests, each `there`
// run 4 of its own, and timing-sensitive ones failed. So a run waits for
// room those still going leave, as many as the machine says, else 4.
#[test]
fn two_runs_send_a_vm_builder_no_more_vm_tests_at_once_than_its_room() {
    let four: Vec<String> = ["-L", "github:o/r/abc"]
        .into_iter()
        .map(str::to_owned)
        .chain((0..4).map(|test| format!("checks.x.vm-{test}")))
        .collect();
    let (at_once, said) = side_by_side([&four, &four]);
    assert_eq!(at_once, ["1", "1"], "one run's 4 at a time, at a room of 4");
    assert_eq!(
        said.matches("so this waits for room").count(),
        1,
        "the second says so once: {said}"
    );
}

// run-subtests --there builds a VM test's subtests through `there`, so
// they wait for room beside other runs' VM tests the same way.
#[test]
fn run_subtests_there_waits_for_the_vm_builders_room_too() {
    let four: Vec<String> = ["-L", "github:o/r/abc"]
        .into_iter()
        .map(str::to_owned)
        .chain((0..4).map(|test| format!("checks.x.vm-{test}")))
        .collect();
    let subtests: Vec<String> = [
        "-L",
        "--apply",
        "subtests: subtests { check = \"vm-x\"; words = [ ]; }",
        "github:o/r/abc",
        "legacyPackages.x.subtests",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let (at_once, said) = side_by_side([&four, &subtests]);
    assert_eq!(at_once, ["1", "1"], "the subtests wait for the 4");
    assert_eq!(
        said.matches("so this waits for room").count(),
        1,
        "the second says so once: {said}"
    );
}

// A run of 4 goes only once nothing else is sent, so while it waited,
// runs of 1 that fit beside the rest kept going ahead of it: once it has
// waited long enough, nothing passes it.
#[test]
fn a_run_waiting_for_room_long_enough_isnt_passed_by_one_that_came_after_it() {
    let tests = |name: &str, count: usize| -> Vec<String> {
        ["-L", "github:o/r/abc"]
            .into_iter()
            .map(str::to_owned)
            .chain((0..count).map(|test| format!("checks.x.vm-{name}{test}")))
            .collect()
    };
    let (first, waits, later) = (tests("a", 3), tests("b", 4), tests("c", 1));
    let (_, sent, said) = one_after_another_with(
        &[&first, &waits, &later],
        &[("NIX_OFFLOAD_PASSING_FOR", "0")],
    );
    let order: Vec<&str> = sent
        .iter()
        .map(|build| {
            ["vm-a0", "vm-b0", "vm-c0"]
                .into_iter()
                .find(|test| build.contains(*test))
                .unwrap_or(build)
        })
        .collect();
    assert_eq!(order, ["vm-a0", "vm-b0", "vm-c0"], "{said}");
    assert_eq!(said.matches("so this waits for room").count(), 2, "{said}");
}

// On 8 November three places sat idle on two VM builders while nine parts
// waited, some for 70 minutes, behind parts that could go only once their
// builder was empty, so one that fits passes one that doesn't yet.
#[test]
fn a_run_that_fits_passes_one_waiting_that_doesnt_yet() {
    let tests = |name: &str, count: usize| -> Vec<String> {
        ["-L", "github:o/r/abc"]
            .into_iter()
            .map(str::to_owned)
            .chain((0..count).map(|test| format!("checks.x.vm-{name}{test}")))
            .collect()
    };
    let (first, waits, later) = (tests("a", 3), tests("b", 4), tests("c", 1));
    let (_, sent, said) = one_after_another(&[&first, &waits, &later]);
    let order: Vec<&str> = sent
        .iter()
        .map(|build| {
            ["vm-a0", "vm-b0", "vm-c0"]
                .into_iter()
                .find(|test| build.contains(*test))
                .unwrap_or(build)
        })
        .collect();
    assert_eq!(order, ["vm-a0", "vm-c0", "vm-b0"], "{said}");
    assert_eq!(said.matches("so this waits for room").count(), 1, "{said}");
}

// The after-merge net sends dozens of VM tests a round, so on 5 November a
// session's few, sent while one waited, waited behind the whole round: a
// bulk run lets any run that isn't, waiting or coming, go before it.
#[test]
fn a_bulk_run_waiting_for_room_lets_one_that_isnt_go_first() {
    let tests = |name: &str, count: usize, flags: &[&str]| -> Vec<String> {
        flags
            .iter()
            .copied()
            .chain(["-L", "github:o/r/abc"])
            .map(str::to_owned)
            .chain((0..count).map(|test| format!("checks.x.vm-{name}{test}")))
            .collect()
    };
    let (first, bulk, later) = (
        tests("a", 3, &[]),
        tests("b", 4, &["--bulk"]),
        tests("c", 2, &[]),
    );
    let (_, sent, said) = one_after_another(&[&first, &bulk, &later]);
    let order: Vec<&str> = sent
        .iter()
        .map(|build| {
            ["vm-a0", "vm-b0", "vm-c0"]
                .into_iter()
                .find(|test| build.contains(*test))
                .unwrap_or(build)
        })
        .collect();
    assert_eq!(order, ["vm-a0", "vm-c0", "vm-b0"], "{said}");
    assert!(
        said.contains("this run is bulk, so runs that aren't go first"),
        "{said}"
    );
}

#[test]
fn a_run_sent_first_goes_ahead_of_every_run_waiting_for_room() {
    // On 8 November the net tested nothing for an hour while sessions'
    // runs held the builders, so the net's are sent first, and go ahead
    // of runs that came before them and aren't.
    let tests = |name: &str, count: usize, flags: &[&str]| -> Vec<String> {
        flags
            .iter()
            .copied()
            .chain(["-L", "github:o/r/abc"])
            .map(str::to_owned)
            .chain((0..count).map(|test| format!("checks.x.vm-{name}{test}")))
            .collect()
    };
    let (building, waiting, first) = (
        tests("a", 3, &[]),
        tests("b", 2, &[]),
        tests("c", 2, &["--bulk", "--first"]),
    );
    let (_, sent, said) = one_after_another(&[&building, &waiting, &first]);
    let order: Vec<&str> = sent
        .iter()
        .map(|build| {
            ["vm-a0", "vm-b0", "vm-c0"]
                .into_iter()
                .find(|test| build.contains(*test))
                .unwrap_or(build)
        })
        .collect();
    assert_eq!(order, ["vm-a0", "vm-c0", "vm-b0"], "{said}");
}

/// Two `there` runs with `runs`' arguments, the second started while the
/// first builds: how many the VM builder built at once as each began, and
/// what both said.
fn side_by_side(runs: [&[String]; 2]) -> (Vec<String>, String) {
    let (at_once, _, said) = one_after_another(&runs);
    (at_once, said)
}

/// `there` runs with `runs`' arguments, each started once the one before
/// it builds on the VM builder or waits for room there, beside a VM
/// builder at a room of 4, each build there lasting 2 seconds: how many
/// it built at once as each began, the VM builds it was sent in the order
/// they were, and what every run said.
fn one_after_another(runs: &[&[String]]) -> (Vec<String>, Vec<String>, String) {
    one_after_another_with(runs, &[])
}

/// `one_after_another`, each run told these variables besides.
fn one_after_another_with(
    runs: &[&[String]],
    told: &[(&str, &str)],
) -> (Vec<String>, Vec<String>, String) {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 0);
    let offload = lending(folder.path(), "offload", "nix-offload");
    let vm = lending(folder.path(), "offload-vm", "nix-offload-vm");
    fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
    fs::write(
        folder.path().join("nix-offload/vm-tests"),
        "checks.*.vm-*\nlegacyPackages.*.subtests\n",
    )
    .unwrap();
    fs::write(bin.join("holds-nix-offload-vm"), "2").unwrap();
    let run = |args: &[String], said: &Path| {
        there_command(&bin, &offload, folder.path())
            .args(args)
            .env("NIX_OFFLOAD_VM_DIR", &vm)
            .envs(told.iter().copied())
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(said).unwrap())
            .spawn()
            .unwrap()
    };
    let vm_builds = || -> Vec<String> {
        fs::read_to_string(bin.join("ssh-sent"))
            .unwrap_or_default()
            .lines()
            .filter(|line| line.starts_with("nix-offload-vm ") && line.contains("nix build"))
            .map(str::to_owned)
            .collect()
    };
    let mut started = Vec::new();
    for (index, args) in runs.iter().enumerate() {
        // Its standard error goes to a file of its own, so its wait can be
        // seen while it waits.
        let said = folder.path().join(format!("said-{index}"));
        let before = vm_builds().len();
        let mut child = run(args, &said);
        let began = std::time::Instant::now();
        while vm_builds().len() == before
            && !fs::read_to_string(&said)
                .unwrap_or_default()
                .contains("so this waits for room")
        {
            assert!(
                began.elapsed() < std::time::Duration::from_secs(30),
                "run {index} neither built nor waited"
            );
            if let Some(status) = child.try_wait().unwrap() {
                panic!("run {index} ended first: {status}");
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        started.push(child);
    }
    let mut said = String::new();
    for (index, mut child) in started.into_iter().enumerate() {
        let status = child.wait().unwrap();
        let told = fs::read_to_string(folder.path().join(format!("said-{index}"))).unwrap();
        assert_eq!(status.code(), Some(0), "run {index}: {told}");
        said.push_str(&told);
    }
    let at_once = fs::read_to_string(bin.join("at-once-nix-offload-vm")).unwrap();
    (
        at_once.lines().map(str::to_owned).collect(),
        vm_builds(),
        said,
    )
}

/// A machine with a VM lender, and a second one once `lend_second` is
/// called, its first VM builder running six at once, and what `there`
/// sends them.
struct TwoVm {
    folder: tempfile::TempDir,
    bin: PathBuf,
    offload: PathBuf,
    vm: PathBuf,
    more: PathBuf,
    /// A third VM lender's folder, lending once `lend_third` is called.
    third: PathBuf,
}

impl TwoVm {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir(&bin).unwrap();
        stand_ins(&bin, 0);
        let offload = lending(folder.path(), "offload", "nix-offload");
        let vm = lending(folder.path(), "offload-vm", "nix-offload-vm");
        let more = folder.path().join("offload-vm-more");
        let third = folder.path().join("offload-vm-3");
        fs::create_dir_all(folder.path().join("nix-offload")).unwrap();
        fs::write(
            folder.path().join("nix-offload/vm-tests"),
            "checks.*.vm-*\n",
        )
        .unwrap();
        fs::write(folder.path().join("nix-offload/vm-builds"), "6\n").unwrap();
        Self {
            folder,
            bin,
            offload,
            vm,
            more,
            third,
        }
    }

    fn lend_third(&self) {
        fs::create_dir(&self.third).unwrap();
        fs::write(self.third.join("lender"), std::process::id().to_string()).unwrap();
        fs::write(
            self.third.join("store"),
            STORE.replace("nix-offload", "nix-offload-vm-3"),
        )
        .unwrap();
    }

    fn lend_second(&self) {
        fs::create_dir(&self.more).unwrap();
        fs::write(self.more.join("lender"), std::process::id().to_string()).unwrap();
        fs::write(
            self.more.join("store"),
            STORE.replace("nix-offload", "nix-offload-vm-more"),
        )
        .unwrap();
    }

    /// A run of one check and `tests` VM tests: how many VM tests each
    /// host was sent, by host, and what it said.
    fn sent(&self, tests: usize) -> (Vec<(String, usize)>, String) {
        let running = self.start(tests);
        self.finish(running)
    }

    /// That run, started, its standard error going to the `said` file.
    fn start(&self, tests: usize) -> std::process::Child {
        let _ = fs::remove_file(self.bin.join("ssh-sent"));
        let said = self.folder.path().join("said");
        there_command(&self.bin, &self.offload, self.folder.path())
            .args(["-L", "github:o/r/abc", "checks.x.a"])
            .args((0..tests).map(|test| format!("checks.x.vm-{test}")))
            .env("NIX_OFFLOAD_VM_DIR", &self.vm)
            .env("NIX_OFFLOAD_VM_MORE_DIR", &self.more)
            .env("NIX_OFFLOAD_VM_EXTRA_DIRS", &self.third)
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&said).unwrap())
            .spawn()
            .unwrap()
    }

    /// What it said so far.
    fn said(&self) -> String {
        fs::read_to_string(self.folder.path().join("said")).unwrap_or_default()
    }

    /// The run started, once it ends: what each host was sent and what it
    /// said. One still waiting after 30 s fails, as one sent behind a queue
    /// that never moves would.
    fn finish(&self, mut running: std::process::Child) -> (Vec<(String, usize)>, String) {
        let said = self.folder.path().join("said");
        let began = std::time::Instant::now();
        let status = loop {
            if let Some(status) = running.try_wait().unwrap() {
                break status;
            }
            if began.elapsed() > std::time::Duration::from_secs(30) {
                let _ = running.kill();
                let _ = running.wait();
                panic!(
                    "it waited rather than going: {}",
                    fs::read_to_string(&said).unwrap_or_default()
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        let said = fs::read_to_string(&said).unwrap();
        assert_eq!(status.code(), Some(0), "{said}");
        let sent = fs::read_to_string(self.bin.join("ssh-sent")).unwrap();
        let mut built: Vec<(String, usize)> = sent
            .lines()
            .filter_map(|line| line.split_once(' '))
            .filter(|(_, command)| command.contains("nix build"))
            .map(|(host, command)| (host.to_owned(), command.matches("#checks.x.vm-").count()))
            .collect();
        built.sort();
        (built, said)
    }
}

/// How many VM tests `host` was sent, of what `TwoVm::sent` says.
fn tests_on(built: &[(String, usize)], host: &str) -> usize {
    built
        .iter()
        .filter(|(to, _)| to == host)
        .map(|(_, tests)| tests)
        .sum()
}

// The VM builder runs six VM tests at once, and the rest wait their turn
// there: on 3 November 49 of 189 runs failed, 37% of VM tests failing
// with 7 to 12 runs overlapping. So while a second VM lender runs, the VM
// tests past the first's room go to its builder, at once beside the
// first's, counting those other runs already sent.
#[test]
fn vm_tests_past_the_vm_builders_room_go_to_a_second_at_once() {
    let rig = TwoVm::new();
    let (built, _) = rig.sent(8);
    assert_eq!(
        tests_on(&built, "nix-offload-vm"),
        8,
        "no second lender runs: {built:?}"
    );

    rig.lend_second();
    let (built, said) = rig.sent(8);
    assert_eq!(
        tests_on(&built, "nix-offload-vm"),
        6,
        "the first's room: {built:?}"
    );
    assert_eq!(
        tests_on(&built, "nix-offload-vm-more"),
        2,
        "the rest: {built:?}"
    );
    assert!(
        built.iter().any(|(to, _)| to == "nix-offload"),
        "the rest: {built:?}"
    );
    assert!(said.contains("so 2 go to a second VM builder"), "{said}");

    let (built, _) = rig.sent(3);
    assert_eq!(
        tests_on(&built, "nix-offload-vm-more"),
        0,
        "room on the first: {built:?}"
    );

    // Another run's five are there already, so one of these fits.
    fs::create_dir_all(rig.vm.join("sending")).unwrap();
    fs::write(
        rig.vm.join(format!("sending/{}.0", std::process::id())),
        "5\n",
    )
    .unwrap();
    // And a run since gone counts for nothing.
    fs::write(rig.vm.join("sending/999999999.0"), "6\n").unwrap();
    let (built, _) = rig.sent(3);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 1, "{built:?}");
    assert_eq!(tests_on(&built, "nix-offload-vm-more"), 2, "{built:?}");
    assert!(
        !rig.vm.join("sending/999999999.0").exists(),
        "the gone run's is taken away"
    );
    let claims = fs::read_dir(rig.vm.join("sending")).unwrap().count();
    assert_eq!(claims, 1, "each run takes its own away as it ends");
}

// On 5 November 168 VM runs in four hours waited a median of 23 to 27
// minutes on two VM builders, mostly for room, so a third and more take
// what's past the first two's room, at once, and each run says how long
// it waited for its builder.
#[test]
fn vm_tests_past_two_vm_builders_room_go_to_a_third_at_once() {
    let rig = TwoVm::new();
    rig.lend_second();
    rig.lend_third();
    for dir in [&rig.vm, &rig.more] {
        fs::create_dir_all(dir.join("sending")).unwrap();
        fs::write(dir.join(format!("sending/{}.0", std::process::id())), "6\n").unwrap();
    }
    let (built, said) = rig.sent(3);
    assert_eq!(tests_on(&built, "nix-offload-vm-3"), 3, "{built:?} {said}");
    assert_eq!(tests_on(&built, "nix-offload-vm"), 0, "{built:?}");
    assert_eq!(tests_on(&built, "nix-offload-vm-more"), 0, "{built:?}");
    let journal = fs::read_to_string(rig.folder.path().join("nix-offload/there.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = journal
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        lines
            .iter()
            .all(|line| line["waited"].as_u64().is_some_and(|waited| waited < 30)),
        "each part says how long it waited for its builder: {journal}"
    );
}

// On 6 November each fresh VM builder's first tests took 65 to 80 minutes
// building the desktop for themselves, against 2 to 12 once warm, so while
// a warm VM builder stands, none goes to one that's cold: one warming, or
// one not yet asked, which is asked for once the warm ones' room runs out.
#[test]
fn vm_tests_go_to_a_warm_vm_builder_while_one_stands() {
    let rig = TwoVm::new();
    rig.lend_second();
    let warming = rig.more.join("warming");
    fs::write(&warming, format!("{} vm-more-1\n", std::process::id())).unwrap();
    // A third whose lender runs with no builder yet.
    fs::create_dir(&rig.third).unwrap();
    fs::write(rig.third.join("lender"), std::process::id().to_string()).unwrap();

    let (built, said) = rig.sent(8);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 8, "{built:?} {said}");
    assert_eq!(tests_on(&built, "nix-offload-vm-more"), 0, "{built:?}");
    assert!(said.contains("so VM tests go to the warm one"), "{said}");
    assert!(
        !rig.third.join("wanted").exists(),
        "the warming one is room enough to come: {said}"
    );

    let (built, said) = rig.sent(14);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 14, "{built:?} {said}");
    assert!(said.contains("and 1 more is asked for"), "{said}");
    assert_eq!(
        fs::read_dir(rig.third.join("wanted")).unwrap().count(),
        1,
        "the third is asked for while the run waits: {said}"
    );

    fs::remove_file(&warming).unwrap();
    let (built, said) = rig.sent(8);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 6, "{built:?} {said}");
    assert_eq!(
        tests_on(&built, "nix-offload-vm-more"),
        2,
        "warm now: {built:?}"
    );
    assert!(
        !said.contains("warm one"),
        "a lender with no builder on its way says nothing: {said}"
    );
}

// A run sent where others wait for room goes only after them, so those
// waiting count as the builder's too: on 4 November ten runs waited 51
// minutes on the first VM builder, with 2 of its 4 claimed, while the
// second sat idle and was given back.
#[test]
fn vm_tests_go_to_a_second_builder_rather_than_wait_behind_runs_on_the_first() {
    let rig = TwoVm::new();
    rig.lend_second();
    fs::create_dir_all(rig.vm.join("sending")).unwrap();
    fs::write(
        rig.vm.join(format!("sending/{}.0", std::process::id())),
        "2\n",
    )
    .unwrap();
    fs::create_dir_all(rig.vm.join("waiting")).unwrap();
    let place = |pid: u32| rig.vm.join(format!("waiting/{:020}.{pid}.0", 1));
    fs::write(place(std::process::id()), "6\n").unwrap();
    // A place whose run is gone counts for nothing.
    fs::write(place(999_999_999), "6\n").unwrap();
    let (built, said) = rig.sent(3);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 0, "{built:?}");
    assert_eq!(tests_on(&built, "nix-offload-vm-more"), 3, "{built:?}");
    assert!(said.contains("8 VM tests sent or waiting"), "{said}");
}

// A run's split is decided as it starts, so one waiting on the first VM
// builder goes to the second once that one has room and nothing waits
// there, rather than waiting out the first.
#[test]
fn a_vm_run_waiting_on_the_first_builder_goes_to_the_second_once_it_has_room() {
    let rig = TwoVm::new();
    rig.lend_second();
    let full = |dir: &Path| {
        fs::create_dir_all(dir.join("sending")).unwrap();
        let claim = dir.join(format!("sending/{}.0", std::process::id()));
        fs::write(&claim, "6\n").unwrap();
        claim
    };
    full(&rig.vm);
    let second = full(&rig.more);
    let running = rig.start(3);
    let began = std::time::Instant::now();
    while !rig.said().contains("so this waits for room") {
        assert!(
            began.elapsed() < std::time::Duration::from_secs(30),
            "it never waited: {}",
            rig.said()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    fs::remove_file(second).unwrap();
    let (built, said) = rig.finish(running);
    assert_eq!(tests_on(&built, "nix-offload-vm"), 0, "{built:?}");
    assert_eq!(tests_on(&built, "nix-offload-vm-more"), 3, "{built:?}");
    assert!(said.contains("another VM builder has room now"), "{said}");
}

#[test]
fn a_command_runs_on_the_lent_builder_with_its_paths_sent_first_or_nowhere() {
    let folder = tempfile::tempdir().unwrap();
    let bin = folder.path().join("bin");
    fs::create_dir(&bin).unwrap();
    stand_ins(&bin, 3);
    let config = folder.path().join("gitconfig");
    fs::write(
        &config,
        "[user]\nname = Tester\nemail = tester@example.com\n",
    )
    .unwrap();
    let tree = folder.path().join("tree");
    fs::create_dir(&tree).unwrap();
    let git = |arguments: &[&str]| git(&tree, &config, arguments);
    git(&["init", "-q", "-b", "main"]);
    git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
    fs::write(tree.join("package.json"), "{ }").unwrap();
    git(&["add", "package.json"]);
    git(&["commit", "-q", "-m", "pushed"]);
    let head = git(&["rev-parse", "HEAD"]);
    git(&["update-ref", "refs/remotes/origin/main", &head]);
    let offload = folder.path().join("offload");
    fs::create_dir(&offload).unwrap();
    let run = || {
        there_command(&bin, &offload, folder.path())
            .args(["--never-ask", "--run", "--copy", "/nix/store/a-runtime"])
            .args(["--longest", "60", "--", "pnpm", "test", "--reporter", "dot"])
            .current_dir(&tree)
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .unwrap()
    };

    // Lent, it runs there on the commit, its paths sent first, and its
    // status is the run's. Its ssh names the builder while it runs, which
    // is what the lender counts a builder in use by.
    fs::write(offload.join("store"), STORE).unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(3), "{ran:?}");
    let sent = fs::read_to_string(bin.join("ssh-was-given")).unwrap();
    assert_eq!(sent.lines().next(), Some("nix-offload"), "{sent}");
    let command = within_far(sent.lines().nth(1).unwrap());
    for part in [
        format!("checkout -q -B nix-offload-changed {head}"),
        "sleep 60".to_owned(),
        "WORK_RUNS_ELSEWHERE=1 pnpm test --reporter dot;".to_owned(),
    ] {
        assert!(command.contains(&part), "{part} in {command}");
    }
    let calls = fs::read_to_string(bin.join("nix-calls")).unwrap();
    assert!(
        calls.contains("copy --to ssh-ng://nix-offload") && calls.contains("/nix/store/a-runtime"),
        "{calls}"
    );

    // Paths that can't go keep it from running anywhere.
    fs::remove_file(bin.join("ssh-was-given")).unwrap();
    fs::write(bin.join("nix-status"), "1").unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    assert!(
        !bin.join("ssh-was-given").exists(),
        "it ran without its paths"
    );
    fs::remove_file(bin.join("nix-status")).unwrap();

    // With no builder lent, nothing runs, here or anywhere.
    fs::remove_file(offload.join("store")).unwrap();
    let ran = run();
    assert_eq!(ran.status.code(), Some(75), "{ran:?}");
    assert!(!bin.join("ssh-was-given").exists(), "it ran somewhere");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("so nothing ran"),
        "{ran:?}"
    );
}

/// A tree for `there --run`: one commit origin has and one on top it
/// lacks, so it goes as a bundle, with a builder lent whose stand-in
/// exits 0.
struct RunTree {
    folder: tempfile::TempDir,
    bin: PathBuf,
    config: PathBuf,
    tree: PathBuf,
    offload: PathBuf,
    head: String,
}

impl RunTree {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let bin = folder.path().join("bin");
        fs::create_dir(&bin).unwrap();
        stand_ins(&bin, 0);
        let config = folder.path().join("gitconfig");
        fs::write(
            &config,
            "[user]\nname = Tester\nemail = tester@example.com\n",
        )
        .unwrap();
        let tree = folder.path().join("tree");
        fs::create_dir(&tree).unwrap();
        let offload = folder.path().join("offload");
        fs::create_dir(&offload).unwrap();
        fs::write(offload.join("store"), STORE).unwrap();
        let mut made = Self {
            folder,
            bin,
            config,
            tree,
            offload,
            head: String::new(),
        };
        made.git(&["init", "-q", "-b", "main"]);
        made.git(&["remote", "add", "origin", "https://github.com/o/r.git"]);
        fs::write(made.tree.join("package.json"), "{ }").unwrap();
        made.git(&["add", "package.json"]);
        made.git(&["commit", "-q", "-m", "pushed"]);
        let pushed = made.git(&["rev-parse", "HEAD"]);
        made.git(&["update-ref", "refs/remotes/origin/main", &pushed]);
        fs::write(made.tree.join("package.json"), "{ \"name\": \"r\" }").unwrap();
        made.git(&["commit", "-q", "-am", "not pushed"]);
        made.head = made.git(&["rev-parse", "HEAD"]);
        made
    }

    fn git(&self, arguments: &[&str]) -> String {
        git(&self.tree, &self.config, arguments)
    }

    /// A repository beside the tree with a commit of its own, which the
    /// tree's session isn't meant to send, and that commit.
    fn hidden(&self) -> (PathBuf, String) {
        let hidden = self.folder.path().join("hidden");
        fs::create_dir(&hidden).unwrap();
        git(&hidden, &self.config, &["init", "-q", "-b", "main"]);
        fs::write(hidden.join("secret"), "not for the builder").unwrap();
        git(&hidden, &self.config, &["add", "secret"]);
        git(&hidden, &self.config, &["commit", "-q", "-m", "hidden"]);
        let commit = git(&hidden, &self.config, &["rev-parse", "HEAD"]);
        (hidden, commit)
    }

    /// `there --run -- pnpm test` from the tree, with `vars` set too.
    fn run(&self, vars: &[(&str, &Path)]) -> Output {
        self.there(&["--never-ask", "--run", "--", "pnpm", "test"], vars)
    }

    /// `there` asked `words` from the tree, with `vars` set too.
    fn there(&self, words: &[&str], vars: &[(&str, &Path)]) -> Output {
        let _ = fs::remove_file(self.bin.join("ssh-was-given"));
        there_command(&self.bin, &self.offload, self.folder.path())
            .args(words)
            .current_dir(&self.tree)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .envs(vars.iter().copied())
            .output()
            .unwrap()
    }

    /// The commit a run that went checked out there.
    fn sent(&self) -> Option<String> {
        let given = fs::read_to_string(self.bin.join("ssh-was-given")).ok()?;
        let command = given.lines().nth(1)?.to_owned();
        command
            .split("checkout -q -B nix-offload-changed ")
            .nth(1)
            .and_then(|rest| rest.get(..40))
            .map(str::to_owned)
    }

    /// Asserts `ran` was refused before anything went, saying `why`.
    fn refused(&self, ran: &Output, why: &str) {
        assert_eq!(ran.status.code(), Some(75), "{ran:?}");
        assert!(!self.bin.join("ssh-was-given").exists(), "it went: {ran:?}");
        assert!(
            String::from_utf8_lossy(&ran.stderr).contains(why),
            "{why}: {ran:?}"
        );
    }
}

// A run goes out of the fence a session works in, as the person, and on
// 11 November one was shown to read through a gitlink the session planted
// into a repository the fence hides, and to run filters the tree's
// attributes picked. So it sends HEAD's objects alone.
#[test]
fn a_run_sends_only_whats_committed_and_refuses_a_tree_with_changes() {
    let run = RunTree::new();
    let (hidden, hidden_head) = run.hidden();
    let marker = run.folder.path().join("a-filter-ran");
    fs::write(
        &run.config,
        format!(
            "[user]\nname = Tester\nemail = tester@example.com\n\
             [filter \"planted\"]\nclean = touch {} && cat\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::write(run.tree.join(".gitattributes"), "* filter=planted\n").unwrap();
    run.git(&["add", ".gitattributes"]);
    run.git(&["commit", "-q", "-m", "attributes"]);
    let _ = fs::remove_file(&marker);
    let head = run.git(&["rev-parse", "HEAD"]);
    // A gitlink staged, its folder naming the hidden repository, and a
    // committed file touched, so a diff has to read it again.
    fs::create_dir(run.tree.join("vendored")).unwrap();
    fs::write(
        run.tree.join("vendored/.git"),
        format!("gitdir: {}\n", hidden.join(".git").display()),
    )
    .unwrap();
    run.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{hidden_head},vendored"),
    ]);
    let file = run.tree.join("package.json");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, &text).unwrap();
    let _ = fs::remove_file(&marker);

    let ran = run.run(&[]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(
        run.sent().as_deref(),
        Some(head.as_str()),
        "HEAD went alone"
    );
    assert!(!marker.exists(), "no filter the tree picked ran");

    fs::write(&file, "{ \"name\": \"changed\" }").unwrap();
    run.refused(&run.run(&[]), "commit them first");
    run.git(&["checkout", "-q", "--", "package.json"]);
    fs::write(run.tree.join("new.js"), "").unwrap();
    run.git(&["add", "new.js"]);
    let _ = fs::remove_file(&marker);
    run.refused(&run.run(&[]), "commit them first");
    assert!(!marker.exists(), "no filter the tree picked ran");
}

#[test]
fn a_run_refuses_a_tree_that_borrows_objects_and_reads_only_its_own() {
    let run = RunTree::new();
    let (hidden, hidden_head) = run.hidden();
    let objects = hidden.join(".git/objects");
    let alternates = run.tree.join(".git/objects/info/alternates");
    fs::create_dir_all(alternates.parent().unwrap()).unwrap();
    fs::write(&alternates, format!("{}\n", objects.display())).unwrap();
    run.refused(&run.run(&[]), "borrows objects from another repository");
    fs::remove_file(&alternates).unwrap();
    let http = run.tree.join(".git/objects/info/http-alternates");
    fs::write(&http, "https://example.com/objects\n").unwrap();
    run.refused(&run.run(&[]), "borrows objects from another repository");
    fs::remove_file(&http).unwrap();

    // Neither the caller's variables nor a replacement reach past the
    // tree's own repository.
    // A replacement with no parent would share no commit with origin.
    let tree = run.git(&["rev-parse", "HEAD^{tree}"]);
    let orphan = run.git(&["commit-tree", &tree, "-m", "replacement"]);
    run.git(&["replace", &run.head, &orphan]);
    let ran = run.run(&[
        ("GIT_DIR", &hidden.join(".git")),
        ("GIT_ALTERNATE_OBJECT_DIRECTORIES", &objects),
    ]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(
        run.sent().as_deref(),
        Some(run.head.as_str()),
        "its own HEAD"
    );
    run.git(&["replace", "-d", &run.head]);
    // Its branch naming the hidden commit finds no such object, though
    // the caller's alternates hold it.
    fs::write(
        run.tree.join(".git/refs/heads/main"),
        format!("{hidden_head}\n"),
    )
    .unwrap();
    run.refused(
        &run.run(&[("GIT_ALTERNATE_OBJECT_DIRECTORIES", &objects)]),
        "has no commit",
    );
}

// A gitlink a session commits names another repository's commit, and the
// run carries that name alone: the hidden commit and what it holds stay
// behind, since the builder never fetches a submodule.
#[test]
fn a_run_carries_a_committed_gitlink_as_its_hash_alone() {
    let run = RunTree::new();
    let (hidden, hidden_head) = run.hidden();
    // The builder's ssh keeps what it's sent, which is the bundle.
    fs::rename(run.bin.join("ssh"), run.bin.join("ssh-builder")).unwrap();
    executable(
        &run.bin.join("ssh"),
        "#!/bin/sh\nhere=$(dirname \"$0\")\n\
         tee -a \"$here/received\" | \"$here/ssh-builder\" \"$@\"\n",
    );
    fs::create_dir(run.tree.join("vendored")).unwrap();
    fs::write(
        run.tree.join("vendored/.git"),
        format!("gitdir: {}\n", hidden.join(".git").display()),
    )
    .unwrap();
    run.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{hidden_head},vendored"),
    ]);
    run.git(&["commit", "-q", "-m", "vendored"]);
    let head = run.git(&["rev-parse", "HEAD"]);

    let ran = run.run(&[]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(run.sent().as_deref(), Some(head.as_str()), "HEAD went");
    let unpacked = run.folder.path().join("unpacked");
    fs::create_dir(&unpacked).unwrap();
    git(&unpacked, &run.config, &["init", "-q"]);
    // What GitHub holds, which the bundle builds on, as the builder has.
    git(
        &unpacked,
        &run.config,
        &[
            "fetch",
            "-q",
            &run.tree.to_string_lossy(),
            "refs/remotes/origin/main:refs/heads/pushed",
        ],
    );
    git(
        &unpacked,
        &run.config,
        &[
            "bundle",
            "unbundle",
            &run.bin.join("received").to_string_lossy(),
        ],
    );
    let has = |object: &str| {
        Command::new("git")
            .args(["cat-file", "-e", object])
            .current_dir(&unpacked)
            .status()
            .unwrap()
            .success()
    };
    assert!(has(&head), "the bundle carries HEAD");
    assert!(
        !has(&hidden_head),
        "the hidden commit stays behind, only named"
    );
}

// A fenced session's changed-checks goes out as the person, and an
// evaluator, a machine nearby or this one would evaluate as them, so with
// `--lent-only` it goes to a lent builder, from HEAD alone, or nowhere.
#[test]
fn changed_checks_kept_to_the_lent_builders_goes_nowhere_else() {
    let run = RunTree::new();
    let changed = [
        "--lent-only",
        "--changed",
        "--build",
        "--no-vm",
        "--system",
        "x86_64-linux",
    ];
    fs::write(run.tree.join("package.json"), "{ \"name\": \"changed\" }").unwrap();
    let ran = run.there(&changed, &[]);
    assert_eq!(ran.status.code(), Some(69), "never run here: {ran:?}");
    assert!(!run.bin.join("ssh-was-given").exists(), "it went: {ran:?}");
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("commit them first"),
        "{ran:?}"
    );
    run.git(&["checkout", "-q", "--", "package.json"]);

    let ran = run.there(&changed, &[]);
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(run.sent().as_deref(), Some(run.head.as_str()), "from HEAD");
    fs::remove_dir_all(run.folder.path().join("nix-offload/passed")).unwrap();

    // With none lent, an evaluator that answers is never asked.
    fs::remove_file(run.offload.join("store")).unwrap();
    let answering = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = answering.local_addr().unwrap();
    fs::write(
        run.folder.path().join("nix-offload/evaluators"),
        format!("{address} far o\n"),
    )
    .unwrap();
    let ran = run.there(&changed, &[]);
    assert_eq!(ran.status.code(), Some(69), "never run here: {ran:?}");
    assert!(!run.bin.join("ssh-was-given").exists(), "it went: {ran:?}");
    drop(answering);

    // Nor is it built here, or sent as a folder's files.
    fs::write(run.offload.join("store"), STORE).unwrap();
    let here = run.there(&["--lent-only", "--here", ".", "checks.x.a"], &[]);
    assert_eq!(here.status.code(), Some(64), "{here:?}");
    let folder = format!("path:{}", run.tree.display());
    let files = run.there(&["--lent-only", &folder, "checks.x.a"], &[]);
    assert_ne!(files.status.code(), Some(0), "{files:?}");
    assert!(
        !run.bin.join("ssh-was-given").exists(),
        "it went: {files:?}"
    );
    assert!(
        String::from_utf8_lossy(&files.stderr).contains("never a folder's files"),
        "{files:?}"
    );
}
