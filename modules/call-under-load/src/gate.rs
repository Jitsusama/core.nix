//! Whether the hour's call may run: no call of Joel's on, as `a-call-is-on`
//! says, and no meeting of his within ten minutes of it, as the google
//! daemon's calendar says. The test call takes no camera of his, but it
//! loads the machine and the line, and a meeting may start a call any
//! moment.

use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

/// How long before and after a meeting the call keeps clear.
const CLEAR_MS: u64 = 10 * 60 * 1000;
/// How old the calendar's last read may be before it's trusted no more.
const STALE_MS: u64 = 60 * 60 * 1000;

/// What `a-call-is-on` said.
#[derive(Debug, PartialEq, Eq)]
pub enum Call {
    On,
    Off,
    /// It couldn't tell, or couldn't be run; taken as a call.
    Unknown(String),
}

/// Asks `a-call-is-on`, which exits 0 for a call, 1 for none and 69 when
/// it can't tell.
pub fn a_call() -> Call {
    match Command::new("a-call-is-on")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => match status.code() {
            Some(0) => Call::On,
            Some(1) => Call::Off,
            code => Call::Unknown(format!("a-call-is-on exited {code:?}")),
        },
        Err(error) => Call::Unknown(format!("a-call-is-on couldn't run: {error}")),
    }
}

/// One meeting from the calendar, its times in ms since 1970.
#[derive(Debug, PartialEq)]
pub struct Meeting {
    pub title: String,
    pub start: u64,
    pub end: u64,
}

/// The calendar as the daemon last read it.
#[derive(Debug, PartialEq)]
pub struct Calendar {
    pub read: u64,
    pub meetings: Vec<Meeting>,
}

/// The calendar from the google daemon's answer to a list of meetings.
/// A meeting Joel declined is left out, and so is a day-long one, which
/// holds a day rather than a call; one that has ended ends then.
pub fn calendar(answer: &Value) -> Result<Calendar, String> {
    if let Some(error) = answer.get("error") {
        return Err(format!("the google daemon refused: {error}"));
    }
    let read = answer["read"]
        .as_u64()
        .ok_or("the google daemon's answer has no read time")?;
    let records = answer["records"]
        .as_array()
        .ok_or("the google daemon's answer has no records")?;
    let meetings = records
        .iter()
        .map(|record| &record["meeting"])
        .filter(|meeting| meeting["allDay"] != true && meeting["answer"] != "no")
        .filter_map(|meeting| {
            let start = meeting["start"].as_u64()?;
            let end = meeting["end"].as_u64()?;
            let done = meeting["done"]["at"].as_u64().unwrap_or(end);
            Some(Meeting {
                // A title is anyone's text, and it's printed to a terminal.
                title: meeting["title"]
                    .as_str()
                    .unwrap_or("a meeting")
                    .chars()
                    .filter(|c| !c.is_control())
                    .collect(),
                start,
                end: end.min(done),
            })
        })
        .collect();
    Ok(Calendar { read, meetings })
}

/// Why a call that starts at `now` and lasts `run` ms may not run, if it
/// may not: a calendar read too long ago, or a meeting within ten minutes.
pub fn kept_clear(calendar: &Calendar, now: u64, run: u64) -> Option<String> {
    let age = now.saturating_sub(calendar.read);
    if age > STALE_MS {
        return Some(format!(
            "the calendar was last read {} min ago",
            age / 60_000
        ));
    }
    calendar
        .meetings
        .iter()
        .find(|m| m.start < now + run + CLEAR_MS && m.end + CLEAR_MS > now)
        .map(|m| format!("{} is within 10 minutes", m.title))
}

/// Asks the google daemon at `socket` for Joel's meetings.
pub fn ask(socket: &Path) -> Result<Value, String> {
    let seen = |error: std::io::Error| format!("{}: {error}", socket.display());
    let mut stream = UnixStream::connect(socket).map_err(seen)?;
    let wait = Some(Duration::from_secs(10));
    stream.set_read_timeout(wait).map_err(seen)?;
    stream.set_write_timeout(wait).map_err(seen)?;
    stream
        .write_all(b"{\"id\":1,\"ask\":\"list\",\"kind\":\"meeting\"}\n")
        .map_err(seen)?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).map_err(seen)?;
    serde_json::from_str(&line).map_err(|error| format!("{}: {error}", socket.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const MIN: u64 = 60_000;

    fn meeting(title: &str, start: u64, end: u64) -> Value {
        json!({"key": "meeting:1", "kind": "meeting",
               "meeting": {"title": title, "start": start, "end": end, "allDay": false,
                           "answer": "yes"}})
    }

    fn at(now: u64, records: Vec<Value>) -> Calendar {
        calendar(&json!({"read": now - MIN, "records": Value::Array(records)})).unwrap()
    }

    #[test]
    fn a_meeting_within_ten_minutes_of_the_call_keeps_it_clear() {
        let now = 1_000 * MIN;
        let soon = at(
            now,
            vec![meeting("Book club", now + 13 * MIN, now + 60 * MIN)],
        );
        let why = kept_clear(&soon, now, 4 * MIN).unwrap();
        assert_eq!(why, "Book club is within 10 minutes");
        let just_over = at(now, vec![meeting("1:1", now - 30 * MIN, now - 9 * MIN)]);
        assert!(
            kept_clear(&just_over, now, 4 * MIN).is_some(),
            "a meeting that ended 9 minutes ago left the hour clear"
        );
    }

    #[test]
    fn a_meeting_further_off_leaves_the_call_to_run() {
        let now = 1_000 * MIN;
        let later = at(
            now,
            vec![meeting("Book club", now + 15 * MIN, now + 60 * MIN)],
        );
        assert_eq!(kept_clear(&later, now, 4 * MIN), None);
        let over = at(now, vec![meeting("1:1", now - 40 * MIN, now - 11 * MIN)]);
        assert_eq!(kept_clear(&over, now, 4 * MIN), None);
    }

    #[test]
    fn a_declined_day_long_or_ended_meeting_holds_no_call() {
        let now = 1_000 * MIN;
        let mut declined = meeting("Town hall", now, now + 60 * MIN);
        declined["meeting"]["answer"] = json!("no");
        let mut day = meeting("Holiday", now - 600 * MIN, now + 600 * MIN);
        day["meeting"]["allDay"] = json!(true);
        let mut ended = meeting("Review", now - 50 * MIN, now + 30 * MIN);
        ended["meeting"]["done"] = json!({"at": now - 20 * MIN, "how": "ended"});
        let calendar = at(now, vec![declined, day, ended]);
        assert_eq!(calendar.meetings.len(), 1);
        assert_eq!(kept_clear(&calendar, now, 4 * MIN), None);
    }

    #[test]
    fn a_calendar_read_over_an_hour_ago_is_not_trusted() {
        let now = 1_000 * MIN;
        let stale = calendar(&json!({"read": now - 61 * MIN, "records": []})).unwrap();
        let why = kept_clear(&stale, now, 4 * MIN).unwrap();
        assert_eq!(why, "the calendar was last read 61 min ago");
    }

    #[test]
    fn an_answer_without_its_read_time_or_with_an_error_is_refused() {
        assert!(
            calendar(&json!({"records": []})).is_err(),
            "an answer without its read time was taken"
        );
        assert!(
            calendar(&json!({"error": {"kind": "busy"}})).is_err(),
            "an error was taken for an answer"
        );
    }
}
