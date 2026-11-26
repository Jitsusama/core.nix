//! `follow`: lighting the lights while a camera streams and the screen
//! isn't locked, and putting them out otherwise or when it's stopped.

use std::ffi::{CString, c_int};
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use crate::evening::{Setting, own, read_kept, tonight, written_kept};
use crate::found::{about, lights, plugged};
use crate::light::{
    BRIGHTNESS, BRIGHTNESS_IS, Light, POWER, POWER_IS, State, TEMPERATURE, TEMPERATURE_IS,
    find_out, percent,
};
use crate::{
    IN_ATTRIB, IN_CLOEXEC, IN_CREATE, IN_DELETE, POLLIN, PollFd, inotify_add_watch, inotify_init1,
    poll,
};

const IN_CLOSE_WRITE: u32 = 0x008;
const IN_MOVED_TO: u32 = 0x080;
const IN_DELETE_SELF: u32 = 0x400;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

/// glibc's `sigset_t`, 1024 bits.
#[repr(C)]
struct SigSet([u64; 16]);

unsafe extern "C" {
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
}

/// Whether any camera's file in the folder names a program. A file being
/// replaced is written beside it first under a dotted name, so one
/// starting with a dot is someone's half-written one, or hidden; a camera
/// named for a version, as `Webcam 2.0`, is a camera's.
fn streaming(folder: &Path) -> bool {
    let Ok(entries) = fs::read_dir(folder) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let temporary = entry.file_name().as_encoded_bytes().starts_with(b".");
        !temporary && fs::read_to_string(entry.path()).is_ok_and(|text| !text.trim().is_empty())
    })
}

/// Whether a lock's file says the screen is locked. No file says it isn't,
/// as before the shell first writes one.
fn locked(marker: &Path) -> bool {
    fs::read_to_string(marker).is_ok_and(|text| text.trim() == "locked")
}

/// Whether the lights should be lit: while a camera streams, unless the
/// screen is locked.
fn wanted(folder: &Path, lock: Option<&Path>) -> bool {
    streaming(folder) && !lock.is_some_and(locked)
}

/// Puts every light plugged in out, saying what failed.
fn put_out_all() {
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        if let Err(error) = Light::open(&node).and_then(|mut light| light.tell(POWER, &[0])) {
            eprintln!("{}: {error}", node.display());
        }
    }
}

/// Lights every light plugged in, saying what failed. Given a file to keep
/// each light's own setting in, each is lit at its own, or the evening's
/// of it in the evening, and its own is kept by its serial.
fn light_all(evening: bool, kept: Option<&Path>) {
    let mut owns = kept
        .and_then(|kept| fs::read_to_string(kept).ok())
        .map(|text| read_kept(&text))
        .unwrap_or_default();
    let before = owns.clone();
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        let owns = kept.is_some().then_some(&mut owns);
        if let Err(error) = light(&node, evening, owns) {
            eprintln!("{}: {error}", node.display());
        }
    }
    if let Some(kept) = kept
        && owns != before
        && let Err(error) = fs::write(kept, written_kept(&owns))
    {
        eprintln!("{}: {error}", kept.display());
    }
}

/// Lights one light. Given its own settings, it's asked its setting while
/// still out, since a question changes nothing, then lit and set to its
/// own, or the evening's of it, so nothing is set while it's out. One
/// already lit, by its button or for a call under way, is left as it is,
/// and one that can't say its setting is only lit.
fn light(node: &Path, evening: bool, owns: Option<&mut Vec<(String, Setting)>>) -> io::Result<()> {
    let mut light = Light::open(node)?;
    let mut state = State::default();
    let seen = match owns {
        Some(_) => {
            let asked = vec![TEMPERATURE_IS, BRIGHTNESS_IS, POWER_IS];
            let heard = find_out(&mut light, &mut state, asked);
            if heard.is_ok() && state.on == Some(true) {
                return Ok(());
            }
            heard.ok().and_then(|()| Setting::of(&state))
        }
        None => None,
    };
    light.tell(POWER, &[1])?;
    let (Some(owns), Some(seen)) = (owns, seen) else {
        return Ok(());
    };
    let about = about(Path::new("/sys"), node);
    let serial = if about.serial.is_empty() {
        node.display().to_string()
    } else {
        about.serial
    };
    let index = owns.iter().position(|(kept, _)| *kept == serial);
    let mine = own(seen, index.map(|index| owns[index].1));
    match index {
        Some(index) => owns[index].1 = mine,
        None => owns.push((serial, mine)),
    }
    let wanted = if evening { mine.for_evening() } else { mine };
    if wanted.kelvin != seen.kelvin {
        light.tell(TEMPERATURE, &wanted.kelvin.to_be_bytes())?;
    }
    if wanted.units != seen.units {
        light.tell(BRIGHTNESS, &wanted.units.to_be_bytes())?;
    }
    if evening && wanted != seen {
        eprintln!(
            "{}: lit warm and a little lower for the evening, {}% at {} K",
            node.display(),
            percent(wanted.units),
            wanted.kelvin
        );
    }
    Ok(())
}

/// A descriptor that becomes readable when SIGTERM or SIGINT arrives,
/// which are blocked so they wait for it.
fn stopping() -> io::Result<File> {
    let mut set = SigSet([0; 16]);
    // SAFETY: plain calls on a set of our own.
    let fd = unsafe {
        sigemptyset(&raw mut set);
        sigaddset(&raw mut set, SIGTERM);
        sigaddset(&raw mut set, SIGINT);
        if sigprocmask(SIG_BLOCK, &raw const set, std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
        signalfd(-1, &raw const set, SFD_CLOEXEC)
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Watches the folder and /dev, the folder for a camera's file and /dev
/// for a light coming, or the permissions udev gives it after, and the
/// lock's folder, if there is one, for the lock's file being written.
fn changes(folder: &Path, lock: Option<&Path>) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let mut watches = vec![
        (
            folder.as_os_str(),
            IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE | IN_DELETE | IN_DELETE_SELF,
        ),
        (Path::new("/dev").as_os_str(), IN_CREATE | IN_ATTRIB),
    ];
    // Its folder rather than the file, since the file can be replaced.
    if let Some(parent) = lock.and_then(Path::parent) {
        watches.push((parent.as_os_str(), IN_CLOSE_WRITE | IN_MOVED_TO));
    }
    for (path, mask) in watches {
        let path = CString::new(path.as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        if unsafe { inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(file)
}

/// Whether a run of inotify events says the folder itself has gone.
fn folder_gone(events: &[u8]) -> bool {
    let mut offset = 0;
    while offset + 16 <= events.len() {
        let mask = u32::from_ne_bytes(events[offset + 4..offset + 8].try_into().unwrap());
        let length = u32::from_ne_bytes(events[offset + 12..offset + 16].try_into().unwrap());
        if mask & IN_DELETE_SELF != 0 {
            return true;
        }
        offset += 16 + length as usize;
    }
    false
}

/// Where the evening's light comes from: rhythm's profile, which says when
/// Joel winds down tonight, and the file each light's own setting is kept
/// in.
pub(crate) struct Evening<'a> {
    pub(crate) profile: &'a Path,
    pub(crate) kept: &'a Path,
}

pub(crate) fn follow(folder: &Path, lock: Option<&Path>, evening: Option<&Evening>) -> ExitCode {
    let (mut stop, mut changed) = match (stopping(), changes(folder, lock)) {
        (Ok(stop), Ok(changed)) => (stop, changed),
        (Err(error), _) | (_, Err(error)) => {
            eprintln!("{}: {error}", folder.display());
            return ExitCode::FAILURE;
        }
    };
    let mut lit = None;
    // Whether the call lit is lit for the evening, decided as it's lit, so
    // the wind-down passing mid-call, or a light plugged in, changes
    // nothing on one already lit.
    let mut for_evening = false;
    let mut lights_seen = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let wanted = wanted(folder, lock);
        // A light that has come since is told too, whatever it was doing,
        // and so is one plugged back in under its old name.
        let now = plugged(Path::new("/sys"), Path::new("/dev"));
        if Some(wanted) != lit || now != lights_seen {
            if wanted {
                if lit != Some(true) {
                    for_evening = evening.is_some_and(|evening| tonight(evening.profile));
                }
                light_all(for_evening, evening.map(|evening| evening.kept));
            } else {
                put_out_all();
            }
            lit = Some(wanted);
            lights_seen = now;
        }
        let mut waiting = [
            PollFd {
                fd: changed.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            PollFd {
                fd: stop.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: two valid pollfds for the length given.
        if unsafe { poll(waiting.as_mut_ptr(), 2, -1) } < 0 {
            continue;
        }
        if waiting[1].revents & POLLIN != 0 {
            let _ = stop.read(&mut buffer);
            put_out_all();
            return ExitCode::SUCCESS;
        }
        // A camera's watch replaces its file in two steps, and a light's
        // node comes before udev opens it; one look after they settle
        // answers for all of it.
        std::thread::sleep(Duration::from_millis(100));
        match changed.read(&mut buffer) {
            Ok(length) if folder_gone(&buffer[..length]) => {
                put_out_all();
                eprintln!("{} went away.", folder.display());
                return ExitCode::FAILURE;
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                eprintln!("{}: {error}", folder.display());
                return ExitCode::FAILURE;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn streaming_is_any_cameras_file_naming_a_program() {
        let root = root("streaming");
        assert!(!streaming(&root), "an empty folder streams");
        fs::write(root.join("camera"), "").unwrap();
        fs::write(root.join("MX Brio 705 for Business"), "\n").unwrap();
        assert!(!streaming(&root), "a camera's empty file names a program");
        fs::write(root.join(".camera.4123"), "zoom\n").unwrap();
        fs::write(root.join(".camera"), "zoom\n").unwrap();
        assert!(!streaming(&root), "a file being replaced is no camera");
        fs::write(root.join("MX Brio 705 for Business"), "zoom\n").unwrap();
        assert!(streaming(&root), "a camera naming a program doesn't stream");
        fs::remove_file(root.join("MX Brio 705 for Business")).unwrap();
        fs::write(root.join("Webcam 2.0"), "zoom\n").unwrap();
        assert!(streaming(&root), "a camera named for a version is a camera");
        assert!(
            !streaming(&root.join("nothing")),
            "a folder that isn't there streams"
        );
    }

    #[test]
    fn a_locked_screen_keeps_the_lights_out_while_a_camera_streams() {
        let root = root("locked");
        let cameras = root.join("cameras");
        fs::create_dir_all(&cameras).unwrap();
        fs::write(cameras.join("MX Brio 705 for Business"), "zoom\n").unwrap();
        let marker = root.join("quickshell-locked");
        assert!(
            wanted(&cameras, None),
            "a camera streams with no lock, yet the lights aren't wanted"
        );
        assert!(wanted(&cameras, Some(&marker)), "no file says unlocked");
        fs::write(&marker, "locked\n").unwrap();
        assert!(
            !wanted(&cameras, Some(&marker)),
            "a locked screen let the lights light"
        );
        fs::write(&marker, "unlocked\n").unwrap();
        assert!(
            wanted(&cameras, Some(&marker)),
            "an unlocked screen kept the lights out"
        );
        fs::write(cameras.join("MX Brio 705 for Business"), "").unwrap();
        assert!(
            !wanted(&cameras, Some(&marker)),
            "no camera streams, yet the lights are wanted"
        );
    }

    #[test]
    fn the_folder_going_is_heard_among_other_events() {
        let mut events = Vec::new();
        for (mask, name) in [
            (IN_CLOSE_WRITE, &b"camera\0\0"[..]),
            (IN_DELETE_SELF, &b""[..]),
        ] {
            events.extend_from_slice(&1i32.to_ne_bytes());
            events.extend_from_slice(&mask.to_ne_bytes());
            events.extend_from_slice(&0u32.to_ne_bytes());
            events.extend_from_slice(&u32::try_from(name.len()).unwrap().to_ne_bytes());
            events.extend_from_slice(name);
        }
        assert!(
            folder_gone(&events),
            "the folder's going was missed among other events"
        );
        assert!(
            !folder_gone(&events[..24]),
            "a camera's file closing was taken for the folder going"
        );
    }
}
