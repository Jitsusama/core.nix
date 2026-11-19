//! What the Slack reader does with how far it had read when that can't
//! be trusted: a mark it can't read, and one a clock set back has left
//! later than now. Either way it reads from ten minutes back, says so,
//! and still hears a stop said now.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{decisions, each, home, journal, reader, stand_in, until_decided};

#[test]
fn a_mark_that_cant_be_trusted_reads_from_ten_minutes_back_and_says_so() {
    let (origin, _) = stand_in();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    let ahead = json!({"since": format!("{}.000000", now + 3600), "keys": []}).to_string();
    for (kept, says) in [
        (
            "{not json".to_owned(),
            "how far Slack was read can't all be read",
        ),
        (
            ahead,
            "how far Slack was read starts later than ten minutes ago",
        ),
    ] {
        let home = home();
        std::fs::write(home.join("state/pineapple/judged.json"), &kept).expect("the mark written");
        let mut reader = reader(&home, &origin);
        // The last minute's three are heard; the hour-old stop is before
        // ten minutes back.
        until_decided(&home, 3);
        reader.kill().expect("the reader stops");
        reader.wait().expect("the reader is gone");
        assert_eq!(
            each(&decisions(&home), "verdict"),
            ["stop", "stop-asked", "stop"],
            "kept {kept}, what was said in the last minute is judged"
        );
        let said = journal(&home);
        assert_eq!(
            said.lines()
                .filter(|line| line.starts_with(&format!("<4>{says}")))
                .count(),
            1,
            "and the journal warns why, once: {said}"
        );
        std::fs::remove_dir_all(&home).expect("the case's home goes");
    }
}
