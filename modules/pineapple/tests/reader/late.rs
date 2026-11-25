//! pineapple-slack hearing a stop of Joel's only after he resumed
//! everything, though he said it just before: it's kept as he left it,
//! resumed, while his stop said an hour before, his edited one, someone
//! else's and his one said after the resume all stand.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{JOEL, PATIENCE, answer, answering, home, journal, reader, stop_in};

/// The stop records under `state`, once there are `count` of them, by
/// when each was said.
pub(super) fn kept(state: &Path, count: usize) -> Vec<Value> {
    let began = Instant::now();
    loop {
        let mut records: Vec<Value> = std::fs::read_dir(state.join("stops"))
            .into_iter()
            .flatten()
            .filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
            .filter_map(|record| serde_json::from_str(&record).ok())
            .collect();
        if records.len() >= count {
            records.sort_by_key(|record| record["said_at"].as_str().map(str::to_owned));
            return records;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "the reader kept {} stops, not {count}",
            records.len()
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// What Slack would answer: in the safe word's search, Joel's own stops
/// said an hour, a minute and 20 seconds back, one of his said 55 seconds
/// back and edited since, and someone else's said 50 seconds back, which
/// alone tags him; anything else as the stand-in answers it.
fn joels_and_others(line: &str, now: u64) -> Value {
    if line.contains("/api/conversations") {
        return answer(line, now);
    }
    let joels = |ago: u64, thread: &str| {
        let mut one = stop_in(now, ago, thread);
        one["user"] = json!(JOEL);
        one["text"] = json!("pineapple override");
        one
    };
    let theirs = stop_in(now, 50, "1700000000.000004");
    let matches = if line.contains("query=pineapple") {
        let mut edited = joels(55, "1700000000.000003");
        edited["edited"] = json!({"user": JOEL, "ts": format!("{}.000000", now - 10)});
        vec![
            joels(20, "1700000000.000001"),
            theirs,
            edited,
            joels(60, "1700000000.000001"),
            joels(3600, "1700000000.000002"),
        ]
    } else {
        vec![theirs]
    };
    json!({"ok": true, "messages": {"matches": matches}})
}

#[test]
fn a_stop_of_joels_said_before_his_resume_and_heard_after_is_kept_resumed() {
    let (origin, _asked) = answering(joels_and_others);
    let home = home();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    // Joel resumed everything from the launcher 40 seconds ago: 20 seconds
    // after his stop said a minute back.
    let at = u64::try_from(now).expect("a time in range") - 40_000;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("a state folder");
    std::fs::write(
        state.join("resumes.jsonl"),
        format!("{}\n", json!({"at": at, "by": "launcher", "scope": "all"})),
    )
    .expect("a resume written");
    let mut reading = reader(&home, &origin);
    let records = kept(&state, 5);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        records
            .iter()
            .map(|record| record["resumed"].clone())
            .collect::<Vec<_>>(),
        [
            Value::Null,
            json!({"at": at, "by": "launcher"}),
            Value::Null,
            Value::Null,
            Value::Null
        ],
        "only his stop said shortly before his resume is kept resumed; his hour-old one, \
         his edited one, someone else's and his one said after stand: {records:?}"
    );
    assert!(
        journal(&home).contains("said before Joel resumed it, kept as resumed"),
        "the journal says why it was kept resumed: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
