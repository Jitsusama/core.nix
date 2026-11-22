//! The things done to the home network rather than read of it: start
//! eero's own speed test, turn "Optimize for conferencing and gaming" on
//! or off, and keep an address for a device or stop keeping it. The CLI
//! does the first two itself (`eero speedtest`, `eero sqm on|off`); the
//! daemon does every one as a listener asks it (listeners.rs), so the
//! shell runs no program to change the home network, and `eero reserve`
//! asks the daemon rather than opening the session. These are the only
//! acts: an ask naming anything else is refused.

use std::fmt;
use std::net::Ipv4Addr;

use serde_json::{Value, json};

use crate::api;
use crate::cloud::{Cloud, Failure};

/// What's asked of the home network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Start eero's own speed test, which it takes most of a minute over.
    SpeedTest,
    /// Turn conferencing and gaming's queueing on, or off.
    Conferencing(bool),
    /// Keep `address` for the device `mac`, so DHCP always gives it that.
    Reserve { mac: Mac, address: Ipv4Addr },
    /// Stop keeping an address for the device `mac`.
    Unreserve { mac: Mac },
}

/// A device's hardware address, written as eero writes it: six pairs of
/// lowercase hex joined by colons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mac([u8; 6]);

impl Mac {
    /// The address `said` names, in either case, or none.
    pub fn parse(said: &str) -> Option<Mac> {
        let mut bytes = [0; 6];
        let mut pairs = said.split(':');
        for byte in &mut bytes {
            let pair = pairs.next()?;
            if pair.len() != 2 || !pair.bytes().all(|digit| digit.is_ascii_hexdigit()) {
                return None;
            }
            *byte = u8::from_str_radix(pair, 16).ok()?;
        }
        pairs.next().is_none().then_some(Mac(bytes))
    }
}

impl fmt::Display for Mac {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        let pairs: Vec<String> = self.0.iter().map(|byte| format!("{byte:02x}")).collect();
        out.write_str(&pairs.join(":"))
    }
}

/// A reservation eero keeps: where it lives, for which device and which
/// address. A field eero writes oddly is none, never a guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub url: String,
    pub mac: Option<Mac>,
    pub address: Option<Ipv4Addr>,
}

/// The reservations in the data of `networks/<id>/reservations`.
pub fn kept(data: &Value) -> Vec<Kept> {
    data.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|reservation| {
            Some(Kept {
                url: reservation["url"].as_str()?.to_owned(),
                mac: reservation["mac"].as_str().and_then(Mac::parse),
                address: reservation["ip"].as_str().and_then(|ip| ip.parse().ok()),
            })
        })
        .collect()
}

/// What a reservation asks of eero, given what it keeps now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// It already stands as asked.
    Nothing,
    /// Add one.
    Add,
    /// Take away the one at this url.
    Remove(String),
}

/// What keeping `address` for `mac` asks, or why it can't be done as it
/// stands: a device keeps one address and an address one device, so one
/// already kept otherwise is unreserved first, on purpose, rather than
/// replaced here.
pub fn to_reserve(kept: &[Kept], mac: Mac, address: Ipv4Addr) -> Result<Change, String> {
    if let Some(held) = kept.iter().find(|held| held.mac == Some(mac)) {
        return if held.address == Some(address) {
            Ok(Change::Nothing)
        } else {
            Err(format!(
                "{mac} already keeps {}; unreserve it first",
                held.address
                    .map_or_else(|| "another address".to_owned(), |a| a.to_string())
            ))
        };
    }
    if let Some(held) = kept.iter().find(|held| held.address == Some(address)) {
        return Err(format!(
            "{address} is already kept for {}; unreserve that first",
            held.mac
                .map_or_else(|| "another device".to_owned(), |m| m.to_string())
        ));
    }
    Ok(Change::Add)
}

/// What no longer keeping an address for `mac` asks.
pub fn to_unreserve(kept: &[Kept], mac: Mac) -> Change {
    kept.iter()
        .find(|held| held.mac == Some(mac))
        .map_or(Change::Nothing, |held| Change::Remove(held.url.clone()))
}

impl Act {
    /// The act a wire ask names, as a row's act serves it: `{"ask": "act",
    /// "act": "speedtest"}` or `{"ask": "act", "act": "sqm", "on": true}`,
    /// or why it isn't one.
    pub fn asked(said: &Value) -> Result<Act, String> {
        match said["act"].as_str() {
            Some("speedtest") => Ok(Act::SpeedTest),
            Some("sqm") => said["on"]
                .as_bool()
                .map(Act::Conferencing)
                .ok_or_else(|| "conferencing and gaming is asked on or off".to_owned()),
            Some("reserve") => Ok(Act::Reserve {
                mac: mac(said)?,
                address: said["address"]
                    .as_str()
                    .and_then(|address| address.parse().ok())
                    .ok_or_else(|| "a reservation names an IPv4 address".to_owned())?,
            }),
            Some("unreserve") => Ok(Act::Unreserve { mac: mac(said)? }),
            Some(other) => Err(format!("eero does nothing called {other}")),
            None => Err("an act names what it does".to_owned()),
        }
    }
}

/// The device an ask's `mac` names, or why it names none.
fn mac(said: &Value) -> Result<Mac, String> {
    said["mac"]
        .as_str()
        .and_then(Mac::parse)
        .ok_or_else(|| "a reservation names a device's MAC, such as 02:00:00:00:00:01".to_owned())
}

/// Does `act` to the network at `network`, such as `/2.2/networks/1001`.
/// Conferencing is read back after, since eero answers a change it ignores
/// as gladly as one it takes.
pub fn done(cloud: &mut Cloud, network: &str, act: Act) -> Result<(), Failure> {
    match act {
        Act::SpeedTest => {
            cloud.post(&format!("{network}/speedtest"), None)?;
        }
        Act::Conferencing(on) => {
            let named = api::network(network, &cloud.get(network)?);
            let settings = named.settings_url.ok_or_else(|| {
                Failure::Refused(format!("eero names no settings for {}", named.name))
            })?;
            cloud.put(&settings, &api::conferencing(on))?;
            let now = api::network(network, &cloud.get(network)?);
            api::taken(on, now.sqm).map_err(Failure::Refused)?;
        }
        Act::Reserve { mac, address } => {
            let reservations = format!("{network}/reservations");
            let held = kept(&cloud.get(&reservations)?);
            if to_reserve(&held, mac, address).map_err(Failure::Refused)? == Change::Add {
                let asked = json!({"mac": mac.to_string(), "ip": address.to_string(),
                    "description": mac.to_string()});
                cloud.post(&reservations, Some(&asked))?;
            }
            if to_reserve(&kept(&cloud.get(&reservations)?), mac, address) != Ok(Change::Nothing) {
                return Err(Failure::Refused(format!(
                    "eero took the reservation of {address} for {mac} but doesn't keep it"
                )));
            }
        }
        Act::Unreserve { mac } => {
            let reservations = format!("{network}/reservations");
            if let Change::Remove(url) = to_unreserve(&kept(&cloud.get(&reservations)?), mac) {
                cloud.delete(&url)?;
            }
            if to_unreserve(&kept(&cloud.get(&reservations)?), mac) != Change::Nothing {
                return Err(Failure::Refused(format!(
                    "eero took the unreservation of {mac} but still keeps its address"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_rows_ask_names_the_act() {
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "speedtest"})),
            Ok(Act::SpeedTest)
        );
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "sqm", "on": false})),
            Ok(Act::Conferencing(false))
        );
    }

    #[test]
    fn an_ask_eero_has_no_act_for_is_refused() {
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "sqm"})),
            Err("conferencing and gaming is asked on or off".to_owned())
        );
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "reboot"})),
            Err("eero does nothing called reboot".to_owned())
        );
    }

    fn mac(said: &str) -> Mac {
        Mac::parse(said).expect("a MAC the test means to be good")
    }

    fn address(said: &str) -> Ipv4Addr {
        said.parse().expect("an address the test means to be good")
    }

    /// The reservations eero answered on the contract's network.
    fn contract() -> Vec<Kept> {
        let path = format!(
            "{}/contract/net.reservations.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let body: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("the contract's file"))
                .expect("the contract's JSON");
        kept(&body["data"])
    }

    #[test]
    fn a_mac_reads_in_either_case_and_writes_as_eero_does() {
        assert_eq!(
            Mac::parse("02:AB:00:00:00:3f").map(|mac| mac.to_string()),
            Some("02:ab:00:00:00:3f".to_owned()),
            "a good MAC didn't read, or wrote back otherwise"
        );
        for bad in [
            "",
            "02:00:00:00:00",
            "02:00:00:00:00:01:02",
            "02:00:00:00:00:1",
            "02:00:00:00:00:+f",
            "02-00-00-00-00-01",
            "02:00:00:00:00:01:",
        ] {
            assert_eq!(Mac::parse(bad), None, "{bad:?} read as a MAC");
        }
    }

    #[test]
    fn the_contracts_reservation_is_kept() {
        assert_eq!(
            contract(),
            vec![Kept {
                url: "/2.2/networks/1001/reservations/2c8db13091a2".into(),
                mac: Some(mac("02:00:00:00:00:38")),
                address: Some(address("10.0.0.208")),
            }],
            "eero's reservation didn't read as it answered it"
        );
    }

    #[test]
    fn a_reservation_is_added_only_where_it_crosses_none() {
        let kept = contract();
        let held = mac("02:00:00:00:00:38");
        let other = mac("02:00:00:00:00:01");
        assert_eq!(
            [
                to_reserve(&kept, held, address("10.0.0.208")),
                to_reserve(&kept, other, address("10.0.0.31")),
                to_reserve(&kept, held, address("10.0.0.31")),
                to_reserve(&kept, other, address("10.0.0.208")),
            ],
            [
                Ok(Change::Nothing),
                Ok(Change::Add),
                Err("02:00:00:00:00:38 already keeps 10.0.0.208; unreserve it first".into()),
                Err(
                    "10.0.0.208 is already kept for 02:00:00:00:00:38; unreserve that first".into()
                ),
            ],
            "a reservation replaced one, or wasn't added where it crossed none"
        );
    }

    #[test]
    fn an_unreservation_removes_only_the_devices_own() {
        let kept = contract();
        assert_eq!(
            [
                to_unreserve(&kept, mac("02:00:00:00:00:38")),
                to_unreserve(&kept, mac("02:00:00:00:00:01")),
            ],
            [
                Change::Remove("/2.2/networks/1001/reservations/2c8db13091a2".into()),
                Change::Nothing,
            ],
            "an unreservation took the wrong reservation, or one that wasn't there"
        );
    }

    #[test]
    fn a_reservations_ask_names_its_device_and_address() {
        assert_eq!(
            [
                Act::asked(&json!({"act": "reserve", "mac": "02:00:00:00:00:01",
                    "address": "10.0.0.31"})),
                Act::asked(&json!({"act": "unreserve", "mac": "02:00:00:00:00:01"})),
                Act::asked(&json!({"act": "reserve", "mac": "02:00:00:00:00:01",
                    "address": "10.0.0"})),
                Act::asked(&json!({"act": "reserve", "mac": "a phone", "address": "10.0.0.31"})),
                Act::asked(&json!({"act": "unreserve"})),
            ],
            [
                Ok(Act::Reserve {
                    mac: mac("02:00:00:00:00:01"),
                    address: address("10.0.0.31"),
                }),
                Ok(Act::Unreserve {
                    mac: mac("02:00:00:00:00:01"),
                }),
                Err("a reservation names an IPv4 address".into()),
                Err("a reservation names a device's MAC, such as 02:00:00:00:00:01".into()),
                Err("a reservation names a device's MAC, such as 02:00:00:00:00:01".into()),
            ],
            "a reservation's ask read wrongly"
        );
    }
}
