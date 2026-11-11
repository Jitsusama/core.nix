//! Devices coming, going and changing, heard from the kernel's uevent
//! socket, which anyone may read.
//!
//! Two groups say it. The kernel's own (`KERNEL`) says it as it happens,
//! before udev has run its rules, which is soon enough for a cable coming
//! or going (monitor/). udev's (`UDEV`) says it again once its rules and
//! what they run are done, which a light sensor needs: its buffer is turned
//! on by udev's rule (nixos.nix), so the kernel's word comes too early to
//! find it (light.rs).

use std::ffi::c_int;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

// std links libc, so these come from it.
unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn bind(fd: c_int, address: *const NetlinkAddress, length: u32) -> c_int;
}

/// A netlink socket's address, as the kernel's `sockaddr_nl` lays it out.
#[repr(C)]
struct NetlinkAddress {
    family: u16,
    pad: u16,
    pid: u32,
    groups: u32,
}

/// The netlink family, as an address holds it.
const AF_NETLINK: u16 = 16;
const SOCK_DGRAM: c_int = 2;
const SOCK_NONBLOCK: c_int = 0o4000;
const SOCK_CLOEXEC: c_int = 0o2_000_000;
const NETLINK_KOBJECT_UEVENT: c_int = 15;
/// Linux's `ENOBUFS`: the socket's buffer filled, and messages were lost.
const ENOBUFS: i32 = 105;

/// The group the kernel's own uevents go to.
pub const KERNEL: u32 = 1;
/// The group udev's go to, once its rules have run.
pub const UDEV: u32 = 2;

/// A socket that hears the uevents of `group`.
pub fn socket_for(group: u32) -> io::Result<AsyncFd<File>> {
    // SAFETY: it takes only numbers.
    let fd = unsafe {
        socket(
            c_int::from(AF_NETLINK),
            SOCK_DGRAM | SOCK_NONBLOCK | SOCK_CLOEXEC,
            NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just made, and nothing else holds it.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let address = NetlinkAddress {
        family: AF_NETLINK,
        pad: 0,
        pid: 0,
        groups: group,
    };
    let length = u32::try_from(std::mem::size_of::<NetlinkAddress>()).unwrap_or(u32::MAX);
    // SAFETY: the address is a sockaddr_nl that outlives the call, and the
    // length is its size.
    if unsafe { bind(file.as_raw_fd(), &raw const address, length) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the File is moved into the AsyncFd, which owns it until it's
    // dropped, and a File keeps one descriptor open for its life.
    unsafe { AsyncFd::register_with_interest(file, Interest::READABLE) }.map_err(io::Error::from)
}

/// Reads all that's waiting on `source`, saying whether any of it passed
/// `test`, or waits forever when there's no source to hear. Messages lost
/// to a full buffer might have passed, so they count as passing.
pub async fn heard(source: Option<&AsyncFd<File>>, test: fn(&[u8]) -> bool) -> io::Result<bool> {
    let Some(source) = source else {
        return std::future::pending().await;
    };
    let mut ready = source.readable().await?;
    // A uevent is at most a few KiB, and inotify's events are far smaller;
    // udev reads its own into 8 KiB.
    let mut read = [0; 8192];
    let mut passed = false;
    loop {
        let mut file = ready.get_inner();
        match file.read(&mut read) {
            Ok(length) => passed |= test(&read[..length]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                ready.clear_ready();
                return Ok(passed);
            }
            Err(error) if error.raw_os_error() == Some(ENOBUFS) => passed = true,
            Err(error) => return Err(error),
        }
    }
}

/// The KEY=value fields of a kernel uevent, after its first, which is the
/// action and the device's path.
pub fn kernel_fields(event: &[u8]) -> impl Iterator<Item = &[u8]> {
    event
        .split(|byte| *byte == 0)
        .skip(1)
        .filter(|field| !field.is_empty())
}

/// The KEY=value fields of a udev uevent, or none when it isn't one. udev's
/// begins with "libudev", a header that says where its fields lie, then
/// the fields, each ending in a NUL.
pub fn udev_fields(event: &[u8]) -> Option<impl Iterator<Item = &[u8]>> {
    // The prefix, a magic number and the header's size, then where the
    // fields begin and how long they run, in the sender's byte order.
    let header = event.strip_prefix(b"libudev\0")?;
    let word = |at: usize| {
        let bytes = header.get(at..at + 4)?;
        let bytes: [u8; 4] = bytes.try_into().ok()?;
        usize::try_from(u32::from_ne_bytes(bytes)).ok()
    };
    let (start, length) = (word(8)?, word(12)?);
    let fields = event.get(start..start.checked_add(length)?)?;
    Some(
        fields
            .split(|byte| *byte == 0)
            .filter(|field| !field.is_empty()),
    )
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A udev uevent with these fields, laid out as udev sends it.
    pub fn udev_event(fields: &[&str]) -> Vec<u8> {
        let mut body = Vec::new();
        for field in fields {
            body.extend_from_slice(field.as_bytes());
            body.push(0);
        }
        // The prefix, then eight words: magic, header size, where the fields
        // begin and how long they run, and four filter hashes.
        let size: u32 = 8 + 8 * 4;
        let length = u32::try_from(body.len()).expect("a test's fields are short");
        let mut event = b"libudev\0".to_vec();
        for word in [0xFEED_CAFE_u32.to_be(), size, size, length, 0, 0, 0, 0] {
            event.extend_from_slice(&word.to_ne_bytes());
        }
        event.extend_from_slice(&body);
        event
    }

    fn strings<'a>(fields: impl Iterator<Item = &'a [u8]>) -> Vec<String> {
        fields
            .map(|field| String::from_utf8_lossy(field).into_owned())
            .collect()
    }

    #[test]
    fn a_udev_uevent_s_fields_are_read_from_where_its_header_says() {
        let event = udev_event(&["ACTION=add", "SUBSYSTEM=iio"]);
        let fields = udev_fields(&event).map(strings);
        assert_eq!(
            fields,
            Some(vec!["ACTION=add".to_owned(), "SUBSYSTEM=iio".to_owned()])
        );
    }

    #[test]
    fn a_kernel_uevent_is_not_a_udev_one() {
        let event = b"add@/devices/iio:device1\0ACTION=add\0SUBSYSTEM=iio\0";
        assert!(udev_fields(event).is_none());
    }

    #[test]
    fn a_udev_uevent_cut_short_has_no_fields() {
        let mut event = udev_event(&["ACTION=add", "SUBSYSTEM=iio"]);
        event.truncate(event.len() - 4);
        assert!(udev_fields(&event).is_none());
        assert!(udev_fields(b"libudev\0\x01\x02").is_none());
    }

    #[test]
    fn a_kernel_uevent_s_fields_follow_its_action_and_path() {
        let event = b"change@/devices/card0\0ACTION=change\0SUBSYSTEM=drm\0";
        assert_eq!(
            strings(kernel_fields(event)),
            vec!["ACTION=change", "SUBSYSTEM=drm"]
        );
    }
}
