//! The clipboard's history, for the launcher.
//!
//! A Wayland program only sees the clipboard while it holds the keys, so the
//! shell can't keep a history by itself. This one program holds the
//! compositor's data-control connection for the whole session instead, reads
//! each new text copy, and keeps the latest in memory, never on disk. The
//! shell connects to its socket and is sent every copy kept, oldest first,
//! then each new one as it comes: one JSON object to a line, `{"at", "text"}`,
//! the time in milliseconds since the epoch. Nothing is started per copy.
//!
//! A copy a password manager marks as secret is never kept, nor anything
//! that isn't text or is larger than `--largest`. A line `clear` from the
//! shell forgets everything, and every client is sent `{"clear":true}`.
//!
//! A copy outlives the program that made it: when the clipboard empties
//! because that program let go of it, as one does when it closes, this one
//! offers the same text in its place, if it kept it. A secret, or anything
//! else it didn't keep, is let go with its program, and once the history is
//! forgotten, what it offers is nothing. Offering it takes the clipboard
//! from whatever holds it, so it waits until every event already sent has
//! been read and the compositor has answered a round trip: a copy made
//! since the clipboard emptied, which the compositor has taken, is then
//! known, and kept rather than replaced.
//!
//! Usage: clipboard-history --socket PATH [--keep N] [--largest BYTES]

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::AsFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, process, thread};

use wayland_client::backend::ObjectId;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child};
use wayland_protocols::ext::data_control::v1::client::{
    ext_data_control_device_v1::{self, ExtDataControlDeviceV1},
    ext_data_control_manager_v1::ExtDataControlManagerV1,
    ext_data_control_offer_v1::{self, ExtDataControlOfferV1},
    ext_data_control_source_v1::{self, ExtDataControlSourceV1},
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

/// A type only this program's own offer has, so it knows the clipboard it
/// set from one a program copied.
const OURS: &str = "application/x-clipboard-history-kept";

/// How long a program has to hand over what it copied before it's skipped.
const PATIENCE: Duration = Duration::from_secs(2);

/// The type to read an offer as, or none for one that's secret or not text.
fn readable(types: &[String]) -> Option<&'static str> {
    if types.iter().any(|t| t == SECRET || t == OURS) {
        return None;
    }
    TEXT.into_iter()
        .find(|wanted| types.iter().any(|t| t == wanted))
}

/// One kept copy.
#[derive(Clone, Debug, PartialEq)]
struct Copy {
    at: u128,
    text: String,
}

impl Copy {
    fn line(&self) -> String {
        format!("{{\"at\":{},\"text\":{}}}\n", self.at, quoted(&self.text))
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
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The copies kept, newest first, the shells listening for more, and what
/// the clipboard holds now when it's a copy kept.
struct History {
    copies: VecDeque<Copy>,
    keep: usize,
    clients: Vec<UnixStream>,
    current: Option<String>,
}

impl History {
    /// Keeps a copy, unless it's blank. One copied again moves to the front
    /// rather than being kept twice. Answers whether it was kept.
    fn add(&mut self, copy: Copy) -> bool {
        if copy.text.trim().is_empty() {
            return false;
        }
        self.copies.retain(|kept| kept.text != copy.text);
        self.copies.push_front(copy);
        self.copies.truncate(self.keep);
        true
    }

    /// What the clipboard now holds: a copy, kept if it can be, or for one
    /// that can't, nothing to give back once its program lets go. Answers
    /// the copy when it was kept.
    fn copied(&mut self, text: Option<String>) -> Option<Copy> {
        self.current = None;
        let copy = Copy {
            at: now(),
            text: text?,
        };
        if !self.add(copy.clone()) {
            return None;
        }
        self.current = Some(copy.text.clone());
        Some(copy)
    }

    /// Forgets every copy, and what the clipboard holds with them.
    fn forget(&mut self) {
        self.copies.clear();
        self.current = None;
    }

    /// Sends a line to every shell, dropping any that has gone.
    fn tell(&mut self, line: &str) {
        self.clients
            .retain_mut(|client| client.write_all(line.as_bytes()).is_ok());
    }
}

/// What the Wayland side needs: the history, the types each offer has said
/// it can be read as until its selection arrives, and while a copy is
/// offered in its program's place, that offer.
struct State {
    history: Arc<Mutex<History>>,
    offers: HashMap<ObjectId, Vec<String>>,
    largest: usize,
    manager: ExtDataControlManagerV1,
    device: ExtDataControlDeviceV1,
    held: Option<ExtDataControlSourceV1>,
    // The clipboard emptied, and nothing has been copied since.
    emptied: bool,
}

impl State {
    /// Offers the copy the clipboard held, in place of the program that let
    /// go of it.
    fn hold(&mut self, handle: &QueueHandle<Self>) {
        let source = self.manager.create_data_source(handle, ());
        for kind in TEXT {
            source.offer(kind.to_string());
        }
        source.offer(OURS.to_string());
        self.device.set_selection(Some(&source));
        if let Some(before) = self.held.replace(source) {
            before.destroy();
        }
    }
}

impl Dispatch<ExtDataControlSourceV1, ()> for State {
    fn event(
        state: &mut Self,
        source: &ExtDataControlSourceV1,
        event: ext_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            // Read when asked, so a history forgotten since gives nothing.
            ext_data_control_source_v1::Event::Send { fd, .. } => {
                let text = state.history.lock().unwrap().current.clone();
                thread::spawn(move || {
                    let mut file = File::from(fd);
                    let _ = file.write_all(text.unwrap_or_default().as_bytes());
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
        _: &(),
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
        _: &(),
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
        _: &(),
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
        _: &(),
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
                let text = readable(&types)
                    .and_then(|kind| receive(connection, &offer, kind, state.largest));
                offer.destroy();
                let mut history = state.history.lock().unwrap();
                if let Some(copy) = history.copied(text) {
                    history.tell(&copy.line());
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

/// Reads an offer as a type, or nothing when it's too large, isn't UTF-8,
/// or its program doesn't hand it over in time.
fn receive(
    connection: &Connection,
    offer: &ExtDataControlOfferV1,
    kind: &str,
    largest: usize,
) -> Option<String> {
    let (reader, writer) = std::io::pipe().ok()?;
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
    if bytes.len() > largest {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis())
}

/// Sends each new shell every copy kept, then listens for what it asks.
fn serve(listener: UnixListener, history: Arc<Mutex<History>>) {
    for client in listener.incoming().flatten() {
        let _ = client.set_write_timeout(Some(Duration::from_secs(1)));
        let Ok(asking) = client.try_clone() else {
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
        let history = Arc::clone(&history);
        thread::spawn(move || {
            for line in BufReader::new(asking).lines() {
                let Ok(line) = line else { break };
                if line.trim() == "clear" {
                    let mut history = history.lock().unwrap();
                    history.forget();
                    history.tell("{\"clear\":true}\n");
                }
            }
        });
    }
}

struct Settings {
    socket: String,
    keep: usize,
    largest: usize,
}

fn settings() -> Result<Settings, String> {
    let mut socket = None;
    let mut keep = 200;
    let mut largest = 1 << 20;
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "--socket" => socket = Some(value()?),
            "--keep" => keep = value()?.parse().map_err(|_| "--keep takes a count")?,
            "--largest" => largest = value()?.parse().map_err(|_| "--largest takes bytes")?,
            other => return Err(format!("{other} isn't something this understands")),
        }
    }
    let socket = socket.ok_or("--socket names where the shell connects")?;
    Ok(Settings {
        socket,
        keep,
        largest,
    })
}

fn run() -> Result<(), String> {
    let settings = settings()?;
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

    // A socket left by an earlier run would refuse the bind.
    let _ = fs::remove_file(&settings.socket);
    let listener = UnixListener::bind(&settings.socket)
        .map_err(|e| format!("can't listen at {}: {e}", settings.socket))?;
    let history = Arc::new(Mutex::new(History {
        copies: VecDeque::new(),
        keep: settings.keep,
        clients: Vec::new(),
        current: None,
    }));
    let serving = Arc::clone(&history);
    thread::spawn(move || serve(listener, serving));

    let mut state = State {
        history,
        offers: HashMap::new(),
        largest: settings.largest,
        manager,
        device,
        held: None,
        emptied: false,
    };
    loop {
        queue
            .blocking_dispatch(&mut state)
            .map_err(|e| format!("lost Wayland: {e}"))?;
        if state.emptied {
            queue
                .roundtrip(&mut state)
                .map_err(|e| format!("lost Wayland: {e}"))?;
        }
        if std::mem::take(&mut state.emptied) && state.history.lock().unwrap().current.is_some() {
            state.hold(&handle);
        }
    }
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
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn a_secret_is_never_read() {
        let offered = types(&["text/plain;charset=utf-8", SECRET]);
        assert_eq!(readable(&offered), None);
    }

    #[test]
    fn text_is_read_as_utf8_first() {
        let offered = types(&["TEXT", "text/plain", "text/plain;charset=utf-8"]);
        assert_eq!(readable(&offered), Some("text/plain;charset=utf-8"));
    }

    #[test]
    fn a_picture_is_not_read() {
        assert_eq!(readable(&types(&["image/png"])), None);
    }

    fn history(keep: usize) -> History {
        History {
            copies: VecDeque::new(),
            keep,
            clients: Vec::new(),
            current: None,
        }
    }

    #[test]
    fn its_own_offer_is_never_read() {
        assert_eq!(readable(&types(&["text/plain", OURS])), None);
    }

    #[test]
    fn a_kept_copy_is_what_the_clipboard_holds() {
        let mut kept = history(10);
        kept.copied(Some("one".to_string()));
        assert_eq!(kept.current.as_deref(), Some("one"));
    }

    #[test]
    fn a_copy_not_kept_leaves_nothing_to_give_back() {
        let mut kept = history(10);
        kept.copied(Some("one".to_string()));
        // A secret, a picture or something too large reads as none.
        assert_eq!(kept.copied(None), None);
        assert_eq!(kept.current, None);
        kept.copied(Some("two".to_string()));
        kept.copied(Some("  ".to_string()));
        assert_eq!(kept.current, None);
    }

    #[test]
    fn forgetting_forgets_what_the_clipboard_holds_too() {
        let mut kept = history(10);
        kept.copied(Some("one".to_string()));
        kept.forget();
        assert!(kept.copies.is_empty());
        assert_eq!(kept.current, None);
    }

    fn copy(at: u128, text: &str) -> Copy {
        Copy {
            at,
            text: text.to_string(),
        }
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
            "{\"at\":5,\"text\":\"say \\\"hi\\\"\\n\\\\ \\u0001\"}\n"
        );
    }
}
