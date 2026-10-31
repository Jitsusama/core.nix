//! What a call is, from what's already written in the runtime folder, with
//! no network: a Tuple call from tuple-call.json, which the shell's Tuple
//! writes while one is on, naming who it's with; or a meeting from
//! google.json, which the google program keeps, the one on now. Each
//! call's folder is named by when it started and what it was, so the file
//! picker and search find it by its title.

use std::path::Path;

use serde_json::Value;

/// How early a meeting can be joined and still be the one on.
const EARLY_MS: u64 = 10 * 60 * 1000;

/// What a call was.
#[derive(Debug, PartialEq, Default)]
pub struct Meeting {
    pub title: String,
    /// The calendar event, for a meeting.
    pub event: Option<String>,
    /// Who it's with, by address, for a Tuple call.
    pub person: Option<String>,
}

/// The call on at `now`, from the files in `runtime`; a Tuple call first,
/// since its file is there only while one is on, and a meeting otherwise.
pub fn on(runtime: &Path, now: u64) -> Meeting {
    let read = |name: &str| -> Value {
        std::fs::read_to_string(runtime.join(name))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    };
    tuple(&read("tuple-call.json"))
        .or_else(|| meeting(&read("google.json"), now))
        .unwrap_or_default()
}

fn tuple(call: &Value) -> Option<Meeting> {
    let with = call["with"].as_array()?;
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
fn meeting(google: &Value, now: u64) -> Option<Meeting> {
    google["meetings"]
        .as_array()?
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
        let google = json!({"meetings": [
            {"id": "old", "title": "Done", "start": NOW - 90 * MINUTE, "end": NOW - 30 * MINUTE},
            {"id": "long", "title": "All day", "start": NOW - 60 * MINUTE, "end": NOW + 9 * MINUTE},
            {"id": "e1", "title": "Planning", "start": NOW - 5 * MINUTE, "end": NOW + 25 * MINUTE},
            {"id": "later", "title": "Lunch", "start": NOW + 60 * MINUTE, "end": NOW + 90 * MINUTE},
        ]});
        let on = meeting(&google, NOW).unwrap();
        assert_eq!(on.title, "Planning");
        assert_eq!(on.event.as_deref(), Some("e1"));
        let early = meeting(&google, NOW - 14 * MINUTE).unwrap();
        assert_eq!(
            early.event.as_deref(),
            Some("e1"),
            "joined a few minutes early"
        );
        assert_eq!(meeting(&json!({}), NOW), None);
    }

    #[test]
    fn a_tuple_call_is_named_by_who_its_with() {
        let call = json!({"since": NOW, "with": [
            {"id": 7, "name": "Ada Lovelace", "email": "Ada@Example.com"},
            {"id": 8, "name": "Grace Hopper", "email": "grace@example.com"},
        ]});
        let on = tuple(&call).unwrap();
        assert_eq!(on.title, "Ada Lovelace and 1 more");
        assert_eq!(on.person.as_deref(), Some("ada@example.com"));
        assert_eq!(tuple(&json!({"since": null, "with": []})), None);
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
