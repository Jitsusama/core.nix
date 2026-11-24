//! UDP probes that run out of hops on the way to an address, for the
//! VPN's pin (src/pin.rs). Each is sent from a chosen local port, so it
//! crosses the router and the line as a flow from that port would, with
//! its TTL set to the hop it should expire at. The router there answers
//! with ICMP time exceeded, which the kernel hands back on the probe's own
//! socket's error queue (`IP_RECVERR`), so no raw socket is needed to
//! hear it. Which hop said it isn't read: the probe's TTL names the hop,
//! and a delay is all a round compares.

use crate::pin::{self, Answer};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::io;
use std::mem::{size_of, zeroed};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

/// The number the last probe carried, counted across every socket: the
/// scout's port is probed again by a fresh socket, which hears what came
/// late to the scout's, so numbers counted per socket would start over
/// where the scout's left answers waiting.
static SENT: AtomicU8 = AtomicU8::new(0);

/// What ICMP says when a packet's time ran out on the way.
const TIME_EXCEEDED: u8 = 11;

/// How long a probe waits for its hop. The slow way's farthest hop
/// answered in about 70 ms on 22 November, and a hop that doesn't answer
/// in half a second is taken for one that never will.
const WAIT: Duration = Duration::from_millis(500);

/// One round toward `to` out of `link`: the farthest hop that answers a
/// walk from the first port, then each port's delay there, from
/// `pin::TRIES` probes each, but for the ports in `held`. A port that
/// can't be bound, or whose probes all went unanswered, reads no delay.
/// The hop is the scout's way's farthest, taken for every port's (pin
/// round 1's F20): where the other way splits off before it, each port
/// still meets a hop of its own way there, so their delays compare; where
/// the other way stops answering short of it, its ports read nothing, so
/// they can't be pinned, and a round that read fewer than `pin::HEARD`
/// ports chooses no pin.
pub fn round(link: &str, to: SocketAddrV4, held: &[u16]) -> BTreeMap<u16, f64> {
    let ports: Vec<u16> = (pin::FIRST_PORT..pin::FIRST_PORT + pin::PORTS)
        .filter(|port| !held.contains(port))
        .collect();
    // The first port that binds walks, so one some program holds doesn't
    // leave every round empty.
    let Some(scout) = ports.iter().find_map(|&p| Probe::from(link, p, to).ok()) else {
        return BTreeMap::new();
    };
    let walked: Vec<Answer> = (1..=pin::MOST_HOPS)
        .filter_map(|hop| scout.at(hop, WAIT).ok().flatten())
        .collect();
    drop(scout);
    let Some(hop) = pin::farthest(&walked) else {
        return BTreeMap::new();
    };
    ports
        .into_iter()
        .filter_map(|port| {
            let probe = Probe::from(link, port, to).ok()?;
            let answers: Vec<Answer> = (0..pin::TRIES)
                .filter_map(|_| probe.at(hop, WAIT).ok().flatten())
                .collect();
            Some((port, pin::delay(&answers, hop)?))
        })
        .collect()
}

/// A UDP socket bound to `port` on `link`, connected to `to`, whose
/// errors are kept for reading.
pub struct Probe {
    fd: OwnedFd,
}

fn checked(result: libc::c_int) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn int_option(
    fd: &OwnedFd,
    level: libc::c_int,
    name: libc::c_int,
    value: libc::c_int,
) -> io::Result<()> {
    // SAFETY: the option's value is an int, of its own size.
    checked(unsafe {
        libc::setsockopt(
            fd.as_raw_fd(),
            level,
            name,
            (&raw const value).cast(),
            libc::socklen_t::try_from(size_of::<libc::c_int>()).unwrap_or(0),
        )
    })
}

fn address(at: SocketAddrV4) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_family: libc::sa_family_t::try_from(libc::AF_INET).unwrap_or(2),
        sin_port: at.port().to_be(),
        sin_addr: libc::in_addr {
            s_addr: u32::from(*at.ip()).to_be(),
        },
        sin_zero: [0; 8],
    }
}

impl Probe {
    pub fn from(link: &str, port: u16, to: SocketAddrV4) -> io::Result<Probe> {
        // SAFETY: socket(2) with constant arguments; its result is checked
        // before it's owned. Closed on exec, since rounds run while the
        // shaper starts tc, nft and ping, which have no use for it.
        let raw = unsafe {
            libc::socket(
                libc::AF_INET,
                libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
                libc::IPPROTO_UDP,
            )
        };
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
        checked(unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_BINDTODEVICE,
                name.as_ptr().cast(),
                length,
            )
        })?;
        int_option(&fd, libc::SOL_IP, libc::IP_RECVERR, 1)?;
        let length = libc::socklen_t::try_from(size_of::<libc::sockaddr_in>()).unwrap_or(0);
        let local = address(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port));
        // SAFETY: the address is a sockaddr_in, of the length given.
        checked(unsafe { libc::bind(fd.as_raw_fd(), (&raw const local).cast(), length) })?;
        let remote = address(to);
        // SAFETY: the address is a sockaddr_in, of the length given.
        checked(unsafe { libc::connect(fd.as_raw_fd(), (&raw const remote).cast(), length) })?;
        Ok(Probe { fd })
    }

    /// Sends one probe to expire at `hop`, and waits at most `wait` for
    /// the hop to say so: the hop's answer, or none where none came.
    pub fn at(&self, hop: u8, wait: Duration) -> io::Result<Option<Answer>> {
        int_option(&self.fd, libc::SOL_IP, libc::IP_TTL, libc::c_int::from(hop))?;
        // An answer to an earlier probe that came after its wait would be
        // taken for this one's, from another hop.
        while self.error()?.is_some() {}
        // Each probe carries a number of its own, which a hop that quotes
        // the probe's data hands back, so an answer that came late to an
        // earlier probe, after the queue was emptied, isn't taken for this
        // one's. A hop that quotes only the header is believed.
        let number = SENT.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
        let sent = Instant::now();
        // SAFETY: the message is a byte, valid for its length.
        let wrote = unsafe { libc::send(self.fd.as_raw_fd(), [number].as_ptr().cast(), 1, 0) };
        if wrote < 0 {
            return Err(io::Error::last_os_error());
        }
        loop {
            let left = wait.saturating_sub(sent.elapsed());
            if left.is_zero() {
                return Ok(None);
            }
            if !self.ready(left)? {
                return Ok(None);
            }
            if let Some((expired, carried)) = self.error()? {
                if carried.is_some_and(|n| n != number) {
                    continue;
                }
                let ms = sent.elapsed().as_secs_f64() * 1_000.0;
                return Ok(expired.then_some(Answer { hop, ms }));
            }
        }
    }

    /// Whether an error waits on the socket within `wait`.
    fn ready(&self, wait: Duration) -> io::Result<bool> {
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: 0,
            revents: 0,
        };
        let ms = libc::c_int::try_from(wait.as_millis()).unwrap_or(libc::c_int::MAX);
        // SAFETY: one pollfd, valid for the call. An error queue's entry
        // shows as POLLERR, which poll sets whatever is asked.
        let ready = unsafe { libc::poll(&raw mut poll, 1, ms) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Interrupted {
                Ok(false)
            } else {
                Err(error)
            };
        }
        Ok(poll.revents & libc::POLLERR != 0)
    }

    /// Reads one entry of the error queue: whether it was a hop saying the
    /// probe's time ran out, and the number the probe carried where the
    /// hop quoted it; or none where the queue was empty.
    fn error(&self) -> io::Result<Option<(bool, Option<u8>)>> {
        let mut data = [0_u8; 64];
        let mut control = [0_u8; 512];
        let mut io = libc::iovec {
            iov_base: data.as_mut_ptr().cast(),
            iov_len: data.len(),
        };
        // SAFETY: a msghdr is plain data, zeroes its empty value.
        let mut message: libc::msghdr = unsafe { zeroed() };
        message.msg_iov = &raw mut io;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = control.len();
        // SAFETY: the message's buffers are valid for their lengths.
        let read = unsafe {
            libc::recvmsg(
                self.fd.as_raw_fd(),
                &raw mut message,
                libc::MSG_ERRQUEUE | libc::MSG_DONTWAIT,
            )
        };
        if read < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::WouldBlock {
                Ok(None)
            } else {
                Err(error)
            };
        }
        let carried = (read > 0).then_some(data[0]);
        // SAFETY: the kernel filled the control buffer with whole cmsgs,
        // up to the length it set, which the macros walk within.
        let mut header = unsafe { libc::CMSG_FIRSTHDR(&raw const message) };
        while !header.is_null() {
            // SAFETY: `header` is a cmsg inside the buffer, per the macros.
            let (level, kind) = unsafe { ((*header).cmsg_level, (*header).cmsg_type) };
            if level == libc::SOL_IP && kind == libc::IP_RECVERR {
                // SAFETY: an IP_RECVERR cmsg holds a sock_extended_err.
                let said: libc::sock_extended_err = unsafe {
                    libc::CMSG_DATA(header)
                        .cast::<libc::sock_extended_err>()
                        .read_unaligned()
                };
                let expired =
                    said.ee_origin == libc::SO_EE_ORIGIN_ICMP && said.ee_type == TIME_EXCEEDED;
                return Ok(Some((expired, carried)));
            }
            // SAFETY: as above, within the buffer the kernel filled.
            header = unsafe { libc::CMSG_NXTHDR(&raw const message, header) };
        }
        Ok(Some((false, carried)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    /// A loopback port nothing listens on. A probe sent there is answered
    /// at once with ICMP's port unreachable, which quotes the probe's data
    /// as a hop's time exceeded does, and loopback's answers are never
    /// rate-limited, so the sandbox needs no router to answer.
    fn closed() -> SocketAddrV4 {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("a loopback port didn't bind");
        let port = socket
            .local_addr()
            .expect("a bound socket had no address")
            .port();
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)
    }

    #[test]
    fn an_answer_hands_back_the_number_its_probe_carried() {
        let probe = Probe::from("lo", 0, closed()).expect("a probe didn't bind on loopback");
        // SAFETY: the message is a byte, valid for its length.
        let wrote = unsafe { libc::send(probe.fd.as_raw_fd(), [7_u8].as_ptr().cast(), 1, 0) };
        assert_eq!(wrote, 1, "the probe wasn't sent");
        assert!(
            probe.ready(WAIT).expect("poll failed"),
            "no answer came to the probe"
        );
        assert_eq!(
            probe.error().expect("the error queue didn't read"),
            Some((false, Some(7))),
            "the answer didn't quote the probe's number, or read as a hop's"
        );
        assert_eq!(
            probe.error().expect("the error queue didn't read"),
            None,
            "one answer was read twice"
        );
    }

    #[test]
    fn a_probe_numbers_itself_past_the_last_sent_and_takes_no_refusal_for_a_hop() {
        let probe = Probe::from("lo", 0, closed()).expect("a probe didn't bind on loopback");
        let before = SENT.load(Ordering::Relaxed);
        let asked = Instant::now();
        assert_eq!(
            probe.at(1, WAIT).expect("the probe wasn't sent"),
            None,
            "a port's refusal was read as a hop's answer"
        );
        // Its own refusal quoted its number, so it was taken as the answer
        // and the probe waited no longer; one that skipped it as another
        // probe's would have waited out the half second.
        assert!(
            asked.elapsed() < WAIT,
            "the probe waited out an answer that came"
        );
        // Other tests' probes may count meanwhile too, so only a change is
        // asked for.
        assert_ne!(
            SENT.load(Ordering::Relaxed),
            before,
            "the probe carried no new number"
        );
    }
}
