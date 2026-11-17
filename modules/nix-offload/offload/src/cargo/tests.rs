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
