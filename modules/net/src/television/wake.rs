//! Waking a TV from standby with Wake-on-LAN: the magic packet, six bytes
//! of 0xff and then its hardware address sixteen times, sent over UDP to
//! port 9. A TV hears it on Wi-Fi only with its Power On with Mobile
//! setting on, which nothing it answers shows, so a wake is judged by
//! whether the TV answers afterwards, never by the send.

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};

/// The port Wake-on-LAN is sent to, by custom; the TV listens on the
/// link, not the port.
const PORT: u16 = 9;

/// The magic packet for hardware address `mac`, as six bytes written in
/// hex, each pair split by `:` or `-`; none for anything else.
pub fn magic(mac: &str) -> Option<[u8; 102]> {
    let bytes: Vec<u8> = mac
        .split([':', '-'])
        .map(|pair| {
            (pair.len() == 2)
                .then(|| u8::from_str_radix(pair, 16).ok())
                .flatten()
        })
        .collect::<Option<_>>()?;
    let address: [u8; 6] = bytes.try_into().ok()?;
    let mut packet = [0xff; 102];
    for copy in packet[6..].chunks_mut(6) {
        copy.copy_from_slice(&address);
    }
    Some(packet)
}

/// Sends the magic packet for `mac` to the whole link, and to `last`, the
/// address the TV was last reached at, since a home network may drop a
/// broadcast its access point forwards to Wi-Fi as it would anything.
pub fn send(mac: &str, last: Option<IpAddr>) -> io::Result<()> {
    let packet = magic(mac).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{mac} isn't a hardware address"),
        )
    })?;
    let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
    socket.set_broadcast(true)?;
    socket.send_to(
        &packet,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), PORT),
    )?;
    if let Some(IpAddr::V4(last)) = last {
        // A unicast send needs the TV's address resolved, which a TV in
        // standby may no longer answer; the broadcast above is the one
        // that counts, so this one failing isn't.
        let _ = socket.send_to(&packet, SocketAddr::new(IpAddr::V4(last), PORT));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packet_is_six_ffs_then_the_address_sixteen_times() {
        let packet = magic("02:00:00:00:00:90").expect("an address");
        assert_eq!(packet[..6], [0xff; 6]);
        for copy in packet[6..].chunks(6) {
            assert_eq!(copy, [0x02, 0, 0, 0, 0, 0x90]);
        }
    }

    #[test]
    fn an_address_in_capitals_or_with_dashes_is_the_same() {
        assert_eq!(magic("0A-1B-2C-3D-4E-5F"), magic("0a:1b:2c:3d:4e:5f"));
    }

    #[test]
    fn anything_but_six_hex_pairs_is_no_address() {
        for word in [
            "",
            "02:00:00:00:00",
            "02:00:00:00:00:90:01",
            "2:00:00:00:00:90",
            "zz:00:00:00:00:90",
        ] {
            assert_eq!(magic(word), None, "{word}");
        }
    }
}
