//! Says which layer the ErgoDox is on, in the file it's given: the layer's
//! name, as the names after the file give them in the board's order, then
//! " locked" while it's locked rather than held, and a newline. While no
//! board that can say is plugged in, the file says "-".
//!
//! It asks the board to watch (keymap/host.c), which tells it each layer as
//! the layer changes and never a key, so what's typed stays between the
//! board and the keyboard's own interface. It finds the board's channel by
//! ZSA's vendor number and the usage page Oryx's channel opens on, and when
//! the board goes it sleeps on the device directory until one comes back.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::ffi::{CString, c_char, c_int};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VENDOR: &str = "HID_ID=0003:00003297:";
const CHANNEL: [u8; 3] = [0x06, 0x60, 0xff];
const REPORT: usize = 32;
const WATCH: u8 = 0x81;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CREATE: u32 = 0x100;

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

/// The board's channel among the hidraw nodes `sys` lists, as its node in
/// `dev`.
fn channel(sys: &Path, dev: &Path) -> Option<PathBuf> {
    let mut nodes: Vec<_> = fs::read_dir(sys.join("class/hidraw"))
        .ok()?
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    nodes.sort();
    nodes.into_iter().find_map(|name| {
        let device = sys.join("class/hidraw").join(&name).join("device");
        let uevent = fs::read_to_string(device.join("uevent")).ok()?;
        let descriptor = fs::read(device.join("report_descriptor")).ok()?;
        (uevent.lines().any(|line| line.starts_with(VENDOR)) && descriptor.starts_with(&CHANNEL))
            .then(|| dev.join(&name))
    })
}

/// A command as hidraw takes it: the report number, 0, then one report.
fn command(code: u8, param: u8) -> [u8; REPORT + 1] {
    let mut report = [0; REPORT + 1];
    report[1] = code;
    report[2] = param;
    report
}

/// What a watch report says: the layer, by its name or its number when it
/// has none, and whether it's locked. Any other report says nothing.
fn told(report: &[u8], names: &[String]) -> Option<String> {
    let [WATCH, layer, locked, ..] = report else {
        return None;
    };
    let name = names
        .get(usize::from(*layer))
        .cloned()
        .unwrap_or_else(|| layer.to_string());
    Some(if *locked == 1 { format!("{name} locked\n") } else { format!("{name}\n") })
}

/// What's been said. Every text is one line, written over the last and then
/// cut to its length, so a reader of the first line never finds it empty.
struct Said {
    file: File,
    last: Option<String>,
}

impl Said {
    fn say(&mut self, text: &str) {
        if self.last.as_deref() == Some(text) {
            return;
        }
        let bytes = text.as_bytes();
        if self.file.write_at(bytes, 0).is_ok() && self.file.set_len(bytes.len() as u64).is_ok() {
            self.last = Some(text.to_owned());
        }
    }
}

/// Watches the board at `node` until it goes, saying each layer it tells of.
fn watch(node: &Path, names: &[String], said: &mut Said) {
    let Ok(mut board) = OpenOptions::new().read(true).write(true).open(node) else {
        return;
    };
    if board.write_all(&command(WATCH, 1)).is_err() {
        return;
    }
    let mut report = [0u8; REPORT];
    loop {
        match board.read(&mut report) {
            Ok(0) => return,
            Ok(length) => {
                if let Some(text) = told(&report[..length], names) {
                    said.say(&text);
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(_) => return,
        }
    }
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: ergodox-watch FILE [NAME...]");
        return ExitCode::from(2);
    };
    let names: Vec<String> = arguments.collect();
    let file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o644)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut said = Said { file, last: None };

    // A board's node appears before udev opens it to the person at the
    // machine, so a change to a node's permissions is a second chance.
    let directory = CString::new("/dev").unwrap();
    // SAFETY: plain calls; the path is a valid C string for the call.
    let changes = unsafe { inotify_init1(IN_CLOEXEC) };
    if changes < 0
        || unsafe { inotify_add_watch(changes, directory.as_ptr(), IN_CREATE | IN_ATTRIB) } < 0
    {
        eprintln!("/dev: {}", std::io::Error::last_os_error());
        return ExitCode::FAILURE;
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let mut changes = unsafe { File::from_raw_fd(changes) };

    let mut buffer = [0u8; 4096];
    loop {
        if let Some(node) = channel(Path::new("/sys"), Path::new("/dev")) {
            watch(&node, &names, &mut said);
        }
        said.say("-\n");
        // Whatever changed, the next pass looks again. A board on other
        // firmware never answers, so it waits for the next change.
        match changes.read(&mut buffer) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                eprintln!("/dev: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("ergodox-watch-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn node(root: &Path, name: &str, id: &str, descriptor: &[u8]) {
        let device = root.join("sys/class/hidraw").join(name).join("device");
        fs::create_dir_all(&device).unwrap();
        fs::write(device.join("uevent"), format!("DRIVER=hid-generic\nHID_ID={id}\n")).unwrap();
        fs::write(device.join("report_descriptor"), descriptor).unwrap();
    }

    fn names() -> Vec<String> {
        ["Coal", "Punk", "Numb"].map(String::from).to_vec()
    }

    #[test]
    fn the_channel_is_zsas_and_opens_on_oryxs_usage_page() {
        let root = root("channel");
        node(&root, "hidraw0", "0003:0000046D:0000C52B", &[0x06, 0x60, 0xff, 0x09]);
        node(&root, "hidraw1", "0003:00003297:00004976", &[0x05, 0x01, 0x09, 0x06]);
        node(&root, "hidraw2", "0003:00003297:00004976", &[0x06, 0x60, 0xff, 0x09]);
        assert_eq!(
            channel(&root.join("sys"), Path::new("/dev")),
            Some(PathBuf::from("/dev/hidraw2"))
        );
    }

    #[test]
    fn no_board_is_no_channel() {
        let root = root("none");
        node(&root, "hidraw0", "0003:00003297:00004976", &[0x05, 0x01]);
        assert_eq!(channel(&root.join("sys"), Path::new("/dev")), None);
        assert_eq!(channel(&root.join("nothing"), Path::new("/dev")), None);
    }

    #[test]
    fn a_command_is_one_report_behind_report_number_zero() {
        let report = command(WATCH, 1);
        assert_eq!(report.len(), 33);
        assert_eq!(report[..3], [0, 0x81, 1]);
        assert!(report[3..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn a_watch_report_says_the_layer_and_whether_it_is_locked() {
        assert_eq!(told(&[0x81, 2, 1], &names()).as_deref(), Some("Numb locked\n"));
        assert_eq!(told(&[0x81, 1, 0], &names()).as_deref(), Some("Punk\n"));
        assert_eq!(told(&[0x81, 4, 0], &names()).as_deref(), Some("4\n"));
    }

    #[test]
    fn no_other_report_says_anything() {
        assert_eq!(told(&[5, 2, 0xfe], &names()), None);
        assert_eq!(told(&[6, 3, 1], &names()), None);
        assert_eq!(told(&[0xff, 0xff], &names()), None);
        assert_eq!(told(&[0x81, 2], &names()), None);
    }

    #[test]
    fn what_is_said_replaces_what_was() {
        let root = root("said");
        fs::create_dir_all(&root).unwrap();
        let path = root.join("layer");
        let file = OpenOptions::new().write(true).create(true).truncate(false).open(&path).unwrap();
        let mut said = Said { file, last: None };
        said.say("Numb locked\n");
        said.say("Punk\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "Punk\n");
        said.say("-\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), "-\n");
    }
}
