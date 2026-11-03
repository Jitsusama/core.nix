//! A raw ICMP socket bound to one link, for the timestamp probes and the
//! gateway's echo. Bound to the link, a probe leaves by it and not by a
//! VPN routed over it, so it measures the link's own queue. A raw socket
//! hears every ICMP message the machine receives; each prober keeps only
//! replies carrying its own identifier.

use crate::stamp::checksum;
use std::ffi::CString;
use std::io;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

const ECHO: u8 = 8;
const ECHO_REPLY: u8 = 0;

/// An echo request (a ping) from this prober, `id`, numbered `seq`.
pub fn echo_request(id: u16, seq: u16) -> [u8; 8] {
    let mut message = [ECHO, 0, 0, 0, 0, 0, 0, 0];
    message[4..6].copy_from_slice(&id.to_be_bytes());
    message[6..8].copy_from_slice(&seq.to_be_bytes());
    let sum = checksum(&message);
    message[2..4].copy_from_slice(&sum.to_be_bytes());
    message
}

/// The number of the echo this prober, `id`, sent that a packet answers,
/// its IPv4 header first; None for anything else.
pub fn echo_reply(packet: &[u8], id: u16) -> Option<u16> {
    let header = usize::from(packet.first()? & 0x0f) * 4;
    let message = packet.get(header..header + 8)?;
    let ours = message[0] == ECHO_REPLY
        && u16::from_be_bytes([message[4], message[5]]) == id
        && checksum(packet.get(header..)?) == 0;
    ours.then(|| u16::from_be_bytes([message[6], message[7]]))
}

/// The sender of a packet a raw socket read, from its IPv4 header.
pub fn source(packet: &[u8]) -> Option<Ipv4Addr> {
    let bytes: [u8; 4] = packet.get(12..16)?.try_into().ok()?;
    Some(Ipv4Addr::from(bytes))
}

/// A raw ICMP socket bound to a link, its reads waiting at most `wait`.
pub struct Socket {
    fd: OwnedFd,
}

impl Socket {
    pub fn bound(link: &str, wait: Duration) -> io::Result<Socket> {
        // SAFETY: socket(2) with constant arguments; its result is checked
        // before it's owned.
        let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `raw` is a socket just opened, owned by nothing else.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let name = CString::new(link).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let length = libc::socklen_t::try_from(name.as_bytes_with_nul().len())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: the option's value is the link's NUL-terminated name, and
        // its length is that name's.
        let bound = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_BINDTODEVICE,
                name.as_ptr().cast(),
                length,
            )
        };
        if bound < 0 {
            return Err(io::Error::last_os_error());
        }
        let timeout = libc::timeval {
            tv_sec: libc::time_t::try_from(wait.as_secs()).unwrap_or(0),
            tv_usec: libc::suseconds_t::from(wait.subsec_micros()),
        };
        // SAFETY: the option's value is a timeval, of its own size.
        let timed = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                (&raw const timeout).cast(),
                libc::socklen_t::try_from(size_of::<libc::timeval>()).unwrap_or(0),
            )
        };
        if timed < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Socket { fd })
    }

    /// Send an ICMP message to `to`.
    pub fn send(&self, to: Ipv4Addr, message: &[u8]) -> io::Result<()> {
        let address = libc::sockaddr_in {
            sin_family: libc::sa_family_t::try_from(libc::AF_INET).unwrap_or(2),
            sin_port: 0,
            sin_addr: libc::in_addr {
                s_addr: u32::from(to).to_be(),
            },
            sin_zero: [0; 8],
        };
        // SAFETY: the message and the address are valid for their lengths.
        let sent = unsafe {
            libc::sendto(
                self.fd.as_raw_fd(),
                message.as_ptr().cast(),
                message.len(),
                0,
                (&raw const address).cast(),
                libc::socklen_t::try_from(size_of::<libc::sockaddr_in>()).unwrap_or(0),
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Read one packet into `buffer`, its length, or None when none came
    /// within the socket's wait.
    pub fn receive(&self, buffer: &mut [u8]) -> io::Result<Option<usize>> {
        // SAFETY: the buffer is valid for its length.
        let read = unsafe {
            libc::recv(
                self.fd.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                0,
            )
        };
        if read >= 0 {
            return Ok(usize::try_from(read).ok());
        }
        let error = io::Error::last_os_error();
        match error.kind() {
            io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted => {
                Ok(None)
            }
            _ => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answered(kind: u8, id: u16, seq: u16) -> Vec<u8> {
        let mut packet = vec![
            0x45, 0, 0, 28, 0, 0, 0, 0, 64, 1, 0, 0, 192, 168, 4, 1, 192, 168, 4, 20,
        ];
        let mut message = vec![kind, 0, 0, 0];
        message.extend(id.to_be_bytes());
        message.extend(seq.to_be_bytes());
        let sum = checksum(&message).to_be_bytes();
        message[2] = sum[0];
        message[3] = sum[1];
        packet.extend(message);
        packet
    }

    #[test]
    fn an_echo_request_holds_its_checksum() {
        let sent = echo_request(0x1234, 9);
        assert_eq!(sent[0], 8);
        assert_eq!(&sent[4..8], &[0x12, 0x34, 0, 9]);
        assert_eq!(checksum(&sent), 0);
    }

    #[test]
    fn only_our_echo_replies_are_heard() {
        assert_eq!(echo_reply(&answered(0, 0x1234, 9), 0x1234), Some(9));
        assert_eq!(
            echo_reply(&answered(0, 0x4321, 9), 0x1234),
            None,
            "another's"
        );
        assert_eq!(
            echo_reply(&answered(8, 0x1234, 9), 0x1234),
            None,
            "a request"
        );
        assert_eq!(
            echo_reply(&answered(0, 0x1234, 9)[..24], 0x1234),
            None,
            "short"
        );
    }

    #[test]
    fn a_packet_says_who_sent_it() {
        assert_eq!(
            source(&answered(0, 1, 1)),
            Some(Ipv4Addr::new(192, 168, 4, 1))
        );
    }
}
