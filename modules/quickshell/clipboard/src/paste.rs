//! A paste's keys: which a window takes, as niri says of the window
//! under the launcher, the wait for the launcher to let go of them, and
//! the virtual keyboard they're pressed on, with its own keymap.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::{CONTROL, SHIFT, State};
use rustix::fs::{MemfdFlags, memfd_create};
use serde_json::Value;
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_seat;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

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
/// rather than press them into it. A paste given up is lost without a word
/// on screen, so it waits past a busy moment: on a loaded builder the
/// launcher took 0.9 s to close and held the keys past a second's wait
/// (2026-11-16), while a launcher still open after three is one that stays.
const LETTING_GO: Duration = Duration::from_secs(3);

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

/// Which paste keys a window takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Keys {
    /// Ctrl+V.
    Plain,
    /// Ctrl+Shift+V, for a terminal, whose Ctrl+V is a key of its own.
    Terminal,
}

impl Keys {
    pub(crate) fn modifiers(self) -> u32 {
        match self {
            Keys::Plain => CONTROL,
            Keys::Terminal => CONTROL | SHIFT,
        }
    }
}

/// What a paste says of its time, in the journal: how long the launcher
/// took to let go of the keys after the ask, which is the shell's frame,
/// and how long the keys took to be pressed after that, which is ours.
pub(crate) fn pasted(letting_go: Duration, pressing: Duration) -> String {
    let ms = |took: Duration| took.as_secs_f64() * 1000.0;
    format!(
        "Pasted: the launcher let go of the keys {:.1} ms after the ask, and they were \
         pressed {:.1} ms after that.",
        ms(letting_go),
        ms(pressing)
    )
}

/// Whether niri's answer to `Layers` says the launcher holds no keys: none
/// when it isn't an answer to that.
pub(crate) fn let_go(answer: &str) -> Option<bool> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let layers = answer.get("Ok")?.get("Layers")?.as_array()?;
    Some(!layers.iter().any(|layer| {
        layer.get("namespace").and_then(Value::as_str) == Some(LAUNCHER)
            && layer.get("keyboard_interactivity").and_then(Value::as_str) != Some("None")
    }))
}

/// What the keys go back to as the launcher lets go of them: a window, by
/// its id, or nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Focus {
    Window(u64),
    Nowhere,
}

/// What niri's answer to `Workspaces` says the keys go back to: the
/// focused workspace's active window. niri says no window is focused
/// while the launcher holds the keys, so its focused window can't say.
/// None when it isn't an answer to that.
pub(crate) fn active(answer: &str) -> Option<Focus> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let workspaces = answer.get("Ok")?.get("Workspaces")?.as_array()?;
    let focused = workspaces
        .iter()
        .find(|workspace| workspace.get("is_focused").and_then(Value::as_bool) == Some(true));
    Some(
        match focused.and_then(|workspace| workspace.get("active_window_id")?.as_u64()) {
            Some(id) => Focus::Window(id),
            None => Focus::Nowhere,
        },
    )
}

/// The keys window `id` takes, from niri's answer to `Windows`: none when
/// it isn't an answer to that or doesn't hold the window.
pub(crate) fn keys_of(answer: &str, id: u64) -> Option<Keys> {
    let answer: Value = serde_json::from_str(answer).ok()?;
    let windows = answer.get("Ok")?.get("Windows")?.as_array()?;
    let window = windows
        .iter()
        .find(|window| window.get("id").and_then(Value::as_u64) == Some(id))?;
    let app = window.get("app_id").and_then(Value::as_str).unwrap_or("");
    Some(if TERMINALS.contains(&app) {
        Keys::Terminal
    } else {
        Keys::Plain
    })
}

/// Asks niri one question, `ask` as its socket takes it, and reads its
/// answer with `read`.
fn asked_niri<T>(socket: &Path, ask: &[u8], read: impl FnOnce(&str) -> Option<T>) -> Option<T> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(LETTING_GO)).ok()?;
    stream.write_all(ask).ok()?;
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).ok()?;
    read(&answer)
}

/// The keys the window under the launcher takes, as niri says, or none
/// with no window focused to paste into. With no niri to ask, or no answer
/// from it, Ctrl+V, as most windows take.
pub(crate) fn under(niri: Option<&Path>) -> Option<Keys> {
    let Some(niri) = niri else {
        return Some(Keys::Plain);
    };
    match asked_niri(niri, b"\"Workspaces\"\n", active) {
        Some(Focus::Window(id)) => Some(
            asked_niri(niri, b"\"Windows\"\n", |answer| keys_of(answer, id)).unwrap_or(Keys::Plain),
        ),
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
pub(crate) fn keys_let_go(niri: Option<&Path>) -> bool {
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

/// A virtual keyboard on the seat, given its own keymap, or none when the
/// compositor has none to give.
pub(crate) fn virtual_keyboard(
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
