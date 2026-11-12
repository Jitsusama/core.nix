//! The daemon's own look at the internet: how long this machine takes to
//! reach the gateway node, two public resolvers and a name, so an outage
//! is seen and timed even when eero's cloud, being across the same
//! internet, can't say.
//!
//! It times TCP connects rather than pings. A ping needs a raw socket,
//! which needs root, or a ping socket, which only the groups in
//! `net.ipv4.ping_group_range` may open; a connect's handshake is a round
//! trip any program may time. The lookup is a DNS query of its own, over
//! UDP to the gateway node, which resolves for the home network, so it
//! times the home network's resolving rather than this machine's cache.
//!
//! What this machine sees is the home network's internet only while it's
//! on the home network, so the gateway is timed, and the history told of
//! an outage, only then (`at_home`).

use std::fs;
use std::io::ErrorKind;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model::HomeNetwork;

/// Two public resolvers, at their DNS-over-HTTPS port, which they keep
/// open for anyone: two, so one's own outage isn't read as the
/// internet's.
pub const INTERNET: [SocketAddr; 2] = [
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 443),
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 443),
];

/// The name looked up: one kept for examples, so it always resolves.
pub const NAME: &str = "example.com";

/// How long each part of a look may take, past which it didn't answer.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// The gateway node's DNS port, which it answers for the home network.
const DNS_PORT: u16 = 53;

/// What one look found.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// Whether this machine was on the home network.
    pub home: bool,
    pub gateway_ms: Option<f64>,
    /// The quicker of the public resolvers' connects.
    pub internet_ms: Option<f64>,
    pub dns_ms: Option<f64>,
    /// Whether any public resolver took the connection.
    pub reachable: bool,
}

/// Where a look reaches.
#[derive(Debug, Clone)]
pub struct Targets {
    /// The gateway node, when this machine is on the home network.
    pub gateway: Option<IpAddr>,
    pub internet: Vec<SocketAddr>,
    pub name: String,
    /// The gateway node's DNS port; the tests put a stand-in elsewhere.
    pub dns_port: u16,
}

impl Targets {
    /// The usual targets, the gateway node's when this machine is home.
    pub fn usual(gateway: Option<Ipv4Addr>) -> Targets {
        Targets {
            gateway: gateway.map(IpAddr::V4),
            internet: INTERNET.to_vec(),
            name: NAME.to_owned(),
            dns_port: DNS_PORT,
        }
    }
}

/// Takes a look at every target at once, so it lasts one `timeout` at
/// most.
pub fn look(targets: &Targets, timeout: Duration) -> Reading {
    let gateway = targets
        .gateway
        .map(|ip| SocketAddr::new(ip, targets.dns_port));
    thread::scope(|scope| {
        let gateway_ms = scope.spawn(|| gateway.and_then(|at| connect_ms(at, timeout, true)));
        let dns_ms = scope.spawn(|| gateway.and_then(|at| lookup_ms(at, &targets.name, timeout)));
        let internet: Vec<_> = targets
            .internet
            .iter()
            .map(|at| scope.spawn(move || connect_ms(*at, timeout, false)))
            .collect();
        let internet_ms = internet
            .into_iter()
            .filter_map(|each| each.join().ok().flatten())
            .reduce(f64::min);
        Reading {
            home: targets.gateway.is_some(),
            gateway_ms: gateway_ms.join().ok().flatten(),
            internet_ms,
            dns_ms: dns_ms.join().ok().flatten(),
            reachable: internet_ms.is_some(),
        }
    })
}

/// How long a TCP connect to `at` takes. A refusal is a round trip too,
/// and counts where `refused_counts` says: the gateway node refusing is
/// the node answering, but a public resolver's refusal may come from
/// anything on the way.
fn connect_ms(at: SocketAddr, timeout: Duration, refused_counts: bool) -> Option<f64> {
    let start = Instant::now();
    match TcpStream::connect_timeout(&at, timeout) {
        Ok(_) => Some(ms(start.elapsed())),
        Err(e) if refused_counts && e.kind() == ErrorKind::ConnectionRefused => {
            Some(ms(start.elapsed()))
        }
        Err(_) => None,
    }
}

/// How long the resolver at `at` takes to answer a query for `name`: an
/// answer, or word that there's no such name, since either is the
/// resolver resolving. A failure it reports is no answer.
fn lookup_ms(at: SocketAddr, name: &str, timeout: Duration) -> Option<f64> {
    let local: SocketAddr = if at.is_ipv4() {
        (Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(local).ok()?;
    socket.connect(at).ok()?;
    let id = query_id();
    let start = Instant::now();
    socket.send(&query(id, name)?).ok()?;
    let mut answer = [0; 512];
    loop {
        let left = timeout
            .checked_sub(start.elapsed())
            .filter(|d| !d.is_zero())?;
        socket.set_read_timeout(Some(left)).ok()?;
        let read = socket.recv(&mut answer).ok()?;
        if answers(&answer[..read], id) {
            return Some(ms(start.elapsed()));
        }
    }
}

/// A DNS query for `name`'s IPv4 address, recursion asked for; `None`
/// for a name no query can carry.
fn query(id: u16, name: &str) -> Option<Vec<u8>> {
    let mut query = Vec::with_capacity(17 + name.len());
    query.extend(id.to_be_bytes());
    // Recursion desired, one question.
    query.extend([0x01, 0x00, 0, 1, 0, 0, 0, 0, 0, 0]);
    for label in name.trim_end_matches('.').split('.') {
        let length = u8::try_from(label.len())
            .ok()
            .filter(|l| (1..64).contains(l))?;
        query.push(length);
        query.extend(label.as_bytes());
    }
    // The root, then type A in class IN.
    query.extend([0, 0, 1, 0, 1]);
    Some(query)
}

/// Whether `reply` answers query `id`: a response with that id, saying
/// either the name's records or that there's no such name.
fn answers(reply: &[u8], id: u16) -> bool {
    let [high, low, flags, codes, ..] = reply else {
        return false;
    };
    let response = flags & 0x80 != 0;
    let code = codes & 0x0f;
    u16::from_be_bytes([*high, *low]) == id && response && matches!(code, 0 | 3)
}

/// A query's id, different each look, so a late answer to the last one
/// isn't taken for this one's.
fn query_id() -> u16 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    u16::try_from((nanos ^ (nanos >> 16)) & 0xffff).unwrap_or_default()
}

/// A duration in milliseconds, to a tenth of one.
fn ms(elapsed: Duration) -> f64 {
    (elapsed.as_secs_f64() * 10_000.0).round() / 10.0
}

/// The gateway node's address on the home network: the gateway node's
/// own, or the LAN gateway eero names.
pub fn gateway(model: &HomeNetwork) -> Option<Ipv4Addr> {
    model
        .nodes
        .iter()
        .find(|node| node.gateway)
        .and_then(|node| node.address.as_deref())
        .or(model.internet.gateway_address.as_deref())?
        .parse()
        .ok()
}

/// A route in this machine's table: to which network, and through which
/// gateway, `0.0.0.0` for one on the link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub destination: Ipv4Addr,
    pub mask: Ipv4Addr,
    pub gateway: Ipv4Addr,
}

/// Whether this machine is on the home network: eero lists one of its
/// hardware addresses among the devices on now, and it reaches the
/// gateway node straight, through a route on the link to the node's
/// network or one through the node. Both, since another network may use
/// the same addresses, and eero's list may be from before this machine
/// left.
pub fn at_home(model: &HomeNetwork, own: &[String], routes: &[Route], gateway: Ipv4Addr) -> bool {
    let listed = model
        .devices
        .iter()
        .any(|device| own.iter().any(|mac| mac.eq_ignore_ascii_case(&device.mac)));
    let reached = routes.iter().any(|route| {
        let mask = u32::from(route.mask);
        route.gateway == gateway
            || (route.gateway.is_unspecified()
                && mask != 0
                && u32::from(gateway) & mask == u32::from(route.destination) & mask)
    });
    listed && reached
}

/// This machine's own hostname, as the kernel holds it, for this machine
/// where eero knows none; none where it can't be read, as `own_macs` finds
/// no addresses there either.
pub fn own_hostname() -> Option<String> {
    let host = fs::read_to_string("/proc/sys/kernel/hostname").ok()?;
    let host = host.trim();
    (!host.is_empty()).then(|| host.to_owned())
}

/// This machine's hardware addresses, one for each interface that has
/// one.
pub fn own_macs() -> Vec<String> {
    fs::read_dir("/sys/class/net")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|interface| fs::read_to_string(interface.path().join("address")).ok())
        .map(|mac| mac.trim().to_ascii_lowercase())
        .filter(|mac| mac.len() == 17 && mac != "00:00:00:00:00:00")
        .collect()
}

/// This machine's IPv4 routes, as the kernel lists them.
pub fn routes() -> Vec<Route> {
    fs::read_to_string("/proc/net/route")
        .map(|table| parse_routes(&table))
        .unwrap_or_default()
}

/// The routes in `/proc/net/route`, whose addresses are the kernel's own
/// numbers in hexadecimal, in this machine's byte order.
fn parse_routes(table: &str) -> Vec<Route> {
    let address = |hex: &str| {
        u32::from_str_radix(hex, 16)
            .ok()
            .map(|n| Ipv4Addr::from(n.to_ne_bytes()))
    };
    table
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            Some(Route {
                destination: address(fields.get(1)?)?,
                gateway: address(fields.get(2)?)?,
                mask: address(fields.get(7)?)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::golden::MODEL;
    use std::net::TcpListener;

    const QUICK: Duration = Duration::from_millis(500);

    /// A port on this machine nothing listens on.
    fn closed_port() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap()
    }

    /// A resolver on this machine that answers one query with `code`, and
    /// the port it's at.
    fn resolver(code: u8) -> u16 {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();
        thread::spawn(move || {
            let mut query = [0; 512];
            let (read, from) = socket.recv_from(&mut query).unwrap();
            let mut reply = query[..read].to_vec();
            reply[2] |= 0x80;
            reply[3] = (reply[3] & 0xf0) | code;
            socket.send_to(&reply, from).unwrap();
        });
        port
    }

    fn targets(gateway: Option<IpAddr>, internet: Vec<SocketAddr>, dns_port: u16) -> Targets {
        Targets {
            gateway,
            internet,
            name: NAME.into(),
            dns_port,
        }
    }

    #[test]
    fn a_target_that_takes_the_connection_is_timed() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let at = listener.local_addr().unwrap();

        let reading = look(&targets(None, vec![closed_port(), at], 0), QUICK);

        assert!(reading.reachable, "one resolver answering is enough");
        assert!(reading.internet_ms.is_some_and(|ms| ms < 500.0));
        assert_eq!(reading.gateway_ms, None, "away from home it isn't timed");
        assert!(!reading.home);
    }

    #[test]
    fn a_public_resolver_refusing_is_no_answer_but_the_gateway_refusing_is() {
        let refusing = closed_port();
        let localhost = Some(refusing.ip());

        let reading = look(&targets(localhost, vec![refusing], refusing.port()), QUICK);

        assert!(!reading.reachable);
        assert_eq!(reading.internet_ms, None);
        assert!(reading.gateway_ms.is_some());
    }

    #[test]
    fn the_gateway_resolving_the_name_is_timed() {
        let port = resolver(0);

        let reading = look(
            &targets(Some(Ipv4Addr::LOCALHOST.into()), vec![], port),
            QUICK,
        );

        assert!(reading.dns_ms.is_some_and(|ms| ms < 500.0));
        assert!(reading.home);
    }

    #[test]
    fn a_resolver_failing_to_resolve_or_not_answering_gives_no_time() {
        let failing = resolver(2);
        let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let localhost = Some(Ipv4Addr::LOCALHOST.into());

        let failed = look(&targets(localhost, vec![], failing), QUICK);
        let unanswered = look(
            &targets(localhost, vec![], silent.local_addr().unwrap().port()),
            QUICK,
        );

        assert_eq!(failed.dns_ms, None);
        assert_eq!(unanswered.dns_ms, None);
    }

    #[test]
    fn a_query_asks_for_the_names_address() {
        let query = query(0x1234, "example.com").unwrap();

        assert_eq!(&query[..4], [0x12, 0x34, 0x01, 0x00]);
        assert_eq!(&query[12..], b"\x07example\x03com\x00\x00\x01\x00\x01");
        assert_eq!(super::query(1, "no..name"), None);
    }

    #[test]
    fn a_reply_to_another_query_or_a_query_itself_answers_nothing() {
        assert!(answers(&[0x12, 0x34, 0x81, 0x80], 0x1234));
        assert!(answers(&[0x12, 0x34, 0x81, 0x83], 0x1234), "no such name");
        assert!(!answers(&[0x12, 0x35, 0x81, 0x80], 0x1234));
        assert!(!answers(&[0x12, 0x34, 0x01, 0x00], 0x1234));
        assert!(!answers(&[0x12], 0x1234));
    }

    #[test]
    fn the_route_table_reads_in_the_kernels_byte_order() {
        let table = concat!(
            "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\t",
            "MTU\tWindow\tIRTT\n",
            "wlan0\t00000000\t0100000A\t0003\t0\t0\t600\t00000000\t0\t0\t0\n",
            "wlan0\t0000000A\t00000000\t0001\t0\t0\t600\t00FFFFFF\t0\t0\t0\n",
        );

        let routes = parse_routes(table);

        if cfg!(target_endian = "little") {
            assert_eq!(routes[0].gateway, Ipv4Addr::new(10, 0, 0, 1));
            assert_eq!(routes[1].destination, Ipv4Addr::new(10, 0, 0, 0));
            assert_eq!(routes[1].mask, Ipv4Addr::new(255, 255, 255, 0));
        }
        assert_eq!(routes.len(), 2);
    }

    #[test]
    fn this_machine_is_home_when_eero_lists_it_and_it_reaches_the_gateway_straight() {
        let model: HomeNetwork = serde_json::from_str(MODEL).unwrap();
        let gateway = gateway(&model).unwrap();
        let own = vec!["02:00:00:00:00:01".to_owned()];
        let on_the_link = Route {
            destination: Ipv4Addr::new(10, 0, 0, 0),
            mask: Ipv4Addr::new(255, 255, 255, 0),
            gateway: Ipv4Addr::UNSPECIFIED,
        };
        let elsewhere = Route {
            destination: Ipv4Addr::new(192, 168, 1, 0),
            ..on_the_link
        };
        let through_another = Route {
            destination: Ipv4Addr::UNSPECIFIED,
            mask: Ipv4Addr::UNSPECIFIED,
            gateway: Ipv4Addr::new(192, 168, 1, 1),
        };

        assert_eq!(gateway, Ipv4Addr::new(10, 0, 0, 1));
        assert!(at_home(&model, &own, &[on_the_link], gateway));
        assert!(!at_home(
            &model,
            &own,
            &[elsewhere, through_another],
            gateway
        ));
        assert!(!at_home(
            &model,
            &["02:00:00:00:00:99".into()],
            &[on_the_link],
            gateway
        ));
    }
}
