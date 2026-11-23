//! Which interface this machine sends to an address by, as its routes say
//! now: the one that holds the address a socket aimed there sends from. A
//! Mac on its Wi-Fi that's then given a cable sends by the cable, and the
//! reporter follows it rather than counting the Wi-Fi that went quiet.

use std::ffi::CStr;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

/// The interface this machine's routes send to `to` by now.
pub fn toward(to: SocketAddr) -> io::Result<String> {
    let any: SocketAddr = if to.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (Ipv6Addr::UNSPECIFIED, 0).into()
    };
    // Aiming a UDP socket sends nothing: it only asks the routes which
    // address it would send from.
    let socket = UdpSocket::bind(any)?;
    socket.connect(to)?;
    holding(socket.local_addr()?.ip())
}

/// The interface that holds `address`.
fn holding(address: IpAddr) -> io::Result<String> {
    let mut first: *mut libc::ifaddrs = std::ptr::null_mut();
    // SAFETY: getifaddrs fills `first` with a list it allocated, freed below
    // and read only before.
    if unsafe { libc::getifaddrs(&raw mut first) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut found = None;
    let mut at = first;
    while !at.is_null() && found.is_none() {
        // SAFETY: every entry of the list is valid until freeifaddrs.
        let entry = unsafe { &*at };
        // SAFETY: an entry's address, when there is one, is its own and as
        // long as its family says.
        let held = unsafe { address_of(entry.ifa_addr) };
        if held == Some(address) && !entry.ifa_name.is_null() {
            // SAFETY: a non-null name is the entry's own and ends in a nul.
            let name = unsafe { CStr::from_ptr(entry.ifa_name) };
            found = Some(name.to_string_lossy().into_owned());
        }
        at = entry.ifa_next;
    }
    // SAFETY: `first` came from getifaddrs and nothing read from it is kept.
    unsafe { libc::freeifaddrs(first) };
    found.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no interface holds {address}, which its routes send from"),
        )
    })
}

/// The IP address `address` holds, if it's one.
///
/// # Safety
///
/// `address` is null, or points at a socket address as long as its family
/// says.
unsafe fn address_of(address: *const libc::sockaddr) -> Option<IpAddr> {
    if address.is_null() {
        return None;
    }
    // SAFETY: as the caller promises.
    let family = i32::from(unsafe { (*address).sa_family });
    if family == libc::AF_INET {
        // SAFETY: an AF_INET address is a sockaddr_in.
        let inet = unsafe { &*address.cast::<libc::sockaddr_in>() };
        Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
            inet.sin_addr.s_addr,
        ))))
    } else if family == libc::AF_INET6 {
        // SAFETY: an AF_INET6 address is a sockaddr_in6.
        let inet6 = unsafe { &*address.cast::<libc::sockaddr_in6>() };
        Some(IpAddr::V6(Ipv6Addr::from(inet6.sin6_addr.s6_addr)))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_machine_sends_to_itself_by_its_loopback() {
        let loopback = toward((Ipv4Addr::LOCALHOST, 47_110).into())
            .expect("no interface was found for this machine's own address");
        assert!(
            loopback.starts_with("lo"),
            "this machine's own address was sent to by {loopback}, not its loopback"
        );
    }

    #[test]
    fn an_address_no_interface_holds_names_none() {
        let error = holding(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 254)))
            .expect_err("an address no interface holds named one");
        assert_eq!(
            error.kind(),
            io::ErrorKind::NotFound,
            "an address no interface holds failed other than as not found: {error}"
        );
    }
}
