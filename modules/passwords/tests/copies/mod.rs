//! Copies the daemon makes, as the shell asks for them: none while no
//! copier is named, and then refused before any secret is read; a copy
//! marked sensitive, held in the foreground and cleared when its time is
//! up; and that time 30 s unless told otherwise, as the fence's terms for
//! copying ask. A copy copied over is ended at once, and its clear neither
//! ends the copy after it nor waits on to its own time. A copy goes with
//! the daemon however it ends, since the shell starts a new one at once,
//! which knows of no copy to clear, and Joel would read the password as
//! gone while it could still be pasted.
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
    // Answered in the copier's time, though a child it left behind still
    // holds its standard error.
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    holds_no_secret(&refused);
    // It's ended, with nothing written to it.
    until("the copier is ended", || !copier_runs(&running.copied()));
    let given = std::fs::read_to_string(running.copied()).unwrap_or_default();
    assert!(
        given.is_empty(),
        "a copier that never said it was ready was given {given:?}"
    );
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
    let early = std::fs::read(running.copied().with_extension("early"))
        .expect("the copier looked for what it was given early");
    assert!(
        early.is_empty(),
        "the copier was given {} bytes before it was ready",
        early.len()
    );
    let held = std::fs::read_to_string(running.copied()).unwrap_or_default();
    assert_eq!(held, "hunter2", "the copy isn't held whole");
}

#[test]
fn a_copier_that_never_says_it_holds_the_copy_is_refused() {
    let running = Running::with_env(
        "copy-unheld",
        &[("keeping", KEEPING)],
        &[("COPIER_UNHELD", "1")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("never said it held the copy")),
        "an unheld copy didn't say why it was refused: {refused}"
    );
    holds_no_secret(&refused);
    until("the unheld copier is ended", || {
        !copier_runs(&running.copied())
    });
}

/// A manager knowing one login whose password, 128 KiB long, is past what
/// a pipe holds, so a copier that never reads it can't take it all.
const LONG: &str = r#"
echo '{"state":"answering"}'
long=$(printf '%131072s' '' | tr ' ' x)
github='{"id":"gh","title":"GitHub","category":"login","version":1'
password='{"id":"password","label":"password","type":"hidden","purpose":"password"'
while read -r line; do
  [[ $line =~ \"id\":([0-9]+) ]] && id=${BASH_REMATCH[1]}
  [[ $line =~ \"ask\":\"([a-z-]+)\" ]] && ask=${BASH_REMATCH[1]}
  case $ask in
    list) printf '{"id":%s,"entries":[%s}]}\n' "$id" "$github" ;;
    get) printf '{"id":%s,"entry":%s,"details":[%s,"value":"%s"}]}}\n' \
      "$id" "$github" "$password" "$long" ;;
    reveal) printf '{"id":%s,"value":"%s"}\n' "$id" "$long" ;;
  esac
done
"#;

#[test]
fn a_copier_that_never_takes_the_copy_is_refused_in_its_time() {
    let running = Running::with_env("copy-deaf", &[("long", LONG)], &[("COPIER_DEAF", "1")]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let refused = client.ask(&json!({"id": 2, "ask": "copy", "key": "entry:long:gh"}));
    assert_eq!(refused["error"]["code"], "refused", "{refused}");
    assert!(
        refused["error"]["message"]
            .as_str()
            .is_some_and(|said| said.contains("never took the whole copy")),
        "a copy the copier never took didn't say why it was refused: {refused}"
    );
    until("the deaf copier is ended", || {
        !copier_runs(&running.copied())
    });
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

#[test]
fn a_copy_copied_over_stays_past_the_time_the_one_before_it_clears() {
    // Each copy clears 5 s on and the second is asked 2 s after the first,
    // so between their two times only the first one's clear could end the
    // second, and it's held past the first's time by half a second.
    let running = Running::with_env(
        "copy-copied-over",
        &[("keeping", KEEPING)],
        &[("PASSWORDS_CLEARS_MS", "5000")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "list", "asked": true}));
    let (first, first_clears) = held(&running, &mut client, 2);
    // Two of the first copy's 5 s pass, so the second's time falls 2 s past it.
    std::thread::sleep(std::time::Duration::from_secs(2));
    let (second, second_clears) = held(&running, &mut client, 3);
    assert!(
        !copier_pids(&running.copied()).contains(&first),
        "the copy copied over still runs"
    );
    while since_the_epoch() < first_clears + 500 {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let looked = since_the_epoch();
    assert!(
        looked < second_clears,
        "the second copy's own time came first, {} ms on",
        looked - second_clears
    );
    assert!(
        copier_pids(&running.copied()).contains(&second),
        "the copy before it cleared the copy copied over it"
    );
    until("the second copy clears in its own time", || {
        !copier_runs(&running.copied())
    });
}

#[test]
fn a_copy_copied_over_stops_waiting_to_clear() {
    // Each copy waits to clear on a thread of its own. The one copied over
    // was ended as it was, so its thread ends at its next look rather than
    // waking on to its own ten minutes: the daemon's threads fall back to
    // what they were with one copy held.
    let running = held_copy("copy-stops-waiting");
    let mut client = running.client();
    // The thread that gave the copier its copy ends once it's given.
    std::thread::sleep(std::time::Duration::from_secs(1));
    let holding_one = threads_of(running.pid());
    held(&running, &mut client, 3);
    until("the copy copied over stops waiting to clear", || {
        threads_of(running.pid()) <= holding_one
    });
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

/// The copier of a copy asked now, once it's the one copier holding the
/// password, and when the daemon says that copy clears.
fn held(running: &Running, client: &mut crate::common::Client, id: u64) -> (u32, u64) {
    let before = copier_pids(&running.copied());
    let copied = client.ask(&json!({"id": id, "ask": "copy", "key": "entry:keeping:gh"}));
    assert_eq!(copied["copied"], true, "{copied}");
    let clears = copied["clears"]
        .as_u64()
        .expect("the copy's answer says when it clears");
    until("one new copier holds the password", || {
        let holding = copier_pids(&running.copied());
        holding.len() == 1 && !before.contains(&holding[0])
    });
    (copier_pids(&running.copied())[0], clears)
}

/// How many threads a process runs.
fn threads_of(pid: u32) -> usize {
    std::fs::read_dir(format!("/proc/{pid}/task"))
        .expect("the daemon runs")
        .count()
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
    !copier_pids(copied).is_empty()
}

/// The stand-in copiers holding the clipboard.
fn copier_pids(copied: &Path) -> Vec<u32> {
    let wanted = copied.to_string_lossy().into_owned();
    std::fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter(|process| {
            std::fs::read(process.path().join("environ"))
                .is_ok_and(|environ| String::from_utf8_lossy(&environ).contains(&wanted))
                && std::fs::read_to_string(process.path().join("cmdline"))
                    .is_ok_and(|line| line.contains("copier"))
        })
        .filter_map(|process| process.file_name().to_str()?.parse().ok())
        .collect()
}
