//! What an interface has sent, in bytes, as the system counts it for any
//! account, 64 bits wide on both: on Linux its statistics in sysfs, and on a
//! Mac the routing socket's interface list.

use std::io;

/// The bytes `interface` has sent, by its counter.
#[cfg(target_os = "linux")]
pub fn by(interface: &str) -> io::Result<u64> {
    from(std::path::Path::new("/sys/class/net"), interface)
}

/// The bytes `interface` has sent, by its statistics under `root`, the
/// kernel's net class folder, or a stand-in's in a test.
#[cfg(target_os = "linux")]
fn from(root: &std::path::Path, interface: &str) -> io::Result<u64> {
    // A name with a slash would read another folder's file.
    if interface.contains('/') || interface.starts_with('.') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no such interface name",
        ));
    }
    let text = std::fs::read_to_string(root.join(interface).join("statistics/tx_bytes"))?;
    text.trim()
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

/// The bytes `interface` has sent, by the 64-bit count the routing
/// socket's interface list gives (NET_RT_IFLIST2), rather than the link
/// entry getifaddrs gives, whose count is 32 bits wide and wraps every
/// 4 GiB, so a count that went back can't be told a wrap from a reset.
#[cfg(target_os = "macos")]
pub fn by(interface: &str) -> io::Result<u64> {
    let name = std::ffi::CString::new(interface)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "no such interface name"))?;
    // SAFETY: `name` is a valid string ending in a nul, read only here.
    let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
    if index == 0 {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no such interface"));
    }
    sent_in(&interfaces()?, index)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such interface"))
}

/// The bytes the interface numbered `index` has sent, as the routing
/// socket's interface list `list` counts them, if the list holds it whole.
#[cfg(target_os = "macos")]
fn sent_in(list: &[u8], index: u32) -> Option<u64> {
    let mut at = 0;
    // Each message starts with its length, two bytes, and its type, the
    // fourth byte; an interface's own is a whole if_msghdr2.
    while let Some(head) = list.get(at..at + 4) {
        let length = usize::from(u16::from_ne_bytes([head[0], head[1]]));
        if length == 0 {
            break;
        }
        let whole = std::mem::size_of::<libc::if_msghdr2>();
        if i32::from(head[3]) == libc::RTM_IFINFO2 && length >= whole && at + whole <= list.len() {
            // SAFETY: the bytes from `at` hold a whole if_msghdr2, as its
            // type and length say, read unaligned since the list is bytes.
            let message: libc::if_msghdr2 =
                unsafe { std::ptr::read_unaligned(list[at..].as_ptr().cast()) };
            if u32::from(message.ifm_index) == index {
                return Some(message.ifm_data.ifi_obytes);
            }
        }
        at += length;
    }
    None
}

/// The routing socket's list of interfaces, each with its 64-bit counts.
#[cfg(target_os = "macos")]
fn interfaces() -> io::Result<Vec<u8>> {
    let mut asked = [libc::CTL_NET, libc::PF_ROUTE, 0, 0, libc::NET_RT_IFLIST2, 0];
    let depth = libc::c_uint::try_from(asked.len()).unwrap_or(libc::c_uint::MAX);
    let mut length: libc::size_t = 0;
    // SAFETY: with no buffer, sysctl writes only the length it needs.
    let sized = unsafe {
        libc::sysctl(
            asked.as_mut_ptr(),
            depth,
            std::ptr::null_mut(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if sized != 0 {
        return Err(io::Error::last_os_error());
    }
    // Room for an interface that comes between the two calls.
    length += length / 8;
    let mut list = vec![0_u8; length];
    // SAFETY: `list` holds `length` bytes, which sysctl writes within and
    // then sets to what it wrote.
    let read = unsafe {
        libc::sysctl(
            asked.as_mut_ptr(),
            depth,
            list.as_mut_ptr().cast(),
            &raw mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    if read != 0 {
        return Err(io::Error::last_os_error());
    }
    list.truncate(length);
    Ok(list)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    /// An interface list's message for the interface numbered `index`,
    /// whole, counting `sent` bytes sent and `heard` heard.
    fn message(index: u16, sent: u64, heard: u64) -> Vec<u8> {
        // SAFETY: if_msghdr2 is numbers alone, so all zeroes is one.
        let mut message: libc::if_msghdr2 = unsafe { std::mem::zeroed() };
        let whole = std::mem::size_of::<libc::if_msghdr2>();
        message.ifm_msglen = u16::try_from(whole).expect("a message longer than its length");
        message.ifm_type = u8::try_from(libc::RTM_IFINFO2).expect("a type past a byte");
        message.ifm_index = index;
        message.ifm_data.ifi_obytes = sent;
        message.ifm_data.ifi_ibytes = heard;
        // SAFETY: `message` is `whole` bytes long and lives to the copy.
        unsafe { std::slice::from_raw_parts((&raw const message).cast::<u8>(), whole) }.to_vec()
    }

    // The list's walk, over a list made here: a message of another type
    // and length passed by, the sent count read rather than the heard one,
    // and all 64 bits of it, and a message cut short or of no length read
    // as nothing rather than past the list's end.
    #[test]
    fn the_list_gives_the_sent_count_of_the_interface_asked_whole() {
        let past_32_bits = 5_000_000_000;
        let mut other = vec![0_u8; 20];
        other[0] = 20;
        other[3] = u8::try_from(libc::RTM_NEWADDR).expect("a type past a byte");
        let list = [other, message(3, past_32_bits, 7), message(4, 9, 9)].concat();
        assert_eq!(
            sent_in(&list, 3),
            Some(past_32_bits),
            "the sent count wasn't read whole, or the heard one was read for it"
        );
        assert_eq!(
            sent_in(&list, 5),
            None,
            "an interface not listed read a count"
        );
        let cut = &list[..list.len() - 1];
        assert_eq!(
            sent_in(cut, 4),
            None,
            "a message cut short was read past the list's end"
        );
        let none_long = [vec![0_u8; 4], message(3, 1, 1)].concat();
        assert_eq!(
            sent_in(&none_long, 3),
            None,
            "the walk went on past a message of no length"
        );
    }

    // What the loopback carries for a datagram sent to this machine is
    // counted as sent, so the routing socket's list is read for real
    // (sysctl's NET_RT_IFLIST2), not a stand-in.
    #[test]
    fn the_loopbacks_count_grows_by_what_it_sends() {
        let before = by("lo0").expect("the loopback's count couldn't be read");
        let socket = std::net::UdpSocket::bind("127.0.0.1:0").expect("no socket to send from");
        let to = socket.local_addr().expect("the socket has no address");
        for _ in 0..10 {
            socket
                .send_to(&[0; 1_000], to)
                .expect("a datagram couldn't be sent");
        }
        let after = by("lo0").expect("the loopback's count couldn't be read again");
        assert!(
            after.saturating_sub(before) >= 10_000,
            "ten datagrams of 1,000 bytes counted {} bytes",
            after.saturating_sub(before)
        );
        assert_eq!(
            by("none0").map_err(|e| e.kind()),
            Err(io::ErrorKind::NotFound),
            "an interface that isn't there read a count"
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn an_interfaces_count_is_read_from_its_statistics() {
        let top = std::env::temp_dir().join(format!("house-report-{}", std::process::id()));
        let root = top.join("net");
        // A count in the stand-in class folder, one beside it that a name
        // with a slash would reach, and a hidden one in it.
        for (folder, count) in [
            ("net/eth0", "123456789\n"),
            ("eth0", "1\n"),
            ("net/.eth0", "1\n"),
        ] {
            let statistics = top.join(folder).join("statistics");
            std::fs::create_dir_all(&statistics).expect("the stand-in folder couldn't be made");
            std::fs::write(statistics.join("tx_bytes"), count)
                .expect("the stand-in count couldn't be written");
        }
        let read = from(&root, "eth0").expect("the stand-in count couldn't be read");
        assert_eq!(read, 123_456_789, "the count read isn't the one written");
        assert!(
            from(&root, "eth1").is_err(),
            "an interface with no statistics read a count"
        );
        assert!(
            from(&root, "../eth0").is_err(),
            "a name with a slash read a count outside the class folder"
        );
        assert!(from(&root, ".eth0").is_err(), "a hidden name read a count");
        std::fs::remove_dir_all(&top).expect("the stand-in folder couldn't be removed");
    }
}
