//! The machine's network links and their addresses (`link:<name>`), as
//! the kernel's route messages say them, in the names `ip -json -details
//! address show` gives them, so the launcher's Network reads them as it
//! read ip and spawns nothing: each link's name, flags, MTU, link type,
//! hardware address and the one it was made with, the kind a virtual one
//! was made as, the bus its card sits on, and its addresses; and, from
//! its folder in `/sys/class/net`, whether the kernel calls it wireless
//! and where its hardware address came from. They're dumped over a
//! netlink socket of the read's own, and [`Heard`] hears the kernel say a
//! link or an address changed, so nothing polls.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

use serde_json::{Map, Value, json};

use crate::Root;

const NET: &str = "/sys/class/net";

/// A message's fixed header: its length, type, flags, sequence and port.
const HEADER: usize = 16;
/// A link message's fixed part: family, type, index, flags and change.
const LINK_FIXED: usize = 16;
/// An address message's fixed part: family, prefix, flags, scope, index.
const ADDRESS_FIXED: usize = 8;

const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
const RTM_NEWLINK: u16 = 16;
const RTM_GETLINK: u16 = 18;
const RTM_NEWADDR: u16 = 20;
const RTM_GETADDR: u16 = 22;
const NLM_F_REQUEST: u16 = 1;
const NLM_F_DUMP: u16 = 0x300;

const IFLA_ADDRESS: u16 = 1;
const IFLA_IFNAME: u16 = 3;
const IFLA_MTU: u16 = 4;
const IFLA_LINKINFO: u16 = 18;
const IFLA_INFO_KIND: u16 = 1;
const IFLA_PERM_ADDRESS: u16 = 54;
const IFLA_PARENT_DEV_BUS_NAME: u16 = 57;
const IFA_ADDRESS: u16 = 1;
const IFA_LOCAL: u16 = 2;
/// An attribute's type, without the flags that say it's nested or in
/// network order.
const TYPE_MASK: u16 = 0x3fff;

/// The groups whose messages say a link, or an IPv4 or IPv6 address,
/// came, changed or went.
const RTMGRP_LINK: u32 = 1;
const RTMGRP_IPV4_IFADDR: u32 = 0x10;
const RTMGRP_IPV6_IFADDR: u32 = 0x100;

/// How long a dump's answer is waited for, so a kernel that never ends
/// one can't hold the read.
const PATIENCE: Duration = Duration::from_secs(2);

/// The flags ip says by name, in its order; it says `NO-CARRIER` first
/// for a link up but not running, as iproute2's `print_link_flags` does:
/// the kernel sets running from the link's operating state, so a Wi-Fi
/// card still authenticating, its carrier up but dormant, has none.
const FLAGS: [(u32, &str); 8] = [
    (0x8, "LOOPBACK"),
    (0x2, "BROADCAST"),
    (0x10, "POINTOPOINT"),
    (0x1000, "MULTICAST"),
    (0x80, "NOARP"),
    (0x100, "PROMISC"),
    (0x1, "UP"),
    (0x10000, "LOWER_UP"),
];
const IFF_UP: u32 = 0x1;
const IFF_RUNNING: u32 = 0x40;

/// Every link's record, in the kernel's order of their indexes, or none
/// where the kernel gives no route socket.
pub fn links(root: &Root) -> Vec<Value> {
    let Some(links) = dump(RTM_GETLINK, &[0; LINK_FIXED]) else {
        return Vec::new();
    };
    let addresses = dump(RTM_GETADDR, &[0; ADDRESS_FIXED]).unwrap_or_default();
    records(&links, &addresses, |name, file| {
        root.said(&format!("{NET}/{name}/{file}"))
    })
}

/// The records these dumps of links and addresses say, with what `sys`
/// says of each link's folder in `/sys/class/net`.
pub fn records(
    links: &[u8],
    addresses: &[u8],
    sys: impl Fn(&str, &str) -> Option<String>,
) -> Vec<Value> {
    let each_address: Vec<(u32, Value)> = messages(addresses)
        .into_iter()
        .filter(|(kind, _)| *kind == RTM_NEWADDR)
        .filter_map(|(_, body)| address(body))
        .collect();
    messages(links)
        .into_iter()
        .filter(|(kind, _)| *kind == RTM_NEWLINK)
        .filter_map(|(_, body)| {
            let (index, mut record) = link(body)?;
            let name = record["ifname"].as_str()?.to_owned();
            let held: Vec<Value> = each_address
                .iter()
                .filter(|(of, _)| *of == index)
                .map(|(_, each)| each.clone())
                .collect();
            record["addr_info"] = Value::Array(held);
            record["wireless"] = json!(
                sys(&name, "uevent").is_some_and(|said| said.lines().any(|l| l == "DEVTYPE=wlan"))
            );
            record["addr_assign_type"] =
                json!(sys(&name, "addr_assign_type").and_then(|said| said.parse::<u64>().ok()));
            record["key"] = json!(format!("link:{name}"));
            record["kind"] = json!("link");
            Some(record)
        })
        .collect()
}

/// A link's index and record, in ip's names, from a link message's body.
fn link(body: &[u8]) -> Option<(u32, Value)> {
    let fixed = body.get(..LINK_FIXED)?;
    let arp = u16::from_ne_bytes([fixed[2], fixed[3]]);
    let index = u32::from_ne_bytes(fixed[4..8].try_into().ok()?);
    let flags = u32::from_ne_bytes(fixed[8..12].try_into().ok()?);
    let mut said = Map::new();
    said.insert("ifindex".to_owned(), json!(index));
    said.insert("flags".to_owned(), json!(flag_names(flags)));
    said.insert("link_type".to_owned(), json!(link_type(arp)));
    let mut permanent = None;
    for (kind, value) in attributes(&body[LINK_FIXED..]) {
        match kind {
            IFLA_IFNAME => {
                said.insert("ifname".to_owned(), json!(text(value)));
            }
            IFLA_MTU => {
                if let Ok(bytes) = value.try_into() {
                    said.insert("mtu".to_owned(), json!(u32::from_ne_bytes(bytes)));
                }
            }
            IFLA_ADDRESS => {
                said.insert("address".to_owned(), json!(hardware(value)));
            }
            IFLA_PERM_ADDRESS => permanent = Some(hardware(value)),
            IFLA_LINKINFO => {
                if let Some((_, made)) = attributes(value)
                    .into_iter()
                    .find(|(kind, _)| *kind == IFLA_INFO_KIND)
                {
                    said.insert("linkinfo".to_owned(), json!({"info_kind": text(made)}));
                }
            }
            IFLA_PARENT_DEV_BUS_NAME => {
                said.insert("parentbus".to_owned(), json!(text(value)));
            }
            _ => {}
        }
    }
    // ip says the address a card was made with only where it uses another.
    if let Some(permanent) = permanent
        && said.get("address").and_then(Value::as_str) != Some(permanent.as_str())
    {
        said.insert("permaddr".to_owned(), json!(permanent));
    }
    said.contains_key("ifname")
        .then_some((index, Value::Object(said)))
}

/// An address's link index and entry in ip's `addr_info`, from an address
/// message's body; none for a family other than IPv4 or IPv6.
fn address(body: &[u8]) -> Option<(u32, Value)> {
    let fixed = body.get(..ADDRESS_FIXED)?;
    let index = u32::from_ne_bytes(fixed[4..8].try_into().ok()?);
    let (mut local, mut peer) = (None, None);
    for (kind, value) in attributes(&body[ADDRESS_FIXED..]) {
        match kind {
            IFA_LOCAL => local = Some(value),
            IFA_ADDRESS => peer = Some(value),
            _ => {}
        }
    }
    // IPv4 says its own address as IFA_LOCAL, IPv6 only as IFA_ADDRESS.
    let own = local.or(peer)?;
    let (family, said) = match i32::from(fixed[0]) {
        libc::AF_INET => (
            "inet",
            Ipv4Addr::from(<[u8; 4]>::try_from(own).ok()?).to_string(),
        ),
        libc::AF_INET6 => (
            "inet6",
            Ipv6Addr::from(<[u8; 16]>::try_from(own).ok()?).to_string(),
        ),
        _ => return None,
    };
    Some((
        index,
        json!({"family": family, "local": said, "prefixlen": fixed[1], "scope": scope(fixed[3])}),
    ))
}

/// A scope as ip names it.
fn scope(scope: u8) -> Value {
    match scope {
        0 => json!("global"),
        200 => json!("site"),
        253 => json!("link"),
        254 => json!("host"),
        255 => json!("nowhere"),
        other => json!(other.to_string()),
    }
}

/// A link type as ip names it, by its hardware type: the three Network
/// tells apart, and the rest by number.
fn link_type(arp: u16) -> String {
    match arp {
        1 => "ether".to_owned(),
        772 => "loopback".to_owned(),
        65534 => "none".to_owned(),
        other => other.to_string(),
    }
}

fn flag_names(flags: u32) -> Vec<&'static str> {
    let mut names = Vec::new();
    if flags & IFF_UP != 0 && flags & IFF_RUNNING == 0 {
        names.push("NO-CARRIER");
    }
    names.extend(
        FLAGS
            .iter()
            .filter(|(bit, _)| flags & bit != 0)
            .map(|(_, name)| *name),
    );
    names
}

/// A hardware address in ip's form, `aa:bb:cc:dd:ee:ff`.
fn hardware(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// A string attribute, without the nul the kernel ends it with.
fn text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// The messages in what a netlink socket said, each as its type and body.
pub fn messages(said: &[u8]) -> Vec<(u16, &[u8])> {
    let mut found = Vec::new();
    let mut rest = said;
    while rest.len() >= HEADER {
        let length = u32::from_ne_bytes([rest[0], rest[1], rest[2], rest[3]]);
        let Ok(length) = usize::try_from(length) else {
            break;
        };
        if length < HEADER || length > rest.len() {
            break;
        }
        found.push((
            u16::from_ne_bytes([rest[4], rest[5]]),
            &rest[HEADER..length],
        ));
        rest = &rest[aligned(length).min(rest.len())..];
    }
    found
}

/// The attributes in a message's body past its fixed part, each as its
/// type and value.
fn attributes(said: &[u8]) -> Vec<(u16, &[u8])> {
    let mut found = Vec::new();
    let mut rest = said;
    while rest.len() >= 4 {
        let length = usize::from(u16::from_ne_bytes([rest[0], rest[1]]));
        if length < 4 || length > rest.len() {
            break;
        }
        found.push((
            u16::from_ne_bytes([rest[2], rest[3]]) & TYPE_MASK,
            &rest[4..length],
        ));
        rest = &rest[aligned(length).min(rest.len())..];
    }
    found
}

/// A length rounded up to the four bytes netlink aligns each part to.
const fn aligned(length: usize) -> usize {
    (length + 3) & !3
}

/// A route socket, bound to these groups, or none where the kernel gives
/// none.
fn route_socket(groups: u32) -> Option<OwnedFd> {
    // SAFETY: socket takes only numbers and returns a descriptor of its
    // own, or -1.
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_RAW | libc::SOCK_CLOEXEC,
            libc::NETLINK_ROUTE,
        )
    };
    if fd < 0 {
        return None;
    }
    // SAFETY: the descriptor was just made, and nothing else holds it.
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    // SAFETY: an all-zero sockaddr_nl is a valid one, of no family yet.
    let mut at: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    at.nl_family = u16::try_from(libc::AF_NETLINK).ok()?;
    at.nl_groups = groups;
    let size = u32::try_from(std::mem::size_of::<libc::sockaddr_nl>()).ok()?;
    // SAFETY: bind reads only the address it's given, of the size said.
    let bound = unsafe { libc::bind(fd.as_raw_fd(), (&raw const at).cast(), size) };
    (bound == 0).then_some(fd)
}

/// What the kernel answers a dump of this kind asked with this fixed
/// part, every message of it, or none where it gave no answer in time.
fn dump(kind: u16, fixed: &[u8]) -> Option<Vec<u8>> {
    let fd = route_socket(0)?;
    let wait = libc::timeval {
        tv_sec: libc::time_t::try_from(PATIENCE.as_secs()).ok()?,
        tv_usec: 0,
    };
    let size = u32::try_from(std::mem::size_of::<libc::timeval>()).ok()?;
    // SAFETY: setsockopt reads only the timeval it's given, of its size.
    let waits = unsafe {
        libc::setsockopt(
            fd.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_RCVTIMEO,
            (&raw const wait).cast(),
            size,
        )
    };
    // A socket that won't time its reads out could hold the read for good.
    if waits != 0 {
        return None;
    }
    let length = u32::try_from(HEADER + fixed.len()).ok()?;
    let mut ask = Vec::with_capacity(HEADER + fixed.len());
    ask.extend_from_slice(&length.to_ne_bytes());
    ask.extend_from_slice(&kind.to_ne_bytes());
    ask.extend_from_slice(&(NLM_F_REQUEST | NLM_F_DUMP).to_ne_bytes());
    ask.extend_from_slice(&1u32.to_ne_bytes());
    ask.extend_from_slice(&0u32.to_ne_bytes());
    ask.extend_from_slice(fixed);
    // SAFETY: send reads only the ask's own bytes.
    let sent = unsafe { libc::send(fd.as_raw_fd(), ask.as_ptr().cast(), ask.len(), 0) };
    if usize::try_from(sent).ok() != Some(ask.len()) {
        return None;
    }
    let mut said = Vec::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        // SAFETY: recv writes at most the buffer's length into it.
        let read =
            unsafe { libc::recv(fd.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len(), 0) };
        let read = usize::try_from(read).ok().filter(|read| *read > 0)?;
        let part = &buffer[..read];
        let ended = messages(part)
            .iter()
            .any(|(kind, _)| *kind == NLMSG_DONE || *kind == NLMSG_ERROR);
        said.extend_from_slice(part);
        if ended {
            return Some(said);
        }
    }
}

/// A route socket that hears the kernel say a link or an address came,
/// changed or went.
pub struct Heard {
    fd: OwnedFd,
}

impl Heard {
    /// Hears the links and addresses from now, or none where the kernel
    /// gives no route socket.
    pub fn on() -> Option<Self> {
        let fd = route_socket(RTMGRP_LINK | RTMGRP_IPV4_IFADDR | RTMGRP_IPV6_IFADDR)?;
        Some(Self { fd })
    }

    /// Whether the kernel said anything changed within `wait`, every
    /// message of a burst taken at once, as one address coming says
    /// several. A burst too quick to keep, which the kernel says by
    /// refusing the read, counts as heard.
    pub fn wait(&mut self, wait: Duration) -> bool {
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ms = libc::c_int::try_from(wait.as_millis()).unwrap_or(libc::c_int::MAX);
        // SAFETY: poll reads and writes only the one pollfd it's given.
        if unsafe { libc::poll(&raw mut poll, 1, ms) } <= 0 {
            return false;
        }
        let mut buffer = vec![0u8; 1 << 16];
        loop {
            // SAFETY: recv writes at most the buffer's length into it.
            let read = unsafe {
                libc::recv(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    libc::MSG_DONTWAIT,
                )
            };
            if read < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock
            {
                return true;
            }
            if read <= 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::ENOBUFS) {
                return true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attribute(kind: u16, value: &[u8]) -> Vec<u8> {
        let mut said = Vec::new();
        said.extend_from_slice(&u16::try_from(4 + value.len()).unwrap().to_ne_bytes());
        said.extend_from_slice(&kind.to_ne_bytes());
        said.extend_from_slice(value);
        said.resize(aligned(said.len()), 0);
        said
    }

    fn message(kind: u16, body: &[u8]) -> Vec<u8> {
        let mut said = Vec::new();
        said.extend_from_slice(&u32::try_from(HEADER + body.len()).unwrap().to_ne_bytes());
        said.extend_from_slice(&kind.to_ne_bytes());
        said.extend_from_slice(&[0; 10]);
        said.extend_from_slice(body);
        said.resize(aligned(said.len()), 0);
        said
    }

    fn link_message(index: u32, arp: u16, flags: u32, attributes: &[Vec<u8>]) -> Vec<u8> {
        let mut body = vec![0, 0];
        body.extend_from_slice(&arp.to_ne_bytes());
        body.extend_from_slice(&index.to_ne_bytes());
        body.extend_from_slice(&flags.to_ne_bytes());
        body.extend_from_slice(&[0; 4]);
        for each in attributes {
            body.extend_from_slice(each);
        }
        message(RTM_NEWLINK, &body)
    }

    fn address_message(
        index: u32,
        family: u8,
        prefix: u8,
        scope: u8,
        kind: u16,
        own: &[u8],
    ) -> Vec<u8> {
        let mut body = vec![family, prefix, 0, scope];
        body.extend_from_slice(&index.to_ne_bytes());
        body.extend_from_slice(&attribute(kind, own));
        message(RTM_NEWADDR, &body)
    }

    fn named(name: &str) -> Vec<u8> {
        attribute(IFLA_IFNAME, format!("{name}\0").as_bytes())
    }

    /// A card on a cable that uses an address other than its own, with two
    /// addresses, and a wireless card up with nothing to reach.
    fn dumps() -> (Vec<u8>, Vec<u8>) {
        let mut links = link_message(
            2,
            1,
            0x1 | 0x2 | 0x40 | 0x1000 | 0x10000,
            &[
                named("enp1s0"),
                attribute(IFLA_MTU, &1500u32.to_ne_bytes()),
                attribute(IFLA_ADDRESS, &[0x02, 0, 0, 0, 0, 0x01]),
                attribute(IFLA_PERM_ADDRESS, &[0x8c, 0x16, 0x45, 0, 0, 0x02]),
                attribute(IFLA_PARENT_DEV_BUS_NAME, b"pci\0"),
            ],
        );
        links.extend(link_message(
            3,
            1,
            0x1 | 0x1000,
            &[named("wlan0"), attribute(IFLA_ADDRESS, &[0xaa; 6])],
        ));
        links.extend(link_message(
            4,
            65534,
            0x1 | 0x40 | 0x10000,
            &[
                named("wg0"),
                attribute(IFLA_LINKINFO, &attribute(IFLA_INFO_KIND, b"wireguard\0")),
            ],
        ));
        links.extend(message(NLMSG_DONE, &[0; 4]));
        let six = "2001:db8::5".parse::<Ipv6Addr>().unwrap().octets();
        let mut addresses = address_message(2, 2, 22, 0, IFA_LOCAL, &[10, 0, 0, 5]);
        addresses.extend(address_message(2, 10, 64, 0, IFA_ADDRESS, &six));
        addresses.extend(message(NLMSG_DONE, &[0; 4]));
        (links, addresses)
    }

    fn sys(name: &str, file: &str) -> Option<String> {
        match (name, file) {
            ("wlan0", "uevent") => Some("DEVTYPE=wlan\nINTERFACE=wlan0".to_owned()),
            ("enp1s0", "addr_assign_type") => Some("3".to_owned()),
            _ => None,
        }
    }

    #[test]
    fn a_link_is_said_in_ips_own_names() {
        let (links, addresses) = dumps();
        let said = records(&links, &addresses, sys);
        assert_eq!(said.len(), 3, "every link is said: {said:?}");
        let cable = &said[0];
        assert_eq!(cable["key"], json!("link:enp1s0"));
        assert_eq!(cable["ifname"], json!("enp1s0"));
        assert_eq!(
            cable["flags"],
            json!(["BROADCAST", "MULTICAST", "UP", "LOWER_UP"])
        );
        assert_eq!(cable["mtu"], json!(1500));
        assert_eq!(cable["link_type"], json!("ether"));
        assert_eq!(cable["address"], json!("02:00:00:00:00:01"));
        assert_eq!(cable["permaddr"], json!("8c:16:45:00:00:02"));
        assert_eq!(cable["parentbus"], json!("pci"));
        assert_eq!(cable["addr_assign_type"], json!(3));
        assert_eq!(cable["wireless"], json!(false));
        assert_eq!(
            cable["addr_info"],
            json!([
                {"family": "inet", "local": "10.0.0.5", "prefixlen": 22, "scope": "global"},
                {"family": "inet6", "local": "2001:db8::5", "prefixlen": 64, "scope": "global"},
            ])
        );
    }

    #[test]
    fn a_link_up_with_nothing_at_the_other_end_says_no_carrier() {
        let (links, addresses) = dumps();
        let said = records(&links, &addresses, sys);
        assert_eq!(said[1]["flags"], json!(["NO-CARRIER", "MULTICAST", "UP"]));
        assert_eq!(said[1]["wireless"], json!(true));
        assert_eq!(said[1]["addr_info"], json!([]));
        assert!(
            said[1].get("permaddr").is_none(),
            "no address it was made with: {}",
            said[1]
        );
    }

    #[test]
    fn a_link_with_a_carrier_but_dormant_says_no_carrier_as_ip_does() {
        let mut links = link_message(5, 1, 0x1 | 0x1000 | 0x10000, &[named("wlan1")]);
        links.extend(message(NLMSG_DONE, &[0; 4]));
        let said = records(&links, &message(NLMSG_DONE, &[0; 4]), sys);
        assert_eq!(
            said[0]["flags"],
            json!(["NO-CARRIER", "MULTICAST", "UP", "LOWER_UP"])
        );
    }

    #[test]
    fn a_made_link_says_what_it_was_made_as() {
        let (links, addresses) = dumps();
        let said = records(&links, &addresses, sys);
        assert_eq!(said[2]["link_type"], json!("none"));
        assert_eq!(said[2]["linkinfo"], json!({"info_kind": "wireguard"}));
        assert!(
            said[2].get("address").is_none(),
            "a tunnel has no hardware address"
        );
    }

    #[test]
    fn a_message_cut_short_ends_what_is_read() {
        let (links, _) = dumps();
        let cut = &links[..links.len() / 2];
        assert!(messages(cut).len() < messages(&links).len());
    }
}
