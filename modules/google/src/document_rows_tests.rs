//! Documents' rows' cases, as tests/drive.qml says Drive.js's and
//! DrivePlaces.js's of the shell's rows today, said in the launcher's own
//! vocabulary.

use super::*;
use crate::rows::marked::{marked, vocabulary, written_in};

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
    let without = |id: &str| {
        let mut lacking = vocabulary();
        lacking["things"]
            .as_array_mut()
            .expect("the vocabulary has no things")
            .retain(|thing| thing["id"] != id);
        Words::read(&lacking)
    };
    assert_eq!(
        without("drive:mine"),
        Err("the vocabulary has no drive:mine".to_owned()),
        "a vocabulary without My Drive read as whole"
    );
    assert_eq!(
        without("drive:fact:sharer"),
        Err("the vocabulary has no drive:fact:sharer".to_owned())
    );
    let mut folderless = vocabulary();
    let file = folderless["things"]
        .as_array_mut()
        .and_then(|things| things.iter_mut().find(|thing| thing["id"] == "drive:file"))
        .expect("the vocabulary has no drive:file");
    file["states"]
        .as_object_mut()
        .expect("drive:file has no states")
        .remove("folder");
    assert_eq!(
        Words::read(&folderless),
        Err("the vocabulary's drive:file has no states.folder.name".to_owned()),
        "a folder would have been named blank"
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
        json!([{"at": 5_000, "as": "opened", "keep": 1}])
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
            "eyebrow": "in Shared with Me \u{b7} Document",
            "title": "Plan",
            "facts": [
                {"name": "Changed", "at": 4_000, "as": "moment"},
                {"name": "Changed By", "said": "Grace Hopper"},
                {"name": "Owner", "said": "Ada Lovelace"},
                {"name": "Shared", "at": 3_000, "as": "moment"},
                {"name": "Shared By", "said": "Grace Hopper"},
                {"name": "ID", "said": "d", "identifies": true}
            ],
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
            .is_some_and(|facts| facts.contains(&json!({"name": "Size", "said": "48 MB"}))),
        "its size wasn't said in decimal units: {drawer}"
    );
    let mut lost = document("x", Is::Other);
    lost.mime = "application/vnd.google-apps.form".into();
    lost.drive = Some("gone".into());
    let drawer = &drawers(&[lost], &[], &words())[0].1;
    assert_eq!(drawer["eyebrow"], "in Shared Drives \u{b7} Form");
    let mut unknown = document("u", Is::Other);
    unknown.mime = "application/vnd.google-apps.unheardof".into();
    let drawer = &drawers(&[unknown], &[], &words())[0].1;
    assert_eq!(
        drawer["eyebrow"], "in My Drive \u{b7} File",
        "a kind of Google's the vocabulary doesn't name is said as a file"
    );
}

/// Documents of every shape a row or drawer words: each kind, untitled,
/// in each place, with every fact, a size in each unit, measured and
/// lasting both short and long, and each of Google's other kinds.
fn spread() -> (Vec<Document>, Vec<Document>) {
    let mut held: Vec<Document> = [
        Is::Document,
        Is::Spreadsheet,
        Is::Presentation,
        Is::Folder,
        Is::Drive,
        Is::File,
        Is::Other,
    ]
    .into_iter()
    .enumerate()
    .map(|(at, kind)| {
        let mut each = document(&format!("k{at}"), kind);
        each.title = String::new();
        each
    })
    .collect();
    for (at, size) in [12, 3_400, 5_600_000, 78_000_000_000]
        .into_iter()
        .enumerate()
    {
        let mut file = document(&format!("s{at}"), Is::File);
        file.title = format!("take{at}.mov");
        file.size = Some(size);
        file.across = Some(1920);
        file.down = Some(1080);
        file.lasts = Some(if at % 2 == 0 { 221_400 } else { 3_723_000 });
        file.changed_by = Some("Grace".into());
        file.changed_by_me = Some(3_500);
        file.created = Some(1_000);
        file.shared_with_me = Some(3_000);
        file.shared_by = Some("Grace".into());
        file.drive = [None, Some("dr".into()), Some("gone".into()), None][at].clone();
        file.about = Some("About".into());
        held.push(file);
    }
    for (at, other) in ["form", "drawing", "map", "site", "script", "vid"]
        .into_iter()
        .enumerate()
    {
        let mut google = document(&format!("g{at}"), Is::Other);
        google.mime = format!("application/vnd.google-apps.{other}");
        held.push(google);
    }
    let mut design = document("dr", Is::Drive);
    design.title = "Design".into();
    (held, vec![design])
}

#[test]
fn every_word_a_document_shows_is_the_vocabularys() {
    let words = Words::read(&marked(&vocabulary())).expect("the marked words didn't read");
    let (held, drives) = spread();
    let mut shown = Vec::new();
    for row in documents(&held, &words) {
        let key = row["key"].as_str().unwrap_or_default().to_owned();
        shown.push((
            format!("{key} name"),
            row["name"].as_str().unwrap_or_default().to_owned(),
        ));
    }
    for (key, drawer) in drawers(&held, &drives, &words) {
        for field in ["eyebrow", "title"] {
            let said = drawer[field].as_str().unwrap_or_default().to_owned();
            shown.push((format!("{key} {field}"), said));
        }
        for fact in drawer["facts"].as_array().into_iter().flatten() {
            for field in ["name", "said"] {
                if let Some(said) = fact[field].as_str() {
                    shown.push((format!("{key} fact {field}"), said.to_owned()));
                }
            }
        }
    }
    assert!(
        shown.len() > 100,
        "only {} words were shown, so the spread reaches too little",
        shown.len()
    );
    // What Drive says of a file itself, which no vocabulary words: its
    // title and ending, who changed, owns and shared it, its id and its
    // owner's words, and the shared drive it's on.
    let own: Vec<String> = held
        .iter()
        .chain(&drives)
        .flat_map(|file| {
            let ending = file.title.rsplit('.').next().map(str::to_uppercase);
            [
                Some(file.title.clone()),
                ending,
                file.owner.clone(),
                file.changed_by.clone(),
                file.shared_by.clone(),
                Some(file.id.clone()),
                file.about.clone(),
            ]
        })
        .flatten()
        .collect();
    let written = written_in(&shown, &own);
    assert!(
        written.is_empty(),
        "words written in document_rows.rs, not the vocabulary:\n{}",
        written.join("\n")
    );
}
