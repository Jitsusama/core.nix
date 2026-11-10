//! Documents' rows' cases, as tests/drive.qml says Drive.js's and
//! DrivePlaces.js's of the shell's rows today.

use super::*;

fn vocabulary() -> Value {
    json!({"things": [
        {"id": "docs:document", "name": "Document", "glyph": "googleDocument",
         "states": {"untitled": {"name": "A Document Without a Title"}}},
        {"id": "sheets:spreadsheet", "name": "Spreadsheet", "glyph": "googleSpreadsheet",
         "states": {"untitled": {"name": "A Spreadsheet Without a Title"}}},
        {"id": "slides:presentation", "name": "Presentation", "glyph": "googlePresentation",
         "states": {"untitled": {"name": "A Presentation Without a Title"}}},
        {"id": "drive:file", "name": "File", "glyph": "path",
         "states": {"untitled": {"name": "A File Without a Name"},
                    "folder": {"name": "Folder", "glyph": "folder"},
                    "drive": {"name": "Shared Drive", "glyph": "folder"}},
         "said": {"in": "in {place}", "onDrive": "{drive} (shared drive)",
                  "opened": "opened {when}"}},
        {"id": "drive:mine", "name": "My Drive"},
        {"id": "drive:place:shared", "name": "Shared with Me"},
        {"id": "drive:place:drives", "name": "Shared Drives"}
    ]})
}

fn words() -> Words {
    Words::read(&vocabulary()).expect("documents' words didn't read")
}

fn document(id: &str, kind: Is) -> Document {
    Document {
        id: id.into(),
        title: "Plan".into(),
        kind,
        link: format!("https://docs.google.com/document/d/{id}/edit"),
        viewed: Some(5_000),
        changed: Some(4_000),
        owner: Some("Ada Lovelace".into()),
        mime: "application/vnd.google-apps.document".into(),
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

#[test]
fn documents_words_are_read_or_the_lack_is_said() {
    let mut lacking = vocabulary();
    lacking["things"].as_array_mut().unwrap().remove(4);
    assert!(
        Words::read(&lacking).is_err_and(|lack| lack.contains("drive:mine")),
        "a vocabulary without My Drive read as whole"
    );
}

#[test]
fn a_document_is_a_row_keyed_as_the_launcher_keys_it_in_the_order_held() {
    let held = [document("d", Is::Document), document("s", Is::Spreadsheet)];
    let rows = documents(&held, &words());
    let row = &rows[1];
    assert_eq!(row["key"], "row:documents:s");
    assert_eq!(row["row"], "gdoc:s");
    assert_eq!(row["place"], "documents");
    assert_eq!(
        row["kind"], "spreadsheet",
        "the row's kind isn't Google's word"
    );
    assert_eq!(row["order"], 1);
    assert_eq!(row["name"], "Plan");
    assert_eq!(row["glyph"], "googleSpreadsheet");
    assert_eq!(row["drawer"], "drawer:documents:s");
    assert_eq!(
        row["side"],
        json!([{"at": 5_000, "as": "opened", "keep": 1, "stays": true}])
    );
    let link = "https://docs.google.com/document/d/s/edit";
    assert_eq!(
        row["acts"],
        json!([
            {"key": "verb:sheets:open", "ask": {"open": {"link": link}}},
            {"key": "verb:sheets:link", "ask": {"copy": link}}
        ])
    );
}

#[test]
fn one_never_opened_says_nothing_beside_it_and_one_untitled_says_so() {
    let mut bare = document("p", Is::Presentation);
    bare.viewed = None;
    bare.title = String::new();
    let rows = documents(&[bare], &words());
    assert_eq!(rows[0]["side"], json!([]));
    assert_eq!(rows[0]["name"], "A Presentation Without a Title");
}

#[test]
fn its_drawer_says_what_changes_most_first_and_where_it_is() {
    let mut shared = document("d", Is::Document);
    shared.changed_by = Some("Grace Hopper".into());
    shared.shared_with_me = Some(3_000);
    shared.shared_by = Some("Grace Hopper".into());
    shared.about = Some("The engine's plan.".into());
    let drawers = drawers(&[shared], &[], &words());
    assert_eq!(drawers[0].0, "drawer:documents:d");
    assert_eq!(
        drawers[0].1,
        json!({
            "title": "Plan",
            "facts": [
                {"label": "Changed", "at": 4_000, "as": "moment"},
                {"label": "Changed By", "value": "Grace Hopper"},
                {"label": "Owner", "value": "Ada Lovelace"},
                {"label": "Shared", "at": 3_000, "as": "moment"},
                {"label": "Shared By", "value": "Grace Hopper"},
                {"label": "ID", "value": "d", "identifies": true}
            ],
            "eyebrow": "in Shared with Me \u{b7} Document",
            "eyebrowItems": [
                {"said": "in Shared with Me", "keep": 2},
                {"said": "Document", "keep": 1}
            ],
            "eyebrowNames": true,
            "body": "The engine's plan."
        })
    );
}

#[test]
fn a_file_is_said_by_its_ending_measure_and_length_on_its_shared_drive() {
    let mut video = document("v", Is::File);
    video.title = "launch.mp4".into();
    video.size = Some(48_200_000);
    video.across = Some(1920);
    video.down = Some(1080);
    video.lasts = Some(221_400);
    video.drive = Some("dr".into());
    let mut design = document("dr", Is::Drive);
    design.title = "Design".into();
    let drawer = &drawers(&[video], &[design], &words())[0].1;
    assert_eq!(
        drawer["eyebrow"],
        "in Design (shared drive) \u{b7} MP4 \u{b7} 1920 \u{d7} 1080 \u{b7} 3:41"
    );
    assert!(
        drawer["facts"]
            .as_array()
            .is_some_and(|facts| facts.contains(&json!({"label": "Size", "value": "48 MB"}))),
        "its size wasn't said in decimal units: {drawer}"
    );
    let mut lost = document("x", Is::Other);
    lost.mime = "application/vnd.google-apps.form".into();
    lost.drive = Some("gone".into());
    let drawer = &drawers(&[lost], &[], &words())[0].1;
    assert_eq!(drawer["eyebrow"], "in Shared Drives \u{b7} Form");
}
