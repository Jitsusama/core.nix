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
            "Jitsusama-core.nix-modules-repos",
        ),
        (Some("git@github.com:o/r"), "r", "", "o-r"),
        (
            Some("ssh://git@host.example/a/b.git/"),
            "b",
            "zone/tool",
            "a-b-zone-tool",
        ),
        (None, "tree", "", "tree"),
        (None, "tree", "x", "tree-x"),
        (
            Some("https://host.example/../x y"),
            "t",
            "..",
            "r-..-x-y-..",
        ),
        (None, ".hidden", "", "r-.hidden"),
        (None, "", "", "r-"),
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
        120,
        "a long name wasn't cut"
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

/// The shells a builder's might be: dash, as most are, and the `sh` here.
fn shells() -> Vec<&'static str> {
    ["sh", "dash"]
        .into_iter()
        .filter(|shell| {
            Command::new(shell)
                .args(["-c", "true"])
                .status()
                .is_ok_and(|status| status.success())
        })
        .collect()
}

/// `WARM` for the key `k` run by `shell` with `home` as its home, running
/// `script` with sh in the target it takes, `vars` set too.
fn warmed(shell: &str, home: &Path, script: &str, vars: &[(&str, &str)]) -> Command {
    let mut warm = Command::new(shell);
    warm.args(["-c", WARM, "nix-offload-warm", "k", "sh", "-c", script])
        .env("HOME", home)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_BUILD_DIR")
        .env_remove("NIX_OFFLOAD_WARM_ROOM_KB")
        .envs(vars.iter().copied());
    warm
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
    for shell in shells() {
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
    for shell in shells() {
        let home = tempfile::tempdir().unwrap();
        let started = home.path().join("started");
        let mut first = warmed(
            shell,
            home.path(),
            &format!("touch {}; sleep 5", started.display()),
            &[],
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
        let began = std::time::Instant::now();
        while !started.exists() {
            assert!(
                began.elapsed() < std::time::Duration::from_secs(10),
                "{shell}: the first never began"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let (beside, _) = said(warmed(
            shell,
            home.path(),
            "echo \"$CARGO_TARGET_DIR\"",
            &[],
        ));
        let _ = first.kill();
        let _ = first.wait();
        assert_eq!(
            beside,
            format!("{}/target\n", slot(home.path(), 1)),
            "{shell}: a run took the target another holds"
        );
    }
}

// A builder's disk holds its Nix store too, so a target is begun afresh
// when the disk is short of room, rather than growing into the store's.
#[test]
fn a_target_begins_afresh_when_the_builder_is_short_of_room() {
    for shell in shells() {
        let home = tempfile::tempdir().unwrap();
        let leave = "mkdir -p \"$CARGO_TARGET_DIR\"; touch \"$CARGO_TARGET_DIR/kept\"";
        said(warmed(shell, home.path(), leave, &[]));
        let (after, told) = said(warmed(
            shell,
            home.path(),
            "ls \"$CARGO_TARGET_DIR\" 2>/dev/null; echo \"$CARGO_TARGET_DIR\"",
            &[("NIX_OFFLOAD_WARM_ROOM_KB", "999999999999")],
        ));
        assert_eq!(
            after,
            format!("{}/target\n", slot(home.path(), 0)),
            "{shell}: the target wasn't begun afresh"
        );
        assert!(told.contains("short of room"), "{shell}: {told}");
    }
}

#[test]
fn a_target_the_run_names_is_kept() {
    for shell in shells() {
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
