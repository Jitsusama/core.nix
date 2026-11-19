use std::fs;

use super::*;

fn words(line: &str) -> Vec<String> {
    line.split(' ').map(str::to_owned).collect()
}

#[test]
fn a_whole_suite_or_clippy_goes_there() {
    // At 04:15 on 16 November a whole crate's cargo test ran here while
    // the lent builders sat idle.
    for line in [
        "test --manifest-path modules/repos/Cargo.toml",
        "test",
        "+nightly test --all-targets",
        "t -p disk-guard",
        "test --manifest-path m/Cargo.toml -- --nocapture --test-threads 4",
        "clippy --manifest-path m/Cargo.toml --all-targets -- -D warnings -W clippy::pedantic",
        "--color never test --features a,b",
        "bench",
    ] {
        assert_eq!(
            way(&words(line)),
            Way::There,
            "cargo {line} didn't go there"
        );
    }
}

#[test]
fn an_offline_run_goes_to_a_lent_builder_online_and_locked() {
    // At 17:45 on 16 November an agent's cargo test --offline went to a lent
    // builder, which has no crates but what it fetches, and failed finding
    // one.
    for (line, sent, offline) in [
        ("test --offline", "test --locked", true),
        (
            "--offline test -p disk-guard",
            "test -p disk-guard --locked",
            true,
        ),
        (
            "clippy --frozen --all-targets",
            "clippy --all-targets --locked",
            true,
        ),
        ("test --locked --frozen", "test --locked", true),
        ("test --frozen --locked", "test --locked", true),
        ("test --config net.offline=true", "test --locked", true),
        (
            "test --config=net.offline=true -p a",
            "test -p a --locked",
            true,
        ),
        (
            "test --config net.retry=4",
            "test --config net.retry=4",
            false,
        ),
        (
            "test --offline -- --nocapture --offline",
            "test --locked -- --nocapture --offline",
            true,
        ),
        ("test --locked", "test --locked", false),
        ("test --locked --locked", "test --locked", false),
        ("test", "test", false),
    ] {
        assert_eq!(
            for_a_lender(&words(line), None),
            (words(sent), offline),
            "cargo {line} wasn't sent as cargo {sent}, asked offline {offline}"
        );
    }
    // CARGO_NET_OFFLINE here doesn't go there, so a run it sets to true
    // goes locked too. Any other value leaves the run's words as they are.
    for (value, sent, offline) in [
        (Some("true"), "test -p a --locked", true),
        (Some("false"), "test -p a", false),
        (Some("yes"), "test -p a", false),
        (Some(""), "test -p a", false),
        (None, "test -p a", false),
    ] {
        assert_eq!(
            for_a_lender(&words("test -p a"), value),
            (words(sent), offline),
            "a run with CARGO_NET_OFFLINE {value:?} went as cargo {sent}, asked offline {offline}"
        );
    }
    assert_eq!(
        for_a_lender(&words("test --locked"), Some("true")),
        (words("test --locked"), true),
        "a locked run asked offline by the environment wasn't sent locked once"
    );
}

#[test]
fn a_named_test_runs_here_at_once() {
    for line in [
        "test --manifest-path m/Cargo.toml --test guard -- survey:: quest_temp::",
        "test -p disk-guard sizes",
        "test --bin nix-offload replay::",
        "t --lib -- --exact a_case",
    ] {
        assert_eq!(
            way(&words(line)),
            Way::As,
            "cargo {line} didn't run here as asked"
        );
    }
}

#[test]
fn what_builds_for_here_waits_its_turn() {
    for line in [
        "build --manifest-path m/Cargo.toml --bins",
        "check",
        "doc -p x",
    ] {
        assert!(
            matches!(way(&words(line)), Way::Turn(_)),
            "cargo {line} didn't wait its turn here"
        );
    }
}

#[test]
fn what_a_program_reads_or_a_light_command_runs_as_asked() {
    // An editor's checks read cargo's json as it goes, so they can't wait
    // or go.
    for line in [
        "check --workspace --message-format=json-diagnostic-rendered-ansi",
        "clippy --message-format json",
        "fmt --manifest-path m/Cargo.toml",
        "metadata --format-version 1",
        "--version",
        "",
    ] {
        assert_eq!(
            way(&words(line)),
            Way::As,
            "cargo {line} didn't run as asked"
        );
    }
}

#[test]
fn words_a_shell_would_read_differently_stay_here() {
    let asked = vec!["test".to_owned(), "--features".to_owned(), "a b".to_owned()];
    assert!(
        matches!(way(&asked), Way::Turn(_)),
        "a word with a space went there, where its shell would split it"
    );
}

#[test]
fn a_suite_goes_only_where_a_builder_stands_and_the_tree_is_committed() {
    assert_eq!(going(true, true), Way::There, "a committed suite stayed");
    assert!(
        matches!(going(false, true), Way::Turn(_)),
        "a suite went with no builder standing"
    );
    assert!(
        matches!(going(true, false), Way::Turn(_)),
        "a suite went with changes a builder's copy wouldn't hold"
    );
}

#[test]
fn a_warm_target_is_named_for_its_repository_and_folder() {
    for (origin, top, dir, key) in [
        (
            Some("https://github.com/Jitsusama/core.nix.git"),
            "core.nix",
            "modules/repos",
            "github.com-Jitsusama-core.nix-modules-repos-a83c86fc",
        ),
        (
            Some("git@github.com:o/r"),
            "r",
            "",
            "github.com-o-r-c78b735a",
        ),
        (
            Some("ssh://git@host.example/a/b.git/"),
            "b",
            "zone/tool",
            "host.example-a-b-zone-tool-71c39923",
        ),
        (None, "tree", "", "tree-5b25b8ef"),
        (None, "tree", "x", "tree-x-d4aff97f"),
        (
            Some("https://host.example/../x y"),
            "t",
            "..",
            "host.example-..-x-y-..-c390aad3",
        ),
        (None, ".hidden", "", "r-.hidden-28280da0"),
        (None, "", "", "r--cbf29ce4"),
    ] {
        assert_eq!(
            warm_key(origin, top, dir),
            key,
            "{origin:?} at {dir:?} in {top:?}"
        );
    }
    let long = "a".repeat(300);
    assert_eq!(
        warm_key(None, &long, "").len(),
        109,
        "a long name wasn't cut"
    );
    assert_ne!(
        warm_key(Some("git@h:o/r-x"), "r-x", ""),
        warm_key(Some("git@h:o-r"), "o-r", "x"),
        "two repositories that read alike once dashed shared a target"
    );
    assert_ne!(
        warm_key(Some("https://a.example/o/r"), "r", ""),
        warm_key(Some("https://b.example/o/r"), "r", ""),
        "one path on two hosts shared a target"
    );
}

// The cargo here names its warm target from the tree it's run in; a
// folder reached through a link is named for the real one, as git names
// its top.
#[test]
fn the_folder_here_is_named_for_its_origin_and_place() {
    let folder = tempfile::tempdir().unwrap();
    let top = folder.path().join("tree");
    fs::create_dir_all(top.join("zone/tool")).unwrap();
    let git = |words: &[&str]| {
        let ran = Command::new("git")
            .arg("-C")
            .arg(&top)
            .args(words)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(ran.status.success(), "git {words:?}: {ran:?}");
    };
    git(&["init", "-q"]);
    let linked = folder.path().join("linked");
    std::os::unix::fs::symlink(&top, &linked).unwrap();
    assert_eq!(
        key_in(&linked.join("zone/tool")),
        warm_key(None, "tree", "zone/tool"),
        "a tree with no origin wasn't named for its top"
    );
    git(&["remote", "add", "origin", "git@github.com:o/r.git"]);
    assert_eq!(
        key_in(&top.join("zone/tool")),
        warm_key(Some("git@github.com:o/r.git"), "tree", "zone/tool"),
    );
    assert_eq!(
        key_in(&top),
        warm_key(Some("git@github.com:o/r.git"), "tree", ""),
    );
}

#[test]
fn a_suite_goes_there_through_its_warm_target() {
    assert_eq!(
        sent_there("k", &words("test --locked")),
        [
            "sh",
            "-c",
            WARM,
            "nix-offload-warm",
            "k",
            "nix",
            "develop",
            "-c",
            "cargo",
            "test",
            "--locked",
        ]
        .map(str::to_owned),
    );
}

/// The shells a builder's might be: dash, as most are, which the package
/// gives its tests, and the `sh` here.
const SHELLS: [&str; 2] = ["sh", "dash"];

/// `WARM` for the key `k` run by `shell` with `home` as its home, running
/// `script` with sh in the target it takes, `vars` set too. Its disk is
/// never short of room unless `vars` says so, so the tests pass on a disk
/// of any size.
fn warmed(shell: &str, home: &Path, script: &str, vars: &[(&str, &str)]) -> Command {
    warmed_for("k", shell, home, script, vars)
}

/// [`warmed`] for the key `key`.
fn warmed_for(key: &str, shell: &str, home: &Path, script: &str, vars: &[(&str, &str)]) -> Command {
    let mut warm = Command::new(shell);
    warm.args(["-c", WARM, "nix-offload-warm", key, "sh", "-c", script])
        .env("HOME", home)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env("NIX_OFFLOAD_WARM_ROOM_KB", "0")
        .envs(vars.iter().copied());
    warm
}

/// A run of `key` under `home` that holds its target until `done` is
/// made, once it has taken it.
fn holding(key: &str, shell: &str, home: &Path, done: &Path) -> std::process::Child {
    let took = home.join(format!("took-{key}"));
    let _ = fs::remove_file(&took);
    let script = format!(
        "touch '{}'; while [ ! -e '{}' ]; do sleep 0.05; done",
        took.display(),
        done.display()
    );
    let child = warmed_for(key, shell, home, &script, &[])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let began = std::time::Instant::now();
    while !took.exists() {
        assert!(
            began.elapsed() < std::time::Duration::from_secs(20),
            "{shell}: a run never took its target"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    child
}

/// Ends the runs `held`, holding until `done` was made.
fn release(done: &Path, held: Vec<std::process::Child>) {
    fs::write(done, "").unwrap();
    for mut child in held {
        child.wait().unwrap();
    }
}

/// What `warmed` said on each stream, once it ended 0.
fn said(mut warm: Command) -> (String, String) {
    let out = warm.output().unwrap();
    assert!(out.status.success(), "{out:?}");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Where the key `k`'s target `n` is under `home`.
fn slot(home: &Path, n: u8) -> String {
    home.join(format!(".cache/nix-offload/cargo/k/{n}"))
        .display()
        .to_string()
}

// Each suite a lent builder ran compiled every crate it reads afresh, in a
// folder gone when it ended: 658 crates for a large repository's zone on
// 19 November. A target kept there for each repository is warm for the
// next run, and a file a run left in it is there for the one after.
#[test]
fn a_run_builds_in_its_repositorys_warm_target_and_the_next_finds_it() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let script = "echo \"$CARGO_TARGET_DIR $CARGO_BUILD_BUILD_DIR\"; \
             ls \"$CARGO_TARGET_DIR\"; mkdir -p \"$CARGO_TARGET_DIR\"; \
             touch \"$CARGO_TARGET_DIR/kept\"";
        let (first, told) = said(warmed(shell, home.path(), script, &[]));
        let at = slot(home.path(), 0);
        assert_eq!(first, format!("{at}/target {at}/build\n"), "{shell}");
        assert!(told.contains("warm target 0 of k"), "{shell}: {told}");
        let (second, _) = said(warmed(shell, home.path(), script, &[]));
        assert_eq!(
            second,
            format!("{at}/target {at}/build\nkept\n"),
            "{shell}: the next run didn't find what the first left"
        );
    }
}

// Two suites of one repository at once on one builder would wait on
// cargo's lock of a shared target, so each takes one no other run holds.
#[test]
fn a_run_beside_another_takes_a_target_of_its_own() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let done = home.path().join("done");
        let first = holding("k", shell, home.path(), &done);
        let (beside, _) = said(warmed(
            shell,
            home.path(),
            "echo \"$CARGO_TARGET_DIR\"",
            &[],
        ));
        release(&done, vec![first]);
        assert_eq!(
            beside,
            format!("{}/target\n", slot(home.path(), 1)),
            "{shell}: a run took the target another holds"
        );
    }
}

// With every target of a key held, a run builds cold, as before, rather
// than wait for one.
#[test]
fn a_run_with_every_target_held_builds_cold() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let done = home.path().join("done");
        let held: Vec<_> = (0..4)
            .map(|_| holding("k", shell, home.path(), &done))
            .collect();
        let (out, told) = said(warmed(
            shell,
            home.path(),
            "echo \"${CARGO_TARGET_DIR-none}\"",
            &[],
        ));
        release(&done, held);
        assert_eq!(out, "none\n", "{shell}: it took a held target");
        assert!(
            told.contains("is free, so this builds cold"),
            "{shell}: {told}"
        );
    }
}

// A builder's disk holds its Nix store too, so when it's short of room the
// targets no run holds are cleared, other keys' first, and the run's own
// begins afresh only when that isn't enough. On 19 November a disk short
// of room cleared only the run's own target, so every run built cold
// while idle keys' targets kept the space.
#[test]
fn a_builder_short_of_room_clears_the_targets_no_run_holds() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let leave = "mkdir -p \"$CARGO_TARGET_DIR\"; touch \"$CARGO_TARGET_DIR/kept\"";
        for key in ["idle", "busy", "k"] {
            said(warmed_for(key, shell, home.path(), leave, &[]));
        }
        let done = home.path().join("done");
        let busy = holding("busy", shell, home.path(), &done);
        let (after, told) = said(warmed(
            shell,
            home.path(),
            "ls \"$CARGO_TARGET_DIR\" 2>/dev/null; echo \"$CARGO_TARGET_DIR\"",
            &[("NIX_OFFLOAD_WARM_ROOM_KB", "999999999999")],
        ));
        release(&done, vec![busy]);
        let cache = home.path().join(".cache/nix-offload/cargo");
        assert!(
            !cache.join("idle/0").exists(),
            "{shell}: an idle key's target kept the room: {told}"
        );
        assert!(
            cache.join("busy/0/target/kept").exists(),
            "{shell}: a target a run held was cleared: {told}"
        );
        assert_eq!(
            after,
            format!("{}/target\n", slot(home.path(), 0)),
            "{shell}: the run's own target wasn't begun afresh"
        );
        assert!(
            told.contains("clears warm target idle/0"),
            "{shell}: {told}"
        );
        assert!(told.contains("still short of room"), "{shell}: {told}");
    }
}

// A lock a builder can't open, or no flock there, builds cold, as before;
// in dash a redirection `exec` couldn't make ended the whole suite.
#[test]
fn a_target_that_cant_be_held_builds_cold() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join(".cache/nix-offload/cargo/k");
        fs::create_dir_all(&root).unwrap();
        for n in 0..4 {
            fs::create_dir(root.join(format!("{n}.lock"))).unwrap();
        }
        let (out, told) = said(warmed(
            shell,
            home.path(),
            "echo \"${CARGO_TARGET_DIR-none}\"",
            &[],
        ));
        assert_eq!(out, "none\n", "{shell}: {told}");
        assert!(told.contains("builds cold"), "{shell}: {told}");

        let bin = home.path().join("bin");
        fs::create_dir(&bin).unwrap();
        for tool in ["sh", "mkdir"] {
            let found = Command::new("sh")
                .args(["-c", &format!("command -v {tool}")])
                .output()
                .unwrap();
            let found = String::from_utf8_lossy(&found.stdout).trim().to_owned();
            std::os::unix::fs::symlink(found, bin.join(tool)).unwrap();
        }
        let shell_path = Command::new("sh")
            .args(["-c", &format!("command -v {shell}")])
            .output()
            .unwrap();
        let shell_path = String::from_utf8_lossy(&shell_path.stdout)
            .trim()
            .to_owned();
        let (out, told) = said(warmed(
            &shell_path,
            home.path(),
            "echo \"${CARGO_TARGET_DIR-none}\"",
            &[("PATH", &bin.to_string_lossy())],
        ));
        assert_eq!(out, "none\n", "{shell}: {told}");
        assert!(told.contains("has no flock"), "{shell}: {told}");
    }
}

#[test]
fn a_target_the_builder_names_is_kept() {
    for shell in SHELLS {
        let home = tempfile::tempdir().unwrap();
        let (out, _) = said(warmed(
            shell,
            home.path(),
            "echo \"$CARGO_TARGET_DIR\"",
            &[("CARGO_TARGET_DIR", "/named")],
        ));
        assert_eq!(out, "/named\n", "{shell}");
    }
}

#[test]
fn the_dev_shells_words_are_read_as_cargo_and_its_own() {
    let given = words("--cargo /nix/store/x-cargo/bin/cargo -- test -p a");
    assert_eq!(
        asked(&given),
        Some(("/nix/store/x-cargo/bin/cargo", &given[3..])),
        "the cargo or its words were misread"
    );
    assert_eq!(
        asked(&words("test")),
        None,
        "words with no cargo were taken"
    );
}
