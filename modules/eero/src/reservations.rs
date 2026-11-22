//! Keeping an address for a device, and no longer keeping it: the two acts
//! that change which device DHCP gives which address, so a wrong one can
//! take a working device's address or cut it off mid-call.
//!
//! Each refuses rather than guesses. A reservation crossing one eero keeps
//! is refused, not replaced; so is an address a device or an eero holds
//! now, one outside the home subnet, and an unreservation where a port
//! forward reaches the address or eero keeps more than one for the device.
//! A list this can't read whole refuses too, since a reservation read as
//! none could be one the act crosses. Each change is judged by reading
//! eero's reservations after it rather than by eero's answer, whose shape
//! for a change was taken from an open-source client rather than seen;
//! where neither says, the outcome is unknown, never done or refused.

use std::net::Ipv4Addr;

use serde_json::{Value, json};

use crate::acts::{Done, Mac};
use crate::cloud::{Asks, Failure};

/// A reservation eero keeps: where it lives, for which device and which
/// address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub url: String,
    pub mac: Mac,
    pub address: Ipv4Addr,
}

/// The reservations in the data of `networks/<id>/reservations`, or why
/// they can't be read whole.
pub fn kept(data: &Value) -> Result<Vec<Kept>, String> {
    let unread = || "eero lists a reservation this can't read; see them in the eero app".to_owned();
    data.as_array()
        .ok_or_else(unread)?
        .iter()
        .map(|reservation| one_kept(reservation).ok_or_else(unread))
        .collect()
}

/// One reservation in eero's list, if it names where it lives, its
/// device and its address.
fn one_kept(reservation: &Value) -> Option<Kept> {
    Some(Kept {
        url: reservation["url"].as_str()?.to_owned(),
        mac: reservation["mac"].as_str().and_then(Mac::parse)?,
        address: reservation["ip"].as_str()?.parse().ok()?,
    })
}

/// What a reservation asks of eero, given what it keeps now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// It already stands as asked.
    Nothing,
    /// Add one.
    Add,
}

/// What keeping `address` for `mac` asks, or why it can't be done as it
/// stands: a device keeps one address and an address one device, so one
/// already kept otherwise is unreserved first, on purpose, rather than
/// replaced here.
pub fn to_reserve(kept: &[Kept], mac: Mac, address: Ipv4Addr) -> Result<Change, String> {
    if let Some(held) = kept.iter().find(|held| held.mac == mac) {
        return if held.address == address {
            Ok(Change::Nothing)
        } else {
            Err(format!(
                "{mac} already keeps {}; unreserve it first",
                held.address
            ))
        };
    }
    if let Some(held) = kept.iter().find(|held| held.address == address) {
        return Err(format!(
            "{address} is already kept for {}; unreserve that first",
            held.mac
        ));
    }
    Ok(Change::Add)
}

/// The reservation no longer keeping an address for `mac` takes away, if
/// any, or why it can't be told which.
pub fn to_unreserve(kept: &[Kept], mac: Mac) -> Result<Option<&Kept>, String> {
    let mut held = kept.iter().filter(|held| held.mac == mac);
    match (held.next(), held.next()) {
        (one, None) => Ok(one),
        (Some(_), Some(_)) => Err(format!(
            "eero keeps more than one address for {mac}; change them in the eero app"
        )),
        (None, Some(_)) => unreachable!("a second reservation came before a first"),
    }
}

/// Whether `address` is one the home subnet, as `subnets_config` names it,
/// gives to a device: inside it, and neither its own address nor its
/// broadcast.
pub fn within(subnets: &Value, address: Ipv4Addr) -> Result<(), String> {
    let home = subnets
        .as_array()
        .and_then(|subnets| {
            subnets
                .iter()
                .find(|subnet| subnet["subnet_kind"] == "main")
        })
        .map(|subnet| &subnet["dedicated_subnet"]);
    let part = |field: &str| {
        home.and_then(|home| home[field].as_str())
            .and_then(|part| part.parse::<Ipv4Addr>().ok())
            .map(u32::from)
    };
    let (Some(base), Some(mask)) = (part("subnet_ip"), part("subnet_mask")) else {
        return Err("eero names no home subnet to check the address against".into());
    };
    let asked = u32::from(address);
    let network = base & mask;
    let broadcast = network | !mask;
    if asked & mask != network || asked == network || asked == broadcast {
        return Err(format!(
            "{address} isn't one the home subnet gives a device"
        ));
    }
    Ok(())
}

/// Who holds `address` now other than `mac`, among the devices and the
/// eeros, if anyone: keeping it for `mac` would take it from them.
pub fn holder(devices: &Value, eeros: &Value, mac: Mac, address: Ipv4Addr) -> Option<String> {
    let said = address.to_string();
    let none = Vec::new();
    let device = devices.as_array().unwrap_or(&none).iter().find(|device| {
        let holds = device["ip"] == said.as_str()
            || device["ips"]
                .as_array()
                .is_some_and(|ips| ips.iter().any(|ip| ip == said.as_str()));
        holds && device["mac"].as_str().and_then(Mac::parse) != Some(mac)
    });
    if let Some(device) = device {
        let named = device["nickname"]
            .as_str()
            .or_else(|| device["hostname"].as_str())
            .or_else(|| device["mac"].as_str())
            .unwrap_or("another device");
        return Some(format!("held by {named}"));
    }
    let eero = eeros
        .as_array()
        .unwrap_or(&none)
        .iter()
        .any(|eero| eero["ip_address"] == said.as_str());
    eero.then(|| "an eero's own".to_owned())
}

/// Whether anything in eero's port forwards names `address`: a forward
/// reaches a device by its reserved address, so taking the reservation
/// away could leave the forward reaching whichever device DHCP gives the
/// address next. The whole answer is searched, since no forward has been
/// seen to know its shape.
pub fn forwarded(forwards: &Value, address: Ipv4Addr) -> bool {
    let said = address.to_string();
    match forwards {
        Value::String(text) => *text == said,
        Value::Array(items) => items.iter().any(|item| forwarded(item, address)),
        Value::Object(fields) => fields.values().any(|field| forwarded(field, address)),
        _ => false,
    }
}

/// Whether `url` is a reservation of `network`'s, the only kind of thing
/// an unreservation may DELETE: it comes from eero's list, and anything
/// else under eero's origin, another host's included, is refused.
pub fn of_network(network: &str, url: &str) -> bool {
    url.strip_prefix(&format!("{network}/reservations/"))
        .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_alphanumeric()))
}

/// Keeps `address` for `mac` on `network`, such as `/2.2/networks/1001`.
pub fn reserve(
    eero: &mut impl Asks,
    network: &str,
    mac: Mac,
    address: Ipv4Addr,
) -> Result<Done, Failure> {
    let reservations = format!("{network}/reservations");
    let before = kept(&eero.get(&reservations)?).map_err(Failure::Refused)?;
    if to_reserve(&before, mac, address).map_err(Failure::Refused)? == Change::Nothing {
        return Ok(Done::Unchanged);
    }
    within(&eero.get(&format!("{network}/subnets_config"))?, address).map_err(Failure::Refused)?;
    let devices = eero.get(&format!("{network}/devices"))?;
    let eeros = eero.get(&format!("{network}/eeros"))?;
    if let Some(holder) = holder(&devices, &eeros, mac, address) {
        return Err(Failure::Refused(format!(
            "{address} is {holder} now; reserve one no device holds"
        )));
    }
    let asked = json!({"mac": mac.to_string(), "ip": address.to_string(),
        "description": mac.to_string()});
    let sent = eero.post(&reservations, Some(&asked));
    settled(eero, &reservations, sent, mac, |kept| {
        to_reserve(kept, mac, address) == Ok(Change::Nothing)
    })
    .map(|()| Done::Made)
}

/// No longer keeps an address for `mac` on `network`.
pub fn unreserve(eero: &mut impl Asks, network: &str, mac: Mac) -> Result<Done, Failure> {
    let reservations = format!("{network}/reservations");
    let before = kept(&eero.get(&reservations)?).map_err(Failure::Refused)?;
    let Some(held) = to_unreserve(&before, mac)
        .map_err(Failure::Refused)?
        .cloned()
    else {
        return Ok(Done::Unchanged);
    };
    if !of_network(network, &held.url) {
        return Err(Failure::Refused(format!(
            "eero names {mac}'s reservation oddly; change it in the eero app"
        )));
    }
    if forwarded(&eero.get(&format!("{network}/forwards"))?, held.address) {
        return Err(Failure::Refused(format!(
            "a port forward reaches {}; change it in the eero app first",
            held.address
        )));
    }
    let sent = eero.delete(&held.url);
    settled(eero, &reservations, sent, mac, |kept| {
        kept.iter().all(|kept| kept.mac != mac)
    })
    .map(|()| Done::Freed(held.address))
}

/// What a change eero was sent came to, judged by its reservations read
/// after it (`stands`) rather than by its answer. A session refused was
/// never taken, so it's said as it is; otherwise what eero keeps decides,
/// and where it can't be read the outcome is unknown.
fn settled(
    eero: &mut impl Asks,
    reservations: &str,
    sent: Result<Value, Failure>,
    mac: Mac,
    stands: impl Fn(&[Kept]) -> bool,
) -> Result<(), Failure> {
    if let Err(failure @ Failure::SignedOut(_)) = sent {
        return Err(failure);
    }
    let after = eero
        .get(reservations)
        .ok()
        .and_then(|after| kept(&after).ok());
    match (sent, after) {
        (_, Some(kept)) if stands(&kept) => Ok(()),
        (Err(failure), Some(_)) => Err(failure),
        (Ok(_), Some(_)) => Err(Failure::Refused(
            "eero answered the change, but doesn't keep it".into(),
        )),
        (_, None) => Err(Failure::Unknown(format!(
            "eero didn't say whether it took the change; read eero devices {mac} to see"
        ))),
    }
}

#[cfg(test)]
#[path = "reservations_tests.rs"]
mod tests;
