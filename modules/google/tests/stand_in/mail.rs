//! Gmail: the mail that wants Joel, told and asked about, read, archived
//! or tidied here or elsewhere, and what Gmail refuses.

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use google::api::Google;
use google::done::Went;
use google::mail::Tidy;
use google::state::Standing;
use google::watch::Watch;
use jiff::Zoned;

use super::{StandIn, credentials, gmail, google, now};

#[test]
fn the_mail_that_wants_joel_is_told_newest_first_and_asked_for_once() {
    let stand_in = StandIn::new(gmail);
    let path = credentials("mail", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.meetings.len(), 1);
    let said: Vec<_> = state
        .mail
        .iter()
        .map(|mail| (mail.id.as_str(), mail.from.as_str(), mail.subject.as_str()))
        .collect();
    assert_eq!(
        said,
        [("new", "Ada", "About new"), ("old", "Ada", "About old")]
    );
    assert_eq!(
        state.mail[0].page,
        "https://mail.google.com/mail/u/0/#inbox/t-new"
    );
    assert_eq!(state.unread, Some(5));
    let searched = stand_in
        .heard()
        .into_iter()
        .find(|heard| heard.line.starts_with("GET /gmail/v1/users/me/messages?"))
        .unwrap();
    assert!(
        searched
            .line
            .contains("q=in%3Ainbox+is%3Aunread+is%3Aimportant")
            || searched
                .line
                .contains("q=in%3Ainbox%20is%3Aunread%20is%3Aimportant"),
        "{}",
        searched.line
    );
    let asked = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.starts_with("GET /gmail/v1/users/me/messages/"))
            .count()
    };
    // The two that want him, and the one he sent lately.
    assert_eq!(asked(&stand_in), 3);
    watch.look(&now());
    assert_eq!(asked(&stand_in), 3, "mail already held was asked for again");
}

#[test]
fn a_months_meetings_held_and_mail_sent_or_read_are_told_a_while_apart() {
    let stand_in = StandIn::new(gmail);
    let path = credentials("lately", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    let lately: Vec<_> = state
        .lately
        .iter()
        .map(|mail| {
            (
                mail.id.as_str(),
                mail.subject.as_str(),
                mail.sent,
                mail.others.clone(),
            )
        })
        .collect();
    assert_eq!(
        lately,
        [("sent", "Lunch", true, vec!["ada@example.com".to_owned()])]
    );
    assert_eq!(state.held.len(), 1);
    let events = |stand_in: &StandIn| -> Vec<String> {
        stand_in
            .heard()
            .into_iter()
            .filter(|heard| {
                heard
                    .line
                    .starts_with("GET /calendar/v3/calendars/primary/events?")
            })
            .map(|heard| heard.line)
            .collect()
    };
    // The month before today, ending where the week's window begins.
    let held = events(&stand_in);
    assert_eq!(held.len(), 2, "{held:?}");
    assert!(
        held[1].contains("timeMin=2026-09-26T04%3A00%3A00Z")
            && held[1].contains("timeMax=2026-10-26T04%3A00%3A00Z"),
        "{}",
        held[1]
    );
    let searched = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains("newer_than"))
            .count()
    };
    assert_eq!(searched(&stand_in), 1);
    // A minute on, only the week and the mail that wants him are asked.
    let later: Zoned = "2026-10-26T15:21:00-04:00[-04:00]".parse().unwrap();
    watch.look(&later);
    assert_eq!(events(&stand_in).len(), 3);
    assert_eq!(
        searched(&stand_in),
        1,
        "the month was asked for again at once"
    );
    // A quarter hour on, the month is asked for again.
    let quarter: Zoned = "2026-10-26T15:35:00-04:00[-04:00]".parse().unwrap();
    watch.look(&quarter);
    assert_eq!(searched(&stand_in), 2);
}

#[test]
fn gmail_failing_keeps_the_mail_held_and_the_meetings_told() {
    let stand_in = StandIn::new(gmail);
    let path = credentials("gmail-down", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let held = watch.look(&now()).mail.clone();
    let down = StandIn::new(|path, body| {
        if path.starts_with("GET /gmail") {
            (503, "{}".into())
        } else {
            google(path, body)
        }
    });
    watch.google = Google::new(down.origins());
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.mail, held);
}

/// The stand-in's Standup, to which Joel and one other are invited, and
/// which takes an answer.
/// The stand-in's Gmail, taking a label off the mail it's asked to.
fn tidied(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("POST /gmail/v1/users/me/messages/") {
        return (200, r#"{"id": "new"}"#.into());
    }
    gmail(path, body)
}

#[test]
fn mail_read_or_archived_takes_one_label_and_wants_joel_no_more() {
    for (how, label) in [(Tidy::Read, "UNREAD"), (Tidy::Archive, "INBOX")] {
        let stand_in = StandIn::new(tidied);
        let path = credentials(label, now().timestamp().as_millisecond() + 3_600_000);
        let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
        watch.look(&now());
        let state = watch.tidy("new", how, &now()).clone();
        let held: Vec<_> = state.mail.iter().map(|mail| mail.id.as_str()).collect();
        assert_eq!(held, ["old"]);
        assert_eq!(state.unread, Some(4));
        let asked = stand_in
            .heard()
            .into_iter()
            .find(|heard| heard.line.starts_with("POST /gmail"))
            .unwrap();
        assert!(
            asked
                .line
                .starts_with("POST /gmail/v1/users/me/messages/new/modify?"),
            "{}",
            asked.line
        );
        let body: serde_json::Value = serde_json::from_str(&asked.body).unwrap();
        assert_eq!(body, serde_json::json!({"removeLabelIds": [label]}));
    }
}

#[test]
fn mail_read_elsewhere_or_tidied_here_says_how_it_went_and_a_meeting_its_end() {
    let read_elsewhere = Arc::new(AtomicBool::new(false));
    let read = Arc::clone(&read_elsewhere);
    let stand_in = StandIn::new(move |path, body| {
        if read.load(Ordering::SeqCst) {
            if path.starts_with("GET /gmail/v1/users/me/messages?") && !path.contains("newer_than")
            {
                return (
                    200,
                    r#"{"messages": [{"id": "old", "threadId": "t1"}]}"#.into(),
                );
            }
            if path.starts_with("GET /gmail/v1/users/me/messages/new?") {
                assert!(path.contains("format=minimal"), "{path}");
                return (200, r#"{"labelIds": ["INBOX", "IMPORTANT"]}"#.into());
            }
        }
        tidied(path, body)
    });
    let path = credentials("done", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.finished, Vec::<google::mail::Mail>::new());
    assert_eq!(state.meetings[0].done, None, "the meeting is still to come");
    // Joel reads the newer one in Gmail: it's done, read there.
    read_elsewhere.store(true, Ordering::SeqCst);
    let state = watch.look(&now()).clone();
    let held: Vec<_> = state.mail.iter().map(|mail| mail.id.as_str()).collect();
    assert_eq!(held, ["old"]);
    let done = state.finished[0].done.expect("it's done");
    assert_eq!(
        (state.finished[0].id.as_str(), done.how),
        ("new", Went::ReadInGmail)
    );
    assert_eq!(done.at, now().timestamp().as_millisecond());
    // He archives the other here: done too, archived here, the latest first.
    let state = watch.tidy("old", Tidy::Archive, &now()).clone();
    let how: Vec<_> = state
        .finished
        .iter()
        .map(|mail| (mail.id.as_str(), mail.done.map(|done| done.how)))
        .collect();
    assert_eq!(
        how,
        [
            ("old", Some(Went::ArchivedHere)),
            ("new", Some(Went::ReadInGmail))
        ]
    );
    // Half a day on, neither is told any more; the meeting has ended.
    let later = now().checked_add(jiff::Span::new().hours(13)).unwrap();
    let state = watch.look(&later).clone();
    assert!(state.finished.is_empty(), "{:?}", state.finished);
    let ended = state.meetings[0].done.expect("it ended");
    assert_eq!(ended.how, Went::Ended);
    assert_eq!(ended.at, state.meetings[0].end);
}

/// Gmail as mail leaves what wants Joel, by `phase`: both listed (0); the
/// newer gone from the list, its labels refused for now (1), then read
/// (2); both listed again (3); signed out (4); and signed in again,
/// maybe as another, where the newer is no mail at all (5).
fn departing(phase: &AtomicU8, path: &str, body: &str) -> (u16, String) {
    let now = phase.load(Ordering::SeqCst);
    if now == 4 && path.starts_with("GET /gmail") {
        return (401, "{}".into());
    }
    if now == 5 && path.starts_with("GET /gmail/v1/users/me/messages/new?") {
        return (404, "{}".into());
    }
    if now == 1 || now == 2 || now == 5 {
        if path.starts_with("GET /gmail/v1/users/me/messages?") && !path.contains("newer_than") {
            return (
                200,
                r#"{"messages": [{"id": "old", "threadId": "t1"}]}"#.into(),
            );
        }
        if path.starts_with("GET /gmail/v1/users/me/messages/new?") {
            return if now == 1 {
                (503, "{}".into())
            } else {
                (200, r#"{"labelIds": ["INBOX", "IMPORTANT"]}"#.into())
            };
        }
    }
    gmail(path, body)
}

#[test]
fn mail_whose_labels_gmail_refuses_is_asked_about_again_on_the_next_look() {
    let phase = Arc::new(AtomicU8::new(0));
    let said = Arc::clone(&phase);
    let stand_in = StandIn::new(move |path, body| departing(&said, path, body));
    let path = credentials("refused", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    phase.store(1, Ordering::SeqCst);
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert!(state.finished.is_empty(), "{:?}", state.finished);
    phase.store(2, Ordering::SeqCst);
    let later = now().checked_add(jiff::Span::new().minutes(1)).unwrap();
    let state = watch.look(&later).clone();
    let how: Vec<_> = state
        .finished
        .iter()
        .map(|mail| (mail.id.as_str(), mail.done.map(|done| done.how)))
        .collect();
    assert_eq!(how, [("new", Some(Went::ReadInGmail))]);
    let done = state.finished[0].done.expect("it's done");
    assert_eq!(
        done.at,
        now().timestamp().as_millisecond(),
        "done when it went, not when Gmail answered"
    );
}

#[test]
fn mail_waiting_to_be_asked_about_goes_when_the_account_does() {
    let phase = Arc::new(AtomicU8::new(0));
    let said = Arc::clone(&phase);
    let stand_in = StandIn::new(move |path, body| departing(&said, path, body));
    let path = credentials("fell", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    watch.look(&now());
    phase.store(1, Ordering::SeqCst);
    watch.look(&now());
    phase.store(4, Ordering::SeqCst);
    assert_eq!(watch.look(&now()).account, Standing::SignedOut);
    // He signs in again, with a sign-in of its own.
    let signed_in = fs::read_to_string(&path)
        .unwrap()
        .replace("\"pis\"", "\"anew\"")
        .replace("\"refresh\"", "\"again\"");
    fs::write(&path, signed_in).unwrap();
    phase.store(5, Ordering::SeqCst);
    let later = now().checked_add(jiff::Span::new().minutes(1)).unwrap();
    let state = watch.look(&later).clone();
    assert_eq!(state.account, Standing::Ready);
    assert!(state.finished.is_empty(), "{:?}", state.finished);
}

#[test]
fn mail_listed_again_before_gmail_says_how_it_left_wants_joel_still() {
    let phase = Arc::new(AtomicU8::new(0));
    let said = Arc::clone(&phase);
    let stand_in = StandIn::new(move |path, body| departing(&said, path, body));
    let path = credentials("again", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    phase.store(1, Ordering::SeqCst);
    watch.look(&now());
    phase.store(3, Ordering::SeqCst);
    let later = now().checked_add(jiff::Span::new().minutes(1)).unwrap();
    let state = watch.look(&later).clone();
    let held: Vec<_> = state.mail.iter().map(|mail| mail.id.as_str()).collect();
    assert_eq!(held, ["new", "old"]);
    assert!(state.finished.is_empty(), "{:?}", state.finished);
    let asked = stand_in
        .heard()
        .into_iter()
        .filter(|heard| {
            heard
                .line
                .starts_with("GET /gmail/v1/users/me/messages/new?")
        })
        .filter(|heard| heard.line.contains("format=minimal"))
        .count();
    assert_eq!(asked, 1, "its labels are asked for only while it's gone");
}

#[test]
fn mail_marked_read_then_unread_wants_joel_again_at_once_in_its_place() {
    let stand_in = StandIn::new(tidied);
    let path = credentials("unread", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    watch.tidy("new", Tidy::Read, &now());
    let state = watch.unread("new", &now()).clone();
    let held: Vec<_> = state.mail.iter().map(|mail| mail.id.as_str()).collect();
    assert_eq!(held, ["new", "old"]);
    assert_eq!(state.unread, Some(5));
    let asked = stand_in
        .heard()
        .into_iter()
        .rfind(|heard| heard.line.starts_with("POST /gmail"))
        .unwrap();
    let body: serde_json::Value = serde_json::from_str(&asked.body).unwrap();
    assert_eq!(body, serde_json::json!({"addLabelIds": ["UNREAD"]}));
    // Marked unread twice, it's held once.
    let again = watch.unread("new", &now()).clone();
    assert_eq!(again.mail.len(), 2);
    assert_eq!(again.unread, Some(5));
}

#[test]
fn mail_archived_isnt_brought_back_by_marking_it_unread() {
    let stand_in = StandIn::new(tidied);
    let path = credentials("archived", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    watch.tidy("new", Tidy::Archive, &now());
    let state = watch.unread("new", &now()).clone();
    let held: Vec<_> = state.mail.iter().map(|mail| mail.id.as_str()).collect();
    assert_eq!(held, ["old"]);
}

#[test]
fn mail_google_refuses_to_mark_stays_held() {
    let stand_in = StandIn::new(|path, body| {
        if path.starts_with("POST /gmail") {
            (404, "{}".into())
        } else {
            gmail(path, body)
        }
    });
    let path = credentials("unmarked", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let before = watch.look(&now()).clone();
    assert_eq!(watch.tidy("new", Tidy::Archive, &now()), &before);
}
