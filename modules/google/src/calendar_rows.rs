//! Calendar's rows, as the launcher shows them, made here rather than in
//! the shell (Joel: "the daemons need to be the data layer"): each meeting
//! from now to a week on, soonest first, in Calendar (`calendar`), and the
//! meetings Joel held lately, newest first and a series once at its
//! latest, in Calendar's Recent (`held`), and the calls among those on a
//! Meet link, once by code, in Meet's Recent (`meet`), each worded as
//! Calendar.qml, Calendar.js and Meet.js word it, with its acts in the one order every surface
//! lists them and its drawer a record of its own.
//!
//! What reads the clock travels as a time for the shell to word: a
//! meeting's side is `{at, as: "meeting", until, allDay}`, which it says
//! as "in 12 min", "now" or "yesterday 15:00", and `joins` says from when
//! until when Return joins its call rather than showing it, so no row
//! changes each minute. What changes only as a meeting ends, its note and
//! whether it's listed, is made against the `now` each row is made at,
//! which the daemon's look each minute moves on.

use std::cmp::Reverse;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use crate::calendar::{Answer, Every, Guest, Meeting, Person, Repeats};
use crate::rows;

/// How soon before its start Return on a meeting joins its call rather
/// than showing it: ten minutes (Calendar.js's `TAKES_OVER`).
const TAKES_OVER: i64 = 600_000;

/// As many as a Recent lists (launcher/Recent.js), so none is made only
/// to be dropped.
const RECENT: usize = 9;

/// Between the items of one run, as every side and fact joins them.
const BETWEEN: &str = " \u{b7} ";

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Which of a day in its month, from the first.
const WHICH: [&str; 6] = ["", "first", "second", "third", "fourth", "fifth"];

/// The words Calendar's rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// A row's name for a meeting with no title.
    pub untitled: String,
    /// The glyph role every meeting's row shows.
    pub glyph: String,
    /// "{count} in the past 30 days", how many of a series came before.
    pub earlier: String,
    /// What Cancel's sure row starts with, "Cancel It Now".
    pub cancel_sure: String,
    /// What Move's sure row starts with, "Move It".
    pub move_sure: String,
    /// The glyph role every call in Meet's Recent shows.
    pub call_glyph: String,
    /// "{first} and {second}", who a call was with.
    pub with_two: String,
    /// "{first} and {count} others".
    pub with_others: String,
}

impl Words {
    /// Calendar's words from the whole vocabulary, or the first one it
    /// lacks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let word = |id: &str, at: &[&str]| -> Result<String, String> {
            let thing = vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))?;
            at.iter()
                .fold(thing, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        Ok(Words {
            untitled: word("calendar:meeting", &["states", "untitled", "name"])?,
            glyph: word("calendar:meeting", &["glyph"])?,
            earlier: word("calendar:meeting", &["said", "earlier"])?,
            cancel_sure: word("verb:calendar:cancel", &["sure"])?,
            move_sure: word("verb:calendar:move", &["sure"])?,
            call_glyph: word("meet:held", &["glyph"])?,
            with_two: word("meet:held", &["said", "two"])?,
            with_others: word("meet:held", &["said", "others"])?,
        })
    }
}

/// The places meetings are listed in: Calendar, its Recent, and Meet's.
const PLACES: [&str; 3] = ["calendar", "held", "meet"];

/// A meeting as a place lists it: the place, what it's keyed by there (its
/// id, or in Meet's Recent its call's code), the meeting, and for one of a
/// series in Calendar's Recent, how many of it came before.
struct Listed<'a> {
    place: &'static str,
    id: String,
    meeting: &'a Meeting,
    earlier: usize,
}

/// Every meeting the three places list, each place in its order: those
/// not over, soonest first; then those held lately (Google.js's
/// `heldLately` and `seriesOnce`), over and not turned down, newest first,
/// each once by its id and a series once at its latest; then the calls
/// among those held lately on a Meet link, each once by its code, where
/// it was newest (Meet.js's `called`).
fn listed<'a>(meetings: &'a [Meeting], held: &'a [Meeting], now: i64) -> Vec<Listed<'a>> {
    let mut coming: Vec<&Meeting> = meetings
        .iter()
        .filter(|meeting| meeting.end > now)
        .collect();
    coming.sort_by_key(|meeting| meeting.start);
    let mut lately: Vec<&Meeting> = held
        .iter()
        .chain(meetings)
        .filter(|meeting| meeting.end <= now && meeting.answer != Some(Answer::No))
        .collect();
    lately.sort_by_key(|meeting| Reverse(meeting.start));
    let mut ids: Vec<&str> = Vec::new();
    lately.retain(|&meeting| {
        let new = !ids.contains(&meeting.id.as_str());
        ids.push(&meeting.id);
        new
    });
    let mut codes: Vec<String> = Vec::new();
    let calls: Vec<Listed> = lately
        .iter()
        .copied()
        .filter_map(|meeting| {
            let code = meeting.link.as_deref().and_then(meet_code)?;
            if codes.contains(&code) {
                return None;
            }
            codes.push(code.clone());
            Some(Listed {
                place: "meet",
                id: code,
                meeting,
                earlier: 0,
            })
        })
        .take(RECENT)
        .collect();
    let mut series: Vec<(&str, Listed)> = Vec::new();
    for meeting in lately {
        let of = meeting.series.as_deref().unwrap_or(&meeting.id);
        match series.iter_mut().find(|(each, _)| *each == of) {
            Some((_, latest)) => latest.earlier += 1,
            None => series.push((
                of,
                Listed {
                    place: "held",
                    id: meeting.id.clone(),
                    meeting,
                    earlier: 0,
                },
            )),
        }
    }
    coming
        .into_iter()
        .map(|meeting| Listed {
            place: "calendar",
            id: meeting.id.clone(),
            meeting,
            earlier: 0,
        })
        .chain(series.into_iter().take(RECENT).map(|(_, held)| held))
        .chain(calls)
        .collect()
}

/// The code a Meet link joins by, as abc-defg-hij, before any query that
/// follows it; none for any other link (Meet.js's `meetCode`).
fn meet_code(link: &str) -> Option<String> {
    let path = link.strip_prefix("https://meet.google.com/")?;
    let code: String = path
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || *c == '-')
        .collect();
    let mut parts = Vec::new();
    let mut rest = code.as_str();
    for (at, length) in [3, 4, 3].into_iter().enumerate() {
        if at > 0 {
            rest = rest.strip_prefix('-').unwrap_or(rest);
        }
        let part = rest.get(..length).filter(|part| !part.contains('-'))?;
        parts.push(part);
        rest = &rest[length..];
    }
    rest.is_empty().then(|| parts.join("-"))
}

/// Every meeting's row in Calendar, its Recent and Meet's, each place's in
/// its order, worded in the zone Joel's clock reads.
pub fn meetings(
    meetings: &[Meeting],
    held: &[Meeting],
    now: i64,
    zone: &TimeZone,
    words: &Words,
) -> Vec<Value> {
    let listed = listed(meetings, held, now);
    let mut orders = [0usize; PLACES.len()];
    listed
        .iter()
        .map(|each| {
            let at = PLACES
                .iter()
                .position(|place| *place == each.place)
                .unwrap_or(0);
            let order = orders[at];
            orders[at] += 1;
            row(each, order, now, zone, words)
        })
        .collect()
}

/// Each of those rows' drawers, as records of their own the launcher
/// watches only while one is shown, by the key its row names.
pub fn drawers(
    meetings: &[Meeting],
    held: &[Meeting],
    now: i64,
    zone: &TimeZone,
    words: &Words,
) -> Vec<(String, Value)> {
    listed(meetings, held, now)
        .iter()
        .map(|each| {
            let key = format!("drawer:{}:{}", each.place, each.id);
            (key, drawer(each, zone, words))
        })
        .collect()
}

fn titled<'a>(meeting: &'a Meeting, words: &'a Words) -> &'a str {
    if meeting.title.is_empty() {
        &words.untitled
    } else {
        &meeting.title
    }
}

/// A meeting's row, or in Meet's Recent its call's (Meet.qml's
/// `calledRow`): named for its meeting, who it was with and when it was
/// held beside it, its times below, found by its guests' addresses too.
fn row(listed: &Listed, order: usize, now: i64, zone: &TimeZone, words: &Words) -> Value {
    let meeting = listed.meeting;
    let id = &meeting.id;
    let call = listed.place == "meet";
    let joins = (meeting.link.is_some() && !meeting.all_day)
        .then(|| json!({"from": meeting.start - TAKES_OVER, "until": meeting.end}));
    // Its times and his answer, or nothing once it's over, where its side
    // says when it was (Calendar.js's noted); a call its times alone.
    let note = if call {
        span(meeting, zone)
    } else if meeting.end <= now {
        String::new()
    } else {
        about(meeting, zone)
    };
    let when = json!({
        "at": meeting.start,
        "as": "meeting",
        "until": meeting.end,
        "allDay": meeting.all_day,
    });
    let with = if call {
        with_whom(meeting, words)
    } else {
        String::new()
    };
    let side: Vec<Value> = (!with.is_empty())
        .then(|| json!({"said": with}))
        .into_iter()
        .chain([when])
        .collect();
    let emails: Vec<&str> = meeting
        .guests
        .iter()
        .filter_map(|guest| guest.email.as_deref())
        .collect();
    let keywords = (call && !emails.is_empty()).then(|| emails.join(" "));
    let (thing, kind, glyph) = if call {
        (
            format!("meet:held:{}", listed.id),
            "call",
            &words.call_glyph,
        )
    } else {
        (format!("calendar:meeting:{id}"), "meeting", &words.glyph)
    };
    json!({
        "key": format!("row:{}:{}", listed.place, listed.id),
        "row": thing,
        "place": listed.place,
        "kind": kind,
        "order": order,
        "name": titled(meeting, words),
        "glyph": glyph,
        "hue": "program",
        "depth": 0,
        "side": side,
        "note": note,
        "keywords": keywords,
        // Whether it's today's, so Calendar's row counts today's meetings.
        "today": on_or_before(meeting.start, now, zone),
        "joins": joins,
        "drawer": format!("drawer:{}:{}", listed.place, listed.id),
        "acts": acts(meeting, words),
        "held": {
            "id": id,
            "link": meeting.link,
            "page": meeting.page,
            "start": meeting.start,
            "end": meeting.end,
            "allDay": meeting.all_day,
            "through": meeting.through,
            "series": meeting.series,
            "earlier": listed.earlier,
            "attachments": meeting.attachments,
        },
    })
}

/// Its acts, in the one order every surface lists them (Calendar.qml's
/// acts): Join the Meeting, Write to the Guests, Show in Calendar, Open
/// the Transcript, Going, Maybe and Not Going, Move the Meeting, Guests,
/// Attachments, Cancel the Meeting and Copy the Link. One the daemon does
/// carries its `ask`; one that steps in names the launcher's `step`, with
/// what that step needs. Cancel asks once, its sure row's name said here.
fn acts(meeting: &Meeting, words: &Words) -> Vec<Value> {
    let id = &meeting.id;
    let mut acts = Vec::new();
    if let Some(link) = &meeting.link {
        acts.push(json!({"key": "verb:calendar:join", "ask": {"open": {"link": link}}}));
    }
    let emails = guest_emails(meeting);
    let writing = rows::writing(&emails, &meeting.title, &[]);
    if !writing.is_empty() {
        acts.push(json!({"key": "verb:calendar:write", "ask": {"open": {"link": writing}}}));
    }
    acts.push(json!({"key": "verb:calendar:see", "step": "see"}));
    // Only a call is written down, and call-transcripts says whether it was.
    if meeting.link.is_some() {
        acts.push(json!({"key": "verb:calendar:transcript", "step": "transcript"}));
    }
    if meeting.answer.is_some() {
        for (said, answer) in [
            ("yes", Answer::Yes),
            ("maybe", Answer::Maybe),
            ("no", Answer::No),
        ] {
            acts.push(json!({
                "key": format!("verb:calendar:{said}"),
                "ask": {"answer": {"id": id, "with": said}},
                "given": meeting.answer == Some(answer),
            }));
        }
    }
    if meeting.through.is_some() && !meeting.all_day {
        acts.push(json!({
            "key": "verb:calendar:move",
            "step": "move",
            "sure": words.move_sure,
            "telling": telling_them(meeting),
        }));
    }
    if !emails.is_empty() {
        acts.push(json!({"key": "verb:calendar:guests", "step": "guests", "emails": emails}));
    }
    if !meeting.attachments.is_empty() {
        acts.push(json!({
            "key": "verb:calendar:attachments",
            "step": "attachments",
            "count": meeting.attachments.len(),
        }));
    }
    if meeting.holds {
        let told = match meeting.guests.len() {
            0 => "Telling Nobody Else".to_owned(),
            1 => "Telling 1 Guest".to_owned(),
            asked => format!("Telling {asked} Guests"),
        };
        acts.push(json!({
            "key": "verb:calendar:cancel",
            "ask": {"cancel": {"id": id}},
            "sure": format!("{}, {told}", words.cancel_sure),
        }));
    }
    if let Some(link) = meeting.link.as_ref().or(meeting.page.as_ref()) {
        let side = if meeting.link.is_some() {
            "its call"
        } else {
            "its page"
        };
        acts.push(json!({"key": "verb:calendar:link", "copy": link, "side": side}));
    }
    acts
}

/// The addresses of those on a meeting besides Joel: who holds it, unless
/// he does, then each guest with one.
fn guest_emails(meeting: &Meeting) -> Vec<&str> {
    let organizer = if meeting.holds {
        None
    } else {
        meeting
            .organizer
            .as_ref()
            .and_then(|person| person.email.as_deref())
    };
    organizer
        .into_iter()
        .chain(
            meeting
                .guests
                .iter()
                .filter_map(|guest| guest.email.as_deref()),
        )
        .collect()
}

/// Who a call was with, as its row says beside it: whoever held its
/// meeting where that wasn't Joel, then each guest but those who said no,
/// each by name or else address: "Ada Lovelace", "Ada Lovelace and Grace
/// Hopper", "Ada Lovelace and 2 others"; nothing for a call nobody else
/// was asked to (Meet.js's `withWhom`).
fn with_whom(meeting: &Meeting, words: &Words) -> String {
    let holder = if meeting.holds {
        None
    } else {
        meeting.organizer.as_ref()
    };
    let names: Vec<String> = holder
        .map(named)
        .into_iter()
        .chain(
            meeting
                .guests
                .iter()
                .filter(|guest| guest.answer != Answer::No)
                .map(|guest| {
                    if guest.name.is_empty() {
                        guest.email.clone().unwrap_or_default()
                    } else {
                        guest.name.clone()
                    }
                }),
        )
        .filter(|name| !name.is_empty())
        .collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => words
            .with_two
            .replace("{first}", first)
            .replace("{second}", second),
        [first, rest @ ..] => words
            .with_others
            .replace("{first}", first)
            .replace("{count}", &rest.len().to_string()),
    }
}

/// Who Google tells of a change to a meeting, as Move's sure row says it:
/// "Telling Nobody Else", "Telling Ada and Grace", or the first two and
/// how many more (Calendar.js's `tellingThem`).
fn telling_them(meeting: &Meeting) -> String {
    let people: Vec<&str> = meeting
        .organizer
        .iter()
        .map(|person| person.name.as_str())
        .chain(meeting.guests.iter().map(|guest| guest.name.as_str()))
        .collect();
    match people.as_slice() {
        [] => "Telling Nobody Else".to_owned(),
        [one] => format!("Telling {one}"),
        [one, other] => format!("Telling {one} and {other}"),
        [one, other, rest @ ..] => {
            let others = if rest.len() == 1 { "Other" } else { "Others" };
            format!("Telling {one}, {other} and {} {others}", rest.len())
        }
    }
}

/// Its details, as the launcher's drawer shows them beside its row
/// (Calendar.js's details): when, how it repeats, how many of its series
/// came before, where, its call and the number it's dialled into by, his
/// answer, who holds it, how those asked answered and who they are, and
/// the files on it, every one plain text; and what it's about below.
fn drawer(listed: &Listed, zone: &TimeZone, words: &Words) -> Value {
    let meeting = listed.meeting;
    let earlier = if listed.earlier > 0 {
        words
            .earlier
            .replace("{count}", &listed.earlier.to_string())
    } else {
        String::new()
    };
    let titles: Vec<&str> = meeting
        .attachments
        .iter()
        .map(|file| file.title.as_str())
        .collect();
    let attachments = if titles.len() == 1 {
        "Attachment"
    } else {
        "Attachments"
    };
    let asked: Vec<String> = meeting.guests.iter().map(guest_asked).collect();
    let facts = [
        ("When", dated(meeting, zone)),
        (
            "Repeats",
            meeting
                .repeats
                .as_ref()
                .map(|rule| repeated(rule, zone))
                .unwrap_or_default(),
        ),
        ("Earlier", earlier),
        ("Where", meeting.place.clone().unwrap_or_default()),
        ("Call", meeting.link.clone().unwrap_or_default()),
        (
            "Dial In",
            meeting
                .dial
                .as_ref()
                .map_or_else(String::new, |dial| match &dial.pin {
                    Some(pin) => format!("{}, PIN {pin}", dial.number),
                    None => dial.number.clone(),
                }),
        ),
        ("Your Answer", answered(meeting.answer).to_owned()),
        (
            "Holds It",
            meeting.organizer.as_ref().map(named).unwrap_or_default(),
        ),
        ("Answers", tallied(&meeting.guests)),
        ("Asked", asked.join("; ")),
        (attachments, titles.join(", ")),
    ];
    json!({
        "title": titled(meeting, words),
        "facts": facts
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(label, value)| json!({"label": label, "value": value}))
            .collect::<Vec<_>>(),
        "body": meeting.about.clone().unwrap_or_default(),
    })
}

/// How Joel answered an invitation, as his calendar would say it to him,
/// or nothing for a meeting that isn't one.
fn answered(answer: Option<Answer>) -> &'static str {
    match answer {
        Some(Answer::Yes) => "going",
        Some(Answer::Maybe) => "maybe",
        Some(Answer::No) => "not going",
        Some(Answer::Waiting) => "not answered",
        None => "",
    }
}

/// Someone by name, or their address when the calendar gives no name.
fn named(person: &Person) -> String {
    if person.name.is_empty() {
        person.email.clone().unwrap_or_default()
    } else {
        person.name.clone()
    }
}

/// A guest by name, with how they answered after a dot.
fn guest_asked(guest: &Guest) -> String {
    let name = if guest.name.is_empty() {
        guest.email.as_deref().unwrap_or_default()
    } else {
        &guest.name
    };
    format!("{name}{BETWEEN}{}", answered(Some(guest.answer)))
}

/// How those asked answered, counted, as a calendar heads its list of
/// guests: "9 going, 2 not going, 3 not answered", in the order an
/// invitation offers them; nothing for one guest, whom Asked says whole.
fn tallied(guests: &[Guest]) -> String {
    if guests.len() < 2 {
        return String::new();
    }
    [Answer::Yes, Answer::Maybe, Answer::No, Answer::Waiting]
        .into_iter()
        .filter_map(|answer| {
            let count = guests.iter().filter(|guest| guest.answer == answer).count();
            (count > 0).then(|| format!("{count} {}", answered(Some(answer))))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Its times and, for an invitation, his answer: "14:00 to 14:30 · maybe".
fn about(meeting: &Meeting, zone: &TimeZone) -> String {
    [span(meeting, zone), answered(meeting.answer).to_owned()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(BETWEEN)
}

/// Its times on Joel's clock, "14:00 to 14:30", or "all day".
fn span(meeting: &Meeting, zone: &TimeZone) -> String {
    if meeting.all_day {
        return "all day".to_owned();
    }
    format!(
        "{} to {}",
        clock(meeting.start, zone),
        clock(meeting.end, zone)
    )
}

/// Its day and times in full: "Monday 26 October · 14:00 to 14:30".
fn dated(meeting: &Meeting, zone: &TimeZone) -> String {
    let Ok(at) = Timestamp::from_millisecond(meeting.start) else {
        return String::new();
    };
    let day = at.to_zoned(zone.clone());
    format!(
        "{} {} {}{BETWEEN}{}",
        DAYS[usize::from(day.weekday().to_sunday_zero_offset().unsigned_abs())],
        day.day(),
        MONTHS[usize::from(day.month().unsigned_abs()) - 1],
        span(meeting, zone)
    )
}

/// A time on Joel's clock, "09:05".
fn clock(time: i64, zone: &TimeZone) -> String {
    Timestamp::from_millisecond(time).map_or_else(
        |_| String::new(),
        |at| at.to_zoned(zone.clone()).strftime("%H:%M").to_string(),
    )
}

/// Whether `time` falls on the day `now` does or before it.
fn on_or_before(time: i64, now: i64, zone: &TimeZone) -> bool {
    let date = |at: i64| {
        Timestamp::from_millisecond(at)
            .ok()
            .map(|at| at.to_zoned(zone.clone()).date())
    };
    date(time) <= date(now)
}

/// 1st, 2nd, 3rd, 4th, 11th, 21st.
fn ordinal(n: i64) -> String {
    let ending = match (n % 100, n % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!("{n}{ending}")
}

/// Which of a day in its month: first, or from the end, last and second
/// last.
fn which(n: i8) -> String {
    let at = usize::from(n.unsigned_abs());
    let from_start = WHICH.get(at).filter(|word| !word.is_empty()).map_or_else(
        || ordinal(i64::from(n.unsigned_abs())),
        |word| (*word).to_owned(),
    );
    match n {
        1.. => from_start,
        -1 => "last".to_owned(),
        _ => format!("{from_start} last"),
    }
}

/// Words in a list, as one says them: "a", "a and b", "a, b and c".
fn listed_words(words: &[String]) -> String {
    match words {
        [] => String::new(),
        [one] => one.clone(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

/// How a meeting repeats, as its details say it: "Every weekday", "Every
/// 2 weeks on Monday and Wednesday", "Every month on the last Friday,
/// until 31 December 2026" (Repeats.js's repeated).
fn repeated(rule: &Repeats, zone: &TimeZone) -> String {
    let (one, many) = match rule.every {
        Every::Day => ("day", "days"),
        Every::Week => ("week", "weeks"),
        Every::Month => ("month", "months"),
        Every::Year => ("year", "years"),
    };
    let plain = rule.days.iter().all(|day| day.nth.is_none());
    let mut days: Vec<u8> = rule.days.iter().map(|day| day.day).collect();
    days.sort_unstable();
    let weekdays = days == [1, 2, 3, 4, 5];
    let mut said = if rule.every == Every::Week && rule.interval == 1 && plain && weekdays {
        "Every weekday".to_owned()
    } else {
        let mut said = if rule.interval == 1 {
            format!("Every {one}")
        } else {
            format!("Every {} {many}", rule.interval)
        };
        let on: Vec<String> = rule
            .days
            .iter()
            .map(|day| {
                let name = DAYS[usize::from(day.day) % DAYS.len()];
                day.nth.map_or_else(
                    || name.to_owned(),
                    |nth| format!("the {} {name}", which(nth)),
                )
            })
            .chain(rule.month_days.iter().map(|&day| {
                if day > 0 {
                    format!("the {}", ordinal(i64::from(day)))
                } else {
                    format!("the {} day", which(day))
                }
            }))
            .collect();
        if !on.is_empty() {
            said = format!("{said} on {}", listed_words(&on));
        }
        said
    };
    if let Some(times) = rule.times {
        said = format!("{said}, {times} times");
    } else if let Some(last) = rule
        .until
        .and_then(|until| Timestamp::from_millisecond(until).ok())
    {
        let last = last.to_zoned(zone.clone());
        said = format!(
            "{said}, until {} {} {}",
            last.day(),
            MONTHS[usize::from(last.month().unsigned_abs()) - 1],
            last.year()
        );
    }
    said
}

#[cfg(test)]
#[path = "calendar_rows_tests.rs"]
mod tests;
