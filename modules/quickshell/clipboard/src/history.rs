//! The copies kept: what each holds, a picture's file, size and type,
//! the history newest first with the room its pictures have, what the
//! clipboard holds now, and what's served of them to the shell.

use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::rows::Words;
use crate::{SECRET, TEXT, now, rows};
use daemon_serve::serve::Shared;
use ready_rows::kept::Kept;
use serde_json::{Value, json};

/// A picture kept: its number, its type, the file holding it, its size in
/// bytes, what tells it from another, and its width and height when its
/// type says them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Picture {
    pub(crate) id: u64,
    pub(crate) kind: &'static str,
    pub(crate) path: PathBuf,
    pub(crate) bytes: usize,
    pub(crate) hash: u64,
    pub(crate) size: Option<(u32, u32)>,
}

/// What a copy holds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Content {
    Text(String),
    Picture(Picture),
}

impl Content {
    /// Whether two copies are the same one made again.
    fn same(&self, other: &Content) -> bool {
        match (self, other) {
            (Content::Text(a), Content::Text(b)) => a == b,
            (Content::Picture(a), Content::Picture(b)) => a.id == b.id,
            _ => false,
        }
    }

    /// The types it's offered as.
    pub(crate) fn types(&self) -> Vec<&'static str> {
        match self {
            Content::Text(_) => TEXT.to_vec(),
            Content::Picture(picture) => vec![picture.kind],
        }
    }
}

/// What was read from an offer, before it's kept.
#[derive(Debug)]
pub(crate) enum Heard {
    Text(String),
    Picture { kind: &'static str, bytes: Vec<u8> },
}

/// One kept copy, and its number.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Copy {
    pub(crate) at: u128,
    pub(crate) id: u64,
    pub(crate) content: Content,
}

impl Copy {
    /// Its row in the launcher's Copies.
    pub(crate) fn row(&self, words: &Words) -> Value {
        match &self.content {
            Content::Text(text) => rows::text(self.id, self.at, text, words),
            Content::Picture(picture) => rows::picture(
                &rows::Picture {
                    id: picture.id,
                    at: self.at,
                    kind: picture.kind,
                    path: &picture.path.to_string_lossy(),
                    bytes: picture.bytes,
                    size: picture.size,
                },
                words,
            ),
        }
    }
}

fn be16(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?).into())
}

fn le16(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?).into())
}

fn le24(bytes: &[u8], at: usize) -> Option<u32> {
    let three = bytes.get(at..at + 3)?;
    Some(u32::from_le_bytes([three[0], three[1], three[2], 0]))
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// A picture's width and height, read from its header, or none when its
/// header doesn't say them where its type puts them.
pub(crate) fn dimensions(kind: &str, bytes: &[u8]) -> Option<(u32, u32)> {
    match kind {
        "image/png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.get(12..16)? == b"IHDR" => {
            Some((be32(bytes, 16)?, be32(bytes, 20)?))
        }
        "image/gif" if bytes.starts_with(b"GIF8") => Some((le16(bytes, 6)?, le16(bytes, 8)?)),
        "image/jpeg" if bytes.starts_with(&[0xff, 0xd8]) => jpeg_dimensions(bytes),
        "image/webp" if bytes.starts_with(b"RIFF") && bytes.get(8..12)? == b"WEBP" => {
            match bytes.get(12..16)? {
                b"VP8X" => Some((1 + le24(bytes, 24)?, 1 + le24(bytes, 27)?)),
                b"VP8L" if *bytes.get(20)? == 0x2f => {
                    let bits = le32(bytes, 21)?;
                    Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
                }
                b"VP8 " if bytes.get(23..26)? == [0x9d, 0x01, 0x2a] => {
                    Some((le16(bytes, 26)? & 0x3fff, le16(bytes, 28)? & 0x3fff))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// A JPEG's width and height, from its first start-of-frame segment.
fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xff {
            return None;
        }
        let marker = *bytes.get(at + 1)?;
        match marker {
            // Padding before a marker.
            0xff => at += 1,
            // Markers that stand alone, with no length.
            0x01 | 0xd0..=0xd8 => at += 2,
            // Every start of frame but the Huffman, arithmetic and
            // restart tables that share their range.
            0xc0..=0xcf if !matches!(marker, 0xc4 | 0xc8 | 0xcc) => {
                return Some((be16(bytes, at + 7)?, be16(bytes, at + 5)?));
            }
            _ => at += 2 + usize::try_from(be16(bytes, at + 2)?).ok()?,
        }
    }
}

/// What a picture's file ends in, by its type.
fn extension(kind: &str) -> &'static str {
    match kind {
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "png",
    }
}

fn hashed(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Writes a file only this account can read, refusing one already there.
fn written(path: &Path, bytes: &[u8]) -> io::Result<()> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?
        .write_all(bytes)
}

/// The copies kept, newest first, what the clipboard holds now when it's a
/// copy kept, where pictures go and how much room they have, what the
/// clipboard holds as the shell is told it, and how many times what's
/// served has changed.
pub(crate) struct History {
    pub(crate) copies: VecDeque<Copy>,
    keep: usize,
    pub(crate) current: Option<Content>,
    pictures: Option<PathBuf>,
    room: usize,
    next: u64,
    pub(crate) now: Value,
    pub(crate) changes: u64,
}

/// What the clipboard holds once a program copies: nothing of a secret,
/// and text only where it was read as text.
pub(crate) fn offered(types: &[String], heard: Option<&Heard>) -> Value {
    if types.iter().any(|kind| kind == SECRET) {
        return rows::now::<&str>(&[], "");
    }
    let text = match heard {
        Some(Heard::Text(text)) => text.as_str(),
        _ => "",
    };
    rows::now(types, text)
}

impl History {
    pub(crate) fn new(keep: usize, pictures: Option<PathBuf>, room: usize) -> History {
        History {
            copies: VecDeque::new(),
            keep,
            current: None,
            pictures,
            room,
            next: 1,
            now: rows::now::<&str>(&[], ""),
            changes: 0,
        }
    }

    /// What the clipboard holds now.
    pub(crate) fn holds(&mut self, now: Value) {
        if now != self.now {
            self.now = now;
            self.changes += 1;
        }
    }

    /// What the clipboard holds once it offers the copy this offers, or
    /// nothing while it offers none.
    pub(crate) fn holds_current(&mut self) {
        let now = match &self.current {
            Some(Content::Text(text)) => rows::now(&TEXT, text),
            Some(content @ Content::Picture(_)) => rows::now(&content.types(), ""),
            None => rows::now::<&str>(&[], ""),
        };
        self.holds(now);
    }

    /// Everything it serves: a row for each copy, newest first, and beside
    /// them how many there are and what the clipboard holds.
    pub(crate) fn records(&self, words: &Words) -> (Vec<Value>, Vec<Value>) {
        let rows = self.copies.iter().map(|copy| copy.row(words)).collect();
        let beside = vec![rows::summary(self.copies.len(), words), self.now.clone()];
        (rows, beside)
    }

    /// Keeps a copy, unless it's blank text. One copied again moves to the
    /// front rather than being kept twice. The oldest go once there are more
    /// than it keeps, and the oldest pictures once they take more room than
    /// they have, never the one just kept. Answers whether it was kept.
    pub(crate) fn add(&mut self, copy: Copy) -> bool {
        if let Content::Text(text) = &copy.content
            && text.trim().is_empty()
        {
            return false;
        }
        self.copies.retain(|kept| !kept.content.same(&copy.content));
        self.copies.push_front(copy);
        let mut gone = Vec::new();
        while self.copies.len() > self.keep {
            gone.extend(self.copies.pop_back());
        }
        while self.picture_bytes() > self.room {
            let Some(oldest) = self
                .copies
                .iter()
                .skip(1)
                .rposition(|kept| matches!(kept.content, Content::Picture(_)))
            else {
                break;
            };
            gone.extend(self.copies.remove(oldest + 1));
        }
        for copy in gone {
            discard(&copy.content);
        }
        self.changes += 1;
        true
    }

    fn picture_bytes(&self) -> usize {
        self.copies
            .iter()
            .map(|kept| match &kept.content {
                Content::Picture(picture) => picture.bytes,
                Content::Text(_) => 0,
            })
            .sum()
    }

    /// A picture as it's kept: the one kept already when it's the same
    /// picture, or else a new one in its own file. None when it's empty, as
    /// one is whose program closed before handing it over, when pictures
    /// aren't kept, or when its file can't be written.
    fn picture(&mut self, kind: &'static str, bytes: &[u8]) -> Option<Content> {
        if bytes.is_empty() {
            return None;
        }
        let hash = hashed(bytes);
        let known = self.copies.iter().find_map(|kept| match &kept.content {
            Content::Picture(picture) if picture.hash == hash && picture.bytes == bytes.len() => {
                Some(picture.clone())
            }
            _ => None,
        });
        if let Some(picture) = known {
            return Some(Content::Picture(picture));
        }
        let id = self.next;
        let path = self
            .pictures
            .as_ref()?
            .join(format!("{id}.{}", extension(kind)));
        written(&path, bytes).ok()?;
        self.next += 1;
        Some(Content::Picture(Picture {
            id,
            kind,
            path,
            bytes: bytes.len(),
            hash,
            size: dimensions(kind, bytes),
        }))
    }

    /// A copy's number: a picture's own, the number text kept already had
    /// when it's copied again, or else the next.
    fn number(&mut self, content: &Content) -> u64 {
        let known = match content {
            Content::Picture(picture) => Some(picture.id),
            Content::Text(_) => self
                .copies
                .iter()
                .find(|kept| kept.content.same(content))
                .map(|kept| kept.id),
        };
        known.unwrap_or_else(|| {
            self.next += 1;
            self.next - 1
        })
    }

    /// What the clipboard now holds: a copy, kept if it can be, or for one
    /// that can't, nothing to give back once its program lets go. Answers
    /// the copy when it was kept.
    pub(crate) fn copied(&mut self, heard: Option<Heard>) -> Option<Copy> {
        self.current = None;
        if self.keep == 0 {
            return None;
        }
        let content = match heard? {
            Heard::Text(text) => Content::Text(text),
            Heard::Picture { kind, bytes } => self.picture(kind, &bytes)?,
        };
        let id = self.number(&content);
        let copy = Copy {
            at: now(),
            id,
            content,
        };
        if !self.add(copy.clone()) {
            return None;
        }
        self.current = Some(copy.content.clone());
        Some(copy)
    }

    /// Copy N, moved to the front as what the clipboard holds, as the shell
    /// asks when it's chosen.
    pub(crate) fn choose(&mut self, id: u64) -> Option<Copy> {
        let at = self.copies.iter().position(|kept| kept.id == id)?;
        let mut copy = self.copies.remove(at)?;
        copy.at = now();
        self.copies.push_front(copy.clone());
        self.current = Some(copy.content.clone());
        self.changes += 1;
        Some(copy)
    }

    /// Forgets every copy, and what the clipboard holds with them, which is
    /// nothing now, since it gives nothing.
    pub(crate) fn forget(&mut self) {
        for copy in self.copies.drain(..) {
            discard(&copy.content);
        }
        self.current = None;
        self.changes += 1;
        self.holds_current();
    }
}

/// What's served to the shell: the interface's watches, the rows as they
/// were last sent, the words they're said in, and how many changes of the
/// history they hold.
pub(crate) struct Served {
    pub(crate) shared: Arc<Shared>,
    pub(crate) kept: Kept,
    pub(crate) words: Words,
    // Emoji's rows, made once, since the list doesn't change.
    pub(crate) emoji: Vec<Value>,
    pub(crate) changes: Option<u64>,
}

impl Served {
    /// Sends the shell what changed in the history since it was last sent,
    /// unless nothing has.
    pub(crate) fn send(&mut self, history: &History) {
        if self.changes == Some(history.changes) {
            return;
        }
        self.changes = Some(history.changes);
        let (rows, beside) = history.records(&self.words);
        let moved = self.kept.take(rows, beside);
        if !moved.changed.is_empty() || !moved.gone.is_empty() {
            let records = self.kept.all().chain(&self.emoji).cloned().collect();
            self.shared.change(records, json!({}));
        }
    }
}

/// Removes a picture's file once it's no longer kept.
fn discard(content: &Content) {
    if let Content::Picture(picture) = content {
        let _ = fs::remove_file(&picture.path);
    }
}

/// Makes the pictures' directory, only this account's, and empties it of
/// any an earlier run left.
pub(crate) fn prepared(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(dir)? {
        fs::remove_file(entry?.path())?;
    }
    Ok(())
}
