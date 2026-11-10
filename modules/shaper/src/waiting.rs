//! Whether anything on this machine has data waiting to go out over the
//! connection, from the kernel's table of TCP connections.
//!
//! The one-way controller (src/oneway.rs) needs it to tell a link that
//! carries little because little is sent from one held back past our CAKE,
//! by a router that queues flow by flow and so delays no probe. A
//! connection to a distant address with at least `WAITING_BYTES` not yet
//! acknowledged, in flight or still to send, wants more than the link
//! gives it. Connections to this network's own machines, and to
//! Tailscale's, never reach CAKE, so they count for nothing.
//!
//! What TCP sends again is read here too, since a router held past what
//! it carries drops the rest, and what it dropped crosses the link twice.

use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

/// What a connection must have unacknowledged to be waiting: more than a
/// window or two of a single flow on a slow upload, so a reply trickling
/// out isn't demand.
const WAITING_BYTES: u64 = 64 * 1024;

/// The state the kernel's table gives an established connection.
const ESTABLISHED: &str = "01";

/// Whether any connection in `/proc/net/tcp` or `tcp6` under `proc` has
/// data waiting to go to a distant address.
pub fn read(proc: &Path) -> bool {
    ["net/tcp", "net/tcp6"]
        .iter()
        .any(|table| fs::read_to_string(proc.join(table)).is_ok_and(|text| waiting(&text)))
}

/// Whether a table in the kernel's text holds a connection waiting.
pub fn waiting(table: &str) -> bool {
    table.lines().skip(1).any(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (Some(remote), Some(&state), Some(queues)) =
            (fields.get(2), fields.get(3), fields.get(4))
        else {
            return false;
        };
        let unacknowledged = queues
            .split_once(':')
            .and_then(|(tx, _)| u64::from_str_radix(tx, 16).ok())
            .unwrap_or(0);
        state == ESTABLISHED
            && unacknowledged >= WAITING_BYTES
            && address(remote).is_some_and(distant)
    })
}

/// An address as the kernel's table writes it: the address's 32-bit words
/// in hex, each in the machine's byte order, then a colon and the port.
fn address(field: &str) -> Option<IpAddr> {
    let (hex, _) = field.split_once(':')?;
    let word = |i: usize| -> Option<[u8; 4]> {
        let word = u32::from_str_radix(hex.get(i * 8..i * 8 + 8)?, 16).ok()?;
        Some(word.to_le_bytes())
    };
    match hex.len() {
        8 => Some(IpAddr::V4(Ipv4Addr::from(word(0)?))),
        32 => {
            let mut bytes = [0_u8; 16];
            for i in 0..4 {
                bytes[i * 4..i * 4 + 4].copy_from_slice(&word(i)?);
            }
            let v6 = Ipv6Addr::from(bytes);
            Some(v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4))
        }
        _ => None,
    }
}

/// The segments TCP has sent again since the machine started, from
/// `/proc/net/snmp` under `proc`.
pub fn resent(proc: &Path) -> Option<u64> {
    fs::read_to_string(proc.join("net/snmp"))
        .ok()
        .and_then(|text| resent_in(&text))
}

/// The `RetransSegs` count in the kernel's SNMP text, whose TCP lines are
/// a line of names and then a line of values.
fn resent_in(snmp: &str) -> Option<u64> {
    let mut tcp = snmp.lines().filter(|line| line.starts_with("Tcp:"));
    let (names, values) = (tcp.next()?, tcp.next()?);
    let at = names
        .split_whitespace()
        .position(|name| name == "RetransSegs")?;
    values.split_whitespace().nth(at)?.parse().ok()
}

/// Whether an address is past the network this machine is on.
fn distant(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            // 100.64.0.0/10 is Tailscale's, which reaches its peers itself.
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || a == 100 && (64..128).contains(&b))
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || first & 0xfe00 == 0xfc00
                || first & 0xffc0 == 0xfe80)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when \
                          retrnsmt   uid  timeout inode";

    /// A connection's line, to an address in the table's own hex.
    fn line(remote: &str, state: &str, tx: u64) -> String {
        format!(
            "   0: 0A06A8C0:D2F4 {remote}:01BB {state} {tx:08X}:00000000 01:00000014 00000000  \
             1000        0 1 1 0000000000000000 20 4 30 10 -1"
        )
    }

    #[test]
    fn an_upload_with_data_waiting_to_a_distant_address_is_waiting() {
        // 9.9.9.9, a public resolver, as the table writes it.
        let table = format!("{HEADER}\n{}", line("09090909", "01", 150_000));
        assert!(waiting(&table), "150 KB unacknowledged wasn't waiting");
    }

    #[test]
    fn a_little_unacknowledged_is_a_reply_trickling_out() {
        let table = format!("{HEADER}\n{}", line("09090909", "01", 20_000));
        assert!(!waiting(&table), "20 KB unacknowledged counted as waiting");
    }

    #[test]
    fn the_networks_own_machines_and_tailscales_dont_count() {
        // 192.168.4.1, 127.0.0.1 and 100.101.102.103.
        let table = format!(
            "{HEADER}\n{}\n{}\n{}",
            line("0104A8C0", "01", 900_000),
            line("0100007F", "01", 900_000),
            line("67666564", "01", 900_000)
        );
        assert!(!waiting(&table), "a nearby address counted as waiting");
    }

    #[test]
    fn a_listening_socket_isnt_a_connection() {
        let table = format!("{HEADER}\n{}", line("09090909", "0A", 900_000));
        assert!(!waiting(&table), "a listening socket counted as waiting");
    }

    #[test]
    fn the_segments_sent_again_are_read_from_tcps_counters() {
        let snmp = "Udp: InDatagrams NoPorts\nUdp: 4 5\n\
                    Tcp: RtoAlgorithm OutSegs RetransSegs InErrs\n\
                    Tcp: 1 131644252 4352186 1219\n";
        assert_eq!(resent_in(snmp), Some(4_352_186));
        assert_eq!(resent_in("Tcp: RtoAlgorithm\n"), None);
    }

    #[test]
    fn ipv6_addresses_are_read_word_by_word() {
        // 2606:4700::1111, a public resolver, and fe80::1 on the link.
        assert_eq!(
            address("00470626000000000000000011110000:01BB"),
            Some("2606:4700::1111".parse().expect("an address"))
        );
        assert_eq!(
            address("000080FE000000000000000001000000:01BB"),
            Some("fe80::1".parse().expect("an address"))
        );
        assert!(
            !distant("fe80::1".parse().expect("an address")),
            "a link-local address counted as distant"
        );
        assert!(
            distant("2606:4700::1111".parse().expect("an address")),
            "a public address counted as nearby"
        );
    }

    #[test]
    fn an_ipv4_address_mapped_into_ipv6_is_read_as_ipv4() {
        // ::ffff:192.168.4.1.
        assert_eq!(
            address("0000000000000000FFFF00000104A8C0:01BB"),
            Some("192.168.4.1".parse().expect("an address"))
        );
    }
}
