//! `shaper proof-call SECONDS`: a call's packets, as `shaper proof` hears
//! them. DNS round trips five times a second, one line each, in ms or
//! `lost`, which the proof runs in app.slice as Joel's own call would be.
//!
//! A call's delay is the network's, so the probe asks a server whose own
//! time to answer is next to none. Google's STUN server, which this asked
//! first, isn't one: on 10 November it answered in a median of 95 ms where
//! a ping to the same address took 35 and a DNS question to Google's
//! resolver 31, in the same 30 seconds, and the kernel's receive stamps
//! put the difference past this machine. It had answered in 23.5 at 19:05
//! and 97 at 22:44, so the proof's calm judged the server's evening. A
//! resolver answers a name it holds from memory as fast as a ping comes
//! back, the same UDP a call sends.

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// Where the questions go: Google's public resolver, near where a Meet
/// call's packets go.
pub const RESOLVER: &str = "8.8.8.8:53";
/// The name asked, which a busy resolver always holds.
const NAME: &[&str] = &["dns", "google"];
/// A question's spacing, and how long one goes unanswered before it's
/// lost.
const CALL_EVERY: Duration = Duration::from_millis(200);
const CALL_LOST: Duration = Duration::from_secs(2);

/// A DNS question for `NAME`'s address, with this id.
pub fn dns_request(id: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(32);
    packet.extend_from_slice(&id.to_be_bytes());
    // A standard query asking for recursion, with one question.
    packet.extend_from_slice(&[0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    for label in NAME {
        packet.push(u8::try_from(label.len()).unwrap_or(0));
        packet.extend_from_slice(label.as_bytes());
    }
    // The name's end, then type A in class IN.
    packet.extend_from_slice(&[0, 0x00, 0x01, 0x00, 0x01]);
    packet
}

/// The id of the question a DNS answer answers, if it is one.
pub fn dns_answer(packet: &[u8]) -> Option<u16> {
    // A whole header, its answer bit set: whatever the resolver says of
    // the name, it said it as soon as the network let it.
    let head = packet.get(0..12)?;
    (head[2] & 0x80 != 0).then(|| u16::from_be_bytes([head[0], head[1]]))
}

/// The call's round trips for `SECONDS`, printed as they're answered.
pub fn command(args: &[String]) -> ExitCode {
    let Some(seconds) = args.first().and_then(|s| s.parse::<u64>().ok()) else {
        eprintln!("usage: shaper proof-call SECONDS");
        return ExitCode::from(2);
    };
    let Some(server) = RESOLVER
        .to_socket_addrs()
        .ok()
        .and_then(|mut all| all.find(SocketAddr::is_ipv4))
    else {
        eprintln!("couldn't find {RESOLVER}");
        return ExitCode::FAILURE;
    };
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return ExitCode::FAILURE;
    };
    let end = Instant::now() + Duration::from_secs(seconds);
    let mut sent: Vec<(u16, Instant)> = Vec::new();
    let mut next = Instant::now();
    // Each run counts from its own place, so a late answer to an earlier
    // run's question isn't taken for one of this run's.
    let mut id = u16::try_from(std::process::id() % 65_536).unwrap_or(0);
    let mut buffer = [0_u8; 512];
    while Instant::now() < end {
        let now = Instant::now();
        if now >= next {
            id = id.wrapping_add(1);
            if socket.send_to(&dns_request(id), server).is_ok() {
                sent.push((id, now));
            }
            next = now + CALL_EVERY;
        }
        sent.retain(|&(_, at)| {
            let gone = now - at > CALL_LOST;
            if gone {
                println!("lost");
            }
            !gone
        });
        let wait = next
            .saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1));
        let _ = socket.set_read_timeout(Some(wait));
        if let Ok(length) = socket.recv(&mut buffer)
            && let Some(answered) = dns_answer(&buffer[..length])
            && let Some(at) = sent.iter().position(|&(s, _)| s == answered)
        {
            let (_, when) = sent.remove(at);
            println!("{:.1}", when.elapsed().as_secs_f64() * 1_000.0);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a resolver sends back for `question`: the same id and question,
    /// marked as an answer.
    fn answered(question: &[u8]) -> Vec<u8> {
        let mut answer = question.to_vec();
        answer[2] |= 0x80;
        answer
    }

    #[test]
    fn a_dns_answer_is_matched_to_its_question() {
        let question = dns_request(0x1234);
        assert_eq!(dns_answer(&answered(&question)), Some(0x1234));
        assert_eq!(dns_answer(&question), None, "a question read as an answer");
        assert_eq!(
            dns_answer(&answered(&question)[..10]),
            None,
            "a short packet read as an answer"
        );
    }

    #[test]
    fn the_question_asks_for_the_name_s_address() {
        let question = dns_request(7);
        assert_eq!(
            &question[12..24],
            b"\x03dns\x06google\x00",
            "not the name asked"
        );
        assert_eq!(&question[24..], &[0, 1, 0, 1], "not an address in class IN");
    }
}
