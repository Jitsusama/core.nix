//! The documents Joel looked at lately, for each Google program's Recent:
//! his Docs, Sheets and Slides, as Drive lists them, the one he looked at
//! last first, as many of each as Recent lists. Each kind is asked for on
//! its own, so a kind he seldom opens still has its own few. What Drive
//! says of each file is read, never what it says.
//!
//! And Drive's own places, for Programs › Google Drive: its Recent, the
//! files of every kind he looked at last, then Starred, Shared with Me and
//! the shared drives he's on. Each file in them is the same `Document` a
//! Docs Recent tells, so every surface draws a Drive file by one row.
//!
//! Of what Drive's API says of a file (`FILE_FIELDS`), these are read:
//! its id, name, type, size, link, when Joel last opened it, when anyone
//! and when he last changed it, when it was made, when it was shared with
//! him and by whom, who owns it, who changed it last, whether it's starred
//! and shared, the shared drive it's on, its description, and a picture's
//! or a video's size and a video's length. These are left out, each for
//! its reason:
//!
//! - `md5Checksum`, `sha1Checksum`, `sha256Checksum`, `headRevisionId`,
//!   `version` and `resourceKey`: Drive's own bookkeeping, which nothing
//!   Joel does here acts on or wonders about; `version` counts Drive's
//!   changes of any kind, never his revisions.
//! - `quotaBytesUsed`: the storage it takes, which for a file is its size
//!   again, said already, and for Google's own kinds Drive's accounting
//!   of revisions rather than anything about the file.
//! - `thumbnailLink`, `iconLink`, `hasThumbnail` and `thumbnailVersion`:
//!   pictures, which the theme's marks stand for here; a thumbnail's link
//!   lasts only hours and would need the token to fetch.
//! - `folderColorRgb`: a colour of Drive's choosing, and the shell names
//!   only the theme's.
//! - `parents`: naming each folder takes another ask a file; the shared
//!   drive it's on is named instead, from the drives already read.
//! - `permissions`, `permissionIds`, `capabilities`, `contentRestrictions`,
//!   `downloadRestrictions`, `copyRequiresWriterPermission`,
//!   `writersCanShare`, `viewersCanCopyContent`, `hasAugmentedPermissions`,
//!   `inheritedPermissionsDisabled` and `linkShareMetadata`: who may do
//!   what with it, which nothing here changes; Drive's own Share, one
//!   Open away, says it whole.
//! - `ownedByMe` and `viewedByMe`: the owner's name and when he opened it
//!   say both.
//! - `trashed`, `explicitlyTrashed`, `trashedTime` and `trashingUser`:
//!   nothing in the bin is ever asked for.
//! - `exportLinks` and `webContentLink`: other ways to fetch it; Copy the
//!   Link copies `webViewLink`, the one that opens it.
//! - `fileExtension`, `fullFileExtension` and `originalFilename`: its name
//!   says its ending, and an upload renamed is known by its new name.
//! - `properties`, `appProperties` and `isAppAuthorized`: what other
//!   programs keep on it, private to them.
//! - `spaces`, `kind`, `teamDriveId` and `labelInfo`: always the same,
//!   replaced by `driveId`, or labels only a Workspace's admin defines.
//! - The rest of `imageMediaMetadata` and `videoMediaMetadata`, a camera's
//!   make, exposure and where a photo was taken among them: a photo
//!   library's to show, which Drive's own details leave out too.
//!
//! Of a shared drive, its name and when it was made are read, and whether
//! Joel hid it, which leaves it out as Drive leaves it out; its colour,
//! background picture, theme, capabilities and restrictions are left out
//! for the reasons a file's are.
//!
//! And Drive's folders, for the launcher's picker and Files: what one
//! folder holds, listed when the shell asks, and a file in it downloaded
//! to open, to hand a program that asked for it, or to keep. Google's own
//! kinds have no bytes of their own, so a copy of one is exported in the
//! kind of file its office program would write.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::files::{Failed, How};
use crate::mail;

/// The most of a folder told, which a picker can still list, and well
/// past what a folder Joel keeps by hand holds.
pub const LISTED_MOST: usize = 1000;

/// How many of the downloads asked lately are told, so several asked
/// together are each still told when the shell next reads.
pub const DOWNLOADS_KEPT: usize = 20;

/// How many of the folders asked lately are told, each its last listing,
/// so two asked together, as a picker stepping in asks them, are each
/// still told when the shell next reads; few, since each may hold a
/// thousand files and the whole state is written on every change.
pub const LISTINGS_KEPT: usize = 4;

/// How many of the saves sent up lately are told, as downloads are.
pub const UPLOADS_KEPT: usize = 20;

/// Drive's type for a folder.
const FOLDER: &str = "application/vnd.google-apps.folder";
/// Drive's type for a shortcut, which stands for the file it points at.
const SHORTCUT: &str = "application/vnd.google-apps.shortcut";
/// Google's own kinds share this start.
const GOOGLES: &str = "application/vnd.google-apps.";

/// How many of each kind the shell is told of, at most: as many as Recent
/// lists, so 1 to 9 pick one.
pub const SHOWN: usize = 9;

/// The most of Starred, Shared with Me or the shared drives told: one of
/// Drive's pages, past what a list in the launcher is gone through by
/// hand, and small enough to send the shell whenever it changes. A place
/// holding more says so.
pub const PLACED_MOST: usize = 100;

/// What Drive is asked of each file the launcher shows: everything this
/// file's opening comment names as read.
pub const FILE_FIELDS: &str = "id,name,mimeType,size,webViewLink,viewedByMeTime,modifiedTime,\
    modifiedByMeTime,createdTime,sharedWithMeTime,starred,shared,driveId,description,\
    owners(displayName),lastModifyingUser(displayName),sharingUser(displayName),\
    imageMediaMetadata(width,height),videoMediaMetadata(width,height,durationMillis),\
    shortcutDetails";

/// What a document is, by the Google program that opens it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// One Drive file, as the shell is told it: a document of Docs, Sheets or
/// Slides, or, in Drive's own places, a file of any kind or a shared drive.
/// Times are in milliseconds since the epoch, and each figure Drive didn't
/// say is none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    /// Drive's id for it, or for what a shortcut points at, which keys its
    /// row.
    pub id: String,
    /// Its title, or nothing for one untitled.
    pub title: String,
    pub kind: Is,
    /// Where it opens.
    pub link: String,
    /// When Joel last looked at it.
    pub viewed: Option<i64>,
    /// When anyone last changed it.
    pub changed: Option<i64>,
    /// Who owns it, by name, or none for one on a shared drive.
    pub owner: Option<String>,
    /// Its type, as Drive names it, or nothing for a shared drive.
    pub mime: String,
    /// How big it is, in bytes, for a file with bytes of its own.
    pub size: Option<u64>,
    /// When it was made.
    pub created: Option<i64>,
    /// Who changed it last, by name.
    pub changed_by: Option<String>,
    /// When Joel himself last changed it.
    pub changed_by_me: Option<i64>,
    /// When it was shared with Joel, for one someone else shared.
    pub shared_with_me: Option<i64>,
    /// Who shared it with him, by name.
    pub shared_by: Option<String>,
    /// Whether anyone but its owner may see it.
    pub shared: bool,
    /// Whether Joel starred it.
    pub starred: bool,
    /// The shared drive it's on, by its id.
    pub drive: Option<String>,
    /// What its owner wrote of it, or none.
    pub about: Option<String>,
    /// A picture's or a video's width and height, in pixels.
    pub across: Option<u64>,
    pub down: Option<u64>,
    /// How long a video runs, in milliseconds.
    pub lasts: Option<u64>,
}

/// The documents Drive answered a kind's search with, in its order, at
/// most `SHOWN`, leaving out any it doesn't say how to open.
pub fn documents(answer: &Value, kind: Kind) -> Vec<Document> {
    files(answer)
        .filter_map(|file| read(file, Some(kind.into())))
        .take(SHOWN)
        .collect()
}

fn files(answer: &Value) -> impl Iterator<Item = &Value> {
    answer
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

/// One file as Drive tells it, a shortcut standing for what it points at,
/// as `kind`, or as its type says; none for one it gives no id or link.
fn read(file: &Value, kind: Option<Is>) -> Option<Document> {
    let text = |name: &str| file.get(name).and_then(Value::as_str);
    let at = |name: &str| {
        text(name)
            .and_then(|written| written.parse::<Timestamp>().ok())
            .map(Timestamp::as_millisecond)
    };
    let who = |pointer: &str| {
        file.pointer(pointer)
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    // Drive writes its 64-bit counts as text, and a picture's own as numbers.
    let count = |pointer: &str| {
        file.pointer(pointer).and_then(|said| {
            said.as_u64()
                .or_else(|| said.as_str().and_then(|text| text.parse().ok()))
        })
    };
    let (id, mime) = match text("mimeType") {
        Some(SHORTCUT) => (
            file.pointer("/shortcutDetails/targetId")?.as_str()?,
            file.pointer("/shortcutDetails/targetMimeType")?
                .as_str()?
                .to_owned(),
        ),
        said => (
            text("id")?,
            said.map(str::to_owned)
                .or_else(|| kind.and_then(Is::google).map(|kind| kind.mime().to_owned()))
                .unwrap_or_default(),
        ),
    };
    let kind = kind.unwrap_or_else(|| Is::shown(&mime));
    let media = |field: &str| {
        count(&format!("/imageMediaMetadata/{field}"))
            .or_else(|| count(&format!("/videoMediaMetadata/{field}")))
    };
    Some(Document {
        id: id.to_owned(),
        title: text("name").unwrap_or_default().to_owned(),
        kind,
        link: text("webViewLink")?.to_owned(),
        viewed: at("viewedByMeTime"),
        changed: at("modifiedTime"),
        owner: who("/owners/0/displayName"),
        size: count("/size").filter(|_| kind == Is::File),
        created: at("createdTime"),
        changed_by: who("/lastModifyingUser/displayName"),
        changed_by_me: at("modifiedByMeTime"),
        shared_with_me: at("sharedWithMeTime"),
        shared_by: who("/sharingUser/displayName"),
        shared: file.get("shared").and_then(Value::as_bool) == Some(true),
        starred: file.get("starred").and_then(Value::as_bool) == Some(true),
        drive: who("/driveId"),
        about: text("description")
            .map(str::trim)
            .filter(|about| !about.is_empty())
            .map(str::to_owned),
        across: media("width"),
        down: media("height"),
        lasts: count("/videoMediaMetadata/durationMillis"),
        mime,
    })
}

/// One of Drive's own places, as the launcher holds them under Google
/// Drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// The files of every kind Joel looked at last, as Drive's own Recent.
    Recent,
    /// What he starred.
    Starred,
    /// What others shared with him, wherever they keep it.
    Shared,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Recent, Place::Starred, Place::Shared];

    /// Drive's search for what it holds, never what's in the bin. Recent
    /// leaves folders out, as Drive's own does, and holds only what he's
    /// looked at, which a file never opened isn't.
    pub fn search(self) -> String {
        match self {
            Place::Recent => format!(
                "trashed = false and viewedByMeTime > '1970-01-01T00:00:00' \
                 and mimeType != '{FOLDER}'"
            ),
            Place::Starred => "starred = true and trashed = false".into(),
            Place::Shared => "sharedWithMe = true and trashed = false".into(),
        }
    }

    /// The order it's listed in: Recent the one opened last first, Starred
    /// the one touched last first, by Drive's `recency`, the latest of its
    /// times, and Shared with Me the one shared last first, as Drive's own
    /// lists each.
    pub fn order(self) -> &'static str {
        match self {
            Place::Recent => "viewedByMeTime desc",
            Place::Starred => "recency desc",
            Place::Shared => "sharedWithMeTime desc",
        }
    }

    /// How many it tells at most: Recent as many as every Recent lists.
    pub fn most(self) -> usize {
        match self {
            Place::Recent => SHOWN,
            Place::Starred | Place::Shared => PLACED_MOST,
        }
    }
}

/// What one of Drive's places holds, in its order, and whether it holds
/// more than are told.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Placed {
    pub files: Vec<Document>,
    pub more: bool,
}

/// What Drive answered a place's search with, in its order, at most
/// `place.most()`, leaving out any it doesn't say how to open.
pub fn placed(answer: &Value, place: Place) -> Placed {
    let mut found: Vec<Document> = files(answer).filter_map(|file| read(file, None)).collect();
    let more = found.len() > place.most() || answer.get("nextPageToken").is_some();
    found.truncate(place.most());
    Placed { files: found, more }
}

/// The shared drives Joel is on and hasn't hidden, each as a Drive file
/// of its own kind, by name, opening in Drive as its own page does.
pub fn shared_drives(answer: &Value) -> Placed {
    let mut found: Vec<Document> = answer
        .get("drives")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|drive| drive.get("hidden").and_then(Value::as_bool) != Some(true))
        .filter_map(|drive| {
            let id = drive
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| is_id(id))?;
            let title = drive
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some(Document {
                id: id.to_owned(),
                title: title.to_owned(),
                kind: Is::Drive,
                link: format!("https://drive.google.com/drive/folders/{id}"),
                viewed: None,
                changed: None,
                owner: None,
                mime: String::new(),
                size: None,
                created: drive
                    .get("createdTime")
                    .and_then(Value::as_str)
                    .and_then(|at| at.parse::<Timestamp>().ok())
                    .map(Timestamp::as_millisecond),
                changed_by: None,
                changed_by_me: None,
                shared_with_me: None,
                shared_by: None,
                shared: true,
                starred: false,
                drive: Some(id.to_owned()),
                about: None,
                across: None,
                down: None,
                lasts: None,
            })
        })
        .collect();
    found.sort_by_cached_key(|drive| drive.title.to_lowercase());
    let more = found.len() > PLACED_MOST || answer.get("nextPageToken").is_some();
    found.truncate(PLACED_MOST);
    Placed { files: found, more }
}

/// Drive's own places as the launcher holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Places {
    pub recent: Vec<Document>,
    pub starred: Placed,
    pub shared: Placed,
    pub drives: Placed,
}

impl Places {
    /// Takes what Drive answered for `place`.
    pub fn hold(&mut self, place: Place, answer: &Value) {
        let found = placed(answer, place);
        match place {
            Place::Recent => self.recent = found.files,
            Place::Starred => self.starred = found,
            Place::Shared => self.shared = found,
        }
    }
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
    /// One of Google's other kinds, a form, a drawing or a site, which
    /// opens only in the browser, so only Drive's places list it.
    Other,
    /// A shared drive, as Drive's places list it.
    Drive,
}

impl From<Kind> for Is {
    fn from(kind: Kind) -> Is {
        match kind {
            Kind::Document => Is::Document,
            Kind::Spreadsheet => Is::Spreadsheet,
            Kind::Presentation => Is::Presentation,
        }
    }
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

    /// What a type is in Drive's places, which list Google's other kinds
    /// too, since each opens by its link.
    fn shown(mime: &str) -> Is {
        Is::of(mime).unwrap_or(Is::Other)
    }

    /// The Google program's kind, for one of Google's own.
    pub fn google(self) -> Option<Kind> {
        match self {
            Is::Document => Some(Kind::Document),
            Is::Spreadsheet => Some(Kind::Spreadsheet),
            Is::Presentation => Some(Kind::Presentation),
            Is::Folder | Is::File | Is::Other | Is::Drive => None,
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

/// A folder asked to be listed, as it stands.
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

/// A file Joel saved into Drive, as it stands: going up, then where it
/// is in Drive or why it isn't.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Uploaded {
    /// Where the picker staged it.
    pub path: String,
    /// The folder it goes into, as the shell names it.
    pub folder: String,
    pub name: String,
    /// When it was sent, in milliseconds since the epoch, which tells one
    /// save from the next of the same file.
    pub asked: i64,
    /// Drive's id for it, once it's up.
    pub id: Option<String>,
    /// Where Drive shows it in the browser, once it's up.
    pub link: Option<String>,
    pub failed: Option<Unsaved>,
    /// The app id of the program that saved it, as the picker wrote it
    /// beside the file, so a notice can take Joel back to it; none for a
    /// save the shell made itself.
    pub from: Option<String>,
}

/// Why a save couldn't go into Drive; its staged copy stays either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Unsaved {
    /// Nobody is signed in, or Google refused the sign-in: signing in
    /// again from pi mends it.
    SignedOut,
    /// Drive couldn't be reached, or wouldn't answer; trying again may
    /// work.
    Unreachable,
    /// Drive has no such folder, or not one Joel may add to.
    Gone,
    /// Joel's sign-in doesn't let this program replace that file: it
    /// grants only the files it made, and this one it didn't.
    ReplaceNotGranted,
    /// The staged copy couldn't be read.
    Unread,
}

/// How much the copies of Drive's files kept to open or hand on take
/// together, `held`, and how much they may, `most`, before those handed
/// out longest ago are let go, in bytes, as Google Drive's drawer in
/// Programs says it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub held: u64,
    pub most: u64,
}

/// What's held of Drive's folders: the folders, the downloads and the
/// saves asked lately, each newest first, Drive's own places, and how
/// much its copies here take.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Drive {
    pub listings: Vec<Listed>,
    pub downloads: Vec<Downloaded>,
    pub uploads: Vec<Uploaded>,
    pub places: Places,
    pub kept: Kept,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A document as Drive says one with nothing but its id and link.
    fn bare(id: &str, kind: Is, mime: &str, link: &str) -> Document {
        Document {
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
                ..bare(
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
        assert!(!found.more);
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
        assert!(paged.more);
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
        assert!(!found.more);
    }

    #[test]
    fn a_file_is_told_the_shell_in_its_words() {
        let told = serde_json::to_value(bare("p1", Is::Other, "x", "https://x")).unwrap();
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
}
