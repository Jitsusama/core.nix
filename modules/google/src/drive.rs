//! The documents Joel looked at lately, for each Google program's Recent:
//! his Docs, Sheets and Slides, as Drive lists them, the one he looked at
//! last first, as many of each as Recent lists. Each kind is asked for on
//! its own, so a kind he seldom opens still has its own few. Only the
//! title, the link, when he looked, when it changed and who owns it are
//! read, never what it says.
//!
//! And Drive's folders, for the launcher's picker and Files: what one
//! folder holds, listed when the shell asks, and a file in it downloaded
//! to open, to hand a program that asked for it, or to keep. Google's own
//! kinds have no bytes of their own, so a copy of one is exported in the
//! kind of file its office program would write.

use jiff::Timestamp;
use serde::Serialize;
use serde_json::Value;

use crate::files::{Failed, How};
use crate::mail;

/// The most of a folder told, which a picker can still list, and well
/// past what a folder Joel keeps by hand holds.
pub const LISTED_MOST: usize = 1000;

/// How many of the downloads asked lately are told, so several asked
/// together are each still told when the shell next reads.
pub const DOWNLOADS_KEPT: usize = 20;

/// Drive's type for a folder.
const FOLDER: &str = "application/vnd.google-apps.folder";
/// Drive's type for a shortcut, which stands for the file it points at.
const SHORTCUT: &str = "application/vnd.google-apps.shortcut";
/// Google's own kinds share this start.
const GOOGLES: &str = "application/vnd.google-apps.";

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

impl Kind {
    /// The kind its copy is exported as, as Drive names it, and the
    /// ending that kind is written with: the office program's own.
    pub fn export(self) -> (&'static str, &'static str) {
        match self {
            Kind::Document => (
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                ".docx",
            ),
            Kind::Spreadsheet => (
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                ".xlsx",
            ),
            Kind::Presentation => (
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                ".pptx",
            ),
        }
    }
}

/// A folder of Drive's, as the shell names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Folder {
    /// My Drive, Joel's own.
    Mine,
    /// What others have shared with him, wherever they keep it.
    Shared,
    /// The shared drives he's on, each listed as a folder.
    Drives,
    /// A folder, or a shared drive, by its id.
    Id(String),
}

impl Folder {
    /// The folder a name says: `root`, `shared`, `drives` or an id.
    pub fn named(name: &str) -> Option<Folder> {
        match name {
            "root" => Some(Folder::Mine),
            "shared" => Some(Folder::Shared),
            "drives" => Some(Folder::Drives),
            id if is_id(id) => Some(Folder::Id(id.to_owned())),
            _ => None,
        }
    }

    /// Drive's search for what it holds, never what's in the bin; none
    /// for the shared drives, which Drive lists apart.
    pub fn search(&self) -> Option<String> {
        match self {
            Folder::Mine => Some("'root' in parents and trashed = false".into()),
            Folder::Shared => Some("sharedWithMe = true and trashed = false".into()),
            Folder::Drives => None,
            Folder::Id(id) => Some(format!("'{id}' in parents and trashed = false")),
        }
    }
}

/// Whether an id is one Drive could have made, so it goes into a search or
/// a path as itself: letters, digits, dashes and underscores.
pub fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// What something in a folder is, as the shell draws it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Is {
    Folder,
    Document,
    Spreadsheet,
    Presentation,
    /// Any other file, with bytes of its own.
    File,
}

impl Is {
    /// What a type of Drive's is, or none for one of Google's own kinds
    /// that's neither a folder nor exported, as a form is.
    fn of(mime: &str) -> Option<Is> {
        Some(match mime {
            FOLDER => Is::Folder,
            mime if mime == Kind::Document.mime() => Is::Document,
            mime if mime == Kind::Spreadsheet.mime() => Is::Spreadsheet,
            mime if mime == Kind::Presentation.mime() => Is::Presentation,
            mime if mime.starts_with(GOOGLES) => return None,
            _ => Is::File,
        })
    }

    /// The Google program's kind, for one of Google's own.
    pub fn google(self) -> Option<Kind> {
        match self {
            Is::Document => Some(Kind::Document),
            Is::Spreadsheet => Some(Kind::Spreadsheet),
            Is::Presentation => Some(Kind::Presentation),
            Is::Folder | Is::File => None,
        }
    }
}

/// Something in a folder, as the shell is told it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Entry {
    /// Drive's id for it, or for what a shortcut points at.
    pub id: String,
    pub name: String,
    pub kind: Is,
    /// Its type, as Drive names it.
    pub mime: String,
    /// How big it is, in bytes, or none for a folder or Google's own kind.
    pub size: Option<u64>,
    /// When anyone last changed it, in milliseconds since the epoch.
    pub changed: Option<i64>,
    /// Who owns it, by name, or none for one on a shared drive.
    pub owner: Option<String>,
    /// Where Drive shows it in the browser.
    pub link: Option<String>,
    /// Whether it's a program or a script, which is never opened here.
    pub runs: bool,
}

/// What a folder holds, as Drive answered: folders first, then by name,
/// at most `LISTED_MOST`, and whether it holds more.
pub fn entries(answer: &Value) -> (Vec<Entry>, bool) {
    let mut found: Vec<Entry> = answer
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(entry)
        .collect();
    found.sort_by_cached_key(|entry| (entry.kind != Is::Folder, entry.name.to_lowercase()));
    let more = found.len() > LISTED_MOST || answer.get("nextPageToken").is_some();
    found.truncate(LISTED_MOST);
    (found, more)
}

/// The shared drives Joel is on, each as a folder, by name.
pub fn drives(answer: &Value) -> (Vec<Entry>, bool) {
    let mut found: Vec<Entry> = answer
        .get("drives")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|drive| {
            let id = drive
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| is_id(id))?;
            Some(Entry {
                id: id.to_owned(),
                name: drive
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                kind: Is::Folder,
                mime: FOLDER.into(),
                size: None,
                changed: None,
                owner: None,
                link: None,
                runs: false,
            })
        })
        .collect();
    found.sort_by_cached_key(|entry| entry.name.to_lowercase());
    (found, answer.get("nextPageToken").is_some())
}

/// One file as Drive tells it, a shortcut standing for what it points at;
/// none for one it gives no id, or one of Google's kinds nothing here
/// can show or copy.
pub fn entry(file: &Value) -> Option<Entry> {
    let text = |name: &str| file.get(name).and_then(Value::as_str);
    let (id, mime) = match text("mimeType")? {
        SHORTCUT => (
            file.pointer("/shortcutDetails/targetId")?.as_str()?,
            file.pointer("/shortcutDetails/targetMimeType")?.as_str()?,
        ),
        mime => (text("id")?, mime),
    };
    let kind = Is::of(mime)?;
    let name = text("name").unwrap_or_default().to_owned();
    Some(Entry {
        id: Some(id).filter(|id| is_id(id))?.to_owned(),
        runs: kind == Is::File && mail::runs(&name, mime),
        name,
        kind,
        mime: mime.to_owned(),
        size: text("size")
            .and_then(|size| size.parse().ok())
            .filter(|_| kind == Is::File),
        changed: text("modifiedTime")
            .and_then(|at| at.parse::<Timestamp>().ok())
            .map(Timestamp::as_millisecond),
        owner: file
            .pointer("/owners/0/displayName")
            .and_then(Value::as_str)
            .map(str::to_owned),
        link: text("webViewLink").map(str::to_owned),
    })
}

/// The folder last asked to be listed, as it stands.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Listed {
    /// The folder, as the shell named it.
    pub folder: String,
    /// When it was asked, in milliseconds since the epoch.
    pub asked: i64,
    pub files: Vec<Entry>,
    /// Whether it holds more than are told.
    pub more: bool,
    pub failed: Option<Unlisted>,
}

/// Why a folder couldn't be listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Unlisted {
    /// Drive has no such folder, or not one Joel may see.
    Gone,
    /// Drive couldn't be reached, or wouldn't answer.
    Unreachable,
}

/// A file asked to be downloaded, as it stands: being fetched, then where
/// it is or why it isn't.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Downloaded {
    /// The file, by Drive's id.
    pub file: String,
    pub how: How,
    /// Where Joel picked to save it, kept from the shell's ask.
    #[serde(skip)]
    pub to: Option<String>,
    /// When it was asked, in milliseconds since the epoch, which tells
    /// one ask from the next of the same file.
    pub asked: i64,
    pub path: Option<String>,
    pub failed: Option<Failed>,
}

/// What's held of Drive's folders: the last listed, and the downloads
/// asked lately, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Drive {
    pub listed: Option<Listed>,
    pub downloads: Vec<Downloaded>,
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
