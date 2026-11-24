//! Echoes a network's gateway through one card, to time how well the access
//! point it's on carries.
//!
//! The echo goes out on an ICMP datagram socket, which the kernel lets any
//! group in `net.ipv4.ping_group_range` open, so wifid needs no capability
//! for it: its bounding set stays empty. The socket is bound to the card,
//! so a VPN's routes or a cable can't carry the echo another way, and to
//! an identifier drawn at random (the kernel's choice when that one is
//! taken), which the kernel sets in each echo with its checksum; it hands
//! back only the answers to this socket's, so an answer is told by its
//! sequence alone. One counts only from the gateway's address, to the
//! socket's identifier and a sequence waited on, and the first sequence is
//! drawn at random too, so a station on the same network that can't read
//! the card's frames has the identifier's 65,535 to guess before a forged
//! answer counts. One that can read them, on an open network or holding
//! its password, can deauthenticate the card as easily where management
//! frames go unprotected.
//!
//! Each card's gateway is echoed in two classes, best effort and video,
//! since an access point can hold one of a station's queues while the
//! other runs: on 23 Nov 2026 a mesh node's 5 GHz point held a card's
//! video and voice answers for 0.9 s at the median, in order, while its
//! best-effort ones came back in 6 ms, and the chooser, hearing only best
//! effort, went back to it within 81 s. Linux answers an echo in the class
//! it was asked in, so behind a gateway running it a video echo times the
//! video queue both ways; one answering unmarked would time it going up
//! only.
//!
//! An answer that comes past [`LOST`] is kept as late, with what it took,
//! rather than passed over: a queue held for seconds answers late, in
//! order, where a gateway that drops an echo never answers it, and only
//! the first says anything of the air.

use std::collections::VecDeque;
use std::ffi::CString;
use std::hash::{BuildHasher, RandomState};
use std::io;
use std::mem;
use std::net::Ipv4Addr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;
use tokio::time::{Duration, Instant, timeout};

use crate::measure::{Echo, SPAN};

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

    /// What it's called in the journal.
    pub fn name(self) -> &'static str {
        match self {
            Class::BestEffort => "best effort",
            Class::Video => "video",
        }
    }
}

/// An answer that came past [`LOST`], to the echo sent at `sent`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Late {
    pub sent: Instant,
    pub took: Duration,
}

/// What a card's echoes still wait on, and the late answers heard since
/// they were last taken. An echo not answered within [`LOST`] is kept by
/// its sequence and when it was sent, so an answer to it that comes later
/// is late. One sent a span ago is let go: its echo has left what's
/// measured, and a second echo can't have its sequence until the sequence
/// comes round, some 18 hours on.
#[derive(Default)]
struct Ledger {
    waiting: VecDeque<(u16, Instant)>,
    late: Vec<Late>,
}

impl Ledger {
    /// Keeps the echo sent at `sent` with this sequence as unanswered, and
    /// lets go of any sent more than a span before it.
    fn wait(&mut self, sequence: u16, sent: Instant) {
        while let Some(&(_, before)) = self.waiting.front() {
            if sent.duration_since(before) <= SPAN {
                break;
            }
            self.waiting.pop_front();
        }
        self.waiting.push_back((sequence, sent));
    }

    /// Takes in an answer with this sequence, read at `read` while the echo
    /// with `awaited`'s is waited on, saying whether it's that one's. One
    /// to an echo still waited on is kept as late; each is late once.
    fn heard(&mut self, sequence: u16, awaited: u16, read: Instant) -> bool {
        if sequence == awaited {
            return true;
        }
        let held = self
            .waiting
            .iter()
            .position(|&(asked, sent)| asked == sequence && read.duration_since(sent) <= SPAN);
        if let Some((_, sent)) = held.and_then(|held| self.waiting.remove(held)) {
            self.late.push(Late {
                sent,
                took: read.duration_since(sent),
            });
        }
        false
    }

    /// The late answers heard since this was last asked.
    fn late(&mut self) -> Vec<Late> {
        mem::take(&mut self.late)
    }
}

/// A card's own socket for echoes.
pub struct Echoer {
    socket: AsyncFd<OwnedFd>,
    sequence: u16,
    ledger: Ledger,
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
        mark(&owned, class)?;
        // An identifier and a first sequence drawn at random, so answering
        // an echo in the gateway's place means guessing them. Not 0, which
        // asks the kernel to choose, in order: that is left for when the
        // one drawn is taken, so a user holding most identifiers can't
        // leave the card unmeasured, though they can then guess its own.
        let [low, high, first, second, ..] = RandomState::new().hash_one(card).to_le_bytes();
        claim(&owned, u16::from_le_bytes([first, second]).max(1))?;
        Ok(Echoer {
            socket: AsyncFd::with_interest(owned, Interest::READABLE)?,
            sequence: u16::from_le_bytes([low, high]),
            ledger: Ledger::default(),
        })
    }

    /// Echoes the gateway once, saying when it was sent and how long its
    /// answer took, or that none came within [`LOST`].
    pub async fn echo(&mut self, gateway: Ipv4Addr) -> io::Result<(Instant, Echo)> {
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
        if let Ok(answered) = timeout(LOST, self.answer(sequence, gateway)).await {
            return answered.map(|()| (sent_at, Echo::Answered(sent_at.elapsed())));
        }
        self.ledger.wait(sequence, sent_at);
        Ok((sent_at, Echo::Lost))
    }

    /// The late answers read since this was last asked.
    pub fn late(&mut self) -> Vec<Late> {
        self.ledger.late()
    }

    /// Waits for the answer to the echo sent with this sequence, keeping
    /// any answer to an earlier one counted lost as late. A late answer is
    /// timed as it's read, which is as it comes while a later echo is
    /// waited on: one coming between echoes is timed at most a second
    /// slower than it was, which leaves it late all the same. Only the
    /// gateway's answers are heard.
    async fn answer(&mut self, sequence: u16, gateway: Ipv4Addr) -> io::Result<()> {
        let Echoer { socket, ledger, .. } = self;
        let mut datagram = [0u8; 64 + CARRIED];
        loop {
            let mut ready = socket.readable().await?;
            let read = ready.try_io(|socket| {
                // SAFETY: all zeros is an address, which the kernel fills.
                let mut from: libc::sockaddr_in = unsafe { mem::zeroed() };
                let mut length =
                    libc::socklen_t::try_from(mem::size_of_val(&from)).map_err(io::Error::other)?;
                // SAFETY: the buffer and the address outlive the call, and
                // each length is its own.
                let read = unsafe {
                    libc::recvfrom(
                        socket.as_raw_fd(),
                        datagram.as_mut_ptr().cast(),
                        datagram.len(),
                        0,
                        (&raw mut from).cast(),
                        &raw mut length,
                    )
                };
                let read = usize::try_from(read).map_err(|_| io::Error::last_os_error())?;
                Ok((read, Ipv4Addr::from(u32::from_be(from.sin_addr.s_addr))))
            });
            let Ok(read) = read else {
                continue;
            };
            let (read, from) = read?;
            if from != gateway {
                continue;
            }
            if let Some(answered) = answered(&datagram[..read])
                && ledger.heard(answered, sequence, Instant::now())
            {
                return Ok(());
            }
        }
    }
}

/// Marks the socket's echoes for the class's queue, leaving best effort's
/// unmarked.
fn mark(socket: &OwnedFd, class: Class) -> io::Result<()> {
    let tos = class.tos();
    if tos == 0 {
        return Ok(());
    }
    let length = libc::socklen_t::try_from(mem::size_of_val(&tos)).map_err(io::Error::other)?;
    // SAFETY: the value outlives the call, and the length is its own.
    let marked = unsafe {
        libc::setsockopt(
            socket.as_raw_fd(),
            libc::IPPROTO_IP,
            libc::IP_TOS,
            (&raw const tos).cast(),
            length,
        )
    };
    if marked < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Binds the socket to the identifier drawn, or to one the kernel chooses
/// when another socket holds it.
fn claim(socket: &OwnedFd, drawn: u16) -> io::Result<()> {
    match identify(socket, drawn) {
        Err(taken) if taken.kind() == io::ErrorKind::AddrInUse => identify(socket, 0),
        bound => bound,
    }
}

/// Binds the socket to the identifier its echoes carry.
fn identify(socket: &OwnedFd, identifier: u16) -> io::Result<()> {
    let address = libc::sockaddr_in {
        sin_family: libc::sa_family_t::try_from(libc::AF_INET).map_err(io::Error::other)?,
        sin_port: identifier.to_be(),
        sin_addr: libc::in_addr {
            s_addr: libc::INADDR_ANY,
        },
        sin_zero: [0; 8],
    };
    let length = libc::socklen_t::try_from(mem::size_of_val(&address)).map_err(io::Error::other)?;
    // SAFETY: the address outlives the call, and the length is its own.
    let bound = unsafe { libc::bind(socket.as_raw_fd(), (&raw const address).cast(), length) };
    if bound < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// An echo request with this sequence, its identifier and checksum left
/// for the kernel.
fn request(sequence: u16) -> Vec<u8> {
    let mut request = vec![REQUEST, 0, 0, 0, 0, 0];
    request.extend(sequence.to_be_bytes());
    request.resize(8 + CARRIED, 0);
    request
}

/// The sequence of the echo what the socket read answers, if it's an
/// answer.
fn answered(read: &[u8]) -> Option<u16> {
    if read.first() != Some(&REPLY) {
        return None;
    }
    let sequence = read.get(6..8)?;
    Some(u16::from_be_bytes([sequence[0], sequence[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A datagram socket, whose port is where a ping socket's identifier
    /// is kept, bound alike.
    fn udp() -> OwnedFd {
        // SAFETY: it takes only numbers.
        let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
        assert!(fd >= 0, "a socket: {}", io::Error::last_os_error());
        // SAFETY: the descriptor was just made, and nothing else holds it.
        unsafe { OwnedFd::from_raw_fd(fd) }
    }

    /// The identifier the socket is bound to, as the kernel reads it.
    fn identifier_of(socket: &OwnedFd) -> u16 {
        // SAFETY: all zeros is an address, which the kernel fills.
        let mut bound: libc::sockaddr_in = unsafe { mem::zeroed() };
        let mut length = libc::socklen_t::try_from(mem::size_of_val(&bound)).unwrap();
        // SAFETY: the address and its length outlive the call.
        let named = unsafe {
            libc::getsockname(socket.as_raw_fd(), (&raw mut bound).cast(), &raw mut length)
        };
        assert_eq!(named, 0, "its name: {}", io::Error::last_os_error());
        u16::from_be(bound.sin_port)
    }

    #[test]
    fn a_socket_is_bound_to_its_identifier_and_refused_one_taken() {
        // The kernel's choice is one free wherever the test runs.
        let first = udp();
        identify(&first, 0).expect("the kernel chooses one free");
        let identifier = identifier_of(&first);

        // Bound in the wrong order, it would land on another, free one.
        let taken = identify(&udp(), identifier).expect_err("one taken is refused");
        assert_eq!(taken.kind(), io::ErrorKind::AddrInUse, "as taken: {taken}");
    }

    #[test]
    fn an_identifier_drawn_that_is_taken_gives_way_to_the_kernel_s_choice() {
        let first = udp();
        identify(&first, 0).expect("the kernel chooses one free");
        let taken = identifier_of(&first);

        let second = udp();
        claim(&second, taken).expect("another is claimed");
        let claimed = identifier_of(&second);
        assert_ne!(claimed, 0, "bound");
        assert_ne!(claimed, taken, "elsewhere");

        let third = udp();
        claim(&third, 0).expect("the kernel chooses one free");
        let free = identifier_of(&third);
        drop(third);
        let fourth = udp();
        claim(&fourth, free).expect("a free one drawn is claimed");
        assert_eq!(identifier_of(&fourth), free, "as drawn");
    }

    #[test]
    fn an_echo_request_carries_its_sequence_and_what_ping_would() {
        let request = request(0x0102);

        assert_eq!(request.len(), 8 + CARRIED, "a header and ping's 56 bytes");
        assert_eq!(request[0], 8, "an echo request");
        assert_eq!(request[1], 0, "of code 0");
        assert_eq!(&request[6..8], &[1, 2], "its sequence, in network order");
    }

    #[test]
    fn an_echo_reply_is_told_by_its_sequence_and_nothing_else_is_one() {
        let mut answer = request(7);
        answer[0] = 0;

        let mut earlier = answer.clone();
        earlier[7] = 6;
        let mut unreachable = answer.clone();
        unreachable[0] = 3;

        assert_eq!(
            answered(&answer),
            Some(7),
            "an echo reply with its sequence"
        );
        assert_eq!(
            answered(&earlier),
            Some(6),
            "a late answer to the one before"
        );
        assert_eq!(
            answered(&unreachable),
            None,
            "a gateway unreachable isn't an answer"
        );
        assert_eq!(answered(&answer[..4]), None, "nor is a header cut short");
    }

    #[test]
    fn an_answer_to_an_echo_counted_lost_is_late_at_what_it_took() {
        // Two echoes held past their second, then let go in order while
        // the next is waited on, as a held queue answers.
        let start = Instant::now();
        let mut ledger = Ledger::default();
        ledger.wait(4, start);
        ledger.wait(5, start + Duration::from_secs(1));

        let read = start + Duration::from_millis(2400);
        assert!(!ledger.heard(5, 6, read), "not the one waited on");
        assert!(!ledger.heard(5, 6, read), "nor when said twice");
        assert!(!ledger.heard(4, 6, read), "nor the one before");
        assert!(!ledger.heard(9, 6, read), "nor one never sent");
        assert!(ledger.heard(6, 6, read), "the one waited on is its own");
        assert_eq!(
            ledger.late(),
            vec![
                Late {
                    sent: start + Duration::from_secs(1),
                    took: Duration::from_millis(1400),
                },
                Late {
                    sent: start,
                    took: Duration::from_millis(2400),
                },
            ],
            "each late once, at what it took, in the order heard"
        );
        assert_eq!(ledger.late(), Vec::new(), "and taken once");
    }

    #[test]
    fn an_echo_sent_a_span_ago_is_waited_on_no_more() {
        let start = Instant::now();
        let mut ledger = Ledger::default();
        ledger.wait(1, start);
        ledger.wait(2, start + Duration::from_secs(1));

        let now = start + SPAN + Duration::from_secs(1);
        ledger.heard(1, 9, now);
        assert_eq!(
            ledger.late(),
            Vec::new(),
            "its echo has left what's measured"
        );
        ledger.heard(2, 9, now);
        assert_eq!(ledger.late().len(), 1, "while one a span back is late");
        ledger.wait(3, now);
        assert_eq!(
            ledger.waiting,
            VecDeque::from([(3, now)]),
            "and the one past it is let go as the next is waited on"
        );
    }

    #[test]
    fn a_video_socket_s_echoes_carry_the_mark_and_best_effort_s_none() {
        let tos_of = |class| {
            // SAFETY: it takes only numbers.
            let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
            assert!(fd >= 0, "a datagram socket opens");
            // SAFETY: the descriptor was just made, and nothing else holds it.
            let socket = unsafe { OwnedFd::from_raw_fd(fd) };
            mark(&socket, class).expect("the socket takes its mark");
            let mut tos: libc::c_int = -1;
            let mut length = libc::socklen_t::try_from(mem::size_of_val(&tos)).unwrap();
            // SAFETY: the value and its length outlive the call.
            let read = unsafe {
                libc::getsockopt(
                    socket.as_raw_fd(),
                    libc::IPPROTO_IP,
                    libc::IP_TOS,
                    (&raw mut tos).cast(),
                    &raw mut length,
                )
            };
            assert_eq!(read, 0, "its mark reads back");
            tos
        };
        assert_eq!(tos_of(Class::Video), 0xa0, "CS5, video's");
        assert_eq!(tos_of(Class::BestEffort), 0, "unmarked");
    }
}
