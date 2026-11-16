//! Calendar and Meet: meetings made for their link and calls followed,
//! every page joined, series and reminders, and meetings answered,
//! cancelled or moved.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use google::api::Google;
use google::calendar::Answer;
use google::state::Standing;
use google::watch::Watch;
use jiff::Zoned;

use super::{EVENTS, StandIn, credentials, gmail, google, now};

#[test]
fn a_meeting_is_made_for_its_link_only_once_the_sign_in_grants_meet() {
    let failing = Arc::new(AtomicBool::new(false));
    let fails = Arc::clone(&failing);
    let stand_in = StandIn::new(move |path, body| {
        if path.starts_with("POST /v2/spaces ") {
            if fails.load(Ordering::SeqCst) {
                return (503, "{}".into());
            }
            return (
                200,
                r#"{"name": "spaces/s1", "meetingUri": "https://meet.google.com/new-abcd-efg",
                    "meetingCode": "new-abcd-efg"}"#
                    .into(),
            );
        }
        google(path, body)
    });
    let path = credentials("meet", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let made = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.starts_with("POST /v2/spaces "))
            .count()
    };
    assert!(
        !watch.look(&now()).meet.granted,
        "a sign-in without Meet's scope grants it"
    );
    watch.making(&now());
    let state = watch.make(&now()).clone();
    assert_eq!(made(&stand_in), 0, "Google was asked without Meet's scope");
    let unmade = state.meet.made.unwrap();
    assert!(unmade.failed && unmade.link.is_none(), "{unmade:?}");
    // Joel signs in again from pi, granting Meet's scope too.
    let file = fs::read_to_string(&path).unwrap().replace(
        r#""scope": "calendar""#,
        r#""scope": "calendar https://www.googleapis.com/auth/meetings.space.created""#,
    );
    fs::write(&path, file).unwrap();
    assert!(
        watch.look(&now()).meet.granted,
        "a sign-in granting meet wasn't seen"
    );
    let asking = watch.making(&now()).meet.made.clone().unwrap();
    assert!(!asking.failed && asking.link.is_none(), "{asking:?}");
    let state = watch.make(&now()).clone();
    assert_eq!(
        state.meet.made.unwrap().link.as_deref(),
        Some("https://meet.google.com/new-abcd-efg")
    );
    assert_eq!(made(&stand_in), 1);
    assert_eq!(state.account, Standing::Ready);
    failing.store(true, Ordering::SeqCst);
    watch.making(&now());
    let state = watch.make(&now()).clone();
    let unmade = state.meet.made.unwrap();
    assert!(unmade.failed && unmade.link.is_none(), "{unmade:?}");
    assert_eq!(state.meetings.len(), 1, "Meet failing lost the meetings");
}

#[test]
fn a_call_followed_says_who_is_in_it_once_the_sign_in_grants_reading_meet() {
    let failing = Arc::new(AtomicBool::new(false));
    let fails = Arc::clone(&failing);
    let stand_in = StandIn::new(move |path, body| {
        if path.starts_with("GET /v2/conferenceRecords?") {
            if fails.load(Ordering::SeqCst) {
                return (503, "{}".into());
            }
            assert!(path.contains("abc-defg-hij"), "{path}");
            return (
                200,
                r#"{"conferenceRecords": [{"name": "conferenceRecords/c1",
                    "startTime": "2026-10-26T19:00:00Z"}]}"#
                    .into(),
            );
        }
        if path.starts_with("GET /v2/conferenceRecords/c1/participants") {
            return (
                200,
                r#"{"participants": [
                    {"signedinUser": {"displayName": "Grace Hopper"},
                     "earliestStartTime": "2026-10-26T19:02:00Z"},
                    {"signedinUser": {"displayName": "Ada Lovelace"},
                     "earliestStartTime": "2026-10-26T19:00:30Z",
                     "latestEndTime": "2026-10-26T19:15:00Z"}]}"#
                    .into(),
            );
        }
        google(path, body)
    });
    let path = credentials("calls", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let asked = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.starts_with("GET /v2/conferenceRecords"))
            .count()
    };
    assert!(
        !watch.following(&now()),
        "a call was followed before anyone asked"
    );
    let state = watch.follow(vec!["abc-defg-hij".into()], &now()).clone();
    assert_eq!(
        asked(&stand_in),
        0,
        "Meet was asked without the scope to read it"
    );
    assert_eq!(state.calls.len(), 1);
    assert!(
        !state.calls[0].granted && !state.calls[0].known,
        "a call followed without reading meet granted was told: {:?}",
        state.calls[0]
    );
    // Joel signs in again from pi, granting the scope that reads calls.
    let file = fs::read_to_string(&path).unwrap().replace(
        r#""scope": "calendar""#,
        r#""scope": "calendar https://www.googleapis.com/auth/meetings.space.readonly""#,
    );
    fs::write(&path, file).unwrap();
    let state = watch.calls(&now()).clone();
    let call = &state.calls[0];
    assert!(call.granted && call.known, "{call:?}");
    assert_eq!(call.started, Some(1_793_041_200_000));
    let names: Vec<&str> = call
        .people
        .iter()
        .map(|person| person.name.as_str())
        .collect();
    assert_eq!(names, ["Ada Lovelace", "Grace Hopper"]);
    assert_eq!(call.people[0].left, Some(1_793_042_100_000));
    // Meet failing keeps what it said last.
    failing.store(true, Ordering::SeqCst);
    assert_eq!(watch.calls(&now()).calls[0].people.len(), 2);
    // A follow the shell doesn't say again lapses, as when it went away.
    failing.store(false, Ordering::SeqCst);
    assert!(watch.following(&now()), "a follow lapsed at once");
    let later = now().checked_add(jiff::Span::new().minutes(2)).unwrap();
    assert!(!watch.following(&later), "a follow nobody said again held");
    // The call's window closed: nothing followed, nothing asked.
    watch.follow(vec!["abc-defg-hij".into()], &now());
    let before = asked(&stand_in);
    assert_eq!(
        watch.follow(Vec::new(), &now()).calls,
        Vec::<google::meet::Call>::new()
    );
    assert!(
        !watch.following(&now()),
        "a call whose window closed was still followed"
    );
    assert_eq!(asked(&stand_in), before);
}

#[test]
fn every_page_of_meetings_is_joined() {
    let stand_in = StandIn::new(|path, _| {
        if path.contains("pageToken=two") {
            (200, EVENTS.replace("e1", "e2"))
        } else {
            (
                200,
                EVENTS.replace(r#""items""#, r#""nextPageToken": "two", "items""#),
            )
        }
    });
    let path = credentials("pages", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let ids: Vec<_> = watch
        .look(&now())
        .meetings
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert_eq!(ids, vec!["e1", "e2"]);
}

#[test]
fn each_series_is_asked_how_it_repeats_once_a_while_and_kept_while_google_fails() {
    let failing = Arc::new(AtomicBool::new(false));
    let fails = Arc::clone(&failing);
    let stand_in = StandIn::new(move |path, body| {
        if path.starts_with("GET /calendar/v3/calendars/primary/events/s?") {
            if fails.load(Ordering::SeqCst) {
                return (503, "{}".into());
            }
            return (
                200,
                r#"{"recurrence": ["RRULE:FREQ=WEEKLY;BYDAY=MO"]}"#.into(),
            );
        }
        if path.starts_with("GET /calendar/v3/calendars/primary/events?") {
            // Two of one series, and one of a series gone since.
            let twice = EVENTS
                .replace(r#""id": "e1","#, r#""id": "s_1", "recurringEventId": "s","#)
                .replace("}]}", "}, ");
            let again = EVENTS
                .replace(r#"{"items": ["#, "")
                .replace(r#""id": "e1","#, r#""id": "s_2", "recurringEventId": "s","#)
                .replace("}]}", "}, ");
            let gone = EVENTS
                .replace(r#"{"items": ["#, "")
                .replace(r#""id": "e1","#, r#""id": "g_1", "recurringEventId": "g","#);
            return (200, format!("{twice}{again}{gone}"));
        }
        if path.starts_with("GET /calendar/v3/calendars/primary/events/g?") {
            return (404, "{}".into());
        }
        google(path, body)
    });
    let path = credentials("series", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let weekly = |watch: &mut Watch, at: &Zoned| {
        watch
            .look(at)
            .meetings
            .iter()
            .map(|meeting| meeting.repeats.as_ref().map(|repeats| repeats.days.len()))
            .collect::<Vec<_>>()
    };
    let asked = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains("/events/s?fields=recurrence "))
            .count()
    };
    assert_eq!(weekly(&mut watch, &now()), [Some(1), Some(1), None]);
    assert_eq!(
        asked(&stand_in),
        1,
        "a series was asked for each of its meetings"
    );
    weekly(&mut watch, &now());
    assert_eq!(asked(&stand_in), 1, "a series was asked again at once");
    failing.store(true, Ordering::SeqCst);
    let later: Zoned = "2026-10-26T15:25:00-04:00[-04:00]".parse().unwrap();
    assert_eq!(
        weekly(&mut watch, &later),
        [Some(1), Some(1), None],
        "a series Google failed to say again lost how it repeats"
    );
    assert_eq!(
        asked(&stand_in),
        2,
        "a series wasn't asked again after five minutes"
    );
}

#[test]
fn the_calendars_reminders_hold_for_meetings_on_every_page() {
    // Google says the calendar's reminders on each page; only the first's
    // are kept, and every meeting that keeps them is reminded by them.
    let stand_in = StandIn::new(|path, _| {
        if path.contains("pageToken=two") {
            (200, EVENTS.replace("e1", "e2"))
        } else {
            (
                200,
                EVENTS.replace(
                    r#""items""#,
                    r#""nextPageToken": "two",
                        "defaultReminders": [{"method": "popup", "minutes": 10}], "items""#,
                ),
            )
        }
    });
    let path = credentials("reminders", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let reminders: Vec<_> = watch
        .look(&now())
        .meetings
        .iter()
        .map(|m| m.reminders.clone())
        .collect();
    assert_eq!(reminders, vec![vec![10], vec![10]]);
}

#[test]
fn a_meeting_joel_declined_is_not_held() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("GET /calendar/v3/calendars/primary/events?") {
            return (
                200,
                r#"{"items": [{"id": "e1", "summary": "Standup",
                    "start": {"dateTime": "2026-10-26T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-26T16:15:00-04:00"},
                    "attendees": [{"email": "joel@example.com", "self": true,
                        "responseStatus": "declined"}]}]}"#
                    .into(),
            );
        }
        gmail(path, body)
    });
    let path = credentials("declined", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    // Nothing that asks whether he's in a meeting, or who he met, counts
    // one he declined (2026-11-10).
    assert_eq!(
        watch.look(&now()).meetings,
        [] as [google::calendar::Meeting; 0],
        "a meeting he declined is held as his"
    );
}

fn invited(path: &str, _: &str) -> (u16, String) {
    if path.starts_with("GET /calendar/v3/calendars/primary/events/e1?") {
        return (
            200,
            r#"{"attendees": [{"email": "a@example.com", "responseStatus": "accepted"},
                {"email": "joel@example.com", "self": true, "responseStatus": "needsAction"}]}"#
                .into(),
        );
    }
    if path.starts_with("PATCH") {
        return (200, r#"{"id": "e1"}"#.into());
    }
    (200, EVENTS.into())
}

#[test]
fn a_meeting_declined_here_goes_at_once() {
    let stand_in = StandIn::new(invited);
    let path = credentials("decline", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    let state = watch.answer("e1", Answer::No, &now()).clone();
    assert!(
        state.meetings.iter().all(|meeting| meeting.id != "e1"),
        "a meeting he declined here is still held until the next look"
    );
}

#[test]
fn an_answer_changes_joels_own_and_is_held_at_once() {
    let stand_in = StandIn::new(invited);
    let path = credentials("answer", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let held = watch.look(&now()).meetings[0].answer;
    let asked = watch.answering("e1", Answer::Maybe).meetings[0].clone();
    assert_eq!(
        (asked.given, asked.answer),
        (Some(Answer::Maybe), held),
        "an answer asked is his at once, and still Google's own until it takes it"
    );
    let state = watch.answer("e1", Answer::Maybe, &now()).clone();
    assert_eq!(
        (state.meetings[0].given, state.meetings[0].answer),
        (None, Some(Answer::Maybe)),
        "an answer Google took is the meeting's own"
    );
    let heard = stand_in.heard();
    let written = heard.last().unwrap();
    assert!(
        written
            .line
            .starts_with("PATCH /calendar/v3/calendars/primary/events/e1?sendUpdates=none"),
        "{}",
        written.line
    );
    assert_eq!(written.authorization, "Bearer pis");
    let body: serde_json::Value = serde_json::from_str(&written.body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"attendees": [
            {"email": "a@example.com", "responseStatus": "accepted"},
            {"email": "joel@example.com", "self": true, "responseStatus": "tentative"}
        ]})
    );
}

#[test]
fn an_answer_google_refuses_changes_nothing_held() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("PATCH") {
            (404, "{}".into())
        } else {
            invited(path, body)
        }
    });
    let path = credentials("unanswered", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    let state = watch.answer("e1", Answer::Yes, &now()).clone();
    assert_eq!(state, before);
}

fn cancelled(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("DELETE") {
        return (204, String::new());
    }
    google(path, body)
}

#[test]
fn a_meeting_joel_holds_is_cancelled_telling_every_guest_and_let_go_at_once() {
    let stand_in = StandIn::new(cancelled);
    let path = credentials("cancel", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    assert!(
        watch.look(&now()).meetings[0].holds,
        "a meeting Joel holds wasn't his"
    );
    let state = watch.cancel("e1", &now()).clone();
    assert_eq!(state.meetings, [] as [google::calendar::Meeting; 0]);
    let heard = stand_in.heard();
    let written = heard.last().unwrap();
    assert!(
        written
            .line
            .starts_with("DELETE /calendar/v3/calendars/primary/events/e1?sendUpdates=all "),
        "{}",
        written.line
    );
    assert_eq!(written.authorization, "Bearer pis");
}

#[test]
fn a_meeting_joel_is_only_asked_to_is_never_cancelled() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("GET /calendar") {
            return (
                200,
                r#"{"items": [{"id": "e1", "summary": "Standup",
                    "start": {"dateTime": "2026-10-26T16:00:00-04:00"},
                    "end": {"dateTime": "2026-10-26T16:15:00-04:00"},
                    "organizer": {"email": "ada@example.com"}}]}"#
                    .into(),
            );
        }
        cancelled(path, body)
    });
    let path = credentials("not-his", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    assert_eq!(watch.cancel("e1", &now()), &before);
    assert!(
        stand_in
            .heard()
            .iter()
            .all(|heard| !heard.line.starts_with("DELETE")),
        "a meeting Joel is only asked to was cancelled"
    );
}

#[test]
fn a_cancel_google_refuses_changes_nothing_held() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("DELETE") {
            (403, "{}".into())
        } else {
            google(path, body)
        }
    });
    let path = credentials(
        "uncancelled",
        now().timestamp().as_millisecond() + 3_600_000,
    );
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    assert_eq!(watch.cancel("e1", &now()), &before);
}

/// The team's Standup, held on a calendar Joel may write to when the
/// calendar list says so, which `lists` answers, and a move Google takes.
fn on_the_team(path: &str, lists: (u16, &str)) -> (u16, String) {
    if path.starts_with("GET /calendar/v3/users/me/calendarList?") {
        return (lists.0, lists.1.into());
    }
    if path.starts_with("GET /calendar") {
        return (
            200,
            r#"{"items": [{"id": "e1", "summary": "Standup",
                "start": {"dateTime": "2026-10-26T16:00:00-04:00"},
                "end": {"dateTime": "2026-10-26T16:15:00-04:00"},
                "organizer": {"email": "team@group.calendar.google.com"}}]}"#
                .into(),
        );
    }
    if path.starts_with("PATCH") {
        return (200, r#"{"id": "e1"}"#.into());
    }
    google(path, "")
}

const THE_TEAMS: (u16, &str) = (
    200,
    r#"{"items": [{"id": "team@group.calendar.google.com"}]}"#,
);

#[test]
fn a_meeting_on_a_calendar_joel_may_write_to_is_moved_telling_everyone() {
    let stand_in = StandIn::new(|path, _| on_the_team(path, THE_TEAMS));
    let path = credentials("move", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let held = watch.look(&now()).meetings[0].clone();
    assert_eq!(
        held.through.as_deref(),
        Some("team@group.calendar.google.com")
    );
    let (start, end) = (held.start + 1_800_000, held.end + 1_800_000);
    let state = watch.reschedule("e1", start, end, &now()).clone();
    assert_eq!(
        (state.meetings[0].start, state.meetings[0].end),
        (start, end)
    );
    let heard = stand_in.heard();
    let asked = heard
        .iter()
        .find(|heard| {
            heard
                .line
                .starts_with("GET /calendar/v3/users/me/calendarList?")
        })
        .unwrap();
    assert!(
        asked.line.contains("minAccessRole=writer"),
        "{}",
        asked.line
    );
    let written = heard.last().unwrap();
    assert!(
        written.line.starts_with(
            "PATCH /calendar/v3/calendars/team@group.calendar.google.com/events/e1?sendUpdates=all"
        ),
        "{}",
        written.line
    );
    let body: serde_json::Value = serde_json::from_str(&written.body).unwrap();
    assert_eq!(
        body,
        serde_json::json!({
            "start": {"dateTime": "2026-10-26T20:30:00Z"},
            "end": {"dateTime": "2026-10-26T20:45:00Z"}
        })
    );
}

#[test]
fn a_meeting_joel_may_not_change_is_never_moved() {
    let stand_in = StandIn::new(|path, _| on_the_team(path, (200, r#"{"items": []}"#)));
    let path = credentials(
        "not-movable",
        now().timestamp().as_millisecond() + 3_600_000,
    );
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    assert_eq!(before.meetings[0].through, None);
    let (start, end) = (before.meetings[0].start + 1, before.meetings[0].end + 1);
    assert_eq!(watch.reschedule("e1", start, end, &now()), &before);
    assert!(
        stand_in
            .heard()
            .iter()
            .all(|heard| !heard.line.starts_with("PATCH")),
        "a meeting Joel may not change was moved"
    );
}

#[test]
fn the_calendars_joel_may_write_to_are_kept_when_only_their_list_fails() {
    let listed = Arc::new(Mutex::new(THE_TEAMS));
    let lists = Arc::clone(&listed);
    let stand_in = StandIn::new(move |path, _| on_the_team(path, *lists.lock().unwrap()));
    let path = credentials("list-fails", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    *listed.lock().unwrap() = (500, "{}");
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(
        state.meetings[0].through.as_deref(),
        Some("team@group.calendar.google.com")
    );
}

#[test]
fn a_move_google_refuses_changes_nothing_held() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("PATCH") {
            (403, "{}".into())
        } else {
            google(path, body)
        }
    });
    let path = credentials("unmoved", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    let (start, end) = (before.meetings[0].start + 1, before.meetings[0].end + 1);
    assert_eq!(watch.reschedule("e1", start, end, &now()), &before);
}
