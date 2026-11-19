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
use super::{Answered, PATIENCE, answer, answering_whole, home, journal, reader, reading};

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
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn the_row_goes_where_the_unit_names_it() {
    // The unit names the file its stop removes (home.nix), so the reader
    // writes there rather than finding its own.
    let (origin, _) =
        answering_whole(|_, _| Answered::ok(json!({"ok": false, "error": "invalid_auth"})));
    let home = home();
    let named = home.join("named/pineapple.json");
    let mut reader = reading(&home, &origin, &[], Some(&named));
    let told = until(&named, true);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(told, "the row is written where the unit names it");
    assert!(
        !waiting(&home).exists(),
        "and not where the reader would find its own"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_name_that_isnt_a_whole_path_is_no_name() {
    let (origin, _) =
        answering_whole(|_, _| Answered::ok(json!({"ok": false, "error": "invalid_auth"})));
    // A name with a quote and a line break is said escaped, so it can't
    // forge a journal line of its own.
    for named in ["", "named/pineapple.json", "named\"\n<3>forged"] {
        let home = home();
        let mut reader = reading(&home, &origin, &[], Some(Path::new(named)));
        let told = until(&waiting(&home), true);
        reader.kill().expect("the reader stops");
        reader.wait().expect("the reader is gone");
        assert!(told, "named {named:?}, the reader tells For You in its own");
        let said = journal(&home);
        assert!(
            said.contains(&format!(
                "<4>SAFE_WORD_WAITING names {named:?}, which isn't a whole path, so For You is \
                 told in {:?}, which",
                waiting(&home).to_string_lossy()
            )),
            "and its journal says the name was passed over, and for where: {said}"
        );
        assert!(
            !said.lines().any(|line| line.starts_with("<3>forged")),
            "and no line of it is the name's own: {said}"
        );
        std::fs::remove_dir_all(&home).expect("the case's home goes");
    }
}

#[test]
fn a_refusal_in_either_search_is_told_though_the_other_cant_be_asked() {
    // One search is refused and the other can't be asked, so the refusal,
    // which Joel can mend, is what the round comes to, whichever it is.
    for phrase_refused in [true, false] {
        let (origin, _) = answering_whole(move |line, _| {
            if line.contains("query=pineapple") == phrase_refused {
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
        let row = row(&home);
        reader.kill().expect("the reader stops");
        reader.wait().expect("the reader is gone");
        assert!(
            told && says(&row, "Slack refused the safe word's sign-in"),
            "a refusal isn't hidden by a search that wasn't asked \
             (the phrase's refused: {phrase_refused}): {row}"
        );
        std::fs::remove_dir_all(&home).expect("the case's home goes");
    }
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
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_sign_in_that_doesnt_say_who_joel_is_is_told_as_none() {
    let (origin, _) = answering_whole(|line, now| Answered::ok(answer(line, now)));
    let home = home();
    std::fs::remove_file(home.join("config/agentic-harness/slack/credentials.json"))
        .expect("the file's sign-in taken away");
    let nobody = json!({"token": {
        "accessToken": "xoxc-stand-in", "cookie": "stand-in", "teamId": "T0TEAM0001",
        "userId": "",
    }});
    let mut reader = reading_with_keyring(&home, &origin, &format!("printf '%s\\n' '{nobody}'"));
    let told = until(&waiting(&home), true);
    let row = row(&home);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(
        told && says(&row, "The safe word has no Slack sign-in it can read"),
        "a sign-in naming nobody can't be read for him: {row}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_slack_failing_in_ways_signing_in_cant_mend_isnt_told_as_deaf() {
    // A server error, and an error word of Slack's that isn't a refusal
    // of the sign-in.
    for (failing, status, body) in [
        ("a server error", 500, json!({})),
        (
            "an error word",
            200,
            json!({"ok": false, "error": "fatal_error"}),
        ),
    ] {
        let asked = Arc::new(AtomicUsize::new(0));
        let asks = Arc::clone(&asked);
        let (origin, _) = answering_whole(move |line, _| {
            if line.contains("search.messages") {
                asks.fetch_add(1, Ordering::SeqCst);
            }
            Answered {
                status,
                headers: Vec::new(),
                body: body.clone(),
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
        assert!(
            searches >= 6,
            "{failing}: Slack was asked only {searches} times"
        );
        assert!(
            !waiting(&home).exists(),
            "{failing}: nothing of Joel's to do that signing in would mend"
        );
        std::fs::remove_dir_all(&home).expect("the case's home goes");
    }
}
