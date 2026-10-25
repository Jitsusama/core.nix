//! The controller, through the kernel's HCI sockets: one, bound to no
//! adapter, for the ioctls that list the adapters, say what each is and
//! list their connections, and one bound to each adapter asked a command,
//! kept open between ticks. The kernel lets a socket with no capability use
//! those ioctls and send the five commands asked here (`hci_sec_filter` in
//! `net/bluetooth/hci_sock.c`), so nothing here needs root. Its bytes are
//! read in `reply`.

use crate::reply::{self, Adapter, Connection, Heard};
use std::collections::HashMap;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::time::{Duration, Instant};

const AF_BLUETOOTH: u16 = 31;
const BTPROTO_HCI: libc::c_int = 1;
const SOL_HCI: libc::c_int = 0;
const HCI_FILTER: libc::c_int = 2;
const HCI_CHANNEL_RAW: u16 = 0;

/// `_IOR('H', number, int)`: the kernel reads and writes the list through
/// the pointer, whatever the size the macro names.
const fn reads(number: libc::Ioctl) -> libc::Ioctl {
    (2 << 30) | (4 << 16) | (0x48 << 8) | number
}
const HCIGETDEVLIST: libc::Ioctl = reads(210);
const HCIGETDEVINFO: libc::Ioctl = reads(211);
const HCIGETCONNLIST: libc::Ioctl = reads(212);

/// How long a command's answer is waited for. The controller answers in a
/// few milliseconds; one that takes this long is busy or gone.
const PATIENCE: Duration = Duration::from_secs(1);
/// An event packet is at most its three bytes of header and 255 of body.
const LONGEST_EVENT: usize = 258;

/// `sockaddr_hci`, which binds a socket to an adapter.
#[repr(C)]
struct Address {
    family: libc::sa_family_t,
    device: u16,
    channel: u16,
}

fn checked(result: libc::c_int) -> io::Result<libc::c_int> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

fn open() -> io::Result<OwnedFd> {
    let kind = libc::SOCK_RAW | libc::SOCK_CLOEXEC;
    // SAFETY: socket takes no pointers, and its result is checked.
    let fd = checked(unsafe { libc::socket(AF_BLUETOOTH.into(), kind, BTPROTO_HCI) })?;
    // SAFETY: the descriptor was just opened, and nothing else owns it.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Fills `list` through an ioctl that reads it and writes it back.
fn listed(fd: RawFd, request: libc::Ioctl, list: &mut [u8]) -> io::Result<()> {
    // SAFETY: the kernel writes no further than the room the list says it
    // has, which is the length of the buffer it was made with.
    checked(unsafe { libc::ioctl(fd, request, list.as_mut_ptr()) }).map(|_| ())
}

/// A socket bound to one adapter, hearing only what answers a command.
fn bound(adapter: u16) -> io::Result<OwnedFd> {
    let fd = open()?;
    let address = Address {
        family: AF_BLUETOOTH,
        device: adapter,
        channel: HCI_CHANNEL_RAW,
    };
    let size = libc::socklen_t::try_from(size_of::<Address>()).unwrap_or(0);
    // SAFETY: the address is a `sockaddr_hci`, of the size given.
    checked(unsafe { libc::bind(fd.as_raw_fd(), (&raw const address).cast(), size) })?;
    let filter = reply::filter();
    let length = libc::socklen_t::try_from(filter.len()).unwrap_or(0);
    // SAFETY: the filter is an `hci_ufilter`, of the length given.
    checked(unsafe {
        libc::setsockopt(
            fd.as_raw_fd(),
            SOL_HCI,
            HCI_FILTER,
            filter.as_ptr().cast(),
            length,
        )
    })?;
    Ok(fd)
}

/// What one look at the adapters found: each adapter, and every adapter's
/// connections that carry data, each with its adapter's id.
#[derive(Debug, Default, Clone)]
pub struct Seen {
    pub adapters: Vec<Adapter>,
    pub connections: Vec<(u16, Connection)>,
}

/// The machine's Bluetooth adapters, asked through the kernel.
#[derive(Default)]
pub struct Radio {
    lister: Option<OwnedFd>,
    bound: HashMap<u16, OwnedFd>,
}

impl Radio {
    /// Every adapter and its connections. With no Bluetooth at all, there
    /// are none.
    pub fn look(&mut self) -> io::Result<Seen> {
        let fd = match &self.lister {
            Some(fd) => fd.as_raw_fd(),
            None => self.lister.insert(open()?).as_raw_fd(),
        };
        let mut adapters = reply::adapter_request();
        if let Err(error) = listed(fd, HCIGETDEVLIST, &mut adapters) {
            // Opened afresh next time, in case the socket's gone bad.
            self.lister = None;
            return Err(error);
        }
        let mut seen = Seen::default();
        for adapter in reply::adapters(&adapters) {
            // An adapter going away as it's asked says nothing, and has no
            // connections.
            let mut info = reply::device_info_request(adapter);
            if listed(fd, HCIGETDEVINFO, &mut info).is_ok()
                && let Some(said) = reply::device_info(&info)
            {
                seen.adapters.push(said);
            }
            let mut list = reply::connection_request(adapter);
            if listed(fd, HCIGETCONNLIST, &mut list).is_ok() {
                let found = reply::connections(&list).into_iter();
                seen.connections.extend(found.map(|c| (adapter, c)));
            }
        }
        // An adapter gone, or with nothing connected, is let go of: its
        // socket is opened again when it has a link to ask of.
        let linked: Vec<u16> = seen.connections.iter().map(|(id, _)| *id).collect();
        self.bound.retain(|adapter, _| linked.contains(adapter));
        Ok(seen)
    }

    /// Asks one of a connection's commands, giving its return parameters.
    pub fn ask(&mut self, adapter: u16, opcode: u16, handle: u16) -> io::Result<Vec<u8>> {
        let fd = match self.bound.get(&adapter) {
            Some(fd) => fd.as_raw_fd(),
            None => self
                .bound
                .entry(adapter)
                .or_insert(bound(adapter)?)
                .as_raw_fd(),
        };
        let asked = asked(fd, opcode, handle);
        if asked
            .as_ref()
            .is_err_and(|error| error.kind() != io::ErrorKind::TimedOut)
        {
            self.bound.remove(&adapter);
        }
        asked
    }
}

fn asked(fd: RawFd, opcode: u16, handle: u16) -> io::Result<Vec<u8>> {
    let mut packet = [0u8; LONGEST_EVENT];
    // What's waiting answered someone else, or a command given up on.
    // SAFETY: the kernel writes at most the buffer's length into it.
    while unsafe {
        libc::recv(
            fd,
            packet.as_mut_ptr().cast(),
            packet.len(),
            libc::MSG_DONTWAIT,
        )
    } > 0
    {}
    let command = reply::command(opcode, &reply::asking(opcode, handle));
    // SAFETY: the kernel reads the command's own length from it.
    let sent = unsafe { libc::write(fd, command.as_ptr().cast(), command.len()) };
    if sent < 0 {
        return Err(io::Error::last_os_error());
    }
    let deadline = Instant::now() + PATIENCE;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let mut waiting = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let wait = libc::c_int::try_from(left.as_millis())
            .unwrap_or(libc::c_int::MAX)
            .max(1);
        // SAFETY: one pollfd, as the count says.
        if checked(unsafe { libc::poll(&raw mut waiting, 1, wait) })? == 0 {
            continue;
        }
        // SAFETY: the kernel writes at most the buffer's length into it.
        let read = unsafe { libc::read(fd, packet.as_mut_ptr().cast(), packet.len()) };
        let Ok(read) = usize::try_from(read) else {
            return Err(io::Error::last_os_error());
        };
        match reply::answer(&packet[..read], opcode) {
            Heard::Done(parameters) => return Ok(parameters.to_vec()),
            Heard::Refused(status) => {
                return Err(io::Error::other(format!(
                    "the controller refused it ({status:#04x})"
                )));
            }
            Heard::Other => {}
        }
    }
}
