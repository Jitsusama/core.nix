//! `nix-offload there` run as a caller runs it, with stand-ins for ssh and
//! nix on its PATH that record what they were given.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// ssh as the builder answers: it writes its verbose log where `-E` says,
/// records every word it was given and its last two (the host and the
/// command) apart, prints what the `says` file beside it holds and, on
/// stderr, what the `complains` file holds, and exits as the `status` file
/// beside it says. Once a test writes
/// `refuses-name-only`, a command asking `--name-only` is refused with 64,
/// as a changed-checks from before it was refuses it.
const SSH: &str = r#"#!/bin/sh
here=$(dirname "$0")
echo "$@" > "$here/ssh-options"
while [ "$#" -gt 2 ]; do
  if [ "$1" = -E ]; then log=$2; fi
  shift
done
printf '%s\n%s\n' "$1" "$2" > "$here/ssh-was-given"
case "$2" in
  *--name-only*) [ -f "$here/refuses-name-only" ] && exit 64 ;;
esac
echo "${WORK_RUNS_ELSEWHERE:-}" > "$here/ssh-marked"
echo "Transferred: sent 3812, received 9 bytes, in 1.0 seconds" > "$log"
[ -f "$here/says" ] && cat "$here/says"
[ -f "$here/complains" ] && cat "$here/complains" >&2
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

/// nix as it builds here: it records its arguments and exits 0, or as
/// the `nix-status` file beside it says.
const NIX: &str = r#"#!/bin/sh
echo "$@" > "$(dirname "$0")/nix-was-given"
echo "${WORK_RUNS_ELSEWHERE:-}" > "$(dirname "$0")/nix-marked"
exit "$(cat "$(dirname "$0")/nix-status" 2>/dev/null || echo 0)"
"#;

const STORE: &str = "ssh-ng://nix-offload?ssh-key=/key&\
                     base64-ssh-public-host-key=c3NoLWVkMjU1MTkgQUFBQQ==\n";

fn stand_ins(folder: &Path, status: u8) {
    for (name, script) in [("ssh", SSH), ("nix", NIX)] {
        let path = folder.join(name);
        executable(&path, script);
    }
    fs::write(folder.join("status"), status.to_string()).unwrap();
}

fn there(folder: &Path, offload: &Path) -> Output {
    there_with(folder, offload, &[])
}

/// `there` given `extra` arguments besides.
fn there_with(folder: &Path, offload: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args([
            "there",
            "-L",
            "github:o/r/abc",
            "checks.x.a",
            "--keep-going",
        ])
        .args(extra)
        .env(
            "PATH",
            format!("{}:{}", folder.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", offload)
        .env("XDG_RUNTIME_DIR", folder)
        .env("XDG_STATE_HOME", folder)
        .env("XDG_CONFIG_HOME", folder)
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
        .split_once("trap 'exit 143' HUP TERM; ")
        .and_then(|(_, rest)| rest.strip_suffix("; exit $?"))
        .unwrap_or_else(|| panic!("it doesn't end with its caller: {command}"))
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
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args([
            "there",
            "-L",
            &format!("path:{}", copy.display()),
            "checks.x.a",
        ])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", &offload)
        .env("XDG_RUNTIME_DIR", folder.path())
        .env("XDG_STATE_HOME", folder.path())
        .env("XDG_CONFIG_HOME", folder.path())
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
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args(["there", &format!("path:{}", tree.display()), "checks.x.a"])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", &offload)
        .env("XDG_RUNTIME_DIR", folder.path())
        .env("XDG_STATE_HOME", folder.path())
        .env("XDG_CONFIG_HOME", folder.path())
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
    fs::write(bin.join("status"), "0").unwrap();
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
    // Its inputs can't go anywhere, so it builds here only when asked to.
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args(["there", "-L", "--here", "--out"])
        .arg(&out)
        .args([flake.as_str(), "packages.x.guide"])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", &offload)
        .env("XDG_RUNTIME_DIR", folder.path())
        .env("XDG_STATE_HOME", folder.path())
        .env("XDG_CONFIG_HOME", folder.path())
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert!(!bin.join("ssh-was-given").exists(), "not sent");
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        said.contains("takes mint, secrets from this machine alone"),
        "{said}"
    );
    assert!(said.contains("the build said this"), "{said}");
    assert_eq!(fs::read_to_string(out.join("guide")).unwrap(), "a guide");
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
    assert!(
        String::from_utf8_lossy(&ran.stderr).contains("waits for a builder to stand again"),
        "{ran:?}"
    );
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
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args([
            "there",
            "-L",
            "--here",
            &flake,
            "checks.aarch64-darwin.a",
            "checks.x86_64-linux.b",
        ])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", &offload)
        .env("XDG_RUNTIME_DIR", folder.path())
        .env("XDG_STATE_HOME", folder.path())
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
    let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
        .args([
            "there",
            &format!("path:{}", tree.display()),
            "checks.aarch64-darwin.a",
        ])
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("NIX_OFFLOAD_DIR", &offload)
        .env("XDG_RUNTIME_DIR", folder.path())
        .env("XDG_STATE_HOME", folder.path())
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
        let ran = Command::new(env!("CARGO_BIN_EXE_nix-offload"))
            .args(["there", "-L", &self.tree.to_string_lossy(), "checks.x.a"])
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("NIX_OFFLOAD_DIR", &self.offload)
            .env("XDG_RUNTIME_DIR", self.folder.path())
            .env("XDG_STATE_HOME", self.folder.path())
            .env("XDG_CONFIG_HOME", self.folder.path())
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
        "tree\napply\nout\nwants\nfolder\nchanged\nhere\nprint\n"
    );
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
        Command::new(env!("CARGO_BIN_EXE_nix-offload"))
            .args(["there", "--changed", "--build", "--no-vm"])
            .args(["--system", "x86_64-linux"])
            .current_dir(&tree)
            .env(
                "PATH",
                format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
            )
            .env("NIX_OFFLOAD_DIR", &offload)
            .env("XDG_RUNTIME_DIR", folder.path())
            .env("XDG_STATE_HOME", folder.path())
            .env("XDG_CONFIG_HOME", folder.path())
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
