//! pineapple-slack telling Joel in For You when it can't hear Slack: a
//! sign-in Slack refuses writes the row, hearing Slack again takes it
//! away, and a Slack that can't be reached writes none.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

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
    let row: Value = std::fs::read_to_string(waiting(&home))
        .ok()
        .and_then(|said| serde_json::from_str(&said).ok())
        .unwrap_or_default();
    refusing.store(false, Ordering::SeqCst);
    let untold = until(&waiting(&home), false);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(told, "Joel is told while Slack refuses the sign-in");
    assert_eq!(row["items"][0]["id"], "pineapple-slack-deaf");
    assert!(
        row["items"][0]["why"]
            .as_str()
            .is_some_and(|why| why.starts_with("Slack refused the safe word's sign-in")),
        "it says why: {row}"
    );
    assert!(untold, "and untold once Slack is heard again");
}

#[test]
fn a_slack_that_cant_be_reached_isnt_told_as_deaf() {
    let (origin, _) = answering_whole(|_, _| Answered {
        status: 500,
        headers: Vec::new(),
        body: json!({}),
    });
    let home = home();
    let mut reader = reader(&home, &origin);
    // Ten rounds at this pace, five times the two that would tell.
    thread::sleep(Duration::from_secs(1));
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(
        !waiting(&home).exists(),
        "nothing of Joel's to do while Slack can't be reached"
    );
}
