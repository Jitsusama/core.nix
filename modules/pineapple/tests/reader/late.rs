//! pineapple-slack hearing a stop only after Joel resumed everything,
//! though it was said before he did: it's kept as he left it, resumed,
//! and no reply says it stands, while one said after his resume stands.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{decisions, each, home, journal, reader, stand_in, until_decided};

#[test]
fn a_stop_said_before_joels_resume_and_heard_after_is_kept_resumed() {
    let (origin, _asked) = stand_in();
    let home = home();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    // Joel resumed everything 40 seconds ago: after the stand-in's stops
    // said an hour and a minute back, and before the one said 20 seconds
    // back.
    let at = u64::try_from(now).expect("a time in range") - 40_000;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    std::fs::write(
        state.join("resumes.jsonl"),
        format!("{}\n", json!({"at": at, "by": "launcher", "scope": "all"})),
    )
    .expect("a resume written");
    let mut reading = reader(&home, &origin);
    until_decided(&home, 4);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "gate"),
        [
            "held:too-old",
            "held:resumed-before-heard",
            "unasked",
            "through:first-in-thread"
        ],
        "the stop Joel resumed says nothing; the one said since is answered"
    );
    let mut records: Vec<Value> = std::fs::read_dir(state.join("stops"))
        .expect("the stops folder")
        .filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
        .filter_map(|record| serde_json::from_str(&record).ok())
        .collect();
    records.sort_by_key(|record| record["said_at"].as_str().map(str::to_owned));
    assert_eq!(
        records
            .iter()
            .map(|record| record["resumed"].clone())
            .collect::<Vec<_>>(),
        [
            json!({"at": at, "by": "launcher"}),
            json!({"at": at, "by": "launcher"}),
            Value::Null
        ],
        "the two said before his resume are kept resumed; the one said after stands"
    );
    assert!(
        journal(&home).contains("said before Joel resumed it, kept as resumed"),
        "the journal says why it was kept resumed: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
