//! Copies the daemon makes, as the shell asks for them: none while no
//! copier is named, and then refused before any secret is read; a copy
//! marked sensitive, held in the foreground and cleared when its time is
//! up; and that time 30 s unless told otherwise, as the fence's terms for
//! copying ask. A copy goes with the daemon however it ends, since the
//! shell starts a new one at once, which knows of no copy to clear, and
//! Joel would read the password as gone while it could still be pasted.
//! The copier is given the secret only once it says it's ready, having
//! shut the account's other processes out of its memory, and a copy is
//! answered as copied only once it says it holds it: one it refuses is
//! answered with its reason, and one that says nothing is ended unwritten.

use std::path::Path;

use crate::common::{KEEPING, Running, until};
use crate::holds_no_secret;
use serde_json::json;

#[test]
fn no_copy_is_made_while_no_copier_is_named() {
    let running = Running::with_env(
        "uncopied",
        &[("keeping", KEEPING)],
        &[("PASSWORDS_COPIER", "")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("browser extension")),
        "a refused copy didn't say what to use instead: {refused}"
    );
    holds_no_secret(&refused);
    // Refused before any secret was read: the manager was asked no reveal.
    let asked = running.asked_of("keeping");
    assert!(
        !asked.contains("reveal"),
        "a secret was read to copy: {asked}"
    );
    assert!(!running.copied().exists(), "something was copied");
}

#[test]
fn a_copy_is_marked_sensitive_and_cleared_when_its_time_is_up() {
    let running = Running::with("copy", &[("keeping", KEEPING)]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    holds_no_secret(&copied);
    // The copier writes what it's given, then its flags, so both are
    // waited for: reading the flags once the copy is there races it.
    let flagged = running.copied().with_extension("flags");
    until("the password is copied with its flags", || {
        std::fs::read_to_string(running.copied()).is_ok_and(|copied| copied == "hunter2")
            && std::fs::read_to_string(&flagged).is_ok_and(|flags| flags.ends_with('\n'))
    });
    let flags = std::fs::read_to_string(&flagged).unwrap();
    assert!(
        flags.contains("--sensitive")
            && flags.contains("--foreground")
            && flags.contains("--say-held"),
        "{flags}"
    );
    // The copier is ended once its time is up, which clears the clipboard.
    until("the copy is cleared", || !copier_runs(&running.copied()));
}

#[test]
fn a_refused_copy_is_answered_with_its_reason() {
    let running = Running::with_env(
        "copy-refused",
        &[("keeping", KEEPING)],
        &[("COPIER_REFUSES", "it is past 1 MiB")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("it is past 1 MiB")),
        "a refused copy didn't say the copier's reason: {refused}"
    );
    holds_no_secret(&refused);
}

#[test]
fn a_copier_that_never_says_its_ready_is_given_nothing() {
    let running = Running::with_env(
        "copy-unready",
        &[("keeping", KEEPING)],
        &[("COPIER_SILENT", "1")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    holds_no_secret(&refused);
    // It's ended, with nothing written to it.
    until("the copier is ended", || !copier_runs(&running.copied()));
    let given = std::fs::read_to_string(running.copied()).unwrap_or_default();
    assert!(given.is_empty(), "a copier that never said it was ready was given {given:?}");
}

#[test]
fn nothing_is_written_to_the_copier_before_its_ready() {
    let running = Running::with_env(
        "copy-waits",
        &[("keeping", KEEPING)],
        &[("COPIER_WAITS", "1"), ("PASSWORDS_CLEARS_MS", "600000")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    // The copier writes its flags once it has read all it was given.
    until("the copier has read the copy", || {
        running.copied().with_extension("flags").exists()
    });
    let early = std::fs::read(running.copied().with_extension("early")).unwrap_or_default();
    assert!(early.is_empty(), "the copier was given {} bytes before it was ready", early.len());
    let held = std::fs::read_to_string(running.copied()).unwrap_or_default();
    assert_eq!(held, "hunter2", "the copy isn't held whole");
}

#[test]
fn a_copy_clears_in_thirty_seconds_unless_told_otherwise() {
    // An empty PASSWORDS_CLEARS_MS reads as none, so the daemon keeps its
    // own time, which the answer's clear time shows without waiting it out.
    let running = Running::with_env(
        "copy-clears",
        &[("keeping", KEEPING)],
        &[("PASSWORDS_CLEARS_MS", "")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let asked = since_the_epoch();
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    let answered = since_the_epoch();
    let clears = copied["clears"]
        .as_u64()
        .expect("the copy's answer says when it clears");
    assert!(
        (asked + 30_000..=answered + 30_000).contains(&clears),
        "the copy clears {} ms after it was asked, not 30 s: {copied}",
        i128::from(clears) - i128::from(asked)
    );
}

#[test]
fn a_copy_goes_as_the_daemon_ends() {
    let mut running = held_copy("copy-ends");
    running.close();
    let started = std::time::Instant::now();
    while !running.ended() {
        assert!(
            started.elapsed() < std::time::Duration::from_secs(20),
            "the daemon outlived its channel"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    until("the copy goes with it", || !copier_runs(&running.copied()));
}

#[test]
fn a_copy_goes_as_the_daemon_is_killed() {
    let mut running = held_copy("copy-killed");
    running.kill();
    until("the copy goes with it", || !copier_runs(&running.copied()));
}

/// A daemon holding a copy far longer than a test takes, so only its
/// ending can end the copy.
fn held_copy(test: &str) -> Running {
    let running = Running::with_env(
        test,
        &[("keeping", KEEPING)],
        &[("PASSWORDS_CLEARS_MS", "600000")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let copied = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    until("the copier holds the password", || {
        copier_runs(&running.copied())
            && std::fs::read_to_string(running.copied()).is_ok_and(|held| held == "hunter2")
    });
    running
}

/// Milliseconds since the epoch, as the daemon tells a clear time.
fn since_the_epoch() -> u64 {
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is past the epoch");
    u64::try_from(since.as_millis()).expect("the time fits in 64 bits")
}

/// Whether the stand-in copier still holds the clipboard.
fn copier_runs(copied: &Path) -> bool {
    let wanted = copied.to_string_lossy().into_owned();
    std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .any(|process| {
            std::fs::read(process.path().join("environ"))
                .is_ok_and(|environ| String::from_utf8_lossy(&environ).contains(&wanted))
                && std::fs::read_to_string(process.path().join("cmdline"))
                    .is_ok_and(|line| line.contains("copier"))
        })
}
