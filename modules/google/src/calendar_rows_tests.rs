//! Calendar's rows' cases, as tests/calendar.qml says Calendar.js's and
//! Repeats.js's of the shell's rows today.

use jiff::tz::{self, TimeZone};
use serde_json::{Value, json};

use super::{Words, drawers, meetings};
use crate::calendar::{Answer, Attachment, Dial, Every, Guest, Meeting, Person, Repeats, Weekday};

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
// Monday 26 October 2026, 14:00 at four hours behind UTC.
const MONDAY: i64 = 1_793_037_600_000;

fn zone() -> TimeZone {
    TimeZone::fixed(tz::offset(-4))
}

fn vocabulary() -> Value {
    json!({"things": [
        {"id": "calendar:meeting", "glyph": "meeting",
         "states": {"untitled": {"name": "A Meeting Without a Title"}},
         "said": {"earlier": "{count} in the past 30 days"}},
        {"id": "verb:calendar:cancel", "sure": "Cancel It Now"},
        {"id": "verb:calendar:move", "sure": "Move It"},
        {"id": "meet:held", "glyph": "joinCall",
         "said": {"two": "{first} and {second}", "others": "{first} and {count} others"}}
    ]})
}

fn words() -> Words {
    Words::read(&vocabulary()).expect("calendar's words didn't read")
}

fn one(id: &str, start: i64) -> Meeting {
    Meeting {
        id: id.into(),
        title: format!("Meeting {id}"),
        start,
        end: start + 30 * MINUTE,
        all_day: false,
        repeats: None,
        series: None,
        answer: None,
        link: None,
        kind: None,
        page: Some(format!("https://calendar.google.com/event?eid={id}")),
        place: None,
        about: None,
        attachments: vec![],
        dial: None,
        holds: true,
        through: None,
        organizer: None,
        guests: vec![],
        reminders: vec![],
        done: None,
    }
}

fn keys(rows: &[Value]) -> Vec<&str> {
    rows.iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect()
}

fn acted(row: &Value) -> Vec<&str> {
    row["acts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|act| act["key"].as_str().unwrap())
        .collect()
}

#[test]
fn calendars_words_are_read_from_the_vocabulary_or_the_lack_is_said() {
    assert_eq!(words().cancel_sure, "Cancel It Now");
    let lacking = json!({"things": [{"id": "calendar:meeting", "glyph": "meeting"}]});
    assert_eq!(
        Words::read(&lacking),
        Err("the vocabulary's calendar:meeting has no states.untitled.name".to_owned())
    );
}

#[test]
fn meetings_to_come_are_listed_soonest_first_and_those_held_newest_first_a_series_once() {
    let later = one("later", MONDAY + 3 * HOUR);
    let soon = one("soon", MONDAY + HOUR);
    let on = one("on", MONDAY - 10 * MINUTE);
    let mut standup = one("standup-2", MONDAY - 2 * HOUR);
    standup.series = Some("standup".into());
    let mut standup_before = one("standup-1", MONDAY - 26 * HOUR);
    standup_before.series = Some("standup".into());
    let mut declined = one("declined", MONDAY - 3 * HOUR);
    declined.answer = Some(Answer::No);
    let rows = meetings(
        &[later, soon, on, standup.clone()],
        &[standup, standup_before, declined],
        MONDAY,
        &zone(),
        &words(),
    );
    assert_eq!(
        keys(&rows),
        [
            "row:calendar:on",
            "row:calendar:soon",
            "row:calendar:later",
            "row:held:standup-2"
        ],
        "a declined meeting is held by nobody, and the standup once"
    );
    let orders: Vec<i64> = rows
        .iter()
        .map(|row| row["order"].as_i64().unwrap())
        .collect();
    assert_eq!(orders, [0, 1, 2, 0], "each place counts its own order");
    assert_eq!(rows[0]["row"], "calendar:meeting:on");
    assert_eq!(rows[3]["held"]["earlier"], 1);
    assert_eq!(rows[3]["drawer"], "drawer:held:standup-2");
    let drawn = drawers(&[], &[rows_meeting_held()], MONDAY, &zone(), &words());
    assert_eq!(drawn.len(), 1);
}

fn rows_meeting_held() -> Meeting {
    one("held", MONDAY - 2 * HOUR)
}

#[test]
fn a_meetings_time_is_the_shells_to_word_and_its_note_its_times_and_answer_until_its_over() {
    let mut invited = one("invited", MONDAY + HOUR);
    invited.answer = Some(Answer::Maybe);
    invited.link = Some("https://meet.google.com/abc-defg-hij".into());
    let tomorrow = one("tomorrow", MONDAY + 20 * HOUR);
    let over = one("over", MONDAY - HOUR);
    let rows = meetings(&[invited, tomorrow, over], &[], MONDAY, &zone(), &words());
    assert_eq!(
        rows[0]["side"],
        json!([{"at": MONDAY + HOUR, "as": "meeting", "until": MONDAY + HOUR + 30 * MINUTE,
            "allDay": false}])
    );
    assert_eq!(rows[0]["note"], "15:00 to 15:30 \u{b7} maybe");
    assert_eq!(rows[0]["today"], true);
    assert_eq!(
        rows[0]["joins"],
        json!({"from": MONDAY + 50 * MINUTE, "until": MONDAY + HOUR + 30 * MINUTE}),
        "Return joins from ten minutes before it starts"
    );
    assert_eq!(rows[1]["today"], false);
    assert_eq!(
        rows[1]["joins"],
        Value::Null,
        "nothing to join without a call"
    );
    assert_eq!(rows[2]["key"], "row:held:over");
    assert_eq!(
        rows[2]["note"], "",
        "once it's over, its side alone says when it was"
    );
}

#[test]
fn its_acts_come_in_the_one_order_each_with_what_it_asks_or_the_step_it_takes() {
    let mut meeting = one("e1", MONDAY + HOUR);
    meeting.title = "Plan & Review".into();
    meeting.link = Some("https://meet.google.com/abc-defg-hij".into());
    meeting.answer = Some(Answer::Yes);
    meeting.holds = false;
    meeting.through = Some("primary".into());
    meeting.organizer = Some(Person {
        name: "Ada Lovelace".into(),
        email: Some("ada@example.com".into()),
    });
    meeting.guests = vec![Guest {
        name: "Grace Hopper".into(),
        email: Some("grace@example.com".into()),
        answer: Answer::Waiting,
    }];
    meeting.attachments = vec![Attachment {
        title: "Agenda".into(),
        link: "https://docs.google.com/document/d/agenda".into(),
        id: Some("agenda".into()),
        kind: None,
    }];
    let rows = meetings(&[meeting], &[], MONDAY, &zone(), &words());
    let row = &rows[0];
    assert_eq!(
        acted(row),
        [
            "verb:calendar:join",
            "verb:calendar:write",
            "verb:calendar:see",
            "verb:calendar:transcript",
            "verb:calendar:yes",
            "verb:calendar:maybe",
            "verb:calendar:no",
            "verb:calendar:move",
            "verb:calendar:guests",
            "verb:calendar:attachments",
            "verb:calendar:link",
        ],
        "Cancel only for one Joel holds"
    );
    let acts = row["acts"].as_array().unwrap();
    assert_eq!(
        acts[0]["ask"],
        json!({"open": {"link": "https://meet.google.com/abc-defg-hij"}})
    );
    assert_eq!(
        acts[1]["ask"]["open"]["link"],
        "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com%2Cgrace%40example.com\
         &su=Plan%20%26%20Review"
    );
    assert_eq!(
        acts[4]["ask"],
        json!({"answer": {"id": "e1", "with": "yes"}})
    );
    assert_eq!(acts[4]["given"], true);
    assert_eq!(acts[5]["given"], false);
    assert_eq!(acts[7]["telling"], "Telling Ada Lovelace and Grace Hopper");
    assert_eq!(
        acts[8]["emails"],
        json!(["ada@example.com", "grace@example.com"])
    );
    assert_eq!(acts[9]["count"], 1);
    assert_eq!(acts[10]["side"], "its call");
}

#[test]
fn one_joel_holds_cancels_once_sure_and_without_a_call_copies_its_page() {
    let mut meeting = one("mine", MONDAY + HOUR);
    meeting.guests = (1..=3)
        .map(|at| Guest {
            name: format!("Guest {at}"),
            email: None,
            answer: Answer::Yes,
        })
        .collect();
    let rows = meetings(&[meeting], &[], MONDAY, &zone(), &words());
    let acts = rows[0]["acts"].as_array().unwrap();
    assert_eq!(
        acted(&rows[0]),
        [
            "verb:calendar:see",
            "verb:calendar:cancel",
            "verb:calendar:link"
        ]
    );
    assert_eq!(acts[1]["ask"], json!({"cancel": {"id": "mine"}}));
    assert_eq!(acts[1]["sure"], "Cancel It Now, Telling 3 Guests");
    assert_eq!(
        acts[2]["copy"],
        "https://calendar.google.com/event?eid=mine"
    );
    assert_eq!(acts[2]["side"], "its page");
}

#[test]
fn its_drawer_says_everything_known_of_it_and_a_series_how_many_came_before() {
    let mut meeting = one("weekly-2", MONDAY - 2 * HOUR);
    meeting.title = String::new();
    meeting.series = Some("weekly".into());
    meeting.repeats = Some(Repeats {
        every: Every::Week,
        interval: 2,
        days: vec![Weekday { day: 1, nth: None }, Weekday { day: 3, nth: None }],
        month_days: vec![],
        times: None,
        until: Some(1_798_747_200_000),
    });
    meeting.place = Some("Room 4".into());
    meeting.link = Some("https://meet.google.com/abc-defg-hij".into());
    meeting.dial = Some(Dial {
        number: "+1 555-010-0199".into(),
        pin: Some("123456789".into()),
    });
    meeting.answer = Some(Answer::Yes);
    meeting.holds = false;
    meeting.organizer = Some(Person {
        name: "Ada Lovelace".into(),
        email: None,
    });
    meeting.guests = vec![
        Guest {
            name: "Grace Hopper".into(),
            email: None,
            answer: Answer::No,
        },
        Guest {
            name: String::new(),
            email: Some("alan@example.com".into()),
            answer: Answer::Yes,
        },
    ];
    meeting.about = Some("What we'll do.".into());
    let mut before = meeting.clone();
    before.id = "weekly-1".into();
    before.start -= 14 * 24 * HOUR;
    before.end -= 14 * 24 * HOUR;
    let drawn = drawers(&[], &[meeting, before], MONDAY, &zone(), &words());
    let keys: Vec<&str> = drawn.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["drawer:held:weekly-2", "drawer:meet:abc-defg-hij"]);
    assert_eq!(
        drawn[1].1["facts"][2]["label"], "Where",
        "a call's drawer counts nothing that came before"
    );
    assert_eq!(
        drawn[0].1,
        json!({
            "title": "A Meeting Without a Title",
            "facts": [
                {"label": "When", "value": "Monday 26 October \u{b7} 12:00 to 12:30"},
                {"label": "Repeats",
                    "value": "Every 2 weeks on Monday and Wednesday, until 31 December 2026"},
                {"label": "Earlier", "value": "1 in the past 30 days"},
                {"label": "Where", "value": "Room 4"},
                {"label": "Call", "value": "https://meet.google.com/abc-defg-hij"},
                {"label": "Dial In", "value": "+1 555-010-0199, PIN 123456789"},
                {"label": "Your Answer", "value": "going"},
                {"label": "Holds It", "value": "Ada Lovelace"},
                {"label": "Answers", "value": "1 going, 1 not going"},
                {"label": "Asked",
                    "value": "Grace Hopper \u{b7} not going; alan@example.com \u{b7} going"},
            ],
            "body": "What we'll do.",
        })
    );
}

#[test]
fn meets_recent_is_each_call_held_on_a_meet_link_once_by_code_saying_who_it_was_with() {
    let call = |id: &str, start: i64, link: &str| {
        let mut meeting = one(id, start);
        meeting.link = Some(link.into());
        meeting
    };
    let mut latest = call(
        "latest",
        MONDAY - HOUR,
        "https://meet.google.com/abc-defg-hij?authuser=0",
    );
    latest.holds = false;
    latest.organizer = Some(Person {
        name: "Ada Lovelace".into(),
        email: Some("ada@example.com".into()),
    });
    latest.guests = [
        ("Grace Hopper", Answer::Yes),
        ("Alan Turing", Answer::No),
        ("Edsger", Answer::Maybe),
    ]
    .into_iter()
    .map(|(name, answer)| Guest {
        name: name.into(),
        email: Some(format!(
            "{}@example.com",
            name.to_lowercase().replace(' ', ".")
        )),
        answer,
    })
    .collect();
    let again = call(
        "again",
        MONDAY - 25 * HOUR,
        "https://meet.google.com/abcdefghij",
    );
    let alone = call(
        "alone",
        MONDAY - 26 * HOUR,
        "https://meet.google.com/xyz-wxyz-xyz",
    );
    let elsewhere = call("zoom", MONDAY - 27 * HOUR, "https://zoom.us/j/123");
    let rows = meetings(
        &[],
        &[latest, again, alone, elsewhere],
        MONDAY,
        &zone(),
        &words(),
    );
    let calls: Vec<&Value> = rows.iter().filter(|row| row["place"] == "meet").collect();
    let called: Vec<&str> = calls
        .iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        called,
        ["row:meet:abc-defg-hij", "row:meet:xyz-wxyz-xyz"],
        "one room is one call however its link is written, and Zoom's is none"
    );
    assert_eq!(calls[0]["row"], "meet:held:abc-defg-hij");
    assert_eq!(calls[0]["kind"], "call");
    assert_eq!(calls[0]["glyph"], "joinCall");
    assert_eq!(
        calls[0]["side"][0],
        json!({"said": "Ada Lovelace and 2 others"})
    );
    assert_eq!(calls[0]["side"][1]["as"], "meeting");
    assert_eq!(calls[0]["note"], "13:00 to 13:30");
    assert_eq!(
        calls[0]["keywords"],
        "grace.hopper@example.com alan.turing@example.com edsger@example.com"
    );
    assert_eq!(
        calls[1]["side"].as_array().map(Vec::len),
        Some(1),
        "nobody to name beside it"
    );
    assert_eq!(calls[1]["keywords"], Value::Null);
    assert_eq!(calls[1]["order"], 1);
}

#[test]
fn a_series_says_how_it_repeats_as_one_would() {
    let said = |every, interval, days: Vec<Weekday>, month_days: Vec<i8>, times| {
        super::repeated(
            &Repeats {
                every,
                interval,
                days,
                month_days,
                times,
                until: None,
            },
            &zone(),
        )
    };
    let plain = |day| Weekday { day, nth: None };
    assert_eq!(
        said(Every::Week, 1, (1..=5).map(plain).collect(), vec![], None),
        "Every weekday"
    );
    assert_eq!(
        said(
            Every::Month,
            1,
            vec![Weekday {
                day: 5,
                nth: Some(-1)
            }],
            vec![],
            None
        ),
        "Every month on the last Friday"
    );
    assert_eq!(
        said(
            Every::Month,
            1,
            vec![Weekday {
                day: 2,
                nth: Some(-2)
            }],
            vec![],
            Some(4)
        ),
        "Every month on the second last Tuesday, 4 times"
    );
    assert_eq!(
        said(Every::Month, 3, vec![], vec![1, 22, -1], None),
        "Every 3 months on the 1st, the 22nd and the last day"
    );
    assert_eq!(said(Every::Day, 1, vec![], vec![], None), "Every day");
}
