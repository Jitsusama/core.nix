//! The program's look at the account, against a stand-in for Google on
//! this machine: what it asks, what it tells the shell, and that the
//! harness's file is left as it was.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use google::api::{Google, Origins};
use google::calendar::Answer;
use google::mail::Tidy;
use google::state::Standing;
use google::watch::Watch;
use jiff::Zoned;

/// One request the stand-in heard: its first line, its Authorization and
/// its body.
#[derive(Clone, Debug)]
struct Heard {
    line: String,
    authorization: String,
    body: String,
}

/// A stand-in for Google's token endpoint and Calendar API, answering each
/// request by `answer`, given its first line and body, with a status and a
/// JSON body.
struct StandIn {
    origin: String,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl StandIn {
    fn new(answer: fn(&str, &str) -> (u16, String)) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let (mut authorization, mut length) = (String::new(), 0);
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end();
                    if header.is_empty() {
                        break;
                    }
                    let (name, value) = header.split_once(": ").unwrap_or((header, ""));
                    match name.to_ascii_lowercase().as_str() {
                        "authorization" => value.clone_into(&mut authorization),
                        "content-length" => length = value.parse().unwrap(),
                        _ => {}
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                let (status, reply) = answer(line.trim_end(), &body);
                kept.lock().unwrap().push(Heard {
                    line: line.trim_end().to_owned(),
                    authorization,
                    body,
                });
                let mut stream = stream;
                write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                )
                .unwrap();
            }
        });
        StandIn { origin, heard }
    }

    fn origins(&self) -> Origins {
        Origins {
            token: format!("{}/token", self.origin),
            api: self.origin.clone(),
        }
    }

    fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }
}

/// The harness's file, in a folder of the test's own, with pi's access
/// token ending at `expires`.
fn credentials(test: &str, expires: i64) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("google-{test}-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("credentials.json");
    fs::write(
        &path,
        format!(
            r#"{{"oauthApp": {{"clientId": "app", "clientSecret": "secret"}},
                "accounts": [{{"name": "work", "isDefault": true}}],
                "tokens": {{"work": {{"access_token": "pis", "refresh_token": "refresh",
                    "expiry_date": {expires}, "token_type": "Bearer", "scope": "calendar"}}}}}}"#
        ),
    )
    .unwrap();
    path
}

fn now() -> Zoned {
    "2026-10-26T15:20:00-04:00[-04:00]".parse().unwrap()
}

const EVENTS: &str = r#"{"items": [{"id": "e1", "summary": "Standup",
    "start": {"dateTime": "2026-10-26T16:00:00-04:00"},
    "end": {"dateTime": "2026-10-26T16:15:00-04:00"},
    "hangoutLink": "https://meet.google.com/abc-defg-hij"}]}"#;

fn google(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("POST /token") {
        assert!(body.contains("grant_type=refresh_token"), "{body}");
        assert!(body.contains("refresh_token=refresh"), "{body}");
        return (
            200,
            r#"{"access_token": "mine", "expires_in": 3599}"#.into(),
        );
    }
    (200, EVENTS.into())
}

#[test]
fn a_stale_token_is_refreshed_and_the_meetings_are_told() {
    let stand_in = StandIn::new(google);
    let path = credentials("refreshed", 0);
    let before = fs::read(&path).unwrap();
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.meetings[0].title, "Standup");
    let heard = stand_in.heard();
    assert!(heard[0].line.starts_with("POST /token"));
    let asked = &heard[1];
    assert_eq!(asked.authorization, "Bearer mine");
    assert!(
        asked
            .line
            .starts_with("GET /calendar/v3/calendars/primary/events?")
    );
    assert!(
        asked.line.contains("timeMin=2026-10-26T04%3A00%3A00Z"),
        "{}",
        asked.line
    );
    assert!(asked.line.contains("singleEvents=true"));
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "the harness's file was written"
    );
}

#[test]
fn pis_fresh_token_is_used_without_a_refresh() {
    let stand_in = StandIn::new(google);
    let path = credentials("fresh", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    assert_eq!(watch.look(&now()).account, Standing::Ready);
    let heard = stand_in.heard();
    assert!(
        heard
            .iter()
            .all(|heard| !heard.line.starts_with("POST /token"))
    );
    assert!(
        heard
            .iter()
            .all(|heard| heard.authorization == "Bearer pis")
    );
}

/// The stand-in's Gmail: two pieces of mail that want Joel, the newer
/// listed second, and five unread in the inbox.
fn gmail(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("GET /gmail/v1/users/me/messages?") {
        return (
            200,
            r#"{"messages": [{"id": "old", "threadId": "t1"}, {"id": "new", "threadId": "t2"}]}"#
                .into(),
        );
    }
    if let Some(rest) = path.strip_prefix("GET /gmail/v1/users/me/messages/") {
        let (id, at) = if rest.starts_with("old?") {
            ("old", "1793040000000")
        } else {
            ("new", "1793044000000")
        };
        return (
            200,
            format!(
                r#"{{"id": "{id}", "threadId": "t-{id}", "internalDate": "{at}",
                    "payload": {{"headers": [{{"name": "From", "value": "Ada <ada@example.com>"}},
                    {{"name": "Subject", "value": "About {id}"}}]}}}}"#
            ),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/labels/INBOX") {
        return (200, r#"{"messagesUnread": 5}"#.into());
    }
    google(path, body)
}

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
    assert_eq!(asked(&stand_in), 2);
    watch.look(&now());
    assert_eq!(asked(&stand_in), 2, "mail already held was asked for again");
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

#[test]
fn a_refused_sign_in_is_signed_out_and_not_asked_again() {
    let stand_in = StandIn::new(|path, _| {
        if path.starts_with("POST /token") {
            (400, r#"{"error": "invalid_grant"}"#.into())
        } else {
            (200, EVENTS.into())
        }
    });
    let mut watch = Watch::new(
        Google::new(stand_in.origins()),
        credentials("refused", 0),
        None,
    );
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::SignedOut);
    assert!(state.meetings.is_empty());
    watch.look(&now());
    assert_eq!(
        stand_in.heard().len(),
        1,
        "a refused sign-in was offered again"
    );
}

#[test]
fn a_revoked_token_is_refreshed_past_once_then_waits_for_a_new_sign_in() {
    let stand_in = StandIn::new(|path, _| {
        if path.starts_with("POST /token") {
            (
                200,
                r#"{"access_token": "mine", "expires_in": 3599}"#.into(),
            )
        } else if path.starts_with("GET") && !path.contains("pageToken") {
            (401, "{}".into())
        } else {
            (200, EVENTS.into())
        }
    });
    let path = credentials("revoked", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    assert_eq!(watch.look(&now()).account, Standing::SignedOut);
    let lines: Vec<_> = stand_in
        .heard()
        .into_iter()
        .map(|heard| heard.authorization)
        .collect();
    assert_eq!(lines, vec!["Bearer pis", "", "Bearer mine"]);
    watch.look(&now());
    assert_eq!(
        stand_in.heard().len(),
        3,
        "a refused sign-in was offered again"
    );
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
fn an_unreachable_google_keeps_what_was_heard() {
    let stand_in = StandIn::new(google);
    let path = credentials("unreachable", 0);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path.clone(), None);
    let heard = watch.look(&now()).heard;
    // A port nothing listens on, as Google is to a laptop offline.
    let closed = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    watch.google = Google::new(Origins {
        token: format!("http://{closed}/token"),
        api: format!("http://{closed}"),
    });
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Unreachable);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.heard, heard);
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
fn an_answer_changes_joels_own_and_is_held_at_once() {
    let stand_in = StandIn::new(invited);
    let path = credentials("answer", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    let state = watch.answer("e1", Answer::Maybe, &now()).clone();
    assert_eq!(state.meetings[0].answer, Some(Answer::Maybe));
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
    assert!(watch.look(&now()).meetings[0].holds);
    let state = watch.cancel("e1", &now()).clone();
    assert!(state.meetings.is_empty());
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
            .all(|heard| !heard.line.starts_with("DELETE"))
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

#[test]
fn no_file_is_no_account_and_google_is_never_asked() {
    let stand_in = StandIn::new(google);
    let mut watch = Watch::new(
        Google::new(stand_in.origins()),
        PathBuf::from("/nonexistent/credentials.json"),
        None,
    );
    assert_eq!(watch.look(&now()).account, Standing::None);
    assert!(stand_in.heard().is_empty());
}
