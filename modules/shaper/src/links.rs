//! Which links reach the internet, what they reach without the router, and
//! which network each is on, read from iproute2's JSON.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::net::{Ipv4Addr, Ipv6Addr};

/// What a wire spends on each full frame beyond the 1,514 bytes CAKE
/// counts: the frame check, preamble and the gap between frames.
const FRAMING: u64 = 4 + 8 + 12;

/// How far under the wire the shaper holds a link, in hundredths: the
/// queue must be the shaper's, and two equal rates leave it to chance.
const UNDER: u64 = 95;

/// How far under the wire a download is held, in hundredths. What arrives
/// is slowed only by its sender backing off, so it comes in bursts past
/// the rate, and the port toward the machine has a buffer of a few
/// milliseconds. At 95 a download on a 100 Mbit/s wire arrived at 92, and
/// that port dropped 2% of everything coming in, a call's packets among
/// them, with no queue for the shaper's pings to see.
const UNDER_DOWN: u64 = 85;

/// The most a link carries each way, in kbit/s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ceiling {
    pub up: u64,
    pub down: u64,
}

/// The most a link can carry in the shaper's own counting, in kbit/s, from
/// the Mbit/s Ethernet says; Wi-Fi says nothing, and is left to the line.
/// A link speed counts the wire's framing, which CAKE doesn't, so a rate
/// of 100 Mbit/s on a 100 Mbit/s wire sent it 1.6% more than it carried:
/// the router's port toward the machine held a queue of 8 ms, under the
/// delay the shaper cuts at, for as long as a download ran.
pub fn ceiling(mbit: Option<u64>) -> Ceiling {
    match mbit {
        Some(mbit) if mbit > 0 && mbit < 1_000_000 => {
            let wire = mbit * 1000 * 1514 / (1514 + FRAMING);
            Ceiling {
                up: wire * UNDER / 100,
                down: wire * UNDER_DOWN / 100,
            }
        }
        _ => Ceiling {
            up: 2_000_000,
            down: 2_000_000,
        },
    }
}

#[derive(Deserialize)]
struct Route {
    dev: Option<String>,
    gateway: Option<String>,
}

/// A link holding a default route in the main table, and its gateway.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Uplink {
    pub name: String,
    pub gateway: Option<String>,
}

/// The links with a default route, from `ip -j route show default` for
/// both families, each named once, in the order the routes came. Tunnels
/// that route by a table of their own (WARP, Tailscale) hold no default
/// route in main, so they never appear.
pub fn uplinks(v4: &str, v6: &str) -> Vec<Uplink> {
    let mut found: Vec<Uplink> = Vec::new();
    for json in [v4, v6] {
        let routes: Vec<Route> = serde_json::from_str(json).unwrap_or_default();
        for route in routes {
            let Some(name) = route.dev else { continue };
            if name.starts_with("shaper") {
                continue;
            }
            match found.iter_mut().find(|u| u.name == name) {
                Some(u) if u.gateway.is_none() => u.gateway = route.gateway,
                Some(_) => {}
                None => found.push(Uplink {
                    name,
                    gateway: route.gateway,
                }),
            }
        }
    }
    found
}

/// Which links follow another's rate, by name: a link whose router is the
/// same as one before it in the routes' order, so both lead to one line.
/// Two estimates of one line would each take it whole, and their rates
/// would add up past it. A link known only by its own name (no router's
/// address learned yet) follows nobody.
pub fn leaders(order: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut follows = BTreeMap::new();
    for (i, (name, network)) in order.iter().enumerate() {
        if network.starts_with("link ") {
            continue;
        }
        if let Some((leader, _)) = order[..i].iter().find(|(_, n)| n == network) {
            follows.insert(name.to_string(), leader.to_string());
        }
    }
    follows
}

/// What each leading link's line carries, in kbit/s sent and received,
/// from what every link carried: its own and its followers', since they
/// reach the line through the one router.
pub fn line_loads(
    carried: &BTreeMap<String, (f64, f64)>,
    follows: &BTreeMap<String, String>,
) -> BTreeMap<String, (f64, f64)> {
    let mut loads: BTreeMap<String, (f64, f64)> = carried
        .iter()
        .filter(|(name, _)| !follows.contains_key(*name))
        .map(|(name, load)| (name.clone(), *load))
        .collect();
    for (name, (sent, received)) in carried {
        if let Some(load) = follows.get(name).and_then(|l| loads.get_mut(l)) {
            load.0 += sent;
            load.1 += received;
        }
    }
    loads
}

#[derive(Deserialize)]
struct Link {
    #[serde(default)]
    addr_info: Vec<Address>,
}

#[derive(Deserialize)]
struct Address {
    family: String,
    local: String,
    prefixlen: u8,
    #[serde(default)]
    scope: String,
}

/// The networks a link's own addresses sit on, from `ip -j addr show dev`,
/// as nft writes prefixes: the network's first address and its length.
/// Link-local addresses are left out, since the table covers them all.
pub fn local_networks(json: &str) -> (Vec<String>, Vec<String>) {
    let links: Vec<Link> = serde_json::from_str(json).unwrap_or_default();
    let (mut v4, mut v6) = (Vec::new(), Vec::new());
    for address in links.into_iter().flat_map(|l| l.addr_info) {
        if address.scope != "global" {
            continue;
        }
        let len = address.prefixlen;
        match address.family.as_str() {
            "inet" => {
                let Ok(ip) = address.local.parse::<Ipv4Addr>() else {
                    continue;
                };
                let mask = if len == 0 {
                    0
                } else {
                    u32::MAX << (32 - len.min(32))
                };
                v4.push(format!("{}/{len}", Ipv4Addr::from(u32::from(ip) & mask)));
            }
            "inet6" => {
                let Ok(ip) = address.local.parse::<Ipv6Addr>() else {
                    continue;
                };
                let mask = if len == 0 {
                    0
                } else {
                    u128::MAX << (128 - len.min(128))
                };
                v6.push(format!("{}/{len}", Ipv6Addr::from(u128::from(ip) & mask)));
            }
            _ => {}
        }
    }
    v4.sort();
    v4.dedup();
    v6.sort();
    v6.dedup();
    (v4, v6)
}

#[derive(Deserialize)]
struct Neighbour {
    lladdr: Option<String>,
}

/// The gateway's hardware address, from `ip -j neigh show dev <link> <gw>`,
/// which names the network: the same router is the same line.
pub fn gateway_address(json: &str) -> Option<String> {
    let found: Vec<Neighbour> = serde_json::from_str(json).ok()?;
    found.into_iter().find_map(|n| n.lladdr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_s_ceiling_is_under_what_its_wire_carries() {
        // A 100 Mbit/s wire carries 98.4 of CAKE's frames, so the shaper
        // must hold under that, not at 100, or the router's port queues.
        assert_eq!(ceiling(Some(100)).up, 93_517);
        assert_eq!(ceiling(Some(1_000)).up, 935_175);
        // Wi-Fi gives no speed, and a link down gives -1 or nothing.
        let none = Ceiling {
            up: 2_000_000,
            down: 2_000_000,
        };
        assert_eq!(ceiling(None), none);
        assert_eq!(ceiling(Some(0)), none);
    }

    #[test]
    fn a_download_is_held_further_under_the_wire_than_an_upload() {
        // What arrives can only be slowed by the sender backing off, so it
        // comes in bursts past the rate, and the port toward the machine
        // drops them once its little buffer is full.
        assert_eq!(ceiling(Some(100)).down, 83_673);
        assert_eq!(ceiling(Some(1_000)).down, 836_735);
    }

    #[test]
    fn a_link_to_the_same_router_follows_the_one_before_it() {
        let router = "6c:ae:f6:00:00:01";
        let follows = leaders(&[
            ("eth0", router),
            ("wlan0", router),
            ("usb0", "02:00:00:00:00:02"),
            ("eth1", router),
        ]);
        assert_eq!(follows.get("wlan0").map(String::as_str), Some("eth0"));
        assert_eq!(follows.get("eth1").map(String::as_str), Some("eth0"));
        assert_eq!(follows.get("eth0"), None);
        assert_eq!(follows.get("usb0"), None);
    }

    #[test]
    fn a_line_carries_what_every_link_to_its_router_carries() {
        // A download over Wi-Fi while the cable leads fills the one line:
        // the cable's controller must see it, or it reads the line's queue
        // against an idle download and cuts the upload instead.
        let carried = BTreeMap::from([
            ("eth0".to_string(), (500.0, 2_000.0)),
            ("wlan0".to_string(), (300.0, 90_000.0)),
            ("usb0".to_string(), (100.0, 400.0)),
        ]);
        let follows = BTreeMap::from([("wlan0".to_string(), "eth0".to_string())]);
        let loads = line_loads(&carried, &follows);
        assert_eq!(loads.get("eth0"), Some(&(800.0, 92_000.0)));
        assert_eq!(loads.get("usb0"), Some(&(100.0, 400.0)));
        assert_eq!(loads.get("wlan0"), None);
    }

    #[test]
    fn a_link_whose_router_isnt_known_follows_nobody() {
        let follows = leaders(&[("eth0", "link eth0"), ("wlan0", "link eth0")]);
        assert!(follows.is_empty());
    }

    #[test]
    fn each_link_with_a_default_route_is_an_uplink_once() {
        let v4 = r#"[{"dst":"default","gateway":"192.168.4.1","dev":"enp0s20f0u1u2","metric":100},
                    {"dst":"default","gateway":"192.168.4.1","dev":"wlp0s20f3","metric":600}]"#;
        let v6 = r#"[{"dst":"default","gateway":"fe80::1","dev":"wlp0s20f3","metric":600}]"#;
        assert_eq!(
            uplinks(v4, v6),
            [
                Uplink {
                    name: "enp0s20f0u1u2".into(),
                    gateway: Some("192.168.4.1".into())
                },
                Uplink {
                    name: "wlp0s20f3".into(),
                    gateway: Some("192.168.4.1".into())
                },
            ]
        );
    }

    #[test]
    fn a_link_with_only_an_ipv6_route_is_an_uplink() {
        let v6 = r#"[{"dst":"default","gateway":"fe80::1","dev":"eth0"}]"#;
        assert_eq!(
            uplinks("[]", v6),
            [Uplink {
                name: "eth0".into(),
                gateway: Some("fe80::1".into())
            }]
        );
    }

    #[test]
    fn nothing_routed_is_no_uplinks() {
        assert!(uplinks("", "[]").is_empty());
    }

    #[test]
    fn a_links_networks_are_its_addresses_prefixes() {
        let json = r#"[{"ifname":"enp","addr_info":[
            {"family":"inet","local":"192.168.4.45","prefixlen":22,"scope":"global"},
            {"family":"inet6","local":"2001:db8:abcd:1::37a3","prefixlen":64,"scope":"global"},
            {"family":"inet6","local":"fe80::37a3","prefixlen":64,"scope":"link"}]}]"#;
        let (v4, v6) = local_networks(json);
        assert_eq!(v4, ["192.168.4.0/22"]);
        assert_eq!(v6, ["2001:db8:abcd:1::/64"]);
    }

    #[test]
    fn the_gateways_hardware_address_names_the_network() {
        let json = r#"[{"dst":"192.168.4.1","lladdr":"6c:ae:f6:ba:85:86","state":["REACHABLE"]}]"#;
        assert_eq!(gateway_address(json).as_deref(), Some("6c:ae:f6:ba:85:86"));
        assert_eq!(gateway_address("[]"), None);
    }
}
