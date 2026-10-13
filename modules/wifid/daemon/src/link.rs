//! What networkd says of a Wi-Fi card's link, from its Describe JSON: the
//! network it's on, whether it can reach beyond it, and the network's own
//! DNS servers, which the check asks rather than whatever resolved would.

use std::net::IpAddr;

use serde_json::Value;

/// A Wi-Fi card's link, as networkd sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub index: i32,
    pub name: String,
    /// The network the card is on, once it has joined one.
    pub ssid: Option<String>,
    /// Whether it has an address and a route beyond the network.
    pub routable: bool,
    /// The DNS servers the network itself handed out.
    pub dns: Vec<IpAddr>,
    /// The sign-in API the network advertises, under RFC 8910.
    pub portal: Option<String>,
}

impl Link {
    /// Reads one link's description, passing over any that isn't a Wi-Fi
    /// card joining networks.
    pub fn described(link: &Value) -> Option<Link> {
        let joining = link["Type"] == "wlan" && link["WirelessLanInterfaceTypeString"] == "station";
        // A card udev is still naming may yet change its name, and one in
        // linger is already gone.
        let settled =
            !["pending", "linger"].contains(&link["AdministrativeState"].as_str().unwrap_or(""));
        if !joining || !settled {
            return None;
        }
        Some(Link {
            index: i32::try_from(link["Index"].as_i64()?).ok()?,
            name: link["Name"].as_str()?.to_string(),
            ssid: text(&link["SSID"]),
            routable: link["OperationalState"] == "routable",
            dns: dns(link),
            portal: text(&link["CaptivePortal"]),
        })
    }
}

/// Whether a cable reaches beyond the machine: a link of a real Ethernet
/// card, not a container's or a bridge's, that's routable.
pub fn wired(links: &[Value]) -> bool {
    links.iter().any(|link| {
        link["Type"] == "ether" && link["Kind"].is_null() && link["OperationalState"] == "routable"
    })
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// The servers in the network's DHCPv4 lease, which stay there when a VPN
/// sets the link's own, as WARP does; without a lease, the link's servers
/// that weren't set at runtime.
fn dns(link: &Value) -> Vec<IpAddr> {
    let options = link["DHCPv4Client"]["Lease"]["Message"]["options"].as_array();
    let leased = options
        .into_iter()
        .flatten()
        .filter(|option| option["tag"] == DNS_OPTION);
    let leased: Vec<IpAddr> = leased
        .filter_map(|option| option["data"].as_str())
        .flat_map(addresses)
        .collect();
    if !leased.is_empty() {
        return leased;
    }
    let listed = link["DNS"].as_array().into_iter().flatten();
    listed
        .filter(|server| server["ConfigSource"] != "runtime")
        .filter_map(|server| server["AddressString"].as_str()?.parse().ok())
        .collect()
}

/// DHCP's option for DNS servers: IPv4 addresses, four bytes each.
const DNS_OPTION: u8 = 6;

/// The addresses in an option's data, written as hex.
fn addresses(hex: &str) -> Vec<IpAddr> {
    let bytes: Option<Vec<u8>> = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok())
        .collect();
    let bytes = bytes.unwrap_or_default();
    let (octets, _) = bytes.as_chunks::<4>();
    octets.iter().map(|octets| IpAddr::from(*octets)).collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn cable(state: &str) -> Value {
        json!({"Index": 2, "Name": "eth0", "Type": "ether", "OperationalState": state})
    }

    #[test]
    fn a_cable_reaching_beyond_the_machine_is_wired() {
        let links = [card(), cable("routable")];

        assert!(wired(&links));
    }

    #[test]
    fn a_cable_plugged_into_nothing_isn_t() {
        let links = [card(), cable("no-carrier")];

        assert!(!wired(&links));
    }

    #[test]
    fn a_container_s_link_isn_t_a_cable() {
        let mut veth = cable("routable");
        veth["Kind"] = json!("veth");

        assert!(!wired(&[veth]));
    }

    fn card() -> Value {
        json!({
            "Index": 3,
            "Name": "wlan2",
            "Type": "wlan",
            "WirelessLanInterfaceTypeString": "station",
            "SSID": "Roadside Inn",
            "OperationalState": "routable",
            "DNS": [{
                "Family": 2,
                "Address": [10, 43, 0, 1],
                "AddressString": "10.43.0.1",
                "ConfigSource": "DHCPv4",
                "ConfigProvider": [10, 43, 0, 1],
                "ConfigProviderString": "10.43.0.1",
            }],
            "CaptivePortal": "https://front-desk.inn/api",
            "DHCPv4Client": {
                "Lease": {
                    "Message": {
                        "options": [
                            {"tag": 1, "data": "ffffff00"},
                            {"tag": 6, "data": "0a2b0001"},
                        ],
                    },
                },
            },
        })
    }

    #[test]
    fn a_wifi_card_s_link_is_read_with_its_network_and_how_far_it_reaches() {
        let link = Link::described(&card());

        assert_eq!(
            link,
            Some(Link {
                index: 3,
                name: "wlan2".to_string(),
                ssid: Some("Roadside Inn".to_string()),
                routable: true,
                dns: vec!["10.43.0.1".parse().unwrap()],
                portal: Some("https://front-desk.inn/api".to_string()),
            })
        );
    }

    #[test]
    fn a_card_on_no_network_yet_has_none_of_one_s_details() {
        let link = json!({
            "Index": 3,
            "Name": "wlan2",
            "Type": "wlan",
            "WirelessLanInterfaceTypeString": "station",
            "OperationalState": "no-carrier",
        });

        let link = Link::described(&link).unwrap();

        assert_eq!(link.ssid, None);
        assert!(!link.routable);
        assert_eq!(link.dns, Vec::<IpAddr>::new());
        assert_eq!(link.portal, None);
    }

    #[test]
    fn the_network_s_servers_come_from_its_lease_when_another_program_set_the_link_s() {
        let mut described = card();
        described["DNS"] = json!([{
            "Family": 2,
            "AddressString": "172.64.36.1",
            "ConfigSource": "runtime",
        }]);
        described["DHCPv4Client"]["Lease"]["Message"]["options"][1]["data"] =
            json!("0a2b00010a2b0002");

        let link = Link::described(&described).unwrap();

        let servers: Vec<IpAddr> = vec!["10.43.0.1".parse().unwrap(), "10.43.0.2".parse().unwrap()];
        assert_eq!(link.dns, servers);
    }

    #[test]
    fn without_a_lease_the_link_s_servers_are_the_network_s_unless_set_at_runtime() {
        let mut described = card();
        described.as_object_mut().unwrap().remove("DHCPv4Client");
        described["DNS"] = json!([
            {"Family": 10, "AddressString": "fe80::1", "ConfigSource": "NDisc"},
            {"Family": 2, "AddressString": "172.64.36.1", "ConfigSource": "runtime"},
        ]);

        let link = Link::described(&described).unwrap();

        assert_eq!(link.dns, vec!["fe80::1".parse::<IpAddr>().unwrap()]);
    }

    #[test]
    fn links_that_arent_a_card_joining_networks_are_passed_over() {
        let cable = json!({"Index": 2, "Name": "enp0s31f6", "Type": "ether"});
        let mut access_point = card();
        access_point["WirelessLanInterfaceTypeString"] = json!("ap");

        assert_eq!(Link::described(&cable), None);
        assert_eq!(Link::described(&access_point), None);
    }

    #[test]
    fn a_card_still_being_named_or_going_away_is_passed_over_too() {
        let mut arriving = card();
        arriving["AdministrativeState"] = json!("pending");
        let mut leaving = card();
        leaving["AdministrativeState"] = json!("linger");

        assert_eq!(Link::described(&arriving), None);
        assert_eq!(Link::described(&leaving), None);
    }
}
