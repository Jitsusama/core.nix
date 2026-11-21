//! An access point asking a card to move to another, by an 802.11v BSS
//! transition request, and what wifid does with one.
//!
//! The eero sends them now and then, 27 to the laptop from 15 to 20 Nov
//! 2026, each naming the points it would rather the card were on, none
//! warning that it would drop the card. wpa_supplicant answers each
//! itself, and says it over D-Bus as `BSSTransitionRequested` only by
//! core.nix's patch, since upstream tells nobody on the bus what was
//! asked.
//!
//! One that doesn't warn is only said: wifid chooses the card's point by
//! how each carries, and the eero judges by what it hears of the card.
//! One that warns is followed, since the point drops the card when its
//! time is up whatever wifid thinks of it. wpa_supplicant can't follow it
//! for a card wifid holds to a point: only that point matches what it
//! may join, so it keeps the card there until it's dropped.

use std::collections::HashMap;
use std::fmt::Write as _;

use zbus::zvariant::{OwnedValue, Value};

/// One request, as wpa_supplicant said it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// Whether the point warns it will drop the card, from itself or from
    /// its whole network.
    pub warns: bool,
    /// In how many milliseconds, where it warns; 0 for at once, or for
    /// one that doesn't say.
    pub in_ms: u32,
    /// How long its candidates hold, in milliseconds.
    pub valid_ms: u32,
    /// The points it names, in the order it gave them.
    pub candidates: Vec<Candidate>,
}

/// A point the request names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Its own address, as 02:00:00:00:01:00.
    pub bssid: String,
    /// How much the point asking would rather the card were there, from 1
    /// to 255; 0 rules it out, and `None` is no preference given.
    pub preference: Option<u8>,
}

/// What the signal says: whether it warns, in how long and for how long in
/// milliseconds, and each candidate's address and preference, -1 for none.
type Said = (bool, u32, u32, Vec<(Vec<u8>, i16)>);

/// What wifid does with a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heed {
    /// Nothing but say it: the request doesn't warn.
    Stay,
    /// Moves the card to this point, the one it would rather of those the
    /// card hears.
    Move(String),
    /// Lets the card join any point of its network, as it hears none the
    /// request names, so wpa_supplicant may follow it or rejoin once
    /// dropped.
    LetGo,
}

impl Request {
    /// Reads the signal's one dictionary, or nothing where it isn't in the
    /// shape the patch says it.
    pub fn from_args(args: &HashMap<String, OwnedValue>) -> Option<Request> {
        let value = |key: &str| args.get(key).map(|value| &**value);
        let Some(Value::Bool(warns)) = value("Disassociating") else {
            return None;
        };
        let Some(Value::U32(in_ms)) = value("DisassociationMs") else {
            return None;
        };
        let Some(Value::U32(valid_ms)) = value("ValidMs") else {
            return None;
        };
        let Some(Value::Array(candidates)) = value("Candidates") else {
            return None;
        };
        let byte = |value: &Value| match value {
            Value::U8(byte) => Some(*byte),
            _ => None,
        };
        let candidate = |value: &Value| {
            let Value::Structure(candidate) = value else {
                return None;
            };
            let [Value::Array(address), Value::I16(preference)] = candidate.fields() else {
                return None;
            };
            let address = address.iter().map(byte).collect::<Option<Vec<u8>>>()?;
            Some((address, *preference))
        };
        let candidates = candidates
            .iter()
            .map(candidate)
            .collect::<Option<Vec<_>>>()?;
        Some(Request::read((*warns, *in_ms, *valid_ms, candidates)))
    }

    /// Reads what the signal says. A candidate whose address isn't six
    /// bytes is passed over.
    fn read((warns, in_ms, valid_ms, candidates): Said) -> Request {
        let candidates = candidates
            .into_iter()
            .filter_map(|(address, preference)| {
                let address: [u8; 6] = address.try_into().ok()?;
                Some(Candidate {
                    bssid: bssid(address),
                    preference: u8::try_from(preference).ok(),
                })
            })
            .collect();
        Request {
            warns,
            in_ms,
            valid_ms,
            candidates,
        }
    }

    /// What the journal says of it, for a card on a point, or on none
    /// known.
    pub fn said(&self, card: &str, from: Option<&str>) -> String {
        let from = from.unwrap_or("its point");
        let mut said = format!("{card} was asked by {from} to move");
        if self.candidates.is_empty() {
            said.push_str(", naming no point");
        } else {
            said.push_str(" to ");
            for (index, candidate) in self.candidates.iter().enumerate() {
                if index > 0 {
                    said.push_str(", ");
                }
                let _ = match candidate.preference {
                    Some(0) => write!(said, "not {}", candidate.bssid),
                    Some(preference) => write!(said, "{} at {preference}", candidate.bssid),
                    None => write!(said, "{}", candidate.bssid),
                };
            }
            let _ = write!(said, ", for {} ms", self.valid_ms);
        }
        if self.warns {
            let _ = write!(said, ", warning it drops the card in {} ms", self.in_ms);
        } else {
            said.push_str(", warning nothing");
        }
        said
    }

    /// Where a card on `from`, hearing these points of its network as
    /// their address and signal, goes for this request: to the candidate
    /// the point asking would rather, of those heard and not ruled out,
    /// the more strongly heard of two it would as soon.
    pub fn heed(&self, from: Option<&str>, heard: &[(String, i16)]) -> Heed {
        if !self.warns {
            return Heed::Stay;
        }
        let elsewhere = |bssid: &str| from.is_none_or(|from| !from.eq_ignore_ascii_case(bssid));
        let chosen = self
            .candidates
            .iter()
            .filter(|candidate| candidate.preference != Some(0) && elsewhere(&candidate.bssid))
            .filter_map(|candidate| {
                let (_, signal) = heard
                    .iter()
                    .find(|(bssid, _)| bssid.eq_ignore_ascii_case(&candidate.bssid))?;
                // No preference given ranks below any given.
                let rank = candidate
                    .preference
                    .map_or(0, |preference| u16::from(preference) + 1);
                Some((rank, *signal, &candidate.bssid))
            })
            .max_by_key(|&(rank, signal, _)| (rank, signal));
        chosen.map_or(Heed::LetGo, |(_, _, bssid)| Heed::Move(bssid.clone()))
    }
}

/// An address written as wpa_supplicant's access points say theirs.
fn bssid(address: [u8; 6]) -> String {
    let octets: Vec<String> = address.iter().map(|octet| format!("{octet:02x}")).collect();
    octets.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: &str = "02:00:00:00:00:00";
    const NEAR: &str = "02:00:00:00:05:00";
    const FAR: &str = "02:00:00:00:06:00";

    fn named(bssid: &str, preference: Option<u8>) -> Candidate {
        Candidate {
            bssid: bssid.to_owned(),
            preference,
        }
    }

    fn warning(candidates: Vec<Candidate>) -> Request {
        Request {
            warns: true,
            in_ms: 2048,
            valid_ms: 25_600,
            candidates,
        }
    }

    fn heard(points: &[(&str, i16)]) -> Vec<(String, i16)> {
        points
            .iter()
            .map(|&(bssid, signal)| (bssid.to_owned(), signal))
            .collect()
    }

    #[test]
    fn a_request_is_read_with_its_candidates_in_order_and_a_bad_address_passed_over() {
        let read = Request::read((
            true,
            2048,
            25_600,
            vec![
                (vec![2, 0, 0, 0, 5, 0], 255),
                (vec![2, 0, 0, 0, 6], 200),
                (vec![2, 0, 0, 0, 0x6a, 0xff], -1),
                (vec![2, 0, 0, 0, 0, 0], 0),
            ],
        ));
        let expected = warning(vec![
            named(NEAR, Some(255)),
            named("02:00:00:00:6a:ff", None),
            named(ON, Some(0)),
        ]);
        assert_eq!(
            read, expected,
            "the candidates as given, the five-byte one left out"
        );
    }

    #[test]
    fn the_signal_is_read_from_its_dictionary_and_refused_out_of_shape() {
        let candidates = vec![
            (vec![2_u8, 0, 0, 0, 5, 0], 255_i16),
            (vec![2, 0, 0, 0, 6, 0], -1),
        ];
        let mut args = HashMap::from([
            ("Disassociating".to_owned(), OwnedValue::from(true)),
            ("DisassociationMs".to_owned(), OwnedValue::from(2048_u32)),
            ("ValidMs".to_owned(), OwnedValue::from(25_600_u32)),
            (
                "Candidates".to_owned(),
                OwnedValue::try_from(Value::from(candidates)).expect("candidates as a value"),
            ),
        ]);
        assert_eq!(
            Request::from_args(&args),
            Some(warning(vec![named(NEAR, Some(255)), named(FAR, None)])),
            "each field as the patch says it",
        );
        args.insert("ValidMs".to_owned(), OwnedValue::from(25_600_i32));
        assert_eq!(
            Request::from_args(&args),
            None,
            "a field of another type is out of shape"
        );
    }

    #[test]
    fn the_journal_says_each_candidate_its_preference_and_the_warning() {
        let request = warning(vec![
            named(NEAR, Some(255)),
            named(FAR, None),
            named(ON, Some(0)),
        ]);
        assert_eq!(
            request.said("wlan2", Some(ON)),
            "wlan2 was asked by 02:00:00:00:00:00 to move to 02:00:00:00:05:00 at 255, \
             02:00:00:00:06:00, not 02:00:00:00:00:00, for 25600 ms, \
             warning it drops the card in 2048 ms",
        );
        let unwarned = Request {
            warns: false,
            candidates: Vec::new(),
            ..request
        };
        assert_eq!(
            unwarned.said("wlan2", None),
            "wlan2 was asked by its point to move, naming no point, warning nothing",
        );
    }

    #[test]
    fn a_request_that_warns_nothing_moves_nothing() {
        let request = Request {
            warns: false,
            ..warning(vec![named(NEAR, Some(255))])
        };
        assert_eq!(
            request.heed(Some(ON), &heard(&[(NEAR, -40)])),
            Heed::Stay,
            "wifid chooses by how a point carries, so a plain ask is only said",
        );
    }

    #[test]
    fn a_warned_card_goes_where_the_point_would_rather_of_those_it_hears() {
        let request = warning(vec![
            named(FAR, Some(255)),
            named(NEAR, Some(100)),
            named("02:00:00:00:07:00", None),
        ]);
        assert_eq!(
            request.heed(Some(ON), &heard(&[(NEAR, -70), ("02:00:00:00:07:00", -30)])),
            Heed::Move(NEAR.to_owned()),
            "the unheard favourite is passed over, and any preference beats none",
        );
        assert_eq!(
            request.heed(Some(ON), &heard(&[(FAR, -80), (NEAR, -30)])),
            Heed::Move(FAR.to_owned()),
            "a preference outranks a stronger signal",
        );
    }

    #[test]
    fn a_warned_card_takes_the_stronger_of_two_the_point_would_as_soon() {
        let request = warning(vec![named(NEAR, Some(100)), named(FAR, Some(100))]);
        assert_eq!(
            request.heed(Some(ON), &heard(&[(NEAR, -70), (FAR, -50)])),
            Heed::Move(FAR.to_owned()),
        );
    }

    #[test]
    fn a_warned_card_never_moves_to_its_own_point_or_one_ruled_out() {
        let request = warning(vec![named(ON, Some(255)), named(NEAR, Some(0))]);
        assert_eq!(
            request.heed(Some(&ON.to_uppercase()), &heard(&[(ON, -30), (NEAR, -40)])),
            Heed::LetGo,
            "its own point however written, and a point at 0, are no way out",
        );
    }

    #[test]
    fn a_warned_card_hearing_no_candidate_is_let_go() {
        assert_eq!(
            warning(vec![named(FAR, Some(255))]).heed(Some(ON), &heard(&[(NEAR, -40)])),
            Heed::LetGo,
        );
        assert_eq!(
            warning(Vec::new()).heed(Some(ON), &heard(&[(NEAR, -40)])),
            Heed::LetGo,
            "one naming nothing leaves wpa_supplicant to rejoin where it can",
        );
    }
}
