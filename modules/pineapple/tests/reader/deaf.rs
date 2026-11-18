//! pineapple-slack telling Joel in For You when it can't hear Slack: a
//! sign-in Slack refuses, or none to read, writes the row, hearing Slack
//! again takes it away, and a Slack that can't be reached writes none.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::keyring::reading_with_keyring;
use super::{Answered, PATIENCE, answer, answering_whole, home, reader};

/// Where For You reads what the safe word waits on Joel for.
fn waiting(home: &Path) -> PathBuf {
    home.join("state/mission/waiting/pineapple.json")
}

/// Whether `file` comes to exist (`exists`) or goes within the patience.
fn until(file: &Path, exists: bool) -> bool {
    let began = Instant::now();
    while file.exists() != exists && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    file.exists() == exists
}

/// The row told, or nothing when none could be read.
fn row(home: &Path) -> Value {
    std::fs::read_to_string(waiting(home))
        .ok()
        .and_then(|said| serde_json::from_str(&said).ok())
        .unwrap_or_default()
}

/// Whether the row told says `why` first.
fn says(row: &Value, why: &str) -> bool {
    row["items"][0]["why"]
        .as_str()
        .is_some_and(|said| said.starts_with(why))
}

#[test]
fn a_refused_sign_in_is_told_in_for_you_until_slack_is_heard_again() {
    let refusing = Arc::new(AtomicBool::new(true));
    let refuses = Arc::clone(&refusing);
    let (origin, _) = answering_whole(move |line, now| {
        Answered::ok(if refuses.load(Ordering::SeqCst) {
            json!({"ok": false, "error": "invalid_auth"})
        } else {
            answer(line, now)
        })
    });
    let home = home();
    let mut reader = reader(&home, &origin);
    let told = until(&waiting(&home), true);
    let row = row(&home);
    refusing.store(false, Ordering::SeqCst);
    let untold = until(&waiting(&home), false);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(told, "Joel is told while Slack refuses the sign-in");
    assert!(
        row["items"][0]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("pineapple-slack-deaf-")),
        "the safe word's row: {row}"
    );
    assert!(
        says(&row, "Slack refused the safe word's sign-in"),
        "it says why: {row}"
    );
    assert!(untold, "and untold once Slack is heard again");
}

#[test]
fn a_refusal_in_either_search_is_told_though_the_other_cant_be_asked() {
    // The phrase's search is refused and the mention's can't be asked, so
    // the refusal, which Joel can mend, is what the round comes to.
    let (origin, _) = answering_whole(|line, _| {
        if line.contains("query=pineapple") {
            Answered::ok(json!({"ok": false, "error": "invalid_auth"}))
        } else {
            Answered {
                status: 500,
                headers: Vec::new(),
                body: json!({}),
            }
        }
    });
    let home = home();
    let mut reader = reader(&home, &origin);
    let told = until(&waiting(&home), true);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(told, "a refusal isn't hidden by a search that wasn't asked");
}

#[test]
fn no_sign_in_to_read_is_told_too() {
    let (origin, _) = answering_whole(|line, now| Answered::ok(answer(line, now)));
    let home = home();
    // Neither the keyring nor the older file holds one: signed out.
    std::fs::remove_file(home.join("config/agentic-harness/slack/credentials.json"))
        .expect("the file's sign-in taken away");
    let mut reader = reading_with_keyring(&home, &origin, "exit 1");
    let told = until(&waiting(&home), true);
    let row = row(&home);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(told, "Joel is told when there's no sign-in to read");
    assert!(
        says(&row, "The safe word has no Slack sign-in it can read"),
        "it says why: {row}"
    );
}

#[test]
fn a_slack_that_cant_be_reached_isnt_told_as_deaf() {
    let asked = Arc::new(AtomicUsize::new(0));
    let asks = Arc::clone(&asked);
    let (origin, _) = answering_whole(move |line, _| {
        if line.contains("search.messages") {
            asks.fetch_add(1, Ordering::SeqCst);
        }
        Answered {
            status: 500,
            headers: Vec::new(),
            body: json!({}),
        }
    });
    let home = home();
    let mut reader = reader(&home, &origin);
    // Three rounds of both searches, more than the two rounds that tell.
    let began = Instant::now();
    while asked.load(Ordering::SeqCst) < 6 && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    let searches = asked.load(Ordering::SeqCst);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(searches >= 6, "Slack was asked only {searches} times");
    assert!(
        !waiting(&home).exists(),
        "nothing of Joel's to do while Slack can't be reached"
    );
}
