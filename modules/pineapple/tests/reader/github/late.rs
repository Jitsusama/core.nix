//! pineapple-github hearing a stop only after Joel resumed everything,
//! though it was said before he did: it's kept as he left it, resumed.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{answer, github_reader, journal, only_stop};
use crate::{answering, home, until_decided};

#[test]
fn github_a_stop_said_before_joels_resume_and_heard_after_is_kept_resumed() {
    let (origin, _) = answering(answer);
    let home = home();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    // Joel resumed everything a minute ago, after the stand-in's stop said
    // a minute and a half back.
    let at = u64::try_from(now).expect("a time in range") - 60_000;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    std::fs::write(
        state.join("resumes.jsonl"),
        format!("{}\n", json!({"at": at, "by": "joel", "scope": "all"})),
    )
    .expect("a resume written");
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 3);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let record: Value = serde_json::from_str(&only_stop(&home).expect("the stop's record"))
        .expect("the record is JSON");
    assert_eq!(
        record["resumed"],
        json!({"at": at, "by": "joel"}),
        "said before his resume, it's kept as that resume left it"
    );
    assert!(
        journal(&home).contains("said before Joel resumed it, kept as resumed"),
        "the journal says why it was kept resumed: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
