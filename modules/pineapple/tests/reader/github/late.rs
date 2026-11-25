//! pineapple-github hearing a stop of Joel's only after he resumed
//! everything, though he said it just before: it's kept as he left it,
//! resumed, while someone else's said as early, and his own edited since,
//! stand.

use std::time::{SystemTime, UNIX_EPOCH};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, comment, github_reader, journal};
use crate::late::kept;
use crate::{answering, home};

#[test]
fn github_a_stop_of_joels_said_before_his_resume_and_heard_after_is_kept_resumed() {
    let (origin, _) = answering(|line, now| {
        let mut edited = comment(3, LOGIN, "pineapple override", now, 80);
        edited["updated_at"] = json!(stamp(now - 10));
        answer_with(
            line,
            now,
            &[
                comment(1, LOGIN, "pineapple override", now, 90),
                comment(
                    2,
                    "colleague",
                    &format!("@{LOGIN} pineapple override"),
                    now,
                    85,
                ),
                edited,
            ],
        )
    });
    let home = home();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    // Joel resumed everything a minute ago, half a minute after the
    // stand-in's comments say he and a colleague stopped.
    let at = u64::try_from(now).expect("a time in range") - 60_000;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    std::fs::write(
        state.join("resumes.jsonl"),
        format!("{}\n", json!({"at": at, "by": "joel", "scope": "all"})),
    )
    .expect("a resume written");
    let mut reading = github_reader(&home, &origin, &[]);
    // By when each was said: his, the colleague's, his edited one.
    let records = kept(&state, 3);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        records
            .iter()
            .map(|record| record["resumed"].clone())
            .collect::<Vec<_>>(),
        [json!({"at": at, "by": "joel"}), Value::Null, Value::Null],
        "his stop said before his resume is kept as it left it; someone else's and \
         his edited one stand: {records:?}"
    );
    assert!(
        journal(&home).contains("said before Joel resumed it, kept as resumed"),
        "the journal says why it was kept resumed: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
