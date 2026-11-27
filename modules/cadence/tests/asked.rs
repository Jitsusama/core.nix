//! `cadence axes --asked WORDS` over a stand-in account: the asks list read
//! against what the hour saw of it, as cadence-spec reads it for the
//! launcher's Asked For, so a list written over is never served as most of
//! Joel's asks gone (25 November's overwrite).

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};

const WORDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json");

/// `cadence axes --asked` run over the account in `folder`, with nothing
/// it reads by command able to run, so only the files there count.
fn asked(folder: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cadence"))
        .args(["axes", "--asked", WORDS])
        .env("HOME", folder)
        .env("XDG_STATE_HOME", folder.join("state"))
        .env("XDG_DATA_HOME", folder.join("data"))
        .env("PATH", "")
        .output()
        .expect("cadence runs")
}

/// An asks list holding `count` open asks of Joel's.
fn asks(count: usize) -> String {
    (0..count)
        .map(|at| {
            json!({"id": format!("a{at}"), "at": "2026-11-26T09:00:00-05:00",
                "words": format!("ask {at}"), "answers": "Y2YBTS"})
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_asks_list_is_read_against_what_the_hour_saw_of_it() {
    let account = tempfile::tempdir().expect("a stand-in account");
    let folder = account.path();
    let list = folder.join("state/mission/asks.jsonl");
    let hour = folder.join("state/cadence/hour.json");

    let ran = asked(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        ran.status.success(),
        "a list never seen and not there: {said}"
    );
    let made: Value = serde_json::from_slice(&ran.stdout).expect("one line of JSON");
    assert_eq!(
        made["head"]["said"], "nothing you asked for is open",
        "{made}"
    );

    fs::create_dir_all(list.parent().expect("its folder")).expect("a mission folder");
    fs::create_dir_all(hour.parent().expect("its folder")).expect("cadence's folder");
    fs::write(&list, asks(5)).expect("five asks");
    let ran = asked(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        !ran.status.success() && said.contains("no record"),
        "a list with no record of the hour's to read it against is unread: {said}"
    );
    let seen = json!({"asks": {"most": 5}}).to_string();
    fs::write(&hour, &seen).expect("the hour saw five");
    let ran = asked(folder);
    assert!(
        ran.status.success(),
        "{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    let made: Value = serde_json::from_slice(&ran.stdout).expect("one line of JSON");
    assert_eq!(
        made["rows"].as_array().map(Vec::len),
        Some(5),
        "each open ask a row: {made}"
    );

    fs::write(&list, asks(2)).expect("the list written over");
    let ran = asked(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        !ran.status.success() && ran.stdout.is_empty(),
        "a list holding fewer asks than the hour saw is unread, so the last rows stay: {said}"
    );
    assert!(said.contains("unread"), "it says why: {said}");
    assert_eq!(
        fs::read_to_string(&hour).expect("the hour's record"),
        seen,
        "Asked For writes nothing of what the hour saw"
    );

    // Short for a minute by the hour's own clock, in seconds, it's still
    // unread; short past the hour's two hours, it's taken as pruned.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("a clock")
        .as_secs();
    let short = |since: u64| json!({"asks": {"most": 5, "short_since": since}}).to_string();
    fs::write(&hour, short(now - 60)).expect("short a minute");
    let ran = asked(folder);
    assert!(
        !ran.status.success(),
        "short a minute, still unread: {}",
        String::from_utf8_lossy(&ran.stdout)
    );
    fs::write(&hour, short(now - 3 * 3600)).expect("short three hours");
    let ran = asked(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(ran.status.success(), "short three hours, pruned: {said}");
    let made: Value = serde_json::from_slice(&ran.stdout).expect("one line of JSON");
    assert_eq!(made["rows"].as_array().map(Vec::len), Some(2), "{made}");

    fs::write(&list, asks(5)).expect("the list whole");
    fs::write(&hour, "{").expect("the hour's record torn");
    let ran = asked(folder);
    let said = String::from_utf8_lossy(&ran.stderr);
    assert!(
        !ran.status.success() && ran.stdout.is_empty(),
        "with the hour's record unreadable, the list is unread, never read against nothing: \
         {said}"
    );
}
