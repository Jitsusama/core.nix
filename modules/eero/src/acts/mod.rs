//! The things done to the home network rather than read of it: start
//! eero's own speed test, turn "Optimize for conferencing and gaming" on
//! or off, and keep an address for a device or stop keeping it. The CLI
//! does the first two itself (`eero speeds run`, `eero internet sqm on`);
//! the daemon does every one as a listener asks it (listeners.rs), so the
//! shell runs no program to change the home network, and `eero devices
//! <MAC> reserve` asks the daemon rather than opening the session. These
//! are the only acts: an ask naming anything else is refused. Beside them
//! are how a reservation is judged and checked (reservations.rs), the
//! log every act is written to (changes.rs), and the rules the daemon
//! carries out each by (carried.rs).

pub mod carried;
pub mod changes;
pub mod reservations;

use std::fmt;
use std::net::Ipv4Addr;

use serde_json::Value;

use crate::api;
use crate::cloud::{Asks, Failure};

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

/// What an act came to, once eero took it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    /// eero made the change.
    Made,
    /// It already stood as asked, so nothing was sent, and there's
    /// nothing to undo.
    Unchanged,
}

/// What an act came to, and for an unreservation the address eero kept
/// for the device before it was asked, which keeping again undoes it.
/// That's known before the change is sent, so it's said whatever eero
/// answers, an outcome no one knows included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Came {
    pub done: Result<Done, Failure>,
    pub held: Option<Ipv4Addr>,
}

impl Came {
    /// What an act that holds no address came to.
    pub fn of(done: Result<Done, Failure>) -> Came {
        Came { done, held: None }
    }
}

/// A device's hardware address, written as eero writes it: six pairs of
/// lowercase hex joined by colons, in the journal too, so a search for
/// the MAC the CLI and the change log show finds its lines there.
#[derive(Clone, Copy, PartialEq, Eq)]
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

impl fmt::Debug for Mac {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, out)
    }
}

impl Act {
    /// The act a wire ask names, as a row's act serves it: `{"ask": "act",
    /// "act": "speedtest"}` or `{"ask": "act", "act": "sqm", "on": true}`,
    /// or why it isn't one. Only the act's name is said back: nothing
    /// else the ask held.
    pub fn asked(said: &Value) -> Result<Act, Unasked> {
        let malformed = |why: &str| Unasked::Malformed(why.to_owned());
        match said["act"].as_str() {
            Some("speedtest") => Ok(Act::SpeedTest),
            Some("sqm") => said["on"]
                .as_bool()
                .map(Act::Conferencing)
                .ok_or_else(|| malformed("conferencing and gaming is asked on or off")),
            Some("reserve") => Ok(Act::Reserve {
                mac: mac(said).map_err(Unasked::Malformed)?,
                address: said["address"]
                    .as_str()
                    .and_then(|address| address.parse().ok())
                    .ok_or_else(|| malformed("a reservation names an IPv4 address"))?,
            }),
            Some("unreserve") => Ok(Act::Unreserve {
                mac: mac(said).map_err(Unasked::Malformed)?,
            }),
            Some(other) => Err(Unasked::Unknown(format!(
                "eero does nothing called {}",
                named(other)
            ))),
            None => Err(Unasked::Unknown("an act names what it does".to_owned())),
        }
    }
}

/// Why an ask is no act the daemon does, told apart since they mean
/// different things to whoever asked: an act it doesn't know, as a
/// daemon older than its asker wouldn't, or one it knows asked with
/// fields it can't read, which is the asker's own mistake.
#[derive(Debug, PartialEq, Eq)]
pub enum Unasked {
    Unknown(String),
    Malformed(String),
}

impl Unasked {
    /// The code the daemon answers and logs it by.
    pub fn code(&self) -> &'static str {
        match self {
            Unasked::Unknown(_) => "bad-act",
            Unasked::Malformed(_) => "bad-ask",
        }
    }

    /// Why, as said back: nothing of the ask but the act's name.
    pub fn why(&self) -> &str {
        match self {
            Unasked::Unknown(why) | Unasked::Malformed(why) => why,
        }
    }
}

/// An act's name as said back or logged: anyone may write one, so it's
/// cut short.
pub fn named(act: &str) -> String {
    act.chars().take(32).collect()
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
/// as gladly as one it takes; a reservation is too (reservations.rs).
pub fn done(eero: &mut impl Asks, network: &str, act: Act) -> Came {
    match act {
        Act::Reserve { mac, address } => {
            Came::of(reservations::reserve(eero, network, mac, address))
        }
        Act::Unreserve { mac } => reservations::unreserve(eero, network, mac),
        Act::SpeedTest => Came::of(
            eero.post(&format!("{network}/speedtest"), None)
                .map(|_| Done::Made),
        ),
        Act::Conferencing(on) => Came::of(conferencing(eero, network, on)),
    }
}

/// Turns conferencing and gaming on or off, and reads it back.
fn conferencing(eero: &mut impl Asks, network: &str, on: bool) -> Result<Done, Failure> {
    let named = api::network(network, &eero.get(network)?);
    let settings = named
        .settings_url
        .ok_or_else(|| Failure::Refused(format!("eero names no settings for {}", named.name)))?;
    eero.put(&settings, &api::conferencing(on))?;
    let now = api::network(network, &eero.get(network)?);
    api::taken(on, now.sqm).map_err(Failure::Refused)?;
    Ok(Done::Made)
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
            Err(Unasked::Malformed(
                "conferencing and gaming is asked on or off".to_owned()
            ))
        );
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": "reboot"})),
            Err(Unasked::Unknown(
                "eero does nothing called reboot".to_owned()
            ))
        );
        assert_eq!(
            [
                Unasked::Unknown(String::new()).code(),
                Unasked::Malformed(String::new()).code()
            ],
            ["bad-act", "bad-ask"],
            "an act asked wrongly was coded as one the daemon doesn't know, or the other way"
        );
        let long = "dns".repeat(40);
        assert_eq!(
            Act::asked(&json!({"ask": "act", "act": long})),
            Err(Unasked::Unknown(format!(
                "eero does nothing called {}",
                &long[..32]
            ))),
            "a long act's name was said back whole"
        );
    }

    fn mac(said: &str) -> Mac {
        Mac::parse(said).expect("a MAC the test means to be good")
    }

    fn address(said: &str) -> Ipv4Addr {
        said.parse().expect("an address the test means to be good")
    }

    #[test]
    fn a_mac_reads_in_either_case_and_writes_as_eero_does() {
        assert_eq!(
            Mac::parse("02:AB:00:00:00:3f").map(|mac| mac.to_string()),
            Some("02:ab:00:00:00:3f".to_owned()),
            "a good MAC didn't read, or wrote back otherwise"
        );
        assert_eq!(
            Mac::parse("02:ab:00:00:00:3f").map(|mac| format!("{mac:?}")),
            Some("02:ab:00:00:00:3f".to_owned()),
            "a MAC debugged otherwise than it writes, as the journal shows it"
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
                Err(Unasked::Malformed(
                    "a reservation names an IPv4 address".into()
                )),
                Err(Unasked::Malformed(
                    "a reservation names a device's MAC, such as 02:00:00:00:00:01".into()
                )),
                Err(Unasked::Malformed(
                    "a reservation names a device's MAC, such as 02:00:00:00:00:01".into()
                )),
            ],
            "a reservation's ask read wrongly"
        );
    }
}
