//! The clipboard's history, for the launcher.
//!
//! A Wayland program only sees the clipboard while it holds the keys, so the
//! shell can't keep a history by itself. This one program holds the
//! compositor's data-control connection for the whole session instead, reads
//! each new copy, and keeps the latest. The shell connects to its socket and
//! is sent every copy kept, oldest first, then each new one as it comes: one
//! JSON object to a line, the time in milliseconds since the epoch in `at`.
//! A text copy is `{"at", "id", "text"}`, its number in `id`. A picture is
//! `{"at", "picture", "type", "path", "bytes", "width", "height"}`: its
//! number, its type, the file holding it, its size, and its width and height
//! when its type says them. Text and pictures are numbered from one count,
//! and a copy made again keeps its number. Nothing is started per copy.
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
//! pictures kept take up at most `--room`, the oldest going first, and the
//! shells are sent `{"gone":[N, ...]}` for those let go, to make room or
//! past what it keeps, since their files go with them.
//!
//! From the shell, a line `copy N` puts copy N back on the clipboard, and
//! `copy "text"`, the text spelled as JSON, puts the shell's own text
//! there, kept as a copy is. `paste N terminal` or `paste N plain`, or the
//! same with the text in place of N, puts it there and then presses the
//! paste keys for the window the launcher closed over: Ctrl+Shift+V for a
//! terminal, whose Ctrl+V is a key of its own, and Ctrl+V anywhere else.
//! They're pressed through the compositor's virtual keyboard, once niri
//! says the launcher's layer has let go of the keys, so they reach the
//! window and not the launcher, and nothing waits on the shell's thread:
//! the window reads the copy from here, too. `clear` forgets everything,
//! every client then being sent `{"clear":true}`.
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
//! Usage: clipboard-history --socket PATH [--keep N] [--largest BYTES]
//!        [--pictures DIR] [--largest-picture BYTES] [--room BYTES]

use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, BufRead, BufReader, PipeReader, PipeWriter, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, process, thread};

use rustix::event::{PollFd, PollFlags, poll};
use rustix::fs::{MemfdFlags, memfd_create};
use serde_json::Value;
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

/// What the shell asks for, or the paste keys, once the launcher has let
/// go of them: how long that took after the ask, and when it did.
#[derive(Debug, PartialEq)]
enum Asked {
    Clear,
    Copy(Which),
    Paste(Which, Keys),
    Press(Keys, Duration, Instant),
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

/// A line from the shell, as what it asks, or none for one it doesn't
/// understand.
fn asked(line: &str) -> Option<Asked> {
    let line = line.trim();
    if line == "clear" {
        return Some(Asked::Clear);
    }
    if let Some(rest) = line.strip_prefix("copy ") {
        let (which, rest) = which(rest)?;
        return rest.trim().is_empty().then_some(Asked::Copy(which));
    }
    let (which, rest) = which(line.strip_prefix("paste ")?)?;
    let keys = match rest.trim() {
        "terminal" => Keys::Terminal,
        "plain" => Keys::Plain,
        _ => return None,
    };
    Some(Asked::Paste(which, keys))
}

/// The copy a line names, and what follows it: text spelled as JSON, or
/// else a number.
fn which(said: &str) -> Option<(Which, &str)> {
    let said = said.trim_start();
    if said.starts_with('"') {
        let mut strings = serde_json::Deserializer::from_str(said).into_iter::<String>();
        let text = strings.next()?.ok()?;
        return Some((Which::Text(text), &said[strings.byte_offset()..]));
    }
    let (number, rest) = said.split_once(' ').unwrap_or((said, ""));
    Some((Which::Kept(number.parse().ok()?), rest))
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

/// Asks niri whether the launcher holds no keys.
fn asked_niri(socket: &Path) -> Option<bool> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(LETTING_GO)).ok()?;
    stream.write_all(b"\"Layers\"\n").ok()?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).ok()?;
    let_go(&answer)
}

/// Waits until niri says the launcher has let go of the keys, or with no
/// niri to ask, a moment. Answers whether they're free to press. Once
/// is enough: niri gives the keys to the window as it ends the turn that
/// answered, before it reads the keys pressed after the answer, and asking
/// again waited behind niri drawing, 89 ms on a builder (2026-11-10).
fn keys_let_go(niri: Option<&Path>) -> bool {
    let Some(niri) = niri else {
        thread::sleep(UNASKED);
        return true;
    };
    let deadline = Instant::now() + LETTING_GO;
    while Instant::now() < deadline {
        if asked_niri(niri) == Some(true) {
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
    fn line(&self) -> String {
        match &self.content {
            Content::Text(text) => format!(
                "{{\"at\":{},\"id\":{},\"text\":{}}}\n",
                self.at,
                self.id,
                quoted(text)
            ),
            Content::Picture(picture) => {
                let size = picture
                    .size
                    .map(|(width, height)| format!(",\"width\":{width},\"height\":{height}"))
                    .unwrap_or_default();
                format!(
                    "{{\"at\":{},\"picture\":{},\"type\":{},\"path\":{},\"bytes\":{}{size}}}\n",
                    self.at,
                    picture.id,
                    quoted(picture.kind),
                    quoted(&picture.path.to_string_lossy()),
                    picture.bytes,
                )
            }
        }
    }
}

/// A string as JSON spells it.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => {
                // Writing to a String can't fail.
                _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
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

/// The copies kept, newest first, the shells listening for more, what the
/// clipboard holds now when it's a copy kept, where pictures go and how
/// much room they have, and the pictures let go since the shells were last
/// told.
struct History {
    copies: VecDeque<Copy>,
    keep: usize,
    clients: Vec<UnixStream>,
    current: Option<Content>,
    pictures: Option<PathBuf>,
    room: usize,
    next: u64,
    gone: Vec<u64>,
}

impl History {
    fn new(keep: usize, pictures: Option<PathBuf>, room: usize) -> History {
        History {
            copies: VecDeque::new(),
            keep,
            clients: Vec::new(),
            current: None,
            pictures,
            room,
            next: 1,
            gone: Vec::new(),
        }
    }

    /// The line telling the shells which pictures were let go to make room,
    /// `{"gone":[N, ...]}`, since they keep rows of their own; none when
    /// nothing went.
    fn went(&mut self) -> Option<String> {
        if self.gone.is_empty() {
            return None;
        }
        let ids: Vec<String> = self.gone.drain(..).map(|id| id.to_string()).collect();
        Some(format!("{{\"gone\":[{}]}}\n", ids.join(",")))
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
            if let Content::Picture(picture) = &copy.content {
                self.gone.push(picture.id);
            }
            discard(&copy.content);
        }
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
        Some(copy)
    }

    /// Forgets every copy, and what the clipboard holds with them.
    fn forget(&mut self) {
        for copy in self.copies.drain(..) {
            discard(&copy.content);
        }
        self.current = None;
    }

    /// Sends a line to every shell, dropping any that has gone.
    fn tell(&mut self, line: &str) {
        self.clients
            .retain_mut(|client| client.write_all(line.as_bytes()).is_ok());
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

/// What the Wayland side needs: the history, the types each offer has said
/// it can be read as until its selection arrives, the largest copies it
/// reads, while a copy is offered in its program's place, that offer, the
/// keyboard a paste's keys are pressed through, and how a paste waiting
/// on the launcher to let go of the keys says it may press them.
struct State {
    history: Arc<Mutex<History>>,
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
        let Some(types) = self
            .history
            .lock()
            .unwrap()
            .current
            .as_ref()
            .map(Content::types)
        else {
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
        let offered = {
            let mut history = self.history.lock().unwrap();
            let offered = match which {
                Which::Kept(id) => history.choose(id),
                Which::Text(text) => history.copied(Some(Heard::Text(text))),
            };
            if let Some(copy) = &offered {
                history.tell(&copy.line());
                if let Some(line) = history.went() {
                    history.tell(&line);
                }
            }
            offered.is_some()
        };
        if offered {
            self.hold(handle);
        }
        offered
    }

    /// Puts a copy on the clipboard, then waits, away from the Wayland
    /// side, for the launcher to let go of the keys, and asks for the paste
    /// keys to be pressed once it has.
    fn paste(&mut self, which: Which, keys: Keys, handle: &QueueHandle<Self>) {
        let asked = Instant::now();
        if !self.offer(which, handle) {
            return;
        }
        let (Ok(mut waking), niri) = (self.waking.try_clone(), self.niri.clone()) else {
            return;
        };
        let pressing = self.pressing.clone();
        thread::spawn(move || {
            if !keys_let_go(niri.as_deref()) {
                eprintln!("The launcher never let go of the keys, so nothing was pasted.");
                return;
            }
            let let_go = Instant::now();
            if pressing
                .send(Asked::Press(keys, let_go - asked, let_go))
                .is_ok()
            {
                let _ = waking.write_all(b"!");
            }
        });
    }

    /// Presses the paste keys through the virtual keyboard: its modifiers
    /// held, V pressed and let go, then the modifiers let go.
    fn press(&self, keys: Keys) {
        let Some(keyboard) = &self.keyboard else {
            eprintln!("The compositor has no virtual keyboard, so nothing was pasted.");
            return;
        };
        let time = u32::try_from(self.started.elapsed().as_millis()).unwrap_or(u32::MAX);
        keyboard.modifiers(keys.modifiers(), 0, 0, 0);
        keyboard.key(time, V, 1);
        keyboard.key(time, V, 0);
        keyboard.modifiers(0, 0, 0, 0);
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
                let content = state.history.lock().unwrap().current.clone();
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
                let mut history = state.history.lock().unwrap();
                if let Some(copy) = history.copied(heard) {
                    history.tell(&copy.line());
                    if let Some(line) = history.went() {
                        history.tell(&line);
                    }
                }
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

/// Sends each new shell every copy kept, then listens for what it asks:
/// `clear` here, and a copy or a paste passed to the Wayland side, woken to
/// do it.
fn serve(
    listener: &UnixListener,
    history: &Arc<Mutex<History>>,
    asked: &mpsc::Sender<Asked>,
    wake: &PipeWriter,
) {
    for client in listener.incoming().flatten() {
        let _ = client.set_write_timeout(Some(Duration::from_secs(1)));
        let (Ok(asking), Ok(mut waking)) = (client.try_clone(), wake.try_clone()) else {
            continue;
        };
        {
            let mut history = history.lock().unwrap();
            let mut client = client;
            let all: String = history.copies.iter().rev().map(Copy::line).collect();
            if client.write_all(all.as_bytes()).is_ok() {
                history.clients.push(client);
            }
        }
        let history = Arc::clone(history);
        let asked = asked.clone();
        thread::spawn(move || {
            for line in BufReader::new(asking).lines() {
                let Ok(line) = line else { break };
                match self::asked(&line) {
                    Some(Asked::Clear) => {
                        let mut history = history.lock().unwrap();
                        history.forget();
                        history.tell("{\"clear\":true}\n");
                    }
                    Some(wanted) => {
                        if asked.send(wanted).is_err() || waking.write_all(b"!").is_err() {
                            break;
                        }
                    }
                    None => {}
                }
            }
        });
    }
}

struct Settings {
    socket: String,
    keep: usize,
    largest: usize,
    pictures: Option<PathBuf>,
    largest_picture: usize,
    room: usize,
}

fn settings() -> Result<Settings, String> {
    let mut socket = None;
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
            "--socket" => socket = Some(value()?),
            "--keep" => keep = value()?.parse().map_err(|_| "--keep takes a count")?,
            "--largest" => largest = bytes(value()?, "--largest")?,
            "--pictures" => pictures = Some(PathBuf::from(value()?)),
            "--largest-picture" => largest_picture = bytes(value()?, "--largest-picture")?,
            "--room" => room = bytes(value()?, "--room")?,
            other => return Err(format!("{other} isn't something this understands")),
        }
    }
    let socket = socket.ok_or("--socket names where the shell connects")?;
    if largest_picture > room {
        return Err("--largest-picture can't be more than --room".to_string());
    }
    Ok(Settings {
        socket,
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

    // A socket left by an earlier run would refuse the bind.
    let _ = fs::remove_file(&settings.socket);
    let listener = UnixListener::bind(&settings.socket)
        .map_err(|e| format!("can't listen at {}: {e}", settings.socket))?;
    let history = Arc::new(Mutex::new(History::new(
        settings.keep,
        settings.pictures.clone(),
        settings.room,
    )));
    let (asked, wanted) = mpsc::channel();
    let (mut woken, wake) = io::pipe().map_err(|e| format!("no pipe: {e}"))?;
    let serving = Arc::clone(&history);
    let (pressing, waking) = (
        asked.clone(),
        wake.try_clone().map_err(|e| format!("no pipe: {e}"))?,
    );
    thread::spawn(move || serve(&listener, &serving, &asked, &wake));

    let mut state = State {
        history,
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
        if std::mem::take(&mut state.emptied) && state.history.lock().unwrap().current.is_some() {
            state.hold(&handle);
        }
        for wanted in wanted.try_iter() {
            match wanted {
                Asked::Copy(which) => {
                    state.offer(which, &handle);
                }
                Asked::Paste(which, keys) => state.paste(which, keys, &handle),
                Asked::Press(keys, letting_go, let_go) => {
                    state.press(keys);
                    eprintln!("{}", pasted(letting_go, let_go.elapsed()));
                }
                Asked::Clear => {}
            }
        }
        queue.flush().map_err(lost)?;
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
        assert!(!oldest.path.exists());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        // The shells are told, once, so their rows go with it.
        assert_eq!(kept.went(), Some(format!("{{\"gone\":[{}]}}\n", oldest.id)));
        assert_eq!(kept.went(), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_picture_past_what_it_keeps_is_told_as_gone() {
        let dir = directory("told");
        let mut kept = History::new(1, Some(dir.clone()), 100);
        let made = kept.copied(picture(b"a picture")).unwrap();
        let Content::Picture(made) = made.content else {
            unreachable!()
        };
        assert_eq!(kept.went(), None);
        kept.copied(text("one"));
        assert_eq!(kept.went(), Some(format!("{{\"gone\":[{}]}}\n", made.id)));
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
        assert_eq!(asked("copy 12"), Some(Asked::Copy(Which::Kept(12))));
        assert_eq!(
            asked("copy \"\\ud83d\\ude00 \\\"hi\\\"\\n\""),
            Some(Asked::Copy(Which::Text("\u{1f600} \"hi\"\n".to_string())))
        );
        assert_eq!(asked("clear\n"), Some(Asked::Clear));
    }

    #[test]
    fn the_shell_asks_for_a_paste_with_the_keys_its_window_takes() {
        assert_eq!(
            asked("paste 3 terminal"),
            Some(Asked::Paste(Which::Kept(3), Keys::Terminal))
        );
        assert_eq!(
            asked("paste \"a b\" plain"),
            Some(Asked::Paste(Which::Text("a b".to_string()), Keys::Plain))
        );
    }

    #[test]
    fn a_line_the_shell_never_sends_asks_nothing() {
        for line in [
            "copy",
            "copy x",
            "copy 3 terminal",
            "paste 3",
            "paste 3 loud",
            "paste \"open",
            "Press",
        ] {
            assert_eq!(asked(line), None, "{line}");
        }
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

    #[test]
    fn a_line_is_json() {
        let line = copy(5, "say \"hi\"\n\\ \u{1}").line();
        assert_eq!(
            line,
            "{\"at\":5,\"id\":0,\"text\":\"say \\\"hi\\\"\\n\\\\ \\u0001\"}\n"
        );
    }

    #[test]
    fn a_pictures_line_says_where_it_is_and_its_size() {
        let line = Copy {
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
        .line();
        assert_eq!(
            line,
            "{\"at\":5,\"picture\":3,\"type\":\"image/png\",\
             \"path\":\"/run/user/1000/pictures/3.png\",\"bytes\":42,\
             \"width\":640,\"height\":480}\n"
        );
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
