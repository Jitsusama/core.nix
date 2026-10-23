//! Which links reach the internet, what they reach without the router, and
//! which network each is on, read from iproute2's JSON.

use serde::Deserialize;
use std::net::{Ipv4Addr, Ipv6Addr};

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
