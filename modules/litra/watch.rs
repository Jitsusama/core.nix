//! `watch`: every light's state as a line of JSON, again each time it
//! changes.

use std::ffi::CString;
use std::fs::File;
use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::found::{About, about, lights};
use crate::light::{
    BRIGHTEST, BRIGHTNESS_IS, DIMMEST, Heard, Light, POWER_IS, State, TEMPERATURE_IS, find_out,
    hear, percent,
};
use crate::{
    IN_ATTRIB, IN_CLOEXEC, IN_CREATE, IN_DELETE, POLLIN, PollFd, inotify_add_watch, inotify_init1,
    poll,
};

const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;

/// Text as a JSON string holds it.
fn quoted(text: &str) -> String {
    use std::fmt::Write as _;
    let mut said = String::with_capacity(text.len() + 2);
    said.push('"');
    for c in text.chars() {
        match c {
            '"' => said.push_str("\\\""),
            '\\' => said.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(said, "\\u{:04x}", u32::from(c));
            }
            c => said.push(c),
        }
    }
    said.push('"');
    said
}

/// The lights that have said everything about themselves, as JSON: each
/// one's node, whether it's on, its share of the brightest and the lumens
/// that is, which its own units are, its warmth, and its name and serial
/// as it gives them.
pub(crate) fn json<'a>(
    lights: impl IntoIterator<Item = (&'a Path, &'a State, &'a About)>,
) -> String {
    let said: Vec<String> = lights
        .into_iter()
        .filter_map(|(node, state, about)| {
            let (on, units, kelvin) = (state.on?, state.units?, state.kelvin?);
            let range = if about.glow {
                format!(r#","dimmest":{DIMMEST},"brightest":{BRIGHTEST}"#)
            } else {
                String::new()
            };
            Some(format!(
                r#"{{"node":{},"on":{on},"percent":{},"lumens":{units}{range},"kelvin":{kelvin},"name":{},"serial":{}}}"#,
                quoted(&node.display().to_string()),
                percent(units),
                quoted(&about.name),
                quoted(&about.serial)
            ))
        })
        .collect();
    format!("[{}]", said.join(","))
}

/// Watches /dev for a light's node coming or going, or the permissions
/// udev gives it after it comes.
fn devices() -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let dev = CString::new("/dev").map_err(io::Error::other)?;
    // SAFETY: the path is a valid C string for the call.
    if unsafe { inotify_add_watch(fd, dev.as_ptr(), IN_CREATE | IN_ATTRIB | IN_DELETE) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

struct Watched {
    node: PathBuf,
    light: Light,
    state: State,
    about: About,
}

impl Watched {
    /// A light found out, with what its HID device says of it.
    fn new(node: PathBuf, light: Light, state: State) -> Self {
        let about = about(Path::new("/sys"), &node);
        Self {
            node,
            light,
            state,
            about,
        }
    }
}

pub(crate) fn watch() -> ExitCode {
    let mut devices = match devices() {
        Ok(devices) => devices,
        Err(error) => {
            eprintln!("/dev: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut watched: Vec<Watched> = Vec::new();
    let mut said: Option<String> = None;
    let mut look = true;
    let mut out = io::stdout().lock();
    let mut buffer = [0u8; 4096];
    loop {
        if look {
            let present = lights(Path::new("/sys"), Path::new("/dev"));
            watched.retain(|light| present.contains(&light.node));
            for node in present {
                if watched.iter().any(|light| light.node == node) {
                    continue;
                }
                // One udev hasn't opened yet is tried again when it does.
                let Ok(mut light) = Light::open(&node) else {
                    continue;
                };
                let mut state = State::default();
                let asking = vec![TEMPERATURE_IS, BRIGHTNESS_IS, POWER_IS];
                if find_out(&mut light, &mut state, asking).is_ok() {
                    watched.push(Watched::new(node, light, state));
                }
            }
            look = false;
        }

        let line = json(
            watched
                .iter()
                .map(|light| (light.node.as_path(), &light.state, &light.about)),
        );
        if said.as_ref() != Some(&line) {
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                // Nobody is reading any more.
                return ExitCode::SUCCESS;
            }
            said = Some(line);
        }

        let mut waiting = vec![PollFd {
            fd: devices.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        waiting.extend(watched.iter().map(|light| PollFd {
            fd: light.light.file.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }));
        // SAFETY: valid pollfds for the length given.
        if unsafe { poll(waiting.as_mut_ptr(), waiting.len() as u64, -1) } < 0 {
            continue;
        }
        if waiting[0].revents & POLLIN != 0 {
            // A node comes before udev opens it to the seat.
            std::thread::sleep(Duration::from_millis(100));
            let _ = devices.read(&mut buffer);
            look = true;
        }
        let mut gone = Vec::new();
        for (index, light) in watched.iter_mut().enumerate() {
            let revents = waiting[index + 1].revents;
            if revents & (POLLERR | POLLHUP) != 0 {
                gone.push(index);
                continue;
            }
            if revents & POLLIN == 0 {
                continue;
            }
            let mut asking = Vec::new();
            let read = loop {
                match light.light.file.read(&mut buffer) {
                    Ok(0) => break Ok(()),
                    Ok(length) => {
                        if let Heard::Ask(question) = hear(&mut light.state, &buffer[..length])
                            && !asking.contains(&question)
                        {
                            asking.push(question);
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break Ok(()),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => break Err(error),
                }
            };
            if read
                .and_then(|()| find_out(&mut light.light, &mut light.state, asking))
                .is_err()
            {
                gone.push(index);
            }
        }
        for index in gone.into_iter().rev() {
            watched.remove(index);
            look = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_says_only_the_lights_that_have_said_everything() {
        let whole = State {
            on: Some(false),
            units: Some(DIMMEST + 39),
            kelvin: Some(5000),
        };
        let partial = State {
            on: Some(true),
            ..State::default()
        };
        let glow = About {
            name: "Logitech Litra Glow".to_owned(),
            serial: "2213FE40".to_owned(),
            glow: true,
        };
        let unknown = About::default();
        assert_eq!(json([]), "[]");
        assert_eq!(
            json([
                (Path::new("/dev/hidraw1"), &whole, &glow),
                (Path::new("/dev/hidraw4"), &partial, &glow),
            ]),
            r#"[{"node":"/dev/hidraw1","on":false,"percent":17,"lumens":59,"dimmest":20,"brightest":250,"kelvin":5000,"name":"Logitech Litra Glow","serial":"2213FE40"}]"#
        );
        assert_eq!(
            json([
                (Path::new("/dev/a\"b"), &whole, &unknown),
                (Path::new("/dev/c\\\u{1}"), &whole, &unknown)
            ]),
            r#"[{"node":"/dev/a\"b","on":false,"percent":17,"lumens":59,"kelvin":5000,"name":"","serial":""},{"node":"/dev/c\\\u0001","on":false,"percent":17,"lumens":59,"kelvin":5000,"name":"","serial":""}]"#
        );
    }
}
