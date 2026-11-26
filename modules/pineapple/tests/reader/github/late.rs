//! pineapple-github hearing a stop from Joel's account only after he
//! resumed everything, though it was said just before: it stands, since
//! his agents post as his account too, so it may be one he never saw.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{LOGIN, answer_with, comment, github_reader, journal};
use crate::late::kept;
use crate::{answering, home};

#[test]
fn github_a_stop_from_joels_account_heard_after_his_resume_stands() {
    let (origin, _) = answering(|line, now| {
        answer_with(
            line,
            now,
            &[comment(1, LOGIN, "pineapple override", now, 90)],
        )
    });
    let home = home();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    // Joel resumed everything a minute ago, half a minute after the
    // stand-in's comment from his account says it stopped.
    let at = u64::try_from(now).expect("a time in range") - 60_000;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    std::fs::write(
        state.join("resumes.jsonl"),
        format!("{}\n", json!({"at": at, "by": "joel", "scope": "all"})),
    )
    .expect("a resume written");
    let mut reading = github_reader(&home, &origin, &[]);
    let records = kept(&state, 1);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        records
            .iter()
            .map(|record| record["resumed"].clone())
            .collect::<Vec<_>>(),
        [Value::Null],
        "a stop from his account stands, though said before his resume: {records:?}"
    );
    assert!(
        !journal(&home).contains("kept as resumed"),
        "and nothing says it was kept resumed: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
