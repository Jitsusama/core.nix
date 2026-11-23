//! What an interface has sent, in bytes, as the system counts it for any
//! account: on Linux its statistics in sysfs, a 64-bit count, and on a Mac
//! the link entry getifaddrs gives, whose count is 32 bits wide and wraps
//! every 4 GiB, an hour's sending at 10 Mbit/s.

use std::io;

/// How wide the counter `by` reads is, in bits.
#[cfg(target_os = "linux")]
pub const BITS: u32 = 64;
#[cfg(target_os = "macos")]
pub const BITS: u32 = 32;

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

/// The bytes `interface` has sent, by its link entry's count.
#[cfg(target_os = "macos")]
pub fn by(interface: &str) -> io::Result<u64> {
    use std::ffi::CStr;

    let link = u8::try_from(libc::AF_LINK).unwrap_or(u8::MAX);
    let mut first: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `first` with a list it allocated, freed below
    // and read only before.
    if unsafe { libc::getifaddrs(&raw mut first) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut found = None;
    let mut at = first;
    while !at.is_null() {
        // SAFETY: every entry of the list is valid until freeifaddrs.
        let entry = unsafe { &*at };
        // SAFETY: a non-null address, name and data are the entry's own, a
        // link entry's data is its if_data, and the name ends in a nul.
        let ours = !entry.ifa_addr.is_null()
            && !entry.ifa_data.is_null()
            && unsafe { (*entry.ifa_addr).sa_family } == link
            && unsafe { CStr::from_ptr(entry.ifa_name) }.to_bytes() == interface.as_bytes();
        if ours {
            // SAFETY: as above, a link entry's data is its if_data.
            let data = unsafe { &*entry.ifa_data.cast::<libc::if_data>() };
            found = Some(u64::from(data.ifi_obytes));
        }
        at = entry.ifa_next;
    }
    // SAFETY: `first` came from getifaddrs and nothing read from it is kept.
    unsafe { libc::freeifaddrs(first) };
    found.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no such interface"))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn an_interfaces_count_is_read_from_its_statistics() {
        let root = std::env::temp_dir().join(format!("house-report-{}", std::process::id()));
        let statistics = root.join("eth0/statistics");
        std::fs::create_dir_all(&statistics).expect("the stand-in folder couldn't be made");
        std::fs::write(statistics.join("tx_bytes"), "123456789\n")
            .expect("the stand-in count couldn't be written");
        let read = from(&root, "eth0").expect("the stand-in count couldn't be read");
        assert_eq!(read, 123_456_789, "the count read isn't the one written");
        assert!(
            from(&root, "eth1").is_err(),
            "an interface with no statistics read a count"
        );
        assert!(
            from(&root, "../eth0").is_err(),
            "a name with a slash read another's file"
        );
        std::fs::remove_dir_all(&root).expect("the stand-in folder couldn't be removed");
    }
}
