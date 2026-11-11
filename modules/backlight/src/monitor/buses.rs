//! Which I2C buses reach a monitor: those of DisplayPort connectors with
//! something on them and of a dock's MST hub, one bus for each monitor,
//! and the watch on /dev and the uevents that say one came or went.

use std::collections::HashMap;
use std::ffi::{CString, c_char, c_int};
use std::fs::{self, File};
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use super::ddc::Identity;
use crate::uevent;

// std links libc, so these come from it.
unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
/// The bus's permissions changed, as when udev opens it to the seat.
const IN_ATTRIB: u32 = 0x4;
pub(super) const IN_CREATE: u32 = 0x100;
pub(super) const IN_DELETE: u32 = 0x200;
/// Events were lost, so anything could have changed.
pub(super) const IN_Q_OVERFLOW: u32 = 0x4000;

/// Whether a DRM connector is a DisplayPort one, not the panel's eDP.
fn is_displayport(connector: &str) -> bool {
    connector
        .strip_prefix("card")
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(_, kind)| kind.starts_with("DP-"))
}

/// How a bus reaches a monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reach {
    /// An MST hub made it for the one monitor behind it.
    ThroughAHub,
    /// It's a DisplayPort connector's, and reaches what's on the cable,
    /// through a hub if there is one.
    ByItsConnector,
}

/// How a bus in /sys/class/i2c-dev reaches a monitor, if it does.
///
/// A connector that DRM says has nothing on it reaches none. A dock's hub is
/// one of those too, since its monitors are on connectors of their own, which
/// its hub's buses reach.
pub(super) fn reach(bus: &Path) -> Option<Reach> {
    let name = fs::read_to_string(bus.join("name")).unwrap_or_default();
    if name.trim() == "DPMST" {
        return Some(Reach::ThroughAHub);
    }
    let device = fs::canonicalize(bus.join("device")).ok()?;
    let connector = device.parent()?;
    let displayport = connector
        .file_name()
        .is_some_and(|connector| is_displayport(&connector.to_string_lossy()));
    let status = fs::read_to_string(connector.join("status")).unwrap_or_default();
    (displayport && status.trim() != "disconnected").then_some(Reach::ByItsConnector)
}

/// A bus that reaches a monitor, and which monitor once its EDID is read.
pub(super) struct Seen<'a> {
    pub(super) name: &'a str,
    pub(super) reach: Reach,
    pub(super) identity: Option<Identity>,
}

/// The connector buses that reach a monitor a hub's bus reaches too, each
/// with the hub's bus it's left to.
pub(super) fn doubled<'a>(seen: &[Seen<'a>]) -> HashMap<&'a str, &'a str> {
    let hubs: HashMap<Identity, &str> = seen
        .iter()
        .filter(|bus| bus.reach == Reach::ThroughAHub)
        .filter_map(|bus| Some((bus.identity?, bus.name)))
        .collect();
    seen.iter()
        .filter(|bus| bus.reach == Reach::ByItsConnector)
        .filter_map(|bus| Some((bus.name, *hubs.get(&bus.identity?)?)))
        .collect()
}

/// The buses that reach a monitor, by name, and have a node in `dev`.
pub(super) fn buses(class: &Path, dev: &Path) -> io::Result<Vec<(String, Reach)>> {
    let entries = match fs::read_dir(class) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // The node goes before the class's entry does, so a bus going is
        // known from the node.
        if !name.starts_with("i2c-") || !dev.join(&name).exists() {
            continue;
        }
        if let Some(reach) = reach(&entry.path()) {
            found.push((name, reach));
        }
    }
    found.sort();
    Ok(found)
}

/// Whether inotify's events name a bus, or lost some that might have.
pub(super) fn names_a_bus(events: &[u8]) -> bool {
    // Each is a descriptor, a mask, a cookie, a length, then a name that
    // long, padded with NULs.
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let mask = word(4);
        let length = word(12) as usize;
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        if mask & IN_Q_OVERFLOW != 0 || name.starts_with(b"i2c-") {
            return true;
        }
        rest = after;
    }
    false
}

/// Whether a kernel uevent says a DRM connector's cable came or went.
pub(super) fn says_a_cable_moved(event: &[u8]) -> bool {
    let fields: Vec<&[u8]> = uevent::kernel_fields(event).collect();
    fields.contains(&&b"SUBSYSTEM=drm"[..]) && fields.contains(&&b"HOTPLUG=1"[..])
}

/// A watch on `dev` that wakes when a node is made, removed, or opened to
/// someone.
pub(super) fn watch(dev: &Path) -> io::Result<AsyncFd<File>> {
    // SAFETY: it takes only flags.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just made, and nothing else holds it.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let path = CString::new(dev.as_os_str().as_bytes())?;
    // SAFETY: the descriptor is the file's, and the path a C string that
    // outlives the call.
    let mask = IN_CREATE | IN_DELETE | IN_ATTRIB;
    if unsafe { inotify_add_watch(file.as_raw_fd(), path.as_ptr(), mask) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the File is moved into the AsyncFd, which owns it until it's
    // dropped, and a File keeps one descriptor open for its life.
    unsafe { AsyncFd::register_with_interest(file, Interest::READABLE) }.map_err(io::Error::from)
}
