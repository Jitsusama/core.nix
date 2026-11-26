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
//! With `--emoji`, naming the launcher's list of them (launcher/emoji.nix),
//! it serves Emoji's rows too, `row:emoji:<N>` in the list's order, each
//! inside its group's row (src/emoji.rs), since it's what pastes them, so
//! the shell reads no list of its own, and answers what's typed in Emoji
//! with the emoji it finds (src/search.rs), so the shell matches none.
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
//! copy N back on the clipboard (not `id`, which names the ask itself), or
//! with `"text"` in place of the number to put the shell's own text there,
//! kept as a copy is. `"paste"`, with
//! `"into": "terminal"` or `"plain"`, puts it there and then presses the
//! paste keys for the window the launcher closed over: Ctrl+Shift+V for a
//! terminal, whose Ctrl+V is a key of its own, and Ctrl+V anywhere else.
//! Without `"into"`, they're the keys of the focused workspace's active
//! window as the paste is asked, and with none, the copy
//! is only put on the clipboard, with nowhere to paste it.
//! They're pressed through the compositor's virtual keyboard, once niri
//! says the launcher's layer has let go of the keys, so they reach the
//! window and not the launcher, and nothing waits on the shell's thread:
//! the window reads the copy from here, too. A paste is answered once its
//! keys are pressed, or refused as `unpasted` once they couldn't be.
//! `"clear"` forgets everything.
//!
//! The selection a middle click pastes, the text highlighted now, is never
//! kept, but a get of `selection:primary` reads it as it's asked, so the
//! launcher spawns nothing to act on it: `{"key", "kind": "selection",
//! "text", "at"}`, its text cut at `SELECTED` bytes, and empty with
//! nothing highlighted or a secret, `at` when it was highlighted, in ms.
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
//! It takes its words as the terminal kit does: `clipboard-history --help`
//! says them (src/commands.rs), and a failure is the kit's, said once as
//! it ends.
//!
//! `--vocabulary` is the launcher's words.json. Its numbers are grouped and pointed
//! as the launcher's counts are, with the comma and the full stop.

mod commands;
mod emoji;
mod history;
mod paste;
mod rows;
mod search;
mod wayland;

use std::collections::HashMap;
use std::fs;
use std::io::{self, PipeReader, PipeWriter, Read, Write};
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, thread};

use commands::Settings;
use terminal_kit::{Exit, Failure};

use crate::history::{Content, Heard, History, Served, prepared};
use crate::paste::{Keys, keys_let_go, pasted, under, virtual_keyboard};
use crate::wayland::{asked_for, read_in_time};
use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Read as Reading, Shared};
use ready_rows::kept::Kept;
use ready_rows::numbers::Numbers;
use rows::Words;
use rustix::event::{PollFd, PollFlags, poll};
use serde_json::{Value, json};
use wayland_client::backend::{ObjectId, WaylandError};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::ExtDataControlDeviceV1,
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::ExtDataControlOfferV1,
    ext_data_control_source_v1::ExtDataControlSourceV1,
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

/// The key the highlighted text is got by.
const PRIMARY: &str = "selection:primary";

/// The most of the highlighted text a get gives, in bytes, as the
/// launcher's AI asks of it.
const SELECTED: usize = 64 << 10;

/// V, in the keymap above, as evdev numbers it.
const V: u32 = 1;

/// The modifiers' masks in the keymap above.
const SHIFT: u32 = 1;
const CONTROL: u32 = 4;

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
    Selection(Reply),
}

/// How long an ask waits on the Wayland side before it's answered as
/// not answering: past a paste's whole wait for the keys.
const ANSWERING: Duration = Duration::from_secs(3);

/// An ask refused, in the interface's form, its words the launcher's.
fn refused(code: &str, message: &str, said: &str) -> Value {
    json!({ "error": { "code": code, "message": message, "said": said } })
}

/// The highlighted text as a get is answered it.
fn selection(text: &str, at: u128) -> Value {
    json!({ "key": PRIMARY, "kind": "selection", "text": text, "at": at })
}

/// The highlighted text read, cut at `SELECTED` bytes: a character cut
/// there is left out, and text that isn't UTF-8 read as near as it can be.
fn selected(mut bytes: Vec<u8>) -> String {
    bytes.truncate(SELECTED);
    match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) if e.utf8_error().error_len().is_none() => {
            let whole = e.utf8_error().valid_up_to();
            let mut bytes = e.into_bytes();
            bytes.truncate(whole);
            String::from_utf8(bytes).unwrap_or_default()
        }
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }
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

/// What the Wayland side needs: the history and what's served of it, the
/// types each offer has said it can be read as until its selection
/// arrives, the largest copies it reads, while a copy is offered in its
/// program's place, that offer, the keyboard a paste's keys are pressed
/// through, and how a paste waiting on the launcher to let go of the keys
/// says it may press them, and the highlighted text's offer, read only as
/// a get asks for it, and since when.
struct State {
    connection: Connection,
    primary: Option<(ExtDataControlOfferV1, Vec<String>)>,
    primary_at: u128,
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
    // The compositor took the clipboard away, which ends the program.
    taken: bool,
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
            // Asked before the launcher lets go, since an ask after waits
            // behind niri drawing, which held a paste's keys 119 ms on a
            // builder (2026-11-11). The workspace's active window is asked,
            // since niri says no window is focused while the launcher holds
            // the keys, which left a paste into kitty with nowhere to go.
            let Some(keys) = keys.or_else(|| under(niri.as_deref())) else {
                let _ = reply.send(json!({ "copied": true }));
                return;
            };
            let said = if keys_let_go(niri.as_deref()) {
                let let_go = Instant::now();
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

    /// Does what was asked of the clipboard's thread: what the shell
    /// wanted, or a paste's keys pressed once the launcher let go of them.
    fn asked(&mut self, asked: Asked, handle: &QueueHandle<Self>) {
        match asked {
            Asked::Wanted(wanted, reply) => self.wanted(wanted, reply, handle),
            Asked::Press(keys, letting_go, let_go, reply) => {
                let answer = if self.press(keys) {
                    eprintln!("{}", pasted(letting_go, let_go.elapsed()));
                    json!({ "pasted": true })
                } else {
                    self.unpasted()
                };
                let _ = reply.send(answer);
            }
            Asked::Unpasted(reply) => {
                let _ = reply.send(self.unpasted());
            }
            Asked::Selection(reply) => self.selection(reply),
        }
    }

    /// Reads the highlighted text from its program, away from the Wayland
    /// side, and answers it: empty with none, or none as text, or a secret.
    fn selection(&self, reply: Reply) {
        let at = self.primary_at;
        let asked = self.primary.as_ref().and_then(|(offer, types)| {
            let Some(Readable::Text(kind)) = readable(types, false) else {
                return None;
            };
            asked_for(&self.connection, offer, kind)
        });
        let Some(reader) = asked else {
            let _ = reply.send(selection("", at));
            return;
        };
        thread::spawn(move || {
            let text = read_in_time(reader, SELECTED).map_or_else(String::new, selected);
            let _ = reply.send(selection(&text, at));
        });
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

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// What the interface names this daemon, and the kinds of record it holds.
const DOMAIN: Domain = Domain {
    name: "clipboard",
    kinds: &["row", "summary", "now", "selection"],
    sole: &[],
    named_only: &["selection"],
};

/// The highlighted text, read on the Wayland side as a get asks for it,
/// and answered as it answers, on the asking connection's thread.
fn read(asked: mpsc::Sender<Asked>, wake: PipeWriter) -> Reading {
    let wake = Mutex::new(wake);
    Box::new(move |key| {
        if key != PRIMARY {
            return None;
        }
        let (reply, answer) = mpsc::channel();
        let handed = asked.send(Asked::Selection(reply)).is_ok()
            && wake
                .lock()
                .is_ok_and(|mut wake| wake.write_all(b"!").is_ok());
        if !handed {
            return None;
        }
        answer.recv_timeout(ANSWERING).ok()
    })
}

/// The shell's own asks, handed to the Wayland side, which is woken to do
/// them, and answered as it answers, on the asking connection's thread. A
/// search needs nothing of Wayland's, so it's answered on that thread
/// from the emoji's `index`, at once.
fn own(asked: mpsc::Sender<Asked>, wake: PipeWriter, index: search::Index) -> Own {
    let wake = Mutex::new(wake);
    Box::new(move |said| {
        if said["ask"] == "search" {
            return Some(index.answer(said));
        }
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

/// Emoji's rows, from the list `--emoji` names, or none without one,
/// their groups' counts grouped with the comma (`Words::figure` over
/// `Numbers::default()`), since this daemon is given no account's number marks.
fn emoji_rows(list: Option<&Path>, words: &Words) -> Result<Vec<Value>, String> {
    list.map_or_else(
        || Ok(Vec::new()),
        |list| {
            fs::read_to_string(list)
                .map(|text| emoji::records(&text, words))
                .map_err(|e| format!("can't read {}: {e}", list.display()))
        },
    )
}

fn lost(error: impl std::fmt::Display) -> String {
    format!("lost Wayland: {error}")
}

fn run(settings: &Settings, words: Words, emoji: Vec<Value>) -> Result<(), String> {
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
    let (selecting, waking_to_read) = (
        asked.clone(),
        wake.try_clone().map_err(|e| format!("no pipe: {e}"))?,
    );
    let shared = Shared::new(DOMAIN, own(asked, wake, search::Index::of(&emoji)));
    shared.read_on_get(&["selection"], read(selecting, waking_to_read));
    let mut served = Served {
        shared: Arc::clone(&shared),
        kept: Kept::default(),
        words,
        emoji,
        changes: None,
    };
    let history = History::new(settings.keep, settings.pictures.clone(), settings.room);
    served.send(&history);
    serve::accept(shared, listener);

    let mut state = State {
        connection: connection.clone(),
        primary: None,
        primary_at: 0,
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
        taken: false,
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
        if state.taken {
            return Err("the compositor took the clipboard away".to_owned());
        }
        // Emptied, the clipboard holds the copy given back, or nothing.
        if std::mem::take(&mut state.emptied) {
            state.history.holds_current();
            if state.history.current.is_some() {
                state.hold(&handle);
            }
        }
        for asked in wanted.try_iter() {
            state.asked(asked, &handle);
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

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), |context| {
        let settings = commands::settings(&context.args)?;
        let unread = |why: String| {
            Failure::new(Exit::Config, "clipboard-history can't read what it says").because(why)
        };
        let words = worded(&settings.vocabulary).map_err(unread)?;
        let emoji = emoji_rows(settings.emoji.as_deref(), &words).map_err(unread)?;
        run(&settings, words, emoji).map_err(|why| {
            Failure::new(
                Exit::Failed,
                "clipboard-history stopped hearing the clipboard",
            )
            .because(why)
        })
    })
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
