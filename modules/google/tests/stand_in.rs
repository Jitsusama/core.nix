//! The program's look at the account, against a stand-in for Google on
//! this machine: what it asks, what it tells the shell, and that the
//! harness's file is left as it was.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use google::api::{Google, Origins};
use google::calendar::Answer;
use google::drive::{DOWNLOADS_KEPT, Is, LISTINGS_KEPT, Unlisted, Unsaved};
use google::files::{Failed, How, Places};
use google::mail::Tidy;
use google::saves;
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

/// The stand-in's Gmail with one piece of mail carrying a menu, whose
/// bytes Gmail keeps apart, and a program.
fn carrying(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("GET /gmail/v1/users/me/messages?") {
        return (
            200,
            r#"{"messages": [{"id": "m", "threadId": "t"}]}"#.into(),
        );
    }
    if path.starts_with("GET /gmail/v1/users/me/messages/m/attachments/menu?") {
        // "hello", as Gmail's URL-safe base64 leaves it, unpadded.
        return (200, r#"{"data": "aGVsbG8"}"#.into());
    }
    if path.starts_with("GET /gmail/v1/users/me/messages/m?") {
        return (
            200,
            r#"{"id": "m", "threadId": "t", "internalDate": "1793040000000",
                "payload": {"mimeType": "multipart/mixed", "headers": [], "parts": [
                {"mimeType": "application/pdf", "filename": "menu.pdf",
                    "body": {"attachmentId": "menu", "size": 5}},
                {"mimeType": "application/octet-stream", "filename": "setup.exe",
                    "body": {"attachmentId": "setup", "size": 9}}]}}"#
                .into(),
        );
    }
    gmail(path, body)
}

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
    assert!(watch.look(&now()).meet.granted);
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
fn a_file_mail_carries_is_fetched_only_when_asked_and_a_program_never_to_open() {
    let stand_in = StandIn::new(carrying);
    let path = credentials("files", now().timestamp().as_millisecond() + 3_600_000);
    let folder = path.with_file_name("files");
    let places = Places {
        kept: folder.join("kept"),
        saves: folder.join("Downloads"),
    };
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let files = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains("/attachments/"))
            .count()
    };
    let state = watch.look(&now()).clone();
    assert_eq!(state.mail[0].attachments.len(), 2);
    assert_eq!(
        files(&stand_in),
        0,
        "a file was fetched before it was asked for"
    );
    // While Gmail answers, the file is said to be on its way.
    let waiting = watch
        .fetching("m", 0, How::Open, None, &now())
        .fetched
        .clone()
        .unwrap();
    assert_eq!((waiting.path, waiting.failed), (None, None));
    let opened = watch.fetch(&places, &now()).fetched.clone().unwrap();
    let opened = PathBuf::from(opened.path.unwrap());
    assert_eq!(opened, places.kept.join("m").join("menu.pdf"));
    assert_eq!(fs::read(&opened).unwrap(), b"hello");
    watch.fetching("m", 0, How::Save, None, &now());
    let saved = watch.fetch(&places, &now()).fetched.clone().unwrap();
    assert_eq!(
        saved.path.map(PathBuf::from),
        Some(places.saves.join("menu.pdf"))
    );
    assert_eq!(files(&stand_in), 2);
    // A copy goes where Joel picked when he picked somewhere, over what's
    // there, since the chooser asked him first.
    let picked = folder.join("picked.pdf");
    fs::write(&picked, b"old").unwrap();
    let to = Some(picked.display().to_string());
    watch.fetching("m", 0, How::Save, to, &now());
    let put = watch.fetch(&places, &now()).fetched.clone().unwrap();
    assert_eq!(put.path.map(PathBuf::from), Some(picked.clone()));
    assert_eq!(fs::read(&picked).unwrap(), b"hello");
    assert_eq!(files(&stand_in), 3);
    // A program is never fetched to open, and a file no mail holds isn't
    // asked for.
    watch.fetching("m", 1, How::Open, None, &now());
    let program = watch.fetch(&places, &now()).fetched.clone().unwrap();
    assert_eq!((program.path, program.failed), (None, Some(Failed::Runs)));
    watch.fetching("m", 5, How::Open, None, &now());
    let gone = watch.fetch(&places, &now()).fetched.clone().unwrap();
    assert_eq!(gone.failed, Some(Failed::Gone));
    assert_eq!(files(&stand_in), 3);
    fs::remove_dir_all(&folder).unwrap();
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

/// The stand-in's Drive: one document of each kind Joel looked at, by the
/// kind each search asks for.
fn drive(path: &str, body: &str) -> (u16, String) {
    if !path.starts_with("GET /drive/v3/files?") {
        return google(path, body);
    }
    let (id, link) = if path.contains("document") {
        ("d1", "document")
    } else if path.contains("spreadsheet") {
        ("s1", "spreadsheets")
    } else {
        ("p1", "presentation")
    };
    (
        200,
        format!(
            r#"{{"files": [{{"id": "{id}", "name": "About {id}",
                "webViewLink": "https://docs.google.com/{link}/d/{id}/edit",
                "viewedByMeTime": "2026-10-26T18:00:00Z",
                "owners": [{{"displayName": "Ada Lovelace"}}]}}]}}"#
        ),
    )
}

#[test]
fn each_kind_of_document_is_told_and_drive_asked_again_only_after_a_while() {
    let stand_in = StandIn::new(drive);
    let path = credentials("documents", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    let said: Vec<_> = state
        .documents
        .iter()
        .map(|document| (document.id.as_str(), document.title.as_str()))
        .collect();
    assert_eq!(
        said,
        [("d1", "About d1"), ("s1", "About s1"), ("p1", "About p1")]
    );
    let searches = |stand_in: &StandIn| {
        stand_in
            .heard()
            .into_iter()
            .filter(|heard| heard.line.starts_with("GET /drive/v3/files?"))
            .collect::<Vec<_>>()
    };
    let first = searches(&stand_in);
    assert_eq!(first.len(), 3);
    assert!(
        first[0].line.contains("orderBy=viewedByMeTime+desc")
            || first[0].line.contains("orderBy=viewedByMeTime%20desc"),
        "{}",
        first[0].line
    );
    assert!(first[0].line.contains("pageSize=9"));
    watch.look(&now());
    assert_eq!(
        searches(&stand_in).len(),
        3,
        "Drive was asked again at once"
    );
    let later: Zoned = "2026-10-26T15:25:00-04:00[-04:00]".parse().unwrap();
    watch.look(&later);
    assert_eq!(
        searches(&stand_in).len(),
        6,
        "Drive wasn't asked again after five minutes"
    );
}

#[test]
fn drive_failing_keeps_the_documents_held_and_the_rest_told() {
    let stand_in = StandIn::new(drive);
    let path = credentials("drive-down", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let held = watch.look(&now()).documents.clone();
    let down = StandIn::new(|path, body| {
        if path.starts_with("GET /drive") {
            (503, "{}".into())
        } else {
            google(path, body)
        }
    });
    watch.google = Google::new(down.origins());
    let later: Zoned = "2026-10-26T15:30:00-04:00[-04:00]".parse().unwrap();
    let state = watch.look(&later).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(state.meetings.len(), 1);
    assert_eq!(state.documents, held);
    assert_eq!(held.len(), 3);
}

/// The stand-in's Drive folders: My Drive holds a folder, a PDF, a
/// document, a shortcut to a sheet, a form and a program; one shared drive;
/// and each file's bytes, or a document's exported copy.
fn folders(path: &str, body: &str) -> (u16, String) {
    const MINE: &str = r#"{"files": [
        {"id": "pdf1", "name": "menu.pdf", "mimeType": "application/pdf", "size": "4",
            "modifiedTime": "2026-10-26T18:00:00Z", "webViewLink": "https://x/pdf1",
            "owners": [{"displayName": "Ada Lovelace"}]},
        {"id": "doc1", "name": "Plan", "mimeType": "application/vnd.google-apps.document",
            "webViewLink": "https://docs.google.com/document/d/doc1/edit"},
        {"id": "cut1", "name": "Numbers", "mimeType": "application/vnd.google-apps.shortcut",
            "shortcutDetails": {"targetId": "sheet1",
                "targetMimeType": "application/vnd.google-apps.spreadsheet"}},
        {"id": "form1", "name": "Survey", "mimeType": "application/vnd.google-apps.form"},
        {"id": "exe1", "name": "setup.exe", "mimeType": "application/octet-stream",
            "size": "9"},
        {"id": "dir1", "name": "Trips", "mimeType": "application/vnd.google-apps.folder"}
    ]}"#;
    let file = |id: &str, name: &str, mime: &str| {
        (
            200,
            format!(r#"{{"id": "{id}", "name": "{name}", "mimeType": "{mime}"}}"#),
        )
    };
    match path.split_once(' ').map_or("", |(_, rest)| rest) {
        rest if rest.starts_with("/drive/v3/files?") && rest.contains("root") => (200, MINE.into()),
        rest if rest.starts_with("/drive/v3/files?") => (404, "{}".into()),
        rest if rest.starts_with("/drive/v3/drives?") => (
            200,
            r#"{"drives": [{"id": "0Team-1", "name": "Team"}]}"#.into(),
        ),
        rest if rest.starts_with("/drive/v3/files/pdf1?alt=media") => (200, "menu".into()),
        rest if rest.starts_with("/drive/v3/files/exe1?alt=media") => (200, "setup".into()),
        rest if rest.starts_with("/drive/v3/files/doc1/export?") => (200, "docx".into()),
        rest if rest.starts_with("/drive/v3/files/pdf1?") => {
            file("pdf1", "menu.pdf", "application/pdf")
        }
        rest if rest.starts_with("/drive/v3/files/exe1?") => {
            file("exe1", "setup.exe", "application/octet-stream")
        }
        rest if rest.starts_with("/drive/v3/files/doc1?") => {
            file("doc1", "Plan", "application/vnd.google-apps.document")
        }
        rest if rest.starts_with("/drive/v3/files/dir1?") => {
            file("dir1", "Trips", "application/vnd.google-apps.folder")
        }
        rest if rest.starts_with("/drive/v3/files/") => (404, "{}".into()),
        _ => google(path, body),
    }
}

#[test]
fn a_drive_folder_is_listed_only_when_asked_folders_first_and_what_cant_be_shown_left_out() {
    let stand_in = StandIn::new(folders);
    let path = credentials("folders", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let listed = watch.list("root", &now()).drive.listings[0].clone();
    assert_eq!(listed.folder, "root");
    assert_eq!((listed.more, listed.failed), (false, None));
    let names: Vec<(&str, Is, Option<u64>, bool)> = listed
        .files
        .iter()
        .map(|entry| (entry.name.as_str(), entry.kind, entry.size, entry.runs))
        .collect();
    assert_eq!(
        names,
        [
            ("Trips", Is::Folder, None, false),
            ("menu.pdf", Is::File, Some(4), false),
            ("Numbers", Is::Spreadsheet, None, false),
            ("Plan", Is::Document, None, false),
            ("setup.exe", Is::File, Some(9), true),
        ]
    );
    assert_eq!(
        listed.files[2].id, "sheet1",
        "a shortcut isn't what it points at"
    );
    assert_eq!(listed.files[1].owner.as_deref(), Some("Ada Lovelace"));
    let searched = &stand_in.heard()[0];
    assert!(searched.line.contains("trashed"), "{}", searched.line);
    assert!(
        searched.line.contains("supportsAllDrives=true"),
        "{}",
        searched.line
    );
    assert_eq!(searched.authorization, "Bearer pis");

    // Two asked together are each told, the newest first, as the shell
    // may read only once both are written.
    let both = watch.list("drives", &now()).drive.listings.clone();
    let folders: Vec<&str> = both.iter().map(|each| each.folder.as_str()).collect();
    assert_eq!(folders, ["drives", "root"]);
    let drives = &both[0];
    assert_eq!(drives.files.len(), 1);
    assert_eq!(
        (drives.files[0].id.as_str(), drives.files[0].kind),
        ("0Team-1", Is::Folder)
    );
    let gone = watch.list("nowhere", &now()).drive.listings[0].clone();
    assert_eq!((gone.files.len(), gone.failed), (0, Some(Unlisted::Gone)));
    // A folder asked again is told once, by its newest listing, and only
    // the last few asked are kept.
    watch.list("root", &now());
    for folder in ["dir1", "dir2", "dir3"] {
        watch.list(folder, &now());
    }
    let kept: Vec<String> = watch
        .list("drives", &now())
        .drive
        .listings
        .iter()
        .map(|each| each.folder.clone())
        .collect();
    assert_eq!(kept, ["drives", "dir3", "dir2", "dir1"]);
    assert_eq!(kept.len(), LISTINGS_KEPT);
    // A folder no id could name is never asked about.
    let asked = stand_in.heard().len();
    watch.list("x' or '1'='1", &now());
    assert_eq!(stand_in.heard().len(), asked);
}

#[test]
fn a_drive_file_is_downloaded_to_open_pick_or_save_and_googles_own_exported() {
    let stand_in = StandIn::new(folders);
    let path = credentials("downloads", now().timestamp().as_millisecond() + 3_600_000);
    let folder = path.with_file_name("files");
    let places = Places {
        kept: folder.join("kept"),
        saves: folder.join("Downloads"),
    };
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let mut download = |file: &str, how: How, to: Option<String>| {
        let waiting = watch.downloading(file, how, to, &now()).drive.downloads[0].clone();
        assert_eq!((waiting.path, waiting.failed), (None, None));
        let done = watch.download(&places, &now()).drive.downloads[0].clone();
        assert_eq!(done.file, file);
        (done.path.map(PathBuf::from), done.failed)
    };
    let (opened, _) = download("pdf1", How::Open, None);
    let opened = opened.unwrap();
    assert_eq!(
        opened,
        places.kept.join("drive").join("pdf1").join("menu.pdf")
    );
    assert_eq!(fs::read(&opened).unwrap(), b"menu");
    // Google's own kind comes as its office program's file.
    let (saved, _) = download("doc1", How::Save, None);
    assert_eq!(saved, Some(places.saves.join("Plan.docx")));
    assert!(
        stand_in
            .heard()
            .iter()
            .any(|heard| heard.line.contains("/doc1/export?mimeType=")
                && heard.line.contains("wordprocessingml")),
        "the document wasn't exported"
    );
    let picked = folder.join("Picked.docx");
    let (put, _) = download("doc1", How::Save, Some(picked.display().to_string()));
    assert_eq!(put, Some(picked.clone()));
    assert_eq!(fs::read(&picked).unwrap(), b"docx");
    // A program is never opened, but may be handed to the one that
    // picked it.
    assert_eq!(
        download("exe1", How::Open, None),
        (None, Some(Failed::Runs))
    );
    let (handed, _) = download("exe1", How::Pick, None);
    assert_eq!(fs::read(handed.unwrap()).unwrap(), b"setup");
    assert_eq!(
        download("dir1", How::Pick, None),
        (None, Some(Failed::Gone))
    );
    assert_eq!(
        download("lost1", How::Pick, None),
        (None, Some(Failed::Gone))
    );
    // Each asked lately is told, newest first, so several asked together
    // are each still told.
    let downloads = watch
        .downloading("pdf1", How::Pick, None, &now())
        .drive
        .downloads
        .clone();
    assert_eq!(downloads.len(), 8);
    assert_eq!(downloads[1].file, "lost1");
    for _ in 0..30 {
        watch.downloading("pdf1", How::Pick, None, &now());
    }
    let state = watch.downloading("pdf1", How::Pick, None, &now()).clone();
    assert_eq!(state.drive.downloads.len(), DOWNLOADS_KEPT);
    fs::remove_dir_all(&folder).unwrap();
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
        meet: format!("http://{closed}"),
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
            .all(|heard| !heard.line.starts_with("PATCH"))
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

/// Drive taking saves: a new file in My Drive, a new version of one this
/// program made, and refusing to replace one it didn't, or to add to a
/// folder that's gone; a folder's listing is the one `folders` gives.
fn saving(path: &str, body: &str) -> (u16, String) {
    let done = |id: &str| {
        (
            200,
            format!(r#"{{"id": "{id}", "webViewLink": "https://x/{id}"}}"#),
        )
    };
    match path.split_once('?').map_or(path, |(start, _)| start) {
        "POST /upload/drive/v3/files" if body.contains("gone1") => (404, "{}".into()),
        "POST /upload/drive/v3/files" | "PATCH /upload/drive/v3/files/new1" => (200, "{}".into()),
        "PATCH /upload/drive/v3/files/pdf1" => (
            403,
            r#"{"error": {"errors": [{"reason": "appNotAuthorizedToFile"}]}}"#.into(),
        ),
        "PUT /upload/session/created HTTP/1.1" | "PUT /upload/session/new1 HTTP/1.1" => {
            done("new1")
        }
        _ => folders(path, body),
    }
}

#[test]
fn a_save_goes_up_once_as_a_new_file_then_a_new_version_never_a_second_file() {
    let stand_in = StandIn::new(saving);
    let path = credentials("saves", now().timestamp().as_millisecond() + 3_600_000);
    let root = path.with_file_name("drive-saves");
    fs::create_dir_all(root.join("root")).unwrap();
    fs::create_dir_all(root.join("gone1")).unwrap();
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let save = |watch: &mut Watch, at: &str, bytes: &str| {
        let staged = saves::staged(&root, &root.join(at)).unwrap();
        fs::write(&staged.path, bytes).unwrap();
        let going = watch.uploading(&staged, &now()).drive.uploads[0].clone();
        assert_eq!((going.id, going.failed), (None, None));
        let done = watch.upload(&now()).drive.uploads[0].clone();
        assert_eq!((done.folder, done.name), (staged.folder, staged.name));
        (done.id, done.link, done.failed)
    };
    // A new file, named and placed as staged, its bytes sent where Drive
    // said, and its id kept beside the staged copy.
    let notes = root.join("root/Notes.txt");
    assert_eq!(
        save(&mut watch, "root/Notes.txt", "notes"),
        (Some("new1".into()), Some("https://x/new1".into()), None)
    );
    assert_eq!(saves::sent(&notes).as_deref(), Some("new1"));
    assert!(!saves::waiting(&notes));
    let heard = stand_in.heard();
    let lines: Vec<&str> = heard.iter().map(|heard| heard.line.as_str()).collect();
    assert!(
        lines[0].starts_with("POST /upload/drive/v3/files?"),
        "{lines:?}"
    );
    assert!(
        lines[0].contains("uploadType=resumable") && lines[0].contains("supportsAllDrives=true")
    );
    let about: serde_json::Value = serde_json::from_str(&heard[0].body).unwrap();
    assert_eq!(
        about,
        serde_json::json!({ "name": "Notes.txt", "parents": ["root"] })
    );
    assert_eq!(heard[0].authorization, "Bearer pis");
    assert_eq!(
        (lines[1], heard[1].body.as_str()),
        ("PUT /upload/session/created HTTP/1.1", "notes")
    );
    // Saved again, it's a new version of the same file.
    assert_eq!(
        save(&mut watch, "root/Notes.txt", "more notes")
            .0
            .as_deref(),
        Some("new1")
    );
    let heard = stand_in.heard();
    assert!(
        heard[2]
            .line
            .starts_with("PATCH /upload/drive/v3/files/new1?")
    );
    assert_eq!(heard[3].body, "more notes");
    assert_eq!(heard.len(), 4, "{heard:?}");
    // A name the folder already holds, as the picker's Replace It, is a
    // new version of that file, which this program didn't make: refused,
    // and never sent as a second file.
    watch.list("root", &now());
    let replaced = save(&mut watch, "root/menu.pdf", "menu");
    assert_eq!(replaced, (None, None, Some(Unsaved::ReplaceNotGranted)));
    assert_eq!(saves::sent(&root.join("root/menu.pdf")), None);
    let heard = stand_in.heard();
    assert!(
        heard
            .last()
            .unwrap()
            .line
            .starts_with("PATCH /upload/drive/v3/files/pdf1?")
    );
    // A folder gone from Drive is said so, its copy kept to save elsewhere.
    assert_eq!(save(&mut watch, "gone1/x.txt", "x").2, Some(Unsaved::Gone));
    assert!(root.join("gone1/x.txt").exists());
    // Every save is told, newest first.
    let told = watch.list("root", &now()).drive.uploads.clone();
    let names: Vec<&str> = told.iter().map(|told| told.name.as_str()).collect();
    assert_eq!(names, ["x.txt", "menu.pdf", "Notes.txt", "Notes.txt"]);
}
