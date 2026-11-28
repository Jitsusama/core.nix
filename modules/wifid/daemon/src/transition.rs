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
//!
//! One warning that the whole network drops the card has no point of that
//! network to go to, so the card is only let go.
//!
//! A warning read from a frame its link doesn't protect could have come
//! from any station in range, naming any point it runs, and a point named
//! is ranked by the preference the frame gives it. Held to its point, a
//! card can't be taken to a forger's point of its own address, so wifid
//! keeps that from such a frame:
//!
//! - On an open network anyone may run a point of its name, so the card
//!   is only let go and nothing is kept off: it rejoins by signal once
//!   dropped, as a forged drop would leave it.
//! - On one joined with a password or an account, the card is moved only
//!   to a point heard with that same security, which nobody runs without
//!   the network's key or proves without its server. One holding a WPA2
//!   network's shared key can already read what a card joining in its
//!   hearing sends, so a point of its own gains it little.
//!
//! Such a frame can still steer a card within its network, to the point
//! it names, and keep its chooser off the point that warned it. That's
//! followed all the same, as a link that doesn't protect its frames takes
//! a forged drop as readily, which costs the card its link outright, and
//! the home network is one: every warning its points send comes
//! unprotected, so following none would follow nothing there. What such a
//! frame can't do is say how long the point is kept off: that's only ever
//! as long as a move settles (`Request::keeps_off`). Each frame sent anew
//! keeps it off anew, so one sent every ten minutes keeps the card off its
//! point as long as they're sent; a forged drop sent as often costs the
//! card its link each time, which is worse, and needs no warning heeded.
//!
//! A card looking for the points named waits in `look`. Long because its
//! cases, over half of it, stay beside the private reading they prove, as
//! every module in wifid keeps them.

use std::collections::HashMap;
use std::fmt::Write as _;

use tokio::time::Duration;
use zbus::zvariant::{OwnedValue, Value};

use crate::choose::SETTLE;
use crate::state::Security;

/// The longest a card that left a point asking keeps its chooser off it.
/// A real point's candidates hold 26 s at most, 255 beacons of 100 TU, so
/// a request holding longer has a beacon interval no point sends. Even a
/// protected request can say one: wpa_supplicant times it by the beacon
/// interval its point's beacons last said (`wnm_sta.c`), and beacons go
/// unprotected, so anyone in range can forge a longer one.
pub const KEEPS_OFF_AT_MOST: Duration = Duration::from_hours(1);

/// One request, as wpa_supplicant said it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// Whether the point warns it will drop the card.
    pub warns: bool,
    /// Whether it warns its whole network will.
    pub leaves_network: bool,
    /// Whether the card's link protects management frames. It says how
    /// the link was joined, not how this frame came: only the kernel's
    /// dropping an unprotected one on such a link, as mac80211 does, means
    /// only the point could have sent it. Every card core.nix's machines
    /// carry runs on mac80211 (iwlwifi); one whose driver does its own
    /// management frames would need that checked before this is trusted.
    pub protected: bool,
    /// In how many milliseconds, where it warns; 0 where the point sets no
    /// timer, which says nothing of when rather than at once.
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
    /// The channel it's on, in MHz, as the point gave it; `None` for one
    /// wpa_supplicant couldn't place.
    pub frequency: Option<u32>,
}

/// What the signal says: whether the point warns, whether its whole
/// network does and whether the link protects its frames, in how long and
/// for how long in milliseconds, and each candidate's address, preference,
/// -1 for none, and channel in MHz, 0 for none.
struct Said {
    warns: bool,
    leaves_network: bool,
    protected: bool,
    in_ms: u32,
    valid_ms: u32,
    candidates: Vec<(Vec<u8>, i16, u32)>,
}

/// What wifid does with a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Heed {
    /// Nothing but say it: the request doesn't warn.
    Stay,
    /// Nothing but say it, as wifid doesn't hold the card to its point:
    /// the request is wpa_supplicant's to follow or turn down.
    Unheld,
    /// Nothing but say it, as the card is already on a move away from the
    /// point that warned it: a point can warn again until the card is
    /// gone, and asking the move again would start it over.
    Leaving,
    /// Lets the card join any point again, as its whole network warns it
    /// drops the card, so no other point of it would keep the card.
    NetworkDrops,
    /// Lets the card join any point again, keeping none off, as the
    /// warning came unprotected on an open network, where any station
    /// could have sent it and could run the points it names.
    Unproven,
    /// Moves the card to this point, the one it would rather of those the
    /// card hears.
    Move(String),
    /// Looks over these channels, in MHz, for the points the request
    /// names, as the card hears none of them. wpa_supplicant forgets a
    /// point it hasn't heard for 3 minutes, and a card on a strong link
    /// looks only once an hour, so a card long on its point most often
    /// hears none of them as it's warned. A look over a few channels
    /// takes a fraction of a second, where one over all of them takes 5
    /// to 6 s, as long as a point most often gives the card.
    Look(Vec<u32>),
    /// Lets the card join any point of its network, as it hears none the
    /// request names that wifid can move it to, having looked, so
    /// wpa_supplicant rejoins wherever it can once dropped. It has most
    /// likely answered the request by then, as it does so as soon as it's
    /// said, so it can't follow it.
    LetGo,
}

impl Heed {
    /// Whether the card leaves the point that warned it this way, so no
    /// move of its goes back there should it miss, and its chooser keeps
    /// off that point for a time (`Request::keeps_off`). One that stays,
    /// unheld or already leaving is none: the first two follow no warning,
    /// and the last was warned before. Nor is one unproven, as a forger
    /// could otherwise keep the card off its own point.
    pub fn leaves(&self) -> bool {
        matches!(
            self,
            Heed::Move(_) | Heed::Look(_) | Heed::LetGo | Heed::NetworkDrops
        )
    }

    /// Whether wifid itself moves the card off the point that warned it
    /// this way, or looks first to: one let go or dropped with its whole
    /// network is left to wpa_supplicant, so nothing of wifid's is under
    /// way that a call's give-way should wait on (`State::warned_on`).
    pub fn moves(&self) -> bool {
        matches!(self, Heed::Move(_) | Heed::Look(_))
    }
}

/// Each channel these candidates are on, once, in order.
fn channels_of<'a>(candidates: impl Iterator<Item = &'a Candidate>) -> Vec<u32> {
    let mut channels: Vec<u32> = candidates
        .filter_map(|candidate| candidate.frequency)
        .collect();
    channels.sort_unstable();
    channels.dedup();
    channels
}

impl Request {
    /// Reads the signal's one dictionary, or nothing where it isn't in the
    /// shape the patch says it.
    pub fn from_args(args: &HashMap<String, OwnedValue>) -> Option<Request> {
        let value = |key: &str| args.get(key).map(|value| &**value);
        let Some(Value::Bool(warns)) = value("Disassociating") else {
            return None;
        };
        let Some(Value::Bool(leaves_network)) = value("LeavingNetwork") else {
            return None;
        };
        let Some(Value::Bool(protected)) = value("Protected") else {
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
            let [
                Value::Array(address),
                Value::I16(preference),
                Value::U32(frequency),
            ] = candidate.fields()
            else {
                return None;
            };
            let address = address.iter().map(byte).collect::<Option<Vec<u8>>>()?;
            Some((address, *preference, *frequency))
        };
        let candidates = candidates
            .iter()
            .map(candidate)
            .collect::<Option<Vec<_>>>()?;
        Some(Request::read(Said {
            warns: *warns,
            leaves_network: *leaves_network,
            protected: *protected,
            in_ms: *in_ms,
            valid_ms: *valid_ms,
            candidates,
        }))
    }

    /// Reads what the signal says. A candidate whose address isn't six
    /// bytes is passed over.
    fn read(said: Said) -> Request {
        let candidates = said
            .candidates
            .into_iter()
            .filter_map(|(address, preference, frequency)| {
                let address: [u8; 6] = address.try_into().ok()?;
                Some(Candidate {
                    bssid: bssid(address),
                    preference: u8::try_from(preference).ok(),
                    frequency: Some(frequency).filter(|&frequency| frequency != 0),
                })
            })
            .collect();
        Request {
            warns: said.warns,
            leaves_network: said.leaves_network,
            protected: said.protected,
            in_ms: said.in_ms,
            valid_ms: said.valid_ms,
            candidates,
        }
    }

    /// How long a card that heeded it this way keeps its chooser off the
    /// point asking, if it left that point (`Heed::leaves`): as long as its
    /// candidates hold, and at least as long as a move the chooser makes
    /// itself holds the card still, so the next minute's choice can't send
    /// the card back to a point that just shed it. An unprotected frame
    /// could say any time, so it keeps the point off only as long as a move
    /// settles, which is what a real point's candidates give anyway.
    pub fn keeps_off(&self, heeded: &Heed) -> Option<Duration> {
        if !heeded.leaves() {
            return None;
        }
        if !self.protected {
            return Some(SETTLE);
        }
        Some(Duration::from_millis(self.valid_ms.into()).clamp(SETTLE, KEEPS_OFF_AT_MOST))
    }

    /// Whether a card on `from`, hearing `heard` over a link of `link`'s
    /// security, would look before it heeded this: taken as held and not
    /// leaving, the most that would move the card. One that doesn't, as
    /// its whole network drops the card or it names a point heard, is
    /// heeded at once even as the card waits on a look (`Looks`).
    pub fn needs_look(
        &self,
        from: Option<&str>,
        heard: &[(String, i16, Security)],
        link: Security,
    ) -> bool {
        matches!(
            self.heed(from, heard, true, false, false, link),
            Heed::Look(_)
        )
    }

    /// Whether it warns of a drop, of its point or its whole network.
    pub fn warns_any(&self) -> bool {
        self.warns || self.leaves_network
    }

    /// Each channel its candidates are on, once, in order.
    pub fn channels(&self) -> Vec<u32> {
        channels_of(self.candidates.iter())
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
        if self.warns_any() {
            if self.leaves_network {
                said.push_str(", warning its whole network drops the card");
            } else {
                said.push_str(", warning it drops the card");
            }
            if self.in_ms == 0 {
                said.push_str(", saying not when");
            } else {
                let _ = write!(said, " in {} ms", self.in_ms);
            }
        } else {
            said.push_str(", warning nothing");
        }
        // Any station can send an unprotected ask as the point, so the
        // journal says which ones came so, warning or not.
        if !self.protected {
            said.push_str(", unprotected");
        }
        said
    }

    /// Where a card on `from`, hearing these points of its network as
    /// their address, signal and security, goes for this request: to the
    /// candidate the point asking would rather, of those heard and not
    /// ruled out, the more strongly heard of two it would as soon. Only a
    /// card wifid holds, `held`, is moved, and only to a point `heard`
    /// names with the security its network is joined with, `link`: one
    /// the card itself heard on its network. One hearing none looks for
    /// them first, unless it already has, `looked`, or no channel of
    /// theirs is known. A card whose whole network drops it is let go,
    /// even one already on a move within it, and one already `leaving` a
    /// point that warned it is otherwise left to that move. An unprotected
    /// warning on an open network only lets the card go.
    pub fn heed(
        &self,
        from: Option<&str>,
        heard: &[(String, i16, Security)],
        held: bool,
        leaving: bool,
        looked: bool,
        link: Security,
    ) -> Heed {
        if !self.warns_any() {
            return Heed::Stay;
        }
        if !held {
            return Heed::Unheld;
        }
        if !self.protected && link == Security::Open {
            return if leaving {
                Heed::Leaving
            } else {
                Heed::Unproven
            };
        }
        if self.leaves_network {
            return Heed::NetworkDrops;
        }
        if leaving {
            return Heed::Leaving;
        }
        let elsewhere = |bssid: &str| from.is_none_or(|from| !from.eq_ignore_ascii_case(bssid));
        let ways_out = self
            .candidates
            .iter()
            .filter(|candidate| candidate.preference != Some(0) && elsewhere(&candidate.bssid));
        let chosen = ways_out
            .clone()
            .filter_map(|candidate| {
                let (_, signal, _) = heard.iter().find(|(bssid, _, security)| {
                    bssid.eq_ignore_ascii_case(&candidate.bssid) && *security == link
                })?;
                // No preference given ranks below any given.
                let rank = candidate
                    .preference
                    .map_or(0, |preference| u16::from(preference) + 1);
                Some((rank, *signal, &candidate.bssid))
            })
            .max_by_key(|&(rank, signal, _)| (rank, signal));
        if let Some((_, _, bssid)) = chosen {
            return Heed::Move(bssid.clone());
        }
        let channels = channels_of(ways_out);
        if looked || channels.is_empty() {
            Heed::LetGo
        } else {
            Heed::Look(channels)
        }
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

    /// A point the request names, on channel 6.
    fn named(bssid: &str, preference: Option<u8>) -> Candidate {
        Candidate {
            bssid: bssid.to_owned(),
            preference,
            frequency: Some(2437),
        }
    }

    /// How a card holding to its point, leaving none, heeds a request
    /// before it's looked.
    fn heeds(request: &Request, heard_now: &[(&str, i16)]) -> Heed {
        request.heed(Some(ON), &heard(heard_now), true, false, false, PASSWORD)
    }

    /// How a card on a network joined with a password hears its points.
    const PASSWORD: Security = Security::Personal;

    fn warning(candidates: Vec<Candidate>) -> Request {
        Request {
            warns: true,
            leaves_network: false,
            protected: true,
            in_ms: 2048,
            valid_ms: 25_600,
            candidates,
        }
    }

    fn heard(points: &[(&str, i16)]) -> Vec<(String, i16, Security)> {
        points
            .iter()
            .map(|&(bssid, signal)| (bssid.to_owned(), signal, PASSWORD))
            .collect()
    }

    #[test]
    fn a_request_is_read_with_its_candidates_in_order_and_a_bad_address_passed_over() {
        let read = Request::read(Said {
            warns: true,
            leaves_network: false,
            protected: true,
            in_ms: 2048,
            valid_ms: 25_600,
            candidates: vec![
                (vec![2, 0, 0, 0, 5, 0], 255, 2437),
                (vec![2, 0, 0, 0, 6], 200, 2437),
                (vec![2, 0, 0, 0, 0x6a, 0xff], -1, 0),
                (vec![2, 0, 0, 0, 0, 0], 0, 2437),
            ],
        });
        let unplaced = Candidate {
            frequency: None,
            ..named("02:00:00:00:6a:ff", None)
        };
        let expected = warning(vec![named(NEAR, Some(255)), unplaced, named(ON, Some(0))]);
        assert_eq!(
            read, expected,
            "the candidates as given, the five-byte one left out, channel 0 none"
        );
    }

    #[test]
    fn the_signal_is_read_from_its_dictionary_and_refused_out_of_shape() {
        let candidates = vec![
            (vec![2_u8, 0, 0, 0, 5, 0], 255_i16, 2437_u32),
            (vec![2, 0, 0, 0, 6, 0], -1, 2437),
        ];
        let mut args = HashMap::from([
            ("Disassociating".to_owned(), OwnedValue::from(true)),
            ("LeavingNetwork".to_owned(), OwnedValue::from(false)),
            ("Protected".to_owned(), OwnedValue::from(true)),
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
        args.insert("ValidMs".to_owned(), OwnedValue::from(25_600_u32));
        let unplaced = vec![(vec![2_u8, 0, 0, 0, 5, 0], 255_i16)];
        args.insert(
            "Candidates".to_owned(),
            OwnedValue::try_from(Value::from(unplaced)).expect("candidates as a value"),
        );
        assert_eq!(
            Request::from_args(&args),
            None,
            "a candidate with no channel is an older patch's"
        );
        args.remove("Protected");
        assert_eq!(
            Request::from_args(&args),
            None,
            "a field left out is out of shape"
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
        let forgeable = Request {
            protected: false,
            ..unwarned
        };
        assert_eq!(
            forgeable.said("wlan2", None),
            "wlan2 was asked by its point to move, naming no point, warning nothing, \
             unprotected",
            "an ask anyone could have sent read as the point's own"
        );
    }

    #[test]
    fn the_journal_says_a_whole_network_warning_one_saying_not_when_and_one_unprotected() {
        let whole = Request {
            warns: false,
            leaves_network: true,
            ..warning(Vec::new())
        };
        assert_eq!(
            whole.said("wlan2", Some(ON)),
            "wlan2 was asked by 02:00:00:00:00:00 to move, naming no point, \
             warning its whole network drops the card in 2048 ms",
        );
        let untimed = Request {
            in_ms: 0,
            protected: false,
            ..warning(Vec::new())
        };
        assert_eq!(
            untimed.said("wlan2", Some(ON)),
            "wlan2 was asked by 02:00:00:00:00:00 to move, naming no point, \
             warning it drops the card, saying not when, unprotected",
            "no timer isn't a drop at once, and a frame anyone could send is said so",
        );
    }

    #[test]
    fn only_a_heed_that_leaves_the_warning_point_keeps_it_off() {
        let leaving = [
            Heed::Move(NEAR.to_owned()),
            Heed::Look(vec![2437]),
            Heed::LetGo,
            Heed::NetworkDrops,
        ];
        for heed in leaving {
            assert!(
                heed.leaves(),
                "{heed:?} would send the card back to the drop"
            );
        }
        for heed in [Heed::Stay, Heed::Unheld, Heed::Leaving] {
            assert!(!heed.leaves(), "{heed:?} follows no new warning");
        }
        assert!(
            !Heed::Unproven.leaves(),
            "a warning anyone could have sent would keep the card off its own point"
        );
    }

    #[test]
    fn only_a_heed_that_moves_or_looks_is_under_way_in_wifid() {
        for heed in [Heed::Move(NEAR.to_owned()), Heed::Look(vec![2437])] {
            assert!(heed.moves(), "{heed:?} wasn't said to move the card");
        }
        let left = [
            Heed::LetGo,
            Heed::NetworkDrops,
            Heed::Stay,
            Heed::Unheld,
            Heed::Leaving,
            Heed::Unproven,
        ];
        for heed in left {
            assert!(
                !heed.moves(),
                "{heed:?}, which wifid doesn't move, was said to"
            );
        }
    }

    #[test]
    fn a_warning_point_is_kept_off_as_long_as_its_candidates_hold_or_a_move_settles() {
        let moved = Heed::Move(NEAR.to_owned());
        assert_eq!(
            warning(Vec::new()).keeps_off(&moved),
            Some(SETTLE),
            "25.6 s of candidates is shorter than a move settles"
        );
        let long = Request {
            valid_ms: 3_600_000,
            ..warning(Vec::new())
        };
        assert_eq!(long.keeps_off(&moved), Some(Duration::from_hours(1)));
        let absurd = Request {
            valid_ms: u32::MAX,
            ..warning(Vec::new())
        };
        assert_eq!(
            absurd.keeps_off(&moved),
            Some(KEEPS_OFF_AT_MOST),
            "a request holding past any real point's can't keep the point off for days"
        );
        let half = Request {
            valid_ms: 1_800_000,
            ..long.clone()
        };
        assert_eq!(
            half.keeps_off(&moved),
            Some(Duration::from_mins(30)),
            "a hold between a move's settling and the most wasn't the request's own"
        );
        let forged = Request {
            protected: false,
            ..long
        };
        assert_eq!(
            forged.keeps_off(&moved),
            Some(SETTLE),
            "an unprotected frame set how long its point was kept off"
        );
        assert_eq!(
            half.keeps_off(&Heed::Leaving),
            None,
            "a card heeding no new warning kept its point off"
        );
    }

    #[test]
    fn a_warning_needs_a_look_only_when_it_names_no_point_heard() {
        let asked = warning(vec![named(NEAR, Some(255))]);
        assert!(
            asked.needs_look(Some(ON), &[], PASSWORD),
            "a card hearing none of the points named was moved without a look"
        );
        assert!(
            !asked.needs_look(Some(ON), &heard(&[(NEAR, -50)]), PASSWORD),
            "a warning naming a point heard waited on a look it doesn't need"
        );
        let whole = Request {
            leaves_network: true,
            ..asked.clone()
        };
        assert!(
            !whole.needs_look(Some(ON), &[], PASSWORD),
            "a whole network dropping the card waited on a look"
        );
        let nowhere = warning(vec![Candidate {
            frequency: None,
            ..named(NEAR, Some(255))
        }]);
        assert!(
            !nowhere.needs_look(Some(ON), &[], PASSWORD),
            "a warning naming a point on no channel waited on a look that can't find it"
        );
        let forged = Request {
            protected: false,
            ..asked
        };
        assert!(
            !forged.needs_look(Some(ON), &[], Security::Open),
            "an unprotected warning on an open network looked for a forger's points"
        );
    }

    #[test]
    fn an_unprotected_warning_on_an_open_network_only_lets_the_card_go() {
        let forged = Request {
            protected: false,
            ..warning(vec![named(NEAR, Some(255))])
        };
        let open = [(NEAR.to_owned(), -40, Security::Open)];
        let heeds_on = |request: &Request, leaving: bool, link: Security| {
            request.heed(Some(ON), &open, true, leaving, false, link)
        };
        assert_eq!(
            heeds_on(&forged, false, Security::Open),
            Heed::Unproven,
            "anyone in range could have sent it and could run the point it names",
        );
        assert_eq!(
            heeds_on(&forged, true, Security::Open),
            Heed::Leaving,
            "a card already leaving keeps to its move",
        );
        let protected = Request {
            protected: true,
            ..forged
        };
        assert_eq!(
            heeds_on(&protected, false, Security::Open),
            Heed::Move(NEAR.to_owned()),
            "a link that protects its frames proves the point sent it",
        );
    }

    #[test]
    fn a_warned_card_moves_only_to_a_point_joined_as_its_own_network_is() {
        let unprotected = Request {
            protected: false,
            ..warning(vec![named(NEAR, Some(255)), named(FAR, Some(10))])
        };
        let heard = [
            (NEAR.to_owned(), -30, Security::Open),
            (FAR.to_owned(), -70, PASSWORD),
        ];
        assert_eq!(
            unprotected.heed(Some(ON), &heard, true, false, false, PASSWORD),
            Heed::Move(FAR.to_owned()),
            "an open point of the same name is anyone's, so the preferred one is passed over",
        );
        assert_eq!(
            unprotected.heed(Some(ON), &heard[..1], true, false, false, PASSWORD),
            Heed::Look(vec![2437]),
            "and one hearing only that looks for the real ones",
        );
        let protected = Request {
            protected: true,
            ..unprotected
        };
        assert_eq!(
            protected.heed(Some(ON), &heard, true, false, false, PASSWORD),
            Heed::Move(FAR.to_owned()),
            "a protected frame still can't send the card to a point of other security",
        );
    }

    #[test]
    fn a_request_that_warns_nothing_moves_nothing() {
        let request = Request {
            warns: false,
            ..warning(vec![named(NEAR, Some(255))])
        };
        assert_eq!(
            heeds(&request, &[(NEAR, -40)]),
            Heed::Stay,
            "wifid chooses by how a point carries, so a plain ask is only said",
        );
    }

    #[test]
    fn a_warned_card_wifid_doesnt_hold_is_left_to_wpa_supplicant() {
        assert_eq!(
            warning(vec![named(NEAR, Some(255))]).heed(
                Some(ON),
                &heard(&[(NEAR, -40)]),
                false,
                false,
                false,
                PASSWORD
            ),
            Heed::Unheld,
            "a card free to join any point is wpa_supplicant's to move",
        );
    }

    #[test]
    fn a_card_already_leaving_a_point_that_warned_it_keeps_to_its_move() {
        let again = warning(vec![named(FAR, Some(255)), named(NEAR, Some(100))]);
        assert_eq!(
            again.heed(
                Some(ON),
                &heard(&[(NEAR, -40), (FAR, -30)]),
                true,
                true,
                false,
                PASSWORD
            ),
            Heed::Leaving,
            "a warning said again isn't a move asked again",
        );
    }

    #[test]
    fn a_card_leaving_for_a_point_of_a_network_that_drops_it_is_let_go() {
        let whole = Request {
            warns: false,
            leaves_network: true,
            ..warning(vec![named(NEAR, Some(255))])
        };
        assert_eq!(
            whole.heed(
                Some(ON),
                &heard(&[(NEAR, -40)]),
                true,
                true,
                false,
                PASSWORD
            ),
            Heed::NetworkDrops,
            "a move to another point of the network would land on one that drops the card",
        );
    }

    #[test]
    fn a_card_whose_whole_network_drops_it_is_let_go_whatever_it_names() {
        let whole = Request {
            warns: false,
            leaves_network: true,
            ..warning(vec![named(NEAR, Some(255))])
        };
        assert_eq!(
            heeds(&whole, &[(NEAR, -40)]),
            Heed::NetworkDrops,
            "no point of a network that drops the card keeps it",
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
            heeds(&request, &[(NEAR, -70), ("02:00:00:00:07:00", -30)]),
            Heed::Move(NEAR.to_owned()),
            "the unheard favourite is passed over, and any preference beats none",
        );
        assert_eq!(
            heeds(&request, &[(FAR, -80), (NEAR, -30)]),
            Heed::Move(FAR.to_owned()),
            "a preference outranks a stronger signal",
        );
    }

    #[test]
    fn a_warned_card_takes_the_stronger_of_two_the_point_would_as_soon() {
        let request = warning(vec![named(NEAR, Some(100)), named(FAR, Some(100))]);
        assert_eq!(
            heeds(&request, &[(NEAR, -70), (FAR, -50)]),
            Heed::Move(FAR.to_owned()),
        );
    }

    #[test]
    fn a_warned_card_never_moves_to_its_own_point_or_one_ruled_out() {
        let request = warning(vec![named(ON, Some(255)), named(NEAR, Some(0))]);
        assert_eq!(
            request.heed(
                Some(&ON.to_uppercase()),
                &heard(&[(ON, -30), (NEAR, -40)]),
                true,
                false,
                false,
                PASSWORD
            ),
            Heed::LetGo,
            "its own point however written, and a point at 0, are no way out, nor looked for",
        );
    }

    #[test]
    fn a_warned_card_hearing_no_candidate_looks_over_their_channels_once() {
        let on_36 = Candidate {
            frequency: Some(5180),
            ..named("02:00:00:00:07:00", None)
        };
        let request = warning(vec![named(FAR, Some(255)), on_36, named(NEAR, Some(10))]);
        assert_eq!(
            heeds(&request, &[(ON, -40)]),
            Heed::Look(vec![2437, 5180]),
            "each channel once, in order",
        );
        assert_eq!(
            request.heed(Some(ON), &heard(&[(ON, -40)]), true, false, true, PASSWORD),
            Heed::LetGo,
            "one that looked and still hears none is let go",
        );
        assert_eq!(
            request.heed(
                Some(ON),
                &heard(&[(ON, -40), (NEAR, -60)]),
                true,
                false,
                true,
                PASSWORD
            ),
            Heed::Move(NEAR.to_owned()),
            "one that looked goes to what it heard",
        );
    }

    #[test]
    fn a_warned_card_with_no_channel_to_look_over_is_let_go() {
        let unplaced = Candidate {
            frequency: None,
            ..named(FAR, Some(255))
        };
        assert_eq!(
            heeds(&warning(vec![unplaced]), &[(NEAR, -40)]),
            Heed::LetGo,
            "a look over every channel takes as long as the point gives",
        );
        assert_eq!(
            heeds(&warning(Vec::new()), &[(NEAR, -40)]),
            Heed::LetGo,
            "one naming nothing leaves wpa_supplicant to rejoin where it can",
        );
    }
}
