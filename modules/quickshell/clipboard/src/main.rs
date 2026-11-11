//! The clipboard's history, for the launcher.
//!
//! A Wayland program only sees the clipboard while it holds the keys, so the
//! shell can't keep a history by itself. This one program holds the
//! compositor's data-control connection for the whole session instead, reads
//! each new copy, and keeps the latest. Nothing is started per copy.
//!
//! It serves the shell as every daemon of the launcher does, over the shell's
//! daemon interface (../../daemon-serve), in the ready-row form
//! (../../ready-rows): each copy kept is a row of the launcher's Copies,
//! `row:copies:<N>`, newest first, worded from the launcher's vocabulary
//! (src/rows.rs), so the shell builds and words nothing. Text and pictures
//! are numbered from one count, and a copy made again keeps its number and
//! moves to the front, which sends that row alone. Beside them go
//! `summary:copies`, how many are kept, and `now:clipboard`, what the
//! clipboard holds now: the types it's offered as, and its text where it's
//! offered as text, kept or not, so the shell's acts on a copy (Copied.qml)
//! ask nothing of their own. A secret, the clipboard emptied with nothing to
//! give back, or the history forgotten holds no types and no text. With
//! `--keep 0` nothing is kept and that is all it serves, for a shell whose
//! launcher keeps no copies.
//!
//! Text is kept in memory, and pictures in files of their own in the
//! directory `--pictures` names, only this account's to read, which should
//! be the runtime directory, since that's memory too and goes when the
//! session does. A picture is a PNG, JPEG, WebP or GIF, read only when the
//! copy offers no text, as a spreadsheet's cells offer both. Without
//! `--pictures`, no picture is kept.
//!
//! A copy a password manager marks as secret is never kept, nor text larger
//! than `--largest`, nor a picture larger than `--largest-picture`. The
//! pictures kept take up at most `--room`, the oldest going first, their
//! rows and files going with them.
//!
//! The shell asks it, as its own asks, `{"ask": "copy", "copy": N}` to put
//! copy N back on the clipboard (not `id`, which names the ask itself), or with `"text"` in place of the number to
//! put the shell's own text there, kept as a copy is. `"paste"`, with
//! `"into": "terminal"` or `"plain"`, puts it there and then presses the
//! paste keys for the window the launcher closed over: Ctrl+Shift+V for a
//! terminal, whose Ctrl+V is a key of its own, and Ctrl+V anywhere else.
//! Without `"into"`, they're the keys of the window niri says has the
//! focus once the launcher lets go, and with none focused, the copy is
//! only put on the clipboard, with nowhere to paste it.
//! They're pressed through the compositor's virtual keyboard, once niri
//! says the launcher's layer has let go of the keys, so they reach the
//! window and not the launcher, and nothing waits on the shell's thread:
//! the window reads the copy from here, too. A paste is answered once its
//! keys are pressed, or refused as `unpasted` once they couldn't be.
//! `"clear"` forgets everything.
//!
//! A copy outlives the program that made it: when the clipboard empties
//! because that program let go of it, as one does when it closes, this one
//! offers the same copy in its place, if it kept it. A secret, or anything
//! else it didn't keep, is let go with its program, and once the history is
//! forgotten, what it offers is nothing. Offering it takes the clipboard
//! from whatever holds it, so it waits until every event already sent has
//! been read and the compositor has answered a round trip: a copy made
//! since the clipboard emptied, which the compositor has taken, is then
//! known, and kept rather than replaced.
//!
//! Usage: clipboard-history --socket PATH --vocabulary WORDS [--keep N]
//!        [--largest BYTES] [--pictures DIR] [--largest-picture BYTES]
//!        [--room BYTES]
//!
//! WORDS is the launcher's words.json. Its numbers are grouped and pointed
//! as the launcher's counts are, with the comma and the full stop.

mod rows;

use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, BufRead, BufReader, PipeReader, PipeWriter, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, process, thread};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use ready_rows::kept::Kept;
use ready_rows::numbers::Numbers;
use rows::Words;
use rustix::event::{PollFd, PollFlags, poll};
use rustix::fs::{MemfdFlags, memfd_create};
use serde_json::{Value, json};
use wayland_client::backend::{ObjectId, WaylandError};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

/// The type KDE's Klipper set the convention with, and 1Password follows,
/// for a copy no history should keep.
const SECRET: &str = "x-kde-passwordManagerHint";

/// Text types, in the order they're asked for: the first an offer has wins.
const TEXT: [&str; 5] = [
    "text/plain;charset=utf-8",
    "UTF8_STRING",
    "text/plain",
    "STRING",
    "TEXT",
];

/// Picture types the shell can draw, in the order they're asked for.
const PICTURE: [&str; 4] = ["image/png", "image/jpeg", "image/webp", "image/gif"];

/// A type only this program's own offer has, so it knows the clipboard it
/// set from one a program copied.
const OURS: &str = "application/x-clipboard-history-kept";

/// How long a program has to hand over what it copied before it's skipped.
const PATIENCE: Duration = Duration::from_secs(2);

/// The layer the launcher draws on, as it names itself to niri, which
/// holds the keys while it's open.
const LAUNCHER: &str = "launcher";

/// The programs whose Ctrl+V is a key of their own, which paste with
/// Ctrl+Shift+V instead: the terminals, by the app id niri gives them, as
/// the shell's Pasting.qml names them.
const TERMINALS: [&str; 5] = [
    "kitty",
    "foot",
    "Alacritty",
    "org.wezfurlong.wezterm",
    "com.mitchellh.ghostty",
];

/// How long the launcher has to let go of the keys before a paste gives up
/// rather than press them into it.
const LETTING_GO: Duration = Duration::from_secs(1);

/// How often niri is asked whether the launcher has let go of the keys.
const ASKING: Duration = Duration::from_millis(1);

/// How long a paste waits with no niri to ask, as the shell's own wait for
/// the keys to move was.
const UNASKED: Duration = Duration::from_millis(150);

/// The virtual keyboard's own keymap: one key, V, at evdev's code 1, with
/// the usual modifiers, so Control and Shift are the masks below
/// whatever keymap Joel types in.
const KEYMAP: &str = "xkb_keymap {
  xkb_keycodes \"paste\" { minimum = 8; maximum = 9; <V> = 9; };
  xkb_types \"paste\" { include \"complete\" };
  xkb_compatibility \"paste\" { include \"complete\" };
  xkb_symbols \"paste\" { key <V> { [ v ] }; };
};
";

/// V, in the keymap above, as evdev numbers it.
const V: u32 = 1;

/// The modifiers' masks in the keymap above.
const SHIFT: u32 = 1;
const CONTROL: u32 = 4;

/// Which paste keys a window takes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Keys {
    /// Ctrl+V.
    Plain,
    /// Ctrl+Shift+V, for a terminal, whose Ctrl+V is a key of its own.
    Terminal,
}

impl Keys {
    fn modifiers(self) -> u32 {
        match self {
            Keys::Plain => CONTROL,
            Keys::Terminal => CONTROL | SHIFT,
        }
    }
}

/// The copy the shell names: one kept, by its number, or text of its own.
#[derive(Debug, PartialEq)]
enum Which {
    Kept(u64),
    Text(String),
}

/// What the shell asks for: a paste with the keys it names, or with none,
/// those of the window under the launcher.
#[derive(Debug, PartialEq)]
enum Wanted {
    Clear,
    Copy(Which),
    Paste(Which, Option<Keys>),
}

/// Where an ask's answer goes: back to the connection that asked, which
/// waits for it on a thread of its own.
type Reply = mpsc::Sender<Value>;

/// What the Wayland side is handed: an ask and where its answer goes, or a
/// paste's keys once the launcher has let go of them, with how long that
/// took after the ask and when it did, or a paste that never could be.
#[derive(Debug)]
enum Asked {
    Wanted(Wanted, Reply),
    Press(Keys, Duration, Instant, Reply),
    Unpasted(Reply),
}

/// How long an ask waits on the Wayland side before it's answered as
/// not answering: past a paste's whole wait for the keys.
const ANSWERING: Duration = Duration::from_secs(3);

/// An ask refused, in the interface's form, its words the launcher's.
fn refused(code: &str, message: &str, said: &str) -> Value {
    json!({ "error": { "code": code, "message": message, "said": said } })
}

/// What a paste says of its time, in the journal: how long the launcher
/// took to let go of the keys after the ask, which is the shell's frame,
/// and how long the keys took to be pressed after that, which is ours.
fn pasted(letting_go: Duration, pressing: Duration) -> String {
    let ms = |took: Duration| took.as_secs_f64() * 1000.0;
    format!(
        "Pasted: the launcher let go of the keys {:.1} ms after the ask, and they were \
         pressed {:.1} ms after that.",
        ms(letting_go),
        ms(pressing)
    )
}

/// What one of the shell's own asks wants, or none for one it doesn't
/// know.
fn wanted(said: &Value) -> Option<Wanted> {
    match said["ask"].as_str()? {
        "clear" => Some(Wanted::Clear),
        "copy" => Some(Wanted::Copy(which(said)?)),
        "paste" => {
            let keys = match &said["into"] {
                Value::Null => None,
                Value::String(into) if into == "terminal" => Some(Keys::Terminal),
                Value::String(into) if into == "plain" => Some(Keys::Plain),
                _ => return None,
            };
            Some(Wanted::Paste(which(said)?, keys))
        }
        _ => None,
    }
}

/// The copy an ask names: one kept by its number, or text of the shell's.
fn which(said: &Value) -> Option<Which> {
    if let Some(id) = said["copy"].as_u64() {
        return Some(Which::Kept(id));
    }
    said["text"]
        .as_str()
        .map(|text| Which::Text(text.to_owned()))
}

/// Whether niri's answer to `Layers` says the launcher holds no keys: none
/// when it isn't an answer to that.
fn let_go(answer: &str) -> Option<bool> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let layers = answer.get("Ok")?.get("Layers")?.as_array()?;
    Some(!layers.iter().any(|layer| {
        layer.get("namespace").and_then(Value::as_str) == Some(LAUNCHER)
            && layer.get("keyboard_interactivity").and_then(Value::as_str) != Some("None")
    }))
}

/// What niri says has the focus: a window, which takes these keys, or
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Focus {
    Window(Keys),
    Nowhere,
}

/// What niri's answer to `FocusedWindow` says has the focus: none when it
/// isn't an answer to that.
fn focused(answer: &str) -> Option<Focus> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let window = answer.get("Ok")?.get("FocusedWindow")?;
    if window.is_null() {
        return Some(Focus::Nowhere);
    }
    let app = window.get("app_id").and_then(Value::as_str).unwrap_or("");
    Some(Focus::Window(if TERMINALS.contains(&app) {
        Keys::Terminal
    } else {
        Keys::Plain
    }))
}

/// Asks niri one question, `ask` as its socket takes it, and reads its
/// answer with `read`.
fn asked_niri<T>(socket: &Path, ask: &[u8], read: fn(&str) -> Option<T>) -> Option<T> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(LETTING_GO)).ok()?;
    stream.write_all(ask).ok()?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).ok()?;
    read(&answer)
}

/// The keys the window under the launcher takes, once it has let go of
/// them, as niri says, or none with no window focused to paste into. With
/// no niri to ask, or no answer from it, Ctrl+V, as most windows take.
fn under(niri: Option<&Path>) -> Option<Keys> {
    match niri.and_then(|niri| asked_niri(niri, b"\"FocusedWindow\"\n", focused)) {
        Some(Focus::Window(keys)) => Some(keys),
        Some(Focus::Nowhere) => None,
        None => Some(Keys::Plain),
    }
}

/// Waits until niri says the launcher has let go of the keys, or with no
/// niri to ask, a moment. Answers whether they're free to press. niri is
/// asked afresh each millisecond, a connection each time, since it answers
/// one question a connection, and the keys are pressed on the first answer
/// that says they're free rather than on a second confirming it: niri
/// gives the keys to the window as it ends the turn that answered, before
/// it reads the keys pressed after the answer, and a second ask waited
/// behind niri drawing, 89 ms on a builder (2026-11-10).
fn keys_let_go(niri: Option<&Path>) -> bool {
    let Some(niri) = niri else {
        thread::sleep(UNASKED);
        return true;
    };
    let deadline = Instant::now() + LETTING_GO;
    while Instant::now() < deadline {
        if asked_niri(niri, b"\"Layers\"\n", let_go) == Some(true) {
            return true;
        }
        thread::sleep(ASKING);
    }
    false
}

/// What an offer is read as.
#[derive(Debug, PartialEq)]
enum Readable {
    Text(&'static str),
    Picture(&'static str),
}

/// The type to read an offer as: text when it has any, a picture when it
/// has only that and pictures are kept, or none for one that's secret.
fn readable(types: &[String], pictures: bool) -> Option<Readable> {
    if types.iter().any(|t| t == SECRET || t == OURS) {
        return None;
    }
    let has = |wanted: &&str| types.iter().any(|t| t == wanted);
    if let Some(kind) = TEXT.into_iter().find(has) {
        return Some(Readable::Text(kind));
    }
    PICTURE
        .into_iter()
        .find(has)
        .filter(|_| pictures)
        .map(Readable::Picture)
}

/// A picture kept: its number, its type, the file holding it, its size in
/// bytes, what tells it from another, and its width and height when its
/// type says them.
#[derive(Clone, Debug, PartialEq)]
struct Picture {
    id: u64,
    kind: &'static str,
    path: PathBuf,
    bytes: usize,
    hash: u64,
    size: Option<(u32, u32)>,
}

/// What a copy holds.
#[derive(Clone, Debug, PartialEq)]
enum Content {
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
    fn types(&self) -> Vec<&'static str> {
        match self {
            Content::Text(_) => TEXT.to_vec(),
            Content::Picture(picture) => vec![picture.kind],
        }
    }
}

/// What was read from an offer, before it's kept.
#[derive(Debug)]
enum Heard {
    Text(String),
    Picture { kind: &'static str, bytes: Vec<u8> },
}

/// One kept copy, and its number.
#[derive(Clone, Debug, PartialEq)]
struct Copy {
    at: u128,
    id: u64,
    content: Content,
}

impl Copy {
    /// Its row in the launcher's Copies.
    fn row(&self, words: &Words) -> Value {
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
fn dimensions(kind: &str, bytes: &[u8]) -> Option<(u32, u32)> {
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
struct History {
    copies: VecDeque<Copy>,
    keep: usize,
    current: Option<Content>,
    pictures: Option<PathBuf>,
    room: usize,
    next: u64,
    now: Value,
    changes: u64,
}

/// What the clipboard holds once a program copies: nothing of a secret,
/// and text only where it was read as text.
fn offered(types: &[String], heard: Option<&Heard>) -> Value {
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
    fn new(keep: usize, pictures: Option<PathBuf>, room: usize) -> History {
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
    fn holds(&mut self, now: Value) {
        if now != self.now {
            self.now = now;
            self.changes += 1;
        }
    }

    /// What the clipboard holds once it offers the copy this offers, or
    /// nothing while it offers none.
    fn holds_current(&mut self) {
        let now = match &self.current {
            Some(Content::Text(text)) => rows::now(&TEXT, text),
            Some(content @ Content::Picture(_)) => rows::now(&content.types(), ""),
            None => rows::now::<&str>(&[], ""),
        };
        self.holds(now);
    }

    /// Everything it serves: a row for each copy, newest first, and beside
    /// them how many there are and what the clipboard holds.
    fn records(&self, words: &Words) -> (Vec<Value>, Vec<Value>) {
        let rows = self.copies.iter().map(|copy| copy.row(words)).collect();
        let beside = vec![rows::summary(self.copies.len(), words), self.now.clone()];
        (rows, beside)
    }

    /// Keeps a copy, unless it's blank text. One copied again moves to the
    /// front rather than being kept twice. The oldest go once there are more
    /// than it keeps, and the oldest pictures once they take more room than
    /// they have, never the one just kept. Answers whether it was kept.
    fn add(&mut self, copy: Copy) -> bool {
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
    fn copied(&mut self, heard: Option<Heard>) -> Option<Copy> {
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
    fn choose(&mut self, id: u64) -> Option<Copy> {
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
    fn forget(&mut self) {
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
struct Served {
    shared: Arc<Shared>,
    kept: Kept,
    words: Words,
    changes: Option<u64>,
}

impl Served {
    /// Sends the shell what changed in the history since it was last sent,
    /// unless nothing has.
    fn send(&mut self, history: &History) {
        if self.changes == Some(history.changes) {
            return;
        }
        self.changes = Some(history.changes);
        let (rows, beside) = history.records(&self.words);
        let moved = self.kept.take(rows, beside);
        if !moved.changed.is_empty() || !moved.gone.is_empty() {
            self.shared
                .change(self.kept.all().cloned().collect(), json!({}));
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
fn prepared(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    for entry in fs::read_dir(dir)? {
        fs::remove_file(entry?.path())?;
    }
    Ok(())
}

/// What the Wayland side needs: the history and what's served of it, the
/// types each offer has said it can be read as until its selection
/// arrives, the largest copies it reads, while a copy is offered in its
/// program's place, that offer, the keyboard a paste's keys are pressed
/// through, and how a paste waiting on the launcher to let go of the keys
/// says it may press them.
struct State {
    history: History,
    served: Served,
    offers: HashMap<ObjectId, Vec<String>>,
    largest: usize,
    largest_picture: usize,
    pictures: bool,
    manager: ExtDataControlManagerV1,
    device: ExtDataControlDeviceV1,
    held: Option<ExtDataControlSourceV1>,
    // The clipboard emptied, and nothing has been copied since.
    emptied: bool,
    keyboard: Option<ZwpVirtualKeyboardV1>,
    niri: Option<PathBuf>,
    pressing: mpsc::Sender<Asked>,
    waking: PipeWriter,
    started: Instant,
}

impl State {
    /// Offers the copy the clipboard holds, in place of the program that
    /// let go of it, or as the shell chose it.
    fn hold(&mut self, handle: &QueueHandle<Self>) {
        let Some(types) = self.history.current.as_ref().map(Content::types) else {
            return;
        };
        let source = self.manager.create_data_source(handle, ());
        for kind in types {
            source.offer(kind.to_string());
        }
        source.offer(OURS.to_string());
        self.device.set_selection(Some(&source));
        if let Some(before) = self.held.replace(source) {
            before.destroy();
        }
    }

    /// Puts a copy on the clipboard, as the shell asked: one kept, or the
    /// shell's own text, kept as a copy is. Answers whether it's there.
    fn offer(&mut self, which: Which, handle: &QueueHandle<Self>) -> bool {
        let offered = match which {
            Which::Kept(id) => self.history.choose(id),
            Which::Text(text) => self.history.copied(Some(Heard::Text(text))),
        };
        if offered.is_none() {
            return false;
        }
        self.history.holds_current();
        self.hold(handle);
        true
    }

    /// Puts a copy on the clipboard, then waits, away from the Wayland
    /// side, for the launcher to let go of the keys, and asks for the paste
    /// keys to be pressed once it has: those named, or those of the window
    /// under it. The paste is answered then, or with no window to paste
    /// into, as a copy.
    fn paste(
        &mut self,
        which: Which,
        keys: Option<Keys>,
        reply: Reply,
        handle: &QueueHandle<Self>,
    ) {
        let asked = Instant::now();
        if !self.offer(which, handle) {
            eprintln!("That copy isn't kept, so nothing was pasted.");
            let _ = reply.send(self.unkept());
            return;
        }
        let (Ok(mut waking), niri) = (self.waking.try_clone(), self.niri.clone()) else {
            let _ = reply.send(self.unpasted());
            return;
        };
        let pressing = self.pressing.clone();
        thread::spawn(move || {
            let said = if keys_let_go(niri.as_deref()) {
                let let_go = Instant::now();
                let Some(keys) = keys.or_else(|| under(niri.as_deref())) else {
                    let _ = reply.send(json!({ "copied": true }));
                    return;
                };
                Asked::Press(keys, let_go - asked, let_go, reply)
            } else {
                eprintln!("The launcher never let go of the keys, so nothing was pasted.");
                Asked::Unpasted(reply)
            };
            if pressing.send(said).is_ok() {
                let _ = waking.write_all(b"!");
            }
        });
    }

    /// Does what the shell asked, answering it then, or for a paste, once
    /// its keys are pressed.
    fn wanted(&mut self, wanted: Wanted, reply: Reply, handle: &QueueHandle<Self>) {
        let answer = match wanted {
            Wanted::Copy(which) => {
                if self.offer(which, handle) {
                    json!({ "copied": true })
                } else {
                    self.unkept()
                }
            }
            Wanted::Paste(which, keys) => return self.paste(which, keys, reply, handle),
            Wanted::Clear => {
                self.history.forget();
                json!({ "cleared": true })
            }
        };
        let _ = reply.send(answer);
    }

    /// The answer to a paste that never landed.
    fn unpasted(&self) -> Value {
        let said = self.served.words.said("unpasted");
        refused("unpasted", "the paste keys were never pressed", &said)
    }

    /// The answer to an ask for a copy that's no longer kept.
    fn unkept(&self) -> Value {
        let said = self.served.words.said("unkept");
        refused("unkept", "that copy is no longer kept", &said)
    }

    /// Presses the paste keys through the virtual keyboard: its modifiers
    /// held, V pressed and let go, then the modifiers let go. Answers
    /// whether there was a keyboard to press them on.
    fn press(&self, keys: Keys) -> bool {
        let Some(keyboard) = &self.keyboard else {
            eprintln!("The compositor has no virtual keyboard, so nothing was pasted.");
            return false;
        };
        let time = u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX);
        keyboard.modifiers(keys.modifiers(), 0, 0, 0);
        keyboard.key(time, V, 1);
        keyboard.key(time, V, 0);
        keyboard.modifiers(0, 0, 0, 0);
        true
    }
}

/// A virtual keyboard on the seat, given its own keymap, or none when the
/// compositor has none to give.
fn virtual_keyboard(
    manager: Option<&ZwpVirtualKeyboardManagerV1>,
    seat: &wl_seat::WlSeat,
    handle: &QueueHandle<State>,
) -> Option<ZwpVirtualKeyboardV1> {
    let keyboard = manager?.create_virtual_keyboard(seat, handle, ());
    let mut keymap = File::from(memfd_create("paste-keymap", MemfdFlags::CLOEXEC).ok()?);
    // The keymap's size counts the NUL that ends it, as XKB reads it.
    keymap.write_all(KEYMAP.as_bytes()).ok()?;
    keymap.write_all(&[0]).ok()?;
    let size = u32::try_from(KEYMAP.len() + 1).ok()?;
    // XKB's text format, the only one the protocol knows.
    keyboard.keymap(1, keymap.as_fd(), size);
    Some(keyboard)
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardManagerV1,
        _: <ZwpVirtualKeyboardManagerV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardV1,
        _: <ZwpVirtualKeyboardV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            // Read when asked, so a history forgotten since gives nothing.
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                let content = state.history.current.clone();
                thread::spawn(move || {
                    let mut file = File::from(fd);
                    match content {
                        Some(Content::Text(text)) => {
                            let _ = file.write_all(text.as_bytes());
                        }
                        Some(Content::Picture(picture)) => {
                            if let Ok(mut kept) = File::open(&picture.path) {
                                let _ = io::copy(&mut kept, &mut file);
                            }
                        }
                        None => {}
                    }
                });
            }
            // Something else was copied, which holds the clipboard now.
            ext_data_control_source_v1::Event::Cancelled => {
                source.destroy();
                if state.held.as_ref() == Some(source) {
                    state.held = None;
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        _: &mut Self,
        _: &wl_seat::WlSeat,
        _: wl_seat::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &ExtDataControlManagerV1,
        _: <ExtDataControlManagerV1 as Proxy>::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtDataControlOfferV1, ()> for State {
    fn event(
        state: &mut Self,
        offer: &ExtDataControlOfferV1,
        event: ext_data_control_offer_v1::Event,
        (): &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let ext_data_control_offer_v1::Event::Offer { mime_type } = event {
            state.offers.entry(offer.id()).or_default().push(mime_type);
        }
    }
}

impl Dispatch<ExtDataControlDeviceV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ExtDataControlDeviceV1,
        event: ext_data_control_device_v1::Event,
        (): &(),
        connection: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_data_control_device_v1::Event::DataOffer { id } => {
                state.offers.insert(id.id(), Vec::new());
            }
            ext_data_control_device_v1::Event::Selection { id: Some(offer) } => {
                state.emptied = false;
                let types = state.offers.remove(&offer.id()).unwrap_or_default();
                // Its own offer, heard back: the clipboard holds what it did.
                if types.iter().any(|kind| kind == OURS) {
                    offer.destroy();
                    return;
                }
                let heard = match readable(&types, state.pictures) {
                    Some(Readable::Text(kind)) => receive(connection, &offer, kind, state.largest)
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .map(Heard::Text),
                    Some(Readable::Picture(kind)) => {
                        receive(connection, &offer, kind, state.largest_picture)
                            .map(|bytes| Heard::Picture { kind, bytes })
                    }
                    None => None,
                };
                offer.destroy();
                let now = offered(&types, heard.as_ref());
                state.history.copied(heard);
                state.history.holds(now);
            }
            // The program holding the clipboard let go of it: what it held
            // is offered once the events after this are read (run).
            ext_data_control_device_v1::Event::Selection { id: None } => {
                state.emptied = true;
            }
            // The selection a middle click pastes isn't kept.
            ext_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                state.offers.remove(&offer.id());
                offer.destroy();
            }
            ext_data_control_device_v1::Event::Finished => {
                eprintln!("The compositor took the clipboard away.");
                process::exit(1);
            }
            _ => {}
        }
    }

    event_created_child!(State, ExtDataControlDeviceV1, [
        ext_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ExtDataControlOfferV1, ()),
    ]);
}

/// Reads an offer as a type, or nothing when it's larger than `largest` or
/// its program doesn't hand it over in time.
fn receive(
    connection: &Connection,
    offer: &ExtDataControlOfferV1,
    kind: &str,
    largest: usize,
) -> Option<Vec<u8>> {
    let (reader, writer) = io::pipe().ok()?;
    offer.receive(kind.to_string(), writer.as_fd());
    connection.flush().ok()?;
    // The program writes into its own copy of the pipe; once it closes that,
    // the read ends.
    drop(writer);
    let (sent, heard) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = reader.take(largest as u64 + 1).read_to_end(&mut bytes);
        let _ = sent.send(read.map(|_| bytes));
    });
    let bytes = heard.recv_timeout(PATIENCE).ok()?.ok()?;
    (bytes.len() <= largest).then_some(bytes)
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// What the interface names this daemon, and the kinds of record it holds.
const DOMAIN: Domain = Domain {
    name: "clipboard",
    kinds: &["row", "summary", "now"],
};

/// The shell's own asks, handed to the Wayland side, which is woken to do
/// them, and answered as it answers, on the asking connection's thread.
fn own(asked: mpsc::Sender<Asked>, wake: PipeWriter) -> Own {
    let wake = Mutex::new(wake);
    Box::new(move |said| {
        let wanted = wanted(said)?;
        let (reply, answer) = mpsc::channel();
        let handed = asked.send(Asked::Wanted(wanted, reply)).is_ok()
            && wake
                .lock()
                .is_ok_and(|mut wake| wake.write_all(b"!").is_ok());
        let silent = || refused("not-answering", "the clipboard isn't answering", "");
        if !handed {
            return Some(silent());
        }
        Some(answer.recv_timeout(ANSWERING).unwrap_or_else(|_| silent()))
    })
}

/// The clipboard's words, from the launcher's vocabulary.
fn worded(vocabulary: &Path) -> Result<Words, String> {
    let text = fs::read_to_string(vocabulary)
        .map_err(|e| format!("can't read {}: {e}", vocabulary.display()))?;
    Words::from_vocabulary(&text, Numbers::default())
        .map_err(|e| format!("{} isn't JSON: {e}", vocabulary.display()))
}

struct Settings {
    socket: PathBuf,
    vocabulary: PathBuf,
    keep: usize,
    largest: usize,
    pictures: Option<PathBuf>,
    largest_picture: usize,
    room: usize,
}

fn settings() -> Result<Settings, String> {
    let (mut socket, mut vocabulary) = (None, None);
    let mut keep = 200;
    let mut largest = 1 << 20;
    let mut pictures = None;
    let mut largest_picture = 64 << 20;
    let mut room = 256 << 20;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        let bytes = |value: String, name: &str| {
            value
                .parse::<usize>()
                .map_err(|_| format!("{name} takes bytes"))
        };
        match argument.as_str() {
            "--socket" => socket = Some(PathBuf::from(value()?)),
            "--vocabulary" => vocabulary = Some(PathBuf::from(value()?)),
            "--keep" => keep = value()?.parse().map_err(|_| "--keep takes a count")?,
            "--largest" => largest = bytes(value()?, "--largest")?,
            "--pictures" => pictures = Some(PathBuf::from(value()?)),
            "--largest-picture" => largest_picture = bytes(value()?, "--largest-picture")?,
            "--room" => room = bytes(value()?, "--room")?,
            other => return Err(format!("{other} isn't something this understands")),
        }
    }
    let socket = socket.ok_or("--socket names where the shell connects")?;
    let vocabulary = vocabulary.ok_or("--vocabulary names the launcher's words")?;
    if largest_picture > room {
        return Err("--largest-picture can't be more than --room".to_string());
    }
    Ok(Settings {
        socket,
        vocabulary,
        keep,
        largest,
        pictures,
        largest_picture,
        room,
    })
}

fn lost(error: impl std::fmt::Display) -> String {
    format!("lost Wayland: {error}")
}

fn run() -> Result<(), String> {
    let settings = settings()?;
    let words = worded(&settings.vocabulary)?;
    if let Some(dir) = &settings.pictures {
        prepared(dir).map_err(|e| format!("can't keep pictures in {}: {e}", dir.display()))?;
    }
    let connection = Connection::connect_to_env().map_err(|e| format!("no Wayland: {e}"))?;
    let (globals, mut queue) =
        registry_queue_init::<State>(&connection).map_err(|e| format!("no registry: {e}"))?;
    let handle = queue.handle();
    let manager: ExtDataControlManagerV1 = globals
        .bind(&handle, 1..=1, ())
        .map_err(|_| "the compositor has no ext_data_control_manager_v1")?;
    let seat: wl_seat::WlSeat = globals
        .bind(&handle, 1..=9, ())
        .map_err(|_| "the compositor has no seat")?;
    let device = manager.get_data_device(&seat, &handle, ());
    let keyboards: Option<ZwpVirtualKeyboardManagerV1> = globals.bind(&handle, 1..=1, ()).ok();
    let keyboard = virtual_keyboard(keyboards.as_ref(), &seat, &handle);

    let listener = serve::listen(&settings.socket)
        .map_err(|e| format!("can't listen at {}: {e}", settings.socket.display()))?;
    let (asked, wanted) = mpsc::channel();
    let (mut woken, wake) = io::pipe().map_err(|e| format!("no pipe: {e}"))?;
    let (pressing, waking) = (
        asked.clone(),
        wake.try_clone().map_err(|e| format!("no pipe: {e}"))?,
    );
    let shared = Shared::new(DOMAIN, own(asked, wake));
    let mut served = Served {
        shared: Arc::clone(&shared),
        kept: Kept::default(),
        words,
        changes: None,
    };
    let history = History::new(settings.keep, settings.pictures.clone(), settings.room);
    served.send(&history);
    serve::accept(shared, listener);

    let mut state = State {
        history,
        served,
        offers: HashMap::new(),
        largest: settings.largest,
        largest_picture: settings.largest_picture,
        pictures: settings.pictures.is_some(),
        manager,
        device,
        held: None,
        emptied: false,
        keyboard,
        niri: env::var_os("NIRI_SOCKET").map(PathBuf::from),
        pressing,
        waking,
        started: Instant::now(),
    };
    loop {
        queue.dispatch_pending(&mut state).map_err(lost)?;
        if state.emptied {
            queue.roundtrip(&mut state).map_err(lost)?;
        }
        // Emptied, the clipboard holds the copy given back, or nothing.
        if std::mem::take(&mut state.emptied) {
            state.history.holds_current();
            if state.history.current.is_some() {
                state.hold(&handle);
            }
        }
        for wanted in wanted.try_iter() {
            match wanted {
                Asked::Wanted(wanted, reply) => state.wanted(wanted, reply, &handle),
                Asked::Press(keys, letting_go, let_go, reply) => {
                    let answer = if state.press(keys) {
                        eprintln!("{}", pasted(letting_go, let_go.elapsed()));
                        json!({ "pasted": true })
                    } else {
                        state.unpasted()
                    };
                    let _ = reply.send(answer);
                }
                Asked::Unpasted(reply) => {
                    let _ = reply.send(state.unpasted());
                }
            }
        }
        queue.flush().map_err(lost)?;
        state.served.send(&state.history);
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let (wayland, asked) = waited(&guard.connection_fd(), &woken)?;
        if wayland {
            match guard.read() {
                Ok(_) => {}
                Err(WaylandError::Io(e)) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(lost(e)),
            }
        } else {
            drop(guard);
        }
        if asked {
            let _ = woken.read(&mut [0; 64]);
        }
    }
}

/// Waits until the compositor or the shell has something to say, and says
/// which.
fn waited(wayland: &impl AsFd, woken: &PipeReader) -> Result<(bool, bool), String> {
    let mut fds = [
        PollFd::new(wayland, PollFlags::IN),
        PollFd::new(woken, PollFlags::IN),
    ];
    loop {
        match poll(&mut fds, None) {
            Ok(_) => break,
            Err(rustix::io::Errno::INTR) => {}
            Err(e) => return Err(format!("can't wait: {e}")),
        }
    }
    Ok((!fds[0].revents().is_empty(), !fds[1].revents().is_empty()))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("clipboard-history: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::words;

    fn types(names: &[&str]) -> Vec<String> {
        names.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_secret_is_never_read() {
        let offered = types(&["text/plain;charset=utf-8", SECRET]);
        assert_eq!(readable(&offered, true), None);
        assert_eq!(readable(&types(&["image/png", SECRET]), true), None);
    }

    #[test]
    fn text_is_read_as_utf8_first() {
        let offered = types(&["TEXT", "text/plain", "text/plain;charset=utf-8"]);
        assert_eq!(
            readable(&offered, true),
            Some(Readable::Text("text/plain;charset=utf-8"))
        );
    }

    #[test]
    fn text_is_read_before_a_picture_offered_beside_it() {
        let offered = types(&["image/png", "text/plain"]);
        assert_eq!(readable(&offered, true), Some(Readable::Text("text/plain")));
    }

    #[test]
    fn a_picture_is_read_as_png_first() {
        let offered = types(&["image/gif", "image/jpeg", "image/png"]);
        assert_eq!(
            readable(&offered, true),
            Some(Readable::Picture("image/png"))
        );
    }

    #[test]
    fn a_picture_is_not_read_when_pictures_arent_kept() {
        assert_eq!(readable(&types(&["image/png"]), false), None);
    }

    #[test]
    fn a_picture_the_shell_cant_draw_is_not_read() {
        assert_eq!(readable(&types(&["image/tiff"]), true), None);
    }

    #[test]
    fn its_own_offer_is_never_read() {
        assert_eq!(readable(&types(&["text/plain", OURS]), true), None);
    }

    /// A directory of its own for one test's pictures.
    fn directory(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("clipboard-history-{}-{name}", process::id()));
        prepared(&dir).unwrap();
        dir
    }

    fn history(keep: usize) -> History {
        History::new(keep, None, 0)
    }

    /// The keys of the rows it serves, in their order.
    fn keys(history: &History) -> Vec<String> {
        let (rows, _) = history.records(&words());
        rows.iter()
            .map(|row| row["key"].as_str().unwrap_or_default().to_owned())
            .collect()
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's what copied takes, as Wayland gives it"
    )]
    fn picture(bytes: &[u8]) -> Option<Heard> {
        Some(Heard::Picture {
            kind: "image/png",
            bytes: bytes.to_vec(),
        })
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's what copied takes, as Wayland gives it"
    )]
    fn text(said: &str) -> Option<Heard> {
        Some(Heard::Text(said.to_string()))
    }

    #[test]
    fn a_kept_copy_is_what_the_clipboard_holds() {
        let mut kept = history(10);
        kept.copied(text("one"));
        assert_eq!(kept.current, Some(Content::Text("one".to_string())));
    }

    #[test]
    fn a_copy_not_kept_leaves_nothing_to_give_back() {
        let mut kept = history(10);
        kept.copied(text("one"));
        // A secret or something too large reads as none.
        assert_eq!(kept.copied(None), None);
        assert_eq!(kept.current, None);
        kept.copied(text("two"));
        kept.copied(text("  "));
        assert_eq!(kept.current, None);
    }

    #[test]
    fn a_picture_is_kept_in_a_file_of_its_own() {
        let dir = directory("kept");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        let copy = kept.copied(picture(b"a picture")).unwrap();
        let Content::Picture(made) = &copy.content else {
            panic!("not a picture: {copy:?}");
        };
        assert_eq!(fs::read(&made.path).unwrap(), b"a picture");
        assert_eq!(made.path.parent(), Some(dir.as_path()));
        let permissions = fs::metadata(&made.path).unwrap().permissions().mode();
        assert_eq!(permissions & 0o777, 0o600);
        assert_eq!(kept.current, Some(copy.content.clone()));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_never_handed_over_is_not_kept() {
        let dir = directory("empty");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        kept.copied(text("one"));
        assert_eq!(kept.copied(picture(b"")), None);
        assert_eq!(kept.current, None);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_is_not_kept_without_a_directory() {
        let mut kept = history(10);
        assert_eq!(kept.copied(picture(b"a picture")), None);
        assert_eq!(kept.current, None);
    }

    #[test]
    fn a_picture_copied_again_is_the_one_kept() {
        let dir = directory("again");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        let first = kept.copied(picture(b"a picture")).unwrap();
        kept.copied(text("between"));
        let again = kept.copied(picture(b"a picture")).unwrap();
        assert_eq!(first.content, again.content);
        assert_eq!(kept.copies.len(), 2);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_oldest_pictures_go_once_they_take_too_much_room() {
        let dir = directory("room");
        let mut kept = History::new(10, Some(dir.clone()), 10);
        let oldest = kept.copied(picture(b"aaaa")).unwrap();
        kept.copied(text("words take no room"));
        kept.copied(picture(b"bbbb"));
        kept.copied(picture(b"cccc"));
        assert_eq!(kept.copies.len(), 3);
        let Content::Picture(oldest) = oldest.content else {
            unreachable!()
        };
        assert!(!oldest.path.exists(), "the oldest picture's file is kept");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        // Its row goes with it.
        assert_eq!(
            keys(&kept),
            ["row:copies:4", "row:copies:3", "row:copies:2"]
        );
        assert_eq!(oldest.id, 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_past_what_it_keeps_leaves_its_row() {
        let dir = directory("told");
        let mut kept = History::new(1, Some(dir.clone()), 100);
        kept.copied(picture(b"a picture"));
        assert_eq!(keys(&kept), ["row:copies:1"]);
        kept.copied(text("one"));
        assert_eq!(keys(&kept), ["row:copies:2"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_past_what_it_keeps_goes_with_its_file() {
        let dir = directory("keep");
        let mut kept = History::new(1, Some(dir.clone()), 100);
        kept.copied(picture(b"a picture"));
        kept.copied(text("one"));
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_chosen_moves_to_the_front_as_what_the_clipboard_holds() {
        let dir = directory("chosen");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        let made = kept.copied(picture(b"a picture")).unwrap();
        kept.copied(text("since"));
        let Content::Picture(picture) = &made.content else {
            unreachable!()
        };
        let chosen = kept.choose(picture.id).unwrap();
        assert_eq!(chosen.content, made.content);
        assert_eq!(kept.copies[0].content, made.content);
        assert_eq!(kept.current, Some(made.content.clone()));
        // A number no copy has, text and pictures sharing one count.
        assert_eq!(kept.choose(99), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn forgetting_forgets_what_the_clipboard_holds_too() {
        let dir = directory("forget");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        kept.copied(text("one"));
        kept.copied(picture(b"a picture"));
        kept.forget();
        assert!(kept.copies.is_empty());
        assert_eq!(kept.current, None);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_earlier_runs_pictures_are_cleared() {
        let dir = directory("earlier");
        fs::write(dir.join("1.png"), b"left behind").unwrap();
        prepared(&dir).unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        let mode = fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
        fs::remove_dir_all(dir).unwrap();
    }

    fn copy(at: u128, text: &str) -> Copy {
        Copy {
            at,
            id: 0,
            content: Content::Text(text.to_string()),
        }
    }

    #[test]
    fn text_is_numbered_from_the_count_pictures_take_theirs_from() {
        let dir = directory("numbered");
        let mut kept = History::new(10, Some(dir.clone()), 100);
        let one = kept.copied(text("one")).unwrap();
        let picture = kept.copied(picture(b"a picture")).unwrap();
        let two = kept.copied(text("two")).unwrap();
        assert_eq!((one.id, picture.id, two.id), (1, 2, 3));
        let Content::Picture(made) = &picture.content else {
            unreachable!()
        };
        assert_eq!(made.id, picture.id);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn text_copied_again_keeps_its_number() {
        let mut kept = history(10);
        let first = kept.copied(text("one")).unwrap();
        kept.copied(text("two"));
        let again = kept.copied(text("one")).unwrap();
        assert_eq!(again.id, first.id);
        assert_eq!(kept.copied(text("three")).unwrap().id, 3);
    }

    #[test]
    fn text_chosen_moves_to_the_front_as_what_the_clipboard_holds() {
        let mut kept = history(10);
        let one = kept.copied(text("one")).unwrap();
        kept.copied(text("two"));
        let chosen = kept.choose(one.id).unwrap();
        assert_eq!(chosen.content, one.content);
        assert_eq!(kept.copies[0].content, one.content);
        assert_eq!(kept.current, Some(one.content));
    }

    #[test]
    fn the_shell_asks_for_a_copy_by_its_number_or_its_text() {
        assert_eq!(
            wanted(&json!({ "ask": "copy", "copy": 12, "id": "ask-1" })),
            Some(Wanted::Copy(Which::Kept(12)))
        );
        assert_eq!(
            wanted(&json!({ "ask": "copy", "text": "\u{1f600} \"hi\"\n" })),
            Some(Wanted::Copy(Which::Text("\u{1f600} \"hi\"\n".to_string())))
        );
        assert_eq!(wanted(&json!({ "ask": "clear" })), Some(Wanted::Clear));
    }

    #[test]
    fn the_shell_asks_for_a_paste_with_the_keys_its_window_takes() {
        assert_eq!(
            wanted(&json!({ "ask": "paste", "copy": 3, "into": "terminal" })),
            Some(Wanted::Paste(Which::Kept(3), Some(Keys::Terminal)))
        );
        assert_eq!(
            wanted(&json!({ "ask": "paste", "text": "a b", "into": "plain" })),
            Some(Wanted::Paste(
                Which::Text("a b".to_string()),
                Some(Keys::Plain)
            ))
        );
    }

    #[test]
    fn a_paste_that_names_no_keys_takes_its_windows() {
        assert_eq!(
            wanted(&json!({ "ask": "paste", "copy": 3 })),
            Some(Wanted::Paste(Which::Kept(3), None))
        );
    }

    #[test]
    fn the_window_niri_says_is_focused_says_its_keys() {
        let window = |app: &str| {
            format!("{{\"Ok\":{{\"FocusedWindow\":{{\"id\":7,\"app_id\":\"{app}\"}}}}}}")
        };
        assert_eq!(
            focused(&window("foot")),
            Some(Focus::Window(Keys::Terminal))
        );
        assert_eq!(
            focused(&window("com.mitchellh.ghostty")),
            Some(Focus::Window(Keys::Terminal))
        );
        assert_eq!(
            focused(&window("firefox")),
            Some(Focus::Window(Keys::Plain))
        );
        assert_eq!(
            focused("{\"Ok\":{\"FocusedWindow\":null}}"),
            Some(Focus::Nowhere)
        );
        assert_eq!(focused("{\"Ok\":{\"Layers\":[]}}"), None);
        assert_eq!(under(None), Some(Keys::Plain));
    }

    #[test]
    fn an_ask_the_shell_never_sends_wants_nothing() {
        for said in [
            json!({ "ask": "copy" }),
            json!({ "ask": "copy", "copy": "x" }),
            json!({ "ask": "copy", "id": 3 }),
            json!({ "ask": "paste", "copy": 3, "into": "loud" }),
            json!({ "ask": "paste", "copy": 3, "into": 1 }),
            json!({ "ask": "press" }),
            json!({}),
        ] {
            assert_eq!(wanted(&said), None, "{said}");
        }
    }

    #[test]
    fn an_ask_is_handed_to_the_wayland_side_and_answered_as_it_answers() {
        let (asked, handed) = mpsc::channel();
        let (mut woken, wake) = io::pipe().unwrap();
        let own = own(asked, wake);
        let answering = thread::spawn(move || {
            let Ok(Asked::Wanted(wanted, reply)) = handed.recv() else {
                panic!("nothing was handed over");
            };
            assert_eq!(wanted, Wanted::Copy(Which::Kept(3)));
            reply.send(json!({ "copied": true })).unwrap();
        });
        let answer = own(&json!({ "ask": "copy", "copy": 3 }));
        answering.join().unwrap();
        assert_eq!(answer, Some(json!({ "copied": true })));
        let mut wakes = [0; 1];
        woken.read_exact(&mut wakes).unwrap();
        assert_eq!(&wakes, b"!");
        // One it doesn't know is the interface's to refuse.
        assert_eq!(own(&json!({ "ask": "press" })), None);
    }

    #[test]
    fn an_ask_with_no_wayland_side_is_refused_as_not_answering() {
        let (asked, handed) = mpsc::channel();
        let (_woken, wake) = io::pipe().unwrap();
        drop(handed);
        let answer = own(asked, wake)(&json!({ "ask": "clear" })).unwrap();
        assert_eq!(answer["error"]["code"], "not-answering");
    }

    #[test]
    fn a_paste_says_how_long_the_launcher_and_the_keys_took() {
        assert_eq!(
            pasted(Duration::from_micros(114_040), Duration::from_micros(8_060)),
            "Pasted: the launcher let go of the keys 114.0 ms after the ask, and they were \
             pressed 8.1 ms after that."
        );
    }

    #[test]
    fn the_keys_are_let_go_once_the_launchers_layer_takes_none() {
        let layers = |keys: &str| {
            format!(
                "{{\"Ok\":{{\"Layers\":[{{\"namespace\":\"launcher\",\"output\":\"DP-4\",\
                 \"layer\":\"Overlay\",\"keyboard_interactivity\":\"{keys}\"}},\
                 {{\"namespace\":\"notices\",\"keyboard_interactivity\":\"OnDemand\"}}]}}}}"
            )
        };
        assert_eq!(let_go(&layers("None")), Some(true));
        assert_eq!(let_go(&layers("OnDemand")), Some(false));
        assert_eq!(let_go(&layers("Exclusive")), Some(false));
        assert_eq!(let_go("{\"Ok\":{\"Layers\":[]}}"), Some(true));
        assert_eq!(let_go("{\"Err\":\"no\"}"), None);
    }

    #[test]
    fn a_terminal_takes_shift_besides() {
        assert_eq!(Keys::Plain.modifiers(), CONTROL);
        assert_eq!(Keys::Terminal.modifiers(), CONTROL | SHIFT);
    }

    #[test]
    fn a_copy_made_again_moves_to_the_front() {
        let mut kept = history(10);
        kept.add(copy(1, "one"));
        kept.add(copy(2, "two"));
        kept.add(copy(3, "one"));
        assert_eq!(kept.copies, [copy(3, "one"), copy(2, "two")]);
    }

    #[test]
    fn the_oldest_goes_first() {
        let mut kept = history(2);
        for (at, text) in ["one", "two", "three"].into_iter().enumerate() {
            kept.add(copy(at as u128, text));
        }
        assert_eq!(kept.copies, [copy(2, "three"), copy(1, "two")]);
    }

    #[test]
    fn a_blank_copy_is_not_kept() {
        let mut kept = history(10);
        assert!(!kept.add(copy(1, " \n\t")));
        assert!(kept.copies.is_empty());
    }

    /// What the clipboard holds, as it's served.
    fn holding(types: &[&str], text: &str) -> Value {
        json!({ "key": "now:clipboard", "kind": "now", "now": { "types": types, "text": text } })
    }

    #[test]
    fn a_copy_is_said_by_its_types_and_its_text() {
        let said = offered(&types(&["text/plain", "image/png"]), text("a\nb").as_ref());
        assert_eq!(said, holding(&["text/plain", "image/png"], "a\nb"));
    }

    #[test]
    fn a_copy_read_as_no_text_says_none() {
        let said = offered(&types(&["image/png"]), picture(b"a picture").as_ref());
        assert_eq!(said, holding(&["image/png"], ""));
        // Text too large to read, or a program that never handed it over.
        assert_eq!(
            offered(&types(&["text/plain"]), None),
            holding(&["text/plain"], "")
        );
    }

    #[test]
    fn a_secret_is_said_as_nothing() {
        let said = offered(&types(&["text/plain", SECRET]), text("hunter2").as_ref());
        assert_eq!(said, holding(&[], ""));
    }

    #[test]
    fn what_the_clipboard_holds_changes_once_each_time_it_changes() {
        let mut history = history(5);
        let one = offered(&types(&["text/plain"]), text("one").as_ref());
        history.holds(one.clone());
        history.holds(one);
        assert_eq!(history.changes, 1);
        history.holds(offered(&types(&["text/plain"]), text("two").as_ref()));
        assert_eq!(history.changes, 2);
        assert_eq!(history.now, holding(&["text/plain"], "two"));
    }

    #[test]
    fn the_copy_it_gives_back_is_said_as_it_offers_it() {
        let mut history = history(5);
        history.copied(text("one"));
        history.holds_current();
        assert_eq!(history.now, holding(&TEXT, "one"));
    }

    #[test]
    fn forgetting_leaves_the_clipboard_holding_nothing() {
        let mut history = history(5);
        history.copied(text("one"));
        history.holds_current();
        history.forget();
        assert_eq!(history.now, holding(&[], ""));
        assert_eq!(keys(&history), Vec::<String>::new());
    }

    fn served() -> Served {
        Served {
            shared: Shared::new(DOMAIN, Box::new(|_| None)),
            kept: Kept::default(),
            words: words(),
            changes: None,
        }
    }

    #[test]
    fn the_copies_are_served_newest_first_beside_their_count_and_the_clipboard() {
        let mut served = served();
        let mut history = history(5);
        served.send(&history);
        assert_eq!(
            served.kept.get("summary:copies").unwrap()["summary"]["said"],
            "empty"
        );
        assert_eq!(
            served.kept.get("now:clipboard").unwrap()["now"]["types"],
            json!([])
        );
        history.copied(text("one"));
        history.copied(text("two"));
        history.holds_current();
        served.send(&history);
        let order = |key: &str| served.kept.get(key).unwrap()["row"]["order"].as_i64();
        assert!(
            order("row:copies:2") < order("row:copies:1"),
            "the newest copy is served first"
        );
        assert_eq!(
            served.kept.get("summary:copies").unwrap()["summary"]["said"],
            "2 copies"
        );
        assert_eq!(
            served.kept.get("now:clipboard").unwrap()["now"]["text"],
            "two"
        );
    }

    #[test]
    fn nothing_is_made_again_while_the_history_stands() {
        let mut served = served();
        let mut history = history(5);
        history.copied(text("one"));
        served.send(&history);
        // Made again only as the history changes, which these rows outlast.
        served.kept = Kept::default();
        served.send(&history);
        assert_eq!(served.kept.all().count(), 0);
    }

    #[test]
    fn keeping_none_keeps_nothing_to_give_back() {
        let mut history = history(0);
        assert_eq!(history.copied(text("one")), None);
        assert_eq!(history.copies.len(), 0);
        assert_eq!(history.current, None);
    }

    #[test]
    fn a_texts_row_holds_it_as_it_was_copied() {
        let row = copy(5, "say \"hi\"\n\\").row(&words());
        assert_eq!(row["key"], "row:copies:0");
        assert_eq!(row["held"]["text"], "say \"hi\"\n\\");
        assert_eq!(row["side"][1], json!({ "at": 5, "as": "came" }));
    }

    #[test]
    fn a_pictures_row_holds_where_it_is_and_its_size() {
        let row = Copy {
            at: 5,
            id: 3,
            content: Content::Picture(Picture {
                id: 3,
                kind: "image/png",
                path: PathBuf::from("/run/user/1000/pictures/3.png"),
                bytes: 42,
                hash: 0,
                size: Some((640, 480)),
            }),
        }
        .row(&words());
        assert_eq!(row["key"], "row:copies:3");
        assert_eq!(
            row["held"],
            json!({ "id": 3, "picture": "/run/user/1000/pictures/3.png",
                    "across": 640, "down": 480 })
        );
        assert_eq!(row["side"][1]["said"], "42 B");
    }

    #[test]
    fn a_png_says_its_size() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend(640u32.to_be_bytes());
        png.extend(480u32.to_be_bytes());
        assert_eq!(dimensions("image/png", &png), Some((640, 480)));
        assert_eq!(dimensions("image/png", &png[..20]), None);
    }

    #[test]
    fn a_gif_says_its_size() {
        let gif = [b"GIF89a".as_slice(), &[0x80, 0x02, 0xe0, 0x01]].concat();
        assert_eq!(dimensions("image/gif", &gif), Some((640, 480)));
    }

    #[test]
    fn a_jpeg_says_its_size_past_its_other_segments() {
        let jpeg = [
            [0xff, 0xd8].as_slice(),
            // An application segment of four bytes, then a Huffman table,
            // which shares the start of frame's range.
            &[0xff, 0xe0, 0x00, 0x04, 0x00, 0x00],
            &[0xff, 0xc4, 0x00, 0x03, 0x00],
            &[0xff, 0xc0, 0x00, 0x11, 0x08, 0x01, 0xe0, 0x02, 0x80],
        ]
        .concat();
        assert_eq!(dimensions("image/jpeg", &jpeg), Some((640, 480)));
        assert_eq!(dimensions("image/jpeg", &jpeg[..12]), None);
    }

    fn webp(chunk: &[u8], body: &[u8]) -> Vec<u8> {
        [b"RIFF\0\0\0\0WEBP".as_slice(), chunk, &[0; 4], body].concat()
    }

    #[test]
    fn a_webp_says_its_size_whichever_way_it_was_made() {
        let extended = webp(b"VP8X", &[0, 0, 0, 0, 0x7f, 0x02, 0x00, 0xdf, 0x01, 0x00]);
        assert_eq!(dimensions("image/webp", &extended), Some((640, 480)));
        // The width and height less one, as WebP keeps them.
        let bits: u32 = (640 - 1) | ((480 - 1) << 14);
        let lossless = webp(b"VP8L", &[[0x2f].as_slice(), &bits.to_le_bytes()].concat());
        assert_eq!(dimensions("image/webp", &lossless), Some((640, 480)));
        let lossy = webp(
            b"VP8 ",
            &[0, 0, 0, 0x9d, 0x01, 0x2a, 0x80, 0x02, 0xe0, 0x01],
        );
        assert_eq!(dimensions("image/webp", &lossy), Some((640, 480)));
    }

    #[test]
    fn a_picture_whose_header_says_nothing_has_no_size() {
        assert_eq!(dimensions("image/png", b"not a picture"), None);
        assert_eq!(dimensions("image/tiff", b"II*\0"), None);
    }
}
