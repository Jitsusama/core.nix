//! `shaper proof-call SECONDS`: a call's packets, as `shaper proof` hears
//! them. STUN round trips five times a second, one line each, in ms or
//! `lost`, which the proof runs in app.slice as Joel's own call would be.

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::process::ExitCode;
use std::time::{Duration, Instant};

pub const STUN: &str = "stun.l.google.com:19302";
/// A STUN probe's spacing, and how long one goes unanswered before it's
/// lost.
const CALL_EVERY: Duration = Duration::from_millis(200);
const CALL_LOST: Duration = Duration::from_secs(2);

/// A STUN binding request with this transaction id.
pub fn stun_request(id: [u8; 12]) -> [u8; 20] {
    let mut packet = [0_u8; 20];
    packet[0..2].copy_from_slice(&0x0001_u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0x2112_A442_u32.to_be_bytes());
    packet[8..20].copy_from_slice(&id);
    packet
}

/// The transaction id a STUN binding success answers, if it is one.
pub fn stun_answer(packet: &[u8]) -> Option<[u8; 12]> {
    let head = packet.get(0..20)?;
    (head[0..2] == 0x0101_u16.to_be_bytes() && head[4..8] == 0x2112_A442_u32.to_be_bytes())
        .then(|| head[8..20].try_into().ok())
        .flatten()
}

/// The call's round trips for `SECONDS`, printed as they're answered.
pub fn command(args: &[String]) -> ExitCode {
    let Some(seconds) = args.first().and_then(|s| s.parse::<u64>().ok()) else {
        eprintln!("usage: shaper proof-call SECONDS");
        return ExitCode::from(2);
    };
    let Some(server) = STUN
        .to_socket_addrs()
        .ok()
        .and_then(|mut all| all.find(SocketAddr::is_ipv4))
    else {
        eprintln!("couldn't find {STUN}");
        return ExitCode::FAILURE;
    };
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return ExitCode::FAILURE;
    };
    let end = Instant::now() + Duration::from_secs(seconds);
    let mut sent: Vec<([u8; 12], Instant)> = Vec::new();
    let mut next = Instant::now();
    let mut seq = 0_u64;
    let mut buffer = [0_u8; 512];
    while Instant::now() < end {
        let now = Instant::now();
        if now >= next {
            seq += 1;
            let mut id = [0_u8; 12];
            id[4..].copy_from_slice(&seq.to_be_bytes());
            if socket.send_to(&stun_request(id), server).is_ok() {
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
            && let Some(id) = stun_answer(&buffer[..length])
            && let Some(at) = sent.iter().position(|&(s, _)| s == id)
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

    #[test]
    fn a_stun_answer_is_matched_to_its_request() {
        let id = [7_u8; 12];
        let mut answer = stun_request(id);
        answer[0..2].copy_from_slice(&0x0101_u16.to_be_bytes());
        assert_eq!(stun_answer(&answer), Some(id));
        assert_eq!(
            stun_answer(&stun_request(id)),
            None,
            "a request read as an answer"
        );
        assert_eq!(
            stun_answer(&answer[..10]),
            None,
            "a short packet read as an answer"
        );
    }
}
