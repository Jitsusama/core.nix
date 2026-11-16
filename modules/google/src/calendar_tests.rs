//! Calendar's cases: what a meeting says of Calendar's answers, how a
//! series repeats, and how each answer is read and written.

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
    let of: Vec<_> = found
        .iter()
        .map(|meeting| meeting.series.as_deref())
        .collect();
    assert_eq!(of, [Some("s"), Some("s"), Some("q"), None]);
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
            series: None,
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
            done: None,
            given: None,
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
    assert!(!found.holds, "a meeting Ada holds reads as Joel's");
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
    assert_eq!(found[1].attachments, [] as [crate::calendar::Attachment; 0]);
}

#[test]
fn a_meeting_joel_holds_is_his_and_names_no_organizer() {
    let answer = json!({"items": [{"id": "e", "start": {"date": "2026-10-27"},
        "end": {"date": "2026-10-28"},
        "organizer": {"email": "joel@example.com", "self": true},
        "attendees": [{"email": "joel@example.com", "self": true, "organizer": true},
            {"email": "a@example.com", "responseStatus": "tentative"}]}]});
    let found = &meetings(&answer, &zone())[0];
    assert!(found.holds, "a meeting Joel holds doesn't read as his");
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
    assert!(
        cut.chars().count() <= ABOUT_MOST,
        "the description ran past {ABOUT_MOST} characters"
    );
    assert!(cut.ends_with("word"), "the cut fell inside a word: {cut:?}");
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
    assert_eq!(answer("tentative"), Some(Answer::Maybe));
    assert_eq!(answer("needsAction"), Some(Answer::Waiting));
}

#[test]
fn a_meeting_joel_declined_is_not_a_meeting() {
    let answer = json!({"items": [{"id": "e", "start": {"date": "2026-10-27"},
        "end": {"date": "2026-10-28"},
        "attendees": [{"self": true, "responseStatus": "declined"},
            {"email": "a@example.com", "organizer": true, "responseStatus": "accepted"}]}]});
    assert!(
        meetings(&answer, &zone()).is_empty(),
        "a meeting he declined is listed"
    );
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
    assert!(found.all_day, "a meeting of whole days reads as timed");
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
