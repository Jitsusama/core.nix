//! Asks a network's own DNS server for a name's addresses.
//!
//! The check has to learn what the network says the check page's address
//! is, and resolved can't tell it: a VPN such as WARP points the link at
//! its own servers while it's connected, and a network that wants a
//! sign-in answers for the check page differently from the rest of the
//! world. So wifid asks the server the network handed out, itself.

use std::hash::{BuildHasher, RandomState};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::timeout;

/// How long a server has to answer before it's taken not to.
const PATIENCE: Duration = Duration::from_secs(3);

/// How long a query waits for its answer before it's sent again, since a
/// query or answer lost over the air is never answered, and DNS over UDP
/// leaves sending again to whoever asks.
const AGAIN: Duration = Duration::from_millis(700);

/// The name's addresses, asked of server through card alone: IPv4 ones
/// where there are any, else IPv6 ones.
pub async fn addresses(card: &str, server: IpAddr, name: &str) -> io::Result<Vec<IpAddr>> {
    let unbound: IpAddr = match server {
        IpAddr::V4(_) => Ipv4Addr::UNSPECIFIED.into(),
        IpAddr::V6(_) => Ipv6Addr::UNSPECIFIED.into(),
    };
    let socket = UdpSocket::bind(SocketAddr::new(unbound, 0)).await?;
    socket.bind_device(Some(card.as_bytes()))?;
    socket.connect(SocketAddr::new(server, 53)).await?;
    for kind in [Kind::V4, Kind::V6] {
        // A number nobody off the network can guess, so nobody can answer
        // in its place.
        let id = low_u16(RandomState::new().hash_one(name));
        let answered = asked(&socket, &query(id, name, kind), id, AGAIN, PATIENCE).await?;
        let addresses = answered.ok_or_else(|| {
            io::Error::new(io::ErrorKind::TimedOut, format!("{server} didn't answer"))
        })?;
        if !addresses.is_empty() {
            return Ok(addresses);
        }
    }
    Ok(Vec::new())
}

/// The lowest two bytes of a number, the rest dropped: all the room a
/// query's number has.
fn low_u16(number: u64) -> u16 {
    let [low, high, ..] = number.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// The lowest byte of a length, the rest dropped. DNS keeps a label to 63
/// bytes, so nothing is dropped from one it allows.
fn low_byte(length: usize) -> u8 {
    length.to_le_bytes()[0]
}

/// Sends the query and waits for its answer, sending it again each time
/// `again` passes without one, until `patience` is up.
async fn asked(
    socket: &UdpSocket,
    query: &[u8],
    id: u16,
    again: Duration,
    patience: Duration,
) -> io::Result<Option<Vec<IpAddr>>> {
    let mut reply = [0; 1500];
    let answered = async {
        let mut resend = tokio::time::interval(again);
        loop {
            tokio::select! {
                _ = resend.tick() => {
                    socket.send(query).await?;
                }
                length = socket.recv(&mut reply) => {
                    if let Some(addresses) = answers(&reply[..length?], id) {
                        return io::Result::Ok(addresses);
                    }
                }
            }
        }
    };
    match timeout(patience, answered).await {
        Ok(addresses) => addresses.map(Some),
        Err(_) => Ok(None),
    }
}

/// The kinds of address a query can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    V4,
    V6,
}

/// A query for a name's addresses of one kind, numbered so its answer can
/// be told from any other.
pub fn query(id: u16, name: &str, kind: Kind) -> Vec<u8> {
    let mut query = Vec::with_capacity(18 + name.len());
    query.extend(id.to_be_bytes());
    // A standard query, asking the server to follow it through for us,
    // with one question.
    query.extend([0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in name.split('.').filter(|label| !label.is_empty()) {
        query.push(low_byte(label.len()));
        query.extend(label.as_bytes());
    }
    query.push(0);
    query.extend(kind.code().to_be_bytes());
    query.extend(INTERNET.to_be_bytes());
    query
}

impl Kind {
    fn code(self) -> u16 {
        match self {
            Kind::V4 => 1,
            Kind::V6 => 28,
        }
    }
}

/// The class every query here is in.
const INTERNET: u16 = 1;

/// The addresses in the answer to the query numbered id, through any
/// aliases it names on the way; none when it answers another query or
/// refuses this one.
pub fn answers(reply: &[u8], id: u16) -> Option<Vec<IpAddr>> {
    let mut at = Reader { reply, at: 0 };
    let answered = at.u16()? == id;
    let flags = at.u16()?;
    let is_answer = flags & 0x8000 != 0;
    // The lowest four bits are the answer's code, none of them set when
    // it succeeded.
    let succeeded = flags.trailing_zeros() >= 4;
    if !(answered && is_answer && succeeded) {
        return None;
    }
    let questions = at.u16()?;
    let records = at.u16()?;
    at.skip(4)?;
    for _ in 0..questions {
        at.name()?;
        at.skip(4)?;
    }
    let mut addresses = Vec::new();
    for _ in 0..records {
        at.name()?;
        let kind = at.u16()?;
        let class = at.u16()?;
        at.skip(4)?;
        let length = usize::from(at.u16()?);
        let data = at.take(length)?;
        if class != INTERNET {
            continue;
        }
        // Aliases need no following: a server that follows a query through
        // puts the addresses of what they name in the same answer.
        if let (1, Ok(octets)) = (kind, <[u8; 4]>::try_from(data)) {
            addresses.push(IpAddr::from(octets));
        }
        if let (28, Ok(octets)) = (kind, <[u8; 16]>::try_from(data)) {
            addresses.push(IpAddr::from(octets));
        }
    }
    Some(addresses)
}

/// Reads a reply in order, failing at its end rather than past it.
struct Reader<'a> {
    reply: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let taken = self.reply.get(self.at..self.at.checked_add(length)?)?;
        self.at += length;
        Some(taken)
    }

    fn skip(&mut self, length: usize) -> Option<()> {
        self.take(length).map(|_| ())
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes = self.take(2)?;
        Some(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    /// Passes over a name, which ends at an empty label or at a pointer to
    /// the rest of it written earlier.
    fn name(&mut self) -> Option<()> {
        loop {
            let length = self.take(1)?[0];
            match length {
                0 => return Some(()),
                pointer if pointer & 0xc0 == 0xc0 => return self.skip(1),
                label => self.skip(usize::from(label))?,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A server on loopback that ignores the first query it's sent and
    /// answers every one after, as a network losing a packet does.
    async fn forgetful() -> (UdpSocket, UdpSocket) {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.connect(server.local_addr().unwrap()).await.unwrap();
        (server, client)
    }

    #[tokio::test]
    async fn a_query_lost_on_the_way_is_sent_again() {
        let (server, client) = forgetful().await;
        let answering = tokio::spawn(async move {
            let mut asked = [0; 512];
            let (_, _) = server.recv_from(&mut asked).await.unwrap();
            let (_, from) = server.recv_from(&mut asked).await.unwrap();
            server.send_to(&answer([0x81, 0x80]), from).await.unwrap();
        });
        let query = query(0x1234, "check.example", Kind::V4);

        let found = asked(
            &client,
            &query,
            0x1234,
            Duration::from_millis(50),
            Duration::from_secs(2),
        );
        let found = found.await.unwrap();
        answering.abort();

        assert_eq!(found.map(|found| found.len()), Some(2));
    }

    #[tokio::test]
    async fn a_server_that_never_answers_gives_nothing_once_patience_is_up() {
        let (_server, client) = forgetful().await;
        let query = query(0x1234, "check.example", Kind::V4);

        let found = asked(
            &client,
            &query,
            0x1234,
            Duration::from_millis(20),
            Duration::from_millis(100),
        );

        assert_eq!(found.await.unwrap(), None);
    }

    #[test]
    fn a_query_asks_for_the_name_s_addresses_of_one_kind() {
        let asked = query(0x1234, "connectivitycheck.gstatic.com", Kind::V4);

        let mut expected = vec![0x12, 0x34, 0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0];
        expected.extend(b"\x11connectivitycheck\x07gstatic\x03com\x00");
        expected.extend([0, 1, 0, 1]);
        assert_eq!(asked, expected);
        assert_eq!(query(1, "inn", Kind::V6)[17..19], [0, 28]);
    }

    /// An answer to query 0x1234 for check.example: an alias to
    /// front-desk.inn, then two addresses for that, with each name after
    /// the question pointing back at one written earlier.
    fn answer(flags: [u8; 2]) -> Vec<u8> {
        let mut reply = vec![0x12, 0x34, flags[0], flags[1], 0, 1, 0, 3, 0, 0, 0, 0];
        reply.extend(b"\x05check\x07example\x00\x00\x01\x00\x01");
        reply.extend([0xc0, 12, 0, 5, 0, 1, 0, 0, 0, 60, 0, 16]);
        reply.extend(b"\x0afront-desk\x03inn\x00");
        reply.extend([0xc0, 43, 0, 1, 0, 1, 0, 0, 0, 60, 0, 4, 10, 43, 0, 1]);
        reply.extend([0xc0, 43, 0, 28, 0, 1, 0, 0, 0, 60, 0, 16]);
        reply.extend([0xfd, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        reply
    }

    #[test]
    fn the_addresses_in_an_answer_are_read_through_its_aliases() {
        let reply = answer([0x81, 0x80]);

        let addresses = answers(&reply, 0x1234);

        let expected: Vec<IpAddr> = vec!["10.43.0.1".parse().unwrap(), "fd00::1".parse().unwrap()];
        assert_eq!(addresses, Some(expected));
    }

    #[test]
    fn an_answer_to_another_query_a_refusal_or_half_an_answer_gives_none() {
        let refused = answer([0x81, 0x85]);
        let truncated = answer([0x81, 0x80])[..40].to_vec();

        assert_eq!(answers(&answer([0x81, 0x80]), 0x4321), None);
        assert_eq!(answers(&refused, 0x1234), None);
        assert_eq!(answers(&truncated, 0x1234), None);
        assert_eq!(answers(&[], 0x1234), None);
    }
}
