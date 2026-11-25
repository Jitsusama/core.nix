//! Drives a Logitech Litra Glow, the light on a desk's monitor.
//!
//! ```text
//! litra on | off | status [--format json]
//! litra brightness PERCENT
//! litra temperature KELVIN
//! litra follow FOLDER [LOCKED [PROFILE KEPT]]
//! litra watch
//! ```
//!
//! The light takes 20-byte HID reports on its hidraw node, which systemd's
//! hwdb opens to the person at the seat: a header, a command, then its
//! values, and it answers a question in a report of the same shape.
//!
//! `follow` lights every light while any camera streams, by the files a
//! camera's watch keeps in FOLDER (one program a line, empty while nobody
//! streams), puts them out once none does, and puts them out when it's
//! stopped. Given LOCKED, a file that says "locked" while the screen is,
//! it keeps them out while it says so, which covers sleep too where the
//! screen locks before the machine sleeps; once unlocked, they follow the
//! cameras again, so one lit by its own button stays out. It sleeps until
//! a file in the folder changes, LOCKED is written, a light is plugged in,
//! or it's told to stop, so it costs nothing in between.
//!
//! Given PROFILE, rhythm's profile, and KEPT, a file of its own, a call
//! lit after tonight's wind-down and before six lights each light warm, at
//! 2700 K, and a little lower, at 70% of its own brightness, since it
//! shines on Joel's eyes from close up while the camera's white balance
//! takes most of the warmth out of how he looks. The evening is decided as
//! a call lights them, so nothing changes on a call already under way. A
//! light's own setting is kept in KEPT by its serial, and the next call by
//! day lights it at its own again; one Joel has set since the evening's
//! counts as his own. With no profile, or none saying a wind-down, nothing
//! changes. Nothing is set on a light while it's out.
//!
//! `watch` prints every light's state as one line of JSON, the same as
//! `status --format json` gives, once at the start and again whenever it
//! changes, for a shell to show it. hidraw gives each reader every report
//! the light sends, so it hears the light's own buttons, which report
//! their new value, and any program's commands, which the light answers
//! with no value, so it asks the light again after one. A light's answer
//! to a question changes nothing but what it knows, so two watches never
//! set each other going. It sleeps until the light says something or a
//! light comes or goes.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::ffi::{c_char, c_int};
use std::path::Path;
use std::process::ExitCode;

use crate::follow::{Evening, follow};
use crate::found::{about, lights};
use crate::light::{
    BRIGHTNESS, Light, POWER, State, TEMPERATURE, ask_state, kelvin, percent, units,
};
use crate::watch::{json, watch};

mod evening;
mod follow;
mod found;
mod light;
mod watch;

// What more than one command asks of the C library: inotify, which
// `follow` and `watch` sleep on, and poll, which every wait for a light
// takes.
const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
const POLLIN: i16 = 0x001;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
}

/// Tells every light plugged in; says so when there's none.
fn every(command: u8, values: &[u8]) -> ExitCode {
    let found = lights(Path::new("/sys"), Path::new("/dev"));
    if found.is_empty() {
        eprintln!("No Litra is plugged in.");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for node in found {
        if let Err(error) = Light::open(&node).and_then(|mut light| light.tell(command, values)) {
            eprintln!("{}: {error}", node.display());
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn status(as_json: bool) -> ExitCode {
    let mut said = Vec::new();
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        match ask_state(&node) {
            Ok(state) if as_json => {
                let about = about(Path::new("/sys"), &node);
                said.push((node, state, about));
            }
            Ok(State {
                on: Some(on),
                units: Some(units),
                kelvin: Some(kelvin),
            }) => {
                let on = if on { "on" } else { "off" };
                println!("{}: {on}, {}%, {kelvin} K", node.display(), percent(units));
            }
            Ok(_) => {}
            Err(error) if as_json => eprintln!("{}: {error}", node.display()),
            Err(error) => println!("{}: {error}", node.display()),
        }
    }
    if as_json {
        println!(
            "{}",
            json(
                said.iter()
                    .map(|(node, state, about)| (node.as_path(), state, about))
            )
        );
    }
    ExitCode::SUCCESS
}

fn number(text: &str, unit: &[char]) -> Option<u16> {
    text.trim_end_matches(unit).parse().ok()
}

const USAGE: &str = "usage: litra on | off | status [--format json] | brightness PERCENT | \
                     temperature KELVIN | follow FOLDER [LOCKED [PROFILE KEPT]] | watch";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments[..] {
        ["on"] => every(POWER, &[1]),
        ["off"] => every(POWER, &[0]),
        ["status"] => status(false),
        ["status", "--format", "json"] => status(true),
        ["watch"] => watch(),
        ["brightness", share] if number(share, &['%']).is_some() => {
            let share = number(share, &['%']).unwrap_or_default();
            every(BRIGHTNESS, &units(share).to_be_bytes())
        }
        ["temperature", warmth] if number(warmth, &['K', 'k']).is_some() => {
            let warmth = number(warmth, &['K', 'k']).unwrap_or_default();
            every(TEMPERATURE, &kelvin(warmth).to_be_bytes())
        }
        ["follow", folder] => follow(Path::new(folder), None, None),
        ["follow", folder, lock] => follow(Path::new(folder), Some(Path::new(lock)), None),
        ["follow", folder, lock, profile, kept] => {
            let evening = Evening {
                profile: Path::new(profile),
                kept: Path::new(kept),
            };
            follow(Path::new(folder), Some(Path::new(lock)), Some(&evening))
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::path::PathBuf;

    /// A folder of the test's own, gone when it is.
    pub(crate) struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn root(name: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("litra-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Scratch(root)
    }
}
