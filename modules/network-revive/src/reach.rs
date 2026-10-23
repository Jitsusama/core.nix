//! What the machine can reach: each link networkd describes as leading
//! beyond it, with its addresses, and the DNS servers resolved asks. Two
//! readings that reach the same are the same network, however many changes
//! came between them, so a link dropping and coming back as it was changes
//! nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reach {
    links: BTreeMap<String, BTreeSet<String>>,
    dns: BTreeSet<String>,
}

impl Reach {
    /// Reads networkd's Describe JSON and the servers resolved lists.
    pub fn read(description: &Value, dns: BTreeSet<String>) -> Reach {
        let links = description["Interfaces"].as_array().into_iter().flatten();
        let links = links.filter(|link| leads_beyond(link));
        Reach {
            links: links.filter_map(uplink).collect(),
            dns,
        }
    }
}

/// Each link with its addresses, then the servers, as the journal says
/// what the machine reaches.
impl fmt::Display for Reach {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let links: Vec<String> = (self.links.iter())
            .map(|(name, addresses)| {
                let addresses = addresses.iter().map(String::as_str);
                [name.as_str()]
                    .into_iter()
                    .chain(addresses)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let servers: Vec<&str> = self.dns.iter().map(String::as_str).collect();
        let links = if links.is_empty() {
            "no link".to_string()
        } else {
            links.join(", ")
        };
        let servers = if servers.is_empty() {
            "no server".to_string()
        } else {
            servers.join(", ")
        };
        write!(f, "{links}; asking {servers}")
    }
}

/// A routable link, other than a bridge or a veth: those are made for the
/// guests and containers on this machine, and starting one changes nothing
/// it reaches. A VPN's tunnel counts, since joining one changes what
/// answers.
fn leads_beyond(link: &Value) -> bool {
    let local = matches!(link["Kind"].as_str(), Some("bridge" | "veth"));
    link["OperationalState"] == "routable" && !local
}

fn uplink(link: &Value) -> Option<(String, BTreeSet<String>)> {
    let name = link["Name"].as_str()?.to_string();
    let addresses = link["Addresses"].as_array().into_iter().flatten();
    let addresses = addresses
        .filter(|address| lasting(address))
        .filter_map(address);
    Some((name, addresses.collect()))
}

/// An address beyond the link itself that stays as long as the network
/// does. A privacy address is left out: IPv6 replaces those through the day
/// on the same network.
fn lasting(address: &Value) -> bool {
    let global = address["Scope"] == UNIVERSE;
    let temporary = address["Flags"]
        .as_u64()
        .is_some_and(|flags| flags & TEMPORARY != 0);
    global && !temporary
}

/// The kernel's scope for an address that reaches beyond its link.
const UNIVERSE: u64 = 0;

/// The kernel's flag for an IPv6 privacy address, `IFA_F_TEMPORARY`.
const TEMPORARY: u64 = 0x01;

fn address(address: &Value) -> Option<String> {
    let text = address["AddressString"].as_str()?;
    let prefix = address["PrefixLength"].as_u64()?;
    Some(format!("{text}/{prefix}"))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn wifi(address: &str) -> Value {
        json!({
            "Name": "wlan0",
            "Type": "wlan",
            "OperationalState": "routable",
            "Addresses": [
                {"AddressString": address, "PrefixLength": 22, "Scope": 0, "Flags": 128},
                {"AddressString": "fe80::1", "PrefixLength": 64, "Scope": 253, "Flags": 128},
            ],
        })
    }

    fn reach(links: &[Value], dns: &[&str]) -> Reach {
        let dns = dns.iter().map(ToString::to_string).collect();
        Reach::read(&json!({ "Interfaces": links }), dns)
    }

    #[test]
    fn it_reads_as_each_link_with_its_addresses_then_the_servers() {
        let mut ethernet = wifi("192.168.1.9");
        ethernet["Name"] = json!("eth0");

        let reach = reach(&[wifi("10.0.0.2"), ethernet], &["10.0.0.1%3", "127.0.2.2"]);

        assert_eq!(
            reach.to_string(),
            "eth0 192.168.1.9/22, wlan0 10.0.0.2/22; asking 10.0.0.1%3, 127.0.2.2"
        );
    }

    #[test]
    fn it_reads_as_nothing_when_it_reaches_nothing() {
        assert_eq!(reach(&[], &[]).to_string(), "no link; asking no server");
    }

    #[test]
    fn an_address_on_the_link_alone_reaches_nothing_more() {
        let mut moved = wifi("10.0.0.2");
        moved["Addresses"][1]["AddressString"] = json!("fe80::2");

        assert_eq!(reach(&[wifi("10.0.0.2")], &[]), reach(&[moved], &[]));
    }

    #[test]
    fn another_address_is_another_network() {
        assert_ne!(
            reach(&[wifi("10.0.0.2")], &[]),
            reach(&[wifi("192.168.1.9")], &[])
        );
    }

    #[test]
    fn other_dns_servers_are_another_network() {
        let before = reach(&[wifi("10.0.0.2")], &["10.0.0.1"]);

        let after = reach(&[wifi("10.0.0.2")], &["127.0.2.2"]);

        assert_ne!(before, after);
    }

    #[test]
    fn a_link_that_stops_reaching_beyond_is_no_longer_counted() {
        let mut down = wifi("10.0.0.2");
        down["OperationalState"] = json!("no-carrier");

        assert_eq!(reach(&[down], &[]), Reach::default());
    }

    #[test]
    fn a_bridge_or_veth_for_guests_changes_nothing() {
        let bridge = json!({"Name": "virbr0", "Kind": "bridge", "OperationalState": "routable"});
        let veth = json!({"Name": "veth0", "Kind": "veth", "OperationalState": "routable"});

        assert_eq!(
            reach(&[wifi("10.0.0.2"), bridge, veth], &[]),
            reach(&[wifi("10.0.0.2")], &[])
        );
    }

    #[test]
    fn a_vpn_joining_is_another_network() {
        let tunnel = json!({"Name": "tun0", "Kind": "tun", "OperationalState": "routable"});

        assert_ne!(
            reach(&[wifi("10.0.0.2"), tunnel], &[]),
            reach(&[wifi("10.0.0.2")], &[])
        );
    }

    #[test]
    fn a_new_privacy_address_is_the_same_network() {
        let privacy = |address: &str| {
            let mut link = wifi("10.0.0.2");
            let temporary =
                json!({"AddressString": address, "PrefixLength": 64, "Scope": 0, "Flags": 1});
            link["Addresses"].as_array_mut().unwrap().push(temporary);
            link
        };

        assert_eq!(
            reach(&[privacy("2001:db8::1")], &[]),
            reach(&[privacy("2001:db8::2")], &[])
        );
    }
}
