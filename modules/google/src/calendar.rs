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
//! description as plain text, the files on it and the number it's dialled
//! into by, how it repeats, when Joel is reminded of it, and the calendar
//! he may change it through, when Google lets him.
//!
//! An occurrence names its series but not how the series repeats, so the
//! watch asks each series' rule of its own and gives it beside the events
//! (`series`), as it gives the calendars Joel may write to.

use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::drive;
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
    /// How it repeats, for one of a series whose rule was heard.
    pub repeats: Option<Repeats>,
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
    /// The files on it, as Google Calendar lists them under it: a doc, a
    /// deck, an agenda.
    pub attachments: Vec<Attachment>,
    /// The number its call is dialled into by, when its conference has one.
    pub dial: Option<Dial>,
    /// Whether Joel holds it, which makes it his to cancel.
    pub holds: bool,
    /// The calendar Joel changes it through, as moving it does, when
    /// Google lets him: his own ("primary") for one he holds or whose
    /// guests may change it, or the calendar it lives on, its organizer's,
    /// when he may write to that; and none otherwise.
    pub through: Option<String>,
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

/// A file on a meeting: its title, where it opens, and, for one of Docs,
/// Sheets or Slides, Drive's id for it and its kind, which make it the
/// document it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Attachment {
    pub title: String,
    pub link: String,
    pub id: Option<String>,
    pub kind: Option<drive::Kind>,
}

/// A call's number to dial into it by, as its conference shows it, and the
/// PIN it then asks for, when it asks one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Dial {
    pub number: String,
    pub pin: Option<String>,
}

/// Someone asked to a meeting, and how they answered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Guest {
    pub name: String,
    pub email: Option<String>,
    pub answer: Answer,
}

/// How a series repeats, as its rule says it (RFC 5545's RRULE), in what
/// the shell words it from: every two weeks on Monday and Wednesday, every
/// month on the last Friday, until 31 December.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Repeats {
    pub every: Every,
    /// How many of them from one to the next: 2 for every other week.
    pub interval: u32,
    /// The days of the week it falls on, in the rule's order.
    pub days: Vec<Weekday>,
    /// The days of the month it falls on, -1 the last.
    #[serde(rename = "monthDays")]
    pub month_days: Vec<i8>,
    /// How many times it happens in all, when the rule says.
    pub times: Option<u32>,
    /// When it last may, in milliseconds since the epoch, when the rule
    /// says.
    pub until: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Every {
    Day,
    Week,
    Month,
    Year,
}

/// A day of the week a series falls on: 0 Sunday to 6 Saturday, as the
/// shell's dates count them, and which of them in its month when the rule
/// says, 1 the first and -1 the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Weekday {
    pub day: u8,
    pub nth: Option<i8>,
}

/// The days of the week as a rule writes them, from Sunday.
const WEEKDAYS: [&str; 7] = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"];

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

/// The meetings in the calendar's answer, in its order, which is by
/// start. Beside Google's events and reminders, the answer may say which
/// calendars Joel may write to (`writable`, from his calendar list), which
/// say what he may change; with none, he may change only his own.
pub fn meetings(answer: &Value, zone: &TimeZone) -> Vec<Meeting> {
    let defaults = popups(answer.get("defaultReminders"));
    let series = answer.get("series");
    let writable: Vec<&str> = answer
        .get("writable")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    answer
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| {
            let mut found = meeting(event, zone, &defaults, &writable)?;
            found.repeats = event
                .get("recurringEventId")
                .and_then(Value::as_str)
                .and_then(|id| series?.get(id)?.as_array())
                .and_then(|lines| {
                    let lines: Vec<&str> = lines.iter().filter_map(Value::as_str).collect();
                    repeats(&lines, zone)
                });
            Some(found)
        })
        .collect()
}

/// The series the answer's meetings are of, each once, in their order:
/// those the watch asks the rules of.
pub fn repeating(answer: &Value) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let series = answer
        .get("items")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| event.get("recurringEventId").and_then(Value::as_str));
    for id in series {
        if !found.iter().any(|kept| kept == id) {
            found.push(id.to_owned());
        }
    }
    found
}

/// How a series repeats, from its `recurrence` lines, read in `zone` when
/// its end is said in no zone of its own. A series of more than one rule,
/// or none, says nothing, rather than half of it. A rule holding a part
/// this doesn't read (as which of a set, BYSETPOS, or a month, BYMONTH)
/// keeps only how often, never days it might not fall on.
pub fn repeats(lines: &[&str], zone: &TimeZone) -> Option<Repeats> {
    let mut rules = lines.iter().filter_map(|line| line.strip_prefix("RRULE:"));
    let rule = rules.next()?;
    if rules.next().is_some() {
        return None;
    }
    let mut every = None;
    let mut found = Repeats {
        every: Every::Day,
        interval: 1,
        days: Vec::new(),
        month_days: Vec::new(),
        times: None,
        until: None,
    };
    let mut unread = false;
    for part in rule.split(';') {
        let Some((name, value)) = part.split_once('=') else {
            unread = true;
            continue;
        };
        let read = match name.to_ascii_uppercase().as_str() {
            "FREQ" => {
                every = match value.to_ascii_uppercase().as_str() {
                    "DAILY" => Some(Every::Day),
                    "WEEKLY" => Some(Every::Week),
                    "MONTHLY" => Some(Every::Month),
                    "YEARLY" => Some(Every::Year),
                    // More often than daily isn't a meeting's.
                    _ => return None,
                };
                true
            }
            "INTERVAL" => value
                .parse()
                .ok()
                .filter(|interval| *interval > 0)
                .map(|interval| found.interval = interval)
                .is_some(),
            "BYDAY" => value
                .split(',')
                .map(weekday)
                .collect::<Option<Vec<_>>>()
                .map(|days| found.days = days)
                .is_some(),
            "BYMONTHDAY" => value
                .split(',')
                .map(|day| day.parse().ok().filter(|day: &i8| *day != 0))
                .collect::<Option<Vec<_>>>()
                .map(|days| found.month_days = days)
                .is_some(),
            "COUNT" => value
                .parse()
                .ok()
                .map(|times| found.times = Some(times))
                .is_some(),
            "UNTIL" => until(value, zone)
                .map(|until| found.until = Some(until))
                .is_some(),
            // Which day a week starts on changes nothing said here.
            "WKST" => true,
            _ => false,
        };
        unread |= !read;
    }
    found.every = every?;
    if unread {
        found.days.clear();
        found.month_days.clear();
    }
    Some(found)
}

/// A day of the week as a rule writes it, MO, or with which of them in its
/// month, 1MO or -1FR.
fn weekday(written: &str) -> Option<Weekday> {
    let at = written.len().checked_sub(2)?;
    let (nth, code) = written.split_at_checked(at)?;
    let day = WEEKDAYS
        .iter()
        .position(|day| day.eq_ignore_ascii_case(code))?;
    let nth = match nth.trim_start_matches('+') {
        "" => None,
        nth => Some(nth.parse().ok().filter(|nth: &i8| *nth != 0)?),
    };
    Some(Weekday {
        day: u8::try_from(day).ok()?,
        nth,
    })
}

/// When a rule's UNTIL is, as 20261231T045959Z, a time in no zone of its
/// own, read in `zone`, as 20261231T235959, or a day, its midnight here.
fn until(written: &str, zone: &TimeZone) -> Option<i64> {
    let number = |from: usize, to: usize| written.get(from..to)?.parse::<i16>().ok();
    let day = Date::new(
        number(0, 4)?,
        i8::try_from(number(4, 6)?).ok()?,
        i8::try_from(number(6, 8)?).ok()?,
    )
    .ok()?;
    let Some(clock) = written.get(8..).filter(|clock| !clock.is_empty()) else {
        return Some(midnight(day, zone).as_millisecond());
    };
    if !clock.starts_with('T') {
        return None;
    }
    let part = |from: usize| i8::try_from(number(from, from + 2)?).ok();
    let time = day.to_datetime(Time::new(part(9)?, part(11)?, part(13)?, 0).ok()?);
    let zone = match clock.get(7..)? {
        "Z" => TimeZone::UTC,
        "" => zone.clone(),
        _ => return None,
    };
    Some(time.to_zoned(zone).ok()?.timestamp().as_millisecond())
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

fn meeting(event: &Value, zone: &TimeZone, defaults: &[i64], writable: &[&str]) -> Option<Meeting> {
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
    let through = if holds || is(event, "guestsCanModify") {
        Some("primary".to_owned())
    } else {
        organizer
            .and_then(|organizer| organizer.get("email"))
            .and_then(Value::as_str)
            .filter(|calendar| writable.contains(calendar))
            .map(str::to_owned)
    };
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
        repeats: None,
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
        attachments: attached(event),
        dial: dialled(event),
        holds,
        through,
        organizer: organizer.filter(|_| !holds).map(person),
        guests,
        reminders: reminded(event, defaults),
    })
}

/// The files on an event, each once by where it opens, in its order,
/// leaving out any with no title to call it by or no link to open.
fn attached(event: &Value) -> Vec<Attachment> {
    let mut files: Vec<Attachment> = Vec::new();
    let found = event
        .get("attachments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(attachment);
    for file in found {
        if !files.iter().any(|kept| kept.link == file.link) {
            files.push(file);
        }
    }
    files
}

fn attachment(file: &Value) -> Option<Attachment> {
    let text = |field: &str| {
        file.get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|said| !said.is_empty())
    };
    let kind = text("mimeType").and_then(|mime| {
        drive::Kind::ALL
            .into_iter()
            .find(|kind| kind.mime() == mime)
    });
    Some(Attachment {
        title: text("title")?.to_owned(),
        link: text("fileUrl")?.to_owned(),
        id: kind.and(text("fileId")).map(str::to_owned),
        kind: kind.filter(|_| text("fileId").is_some()),
    })
}

/// The first number an event's conference is dialled into by: its label
/// as Google shows it, else its own `tel:` link's, and its PIN.
fn dialled(event: &Value) -> Option<Dial> {
    let point = event
        .pointer("/conferenceData/entryPoints")
        .and_then(Value::as_array)?
        .iter()
        .find(|point| point.get("entryPointType").and_then(Value::as_str) == Some("phone"))?;
    let text = |field: &str| {
        point
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|said| !said.is_empty())
    };
    let number = text("label").or_else(|| text("uri").map(|uri| uri.trim_start_matches("tel:")))?;
    Some(Dial {
        number: number.to_owned(),
        pin: text("pin").map(str::to_owned),
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
    clipped(kept, ABOUT_MOST)
}

/// Text at most `most` characters long, cut at the last word before.
pub fn clipped(text: String, most: usize) -> String {
    if text.chars().count() <= most {
        return text;
    }
    let cut: String = text.chars().take(most).collect();
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

    fn weekly(days: &[(u8, Option<i8>)]) -> Vec<Weekday> {
        days.iter()
            .map(|&(day, nth)| Weekday { day, nth })
            .collect()
    }

    #[test]
    fn a_meeting_of_a_series_says_how_the_series_repeats() {
        let answer = json!({
            "items": [
                {"id": "s_1", "recurringEventId": "s", "summary": "Standup",
                    "start": {"dateTime": "2026-10-26T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-26T16:15:00-04:00"}},
                {"id": "s_2", "recurringEventId": "s", "summary": "Standup",
                    "start": {"dateTime": "2026-10-28T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-28T16:15:00-04:00"}},
                {"id": "q_1", "recurringEventId": "q", "summary": "Unheard",
                    "start": {"dateTime": "2026-10-27T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-27T16:15:00-04:00"}},
                {"id": "once", "summary": "Once",
                    "start": {"dateTime": "2026-10-29T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-29T16:15:00-04:00"}}
            ],
            "series": {"s": ["EXDATE;TZID=America/Toronto:20261102T160000",
                "RRULE:FREQ=WEEKLY;WKST=SU;INTERVAL=2;BYDAY=MO,WE"]}
        });
        assert_eq!(repeating(&answer), ["s", "q"]);
        let found = meetings(&answer, &zone());
        let said: Vec<_> = found
            .iter()
            .map(|meeting| meeting.repeats.clone())
            .collect();
        let fortnightly = Repeats {
            every: Every::Week,
            interval: 2,
            days: weekly(&[(1, None), (3, None)]),
            month_days: vec![],
            times: None,
            until: None,
        };
        assert_eq!(
            said,
            [Some(fortnightly.clone()), Some(fortnightly), None, None]
        );
    }

    #[test]
    fn a_rule_says_which_of_a_day_in_its_month_and_when_it_ends() {
        let rule = |line: &str| repeats(&[line], &zone()).unwrap();
        let monthly = rule("RRULE:FREQ=MONTHLY;BYDAY=-1FR;COUNT=6");
        assert_eq!((monthly.every, monthly.times), (Every::Month, Some(6)));
        assert_eq!(monthly.days, weekly(&[(5, Some(-1))]));
        assert_eq!(
            rule("RRULE:FREQ=MONTHLY;BYDAY=+1MO").days,
            weekly(&[(1, Some(1))])
        );
        assert_eq!(
            rule("RRULE:FREQ=MONTHLY;BYMONTHDAY=15,-1").month_days,
            [15, -1]
        );
        // At 04:59:59 in Greenwich, 00:59:59 here.
        assert_eq!(
            rule("RRULE:FREQ=DAILY;UNTIL=20261231T045959Z").until,
            Some(1_798_693_199_000)
        );
        assert_eq!(
            rule("RRULE:FREQ=DAILY;UNTIL=20261231T005959").until,
            Some(1_798_693_199_000)
        );
        assert_eq!(
            rule("RRULE:FREQ=YEARLY;UNTIL=20261231").until,
            Some(1_798_689_600_000)
        );
    }

    #[test]
    fn a_rule_says_nothing_it_cant_read_wholly() {
        let rule = |lines: &[&str]| repeats(lines, &zone());
        // The last weekday of each month: without reading which of the set,
        // its days would be wrong, so only how often is said.
        let last = rule(&["RRULE:FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1"]).unwrap();
        assert_eq!((last.every, last.days.len()), (Every::Month, 0));
        assert_eq!(rule(&["RRULE:FREQ=WEEKLY;BYDAY=XX"]).unwrap().days, []);
        assert_eq!(rule(&["RRULE:FREQ=HOURLY"]), None);
        assert_eq!(rule(&["RRULE:INTERVAL=2"]), None);
        assert_eq!(rule(&["EXDATE:20261102T160000Z"]), None);
        assert_eq!(rule(&["RRULE:FREQ=DAILY", "RRULE:FREQ=WEEKLY"]), None);
        assert_eq!(rule(&["RRULE:FREQ=DAILY;UNTIL=2026"]).unwrap().until, None);
        assert_eq!(
            rule(&["RRULE:FREQ=DAILY;UNTIL=20261231T2"]).unwrap().until,
            None
        );
        assert_eq!(
            rule(&["RRULE:FREQ=DAILY;UNTIL=20261231T256060Z"])
                .unwrap()
                .until,
            None
        );
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
                repeats: None,
                answer: Some(Answer::Maybe),
                link: Some("https://meet.google.com/abc-defg-hij".into()),
                kind: Some(Kind::Meet),
                page: Some("https://www.google.com/calendar/event?eid=e1".into()),
                place: None,
                about: None,
                attachments: vec![],
                dial: None,
                holds: false,
                through: None,
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
    fn a_meeting_names_its_files_and_the_number_its_call_is_dialled_into_by() {
        let answer = json!({"items": [{"id": "e",
        "start": {"dateTime": "2026-10-26T16:00:00Z"},
        "end": {"dateTime": "2026-10-26T16:30:00Z"},
        "attachments": [
            {"fileUrl": "https://docs.google.com/document/d/1", "title": "Agenda",
             "fileId": "1", "mimeType": "application/vnd.google-apps.document"},
            {"fileUrl": "https://docs.google.com/document/d/1", "title": "Agenda",
             "fileId": "1", "mimeType": "application/vnd.google-apps.document"},
            {"fileUrl": "https://drive.google.com/file/d/2", "title": " "},
            {"fileUrl": "https://drive.google.com/file/d/4", "title": "Menu",
             "fileId": "4", "mimeType": "application/pdf"},
            {"title": "Nowhere"}
        ],
        "conferenceData": {"entryPoints": [
            {"entryPointType": "video", "uri": "https://meet.google.com/abc-defg-hij"},
            {"entryPointType": "phone", "uri": "tel:+1-555-010-0199",
             "label": "+1 555-010-0199", "pin": "123456789"},
            {"entryPointType": "phone", "uri": "tel:+44-20-0000-0000"}
        ]}}]});
        let found = &meetings(&answer, &zone())[0];
        assert_eq!(
            found.attachments,
            [
                Attachment {
                    title: "Agenda".into(),
                    link: "https://docs.google.com/document/d/1".into(),
                    id: Some("1".into()),
                    kind: Some(drive::Kind::Document),
                },
                Attachment {
                    title: "Menu".into(),
                    link: "https://drive.google.com/file/d/4".into(),
                    id: None,
                    kind: None,
                }
            ]
        );
        assert_eq!(
            found.dial,
            Some(Dial {
                number: "+1 555-010-0199".into(),
                pin: Some("123456789".into()),
            })
        );
    }

    #[test]
    fn a_number_without_a_label_is_its_link_and_one_without_a_conference_is_none() {
        let answer = json!({"items": [
            {"id": "a", "start": {"dateTime": "2026-10-26T16:00:00Z"},
             "end": {"dateTime": "2026-10-26T16:30:00Z"},
             "conferenceData": {"entryPoints": [
                {"entryPointType": "phone", "uri": "tel:+1-555-010-0199"}]}},
            {"id": "b", "start": {"dateTime": "2026-10-26T17:00:00Z"},
             "end": {"dateTime": "2026-10-26T17:30:00Z"}}
        ]});
        let found = meetings(&answer, &zone());
        assert_eq!(
            found[0].dial,
            Some(Dial {
                number: "+1-555-010-0199".into(),
                pin: None,
            })
        );
        assert_eq!(found[1].dial, None);
        assert!(found[1].attachments.is_empty());
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

    #[test]
    fn a_meeting_is_changed_through_a_calendar_google_lets_joel_change_it_on() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let by = |email: &str, mine: bool| json!({"email": email, "self": mine});
        let answer = json!({
            "writable": ["team@group.calendar.google.com"],
            "items": [
                {"id": "his", "start": at, "end": at, "organizer": by("joel@example.com", true)},
                {"id": "open", "start": at, "end": at, "guestsCanModify": true,
                    "organizer": by("ada@example.com", false)},
                {"id": "team", "start": at, "end": at,
                    "organizer": by("team@group.calendar.google.com", false)},
                {"id": "ada", "start": at, "end": at, "organizer": by("ada@example.com", false)},
                {"id": "closed", "start": at, "end": at, "guestsCanModify": false,
                    "organizer": by("grace@example.com", false)}
            ]
        });
        let through: Vec<_> = meetings(&answer, &zone())
            .into_iter()
            .map(|m| (m.id, m.through))
            .collect();
        assert_eq!(
            through,
            vec![
                ("his".into(), Some("primary".into())),
                ("open".into(), Some("primary".into())),
                ("team".into(), Some("team@group.calendar.google.com".into())),
                ("ada".into(), None),
                ("closed".into(), None),
            ]
        );
    }

    #[test]
    fn with_no_calendar_list_only_his_own_is_his_to_change() {
        let at = json!({"dateTime": "2026-10-26T16:00:00Z"});
        let found = &meetings(
            &json!({"items": [{"id": "team", "start": at, "end": at,
                "organizer": {"email": "team@group.calendar.google.com"}}]}),
            &zone(),
        )[0];
        assert_eq!(found.through, None);
    }
}
