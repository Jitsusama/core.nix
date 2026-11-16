//! Drive: the files mail carries, the documents and places told, folders
//! listed, files downloaded and saves sent up.

use std::fs;
use std::path::PathBuf;

use google::api::Google;
use google::drive::{DOWNLOADS_KEPT, Document, Is, LISTINGS_KEPT, Unlisted, Unsaved};
use google::files::{Failed, How, Places};
use google::saves;
use google::state::Standing;
use google::watch::Watch;
use jiff::Zoned;

use super::{StandIn, asked, credentials, gmail, google, now};

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
fn a_file_mail_carries_is_fetched_only_when_asked_and_a_program_never_to_open() {
    let stand_in = StandIn::new(carrying);
    let path = credentials("files", now().timestamp().as_millisecond() + 3_600_000);
    let folder = path.with_file_name("files");
    let places = Places {
        kept: folder.join("kept"),
        drive: folder.join("drive"),
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

/// The stand-in's Drive places: a PDF and a sheet Joel looked at lately, a
/// starred folder, a page more of what's shared with him than one asks,
/// and two shared drives, one he hid.
const RECENT: &str = r#"{"files": [
    {"id": "pdf1", "name": "menu.pdf", "mimeType": "application/pdf", "size": "4",
        "webViewLink": "https://drive.google.com/file/d/pdf1/view",
        "viewedByMeTime": "2026-10-26T19:00:00Z", "owners": [{"displayName": "Ada Lovelace"}]},
    {"id": "s1", "name": "About s1", "mimeType": "application/vnd.google-apps.spreadsheet",
        "webViewLink": "https://docs.google.com/spreadsheets/d/s1/edit",
        "viewedByMeTime": "2026-10-26T18:00:00Z"}
]}"#;
const STARRED: &str = r#"{"files": [
    {"id": "dir1", "name": "Trips", "mimeType": "application/vnd.google-apps.folder",
        "starred": true, "webViewLink": "https://drive.google.com/drive/folders/dir1"}
]}"#;
const SHARED: &str = r#"{"nextPageToken": "more", "files": [
    {"id": "b1", "name": "Team Budget", "mimeType": "application/vnd.google-apps.spreadsheet",
        "webViewLink": "https://docs.google.com/spreadsheets/d/b1/edit",
        "sharedWithMeTime": "2026-10-25T12:00:00Z",
        "sharingUser": {"displayName": "Grace Hopper"}}
]}"#;
const DRIVES: &str = r#"{"drives": [
    {"id": "0Team", "name": "Team", "createdTime": "2026-01-05T12:00:00Z"},
    {"id": "0Old", "name": "Archive", "hidden": true}
]}"#;

/// The stand-in's Drive: one document of each kind Joel looked at, by the
/// kind each search asks for, and its places, by theirs.
fn drive(path: &str, body: &str) -> (u16, String) {
    if path.starts_with("GET /drive/v3/drives?") {
        return (200, DRIVES.into());
    }
    if !path.starts_with("GET /drive/v3/files?") {
        return google(path, body);
    }
    let search = asked(path, "q");
    if search.starts_with("starred") {
        return (200, STARRED.into());
    }
    if search.starts_with("sharedWithMe") {
        return (200, SHARED.into());
    }
    if search.contains("folder") {
        return (200, RECENT.into());
    }
    let (id, link) = if search.contains("document") {
        ("d1", "document")
    } else if search.contains("spreadsheet") {
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
            .filter(|heard| {
                heard.line.starts_with("GET /drive/v3/files?")
                    && asked(&heard.line, "q").starts_with("mimeType = ")
            })
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
    let first = watch.look(&now()).clone();
    let held = first.documents.clone();
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
    assert_eq!(state.drive.places, first.drive.places);
    assert_eq!(first.drive.places.recent.len(), 2);
}

#[test]
fn drives_places_are_told_each_as_drive_lists_it() {
    let stand_in = StandIn::new(drive);
    let path = credentials("places", now().timestamp().as_millisecond() + 3_600_000);
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let places = watch.look(&now()).drive.places.clone();
    let ids = |files: &[google::drive::Document]| -> Vec<String> {
        files.iter().map(|file| file.id.clone()).collect()
    };
    // Recent holds every kind, in Drive's order, each as every row reads it.
    assert_eq!(ids(&places.recent), ["pdf1", "s1"]);
    assert_eq!(
        (places.recent[0].kind, places.recent[0].size),
        (Is::File, Some(4))
    );
    assert_eq!(places.recent[1].kind, Is::Spreadsheet);
    assert_eq!(
        (ids(&places.starred.files), places.starred.more),
        (vec!["dir1".to_owned()], false)
    );
    // Drive saying there's another page is told as more.
    assert_eq!(
        (ids(&places.shared.files), places.shared.more),
        (vec!["b1".to_owned()], true)
    );
    assert_eq!(
        places.shared.files[0].shared_by.as_deref(),
        Some("Grace Hopper")
    );
    // A shared drive he hid is left out, as Drive leaves it out.
    let drives: Vec<_> = places
        .drives
        .files
        .iter()
        .map(|drive| (drive.title.as_str(), drive.link.as_str()))
        .collect();
    assert_eq!(
        drives,
        [("Team", "https://drive.google.com/drive/folders/0Team")]
    );
}

#[test]
fn each_place_is_asked_by_its_own_search_and_again_with_the_documents() {
    let stand_in = StandIn::new(drive);
    let path = credentials(
        "places-asked",
        now().timestamp().as_millisecond() + 3_600_000,
    );
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    watch.look(&now());
    // Each place is asked for by its own search, in its own order, from
    // every drive, a page of it at most, Recent as many as Recent lists.
    let asks = |stand_in: &StandIn| -> Vec<(String, String, String, String)> {
        stand_in
            .heard()
            .into_iter()
            .filter(|heard| heard.line.starts_with("GET /drive/v3/files?"))
            .map(|heard| {
                let of = |name: &str| asked(&heard.line, name);
                (of("q"), of("orderBy"), of("pageSize"), of("corpora"))
            })
            .filter(|(search, ..)| !search.starts_with("mimeType = "))
            .collect()
    };
    let all = |order: &str, size: &str| (order.to_owned(), size.to_owned(), "allDrives".to_owned());
    let first: Vec<_> = asks(&stand_in)
        .into_iter()
        .map(|(search, order, size, corpora)| (search, (order, size, corpora)))
        .collect();
    assert_eq!(
        first,
        [
            (
                "trashed = false and viewedByMeTime > '1970-01-01T00:00:00' and mimeType != \
                 'application/vnd.google-apps.folder'"
                    .to_owned(),
                all("viewedByMeTime desc", "9")
            ),
            (
                "starred = true and trashed = false".to_owned(),
                all("recency desc", "100")
            ),
            (
                "sharedWithMe = true and trashed = false".to_owned(),
                all("sharedWithMeTime desc", "100")
            ),
        ]
    );
    // What every row says of a file is asked of each, and of each drive
    // when it was made and whether he hid it.
    let heard = stand_in.heard();
    let place = heard
        .iter()
        .find(|heard| asked(&heard.line, "q").starts_with("starred"))
        .unwrap();
    assert_eq!(
        asked(&place.line, "fields"),
        format!("nextPageToken,files({})", google::drive::FILE_FIELDS)
    );
    let drives = heard
        .iter()
        .find(|heard| heard.line.starts_with("GET /drive/v3/drives?"))
        .unwrap();
    assert_eq!(
        asked(&drives.line, "fields"),
        "nextPageToken,drives(id,name,createdTime,hidden)"
    );
    // Asked again only once the documents are, five minutes on.
    watch.look(&now());
    assert_eq!(
        asks(&stand_in).len(),
        3,
        "Drive's places were asked again at once"
    );
    let later: Zoned = "2026-10-26T15:25:00-04:00[-04:00]".parse().unwrap();
    watch.look(&later);
    assert_eq!(
        asks(&stand_in).len(),
        6,
        "Drive's places weren't asked again after five minutes"
    );
}

#[test]
fn a_place_drive_wont_answer_costs_the_rest_nothing() {
    let stand_in = StandIn::new(|path, body| {
        if asked(path, "q").starts_with("starred") {
            (400, "{}".into())
        } else {
            drive(path, body)
        }
    });
    let path = credentials(
        "a-place-down",
        now().timestamp().as_millisecond() + 3_600_000,
    );
    let mut watch = Watch::new(Google::new(stand_in.origins()), path, None);
    let state = watch.look(&now()).clone();
    assert_eq!(state.account, Standing::Ready);
    assert_eq!(state.documents.len(), 3);
    assert_eq!(state.drive.places.starred.files, Vec::<Document>::new());
    assert_eq!(state.drive.places.recent.len(), 2);
    assert_eq!(state.drive.places.shared.files.len(), 1);
    assert_eq!(state.drive.places.drives.files.len(), 1);
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
            format!(
                r#"{{"id": "{id}", "name": "{name}", "mimeType": "{mime}",
                    "modifiedTime": "2026-10-26T18:00:00Z"}}"#
            ),
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
        drive: folder.join("drive"),
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
    assert_eq!(opened, places.drive.join("pdf1").join("menu.pdf"));
    assert_eq!(fs::read(&opened).unwrap(), b"menu");
    // Asked again while Drive hasn't changed it, the copy kept is handed
    // out, and its bytes never asked for twice.
    let (picked, _) = download("pdf1", How::Pick, None);
    assert_eq!(picked, Some(opened.clone()));
    let fetched = |stand_in: &StandIn| {
        stand_in
            .heard()
            .iter()
            .filter(|heard| heard.line.contains("/pdf1?alt=media"))
            .count()
    };
    assert_eq!(fetched(&stand_in), 1);
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
    assert_eq!(downloads.len(), 9);
    assert_eq!(downloads[1].file, "lost1");
    for _ in 0..30 {
        watch.downloading("pdf1", How::Pick, None, &now());
    }
    let state = watch.downloading("pdf1", How::Pick, None, &now()).clone();
    assert_eq!(state.drive.downloads.len(), DOWNLOADS_KEPT);
    fs::remove_dir_all(&folder).unwrap();
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
    // The picker says which program saved it, beside the file.
    fs::write(root.join("root/.Notes.txt.from"), "org.gnome.TextEditor").unwrap();
    let save = |watch: &mut Watch, at: &str, bytes: &str| {
        let staged = saves::staged(&root, &root.join(at)).unwrap();
        fs::write(&staged.path, bytes).unwrap();
        let going = watch.uploading(&staged, &now()).drive.uploads[0].clone();
        assert_eq!((going.id, going.failed), (None, None));
        let from = if at == "root/Notes.txt" {
            Some("org.gnome.TextEditor")
        } else {
            None
        };
        assert_eq!(going.from.as_deref(), from);
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
