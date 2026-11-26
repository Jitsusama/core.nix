use super::*;
use serde_json::json;

const MIN: u64 = 60_000;

fn record(title: &str, start: u64, end: u64) -> Value {
    json!({"heard": 0, "key": title, "kind": "meeting",
           "meeting": {"title": title, "start": start, "end": end, "answer": "yes"}})
}

fn meeting(title: &str, start: u64, end: u64) -> Meeting {
    Meeting {
        title: title.into(),
        start,
        end,
    }
}

#[test]
fn an_agent_is_kept_off_the_camera_from_a_quarter_hour_before_until_the_end() {
    let start = 1_000 * MIN;
    let meetings = [meeting("Design review", start, start + 30 * MIN)];
    for (now, guarded, when) in [
        (start - 16 * MIN, false, "16 minutes before"),
        (
            start - AHEAD_MS - 1,
            false,
            "just over a quarter hour before",
        ),
        (start - AHEAD_MS, true, "a quarter hour before"),
        (start - 10 * MIN, true, "10 minutes before"),
        (start, true, "as it starts"),
        (start + 30 * MIN - 1, true, "as it ends"),
        (start + 30 * MIN, false, "once it has ended"),
    ] {
        assert_eq!(
            near(&meetings, now).is_some(),
            guarded,
            "the guard was {} {when}",
            if guarded { "down" } else { "up" }
        );
    }
}

#[test]
fn of_meetings_close_together_the_guard_says_the_one_ending_last() {
    let now = 1_000 * MIN;
    let meetings = [
        meeting("Standup", now - 5 * MIN, now + 10 * MIN),
        meeting("Planning", now + 10 * MIN, now + 70 * MIN),
        meeting("Lunch", now + 2 * 60 * MIN, now + 3 * 60 * MIN),
    ];
    assert_eq!(
        near(&meetings, now).map(|m| m.title.as_str()),
        Some("Planning"),
        "the guard didn't say it stays up through the meeting just after"
    );
}

#[test]
fn a_declined_day_long_or_finished_meeting_keeps_no_guard() {
    let now = 1_000 * MIN;
    let mut declined = record("Town hall", now, now + 60 * MIN);
    declined["meeting"]["answer"] = json!("no");
    let mut day = record("Offsite", now, now + 24 * 60 * MIN);
    day["meeting"]["allDay"] = json!(true);
    let mut done = record("Sync", now - 30 * MIN, now + 30 * MIN);
    done["meeting"]["done"] = json!({"at": now - MIN});
    let kept = record("Retro", now + 5 * MIN, now + 35 * MIN);
    let meetings = read(&json!({"read": now, "records": [declined, day, done, kept]})).unwrap();
    assert_eq!(
        meetings,
        vec![
            meeting("Sync", now - 30 * MIN, now - MIN),
            meeting("Retro", now + 5 * MIN, now + 35 * MIN)
        ],
        "the meetings weren't read by call-under-load's rules"
    );
    assert_eq!(
        near(&meetings, now).map(|m| m.title.as_str()),
        Some("Retro"),
        "a meeting marked done still kept the guard up"
    );
}

#[test]
fn an_answer_without_its_read_time_or_with_an_error_is_refused() {
    assert!(
        read(&json!({"records": []})).is_err(),
        "an answer given without reaching Google was taken as no meetings"
    );
    assert!(
        read(&json!({"error": {"code": "unavailable"}})).is_err(),
        "a refusal was taken as no meetings"
    );
    assert!(
        read(&json!({"read": 1})).is_err(),
        "an answer without records was taken as no meetings"
    );
}

#[test]
fn a_title_s_control_characters_are_dropped() {
    // The escape is made from its character, since spelling one in the
    // source is what commands-agree's inks point refuses.
    let title = format!("Ad{}[2Jmin", char::from(0x1b));
    let answer = json!({"read": 0, "records": [record(&title, 1, 2)]});
    assert_eq!(
        read(&answer).unwrap()[0].title,
        "Ad[2Jmin",
        "a title's escape reached the journal"
    );
}

#[test]
fn kept_meetings_come_back_as_they_were_and_none_kept_reads_as_none() {
    let folder = std::env::temp_dir().join(format!("camera-guard-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("meetings.json");
    assert_eq!(
        kept(&file),
        Ok(Vec::new()),
        "a first run without a kept file didn't start from none"
    );
    let meetings = vec![meeting("Design review", 5 * MIN, 35 * MIN)];
    keep(&file, &meetings).unwrap();
    assert_eq!(
        kept(&file),
        Ok(meetings),
        "the meetings kept weren't those read back"
    );
    std::fs::write(&file, "not json").unwrap();
    assert!(
        kept(&file).is_err(),
        "a broken kept file was read as no meetings"
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

/// Asks a stand-in daemon on a socket of its own that answers with `said`,
/// then closes.
fn asked(name: &str, said: Vec<u8>) -> Result<Value, String> {
    let folder = std::env::temp_dir().join(format!("camera-guard-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let socket = folder.join("google.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let daemon = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut ask = String::new();
        BufReader::new(&stream).read_line(&mut ask).unwrap();
        // The guard stops reading at its limit and hangs up, which a write
        // past it hears as a broken pipe; that's the guard's choice, not a fault.
        let _ = stream.write_all(&said);
    });
    let answer = ask(&socket);
    daemon.join().unwrap();
    std::fs::remove_dir_all(&folder).unwrap();
    answer
}

#[test]
fn an_answer_is_read_and_one_past_the_limit_is_refused() {
    assert_eq!(
        asked("short", b"{\"read\":1,\"records\":[]}\n".to_vec()),
        Ok(json!({"read": 1, "records": []})),
        "a daemon's one-line answer wasn't read"
    );
    let endless = vec![b' '; usize::try_from(LONGEST).unwrap() + 1];
    let refused = asked("endless", endless).unwrap_err();
    assert!(
        refused.ends_with(&format!("the answer ran past {LONGEST} bytes")),
        "an answer past the limit wasn't refused as too long: {refused}"
    );
}
