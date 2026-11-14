//! google-rows, which a stand-in for the daemon has Calendar's rows made
//! by: a meeting told as the daemon tells one comes back as the records
//! the daemon would serve of it, its row, the rows inside and its drawer;
//! the same as JSON with `--json`; and a state it can't read is said as
//! bad input.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// The launcher's words, which google-reads-the-calendar hands the tests.
fn words() -> String {
    std::env::var("LAUNCHER_WORDS")
        .expect("LAUNCHER_WORDS names no vocabulary; google-reads-the-calendar hands it one")
}

/// What google-rows says, and how it ends, asked `input` with `flags`.
fn asked(flags: &[&str], input: &str) -> std::process::Output {
    let mut rows = Command::new(env!("CARGO_BIN_EXE_google-rows"))
        .args(flags)
        .arg(words())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("google-rows starts");
    rows.stdin
        .take()
        .expect("google-rows takes its input")
        .write_all(input.as_bytes())
        .expect("google-rows reads what it's asked");
    rows.wait_with_output().expect("google-rows ends")
}

#[test]
fn the_records_as_json_name_their_schema() {
    let made = asked(&["--json"], r#"{"meetings": [], "held": [], "now": 0}"#);
    assert!(made.status.success(), "google-rows failed: {made:?}");
    let answer: Value = serde_json::from_slice(&made.stdout).expect("google-rows says JSON");
    assert_eq!(
        answer,
        json!({"schema": "google-rows.records/1", "records": [], "answer": null}),
        "no meetings, as JSON, isn't an empty list of records under its schema"
    );
}

#[test]
fn a_state_it_cant_read_is_bad_input() {
    let made = asked(&[], "not a state");
    assert_eq!(
        made.status.code(),
        Some(65),
        "a state it can't read didn't exit 65: {made:?}"
    );
    assert!(
        made.stdout.is_empty(),
        "it answered a state it couldn't read"
    );
}

#[test]
fn a_meeting_told_comes_back_as_the_records_the_daemon_serves() {
    let now: i64 = 1_800_000_000_000;
    let meeting = json!({
        "id": "review", "title": "Design Review", "start": now + 300_000,
        "end": now + 2_100_000, "allDay": false, "repeats": null, "answer": "yes",
        "link": "https://meet.google.com/pqr-stuv-wxy", "kind": "meet",
        "page": "https://calendar.google.com/calendar/event?eid=review", "place": null,
        "about": null, "attachments": [], "dial": null, "holds": true, "through": "primary",
        "organizer": null, "guests": [], "reminders": [1], "guestsCounted": 0
    });
    let input = json!({"meetings": [meeting], "held": [], "now": now});
    let made = asked(&[], &input.to_string());
    assert!(made.status.success(), "google-rows failed: {made:?}");
    let records: Value = serde_json::from_slice(&made.stdout).expect("google-rows says JSON");
    let keyed = |key: &str| {
        records
            .as_array()
            .and_then(|all| all.iter().find(|record| record["key"] == key))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let row = keyed("row:calendar:review");
    assert_eq!(
        row["kind"], "row",
        "the meeting isn't served as a row: {records}"
    );
    assert_eq!(
        row["row"]["thing"], "calendar:meeting:review",
        "the row isn't the meeting's"
    );
    assert_eq!(
        row["row"]["acts"][0]["key"], "verb:calendar:join",
        "Return doesn't join a call five minutes off"
    );
    assert_eq!(
        keyed("row:calendar:review:see")["row"]["parent"],
        "row:calendar:review",
        "Show in Calendar isn't served inside it"
    );
    assert_eq!(
        keyed("drawer:calendar:review")["kind"],
        "drawer",
        "its drawer isn't served"
    );
}
