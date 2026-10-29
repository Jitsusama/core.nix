//! Joel's meetings: today's and the next week's, from his primary
//! calendar.
//!
//! The window runs from the start of today to the start of the eighth day
//! after, in the machine's own time zone, so it's the same all day and
//! moves at midnight. Each repeating meeting comes as its own occurrence.
//! A cancelled one, and the calendar's other kinds of event (out of office,
//! focus time, where he's working, birthdays), aren't meetings.
//!
//! Each meeting says what the launcher shows of it: where, who holds it,
//! who else is asked and how each answered, and what it's about, its
//! description as plain text, and when Joel is reminded of it.

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
    /// Where it is, as its location says it: a room, an address, a link.
    pub place: Option<String>,
    /// What it's about: its description as plain text, at most
    /// `ABOUT_MOST` characters.
    pub about: Option<String>,
    /// Whether Joel holds it, which makes it his to cancel or move.
    pub holds: bool,
    /// Who holds it, when it isn't Joel.
    pub organizer: Option<Person>,
    /// Everyone else asked, as the event lists them: neither Joel, nor
    /// who holds it, nor a room.
    pub guests: Vec<Guest>,
    /// When Google Calendar reminds Joel of it, in minutes before its
    /// start, furthest first: its own reminders, or the calendar's when it
    /// keeps those, and only those that pop up, since one by email is mail.
    pub reminders: Vec<i64>,
}

/// Someone on a meeting, by the name the calendar gives, or their address
/// when it gives none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Person {
    pub name: String,
    pub email: Option<String>,
}

/// Someone asked to a meeting, and how they answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Guest {
    pub name: String,
    pub email: Option<String>,
    pub answer: Answer,
}

/// How much of a description is kept: enough to read in the launcher,
/// never a whole pasted agenda.
pub const ABOUT_MOST: usize = 2000;

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
    let defaults = popups(answer.get("defaultReminders"));
    answer
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| meeting(event, zone, &defaults))
        .collect()
}

/// The minutes before its start each reminder in a list pops up, furthest
/// first, each once.
fn popups(reminders: Option<&Value>) -> Vec<i64> {
    let mut minutes: Vec<i64> = reminders
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|reminder| reminder.get("method").and_then(Value::as_str) == Some("popup"))
        .filter_map(|reminder| reminder.get("minutes").and_then(Value::as_i64))
        .filter(|minutes| *minutes >= 0)
        .collect();
    minutes.sort_unstable_by(|a, b| b.cmp(a));
    minutes.dedup();
    minutes
}

/// An event's reminders: its own overrides, or the calendar's defaults
/// when it keeps those, as it does when it says nothing of them.
fn reminded(event: &Value, defaults: &[i64]) -> Vec<i64> {
    match event.get("reminders") {
        Some(reminders) if reminders.get("useDefault").and_then(Value::as_bool) == Some(false) => {
            popups(reminders.get("overrides"))
        }
        _ => defaults.to_vec(),
    }
}

fn meeting(event: &Value, zone: &TimeZone, defaults: &[i64]) -> Option<Meeting> {
    let text = |field: &str| event.get(field).and_then(Value::as_str);
    if text("status") == Some("cancelled") {
        return None;
    }
    if text("eventType").is_some_and(|kind| kind != "default") {
        return None;
    }
    let (start, start_all_day) = when(event.get("start")?, zone)?;
    let (end, _) = when(event.get("end")?, zone)?;
    let attendees: &[Value] = event
        .get("attendees")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice);
    let is = |someone: &Value, what: &str| someone.get(what).and_then(Value::as_bool) == Some(true);
    let answer = attendees
        .iter()
        .find(|attendee| is(attendee, "self"))
        // A meeting Joel holds is his own, not an invitation to answer.
        .filter(|me| !is(me, "organizer"))
        .map(answered);
    let organizer = event.get("organizer");
    // Google names who holds each event; one it didn't would be Joel's
    // only with nobody else on it.
    let holds = organizer.map_or_else(
        || attendees.iter().all(|attendee| is(attendee, "self")),
        |organizer| is(organizer, "self"),
    );
    let guests = attendees
        .iter()
        .filter(|attendee| {
            !is(attendee, "self") && !is(attendee, "organizer") && !is(attendee, "resource")
        })
        .map(|attendee| {
            let person = person(attendee);
            Guest {
                name: person.name,
                email: person.email,
                answer: answered(attendee),
            }
        })
        .collect();
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
        place: text("location")
            .map(str::trim)
            .filter(|place| !place.is_empty())
            .map(str::to_owned),
        about: text("description")
            .map(plain)
            .filter(|about| !about.is_empty()),
        holds,
        organizer: organizer.filter(|_| !holds).map(person),
        guests,
        reminders: reminded(event, defaults),
    })
}

fn answered(attendee: &Value) -> Answer {
    match attendee.get("responseStatus").and_then(Value::as_str) {
        Some("accepted") => Answer::Yes,
        Some("tentative") => Answer::Maybe,
        Some("declined") => Answer::No,
        _ => Answer::Waiting,
    }
}

fn person(someone: &Value) -> Person {
    let text = |field: &str| {
        someone
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|said| !said.is_empty())
    };
    let email = text("email").map(str::to_owned);
    Person {
        name: text("displayName")
            .map(str::to_owned)
            .or_else(|| email.clone())
            .unwrap_or_default(),
        email,
    }
}

/// A description as plain text. Google Calendar writes one as a little
/// HTML: a line break or a paragraph's end becomes a new line, every other
/// tag goes, the few entities it writes are read, blank lines run to one,
/// and past `ABOUT_MOST` characters it's cut at the last word.
pub fn plain(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            rest = "";
            break;
        };
        let tag = rest[open + 1..open + close].trim().to_ascii_lowercase();
        let name = tag.trim_start_matches('/');
        let name = name.split([' ', '/']).next().unwrap_or("");
        // A line break always breaks; a list item or a block's end only
        // where a line hasn't just ended, so a list doesn't double them.
        let block = matches!(name, "li")
            || (tag.starts_with('/') && matches!(name, "p" | "div" | "ul" | "ol"));
        if name == "br" || (block && !text.is_empty() && !text.ends_with('\n')) {
            text.push('\n');
        }
        rest = &rest[open + close + 1..];
    }
    text.push_str(rest);
    let text = text
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&");
    // A paragraph after blank lines is set off by one, whatever ran
    // before it.
    let mut kept = String::with_capacity(text.len());
    let mut gap = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            gap = !kept.is_empty();
            continue;
        }
        if !kept.is_empty() {
            kept.push_str(if gap { "\n\n" } else { "\n" });
        }
        kept.push_str(line);
        gap = false;
    }
    if kept.chars().count() <= ABOUT_MOST {
        return kept;
    }
    let cut: String = kept.chars().take(ABOUT_MOST).collect();
    match cut.rfind(char::is_whitespace) {
        Some(space) => cut[..space].trim_end().to_owned(),
        None => cut,
    }
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
                place: None,
                about: None,
                holds: false,
                organizer: None,
                guests: vec![Guest {
                    name: "a@example.com".into(),
                    email: Some("a@example.com".into()),
                    answer: Answer::Yes,
                }],
                reminders: vec![],
            }]
        );
    }

    #[test]
    fn a_meeting_says_where_who_holds_it_who_else_is_asked_and_what_its_about() {
        let answer = json!({"items": [{
            "id": "e",
            "start": {"dateTime": "2026-10-26T16:00:00Z"},
            "end": {"dateTime": "2026-10-26T16:30:00Z"},
            "location": "  Room 4, Toronto  ",
            "description": "<p>Agenda:</p><ul><li>Numbers &amp; plans</li><li>Q&amp;A</li></ul>",
            "organizer": {"email": "ada@example.com", "displayName": "Ada Lovelace"},
            "attendees": [
                {"email": "ada@example.com", "displayName": "Ada Lovelace", "organizer": true,
                 "responseStatus": "accepted"},
                {"email": "joel@example.com", "self": true, "responseStatus": "needsAction"},
                {"email": "grace@example.com", "displayName": "Grace Hopper",
                 "responseStatus": "declined"},
                {"email": "room4@resource.example.com", "resource": true,
                 "responseStatus": "accepted"}
            ]
        }]});
        let found = &meetings(&answer, &zone())[0];
        assert_eq!(found.place.as_deref(), Some("Room 4, Toronto"));
        assert_eq!(
            found.about.as_deref(),
            Some("Agenda:\nNumbers & plans\nQ&A")
        );
        assert!(!found.holds);
        assert_eq!(
            found.organizer,
            Some(Person {
                name: "Ada Lovelace".into(),
                email: Some("ada@example.com".into()),
            })
        );
        assert_eq!(
            found.guests,
            vec![Guest {
                name: "Grace Hopper".into(),
                email: Some("grace@example.com".into()),
                answer: Answer::No,
            }]
        );
    }

    #[test]
    fn a_meeting_joel_holds_is_his_and_names_no_organizer() {
        let answer = json!({"items": [{"id": "e", "start": {"date": "2026-10-27"},
            "end": {"date": "2026-10-28"},
            "organizer": {"email": "joel@example.com", "self": true},
            "attendees": [{"email": "joel@example.com", "self": true, "organizer": true},
                {"email": "a@example.com", "responseStatus": "tentative"}]}]});
        let found = &meetings(&answer, &zone())[0];
        assert!(found.holds);
        assert_eq!(found.organizer, None);
        assert_eq!(found.guests.len(), 1);
        assert_eq!(found.guests[0].answer, Answer::Maybe);
    }

    #[test]
    fn a_description_reads_as_plain_text() {
        assert_eq!(plain("one<br>two<br><br><br>three"), "one\ntwo\n\nthree");
        assert_eq!(
            plain("<a href=\"https://example.com\">the doc</a>&nbsp;is &lt;here&gt;"),
            "the doc is <here>"
        );
        assert_eq!(plain("plain words\n\n\nkept"), "plain words\n\nkept");
        assert_eq!(plain("   <p> </p>  "), "");
        assert_eq!(plain("half a <b tag"), "half a");
        assert_eq!(plain("<p>one</p><p>two</p>"), "one\ntwo");
    }

    #[test]
    fn a_long_description_is_cut_at_a_word() {
        let long = "word ".repeat(ABOUT_MOST);
        let cut = plain(&long);
        assert!(cut.chars().count() <= ABOUT_MOST);
        assert!(cut.ends_with("word"));
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

    #[test]
    fn a_meeting_reminds_joel_as_it_or_its_calendar_says_by_popup_alone() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let answer = json!({
            "defaultReminders": [
                {"method": "popup", "minutes": 10},
                {"method": "email", "minutes": 60},
                {"method": "popup", "minutes": 30}
            ],
            "items": [
                {"id": "keeps", "start": at, "end": at, "reminders": {"useDefault": true}},
                {"id": "silent", "start": at, "end": at},
                {"id": "own", "start": at, "end": at, "reminders": {"useDefault": false,
                    "overrides": [{"method": "popup", "minutes": 5},
                        {"method": "popup", "minutes": 1440},
                        {"method": "popup", "minutes": 5},
                        {"method": "email", "minutes": 15}]}},
                {"id": "none", "start": at, "end": at, "reminders": {"useDefault": false}}
            ]
        });
        let reminders: Vec<_> = meetings(&answer, &zone())
            .into_iter()
            .map(|m| (m.id, m.reminders))
            .collect();
        assert_eq!(
            reminders,
            vec![
                ("keeps".into(), vec![30, 10]),
                ("silent".into(), vec![30, 10]),
                ("own".into(), vec![1440, 5]),
                ("none".into(), vec![]),
            ]
        );
    }

    #[test]
    fn a_calendar_with_no_reminders_reminds_of_nothing() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let found = &meetings(
            &json!({"items": [{"id": "x", "start": at, "end": at}]}),
            &zone(),
        )[0];
        assert_eq!(found.reminders, Vec::<i64>::new());
    }
}
