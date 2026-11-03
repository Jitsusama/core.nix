//! What a call is, from what this machine already knows, with no network:
//! a Tuple call from tuple-call, the daemon following Tuple's, asked on
//! its socket who it's with; or a meeting from the google program's
//! daemon, asked on its socket for the meetings it holds, the one on now.
//! Each call's folder is named by when it started and what it was, so the
//! file picker and search find it by its title.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// How early a meeting can be joined and still be the one on.
const EARLY_MS: u64 = 10 * 60 * 1000;
/// How long a daemon is given to answer. Each answers from what it holds,
/// so a second is long; a call starting is written down either way, under
/// a meeting's name or a bare time.
const ANSWERS_WITHIN: Duration = Duration::from_secs(1);

/// What a call was.
#[derive(Debug, PartialEq, Default)]
pub struct Meeting {
    pub title: String,
    /// The calendar event, for a meeting.
    pub event: Option<String>,
    /// Who it's with, by address, for a Tuple call.
    pub person: Option<String>,
}

/// The call on at `now`, from what's in `runtime`; a Tuple call first,
/// since it has people only while one is on, and a meeting otherwise.
pub fn on(runtime: &Path, now: u64) -> Meeting {
    tuple_call(&runtime.join("daemons/tuple.sock"))
        .as_ref()
        .and_then(tuple)
        .or_else(|| meeting(&meetings(&runtime.join("daemons/google.sock")), now))
        .unwrap_or_default()
}

/// tuple-call's record of the call; or none when it isn't there or
/// doesn't answer in time.
fn tuple_call(socket: &Path) -> Option<Value> {
    let said = asked(
        socket,
        &json!({"id": 1, "ask": "get", "keys": ["call:current"]}),
    )?;
    said["records"].get(0).cloned()
}

/// The meetings the google program's daemon holds; or none when it isn't
/// there or doesn't answer in time.
fn meetings(socket: &Path) -> Vec<Value> {
    let Some(said) = asked(socket, &json!({"id": 1, "ask": "list", "kind": "meeting"})) else {
        return Vec::new();
    };
    said["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| record["meeting"].clone())
        .collect()
}

/// A daemon's answer to one ask, by the daemons' interface: one line out
/// and one back; or none when it isn't there or doesn't answer in time.
fn asked(socket: &Path, ask: &Value) -> Option<Value> {
    let stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(ANSWERS_WITHIN)).ok()?;
    stream.set_write_timeout(Some(ANSWERS_WITHIN)).ok()?;
    writeln!(&stream, "{ask}").ok()?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).ok()?;
    serde_json::from_str(&line).ok()
}

/// A Tuple call, named by who it's with.
fn tuple(call: &Value) -> Option<Meeting> {
    let with = call["people"].as_array()?;
    let first = with.first()?;
    let name = |person: &Value| person["name"].as_str().unwrap_or_default().to_owned();
    let mut title = name(first);
    if with.len() > 1 {
        title = format!("{title} and {} more", with.len() - 1);
    }
    Some(Meeting {
        title,
        event: None,
        person: first["email"].as_str().map(str::to_lowercase),
    })
}

/// The meeting on at `now`: of those started (or about to) and not ended,
/// the one started latest, since Joel can be in only one and has moved on
/// to the newer.
fn meeting(meetings: &[Value], now: u64) -> Option<Meeting> {
    meetings
        .iter()
        .filter(|meeting| {
            let start = meeting["start"].as_u64().unwrap_or(u64::MAX);
            let end = meeting["end"].as_u64().unwrap_or(0);
            start <= now + EARLY_MS && now <= end
        })
        .max_by_key(|meeting| meeting["start"].as_u64())
        .map(|meeting| Meeting {
            title: meeting["title"].as_str().unwrap_or_default().to_owned(),
            event: meeting["id"].as_str().map(str::to_owned),
            person: None,
        })
}

/// A call's folder's name: its start in local time, then what it was,
/// with what a file name can't hold taken out.
pub fn folder(started: u64, title: &str) -> String {
    let when = local(started);
    let title: String = title
        .chars()
        .map(|c| if c == '/' || c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let title: String = title.chars().take(80).collect();
    let title = title.trim_start_matches('.');
    if title.is_empty() {
        when
    } else {
        format!("{when} {title}")
    }
}

/// glibc's broken-down time.
#[repr(C)]
#[derive(Default)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    mday: i32,
    mon: i32,
    year: i32,
    wday: i32,
    yday: i32,
    isdst: i32,
    gmtoff: i64,
    zone: usize,
}

unsafe extern "C" {
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
}

/// A time in milliseconds as the machine's local date and time, as
/// "2026-10-31 09.30".
fn local(ms: u64) -> String {
    let seconds = i64::try_from(ms / 1000).unwrap_or(i64::MAX);
    let mut tm = Tm::default();
    // SAFETY: both pointers are to values alive across the call.
    if unsafe { localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return "a call".to_owned();
    }
    format!(
        "{}-{:02}-{:02} {:02}.{:02}",
        tm.year + 1900,
        tm.mon + 1,
        tm.mday,
        tm.hour,
        tm.min
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const NOW: u64 = 1_800_000_000_000;
    const MINUTE: u64 = 60_000;

    #[test]
    fn the_meeting_on_is_the_latest_started_of_those_not_ended() {
        let google = [
            json!({"id": "old", "title": "Done", "start": NOW - 90 * MINUTE, "end": NOW - 30 * MINUTE}),
            json!({"id": "long", "title": "All day", "start": NOW - 60 * MINUTE, "end": NOW + 9 * MINUTE}),
            json!({"id": "e1", "title": "Planning", "start": NOW - 5 * MINUTE, "end": NOW + 25 * MINUTE}),
            json!({"id": "later", "title": "Lunch", "start": NOW + 60 * MINUTE, "end": NOW + 90 * MINUTE}),
        ];
        let on = meeting(&google, NOW).unwrap();
        assert_eq!(on.title, "Planning");
        assert_eq!(on.event.as_deref(), Some("e1"));
        let early = meeting(&google, NOW - 14 * MINUTE).unwrap();
        assert_eq!(
            early.event.as_deref(),
            Some("e1"),
            "joined a few minutes early"
        );
        assert_eq!(meeting(&[], NOW), None);
    }

    #[test]
    fn a_tuple_call_is_named_by_who_its_with() {
        let call = json!({"key": "call:current", "state": "connected", "since": NOW, "people": [
            {"id": "7", "name": "Ada Lovelace", "email": "Ada@Example.com"},
            {"id": "8", "name": "Grace Hopper", "email": "grace@example.com"},
        ]});
        let on = tuple(&call).unwrap();
        assert_eq!(on.title, "Ada Lovelace and 1 more");
        assert_eq!(on.person.as_deref(), Some("ada@example.com"));
        let none = json!({"key": "call:current", "state": "ringing", "since": null, "people": []});
        assert_eq!(tuple(&none), None);
    }

    /// A fresh runtime folder with two stand-in daemons: tuple-call,
    /// answering its one ask with `record`, and the google program's,
    /// holding Planning, on now.
    fn runtime(name: &str, record: &Value) -> std::path::PathBuf {
        let runtime = std::env::temp_dir().join(format!("meeting-{name}-{}", std::process::id()));
        std::fs::create_dir_all(runtime.join("daemons")).unwrap();
        let call = json!({"records": [record]});
        answering(
            &runtime.join("daemons/tuple.sock"),
            "get",
            "keys",
            json!(["call:current"]),
            call,
        );
        let planning =
            json!({"id": "e1", "title": "Planning", "start": NOW - MINUTE, "end": NOW + MINUTE});
        let meetings = json!({"records": [
            {"key": "meeting:e1", "kind": "meeting", "heard": NOW, "meeting": planning},
        ]});
        answering(
            &runtime.join("daemons/google.sock"),
            "list",
            "kind",
            json!("meeting"),
            meetings,
        );
        runtime
    }

    /// A stand-in daemon at `socket` answering one `ask`, whose `field` must
    /// be `meant`, with `answer` under the ask's id.
    fn answering(
        socket: &Path,
        ask: &'static str,
        field: &'static str,
        meant: Value,
        answer: Value,
    ) {
        let _ = std::fs::remove_file(socket);
        let listener = std::os::unix::net::UnixListener::bind(socket).unwrap();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
            let asked: Value = serde_json::from_str(&line).unwrap();
            assert_eq!((asked["ask"].as_str(), &asked[field]), (Some(ask), &meant));
            let mut answer = answer;
            answer["id"] = asked["id"].clone();
            writeln!(&stream, "{answer}").unwrap();
        });
    }

    #[test]
    fn tuple_call_is_asked_and_a_tuple_call_comes_before_a_meeting() {
        let call = json!({"state": "connected", "people": [{"name": "Ada", "email": "a@x"}]});
        let on = on(&runtime("tuple", &call), NOW);
        assert_eq!(on.title, "Ada");
        assert_eq!(on.event, None);
    }

    #[test]
    fn with_no_tuple_call_on_the_meeting_is_the_call() {
        let none = json!({"state": "none", "since": null, "people": []});
        assert_eq!(on(&runtime("none", &none), NOW).title, "Planning");
        let gone = runtime("gone", &none);
        std::fs::remove_file(gone.join("daemons/tuple.sock")).unwrap();
        assert_eq!(on(&gone, NOW).title, "Planning");
    }

    #[test]
    fn with_neither_daemon_there_the_call_is_nothing_known() {
        let none = json!({"state": "none", "since": null, "people": []});
        let bare = runtime("bare", &none);
        std::fs::remove_file(bare.join("daemons/tuple.sock")).unwrap();
        std::fs::remove_file(bare.join("daemons/google.sock")).unwrap();
        assert_eq!(on(&bare, NOW), Meeting::default());
    }

    #[test]
    fn a_folder_is_named_by_its_start_and_a_title_a_file_can_hold() {
        let name = folder(NOW, " Q4 / planning\n");
        assert!(name.ends_with(" Q4 planning"), "{name}");
        assert_eq!(name.len(), "2027-01-15 03.20 Q4 planning".len(), "{name}");
        assert!(!folder(NOW, "..").contains(".."));
        assert_eq!(folder(NOW, "").len(), "2027-01-15 03.20".len());
    }
}
