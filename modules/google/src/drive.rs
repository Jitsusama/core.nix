//! The documents Joel looked at lately, for each Google program's Recent:
//! his Docs, Sheets and Slides, as Drive lists them, the one he looked at
//! last first, as many of each as Recent lists. Each kind is asked for on
//! its own, so a kind he seldom opens still has its own few. Only the
//! title, the link, when he looked, when it changed and who owns it are
//! read, never what it says.

use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

/// How many of each kind the shell is told of, at most: as many as Recent
/// lists, so 1 to 9 pick one.
pub const SHOWN: usize = 9;

/// What a document is, by the Google program that opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// Google Docs.
    Document,
    /// Google Sheets.
    Spreadsheet,
    /// Google Slides.
    Presentation,
}

impl Kind {
    pub const ALL: [Kind; 3] = [Kind::Document, Kind::Spreadsheet, Kind::Presentation];

    /// Its type, as Drive names it.
    pub fn mime(self) -> &'static str {
        match self {
            Kind::Document => "application/vnd.google-apps.document",
            Kind::Spreadsheet => "application/vnd.google-apps.spreadsheet",
            Kind::Presentation => "application/vnd.google-apps.presentation",
        }
    }

    /// Drive's search for those of this kind Joel has looked at, never one
    /// in the bin: one never opened has no time he looked, so it isn't
    /// recent.
    pub fn search(self) -> String {
        format!(
            "mimeType = '{}' and trashed = false and viewedByMeTime > '1970-01-01T00:00:00'",
            self.mime()
        )
    }
}

/// One document, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Document {
    /// Drive's id for it, which keys its row.
    pub id: String,
    /// Its title, or nothing for one untitled.
    pub title: String,
    pub kind: Kind,
    /// Where it opens.
    pub link: String,
    /// When Joel last looked at it, in milliseconds since the epoch.
    pub viewed: Option<i64>,
    /// When anyone last changed it.
    pub changed: Option<i64>,
    /// Who owns it, by name, or none for one on a shared drive.
    pub owner: Option<String>,
}

/// The documents Drive answered a kind's search with, in its order, at
/// most `SHOWN`, leaving out any it doesn't say how to open.
pub fn documents(answer: &Value, kind: Kind) -> Vec<Document> {
    answer
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|file| read(file, kind))
        .take(SHOWN)
        .collect()
}

fn read(file: &Value, kind: Kind) -> Option<Document> {
    let text = |name: &str| file.get(name).and_then(Value::as_str);
    let at = |name: &str| {
        text(name)
            .and_then(|written| written.parse::<Timestamp>().ok())
            .map(Timestamp::as_millisecond)
    };
    Some(Document {
        id: text("id")?.to_owned(),
        title: text("name").unwrap_or_default().to_owned(),
        kind,
        link: text("webViewLink")?.to_owned(),
        viewed: at("viewedByMeTime"),
        changed: at("modifiedTime"),
        owner: file
            .pointer("/owners/0/displayName")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_document_says_its_title_link_times_and_owner() {
        let answer = json!({ "files": [{
            "id": "d1",
            "name": "Quarterly Plan",
            "mimeType": Kind::Document.mime(),
            "webViewLink": "https://docs.google.com/document/d/d1/edit",
            "viewedByMeTime": "2026-10-28T14:00:00.000Z",
            "modifiedTime": "2026-10-27T09:30:00Z",
            "owners": [{ "displayName": "Ada Lovelace" }]
        }]});
        assert_eq!(
            documents(&answer, Kind::Document),
            vec![Document {
                id: "d1".into(),
                title: "Quarterly Plan".into(),
                kind: Kind::Document,
                link: "https://docs.google.com/document/d/d1/edit".into(),
                viewed: Some(1_793_196_000_000),
                changed: Some(1_793_093_400_000),
                owner: Some("Ada Lovelace".into()),
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
    fn each_kind_is_searched_for_only_if_looked_at_and_never_in_the_bin() {
        assert_eq!(
            Kind::Presentation.search(),
            "mimeType = 'application/vnd.google-apps.presentation' and trashed = false \
             and viewedByMeTime > '1970-01-01T00:00:00'"
        );
    }
}
