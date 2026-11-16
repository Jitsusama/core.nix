//! Every TCP socket on this machine with what it has sent, read from the
//! kernel's socket diagnostics over netlink (`sock_diag`), as `ss -ti` reads
//! them, without running it: the daemon samples often, and a program per
//! sample is the slow way.

use std::io;
use std::mem::size_of;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// One TCP socket and its counters, as the kernel says them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socket {
    /// The kernel's cookie for it, which no other socket shares while it lives.
    pub cookie: u64,
    pub local: IpAddr,
    pub peer: IpAddr,
    pub peer_port: u16,
    pub uid: u32,
    pub inode: u32,
    /// Bytes handed to the wire, resends included.
    pub bytes_sent: u64,
    pub bytes_retrans: u64,
}

// The kernel's numbers, from linux/netlink.h, linux/sock_diag.h and
// linux/inet_diag.h; libc doesn't carry the diag ones.
const SOCK_DIAG_BY_FAMILY: u16 = 20;
const NLMSG_DONE: u16 = 3;
const NLMSG_ERROR: u16 = 2;
const INET_DIAG_INFO: u16 = 2;
const HEADER: usize = 16;
const REQUEST: usize = 56;
const DIAG_MSG: usize = 72;
// Every state but listening, closed and the time-wait husks, which send
// nothing: the bitmask is 1 << state for ESTABLISHED (1) through
// CLOSING (11), less TIME_WAIT (6), CLOSE (7) and LISTEN (10).
const SENDING_STATES: u32 =
    (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5) | (1 << 8) | (1 << 9) | (1 << 11);
// Where struct tcp_info keeps the two counters read (linux/tcp.h), the
// last fields a kernel before 4.19 lacks; a shorter answer reads none.
const BYTES_SENT_AT: usize = 200;
const BYTES_RETRANS_AT: usize = 208;
/// The address families, as the request's one byte carries them.
const INET: u8 = 2;
const INET6: u8 = 10;

/// Every TCP socket over IPv4 and IPv6 that can be sending.
pub fn sockets() -> io::Result<Vec<Socket>> {
    let mut all = dump(INET)?;
    all.extend(dump(INET6)?);
    Ok(all)
}

fn dump(family: u8) -> io::Result<Vec<Socket>> {
    let link = Link::open()?;
    link.send(&request(family))?;
    let mut found = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = link.receive(&mut buffer)?;
        if read == 0 {
            return Ok(found);
        }
        match messages(&buffer[..read], &mut found)? {
            Walked::Done => return Ok(found),
            Walked::More => {}
        }
    }
}

/// The dump request for one family: every sending state, with `tcp_info`.
fn request(family: u8) -> Vec<u8> {
    let length = u32::try_from(HEADER + REQUEST).unwrap_or(u32::MAX);
    let flags = u16::try_from(libc::NLM_F_REQUEST | libc::NLM_F_DUMP).unwrap_or(0);
    let mut bytes = Vec::with_capacity(HEADER + REQUEST);
    bytes.extend_from_slice(&length.to_ne_bytes());
    bytes.extend_from_slice(&SOCK_DIAG_BY_FAMILY.to_ne_bytes());
    bytes.extend_from_slice(&flags.to_ne_bytes());
    bytes.extend_from_slice(&1u32.to_ne_bytes());
    bytes.extend_from_slice(&0u32.to_ne_bytes());
    bytes.push(family);
    bytes.push(u8::try_from(libc::IPPROTO_TCP).unwrap_or(6));
    bytes.push(1 << (INET_DIAG_INFO - 1));
    bytes.push(0);
    bytes.extend_from_slice(&SENDING_STATES.to_ne_bytes());
    bytes.resize(HEADER + REQUEST, 0);
    bytes
}

enum Walked {
    Done,
    More,
}

/// Walks one read's netlink messages, keeping each socket found.
fn messages(mut bytes: &[u8], found: &mut Vec<Socket>) -> io::Result<Walked> {
    while bytes.len() >= HEADER {
        let length = u32_at(bytes, 0) as usize;
        let kind = u16::from_ne_bytes([bytes[4], bytes[5]]);
        if length < HEADER || length > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "the kernel's socket list was cut short",
            ));
        }
        match kind {
            NLMSG_DONE => return Ok(Walked::Done),
            NLMSG_ERROR => {
                let code = i32::from_ne_bytes(bytes[16..20].try_into().unwrap_or([0; 4]));
                return Err(io::Error::from_raw_os_error(-code));
            }
            SOCK_DIAG_BY_FAMILY => {
                if let Some(socket) = socket(&bytes[HEADER..length]) {
                    found.push(socket);
                }
            }
            _ => {}
        }
        bytes = &bytes[aligned(length).min(bytes.len())..];
    }
    Ok(Walked::More)
}

/// One `inet_diag_msg` and its attributes, or nothing for one without a
/// `tcp_info` long enough to hold what's sent.
pub(super) fn socket(body: &[u8]) -> Option<Socket> {
    if body.len() < DIAG_MSG {
        return None;
    }
    let family = body[0];
    let peer_port = u16::from_be_bytes([body[6], body[7]]);
    let local = address(family, &body[8..24])?;
    let peer = address(family, &body[24..40])?;
    let cookie = u64::from(u32_at(body, 44)) | (u64::from(u32_at(body, 48)) << 32);
    let uid = u32_at(body, 64);
    let inode = u32_at(body, 68);
    let mut attributes = &body[DIAG_MSG..];
    while attributes.len() >= 4 {
        let length = usize::from(u16::from_ne_bytes([attributes[0], attributes[1]]));
        let kind = u16::from_ne_bytes([attributes[2], attributes[3]]);
        if length < 4 || length > attributes.len() {
            return None;
        }
        if kind == INET_DIAG_INFO {
            let info = &attributes[4..length];
            if info.len() < BYTES_RETRANS_AT + 8 {
                return None;
            }
            return Some(Socket {
                cookie,
                local,
                peer,
                peer_port,
                uid,
                inode,
                bytes_sent: u64_at(info, BYTES_SENT_AT),
                bytes_retrans: u64_at(info, BYTES_RETRANS_AT),
            });
        }
        attributes = &attributes[aligned(length).min(attributes.len())..];
    }
    None
}

fn address(family: u8, bytes: &[u8]) -> Option<IpAddr> {
    let octets: [u8; 16] = bytes.try_into().ok()?;
    match family {
        INET => Some(IpAddr::V4(Ipv4Addr::new(
            octets[0], octets[1], octets[2], octets[3],
        ))),
        INET6 => {
            let v6 = Ipv6Addr::from(octets);
            // A v4 peer on a v6 socket is the v4 peer, for every reader.
            Some(v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4))
        }
        _ => None,
    }
}

fn aligned(length: usize) -> usize {
    (length + 3) & !3
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]))
}

fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
}

/// A netlink socket to the kernel's socket diagnostics, closed when dropped.
struct Link(libc::c_int);

impl Link {
    fn open() -> io::Result<Self> {
        // SAFETY: a plain socket(2) call; the descriptor it returns is
        // owned by the Link from here and closed once, in drop.
        let fd = unsafe {
            libc::socket(
                libc::AF_NETLINK,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                libc::NETLINK_SOCK_DIAG,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(fd))
    }

    fn send(&self, bytes: &[u8]) -> io::Result<()> {
        // SAFETY: an all-zero sockaddr_nl is valid; the kernel is pid 0.
        let mut kernel: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
        kernel.nl_family = u16::try_from(libc::AF_NETLINK).unwrap_or(16);
        let size = u32::try_from(size_of::<libc::sockaddr_nl>()).unwrap_or(12);
        // SAFETY: the buffer and the address outlive the call, and their
        // lengths are the ones given.
        let sent = unsafe {
            libc::sendto(
                self.0,
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
                std::ptr::from_ref(&kernel).cast(),
                size,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn receive(&self, buffer: &mut [u8]) -> io::Result<usize> {
        // SAFETY: the kernel writes at most buffer.len() bytes into it.
        let read = unsafe { libc::recv(self.0, buffer.as_mut_ptr().cast(), buffer.len(), 0) };
        usize::try_from(read).map_err(|_| io::Error::last_os_error())
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        // SAFETY: the descriptor is the Link's own and is closed only here.
        unsafe {
            libc::close(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::{TcpListener, TcpStream};

    #[test]
    fn a_connection_made_here_is_listed_with_what_it_sent() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let (_server, _) = listener.accept().unwrap();
        client.write_all(&[7u8; 5000]).unwrap();
        client.flush().unwrap();

        let all = sockets().unwrap();
        let ours = all
            .iter()
            .find(|socket| socket.peer_port == port && socket.peer.is_loopback())
            .expect("the client's socket is listed");
        assert_eq!(ours.bytes_sent, 5000);
        assert_eq!(ours.bytes_retrans, 0);
        assert_ne!(ours.inode, 0);
    }

    #[test]
    fn a_message_too_short_for_tcp_info_reads_as_none() {
        assert_eq!(socket(&[0u8; DIAG_MSG - 1]), None);
        let mut body = vec![0u8; DIAG_MSG];
        body[0] = u8::try_from(libc::AF_INET).unwrap();
        assert_eq!(socket(&body), None);
    }
}
