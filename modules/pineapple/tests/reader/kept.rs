//! What the Slack reader does with how far it had read when that can't
//! be trusted: a mark it can't read, one a clock set back has left later
//! than now, and one whose judged keys are spoiled. Each is said once,
//! and a stop said now is still heard.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{decisions, each, home, journal, reader, stand_in, until_decided, until_marked_anew};

/// The three warnings, one for each way the mark can't be trusted.
const UNREADABLE: &str = "how far Slack was read can't all be read";
const AHEAD: &str = "how far Slack was read starts later than ten minutes ago";
const UNJUDGED: &str = "which Slack messages were judged can't all be read";

#[test]
fn a_mark_that_cant_be_trusted_is_said_once_and_a_stop_is_still_heard() {
    let (origin, _) = stand_in();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let since = |at: u64| format!("{at}.000000");
    // From ten minutes back the last minute's three are heard, and the
    // hour-old stop isn't; from two hours back, it is too.
    let recent = ["stop", "stop-asked", "stop"].as_slice();
    let all = ["stop", "stop", "stop-asked", "stop"].as_slice();
    for (kept, says, verdicts) in [
        ("{not json".to_owned(), UNREADABLE, recent),
        (
            json!({"since": since(now + 3600), "keys": []}).to_string(),
            AHEAD,
            recent,
        ),
        (
            json!({"since": since(now - 7200), "keys": "spoiled"}).to_string(),
            UNJUDGED,
            all,
        ),
    ] {
        let home = home();
        std::fs::write(home.join("state/pineapple/judged.json"), &kept).expect("the mark written");
        let mut reader = reader(&home, &origin);
        until_decided(&home, verdicts.len());
        // The first mark may be the deciding round's own, so three marks
        // make at least two rounds more, where a warning said each round
        // would show.
        for _ in 0..3 {
            until_marked_anew(&home, "judged.json");
        }
        reader.kill().expect("the reader stops");
        reader.wait().expect("the reader is gone");
        assert_eq!(
            each(&decisions(&home), "verdict"),
            verdicts,
            "kept {kept}, what was said since is judged, once"
        );
        let said = journal(&home);
        // Only the warnings about the mark; the reader's others are not
        // this case's business.
        let warned: Vec<&str> = said
            .lines()
            .filter(|line| {
                [UNREADABLE, AHEAD, UNJUDGED]
                    .iter()
                    .any(|warning| line.starts_with(&format!("<4>{warning}")))
            })
            .collect();
        assert!(
            warned.len() == 1 && warned[0].starts_with(&format!("<4>{says}")),
            "kept {kept}, the journal warns {says:?} once, and no other warning about the \
             mark: {said}"
        );
        std::fs::remove_dir_all(&home).expect("the case's home goes");
    }
}
