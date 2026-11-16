//! Drive's cases: what a document, a place's files and the shared drives
//! say of Drive's answers, and how each is asked for; and the documents
//! every test of the crate starts from, said once here so a new figure of
//! Drive's is added to them in one place.

use super::*;
use serde_json::json;

impl Document {
    /// A document as Drive says one with nothing but its id and link.
    pub(crate) fn bare(id: &str, kind: Is, mime: &str, link: &str) -> Self {
        Self {
            id: id.into(),
            title: String::new(),
            kind,
            link: link.into(),
            viewed: None,
            changed: None,
            owner: None,
            mime: mime.into(),
            size: None,
            created: None,
            changed_by: None,
            changed_by_me: None,
            shared_with_me: None,
            shared_by: None,
            shared: false,
            starred: false,
            drive: None,
            about: None,
            across: None,
            down: None,
            lasts: None,
        }
    }

    /// A Google document of Ada Lovelace's, changed at 4 s and last looked
    /// at by Joel at 5 s.
    pub(crate) fn plan(id: &str, title: &str, kind: Is) -> Self {
        Self {
            title: title.into(),
            viewed: Some(5_000),
            changed: Some(4_000),
            owner: Some("Ada Lovelace".into()),
            ..Self::bare(
                id,
                kind,
                "application/vnd.google-apps.document",
                &format!("https://docs.google.com/document/d/{id}/edit"),
            )
        }
    }
}

#[test]
fn a_document_says_every_figure_drive_gives_that_is_read() {
    let answer = json!({ "files": [{
        "id": "d1",
        "name": "Quarterly Plan",
        "mimeType": Kind::Document.mime(),
        "webViewLink": "https://docs.google.com/document/d/d1/edit",
        "viewedByMeTime": "2026-10-28T14:00:00.000Z",
        "modifiedTime": "2026-10-27T09:30:00Z",
        "modifiedByMeTime": "2026-10-26T09:30:00Z",
        "createdTime": "2026-10-20T08:00:00Z",
        "sharedWithMeTime": "2026-10-21T08:00:00Z",
        "starred": true,
        "shared": true,
        "driveId": "0Team",
        "description": "  What we do next quarter.\n",
        "owners": [{ "displayName": "Ada Lovelace" }],
        "lastModifyingUser": { "displayName": "Grace Hopper" },
        "sharingUser": { "displayName": "Ada Lovelace" },
        "size": "2048"
    }]});
    assert_eq!(
        documents(&answer, Kind::Document),
        vec![Document {
            title: "Quarterly Plan".into(),
            viewed: Some(1_793_196_000_000),
            changed: Some(1_793_093_400_000),
            owner: Some("Ada Lovelace".into()),
            created: Some(1_792_483_200_000),
            changed_by: Some("Grace Hopper".into()),
            changed_by_me: Some(1_793_007_000_000),
            shared_with_me: Some(1_792_569_600_000),
            shared_by: Some("Ada Lovelace".into()),
            shared: true,
            starred: true,
            drive: Some("0Team".into()),
            about: Some("What we do next quarter.".into()),
            // Google's own kinds have no bytes of their own to weigh.
            ..Document::bare(
                "d1",
                Is::Document,
                Kind::Document.mime(),
                "https://docs.google.com/document/d/d1/edit"
            )
        }]
    );
}

#[test]
fn one_on_a_shared_drive_has_no_owner_and_one_untitled_no_title() {
    let answer = json!({ "files": [{
        "id": "s1",
        "mimeType": Kind::Spreadsheet.mime(),
        "webViewLink": "https://docs.google.com/spreadsheets/d/s1/edit"
    }]});
    let found = documents(&answer, Kind::Spreadsheet);
    assert_eq!(found[0].title, "");
    assert_eq!(found[0].owner, None);
    assert_eq!(found[0].viewed, None);
}

#[test]
fn one_with_no_link_is_left_out_and_no_more_than_nine_are_kept() {
    let mut files: Vec<Value> = (0..12)
        .map(|at| json!({ "id": format!("p{at}"), "webViewLink": format!("https://x/{at}") }))
        .collect();
    files.insert(0, json!({ "id": "lost" }));
    let found = documents(&json!({ "files": files }), Kind::Presentation);
    assert_eq!(found.len(), SHOWN);
    assert_eq!(found[0].id, "p0");
}

#[test]
fn a_document_said_without_its_type_is_its_kinds() {
    let answer = json!({ "files": [{ "id": "s1", "webViewLink": "https://x/s1" }]});
    assert_eq!(
        documents(&answer, Kind::Spreadsheet)[0].mime,
        Kind::Spreadsheet.mime()
    );
}

#[test]
fn a_place_holds_every_kind_a_shortcut_as_what_it_points_at() {
    let answer = json!({ "files": [
        { "id": "pdf1", "name": "menu.pdf", "mimeType": "application/pdf",
            "size": "2400000", "webViewLink": "https://drive.google.com/file/d/pdf1/view" },
        { "id": "dir1", "name": "Trips", "mimeType": FOLDER, "size": "0",
            "webViewLink": "https://drive.google.com/drive/folders/dir1" },
        { "id": "form1", "name": "Survey", "mimeType": "application/vnd.google-apps.form",
            "webViewLink": "https://docs.google.com/forms/d/form1/edit" },
        { "id": "cut1", "name": "Numbers", "mimeType": SHORTCUT,
            "webViewLink": "https://drive.google.com/file/d/cut1/view",
            "shortcutDetails": { "targetId": "sheet1",
                "targetMimeType": Kind::Spreadsheet.mime() } },
        { "id": "cut2", "name": "Lost", "mimeType": SHORTCUT, "webViewLink": "https://x" },
        { "id": "nolink", "name": "Nowhere", "mimeType": "text/plain" }
    ]});
    let found = placed(&answer, Place::Starred);
    let said: Vec<_> = found
        .files
        .iter()
        .map(|file| (file.id.as_str(), file.kind, file.size))
        .collect();
    assert_eq!(
        said,
        [
            ("pdf1", Is::File, Some(2_400_000)),
            ("dir1", Is::Folder, None),
            ("form1", Is::Other, None),
            ("sheet1", Is::Spreadsheet, None),
        ]
    );
    assert!(!found.more, "a place with no next page says it holds more");
}

#[test]
fn a_picture_says_its_size_and_a_video_its_length_too() {
    let answer = json!({ "files": [
        { "id": "png1", "mimeType": "image/png", "webViewLink": "https://x/png1",
            "imageMediaMetadata": { "width": 2400, "height": 1600 } },
        { "id": "mp41", "mimeType": "video/mp4", "webViewLink": "https://x/mp41",
            "videoMediaMetadata": { "width": 1920, "height": 1080,
                "durationMillis": "221000" } }
    ]});
    let found = placed(&answer, Place::Recent).files;
    let said: Vec<_> = found
        .iter()
        .map(|file| (file.across, file.down, file.lasts))
        .collect();
    assert_eq!(
        said,
        [
            (Some(2400), Some(1600), None),
            (Some(1920), Some(1080), Some(221_000))
        ]
    );
}

#[test]
fn recent_keeps_nine_and_a_place_says_when_it_holds_more() {
    let files: Vec<Value> = (0..12)
        .map(|at| json!({ "id": format!("f{at}"), "webViewLink": format!("https://x/{at}") }))
        .collect();
    let answer = json!({ "files": files });
    let recent = placed(&answer, Place::Recent);
    assert_eq!((recent.files.len(), recent.more), (SHOWN, true));
    let starred = placed(&answer, Place::Starred);
    assert_eq!((starred.files.len(), starred.more), (12, false));
    let paged = placed(
        &json!({ "files": [], "nextPageToken": "next" }),
        Place::Shared,
    );
    assert!(paged.more, "a place with a next page says it holds no more");
}

#[test]
fn each_place_is_searched_and_ordered_as_drive_lists_it() {
    let said: Vec<_> = Place::ALL
        .iter()
        .map(|place| (place.search(), place.order()))
        .collect();
    assert_eq!(
        said,
        [
            (
                "trashed = false and viewedByMeTime > '1970-01-01T00:00:00' and mimeType \
                 != 'application/vnd.google-apps.folder'"
                    .to_owned(),
                "viewedByMeTime desc"
            ),
            (
                "starred = true and trashed = false".to_owned(),
                "recency desc"
            ),
            (
                "sharedWithMe = true and trashed = false".to_owned(),
                "sharedWithMeTime desc"
            ),
        ]
    );
}

#[test]
fn the_shared_drives_are_held_by_name_each_opening_in_drive_none_hidden() {
    let answer = json!({ "drives": [
        { "id": "0Team", "name": "Team", "createdTime": "2026-10-20T08:00:00Z" },
        { "id": "0Old", "name": "Archive", "hidden": true },
        { "id": "0Art", "name": "art" },
        { "id": "../x", "name": "Strange" }
    ]});
    let found = shared_drives(&answer);
    let said: Vec<_> = found
        .files
        .iter()
        .map(|drive| {
            (
                drive.title.as_str(),
                drive.kind,
                drive.link.as_str(),
                drive.created,
            )
        })
        .collect();
    assert_eq!(
        said,
        [
            (
                "art",
                Is::Drive,
                "https://drive.google.com/drive/folders/0Art",
                None
            ),
            (
                "Team",
                Is::Drive,
                "https://drive.google.com/drive/folders/0Team",
                Some(1_792_483_200_000)
            ),
        ]
    );
    assert!(!found.more, "the shared drives say they hold more");
}

#[test]
fn a_file_is_told_the_shell_in_its_words() {
    let told = serde_json::to_value(Document::bare("p1", Is::Other, "x", "https://x")).unwrap();
    let names: Vec<_> = told.as_object().unwrap().keys().cloned().collect();
    for name in ["changedBy", "changedByMe", "sharedWithMe", "sharedBy"] {
        assert!(names.contains(&name.to_owned()), "{names:?}");
    }
    assert_eq!(told["kind"], "other");
}

#[test]
fn each_kind_is_searched_for_only_if_looked_at_and_never_in_the_bin() {
    assert_eq!(
        Kind::Presentation.search(),
        "mimeType = 'application/vnd.google-apps.presentation' and trashed = false \
         and viewedByMeTime > '1970-01-01T00:00:00'"
    );
}
