//! Echoes a network's gateway through one card, to time how well the access
//! point it's on carries.
//!
//! The echo goes out on an ICMP datagram socket, which the kernel lets any
//! group in `net.ipv4.ping_group_range` open, so wifid needs no capability
//! for it: its bounding set stays empty. The socket is bound to the card,
//! so a VPN's routes or a cable can't carry the echo another way. The
//! kernel sets each echo's identifier and checksum, and hands back only the
//! answers to this socket's, so an answer is told by its sequence alone.
//!
//! Each card's gateway is echoed in two classes, best effort and video,
//! since an access point can hold one of a station's queues while the
//! other runs: on 23 Nov 2026 a mesh node's 5 GHz point held a card's
//! video and voice answers for 0.9 s at the median, in order, while its
//! best-effort ones came back in 6 ms, and the chooser, hearing only best
//! effort, went back to it within 81 s. A gateway answers an echo in the
//! class it was asked in, so a video echo times the video queue both ways.

use std::ffi::CString;
use std::io;
use std::mem;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use tokio::time::{Duration, Instant, timeout};

use crate::measure::Echo;

/// How long an echo is waited for before it counts as lost: past what
/// anyone would bear on a call.
pub const LOST: Duration = Duration::from_secs(1);

/// ICMP's echo request and its answer.
const REQUEST: u8 = 8;
const REPLY: u8 = 0;

/// What each echo carries beyond its header, as `ping` sends by default.
const CARRIED: usize = 56;

/// The class an echo is sent in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Unmarked, as most of what a card carries goes.
    BestEffort,
    /// Marked CS5, which Wi-Fi sends in its video queue, as a call marked
    /// for one would go.
    Video,
}

impl Class {
    /// The type of service its echoes carry: Wi-Fi takes a frame's queue
    /// from the top three bits, 5 being video's.
    fn tos(self) -> libc::c_int {
        match self {
            Class::BestEffort => 0,
            Class::Video => 0xa0,
        }
    }
}

/// A card's own socket for echoes.
pub struct Echoer {
    socket: AsyncFd<OwnedFd>,
    sequence: u16,
}

impl Echoer {
    /// Opens an echo socket bound to the card named, sending in the class
    /// given.
    pub fn open(card: &str, class: Class) -> io::Result<Echoer> {
        // SAFETY: it takes only numbers.
        let fd = unsafe {
            libc::socket(
                libc::AF_INET,
                libc::SOCK_DGRAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                libc::IPPROTO_ICMP,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the descriptor was just made, and nothing else holds it.
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        let name = CString::new(card).map_err(io::Error::other)?;
        let length =
            libc::socklen_t::try_from(name.as_bytes_with_nul().len()).map_err(io::Error::other)?;
        // SAFETY: the name is a C string that outlives the call, and the
        // length is its own.
        let bound = unsafe {
            libc::setsockopt(
                owned.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_BINDTODEVICE,
                name.as_ptr().cast(),
                length,
            )
        };
        if bound < 0 {
            return Err(io::Error::last_os_error());
        }
        let tos = class.tos();
        if tos != 0 {
            let length =
                libc::socklen_t::try_from(mem::size_of_val(&tos)).map_err(io::Error::other)?;
            // SAFETY: the value outlives the call, and the length is its own.
            let marked = unsafe {
                libc::setsockopt(
                    owned.as_raw_fd(),
                    libc::IPPROTO_IP,
                    libc::IP_TOS,
                    (&raw const tos).cast(),
                    length,
                )
            };
            if marked < 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(Echoer {
            socket: AsyncFd::with_interest(owned, Interest::READABLE)?,
            sequence: 0,
        })
    }

    /// Echoes the gateway once, saying how long its answer took, or that
    /// none came within [`LOST`].
    pub async fn echo(&mut self, gateway: Ipv4Addr) -> io::Result<Echo> {
        self.sequence = self.sequence.wrapping_add(1);
        let sequence = self.sequence;
        let message = request(sequence);
        let to = libc::sockaddr_in {
            sin_family: libc::sa_family_t::try_from(libc::AF_INET).map_err(io::Error::other)?,
            sin_port: 0,
            sin_addr: libc::in_addr {
                s_addr: u32::from(gateway).to_be(),
            },
            sin_zero: [0; 8],
        };
        let length = libc::socklen_t::try_from(mem::size_of::<libc::sockaddr_in>())
            .map_err(io::Error::other)?;
        let sent_at = Instant::now();
        // SAFETY: the message and the address outlive the call, and each
        // length is its own.
        let sent = unsafe {
            libc::sendto(
                self.socket.as_raw_fd(),
                message.as_ptr().cast(),
                message.len(),
                0,
                (&raw const to).cast(),
                length,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        match timeout(LOST, self.answer(sequence)).await {
            Ok(answered) => answered.map(|()| Echo::Answered(sent_at.elapsed())),
            Err(_) => Ok(Echo::Lost),
        }
    }

    /// Waits for the answer to the echo sent with this sequence, passing
    /// over late answers to earlier ones.
    async fn answer(&self, sequence: u16) -> io::Result<()> {
        let mut datagram = [0u8; 64 + CARRIED];
        loop {
            let mut ready = self.socket.readable().await?;
            let read = ready.try_io(|socket| {
                // SAFETY: the buffer outlives the call, and the length is
                // its own.
                let read = unsafe {
                    libc::recv(
                        socket.as_raw_fd(),
                        datagram.as_mut_ptr().cast(),
                        datagram.len(),
                        0,
                    )
                };
                usize::try_from(read).map_err(|_| io::Error::last_os_error())
            });
            let Ok(read) = read else {
                continue;
            };
            if answers(&datagram[..read?], sequence) {
                return Ok(());
            }
        }
    }
}

/// An echo request with this sequence, its identifier and checksum left
/// for the kernel.
fn request(sequence: u16) -> Vec<u8> {
    let mut request = vec![REQUEST, 0, 0, 0, 0, 0];
    request.extend(sequence.to_be_bytes());
    request.resize(8 + CARRIED, 0);
    request
}

/// Whether what the socket read is the answer to the echo with this
/// sequence.
fn answers(read: &[u8], sequence: u16) -> bool {
    read.first() == Some(&REPLY) && read.get(6..8) == Some(&sequence.to_be_bytes()[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_echo_request_carries_its_sequence_and_what_ping_would() {
        let request = request(0x0102);

        assert_eq!(request.len(), 8 + CARRIED, "a header and ping's 56 bytes");
        assert_eq!(request[0], 8, "an echo request");
        assert_eq!(request[1], 0, "of code 0");
        assert_eq!(&request[6..8], &[1, 2], "its sequence, in network order");
    }

    #[test]
    fn a_video_echo_goes_in_wi_fis_video_queue_and_best_effort_unmarked() {
        assert_eq!(Class::Video.tos() >> 5, 5, "user priority 5, video's");
        assert_eq!(Class::BestEffort.tos(), 0, "best effort carries no mark");
    }

    #[test]
    fn only_the_answer_to_this_echo_is_its_answer() {
        let mut answer = request(7);
        answer[0] = 0;

        let mut earlier = answer.clone();
        earlier[7] = 6;
        let mut unreachable = answer.clone();
        unreachable[0] = 3;

        assert!(answers(&answer, 7), "an echo reply with its sequence");
        assert!(!answers(&earlier, 7), "a late answer to the one before");
        assert!(
            !answers(&unreachable, 7),
            "a gateway unreachable isn't an answer"
        );
        assert!(!answers(&answer[..4], 7), "nor is a header cut short");
    }
}
