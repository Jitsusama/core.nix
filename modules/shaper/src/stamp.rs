//! ICMP timestamps (RFC 792, types 13 and 14), which give each direction's
//! delay apart where a ping gives only their sum. A reflector stamps when
//! a request arrived and when its reply left, in milliseconds since
//! midnight UT, so the way out is its arrival less our sending, and the way
//! back our arrival less its sending. The two clocks disagree by some
//! offset, which adds to one direction and comes off the other; a baseline
//! per reflector takes it out, since only a rise above it is a queue.

use std::time::{SystemTime, UNIX_EPOCH};

/// A day in milliseconds, where the stamps wrap.
pub const DAY_MS: u32 = 86_400_000;

/// A stamp's high bit says its clock isn't milliseconds since midnight UT,
/// so its differences mean nothing.
const NOT_STANDARD: u32 = 1 << 31;

/// A timestamp request from this prober, `id`, numbered `seq`, sent at
/// `originate`.
pub fn request(id: u16, seq: u16, originate: u32) -> [u8; 20] {
    let mut message = [0; 20];
    message[0] = TIMESTAMP;
    message[4..6].copy_from_slice(&id.to_be_bytes());
    message[6..8].copy_from_slice(&seq.to_be_bytes());
    message[8..12].copy_from_slice(&originate.to_be_bytes());
    let sum = checksum(&message);
    message[2..4].copy_from_slice(&sum.to_be_bytes());
    message
}

const TIMESTAMP: u8 = 13;
const TIMESTAMP_REPLY: u8 = 14;

/// The internet checksum of `bytes`: the ones' complement of their ones'
/// complement sum, taken 16 bits at a time.
pub fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes
        .chunks(2)
        .map(|pair| u32::from(u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)])))
        .sum();
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !u16::try_from(sum).unwrap_or(u16::MAX)
}

/// What a reflector's reply says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reply {
    pub seq: u16,
    pub originate: u32,
    pub receive: u32,
    pub transmit: u32,
}

/// The reply to this prober, `id`, in a packet a raw socket read, its IPv4
/// header first; None for anything else a raw ICMP socket hears.
pub fn reply(packet: &[u8], id: u16) -> Option<Reply> {
    let header = usize::from(packet.first()? & 0x0f) * 4;
    let message = packet.get(header..header + 20)?;
    let word = |at: usize| {
        u32::from_be_bytes([
            message[at],
            message[at + 1],
            message[at + 2],
            message[at + 3],
        ])
    };
    let ours = message[0] == TIMESTAMP_REPLY
        && u16::from_be_bytes([message[4], message[5]]) == id
        && checksum(message) == 0;
    ours.then(|| Reply {
        seq: u16::from_be_bytes([message[6], message[7]]),
        originate: word(8),
        receive: word(12),
        transmit: word(16),
    })
}

/// Each direction's delay, in milliseconds, clock offset included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OneWay {
    pub out_ms: f64,
    pub back_ms: f64,
}

/// The delays a reply gives, read when it arrived at `finished`; None when
/// the reflector's clock isn't the standard one.
pub fn one_way(reply: &Reply, finished: u32) -> Option<OneWay> {
    if (reply.receive | reply.transmit) & NOT_STANDARD != 0 {
        return None;
    }
    Some(OneWay {
        out_ms: apart(reply.receive, reply.originate),
        back_ms: apart(finished, reply.transmit),
    })
}

/// How far `later` is past `earlier` on a clock that wraps each midnight,
/// taken the short way round, so a stamp just past midnight reads as just
/// after one just before it.
fn apart(later: u32, earlier: u32) -> f64 {
    let day = i64::from(DAY_MS);
    let difference = (i64::from(later) - i64::from(earlier)).rem_euclid(day);
    let shortest = if difference > day / 2 {
        difference - day
    } else {
        difference
    };
    // Within half a day, so exact in an f64.
    #[expect(
        clippy::cast_precision_loss,
        reason = "at most half a day of milliseconds"
    )]
    let ms = shortest as f64;
    ms
}

/// Milliseconds since midnight UT at `now`.
pub fn since_midnight(now: SystemTime) -> u32 {
    let since = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let ms = since.as_millis() % u128::from(DAY_MS);
    u32::try_from(ms).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// A reply as a raw socket reads it: a 20-byte IPv4 header, then the
    /// ICMP message.
    fn packet(kind: u8, id: u16, seq: u16, stamps: [u32; 3]) -> Vec<u8> {
        let mut ip = vec![
            0x45, 0, 0, 40, 0, 0, 0, 0, 64, 1, 0, 0, 9, 9, 9, 9, 192, 168, 4, 20,
        ];
        let mut icmp = vec![kind, 0, 0, 0];
        icmp.extend(id.to_be_bytes());
        icmp.extend(seq.to_be_bytes());
        for stamp in stamps {
            icmp.extend(stamp.to_be_bytes());
        }
        let sum = checksum(&icmp).to_be_bytes();
        icmp[2] = sum[0];
        icmp[3] = sum[1];
        ip.extend(icmp);
        ip
    }

    #[test]
    fn a_request_is_a_timestamp_with_a_checksum_that_holds() {
        let sent = request(0x1234, 7, 1_000);
        assert_eq!(sent[0], 13, "type");
        assert_eq!(sent[1], 0, "code");
        assert_eq!(&sent[4..8], &[0x12, 0x34, 0, 7]);
        assert_eq!(&sent[8..12], &1_000_u32.to_be_bytes());
        assert_eq!(
            &sent[12..20],
            &[0; 8],
            "the reflector's stamps are left for it"
        );
        assert_eq!(
            checksum(&sent),
            0,
            "a checksum summed with its message gives zero"
        );
    }

    #[test]
    fn a_reply_gives_the_reflectors_stamps() {
        let heard = reply(&packet(14, 0x1234, 7, [1_000, 1_023, 1_024]), 0x1234);
        assert_eq!(
            heard,
            Some(Reply {
                seq: 7,
                originate: 1_000,
                receive: 1_023,
                transmit: 1_024,
            })
        );
    }

    #[test]
    fn what_a_raw_socket_hears_for_others_isnt_a_reply() {
        let others = packet(14, 0x4321, 7, [1_000, 1_023, 1_024]);
        assert_eq!(reply(&others, 0x1234), None, "another prober's reply");
        let echo = packet(0, 0x1234, 7, [1_000, 1_023, 1_024]);
        assert_eq!(reply(&echo, 0x1234), None, "a ping's reply");
        let mut broken = packet(14, 0x1234, 7, [1_000, 1_023, 1_024]);
        broken[30] ^= 1;
        assert_eq!(reply(&broken, 0x1234), None, "a checksum that fails");
        assert_eq!(reply(&broken[..30], 0x1234), None, "a short packet");
    }

    #[test]
    fn each_direction_is_its_own_delay() {
        let heard = Reply {
            seq: 1,
            originate: 1_000,
            receive: 1_023,
            transmit: 1_024,
        };
        assert_eq!(
            one_way(&heard, 1_065),
            Some(OneWay {
                out_ms: 23.0,
                back_ms: 41.0,
            })
        );
    }

    #[test]
    fn a_clock_behind_ours_reads_as_a_way_out_below_zero() {
        let heard = Reply {
            seq: 1,
            originate: 1_000,
            receive: 990,
            transmit: 991,
        };
        assert_eq!(
            one_way(&heard, 1_065),
            Some(OneWay {
                out_ms: -10.0,
                back_ms: 74.0,
            })
        );
    }

    #[test]
    fn stamps_across_midnight_still_read() {
        let heard = Reply {
            seq: 1,
            originate: DAY_MS - 5,
            receive: 18,
            transmit: 19,
        };
        assert_eq!(
            one_way(&heard, 60),
            Some(OneWay {
                out_ms: 23.0,
                back_ms: 41.0,
            })
        );
    }

    #[test]
    fn a_clock_that_isnt_the_standard_one_gives_nothing() {
        let heard = Reply {
            seq: 1,
            originate: 1_000,
            receive: 1_023 | NOT_STANDARD,
            transmit: 1_024 | NOT_STANDARD,
        };
        assert_eq!(one_way(&heard, 1_065), None);
    }

    #[test]
    fn midnight_is_where_the_stamps_start() {
        let at = UNIX_EPOCH + Duration::from_millis(3 * u64::from(DAY_MS) + 1_234);
        assert_eq!(since_midnight(at), 1_234);
    }
}
