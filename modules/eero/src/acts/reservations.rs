//! Keeping an address for a device, and no longer keeping it: the two acts
//! that change which device DHCP gives which address, so a wrong one can
//! take a working device's address or cut it off mid-call.
//!
//! Each refuses rather than guesses. A reservation crossing one eero keeps
//! is refused, not replaced; so is an address a device or an eero holds
//! now, one outside the range the home subnet gives out, and an
//! unreservation where a port forward may reach the address or eero keeps
//! more than one for the device. A list this can't read whole refuses
//! too, since a reservation, device or forward read as none could be one
//! the act crosses. Each change is judged by reading eero's reservations
//! after it rather than by eero's answer, whose shape for a change was
//! taken from an open-source client rather than seen; only eero's own
//! refusal of a change it then doesn't keep is said refused, and where
//! neither says, the outcome is unknown, never done or refused.

use std::net::Ipv4Addr;

use serde_json::{Value, json};

use crate::acts::{Came, Done, Mac};
use crate::cloud::{Asks, Failure};

/// A reservation eero keeps: where it lives, for which device and which
/// address, and the url eero names that device by, if it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    pub url: String,
    pub mac: Mac,
    pub address: Ipv4Addr,
    pub device: Option<String>,
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
        device: reservation["device"].as_str().map(str::to_owned),
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
/// gives to a device: inside it, neither its own address nor its
/// broadcast, and inside the range DHCP gives out. One below or above that
/// range is where a device set up by hand keeps its own address, which eero
/// may not list, so the holder check can't see it.
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
    let (Some(base), Some(mask), Some(start), Some(end)) = (
        part("subnet_ip"),
        part("subnet_mask"),
        part("start_ip"),
        part("end_ip"),
    ) else {
        return Err("eero names no home subnet and range to check the address against".into());
    };
    let asked = u32::from(address);
    let network = base & mask;
    let broadcast = network | !mask;
    if asked & mask != network || asked == network || asked == broadcast {
        return Err(format!(
            "{address} isn't one the home subnet gives a device"
        ));
    }
    if !(start..=end).contains(&asked) {
        return Err(format!(
            "{address} is outside the range eero gives out, {} to {}",
            Ipv4Addr::from(start),
            Ipv4Addr::from(end)
        ));
    }
    Ok(())
}

/// Who holds `address` now other than `mac`, among the devices and the
/// eeros, if anyone: keeping it for `mac` would take it from them. A list
/// of either that isn't one can't say no one does, so it's refused.
pub fn holder(
    devices: &Value,
    eeros: &Value,
    mac: Mac,
    address: Ipv4Addr,
) -> Result<Option<String>, String> {
    fn listed<'a>(answer: &'a Value, what: &str) -> Result<&'a Vec<Value>, String> {
        answer
            .as_array()
            .ok_or_else(|| format!("eero lists its {what} in a way this can't read"))
    }
    let said = address.to_string();
    let devices = listed(devices, "devices")?;
    let eeros = listed(eeros, "eeros")?;
    let device = devices.iter().find(|device| {
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
        return Ok(Some(format!("held by {named}")));
    }
    let eero = eeros.iter().any(|eero| eero["ip_address"] == said.as_str());
    Ok(eero.then(|| "an eero's own".to_owned()))
}

/// Whether anything in eero's port forwards may reach `held`: a forward
/// reaches a device by its reserved address, so taking the reservation
/// away could leave the forward reaching whichever device DHCP gives the
/// address next. No forward has been seen to know its shape, so the whole
/// answer is searched for anything naming the reservation anywhere in a
/// string or a number: its address, dotted or as one number, however it's
/// written around (with a port, a mask, in a range or after a word); its
/// id or its device's, as eero's urls end, in any url or alone; or the
/// device's MAC, in either case, with any separator or none. A match
/// that wasn't meant only costs a refusal, so it's matched loosely. A
/// list that isn't one can't say none does, so it's refused.
pub fn forwarded(forwards: &Value, held: &Kept) -> Result<bool, String> {
    if !forwards.is_array() {
        return Err("eero lists its port forwards in a way this can't read".into());
    }
    let address = held.address.to_string();
    let number = u32::from(held.address).to_string();
    let last = |url: &str| {
        url.rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
    };
    let ids: Vec<String> = [Some(held.url.as_str()), held.device.as_deref()]
        .into_iter()
        .flatten()
        .map(last)
        .filter(|id| !id.is_empty())
        .collect();
    let bare = held.mac.to_string().replace(':', "");
    let names = |text: &str| {
        let text = text.to_ascii_lowercase();
        let unseparated: String = text
            .chars()
            .filter(|c| !matches!(c, ':' | '-' | '.'))
            .collect();
        words(&text, |c| c.is_ascii_digit() || c == '.')
            .map(|word| word.trim_matches('.'))
            .any(|word| word == address || word == number)
            || words(&text, char::is_alphanumeric).any(|word| ids.iter().any(|id| id == word))
            || words(&unseparated, char::is_alphanumeric).any(|word| word == bare)
    };
    Ok(reaches(forwards, &names))
}

/// The runs of `text` made only of what `within` takes.
fn words(text: &str, within: impl Fn(char) -> bool) -> impl Iterator<Item = &str> {
    text.split(move |c| !within(c))
        .filter(|word| !word.is_empty())
}

/// Whether anything in `answer`, a string or a number as it's written,
/// is one `names` says names the reservation.
fn reaches(answer: &Value, names: &impl Fn(&str) -> bool) -> bool {
    match answer {
        Value::String(text) => names(text),
        Value::Number(number) => names(&number.to_string()),
        Value::Array(items) => items.iter().any(|item| reaches(item, names)),
        Value::Object(fields) => fields.values().any(|field| reaches(field, names)),
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
    if let Some(holder) = holder(&devices, &eeros, mac, address).map_err(Failure::Refused)? {
        return Err(Failure::Refused(format!(
            "{address} is {holder} now; reserve one no device holds"
        )));
    }
    let asked = json!({"mac": mac.to_string(), "ip": address.to_string(),
        "description": mac.to_string()});
    let sent = eero.post(&reservations, Some(&asked));
    let unknown = format!("read eero devices {mac} to see");
    settled(eero, &reservations, sent, &unknown, |kept| {
        to_reserve(kept, mac, address) == Ok(Change::Nothing)
    })
    .map(|()| Done::Made)
}

/// No longer keeps an address for `mac` on `network`, saying the address
/// it kept once it's found, whatever eero answers after.
pub fn unreserve(eero: &mut impl Asks, network: &str, mac: Mac) -> Came {
    let held = match found(eero, network, mac) {
        Ok(Some(held)) => held,
        Ok(None) => return Came::of(Ok(Done::Unchanged)),
        Err(failure) => return Came::of(Err(failure)),
    };
    let reservations = format!("{network}/reservations");
    // Its undo is to keep the address again, which `reserve` refuses
    // outside the range eero gives out, so one there is left to the app
    // rather than taken away with an undo that couldn't be done.
    let undoable = eero
        .get(&format!("{network}/subnets_config"))
        .and_then(|subnets| {
            within(&subnets, held.address).map_err(|why| {
                Failure::Refused(format!(
                    "{why}, so keeping it again couldn't undo this; change it in the eero app"
                ))
            })
        });
    if let Err(failure) = undoable {
        return Came::of(Err(failure));
    }
    let checked = eero
        .get(&format!("{network}/forwards"))
        .and_then(|forwards| forwarded(&forwards, &held).map_err(Failure::Refused));
    let done = match checked {
        Ok(true) => Err(Failure::Refused(format!(
            "a port forward may reach {}; change it in the eero app first",
            held.address
        ))),
        Ok(false) => {
            let sent = eero.delete(&held.url);
            let unknown = format!(
                "it kept {address} for {mac}, so if that's gone, eero devices {mac} reserve \
                 {address} keeps it again",
                address = held.address
            );
            settled(eero, &reservations, sent, &unknown, |kept| {
                kept.iter().all(|kept| kept.mac != mac)
            })
            .map(|()| Done::Made)
        }
        Err(failure) => Err(failure),
    };
    Came {
        done,
        held: Some(held.address),
    }
}

/// The one reservation eero keeps for `mac`, if any, once it's known to
/// be the network's to delete.
fn found(eero: &mut impl Asks, network: &str, mac: Mac) -> Result<Option<Kept>, Failure> {
    let before = kept(&eero.get(&format!("{network}/reservations"))?).map_err(Failure::Refused)?;
    let Some(held) = to_unreserve(&before, mac).map_err(Failure::Refused)? else {
        return Ok(None);
    };
    if !of_network(network, &held.url) {
        return Err(Failure::Refused(format!(
            "eero names {mac}'s reservation oddly; change it in the eero app"
        )));
    }
    Ok(Some(held.clone()))
}

/// What a change eero was sent came to, judged by its reservations read
/// after it (`stands`) rather than by its answer. A session refused was
/// never taken, so it's said as it is, and a change eero refused that it
/// then doesn't keep is refused. Otherwise only what eero keeps decides: a
/// send that timed out may still land after the read, and eero's list may
/// lag its answer, so a change that doesn't show yet is unknown, said with
/// `unknown`, how to find out.
fn settled(
    eero: &mut impl Asks,
    reservations: &str,
    sent: Result<Value, Failure>,
    unknown: &str,
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
        (Err(failure @ Failure::Refused(_)), Some(_)) => Err(failure),
        (_, Some(_)) => Err(Failure::Unknown(format!(
            "eero doesn't show the change yet, though it may still take it; {unknown}"
        ))),
        (_, None) => Err(Failure::Unknown(format!(
            "eero didn't say whether it took the change; {unknown}"
        ))),
    }
}

#[cfg(test)]
#[path = "reservations_tests.rs"]
mod tests;
