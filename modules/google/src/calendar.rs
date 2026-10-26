//! Joel's meetings: today's and the next week's, from his primary
//! calendar.
//!
//! The window runs from the start of today to the start of the eighth day
//! after, in the machine's own time zone, so it's the same all day and
//! moves at midnight. Each repeating meeting comes as its own occurrence.
//! A cancelled one, and the calendar's other kinds of event (out of office,
//! focus time, where he's working, birthdays), aren't meetings.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::link::{self, Kind};

/// One meeting, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Meeting {
    /// The event's own id, which answering it names.
    pub id: String,
    pub title: String,
    /// When it starts and ends, in milliseconds since the epoch. A
    /// meeting lasting all day runs from one midnight to another here.
    pub start: i64,
    pub end: i64,
    #[serde(rename = "allDay")]
    pub all_day: bool,
    /// How Joel answered, or none when he wasn't asked: his own event, or
    /// one with nobody else on it.
    pub answer: Option<Answer>,
    /// The link it's joined by, and which call that is.
    pub link: Option<String>,
    pub kind: Option<Kind>,
    /// The event's page in Google Calendar.
    pub page: Option<String>,
}

/// How Joel answered an invitation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Yes,
    Maybe,
    No,
    /// Not answered yet.
    Waiting,
}

impl Answer {
    /// The answer as the Calendar API writes an attendee's.
    pub fn status(self) -> &'static str {
        match self {
            Answer::Yes => "accepted",
            Answer::Maybe => "tentative",
            Answer::No => "declined",
            Answer::Waiting => "needsAction",
        }
    }
}

/// The window asked for: from the start of the day `now` falls in, for
/// eight days.
pub fn window(now: &Zoned) -> (Timestamp, Timestamp) {
    let today = now.date();
    let from = midnight(today, now.time_zone());
    let to = midnight(today.saturating_add(8.days()), now.time_zone());
    (from, to)
}

fn midnight(day: Date, zone: &TimeZone) -> Timestamp {
    day.to_zoned(zone.clone())
        .map_or(Timestamp::UNIX_EPOCH, |zoned| zoned.timestamp())
}

/// The meetings in one page of the API's answer, in its order, which is
/// by start.
pub fn meetings(answer: &Value, zone: &TimeZone) -> Vec<Meeting> {
    answer
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| meeting(event, zone))
        .collect()
}

fn meeting(event: &Value, zone: &TimeZone) -> Option<Meeting> {
    let text = |field: &str| event.get(field).and_then(Value::as_str);
    if text("status") == Some("cancelled") {
        return None;
    }
    if text("eventType").is_some_and(|kind| kind != "default") {
        return None;
    }
    let (start, start_all_day) = when(event.get("start")?, zone)?;
    let (end, _) = when(event.get("end")?, zone)?;
    let answer = event
        .get("attendees")
        .and_then(Value::as_array)
        .and_then(|attendees| {
            attendees
                .iter()
                .find(|attendee| attendee.get("self").and_then(Value::as_bool) == Some(true))
        })
        // A meeting Joel holds is his own, not an invitation to answer.
        .filter(|me| me.get("organizer").and_then(Value::as_bool) != Some(true))
        .and_then(|me| me.get("responseStatus").and_then(Value::as_str))
        .map(|status| match status {
            "accepted" => Answer::Yes,
            "tentative" => Answer::Maybe,
            "declined" => Answer::No,
            _ => Answer::Waiting,
        });
    let (link, kind) =
        link::of(event).map_or((None, None), |(link, kind)| (Some(link), Some(kind)));
    Some(Meeting {
        id: text("id")?.to_owned(),
        // A meeting someone else holds whose details Joel can't see comes
        // without a title.
        title: text("summary").unwrap_or("").to_owned(),
        start,
        end,
        all_day: start_all_day,
        answer,
        link,
        kind,
        page: text("htmlLink").map(str::to_owned),
    })
}

/// When a start or end is, in milliseconds, and whether it's a whole day.
fn when(point: &Value, zone: &TimeZone) -> Option<(i64, bool)> {
    if let Some(at) = point.get("dateTime").and_then(Value::as_str) {
        let at: Timestamp = at.parse().ok()?;
        return Some((at.as_millisecond(), false));
    }
    let day: Date = point.get("date").and_then(Value::as_str)?.parse().ok()?;
    Some((midnight(day, zone).as_millisecond(), true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn zone() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-4))
    }

    #[test]
    fn the_window_is_today_and_the_next_seven_days_here() {
        let now = "2026-10-26T15:20:00-04:00[-04:00]"
            .parse::<Zoned>()
            .unwrap();
        let (from, to) = window(&now);
        assert_eq!(from.to_string(), "2026-10-26T04:00:00Z");
        assert_eq!(to.to_string(), "2026-11-03T04:00:00Z");
    }

    #[test]
    fn a_meeting_says_when_its_link_and_joels_answer() {
        let answer = json!({"items": [{
            "id": "e1",
            "status": "confirmed",
            "summary": "Standup",
            "start": {"dateTime": "2026-10-26T16:00:00-04:00", "timeZone": "America/Toronto"},
            "end": {"dateTime": "2026-10-26T16:15:00-04:00"},
            "hangoutLink": "https://meet.google.com/abc-defg-hij",
            "htmlLink": "https://www.google.com/calendar/event?eid=e1",
            "attendees": [
                {"email": "a@example.com", "responseStatus": "accepted"},
                {"email": "joel@example.com", "self": true, "responseStatus": "tentative"}
            ]
        }]});
        let found = meetings(&answer, &zone());
        assert_eq!(
            found,
            vec![Meeting {
                id: "e1".into(),
                title: "Standup".into(),
                start: 1_793_044_800_000,
                end: 1_793_045_700_000,
                all_day: false,
                answer: Some(Answer::Maybe),
                link: Some("https://meet.google.com/abc-defg-hij".into()),
                kind: Some(Kind::Meet),
                page: Some("https://www.google.com/calendar/event?eid=e1".into()),
            }]
        );
    }

    #[test]
    fn each_answer_reads_as_joel_would_say_it() {
        let with = |status: &str| {
            json!({"items": [{"id": "e", "start": {"date": "2026-10-27"},
                "end": {"date": "2026-10-28"},
                "attendees": [{"self": true, "responseStatus": status}]}]})
        };
        let answer = |status| meetings(&with(status), &zone())[0].answer;
        assert_eq!(answer("accepted"), Some(Answer::Yes));
        assert_eq!(answer("declined"), Some(Answer::No));
        assert_eq!(answer("needsAction"), Some(Answer::Waiting));
    }

    #[test]
    fn a_meeting_joel_holds_is_not_his_to_answer() {
        let answer = json!({"items": [{"id": "e", "start": {"date": "2026-10-27"},
            "end": {"date": "2026-10-28"},
            "attendees": [{"self": true, "organizer": true, "responseStatus": "accepted"},
                {"email": "a@example.com", "responseStatus": "needsAction"}]}]});
        assert_eq!(meetings(&answer, &zone())[0].answer, None);
    }

    #[test]
    fn each_answer_is_written_as_the_api_writes_it() {
        let written: Vec<_> = [Answer::Yes, Answer::Maybe, Answer::No, Answer::Waiting]
            .into_iter()
            .map(Answer::status)
            .collect();
        assert_eq!(
            written,
            ["accepted", "tentative", "declined", "needsAction"]
        );
    }

    #[test]
    fn a_whole_day_runs_midnight_to_midnight_here() {
        let answer = json!({"items": [{"id": "e", "summary": "Offsite",
            "start": {"date": "2026-10-27"}, "end": {"date": "2026-10-28"}}]});
        let found = &meetings(&answer, &zone())[0];
        assert!(found.all_day);
        assert_eq!(found.answer, None);
        assert_eq!(found.end - found.start, 86_400_000);
        assert_eq!(
            Timestamp::from_millisecond(found.start)
                .unwrap()
                .to_string(),
            "2026-10-27T04:00:00Z"
        );
    }

    #[test]
    fn cancelled_events_and_other_kinds_are_not_meetings() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let answer = json!({"items": [
            {"id": "a", "status": "cancelled", "start": at, "end": at},
            {"id": "b", "eventType": "outOfOffice", "start": at, "end": at},
            {"id": "c", "eventType": "focusTime", "start": at, "end": at},
            {"id": "d", "eventType": "workingLocation", "start": {"date": "2026-10-26"},
             "end": {"date": "2026-10-27"}},
            {"id": "e", "eventType": "default", "start": at, "end": at}
        ]});
        let ids: Vec<_> = meetings(&answer, &zone())
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, vec!["e"]);
    }

    #[test]
    fn a_meeting_without_a_title_or_link_is_still_a_meeting() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let found = &meetings(
            &json!({"items": [{"id": "x", "start": at, "end": at}]}),
            &zone(),
        )[0];
        assert_eq!(found.title, "");
        assert_eq!(found.link, None);
        assert_eq!(found.kind, None);
    }
}
