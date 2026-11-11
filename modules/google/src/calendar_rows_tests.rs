//! Calendar's rows' cases, as tests/calendar.qml says Calendar.js's and
//! Repeats.js's of the shell's rows today, said in the launcher's own
//! vocabulary, which the check hands them (package.nix's `words`).

use jiff::tz::{self, TimeZone};
use serde_json::{Value, json};

use super::{Words, drawers, meetings};
use crate::calendar::{Answer, Attachment, Dial, Every, Guest, Meeting, Person, Repeats, Weekday};
use crate::rows::marked::vocabulary;

pub(super) const MINUTE: i64 = 60_000;
pub(super) const HOUR: i64 = 60 * MINUTE;
// Monday 26 October 2026, 14:00 at four hours behind UTC.
pub(super) const MONDAY: i64 = 1_793_037_600_000;

pub(super) fn zone() -> TimeZone {
    TimeZone::fixed(tz::offset(-4))
}

fn words() -> Words {
    Words::read(&vocabulary()).expect("calendar's words didn't read")
}

pub(super) fn one(id: &str, start: i64) -> Meeting {
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

/// The rows a place lists, without those inside them.
fn top(rows: &[Value]) -> Vec<Value> {
    rows.iter()
        .filter(|row| row["parent"].is_null())
        .cloned()
        .collect()
}

/// The rows inside the row keyed `parent`, in their order.
fn within<'a>(rows: &'a [Value], parent: &str) -> Vec<&'a Value> {
    rows.iter().filter(|row| row["parent"] == parent).collect()
}

/// The row keyed `key`.
fn at<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|row| row["key"] == key)
        .unwrap_or_else(|| panic!("no row {key}"))
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
    let mut lacking = vocabulary();
    let meeting = lacking["things"]
        .as_array_mut()
        .and_then(|things| {
            things
                .iter_mut()
                .find(|thing| thing["id"] == "calendar:meeting")
        })
        .expect("the vocabulary has no calendar:meeting");
    meeting["said"]
        .as_object_mut()
        .expect("calendar:meeting says nothing")
        .remove("pin");
    assert_eq!(
        Words::read(&lacking),
        Err("the vocabulary's calendar:meeting has no said.pin".to_owned())
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
    let rows = top(&meetings(
        &[later, soon, on, standup.clone()],
        &[standup, standup_before, declined],
        MONDAY,
        &zone(),
        &words(),
    ));
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
    assert!(
        rows.iter().all(|row| row["order"].is_null()),
        "the daemon spaces each place's order, not the rows"
    );
    assert_eq!(rows[0]["thing"], "calendar:meeting:on");
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
    let rows = top(&meetings(
        &[invited.clone(), tomorrow, over],
        &[],
        MONDAY,
        &zone(),
        &words(),
    ));
    assert_eq!(
        rows[0]["side"],
        json!([{"at": MONDAY + HOUR, "as": "meeting", "until": MONDAY + HOUR + 30 * MINUTE,
            "allDay": false}])
    );
    assert_eq!(rows[0]["note"], "15:00 to 15:30 \u{b7} maybe");
    assert_eq!(rows[0]["today"], true);
    let returns = |now: i64| {
        top(&meetings(&[invited.clone()], &[], now, &zone(), &words()))[0]["acts"][0].clone()
    };
    let details = json!({"key": "verb:details", "step": "drawer"});
    let join = json!({"key": "verb:calendar:join", "open": "https://meet.google.com/abc-defg-hij",
        "in": "google-opens"});
    assert_eq!(
        [
            returns(MONDAY + 49 * MINUTE),
            returns(MONDAY + 50 * MINUTE),
            returns(MONDAY + HOUR + 29 * MINUTE),
        ],
        [details.clone(), join.clone(), join],
        "Return joins from ten minutes before it starts until it ends, showing it before"
    );
    assert_eq!(
        rows[0]["acts"][1],
        json!({"key": "verb:more", "step": "rows"}),
        "its acts are the rows inside it"
    );
    assert_eq!(rows[1]["today"], false);
    assert_eq!(
        rows[1]["acts"][0], details,
        "nothing to join without a call"
    );
    assert_eq!(rows[2]["key"], "row:held:over");
    assert_eq!(
        rows[2]["note"], "",
        "once it's over, its side alone says when it was"
    );
}

#[test]
fn its_acts_are_rows_inside_it_in_the_one_order_each_doing_what_it_names() {
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
    let inside = within(&rows, "row:calendar:e1");
    let named: Vec<(&str, &str)> = inside
        .iter()
        .map(|row| {
            (
                row["name"].as_str().unwrap(),
                row["glyph"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("Join the Meeting", "joinCall"),
            ("Write to the Guests", "writeMail"),
            ("Show in Calendar", "meetingDetails"),
            ("Going", "going"),
            ("Maybe", "maybeGoing"),
            ("Not Going", "notGoing"),
            ("Move the Meeting", "moveMeeting"),
            ("Guests", "people"),
            ("Attachments", "attachment"),
            ("Copy the Link", "link"),
        ],
        "Cancel only for one Joel holds, and the transcript call-transcripts' to serve"
    );
    assert!(
        inside.iter().all(|row| row["depth"] == 1
            && row["thing"]
                .as_str()
                .unwrap()
                .starts_with("calendar:meeting:e1:")),
        "each is a row of the meeting's, one deep"
    );
    let first = |row: &Value| row["acts"][0].clone();
    assert_eq!(
        first(inside[0]),
        json!({"key": "verb:calendar:join", "open": "https://meet.google.com/abc-defg-hij",
            "in": "google-opens"}),
        "joining opens the link in Google's window, as ready-rows' open form"
    );
    assert_eq!(
        first(inside[1])["open"],
        "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=ada%40example.com%2Cgrace%40example.com\
         &su=Plan%20%26%20Review"
    );
    assert_eq!(
        (first(inside[2]), inside[2]["drawer"].clone()),
        (
            json!({"key": "verb:calendar:see", "step": "drawer"}),
            json!("drawer:calendar:e1")
        ),
        "Show in Calendar shows the meeting's own drawer"
    );
    assert_eq!(
        first(inside[3]),
        json!({"key": "verb:calendar:yes", "ask": {"answer": {"id": "e1", "with": "yes"}}}),
        "Going asks the daemon to answer"
    );
    assert_eq!(
        [inside[3]["side"].clone(), inside[4]["side"].clone()],
        [json!([{"said": "your answer"}]), json!([])],
        "only the answer given says so"
    );
    assert_eq!(
        (
            inside[6]["kind"].clone(),
            first(inside[6]),
            inside[6]["moves"]["telling"].clone()
        ),
        (
            json!("move"),
            json!({"key": "verb:more", "step": "rows"}),
            json!("Telling Ada Lovelace and Grace Hopper")
        ),
        "Move steps into the times the shell makes, till they're served"
    );
    assert_eq!(
        (first(inside[9]), inside[9]["side"].clone()),
        (
            json!({"key": "verb:calendar:link", "copy": "https://meet.google.com/abc-defg-hij"}),
            json!([{"said": "its call"}])
        ),
        "with a call it copies the call's link"
    );
}

#[test]
fn guests_and_attachments_step_into_who_and_what_is_on_it() {
    let guest = |name: &str, email: Option<&str>, answer| Guest {
        name: name.into(),
        email: email.map(str::to_owned),
        answer,
    };
    let mut meeting = one("e1", MONDAY + HOUR);
    meeting.holds = false;
    meeting.answer = Some(Answer::Yes);
    meeting.organizer = Some(Person {
        name: "Ada Lovelace".into(),
        email: Some("ada@example.com".into()),
    });
    meeting.guests = vec![
        guest("Grace Hopper", Some("Grace@Example.com"), Answer::Maybe),
        guest("", Some("grace@example.com"), Answer::Maybe),
        guest("", Some("alan@example.com"), Answer::Waiting),
        guest("A Room", None, Answer::Yes),
    ];
    meeting.attachments = vec![Attachment {
        title: "Agenda".into(),
        link: "https://docs.google.com/document/d/agenda".into(),
        id: Some("agenda".into()),
        kind: None,
    }];
    let rows = meetings(&[meeting], &[], MONDAY, &zone(), &words());
    let guests = at(&rows, "row:calendar:e1:guests");
    assert_eq!(
        (guests["acts"].clone(), guests["side"].clone()),
        (
            json!([{"key": "verb:more", "step": "rows"}]),
            json!([{"said": "3"}])
        ),
        "Guests steps in, saying how many"
    );
    let who = within(&rows, "row:calendar:e1:guests");
    let said: Vec<(&str, Value)> = who
        .iter()
        .map(|row| (row["name"].as_str().unwrap(), row["side"].clone()))
        .collect();
    assert_eq!(
        said,
        [
            ("Ada Lovelace", json!([])),
            ("Grace Hopper", json!([{"said": "maybe"}])),
            ("alan@example.com", json!([{"said": "not answered"}])),
        ],
        "who holds it, then each address once, as Guests counts them, a room with none left out"
    );
    assert_eq!(
        who[1]["acts"],
        json!([
            {"key": "verb:person:mail", "in": "google-opens",
                "open": "https://mail.google.com/mail/u/0/?view=cm&fs=1&to=Grace%40Example.com\
                         &su=Meeting%20e1"},
            {"key": "verb:calendar:address", "copy": "Grace@Example.com"},
        ]),
        "Return writes to them, and their address is copied"
    );
    assert_eq!(
        (who[1]["depth"].clone(), who[1]["hue"].clone()),
        (json!(2), json!("person")),
        "a guest is a person two deep"
    );
    let files = within(&rows, "row:calendar:e1:attachments");
    assert_eq!(
        (
            files.len(),
            files[0]["name"].clone(),
            files[0]["acts"].clone()
        ),
        (
            1,
            json!("Agenda"),
            json!([{"key": "calendar:attachment", "in": "google-opens",
                "open": "https://docs.google.com/document/d/agenda"}])
        ),
        "each file opens in Google's window"
    );
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
    let inside = within(&rows, "row:calendar:mine");
    assert_eq!(
        inside.iter().flat_map(|row| acted(row)).collect::<Vec<_>>(),
        ["verb:calendar:see", "verb:more", "verb:calendar:link"],
        "Cancel steps in to be sure"
    );
    let sure: Vec<(Value, Value, Value)> = within(&rows, "row:calendar:mine:cancel")
        .iter()
        .map(|row| {
            (
                row["name"].clone(),
                row["sure"].clone(),
                row["acts"].clone(),
            )
        })
        .collect();
    assert_eq!(
        sure,
        [(
            json!("Cancel It Now, Telling 3 Guests"),
            json!(true),
            json!([{"key": "verb:calendar:cancel", "ask": {"cancel": {"id": "mine"}}}])
        )],
        "its one row asks the daemon to cancel once Joel's sure"
    );
    assert_eq!(
        inside[2]["acts"][0]["copy"], "https://calendar.google.com/event?eid=mine",
        "without a call it copies its page"
    );
    assert_eq!(
        inside[2]["side"],
        json!([{"said": "its page"}]),
        "it says what it copies"
    );
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
        drawn[1].1["facts"][2]["name"], "Where",
        "a call's drawer counts nothing that came before"
    );
    assert_eq!(
        drawn[1].1["eyebrow"], "on Meet \u{b7} call",
        "a call's eyebrow says where it is and what"
    );
    assert_eq!(
        drawn[0].1,
        json!({
            "eyebrow": "on Calendar \u{b7} meeting",
            "title": "A Meeting Without a Title",
            "facts": [
                {"name": "When", "said": "Monday 26 October", "qualifier": "12:00 to 12:30"},
                {"name": "Repeats",
                    "said": "every 2 weeks on Monday and Wednesday, until 31 December 2026"},
                {"name": "Earlier", "said": "1 in the past 30 days"},
                {"name": "Where", "said": "Room 4"},
                {"name": "Call", "said": "https://meet.google.com/abc-defg-hij",
                    "identifies": true},
                {"name": "Dial In", "said": "+1 555-010-0199", "qualifier": "PIN 123456789",
                    "identifies": true},
                {"name": "Your Answer", "said": "going"},
                {"name": "Holds It", "said": "Ada Lovelace"},
                {"name": "Answers", "said": "1 going, 1 not going"},
                {"name": "Asked",
                    "said": "Grace Hopper \u{b7} not going; alan@example.com \u{b7} going"},
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
    let calls: Vec<&Value> = rows
        .iter()
        .filter(|row| row["place"] == "meet" && row["parent"].is_null())
        .collect();
    let called: Vec<&str> = calls
        .iter()
        .map(|row| row["key"].as_str().unwrap())
        .collect();
    assert_eq!(
        called,
        ["row:meet:abc-defg-hij", "row:meet:xyz-wxyz-xyz"],
        "one room is one call however its link is written, and Zoom's is none"
    );
    assert_eq!(calls[0]["thing"], "meet:held:abc-defg-hij");
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
            &words(),
        )
    };
    let plain = |day| Weekday { day, nth: None };
    assert_eq!(
        said(Every::Week, 1, (1..=5).map(plain).collect(), vec![], None),
        "every weekday"
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
        "every month on the last Friday"
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
        "every month on the second last Tuesday, 4 times"
    );
    assert_eq!(
        said(Every::Month, 3, vec![], vec![1, 22, -1], None),
        "every 3 months on the 1st, the 22nd and the last day"
    );
    assert_eq!(said(Every::Day, 1, vec![], vec![], None), "every day");
    assert_eq!(
        said(
            Every::Month,
            1,
            vec![Weekday {
                day: 4,
                nth: Some(7)
            }],
            vec![11, 23],
            None
        ),
        "every month on the 7th Thursday, the 11th and the 23rd",
        "past the fifth, which day is said as a figure"
    );
}

#[test]
fn guests_count_each_address_once_and_who_holds_it_unless_joel_does() {
    let guest = |email: Option<&str>| Guest {
        name: String::new(),
        email: email.map(str::to_owned),
        answer: Answer::Waiting,
    };
    let mut meeting = one("guested", MONDAY);
    meeting.holds = false;
    meeting.organizer = Some(Person {
        name: "Ada Lovelace".into(),
        email: Some("ada@example.com".into()),
    });
    meeting.guests = vec![
        guest(Some("grace@example.com")),
        guest(Some("Grace@Example.com")),
        guest(Some("stranger@example.com")),
        guest(None),
    ];
    assert_eq!(
        super::guests_counted(&meeting),
        3,
        "not Ada, Grace once in any case, and the stranger, a room with no address left out"
    );
    let rows = meetings(&[meeting.clone()], &[], MONDAY, &zone(), &words());
    assert_eq!(
        at(&rows, "row:calendar:guested:guests")["side"],
        json!([{"said": "3"}]),
        "the Guests row didn't say its count"
    );
    assert_eq!(
        within(&rows, "row:calendar:guested:guests").len(),
        3,
        "a row for each address counted"
    );
    meeting.holds = true;
    assert_eq!(
        super::guests_counted(&meeting),
        2,
        "one Joel holds didn't count its guests alone"
    );
}
