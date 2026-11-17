//! The program's look at the account, against a stand-in for Google on
//! this machine: what it asks, what it tells the shell, and that the
//! harness's file is left as it was. The stand-in and what every look
//! shares are here; the looks themselves are beside this, a file to each
//! of signing in, mail, the calendar, Drive and the receptors.

mod calendar;
mod drive;
mod mail;
mod receptors;
mod sign_in;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use google::api::Origins;
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
    fn new(answer: impl Fn(&str, &str) -> (u16, String) + Send + 'static) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        let here = origin.clone();
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
                // A save begun resumably is told where its bytes go, as
                // Google tells it: a session named for the file replaced,
                // or `created` for a new one.
                let session = line
                    .contains("uploadType=resumable")
                    .then(|| {
                        let path = line.split(' ').nth(1).unwrap_or("");
                        let file = path.split('?').next().unwrap_or("");
                        match file.strip_prefix("/upload/drive/v3/files/") {
                            Some(id) => id.to_owned(),
                            None => "created".to_owned(),
                        }
                    })
                    .filter(|_| status == 200)
                    .map(|id| format!("Location: {here}/upload/session/{id}\r\n"))
                    .unwrap_or_default();
                let mut stream = stream;
                write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{session}\
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
            meet: self.origin.clone(),
        }
    }

    fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }
}

/// How many folders this process has handed out, so each is its own.
static HANDED: AtomicUsize = AtomicUsize::new(0);

/// An empty folder that is this call's alone, whatever `test` names it.
/// Looks run beside each other in one process, and two that chose one
/// name shared a sign-in: one's write truncates the file before filling
/// it, so the other's look could read it empty or half-written. One left
/// by an earlier run is emptied rather than read.
fn folder(test: &str) -> PathBuf {
    let handed = HANDED.fetch_add(1, Ordering::SeqCst);
    let folder =
        std::env::temp_dir().join(format!("google-{test}-{}-{handed}", std::process::id()));
    // None there is the usual case, and fine.
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// The harness's file, in a folder of its own, with pi's access token
/// ending at `expires`.
fn credentials(test: &str, expires: i64) -> PathBuf {
    let folder = folder(test);
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

#[test]
fn two_looks_of_one_name_are_handed_sign_ins_of_their_own() {
    // The mail and sign-in looks both named theirs "refused", and the
    // mail look failed train runs when it read the other's half-written.
    assert_ne!(
        credentials("shared", 0),
        credentials("shared", 0),
        "two looks of one name were handed the same sign-in"
    );
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

/// The stand-in's Gmail: two pieces of mail that want Joel, the newer
/// listed second, five unread in the inbox, and one Joel sent lately.
fn gmail(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("GET /gmail/v1/users/me/messages?") && path.contains("newer_than") {
        return (
            200,
            r#"{"messages": [{"id": "sent", "threadId": "t3"}]}"#.into(),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/messages/sent?") {
        return (
            200,
            r#"{"id": "sent", "threadId": "t3", "internalDate": "1793030000000",
                "labelIds": ["SENT"],
                "payload": {"headers": [{"name": "From", "value": "Joel <joel@example.com>"},
                {"name": "To", "value": "Ada <ada@example.com>"},
                {"name": "Subject", "value": "Lunch"}]}}"#
                .into(),
        );
    }
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

/// What a request to the stand-in asks by `name`, as Google reads it.
fn asked(path: &str, name: &str) -> String {
    let query = path
        .split(' ')
        .nth(1)
        .and_then(|target| target.split_once('?'))
        .map_or("", |(_, query)| query);
    let said = query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&format!("{name}=")))
        .unwrap_or("");
    let mut bytes = Vec::new();
    let mut rest = said.bytes();
    while let Some(byte) = rest.next() {
        match byte {
            b'+' => bytes.push(b' '),
            b'%' => {
                let high = rest.next().unwrap_or(b'0');
                let low = rest.next().unwrap_or(b'0');
                let hex = String::from_utf8(vec![high, low]).unwrap();
                bytes.push(u8::from_str_radix(&hex, 16).unwrap());
            }
            byte => bytes.push(byte),
        }
    }
    String::from_utf8(bytes).unwrap()
}
