//! The call signal: how long a call's packets take to cross the line and
//! come back while the work behind Joel fills it.
//!
//! It asks a STUN server for this machine's address, as a call does before
//! it starts, fifty times a second, as often as a call sends its sound, and
//! times each answer. A request is an app's UDP to a port other than 443,
//! so the shaper sends it as a call, and its answer comes back through the
//! same queue, so each time is the round trip a call's sound would have.
//! Google's server sits among Meet's media servers, so the path is a call's
//! too.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Where the requests go: Google's STUN server, which Meet uses.
pub const SERVER: &str = "stun.l.google.com:19302";
/// How often a request goes: as often as a call sends its sound.
pub const EVERY: Duration = Duration::from_millis(20);
/// How long an answer may take before its request counts as lost. A call
/// would long since have played silence in its place.
pub const PATIENCE: Duration = Duration::from_secs(1);

/// The message types and the fixed cookie of RFC 8489, section 5.
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_ANSWER: u16 = 0x0101;
const MAGIC_COOKIE: u32 = 0x2112_A442;
/// Every STUN message opens with a header of this many bytes.
const HEADER: usize = 20;

/// A request's transaction id, which its answer carries back.
pub type Id = [u8; 12];

/// What a run found: each answered request's round trip, and how many
/// requests went unanswered.
#[derive(Debug, Default)]
pub struct Calls {
    pub times: Vec<Duration>,
    pub lost: usize,
}

/// A binding request: the header alone, with no attributes.
pub fn request(id: Id) -> [u8; HEADER] {
    let mut packet = [0; HEADER];
    packet[0..2].copy_from_slice(&BINDING_REQUEST.to_be_bytes());
    // Bytes 2 and 3 are the length of what follows the header: nothing.
    packet[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    packet[8..].copy_from_slice(&id);
    packet
}

/// The id a binding answer carries, or nothing when the datagram isn't one.
pub fn answered(datagram: &[u8]) -> Option<Id> {
    let header = datagram.get(..HEADER)?;
    let kind = u16::from_be_bytes([header[0], header[1]]);
    let cookie = u32::from_be_bytes([header[4], header[5], header[6], header[7]]);
    if kind != BINDING_ANSWER || cookie != MAGIC_COOKIE {
        return None;
    }
    header[8..].try_into().ok()
}

/// Asks `server` at a call's pace for `length`.
pub fn watch(server: &str, length: Duration) -> Result<Calls, String> {
    let address = server
        .to_socket_addrs()
        .map_err(|e| format!("finding {server}: {e}"))?
        .next()
        .ok_or_else(|| format!("{server} has no address"))?;
    paced(address, length, EVERY, PATIENCE)
}

/// Sends `server` a request every `every` for `length`, and counts one
/// unanswered after `patience` as lost.
pub fn paced(
    server: SocketAddr,
    length: Duration,
    every: Duration,
    patience: Duration,
) -> Result<Calls, String> {
    let here: SocketAddr = if server.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(here).map_err(|e| format!("opening a socket: {e}"))?;
    // Connected, the socket hears only the server.
    socket
        .connect(server)
        .map_err(|e| format!("reaching {server}: {e}"))?;
    let mut ids = Ids::new();
    let mut waiting: HashMap<Id, Instant> = HashMap::new();
    let mut calls = Calls::default();
    let end = Instant::now() + length;
    let mut send_at = Instant::now();
    let mut datagram = [0; 576];
    loop {
        let now = Instant::now();
        waiting.retain(|_, sent| {
            let late = now.duration_since(*sent) >= patience;
            calls.lost += usize::from(late);
            !late
        });
        let sending = send_at < end;
        if sending && now >= send_at {
            let id = ids.mint();
            socket
                .send(&request(id))
                .map_err(|e| format!("asking {server}: {e}"))?;
            waiting.insert(id, Instant::now());
            // A probe held off its core skips what it missed rather than
            // sending it all at once, as a call's sound would.
            send_at = (send_at + every).max(now);
            continue;
        }
        if !sending && waiting.is_empty() {
            return Ok(calls);
        }
        let expiry = waiting.values().min().map(|sent| *sent + patience);
        let deadline = match (sending.then_some(send_at), expiry) {
            (Some(send), Some(expiry)) => send.min(expiry),
            (Some(at), None) | (None, Some(at)) => at,
            (None, None) => now,
        };
        // A read timeout of zero is refused, so the shortest wait is a
        // millisecond.
        let wait = deadline
            .saturating_duration_since(now)
            .max(Duration::from_millis(1));
        socket
            .set_read_timeout(Some(wait))
            .map_err(|e| format!("waiting on {server}: {e}"))?;
        match socket.recv(&mut datagram) {
            Ok(length) => {
                let sent = answered(&datagram[..length]).and_then(|id| waiting.remove(&id));
                if let Some(sent) = sent {
                    calls.times.push(sent.elapsed());
                }
            }
            // A refusal is the network saying a request went nowhere; it
            // stays waited on and counts as lost in its turn.
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::WouldBlock
                        | ErrorKind::TimedOut
                        | ErrorKind::Interrupted
                        | ErrorKind::ConnectionRefused
                ) => {}
            Err(e) => return Err(format!("hearing from {server}: {e}")),
        }
    }
}

/// Transaction ids unique to a run: when it began, then a count.
struct Ids {
    run: u32,
    count: u64,
}

impl Ids {
    fn new() -> Ids {
        let run = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.subsec_nanos());
        Ids { run, count: 0 }
    }

    fn mint(&mut self) -> Id {
        let mut id = [0; 12];
        id[..4].copy_from_slice(&self.run.to_be_bytes());
        id[4..].copy_from_slice(&self.count.to_be_bytes());
        self.count += 1;
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// A STUN server's stand-in on this machine, answering every request
    /// but each `drop`th, until nothing has come for a second.
    fn stand_in(drop: Option<usize>) -> (SocketAddr, thread::JoinHandle<()>) {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).expect("binding the stand-in");
        let address = socket.local_addr().expect("the stand-in's address");
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("the stand-in's timeout");
        let server = thread::spawn(move || {
            let mut datagram = [0; 576];
            let mut heard = 0;
            while let Ok((length, from)) = socket.recv_from(&mut datagram) {
                heard += 1;
                if drop.is_some_and(|every| heard % every == 0) || length < HEADER {
                    continue;
                }
                let mut answer = [0; HEADER];
                answer.copy_from_slice(&datagram[..HEADER]);
                answer[0..2].copy_from_slice(&BINDING_ANSWER.to_be_bytes());
                socket.send_to(&answer, from).expect("answering");
            }
        });
        (address, server)
    }

    #[test]
    fn a_request_is_a_bare_binding_header_carrying_its_id() {
        let id = [7; 12];
        let packet = request(id);
        assert_eq!(packet[0..2], [0x00, 0x01]);
        assert_eq!(packet[2..4], [0, 0], "nothing follows the header");
        assert_eq!(packet[4..8], [0x21, 0x12, 0xA4, 0x42]);
        assert_eq!(packet[8..], id);
    }

    #[test]
    fn an_answer_gives_back_the_id_it_carries() {
        let id = [3; 12];
        let mut answer = request(id).to_vec();
        answer[0..2].copy_from_slice(&BINDING_ANSWER.to_be_bytes());
        // A real answer carries this machine's address after the header.
        answer.extend_from_slice(&[0; 12]);
        assert_eq!(answered(&answer), Some(id));
    }

    #[test]
    fn what_isnt_a_binding_answer_is_no_answer() {
        let id = [5; 12];
        assert_eq!(answered(&request(id)), None, "a request");
        let mut uncookied = request(id);
        uncookied[0..2].copy_from_slice(&BINDING_ANSWER.to_be_bytes());
        uncookied[4] = 0;
        assert_eq!(answered(&uncookied), None, "no cookie");
        assert_eq!(answered(&[0x01, 0x01, 0, 0]), None, "too short");
    }

    #[test]
    fn no_two_requests_in_a_run_share_an_id() {
        let mut ids = Ids::new();
        assert_ne!(ids.mint(), ids.mint());
    }

    #[test]
    fn every_answered_request_is_timed_and_none_lost() {
        let (server, stand_in) = stand_in(None);
        let calls = paced(server, Duration::from_millis(200), EVERY, PATIENCE).expect("asking");
        stand_in.join().expect("the stand-in");
        assert_eq!(calls.lost, 0, "{calls:?}");
        assert!(calls.times.len() >= 5, "{calls:?}");
    }

    #[test]
    fn a_request_never_answered_is_lost() {
        let (server, stand_in) = stand_in(Some(2));
        let calls = paced(server, Duration::from_millis(200), EVERY, PATIENCE).expect("asking");
        stand_in.join().expect("the stand-in");
        assert!(calls.lost >= 1, "{calls:?}");
        assert!(calls.lost.abs_diff(calls.times.len()) <= 1, "{calls:?}");
    }
}
