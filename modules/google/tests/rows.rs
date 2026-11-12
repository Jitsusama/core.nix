//! google-rows, which a stand-in for the daemon has Calendar's rows made
//! by: a meeting told as the daemon tells one comes back as the records
//! the daemon would serve of it, its row, the rows inside and its drawer.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

#[test]
fn a_meeting_told_comes_back_as_the_records_the_daemon_serves() {
    let words = std::env::var("LAUNCHER_WORDS")
        .expect("LAUNCHER_WORDS names no vocabulary; google-reads-the-calendar hands it one");
    let now: i64 = 1_800_000_000_000;
    let meeting = json!({
        "id": "review", "title": "Design Review", "start": now + 300_000,
        "end": now + 2_100_000, "allDay": false, "repeats": null, "answer": "yes",
        "link": "https://meet.google.com/pqr-stuv-wxy", "kind": "meet",
        "page": "https://calendar.google.com/calendar/event?eid=review", "place": null,
        "about": null, "attachments": [], "dial": null, "holds": true, "through": "primary",
        "organizer": null, "guests": [], "reminders": [1], "guestsCounted": 0
    });
    let mut rows = Command::new(env!("CARGO_BIN_EXE_google-rows"))
        .arg(&words)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("google-rows starts");
    let asked = json!({"meetings": [meeting], "held": [], "now": now});
    rows.stdin
        .take()
        .expect("google-rows takes its input")
        .write_all(asked.to_string().as_bytes())
        .expect("google-rows reads what it's asked");
    let made = rows.wait_with_output().expect("google-rows ends");
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
